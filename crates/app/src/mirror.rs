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

//! The app's mirror of the session (`docs/gui.md`, A frame; decision 59): the one place the
//! session's events enter the app. [`Mirror::apply`] takes one event, updates what the app
//! keeps of the session and returns the [`Reaction`]s the app owes in reply, the things
//! only egui, the dialog host or the platform can do. It holds no egui type and reaches
//! nothing outside the mirror, so a test feeds it events and reads the state and the
//! reactions back.
//!
//! - **What it holds:** the photo ([`PhotoState`]), the modules' values as the session last
//!   reported them with the changes sent since shown over them, what the side panel's
//!   editing tabs draw of them ([`Shown`]), the latest frame, the scopes' reading, whether
//!   the edit is on disk ([`Saving`]), the jobs ([`jobs::Mirror`]), the failure the central
//!   panel shows, the open folder ([`folder::FolderState`]) and the Export dialog's floor
//!   for previews. Panels read it each frame; nothing subscribes.
//! - **What it does not hold:** the app's own state (the theme, a tab, a dialog open). That
//!   changes by intents after the frame, with no event.
//! - **No photo:** before any, from the moment another is asked for until its values
//!   arrive, and after an open fails, the side panel draws every module at its
//!   definition's defaults and Scopes with no graph, disabled ([`Shown::Nothing`]): never a blank
//!   panel. The photo's values replace them in one apply ([`Shown::Photo`]).
//! - **Stale events:** while a photo opens, what an earlier open left on its way is dropped
//!   ([`PhotoState::stale`]); a library event naming a command other than the latest open or
//!   close is dropped by the folder ([`folder::FolderState::apply`]).
//! - **A refused command** ([`SessionError::Refused`]) names its number: its pending change
//!   stops showing and a save it would have waited for is not waited for. The values report
//!   that follows a refusal in the same batch says what the edit holds.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use apertrust_engine::session::{
    ControlState, CoordinateState, Event, ExportPreview, Frame, ModuleState, Photo, PresetState,
    SessionError, Values,
};
use apertrust_engine::{edit, sidecar};
use apertrust_modules::Module;
use apertrust_plugin::ImageContext;
use apertrust_ui::components::SaveState;

use crate::folder::{self, FolderState};
use crate::jobs;
use crate::modules::Pending;
use crate::scopes::Scopes;

/// What the app must do in reply to an event, that the mirror cannot: it touches egui, the
/// dialog host or the platform.
#[derive(Debug, PartialEq)]
pub enum Reaction {
    /// The jobs window is to show the jobs as the mirror holds them: open, if the event was
    /// a job the user started (decision 57). Never [`jobs::Show::Nothing`].
    Jobs(jobs::Show),
    /// The Export dialog open shows this preview, answering the command numbered `through`,
    /// when it is the newest the dialog holds. An answer to an earlier dialog's command
    /// ([`Mirror::previews_from`]) is dropped before it gets here.
    Preview {
        through: u64,
        preview: Box<ExportPreview>,
    },
    /// The folder at this path opened: it joins the recent folders.
    FolderOpened(PathBuf),
    /// The folder at this path could not be opened: its entry in the recent folders says so.
    FolderFailed { path: PathBuf, reason: String },
}

/// What the side panel's editing tabs draw (`docs/gui.md`, The side panel): the values
/// the mirror holds, and whether what the user does to them reaches the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The open photo's values and its scopes; its controls send their commands, unless the
    /// photo is open for viewing only (`editable` false).
    Photo { editable: bool },
    /// No photo, or one opening whose values have not arrived: every module at its
    /// definition's defaults and Scopes with no graph, drawn disabled, with no marks, so the
    /// person sees at once that a click on another photo was taken.
    Nothing,
}

impl Shown {
    /// Whether the side panel's resets are drawn enabled.
    pub fn editable(self) -> bool {
        self == Shown::Photo { editable: true }
    }
}

/// The session as the app last heard of it.
pub struct Mirror {
    /// The modules the session was started with, whose views show the pending values.
    modules: &'static [&'static dyn Module],
    pub photo: PhotoState,
    /// The modules' controls and the unowned parameters as the engine last reported them,
    /// with the pending ones in their place (`docs/gui.md`, A frame); with no photo, and
    /// while one opens, every module untouched ([`untouched`]).
    pub values: Values,
    /// What the side panel draws of `values`.
    shown: Shown,
    /// The values with no photo, made once.
    untouched: Values,
    /// What the widgets set that no report includes yet, with each command's number.
    pub pending: Vec<(u64, Pending)>,
    /// What failed since the last frame, shown in the central panel until the next one.
    pub failure: Option<String>,
    /// The latest frame, which the viewport draws.
    pub frame: Option<Frame>,
    /// The latest scopes reading, and what the session was asked to count.
    pub scopes: Scopes,
    /// Whether the photo's edit is on disk as it is now, as the top panel says.
    pub saving: Saving,
    /// The jobs the session has reported, which the spinner and the jobs window show
    /// (decision 57).
    pub jobs: jobs::Mirror,
    /// The open folder and its photos.
    pub folder: FolderState,
    /// The number of the first preview the Export dialog open asked for, then of the newest
    /// answer shown: an answer to an earlier command is from an earlier dialog or is
    /// outdated, and shows nothing.
    export_floor: u64,
}

impl Mirror {
    /// The modules the session was started with.
    pub fn modules(&self) -> &'static [&'static dyn Module] {
        self.modules
    }

    /// The mirror before any event: no photo, every module untouched, the jobs as given.
    pub fn new(modules: &'static [&'static dyn Module], jobs: jobs::Mirror) -> Mirror {
        let untouched = untouched(modules);
        Mirror {
            modules,
            photo: PhotoState::None,
            values: untouched.clone(),
            shown: Shown::Nothing,
            untouched,
            pending: Vec::new(),
            failure: None,
            frame: None,
            scopes: Scopes::default(),
            saving: Saving::default(),
            jobs,
            folder: FolderState::default(),
            export_floor: 0,
        }
    }

    /// What the side panel's editing tabs draw.
    pub fn shown(&self) -> Shown {
        self.shown
    }

    /// Whether a change of the edit the side panel's controls make is to be sent: only the
    /// open photo's, once its values have arrived, and never while it is open for viewing
    /// only.
    pub fn takes_edits(&self) -> bool {
        self.shown == Shown::Photo { editable: true }
    }

    /// The raw at `path` is asked for by the open numbered `open`: nothing of the photo
    /// left is shown for the new one, and the side panel shows no photo until the new
    /// one's values arrive.
    pub fn opening(&mut self, path: PathBuf, open: u64) {
        self.photo = PhotoState::Opening { path, open };
        self.folder.locate(self.photo.path());
        self.saving.leaving();
        self.frame = None;
        self.failure = None;
        self.no_photo();
    }

    /// No photo is open: every module untouched, and no scopes reading.
    fn no_photo(&mut self) {
        self.shown = Shown::Nothing;
        self.values = self.untouched.clone();
        self.pending.clear();
        self.scopes.forget();
    }

    /// The Export dialog opened and asked for the preview numbered `number`: an answer to
    /// an earlier command is from an earlier dialog.
    pub fn previews_from(&mut self, number: u64) {
        self.export_floor = number;
    }

    /// Shows `pending`, sent as command `number`, until a report includes it. Only this
    /// change is shown over the values as they are, which hold every earlier one: shown
    /// again, a coordinate's change would start from coordinates later changes have moved.
    pub fn pend(&mut self, number: u64, pending: Pending) {
        let image = self.photo.image();
        show(&mut self.values, &pending, self.modules, &image);
        self.pending.push((number, pending));
    }

    /// Takes in one event, but for one an earlier open left on its way while another opens
    /// ([`PhotoState::stale`]), and returns what the app owes in reply.
    pub fn apply(&mut self, event: Event) -> Vec<Reaction> {
        let mut reactions = Vec::new();
        if self.photo.stale(&event) {
            return reactions;
        }
        // Where the photo open is in the folder changes only with the photo or the listing.
        let moves = matches!(
            event,
            Event::Opened(_) | Event::Library(_) | Event::Failed(SessionError::Open { .. })
        );
        // The folder takes its own events in, and the reading's job, which the jobs' mirror
        // sees too; what the app owes for them comes back from it.
        match self.folder.apply(&event) {
            Some(folder::Outcome::Opened(path)) => reactions.push(Reaction::FolderOpened(path)),
            Some(folder::Outcome::Failed { path, reason }) => {
                reactions.push(Reaction::FolderFailed { path, reason });
            }
            None => {}
        }
        match event {
            Event::Opened(photo) => {
                self.saving.opened(&photo.path, photo.view_only.is_some());
                self.photo = PhotoState::Open(photo);
            }
            Event::Saved { path, through } => self.saving.saved(&path, through),
            // The open photo's values, which replace the untouched ones in one apply; a
            // session with no photo open sends none.
            Event::Values(values) => {
                if let PhotoState::Open(photo) = &self.photo {
                    // The photo's cell and the Edited sort follow its edit at once.
                    self.folder.edited(&photo.path, values.edited);
                    let editable = photo.view_only.is_none();
                    self.saving.reported(&values);
                    let image = self.photo.image();
                    self.values = reported(values, &mut self.pending, self.modules, &image);
                    self.shown = Shown::Photo { editable };
                }
            }
            Event::Frame(frame) => {
                self.frame = Some(frame);
                self.failure = None;
            }
            Event::Scopes(reading) => self.scopes.receive(&reading),
            // The Export dialog shows the preview, the newest it holds.
            Event::ExportPreview { through, preview } => {
                if through >= self.export_floor {
                    // A preview cancelled in time sends nothing, but one that had finished
                    // sends its answer: only a newer one than this shows.
                    self.export_floor = through;
                    reactions.push(Reaction::Preview { through, preview });
                }
            }
            // The jobs' progress and the export's results are the spinner's and the jobs
            // window's (decision 57).
            event @ (Event::Job { .. } | Event::Export { .. }) => {
                let show = self.jobs.apply(&event);
                jobs_reaction(show, &mut reactions);
            }
            // The folder took these in above.
            Event::Library(_) => {}
            // The session logs a failed save, a module left out and the scopes turned off as
            // they happen; the rest only reach the app.
            Event::Failed(SessionError::Save {
                path,
                through,
                source,
            }) => {
                self.saving
                    .write_failed(&path, through, &source.to_string());
            }
            Event::Failed(error) => self.failed(error, &mut reactions),
        }
        if moves {
            self.folder.locate(self.photo.path());
        }
        reactions
    }

    fn failed(&mut self, error: SessionError, reactions: &mut Vec<Reaction>) {
        if let SessionError::Refused { command, .. } = &error {
            // It changed nothing: what was shown for it, and a save awaited for it, stop.
            let command = *command;
            self.pending.retain(|(number, _)| *number != command);
            self.saving.refused(command);
        }
        if !matches!(error, SessionError::Module { .. } | SessionError::Scopes(_)) {
            tracing::error!(target: "pipe", "{error}");
        }
        // The scopes are off for the session: their last reading stops showing.
        if matches!(error, SessionError::Scopes(_)) {
            self.scopes.forget();
        }
        // The open under way failed, or the engine ended before it could finish; anything
        // else is no open's.
        let ends_open = matches!(
            error,
            SessionError::Open { .. } | SessionError::Panicked(_) | SessionError::Stopped
        );
        // No job the engine ran will report again: they end as failed.
        if matches!(error, SessionError::Panicked(_) | SessionError::Stopped) {
            self.saving.stopped();
            let show = self.jobs.engine_stopped();
            jobs_reaction(show, reactions);
        }
        match &self.photo {
            PhotoState::Opening { path, .. } if ends_open => {
                let message = format!("{}: {error}", path.display());
                self.photo = PhotoState::Failed {
                    path: path.clone(),
                    message,
                };
                // The session closed the photo left before it tried: none is open.
                self.no_photo();
            }
            // The scopes failing turns them off, never the frame (decision 19): nothing over
            // the photo.
            _ if matches!(error, SessionError::Scopes(_)) => {}
            _ => self.failure = Some(error.to_string()),
        }
    }
}

/// Every module's controls at its definition's defaults, as the side panel draws them with
/// no photo open: none can change anything, none is marked, no preset row's option can be
/// applied, and a choice lists every option. A preset row reads the option the defaults
/// match and a view's coordinates are the module's view of them, both on the no-photo
/// image ([`PhotoState::image`]); a row whose module cannot say reads Custom, and a view
/// whose module cannot is left out, as the session leaves it out.
fn untouched(modules: &[&dyn Module]) -> Values {
    let image = PhotoState::None.image();
    let states = modules.iter().map(|module| {
        let definition = module.definition();
        let defaults = definition.defaults();
        let controls: Vec<ControlState> = definition
            .controls
            .iter()
            .map(|control| ControlState {
                control: control.id,
                value: control.default(),
                default: control.default(),
                mark: None,
                available: false,
                options: Vec::new(),
                range: edit::range(*module, control.id, &defaults).map(|(range, _)| range),
            })
            .collect();
        let presets = definition
            .presets
            .iter()
            .map(|row| PresetState {
                preset: row.id,
                matching: catch_unwind(AssertUnwindSafe(|| {
                    module.matching(row.id, &defaults, &image)
                }))
                .unwrap_or_default(),
                available: Vec::new(),
            })
            .collect();
        let mut views = Vec::new();
        for view in definition.views {
            let viewed = catch_unwind(AssertUnwindSafe(|| module.view(view.id, &defaults, &image)));
            let Ok(Some(coordinates)) = viewed else {
                continue;
            };
            if coordinates.len() != view.coordinates.len() {
                continue;
            }
            for (coordinate, value) in view.coordinates.iter().zip(coordinates) {
                views.push(CoordinateState {
                    view: view.id,
                    coordinate: coordinate.id,
                    value,
                    default: value,
                    mark: None,
                    available: false,
                });
            }
        }
        ModuleState {
            module: definition.id,
            edited: false,
            controls,
            presets,
            views,
        }
    });
    Values {
        modules: states.collect(),
        settings: Vec::new(),
        edited: false,
        through: 0,
        extent: [0, 0],
    }
}

/// Asks the jobs window to show, unless the event changed nothing it shows.
fn jobs_reaction(show: jobs::Show, reactions: &mut Vec<Reaction>) {
    if show != jobs::Show::Nothing {
        reactions.push(Reaction::Jobs(show));
    }
}

pub enum PhotoState {
    None,
    /// Asked for by the open the session numbered `open`
    /// ([`Session::send`](apertrust_engine::session::Session::send)).
    Opening {
        path: PathBuf,
        open: u64,
    },
    Open(Photo),
    /// It could not be opened, as `message` says.
    Failed {
        path: PathBuf,
        message: String,
    },
}

impl PhotoState {
    /// The image context a module's view of the pending values reads ([`Pending::show`]):
    /// the open photo's, as the session reports it in `Photo` (decision 27), so a view
    /// that reads the camera, as White balance's temperature and tint read its matrix,
    /// shows its pending values as the session will. The pending display's availability
    /// reads it too (decision 30), where the pending values move the module's answer.
    pub fn image(&self) -> ImageContext {
        match self {
            PhotoState::Open(photo) => (*photo.image).clone(),
            _ => ImageContext::full(0, 0),
        }
    }

    /// The photo's file, open or not: where the open dialog starts.
    pub fn path(&self) -> Option<&Path> {
        match self {
            PhotoState::None => None,
            PhotoState::Opening { path, .. } | PhotoState::Failed { path, .. } => Some(path),
            PhotoState::Open(photo) => Some(&photo.path),
        }
    }

    /// Whether `event` is left over from before the open under way, and so not the new
    /// photo's: while a photo opens, an earlier open's photo or failure, and every values
    /// report, frame and scopes reading until its own `Opened`.
    fn stale(&self, event: &Event) -> bool {
        let PhotoState::Opening { open, .. } = self else {
            return false;
        };
        match event {
            Event::Opened(photo) => photo.open != *open,
            Event::Failed(SessionError::Open { open: failed, .. }) => failed != open,
            Event::Values(_) | Event::Frame(_) | Event::Scopes(_) => true,
            _ => false,
        }
    }
}

/// How long a write of the open photo's edit may go unconfirmed before the top panel says
/// Saving… (`docs/gui.md`, The edit, saved).
pub const SAVING_AFTER: Duration = Duration::from_secs(2);

/// Whether the open photo's edit is on disk as it stands, from what the app sent and the
/// session reported, for the top panel's save state: nothing while saving works, Saving…
/// while a write is unconfirmed [`SAVING_AFTER`] it started, and a failed write's reason
/// kept until one succeeds. Once another photo is asked for, the photo left is another
/// photo's: its last write's success changes nothing, and its failure is kept apart
/// ([`Saving::left`]), since that edit is then lost.
#[derive(Debug, Default)]
pub struct Saving {
    /// The open photo's edit file; none while another photo opens.
    sidecar: Option<PathBuf>,
    /// Whether the photo open is open for viewing only: its edit is never saved.
    view_only: bool,
    /// The commands sent that change the edit and that no write includes yet; a refused
    /// command is taken out ([`Saving::refused`]), and so is a completed change that
    /// changed nothing, which no write follows ([`Saving::reported`]).
    unsaved: Vec<Sent>,
    /// The session's last values report, which tells a change that changed nothing.
    report: Option<Values>,
    /// Why the last write of the open photo's edit failed, until one succeeds.
    pub failure: Option<String>,
    /// The edit file of a photo no longer open whose last write failed, and why: that
    /// edit is neither on disk nor in memory. Kept over the photos opened after it, until
    /// another is asked for or a write of that file succeeds.
    pub left: Option<(PathBuf, String)>,
}

/// A command sent that changes the edit.
#[derive(Debug)]
struct Sent {
    number: u64,
    /// When it was sent, where it completes a change, which the session saves at once: a
    /// write starts with it. A gesture's change waits for the one that ends it.
    writes: Option<Instant>,
}

impl Saving {
    /// Another photo is asked for: the open one's reports are now another photo's.
    fn leaving(&mut self) {
        self.unsaved.clear();
        self.report = None;
        self.sidecar = None;
        self.view_only = false;
        self.failure = None;
        self.left = None;
    }

    /// `photo` opened, for viewing only where `view_only` says so: then no change or
    /// write follows, and the save state says nothing.
    fn opened(&mut self, photo: &Path, view_only: bool) {
        self.unsaved.clear();
        self.report = None;
        self.view_only = view_only;
        self.sidecar = Some(sidecar::path_for(photo));
        self.failure = None;
    }

    /// What the top panel's save state shows at `now`.
    pub fn state(&self, now: Instant) -> SaveState<'_> {
        if self.sidecar.is_none() || self.view_only {
            return SaveState::None;
        }
        if let Some(why) = &self.failure {
            return SaveState::Failed(why);
        }
        match self.saving_from() {
            Some(from) if from <= now => SaveState::Saving,
            _ => SaveState::Saved,
        }
    }

    /// When Saving… is to show, while it is still to come: the app asks for a repaint
    /// then.
    pub fn saving_from(&self) -> Option<Instant> {
        let started = self.unsaved.iter().filter_map(|sent| sent.writes).min()?;
        (self.failure.is_none()).then_some(started + SAVING_AFTER)
    }

    /// Whether a write the session was handed for the open photo is still unconfirmed:
    /// Quit waits for it, to know whether it failed.
    pub fn writing(&self) -> bool {
        self.unsaved.iter().any(|sent| sent.writes.is_some())
    }

    /// Why the last write failed, the open photo's or, else, that of the photo left.
    pub fn failed(&self) -> Option<&str> {
        (self.failure.as_deref()).or(self.left.as_ref().map(|(_, why)| why.as_str()))
    }

    /// The command numbered `number`, which changes the edit, was sent at `now`;
    /// `completes` where it completes a change, which the session saves at once.
    pub fn sent(&mut self, number: u64, completes: bool, now: Instant) {
        if self.sidecar.is_some() && !self.view_only {
            let writes = completes.then_some(now);
            self.unsaved.push(Sent { number, writes });
        }
    }

    /// The session reported the open photo's values, `values`. Where the commands it adds
    /// changed nothing, the session hands over no write for them, unless a change before
    /// them is still to write (a gesture's, written with the change that completes it):
    /// no write is waited for them.
    fn reported(&mut self, values: &Values) {
        let before = self.report.as_ref().map_or(0, |report| report.through);
        let same = self.report.as_ref().is_some_and(|report| {
            (report.modules == values.modules)
                && (report.settings == values.settings)
                && (report.edited, report.extent) == (values.edited, values.extent)
        });
        let earlier = self.unsaved.iter().rfind(|sent| sent.number <= before);
        if same && earlier.is_none_or(|sent| sent.writes.is_some()) {
            let added = |sent: &Sent| sent.number > before && sent.number <= values.through;
            self.unsaved.retain(|sent| !added(sent));
        }
        self.report = Some(values.clone());
    }

    fn saved(&mut self, path: &Path, through: u64) {
        if self.sidecar.as_deref() != Some(path) {
            if self.left.as_ref().is_some_and(|(left, _)| left == path) {
                self.left = None;
            }
            return;
        }
        self.failure = None;
        self.unsaved.retain(|sent| sent.number > through);
    }

    /// The command numbered `number` was refused: it changed nothing, so no write follows
    /// it.
    fn refused(&mut self, number: u64) {
        self.unsaved.retain(|sent| sent.number != number);
    }

    /// The engine has ended: nothing sent will be written or reported.
    fn stopped(&mut self) {
        self.unsaved.clear();
    }

    fn write_failed(&mut self, path: &Path, through: u64, why: &str) {
        if self.sidecar.as_deref() != Some(path) {
            self.left = Some((path.to_owned(), why.to_owned()));
            return;
        }
        // The write tried is over; a later change, or Try again, writes again.
        self.unsaved.retain(|sent| sent.number > through);
        self.failure = Some(why.to_owned());
    }
}

/// The engine's report with the changes sent after the commands it includes shown over
/// it in the order they were sent, so a slider never jumps back to a value the engine
/// reports late. Changes the report includes are no longer pending.
pub fn reported(
    mut reported: Values,
    pending: &mut Vec<(u64, Pending)>,
    modules: &[&dyn Module],
    image: &ImageContext,
) -> Values {
    pending.retain(|(number, _)| *number > reported.through);
    for (_, change) in pending.iter() {
        show(&mut reported, change, modules, image);
    }
    reported
}

/// Shows a change sent in `values` as the session will report it ([`Pending::show`]): a
/// control set shows as kept, as the session keeps it, one reset its default, released,
/// and a lossless view's coordinates and controls move together. `image` is what the
/// module's view reads.
pub fn show(values: &mut Values, change: &Pending, modules: &[&dyn Module], image: &ImageContext) {
    let id = change.module();
    let module = modules.iter().find(|module| module.definition().id == id);
    let state = values.modules.iter_mut().find(|state| state.module == id);
    let (Some(module), Some(state)) = (module, state) else {
        return;
    };
    change.show(*module, state, image);
    state.edited = state.controls.iter().any(|c| c.value != c.default);
    values.edited |= state.edited;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use apertrust_engine::session::{ControlValue, Mark};

    /// The save state at `t` plus `seconds`.
    fn at(saving: &Saving, t: Instant, seconds: f32) -> SaveState<'_> {
        saving.state(t + Duration::from_secs_f32(seconds))
    }

    #[test]
    fn nothing_shows_at_rest_and_saving_only_once_a_write_is_two_seconds_unconfirmed() {
        let (photo, other) = (Path::new("/photos/a.ARW"), Path::new("/photos/b.ARW"));
        let (edit, other_edit) = (sidecar::path_for(photo), sidecar::path_for(other));
        let t = Instant::now();
        let mut saving = Saving::default();
        assert_eq!(at(&saving, t, 0.0), SaveState::None);
        saving.opened(photo, false);
        assert_eq!(at(&saving, t, 9.0), SaveState::Saved);
        // A drag's changes start no write: nothing shows, however long it lasts.
        saving.sent(2, false, t);
        assert_eq!(
            (at(&saving, t, 9.0), saving.writing()),
            (SaveState::Saved, false)
        );
        assert_eq!(saving.saving_from(), None);
        // Its end, and a change after it, start writes; the first is confirmed late.
        saving.sent(3, true, t);
        saving.sent(5, true, t + Duration::from_secs(1));
        assert!(saving.writing());
        assert_eq!(saving.saving_from(), Some(t + SAVING_AFTER));
        assert_eq!(at(&saving, t, 1.9), SaveState::Saved);
        assert_eq!(at(&saving, t, 2.0), SaveState::Saving);
        // Command 3's write: command 5's is still unconfirmed, since t + 1 s.
        saving.saved(&edit, 3);
        assert_eq!(at(&saving, t, 2.5), SaveState::Saved);
        assert_eq!(at(&saving, t, 3.0), SaveState::Saving);
        // Another photo's write changes nothing, nor does its failure the open photo's.
        saving.saved(&other_edit, 9);
        saving.write_failed(&other_edit, 9, "disk full");
        assert_eq!(at(&saving, t, 3.0), SaveState::Saving);
        saving.saved(&edit, 5);
        assert_eq!(
            (at(&saving, t, 9.0), saving.writing()),
            (SaveState::Saved, false)
        );
        // The old photo's last write, after the next one opens, changes nothing either.
        saving.leaving();
        saving.opened(other, false);
        saving.sent(7, true, t);
        saving.saved(&edit, 7);
        assert_eq!(at(&saving, t, 9.0), SaveState::Saving);
    }

    #[test]
    fn a_failed_write_shows_until_a_later_one_succeeds() {
        let photo = Path::new("/photos/a.ARW");
        let edit = sidecar::path_for(photo);
        let t = Instant::now();
        let mut saving = Saving::default();
        saving.opened(photo, false);
        saving.sent(3, true, t);
        saving.write_failed(&edit, 3, "disk full");
        // The write is over: Quit waits for nothing, and Saving… never shows over it.
        assert_eq!(at(&saving, t, 0.0), SaveState::Failed("disk full"));
        assert!(!saving.writing());
        assert_eq!(saving.failed(), Some("disk full"));
        // A later change's write: the warning stays until it succeeds, with no Saving….
        saving.sent(4, true, t);
        assert_eq!(saving.saving_from(), None);
        assert_eq!(at(&saving, t, 9.0), SaveState::Failed("disk full"));
        saving.saved(&edit, 4);
        assert_eq!(at(&saving, t, 9.0), SaveState::Saved);
        assert_eq!(saving.failed(), None);
    }

    #[test]
    fn a_failed_last_write_of_the_photo_left_is_kept_over_the_next_one() {
        let (a, b) = (Path::new("/photos/a.ARW"), Path::new("/photos/b.ARW"));
        let t = Instant::now();
        let mut saving = Saving::default();
        saving.opened(a, false);
        saving.sent(3, true, t);
        // B is asked for, and A's last write fails before B opens: kept as A's, not
        // cleared as B opens, and B's own state is untouched.
        saving.leaving();
        saving.write_failed(&sidecar::path_for(a), 3, "disk full");
        saving.opened(b, false);
        let left = (sidecar::path_for(a), "disk full".to_owned());
        assert_eq!(saving.left.as_ref(), Some(&left));
        assert_eq!(
            (at(&saving, t, 9.0), &saving.failure),
            (SaveState::Saved, &None)
        );
        // Quit asks about it all the same.
        assert_eq!(saving.failed(), Some("disk full"));
        // Or after B opens.
        saving.left = None;
        saving.write_failed(&sidecar::path_for(a), 3, "disk full");
        assert_eq!(saving.left.as_ref(), Some(&left));
        // A later write of that file, or another photo asked for, puts it away.
        saving.saved(&sidecar::path_for(b), 9);
        assert!(saving.left.is_some());
        saving.saved(&sidecar::path_for(a), 9);
        assert!(saving.left.is_none());
        saving.write_failed(&sidecar::path_for(a), 3, "disk full");
        saving.leaving();
        assert!(saving.left.is_none());
    }

    #[test]
    fn a_photo_open_for_viewing_only_shows_nothing() {
        let photo = Path::new("/photos/a.ARW");
        let t = Instant::now();
        let mut saving = Saving::default();
        saving.opened(photo, true);
        assert_eq!(at(&saving, t, 9.0), SaveState::None);
        // Nothing it could be sent is waited for.
        saving.sent(3, true, t);
        assert_eq!(
            (at(&saving, t, 9.0), saving.writing()),
            (SaveState::None, false)
        );
        // The next photo, editable, is saved.
        saving.leaving();
        saving.opened(Path::new("/photos/b.ARW"), false);
        assert_eq!(at(&saving, t, 9.0), SaveState::Saved);
    }

    #[test]
    fn a_completed_change_that_changed_nothing_waits_for_no_write() {
        let photo = Path::new("/photos/a.ARW");
        let edit = sidecar::path_for(photo);
        let t = Instant::now();
        let mut saving = Saving::default();
        saving.opened(photo, false);
        saving.reported(&report(0.5, 1));
        // A reset of what is already at its default: the values stay as they were.
        saving.sent(2, true, t);
        saving.reported(&report(0.5, 2));
        assert!(!saving.writing());
        assert_eq!(at(&saving, t, 9.0), SaveState::Saved);
        // A drag's last value, done, the same as the one before it: the drag's change is
        // still to write, and is written with it.
        saving.sent(3, false, t);
        saving.reported(&report(0.8, 3));
        saving.sent(4, true, t);
        saving.reported(&report(0.8, 4));
        assert!(saving.writing());
        saving.saved(&edit, 4);
        assert!(!saving.writing());
        // A write under way, then a change that changes nothing: only the write is
        // waited for.
        saving.sent(5, true, t);
        saving.reported(&report(1.0, 5));
        saving.sent(6, true, t);
        saving.reported(&report(1.0, 6));
        saving.saved(&edit, 5);
        assert!(!saving.writing());
        // The engine ending, nothing is waited for.
        saving.sent(7, true, t);
        saving.stopped();
        assert!(!saving.writing());
    }

    #[test]
    fn while_a_photo_opens_what_an_earlier_open_left_is_dropped() {
        let photo = |open| Photo {
            path: PathBuf::from(format!("{open}.ARW")),
            open,
            camera: String::new(),
            width: 84,
            height: 62,
            decode_ms: 0.0,
            upload_ms: 0.0,
            view_only: None,
            exposure_time: None,
            aperture: None,
            focal_length: None,
            image: Box::new(ImageContext::full(84, 62)),
        };
        let values = Event::Values(Values {
            modules: Vec::new(),
            settings: Vec::new(),
            edited: false,
            through: 4,
            extent: [84, 62],
        });
        let failed = |open| {
            Event::Failed(SessionError::Open {
                open,
                error: Box::new(SessionError::NotAMosaic),
            })
        };
        let opening = PhotoState::Opening {
            path: PathBuf::from("5.ARW"),
            open: 5,
        };
        assert!(opening.stale(&Event::Opened(photo(3))));
        assert!(opening.stale(&failed(3)));
        assert!(opening.stale(&values));
        assert!(!opening.stale(&Event::Opened(photo(5))));
        assert!(!opening.stale(&failed(5)));
        assert!(!opening.stale(&Event::Failed(SessionError::Stopped)));
        // A reading of the photo left, counted before it closed.
        let reading = || Event::Scopes(crate::scopes::testing::reading(true, false, false));
        assert!(opening.stale(&reading()));
        // Once it is open, everything is its own.
        let open = PhotoState::Open(photo(5));
        assert!(!open.stale(&values));
        assert!(!open.stale(&reading()));
        // A file that failed to open still says where the next dialog starts.
        let failed = PhotoState::Failed {
            path: PathBuf::from("raws/5.ARW"),
            message: String::new(),
        };
        assert_eq!(failed.path(), Some(Path::new("raws/5.ARW")));
    }

    #[test]
    fn a_value_set_stays_until_a_report_includes_it() {
        use apertrust_engine::session::{ControlState, ModuleState};

        let control = |id, value: f32, mark| ControlState {
            control: id,
            value: ControlValue::Number(value),
            default: ControlValue::Number(0.0),
            mark,
            available: true,
            options: Vec::new(),
            range: None,
        };
        let exposure = |black, black_mark, exposure, exposure_mark| ModuleState {
            module: "exposure",
            edited: black != 0.0 || exposure != 0.0,
            controls: vec![
                control("black_level", black, black_mark),
                control("exposure", exposure, exposure_mark),
            ],
            presets: Vec::new(),
            views: Vec::new(),
        };
        let report = |black, value, through| Values {
            modules: vec![exposure(black, None, value, Some(Mark::Kept))],
            settings: Vec::new(),
            edited: true,
            through,
            extent: [84, 62],
        };
        let image = ImageContext::full(84, 62);
        let mirror = |report, pending: &mut Vec<_>| {
            reported(report, pending, apertrust_modules::MODULES, &image)
        };
        let set_black = |value| Pending::Control {
            module: "exposure",
            control: "black_level",
            value: ControlValue::Number(value),
        };
        // A drag sent commands 4 to 6, then Delete reset exposure as command 7; the engine
        // reports command 4 first, then every one.
        let mut pending = vec![
            (4, set_black(0.05)),
            (5, set_black(0.1)),
            (6, set_black(0.2)),
            (
                7,
                Pending::Reset {
                    module: "exposure",
                    control: "exposure",
                },
            ),
        ];
        let shown = mirror(report(0.05, 1.0, 4), &mut pending);
        assert_eq!(shown.modules, [exposure(0.2, Some(Mark::Kept), 0.0, None)]);
        assert_eq!(pending.len(), 3);
        let shown = mirror(report(0.2, 0.0, 7), &mut pending);
        assert_eq!(shown.modules, [exposure(0.2, None, 0.0, Some(Mark::Kept))]);
        assert!(pending.is_empty());

        // The marker and the header's reset show at once too.
        let mut pending = vec![
            (
                8,
                Pending::Keep {
                    module: "exposure",
                    control: "exposure",
                    kept: false,
                },
            ),
            (9, Pending::ResetModule { module: "exposure" }),
        ];
        let shown = mirror(report(0.2, 1.0, 7), &mut pending[..1].to_vec());
        assert_eq!(shown.modules, [exposure(0.2, None, 1.0, None)]);
        let shown = mirror(report(0.2, 1.0, 7), &mut pending);
        assert_eq!(shown.modules, [exposure(0.0, None, 0.0, None)]);
    }

    // Feeding `apply` event sequences: the mirror and the reactions it returns.

    pub(crate) fn photo(open: u64) -> Photo {
        Photo {
            path: PathBuf::from("/photos/a.ARW"),
            open,
            camera: String::new(),
            width: 84,
            height: 62,
            decode_ms: 0.0,
            upload_ms: 0.0,
            view_only: None,
            exposure_time: None,
            aperture: None,
            focal_length: None,
            image: Box::new(ImageContext::full(84, 62)),
        }
    }

    /// The exposure module with Exposure at `exposure`, reported after `through` commands.
    pub(crate) fn report(exposure: f32, through: u64) -> Values {
        use apertrust_engine::session::{ControlState, ModuleState};
        Values {
            modules: vec![ModuleState {
                module: "exposure",
                edited: exposure != 0.0,
                controls: vec![ControlState {
                    control: "exposure",
                    value: ControlValue::Number(exposure),
                    default: ControlValue::Number(0.0),
                    mark: None,
                    available: true,
                    options: Vec::new(),
                    range: None,
                }],
                presets: Vec::new(),
                views: Vec::new(),
            }],
            settings: Vec::new(),
            edited: exposure != 0.0,
            through,
            extent: [84, 62],
        }
    }

    fn set_exposure(value: f32) -> Pending {
        Pending::Control {
            module: "exposure",
            control: "exposure",
            value: ControlValue::Number(value),
        }
    }

    /// The Exposure the widgets show.
    fn shown(mirror: &Mirror) -> ControlValue {
        mirror.values.modules[0].controls[0].value.clone()
    }

    pub(crate) fn mirror() -> Mirror {
        Mirror::new(apertrust_modules::MODULES, jobs::Mirror::new(None))
    }

    /// A change sent as `number`, as the app sends it: shown at once, and the edit not
    /// saved until a write includes it.
    fn send(mirror: &mut Mirror, number: u64, value: f32) {
        mirror.pend(number, set_exposure(value));
        mirror.saving.sent(number, true, Instant::now());
    }

    fn refused(command: u64, why: &str) -> Event {
        Event::Failed(SessionError::Refused {
            command,
            error: Box::new(SessionError::Gpu(why.to_owned())),
        })
    }

    #[test]
    fn a_drag_shows_its_last_value_until_the_reports_catch_up() {
        let mut m = mirror();
        let edit = sidecar::path_for(&photo(1).path);
        assert_eq!(m.apply(Event::Opened(photo(1))), []);
        assert!(!m.saving.writing());
        assert_eq!(m.apply(Event::Values(report(0.0, 1))), []);
        // A drag sends commands 2 to 4; each is shown at once.
        for (number, value) in [(2, 0.5), (3, 1.0), (4, 1.5)] {
            send(&mut m, number, value);
        }
        assert_eq!(shown(&m), ControlValue::Number(1.5));
        assert!(m.saving.writing());
        // The engine reports command 2 only: the later two still show over it.
        assert_eq!(m.apply(Event::Values(report(0.5, 2))), []);
        assert_eq!(shown(&m), ControlValue::Number(1.5));
        assert_eq!(m.pending.len(), 2);
        // A write of the first three leaves the edit not saved; the last makes it so.
        let saved = |through| Event::Saved {
            path: edit.clone(),
            through,
        };
        assert_eq!(m.apply(saved(3)), []);
        assert!(m.saving.writing());
        assert_eq!(m.apply(Event::Values(report(1.5, 4))), []);
        assert!(m.pending.is_empty());
        assert_eq!(shown(&m), ControlValue::Number(1.5));
        assert_eq!(m.apply(saved(4)), []);
        assert!(!m.saving.writing());
    }

    #[test]
    fn a_frame_replaces_the_last_and_clears_the_failure() {
        use apertrust_engine::compute::Timings;
        use apertrust_engine::gpu::{GpuContext, GpuOptions};
        let Ok(gpu) = pollster::block_on(GpuContext::new(&GpuOptions::default())) else {
            eprintln!("no GPU adapter: a frame needs a buffer");
            return;
        };
        let frame = |through| Frame {
            buffer: gpu.device.create_buffer(&eframe::wgpu::BufferDescriptor {
                label: None,
                size: 16,
                usage: eframe::wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            width: 2,
            height: 2,
            through,
            timings: Timings::default(),
            latency_ms: 0.0,
        };
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        // A failure stays until the next frame, which is the photo drawn again.
        m.apply(Event::Failed(SessionError::Gpu("out of memory".to_owned())));
        assert_eq!(m.failure.as_deref(), Some("the GPU: out of memory"));
        assert!(m.frame.is_none());
        assert_eq!(m.apply(Event::Frame(frame(3))), []);
        assert_eq!(m.frame.as_ref().map(|f| f.through), Some(3));
        assert_eq!(m.failure, None);
        m.apply(Event::Frame(frame(5)));
        assert_eq!(m.frame.as_ref().map(|f| f.through), Some(5));
    }

    #[test]
    fn a_refused_command_stops_showing_and_stops_being_waited_for() {
        let mut m = mirror();
        let edit = sidecar::path_for(&photo(1).path);
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        // Command 2 is written; command 3 is refused, as the session reports before the
        // values that include it.
        send(&mut m, 2, 1.0);
        send(&mut m, 3, 7.0);
        assert_eq!(shown(&m), ControlValue::Number(7.0));
        assert_eq!(m.apply(refused(3, "out of range")), []);
        assert_eq!(m.failure.as_deref(), Some("the GPU: out of range"));
        assert_eq!(m.pending.len(), 1);
        // Command 2 is still waited for.
        assert!(m.saving.writing());
        m.apply(Event::Values(report(1.0, 3)));
        assert_eq!(shown(&m), ControlValue::Number(1.0));
        m.apply(Event::Saved {
            path: edit,
            through: 2,
        });
        assert!(!m.saving.writing());

        // A refusal of the only change the edit waited for leaves it saved: no write
        // follows a change that changed nothing.
        send(&mut m, 4, 7.0);
        assert!(m.saving.writing());
        m.apply(refused(4, "no"));
        assert_eq!((m.saving.writing(), m.pending.len()), (false, 0));
    }

    #[test]
    fn a_failed_write_is_told_until_one_succeeds() {
        let mut m = mirror();
        let edit = sidecar::path_for(&photo(1).path);
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        send(&mut m, 2, 1.0);
        let failed = SessionError::Save {
            path: edit.clone(),
            through: 2,
            source: std::io::Error::other("disk full"),
        };
        assert_eq!(m.apply(Event::Failed(failed)), []);
        assert!(!m.saving.writing());
        let why = m.saving.failure.as_deref();
        assert!(why.is_some_and(|why| why.contains("disk full")));
        // The failed save is the saving's, not the central panel's failure.
        assert_eq!(m.failure, None);
        // A refusal does not hide it.
        m.apply(refused(2, "no"));
        assert!(m.saving.failure.is_some());
        m.apply(Event::Saved {
            path: edit,
            through: 2,
        });
        let saving = (m.saving.state(Instant::now()), m.saving.failure.as_ref());
        assert_eq!(saving, (SaveState::Saved, None));
    }

    #[test]
    fn a_failed_open_is_the_photo_s_and_a_failure_later_is_the_central_panel_s() {
        let mut m = mirror();
        m.opening(PathBuf::from("/photos/b.ARW"), 4);
        // The photo left on its way, the values of the old one: dropped.
        assert_eq!(m.apply(Event::Opened(photo(3))), []);
        assert_eq!(m.apply(Event::Values(report(1.0, 9))), []);
        assert_eq!(m.values, m.untouched);
        let failed = Event::Failed(SessionError::Open {
            open: 4,
            error: Box::new(SessionError::NotAMosaic),
        });
        assert_eq!(m.apply(failed), []);
        match &m.photo {
            PhotoState::Failed { path, message } => {
                assert_eq!(path, Path::new("/photos/b.ARW"));
                assert!(message.ends_with("only 16-bit mosaics can be developed so far"));
            }
            _ => panic!("the open failed"),
        }
        assert_eq!(m.failure, None);
        // With a photo open, a failure is for the central panel; a scopes failure shows
        // nothing, never the frame.
        m.apply(Event::Opened(photo(5)));
        let reading = crate::scopes::testing::reading(true, false, false);
        m.apply(Event::Scopes(reading));
        m.apply(Event::Failed(SessionError::Scopes("no staging".to_owned())));
        assert_eq!(m.failure, None);
        m.apply(Event::Failed(SessionError::Gpu("lost".to_owned())));
        assert_eq!(m.failure.as_deref(), Some("the GPU: lost"));
    }

    #[test]
    fn a_job_s_progress_to_its_end_asks_the_jobs_window_to_follow() {
        use apertrust_engine::jobs::{Queue, State};
        use apertrust_engine::session::JobEvent;
        use apertrust_ui::components::JobPhase;
        let words = apertrust_ui::Translator::new(apertrust_ui::Locale::EnGb);
        let id = jobs::tests::ids(1)[0];
        let mut m = mirror();
        // A job the user started opens the window; its progress and its end update it.
        let added = jobs::tests::added(id, Queue::UserExport, Some(7));
        assert_eq!(m.apply(added), [Reaction::Jobs(jobs::Show::Open)]);
        let progress = Event::Job {
            event: JobEvent::Progress {
                job: id,
                fraction: 0.5,
            },
            command: None,
        };
        assert_eq!(m.apply(progress), [Reaction::Jobs(jobs::Show::Update)]);
        assert_eq!(m.jobs.view(&words)[0].phase, JobPhase::Queued);
        let ended = jobs::tests::ended(id, State::Finished, None);
        assert_eq!(m.apply(ended), [Reaction::Jobs(jobs::Show::Update)]);
        assert_eq!(m.jobs.view(&words)[0].phase, JobPhase::Finished);
        // A system job turns the spinner and never opens the window.
        let other = jobs::tests::ids(1)[0];
        let thumbnails = jobs::tests::added(other, Queue::SystemBackground, None);
        assert_eq!(m.apply(thumbnails), [Reaction::Jobs(jobs::Show::Update)]);
    }

    #[test]
    fn the_engine_stopping_ends_the_jobs_and_the_open_under_way() {
        use apertrust_engine::jobs::Queue;
        use apertrust_ui::components::JobPhase;
        let words = apertrust_ui::Translator::new(apertrust_ui::Locale::EnGb);
        let id = jobs::tests::ids(1)[0];
        let mut m = mirror();
        m.apply(jobs::tests::added(id, Queue::UserExport, Some(1)));
        m.opening(PathBuf::from("/photos/b.ARW"), 2);
        let reactions = m.apply(Event::Failed(SessionError::Stopped));
        assert_eq!(reactions, [Reaction::Jobs(jobs::Show::Update)]);
        assert!(matches!(m.photo, PhotoState::Failed { .. }));
        assert_eq!(m.jobs.view(&words)[0].phase, JobPhase::Failed);
        // No job left to end: nothing more to show.
        assert_eq!(m.apply(Event::Failed(SessionError::Stopped)), []);
    }

    #[test]
    fn an_export_preview_shows_only_if_newer_than_the_dialog_s_and_the_last_shown() {
        let preview = || {
            Box::new(ExportPreview {
                problem: None,
                sequence_added: false,
                not_here: Vec::new(),
                names: Vec::new(),
                size: None,
                folders: Vec::new(),
            })
        };
        let answer = |through| Event::ExportPreview {
            through,
            preview: preview(),
        };
        let shown = |through| {
            vec![Reaction::Preview {
                through,
                preview: preview(),
            }]
        };
        let mut m = mirror();
        // The dialog opened and asked for command 5: an answer to an earlier dialog's
        // command shows nothing.
        m.previews_from(5);
        assert_eq!(m.apply(answer(4)), []);
        assert_eq!(m.apply(answer(5)), shown(5));
        // One that had finished, though a newer was asked for, arrives after it.
        assert_eq!(m.apply(answer(7)), shown(7));
        assert_eq!(m.apply(answer(6)), []);
        assert_eq!(m.apply(answer(8)), shown(8));
        // A new dialog starts over.
        m.previews_from(12);
        assert_eq!(m.apply(answer(9)), []);
    }

    /// Whether `values` are every module's, untouched and unavailable, as with no photo.
    fn untouched_all(values: &Values) -> bool {
        let ids: Vec<&str> = apertrust_modules::MODULES
            .iter()
            .map(|module| module.definition().id)
            .collect();
        let shown: Vec<&str> = values.modules.iter().map(|state| state.module).collect();
        let controls = values.modules.iter().flat_map(|state| &state.controls);
        let coordinates = values.modules.iter().flat_map(|state| &state.views);
        let presets = values.modules.iter().flat_map(|state| &state.presets);
        ids == shown
            && !values.edited
            && controls.clone().all(|c| c.value == c.default)
            && controls.clone().all(|c| !c.available && c.mark.is_none())
            && coordinates
                .clone()
                .all(|c| !c.available && c.mark.is_none())
            && presets.clone().all(|p| p.available.is_empty())
    }

    #[test]
    fn with_no_photo_every_module_is_untouched_and_takes_no_edit() {
        let m = mirror();
        assert_eq!(m.shown(), Shown::Nothing);
        assert!(!m.takes_edits() && !m.shown().editable());
        assert!(untouched_all(&m.values));
        // Each control at its definition's default, whatever the photo would make it.
        for (module, state) in apertrust_modules::MODULES.iter().zip(&m.values.modules) {
            for (control, shown) in module.definition().controls.iter().zip(&state.controls) {
                assert_eq!(shown.value, control.default(), "{}", control.id);
            }
        }
        // A lossless view's coordinates are there to draw, as White balance's temperature
        // and tint are.
        let temperature = m.values.modules.iter().find(|s| s.module == "temperature");
        assert!(temperature.is_some_and(|state| !state.views.is_empty()));
    }

    #[test]
    fn a_switch_shows_no_photo_until_the_next_one_s_values_arrive() {
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(1.5, 1)));
        m.apply(Event::Scopes(crate::scopes::testing::reading(
            true, false, false,
        )));
        assert_eq!(m.shown(), Shown::Photo { editable: true });
        assert!(m.takes_edits());
        // Another photo is asked for: at once no photo, every module untouched, no scopes,
        // and nothing sent for them.
        m.pend(2, set_exposure(2.0));
        m.opening(PathBuf::from("/photos/b.ARW"), 3);
        assert_eq!(m.shown(), Shown::Nothing);
        assert!(!m.takes_edits());
        assert!(untouched_all(&m.values) && m.pending.is_empty());
        assert!(m.scopes.measured().is_none());
        // What the photo left still sends, and the next one's Opened, change nothing shown.
        m.apply(Event::Values(report(3.0, 2)));
        m.apply(Event::Scopes(crate::scopes::testing::reading(
            false, true, false,
        )));
        m.apply(Event::Opened(photo(3)));
        assert_eq!(m.shown(), Shown::Nothing);
        assert!(untouched_all(&m.values) && m.scopes.measured().is_none());
        // Its values replace them in one apply.
        m.apply(Event::Values(report(0.5, 3)));
        assert_eq!(m.shown(), Shown::Photo { editable: true });
        assert_eq!(m.values.modules.len(), 1);
        assert_eq!(shown(&m), ControlValue::Number(0.5));
        assert!(m.takes_edits());
    }

    #[test]
    fn a_photo_open_for_viewing_only_takes_no_edit_and_its_resets_are_disabled() {
        let mut m = mirror();
        let mut view_only = photo(1);
        view_only.view_only = Some(sidecar::ViewOnly {
            build: None,
            newer: sidecar::Newer::Version(9),
        });
        m.apply(Event::Opened(view_only));
        m.apply(Event::Values(report(0.0, 1)));
        assert_eq!(m.shown(), Shown::Photo { editable: false });
        assert!(!m.takes_edits() && !m.shown().editable());
        m.opening(PathBuf::from("/photos/b.ARW"), 2);
        m.apply(Event::Opened(photo(2)));
        m.apply(Event::Values(report(0.0, 2)));
        assert_eq!(m.shown(), Shown::Photo { editable: true });
    }

    #[test]
    fn a_failed_open_leaves_no_photo_in_the_side_panel() {
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(1.5, 1)));
        m.apply(Event::Scopes(crate::scopes::testing::reading(
            true, false, false,
        )));
        m.opening(PathBuf::from("/photos/b.ARW"), 2);
        let failed = Event::Failed(SessionError::Open {
            open: 2,
            error: Box::new(SessionError::NotAMosaic),
        });
        m.apply(failed);
        assert!(matches!(m.photo, PhotoState::Failed { .. }));
        assert_eq!(m.shown(), Shown::Nothing);
        assert!(untouched_all(&m.values) && m.pending.is_empty());
        assert!(m.scopes.measured().is_none());
        // The next photo opening shows no photo until its values arrive.
        m.opening(PathBuf::from("/photos/c.ARW"), 3);
        assert_eq!(m.shown(), Shown::Nothing);
        m.apply(Event::Opened(photo(3)));
        assert_eq!(m.shown(), Shown::Nothing);
        m.apply(Event::Values(report(0.0, 3)));
        assert_eq!(m.shown(), Shown::Photo { editable: true });
    }

    #[test]
    fn opening_another_photo_forgets_the_last_one_s_frame_failure_and_save() {
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Failed(SessionError::Gpu("lost".to_owned())));
        m.opening(PathBuf::from("/photos/b.ARW"), 2);
        assert_eq!(m.failure, None);
        assert!(m.frame.is_none());
        assert_eq!(m.saving.state(Instant::now()), SaveState::None);
        assert!(matches!(m.photo, PhotoState::Opening { open: 2, .. }));
        assert_eq!(m.apply(Event::Opened(photo(2))), []);
        assert!(matches!(m.photo, PhotoState::Open(_)));
        assert!(!m.saving.writing());
    }

    use apertrust_engine::jobs::{JobEvent, Message, Queue, State};
    use apertrust_engine::session::LibraryEvent;
    use apertrust_engine::session::library::{EditState, Photo as Listed};

    fn listed(open: u64, names: &[&str], done: bool) -> Event {
        let photos = names
            .iter()
            .map(|name| Listed {
                name: (*name).to_owned(),
                size: 1,
                modified: None,
                edit: EditState::None,
            })
            .collect();
        Event::Library(LibraryEvent::Listed { open, photos, done })
    }

    fn folder_opened(open: u64, folder: &str) -> Event {
        Event::Library(LibraryEvent::Opened {
            open,
            name: "Wedding".to_owned(),
            folder: PathBuf::from(folder),
        })
    }

    #[test]
    fn a_folder_opens_to_done_and_joins_the_recent_folders_once() {
        let mut m = mirror();
        m.folder.opening(PathBuf::from("/photos/Wedding"), 3);
        assert_eq!(
            m.apply(folder_opened(3, "/photos/Wedding")),
            [Reaction::FolderOpened(PathBuf::from("/photos/Wedding"))]
        );
        assert_eq!(m.apply(listed(3, &["a.ARW"], false)), []);
        assert_eq!(m.apply(listed(3, &["b.ARW"], true)), []);
        let folder::Folder::Open(listing) = &m.folder.folder else {
            panic!("open: {:?}", m.folder.folder);
        };
        assert_eq!(listing.photos.len(), 2);
        assert!(listing.done);
        // The photo is no folder's business: nothing of it changed.
        assert!(matches!(m.photo, PhotoState::None));
    }

    #[test]
    fn a_second_open_drops_the_first_one_s_late_events_and_a_failure_says_so() {
        let mut m = mirror();
        m.folder.opening(PathBuf::from("/photos/one"), 2);
        m.folder.opening(PathBuf::from("/photos/two"), 3);
        assert_eq!(m.apply(folder_opened(2, "/photos/one")), []);
        assert_eq!(m.apply(listed(2, &["late.ARW"], true)), []);
        assert!(matches!(m.folder.folder, folder::Folder::Opening { .. }));
        let failed = Event::Library(LibraryEvent::Failed {
            open: 3,
            reason: "the folder is missing".to_owned(),
        });
        assert_eq!(
            m.apply(failed),
            [Reaction::FolderFailed {
                path: PathBuf::from("/photos/two"),
                reason: "the folder is missing".to_owned()
            }]
        );
        // A close ends it all, whatever the reading still sends.
        m.folder.opening(PathBuf::from("/photos/three"), 4);
        m.apply(folder_opened(4, "/photos/three"));
        m.folder.closing(5);
        assert_eq!(m.apply(listed(4, &["late.ARW"], true)), []);
        m.apply(Event::Library(LibraryEvent::Closed { open: 5 }));
        assert_eq!(m.folder.folder, folder::Folder::None);
    }

    #[test]
    fn the_folder_s_reading_turns_the_spinner_but_never_opens_the_jobs_window() {
        let mut m = mirror();
        let job = crate::jobs::tests::ids(1)[0];
        let added = Event::Job {
            event: JobEvent::Added {
                job,
                kind: "folder",
                queue: Some(Queue::UserBackground),
                message: Message::new("job-folder"),
                cancellable: true,
            },
            command: Some(7),
        };
        m.folder.opening(PathBuf::from("/photos/Wedding"), 7);
        // Shown in the jobs' list, so the spinner turns, but nothing opens the window.
        assert_eq!(m.apply(added), [Reaction::Jobs(jobs::Show::Update)]);
        // Cancelled from the jobs window, the open is over and says it was cancelled.
        let ended = Event::Job {
            event: JobEvent::Ended {
                job,
                state: State::Cancelled,
                failure: None,
            },
            command: None,
        };
        m.apply(ended);
        assert!(matches!(
            m.folder.folder,
            folder::Folder::Failed { reason: None, .. }
        ));
    }

    #[test]
    fn the_active_cell_follows_the_photo_and_the_listing_without_a_scan_each_frame() {
        let mut m = mirror();
        m.folder.opening(PathBuf::from("/photos"), 2);
        m.opening(PathBuf::from("/photos/b.ARW"), 3);
        m.apply(folder_opened(2, "/photos"));
        // The listing's first chunk has not reached the photo yet.
        m.apply(listed(2, &["a.ARW"], false));
        assert_eq!(m.folder.active(), None);
        m.apply(listed(2, &["b.ARW", "c.ARW"], true));
        assert_eq!(m.folder.active(), Some(1));
        // Another photo of the folder opens, and then one of another folder.
        m.opening(PathBuf::from("/photos/c.ARW"), 4);
        assert_eq!(m.folder.active(), Some(2));
        m.opening(PathBuf::from("/elsewhere/c.ARW"), 5);
        assert_eq!(m.folder.active(), None);
    }

    #[test]
    fn the_open_photo_s_cell_and_the_edited_sort_follow_its_edit_at_once() {
        use apertrust_ui::components::{Sort, SortKey};
        let mut m = mirror();
        m.folder.sort(Sort {
            key: SortKey::Edited,
            descending: false,
        });
        m.folder.opening(PathBuf::from("/photos"), 2);
        m.apply(folder_opened(2, "/photos"));
        m.apply(listed(2, &["a.ARW", "b.ARW"], true));
        m.opening(PathBuf::from("/photos/a.ARW"), 3);
        m.apply(Event::Opened(photo(3)));
        let edit = |m: &Mirror| match &m.folder.folder {
            folder::Folder::Open(listing) => listing.photos[0].edit.clone(),
            other => panic!("not open: {other:?}"),
        };
        // Untouched, a.ARW comes first by its name.
        m.apply(Event::Values(report(0.0, 1)));
        assert_eq!(edit(&m), EditState::None);
        assert_eq!(m.folder.order(), [0, 1]);
        // Edited, its cell says so and it moves after the photos without an edit.
        m.apply(Event::Values(report(1.0, 2)));
        assert_eq!(edit(&m), EditState::Edited);
        assert_eq!(m.folder.order(), [1, 0]);
        assert_eq!(m.folder.active_shown(), Some(1));
        // Reset, it is back.
        m.apply(Event::Values(report(0.0, 3)));
        assert_eq!(edit(&m), EditState::None);
        assert_eq!(m.folder.order(), [0, 1]);
    }
}
