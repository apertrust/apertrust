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
//! - **What it holds:** the photo ([`PhotoState`]), the tools' values as the session last
//!   reported them with the changes sent since shown over them, what the Tools panel's
//!   editing tabs draw of them ([`Shown`]), the latest frame, the scopes' reading, whether
//!   the edit is on disk ([`Saving`]), the photo's history, which names what Undo and Redo
//!   would do ([`History`]), the active colour picker ([`pickers::Mirror`]), the jobs
//!   ([`jobs::Mirror`]), the failure the Viewer shows, the open folder
//!   ([`folder::FolderState`]), the selection and the active photo as the session last
//!   told them ([`Selection`], decision 37) and the Export tool's floor for previews.
//!   Panels read it each frame; nothing subscribes.
//! - **What it does not hold:** the app's own state (the theme, a tab, a dialog open). That
//!   changes by intents after the frame, with no event.
//! - **No photo:** before any, from the moment another is asked for until its values
//!   arrive, and after an open fails, the Tools panel draws every tool at its
//!   definition's defaults and Scopes with no graph, disabled ([`Shown::Nothing`]): never a blank
//!   panel. The photo's values replace them in one apply ([`Shown::Photo`]).
//! - **Stale events:** while a photo opens, what an earlier open left on its way is dropped
//!   ([`PhotoState::stale`]), and so is what a photo closed left ([`Mirror::closing`]); a library event naming a command other than the latest open or
//!   close is dropped by the folder ([`folder::FolderState::apply`]).
//! - **A refused command** ([`SessionError::Refused`]) names its number: its pending change
//!   stops showing, a save it would have waited for is not waited for, and a job it would
//!   have started is not waited for by quitting. The values report that follows a refusal
//!   in the same batch says what the edit holds.

use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use apertrust_engine::session::library::Thumbnail;
use apertrust_engine::session::{
    ControlState, ControlValue, CoordinateState, Event, ExportPreview, Frame, History, OpenPreview,
    Photo, PresetState, PreviewSource, Selection, SelectionCommand, SessionError, ToolState,
    Values, ViewState,
};
use apertrust_engine::{edit, sidecar};
use apertrust_plugin::ImageContext;
use apertrust_tools::ProcessingTool;
use apertrust_ui::components::SaveState;

use crate::folder::{self, FolderState};
use crate::jobs;
use crate::pickers;
use crate::scopes::Scopes;
use crate::tools::Pending;

/// What the app must do in reply to an event, that the mirror cannot: it touches egui, the
/// dialog host or the platform.
#[derive(Debug, PartialEq)]
pub enum Reaction {
    /// The jobs window is to show the jobs as the mirror holds them: open, if the event was
    /// a job the user started (decision 57). Never [`jobs::Show::Nothing`].
    Jobs(jobs::Show),
    /// The Export tool shows this preview, answering the command numbered `through`, when
    /// it is the newest the tool holds. An answer to a command before the tool was last
    /// shown, or for another photo ([`Mirror::previews_from`]), is dropped before it gets
    /// here.
    Preview {
        through: u64,
        preview: Box<ExportPreview>,
    },
    /// The folder at this path opened: it joins the recent folders.
    FolderOpened(PathBuf),
    /// The folder at this path could not be opened: its entry in the recent folders says so.
    FolderFailed { path: PathBuf, reason: String },
    /// A tool's pick measured these settings, as a measure mode's target: the app keeps them
    /// in its settings, as darktable writes its target sliders and darktablerc.
    Measured {
        tool: &'static str,
        settings: Vec<(&'static str, ControlValue)>,
    },
    /// The first live frame of the photo the open numbered `open` replaced its first
    /// picture, the mirror's of `generation`: the Viewer fades that picture out over
    /// the frame (decision 77). Only that swap: never an edit's frame, nor another photo's.
    Swapped { open: u64, generation: u64 },
    /// The session refused the view the command numbered `command` asked for, past Fit
    /// where it cannot render yet (`SessionError::Zoom`, decision 99): the zoom goes back to
    /// Fit, with nothing said over the photo, unless a later view overtook it.
    Unzoomed { command: u64 },
}

/// What the Tools panel's editing tabs draw (`docs/gui.md`, The Tools panel): the values
/// the mirror holds, and whether what the user does to them reaches the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The open photo's values and its scopes; its controls send their commands, unless the
    /// photo is open for viewing only (`editable` false).
    Photo { editable: bool },
    /// A photo opening whose values have arrived but which is not yet live (decision 77):
    /// its values, every control drawn disabled with its tooltip saying why, and Scopes
    /// with no graph, as with no photo.
    Opening,
    /// No photo, or one opening whose values have not arrived: every tool at its
    /// definition's defaults and Scopes with no graph, drawn disabled, with no marks, so the
    /// person sees at once that a click on another photo was taken.
    Nothing,
}

impl Shown {
    /// Whether the Tools panel's resets are drawn enabled.
    pub fn editable(self) -> bool {
        self == Shown::Photo { editable: true }
    }
}

/// The session as the app last heard of it.
pub struct Mirror {
    /// The tools the session was started with, whose views show the pending values.
    tools: &'static [&'static dyn ProcessingTool],
    pub photo: PhotoState,
    /// The tools' controls and the unowned parameters as the engine last reported them,
    /// with the pending ones in their place (`docs/gui.md`, A frame); with no photo, and
    /// while one opens, every tool untouched ([`untouched`]).
    pub values: Values,
    /// What the Tools panel draws of `values`.
    shown: Shown,
    /// The tools that change the photo in `values`, in the tools' order, worked out as
    /// `values` changes, never per frame: what the dot on the Tools panel's ⋮ reads
    /// ([`Mirror::edited`]).
    edited: Vec<&'static str>,
    /// The values with no photo, made once.
    untouched: Values,
    /// What the widgets set that no report includes yet, with each command's number.
    pub pending: Vec<(u64, Pending)>,
    /// What failed since the last frame, shown in the Viewer until the next one.
    pub failure: Option<String>,
    /// What the session said of the photo's history that lasts until another photo is asked
    /// for, which no frame clears: another program's change to its file that kept no move
    /// of the session's (`SessionError::MoveNotKept`).
    pub notice: Option<String>,
    /// The latest frame, which the viewport draws.
    pub frame: Option<Frame>,
    /// What the Viewer shows of the photo until its first live frame (decision 77):
    /// the largest thumbnail the folder held for it as it was asked for, then the
    /// session's picture where it is larger or rendered.
    pub picture: Option<Picture>,
    /// Raised whenever `picture` changes, so a texture made of it knows it is current.
    picture_generation: u64,
    /// Whether the photo is live: its values take changes, and its frames come.
    live: bool,
    /// Whether the photo was closed, with none asked for since: what the session still
    /// sends of it is dropped ([`Mirror::closing`]).
    closed: bool,
    /// When the photo open was asked for, from which Rendering… waits.
    asked_at: Option<Instant>,
    /// The latest scopes reading, and what the session was asked to count.
    pub scopes: Scopes,
    /// Whether the photo's edit is on disk as it is now, as the Toolbar says.
    pub saving: Saving,
    /// The open photo's history as the session last reported it, the latest by its
    /// `through` (decision 78): what Undo and Redo would do, which Edit's items name; none
    /// with no photo, and while one opens until its own arrives.
    pub history: Option<History>,
    /// The active colour picker, as the session last reported it with what the app asked
    /// since (decision 79); none with no photo, and while one opens.
    pub pickers: pickers::Mirror,
    /// What the tools' picks showed on the photo, for their patches; none with no photo.
    pub picked: pickers::Picked,
    /// The jobs the session has reported, which the spinner and the jobs window show
    /// (decision 57).
    pub jobs: jobs::Mirror,
    /// The open folder and its photos.
    pub folder: FolderState,
    /// The selection and the active photo, the session's (decision 37), as it last told
    /// them, the latest by its `through`: empty before it tells any.
    pub selection: Selection,
    /// The photos of `selection`.
    selected: HashSet<PathBuf>,
    /// The session's view and its placement (decision 99), shared by every client of the
    /// session, as it last told them, the latest by its `through`: none before it tells any.
    /// Not a photo's: another photo keeps it.
    pub view: Option<ViewState>,
    /// The orientation the session last reported for the photo, its frames', whatever change
    /// of Orientation is still pending, which `values` shows at once: what the zoom's
    /// retained centre turns with (`docs/gui.md`, The zoom's centre).
    pub orientation: crate::zoom::Orientation,
    /// The selection commands and the opens the app sent that no fact includes yet, in the
    /// order sent ([`Mirror::selecting`]).
    asked: Vec<Asked>,
    /// The number of the Previous Selection sent that no fact includes yet, unless a photo
    /// was asked for or closed since: once one does, the photo the session made active is
    /// shown ([`Mirror::follow`]).
    previous: Option<u64>,
    /// `selected` with `asked` laid over it, as the session will make it: what the cells
    /// mark ([`Mirror::is_selected`]).
    selected_shown: HashSet<PathBuf>,
    /// The disk cache's latest sizes and the Clears under way, Settings › Storage's.
    pub cache: crate::storage::Mirror,
    /// The number of the first preview the Export tool asked for as it was shown, then of
    /// the newest answer shown: an answer to an earlier command is from an earlier showing
    /// or another photo, or is outdated, and shows nothing.
    export_floor: u64,
    /// How many times what the Export tool's preview of the open photo's size depends on
    /// changed: its history, in its steps or where it ends, and its becoming live, when its
    /// output size is first known.
    edit_changes: u64,
}

impl Mirror {
    /// The tools the session was started with.
    pub fn tools(&self) -> &'static [&'static dyn ProcessingTool] {
        self.tools
    }

    /// The mirror before any event: no photo, every tool untouched, the jobs as given.
    pub fn new(tools: &'static [&'static dyn ProcessingTool], jobs: jobs::Mirror) -> Mirror {
        let untouched = untouched(tools);
        Mirror {
            tools,
            photo: PhotoState::None,
            values: untouched.clone(),
            shown: Shown::Nothing,
            edited: Vec::new(),
            untouched,
            pending: Vec::new(),
            failure: None,
            notice: None,
            frame: None,
            picture: None,
            picture_generation: 0,
            live: false,
            closed: false,
            asked_at: None,
            scopes: Scopes::default(),
            saving: Saving::default(),
            history: None,
            pickers: pickers::Mirror::default(),
            picked: pickers::Picked::default(),
            jobs,
            folder: FolderState::default(),
            selection: Selection::default(),
            selected: HashSet::new(),
            view: None,
            orientation: crate::zoom::Orientation::default(),
            asked: Vec::new(),
            previous: None,
            selected_shown: HashSet::new(),
            cache: crate::storage::Mirror::default(),
            export_floor: 0,
            edit_changes: 0,
        }
    }

    /// What the Tools panel's editing tabs draw.
    pub fn shown(&self) -> Shown {
        self.shown
    }

    /// The tools that change the photo, in the tools' order: each the session reports
    /// edited, with the changes pending shown (decision 96's rule, `ToolState::edited`):
    /// a switched tool while it is on, any other while a control is off the photo's
    /// default. None with no photo.
    pub fn edited(&self) -> &[&'static str] {
        &self.edited
    }

    /// Works out [`Mirror::edited`] again, after `values` changed.
    fn values_changed(&mut self) {
        let tools = self.values.tools.iter();
        self.edited = tools.filter(|t| t.edited).map(|t| t.tool).collect();
    }

    /// Whether a change of the edit the Tools panel's controls make is to be sent: only the
    /// open photo's, once its values have arrived, never while it is open for viewing
    /// only, and not while a Previous Selection sent may have made another photo active.
    pub fn takes_edits(&self) -> bool {
        // Not while Previous Selection may have made another photo active.
        self.shown == Shown::Photo { editable: true } && self.previous.is_none()
    }

    /// Whether Reset would change the photo: its edit holds anything, or the history's end
    /// lies before a default step, leaving a tool every photo runs not applied, which
    /// Reset applies as a step (decision 78). Its edit's `edited` alone stays what the
    /// folder marks as edited.
    pub fn resettable(&self) -> bool {
        let unapplied = |state: &ToolState| {
            !state.enabled
                && (self.tools.iter()).any(|tool| {
                    tool.definition().tool.id == state.tool && !tool.definition().switched
                })
        };
        self.values.edited || self.values.tools.iter().any(unapplied)
    }

    /// The raw at `path` is asked for by the open numbered `open`, `now`: nothing of the
    /// photo left is shown for the new one, the Tools panel shows no photo until the new
    /// one's values arrive, and the Viewer shows at once the largest thumbnail the
    /// folder holds of it, if any (decision 77).
    pub fn opening(&mut self, path: PathBuf, open: u64, now: Instant) {
        // The session selects a photo opened from outside the selection alone.
        self.ask(open, Ask::Open(path.clone()));
        // Asked for, it wins over the photo a Previous Selection sent before would show.
        self.previous = None;
        self.show_opening(path, open, now);
    }

    /// The raw at `path` shown opening for the open numbered `open`, `now`, as
    /// [`Mirror::opening`] shows it.
    fn show_opening(&mut self, path: PathBuf, open: u64, now: Instant) {
        let held = self.folder.largest_thumbnail(&path).cloned();
        self.photo = PhotoState::Opening { path, open };
        self.closed = false;
        self.folder.locate(self.photo.path());
        self.saving.leaving();
        self.frame = None;
        self.failure = None;
        self.notice = None;
        self.no_photo();
        self.live = false;
        self.asked_at = Some(now);
        self.set_picture(held.map(|image| Picture {
            image,
            rendered: false,
        }));
    }

    /// The photo is closed, File › Close's `Command::Close` sent: none is open from now on,
    /// the Tools panel shows no photo, and what the session still sends of it is dropped,
    /// but the write of its edit, which `saving` waits for as for a photo left.
    pub fn closing(&mut self) {
        // Closed, no photo a Previous Selection sent before would show comes.
        self.previous = None;
        self.show_closed();
    }

    /// Select None is sent with no photo shown: no photo a Previous Selection sent before
    /// it would show comes, as [`Mirror::closing`] says where one is shown.
    pub fn clearing(&mut self) {
        self.previous = None;
    }

    /// No photo shown, as [`Mirror::closing`] leaves it.
    fn show_closed(&mut self) {
        self.photo = PhotoState::None;
        self.closed = true;
        self.folder.locate(None);
        self.saving.leaving();
        self.frame = None;
        self.failure = None;
        self.notice = None;
        self.no_photo();
        self.live = false;
        self.asked_at = None;
        self.set_picture(None);
    }

    /// The photos selected: as the session last told them, with the app's commands it has
    /// not told yet laid over them (decision 37).
    fn shown_selection(&self) -> &HashSet<PathBuf> {
        &self.selected_shown
    }

    /// Whether `path` is selected, as the Browser's cells mark it: with the app's selection
    /// commands not yet told laid over the session's, as a value sent is ([`Mirror::selecting`]).
    pub fn is_selected(&self, path: &Path) -> bool {
        self.shown_selection().contains(path)
    }

    /// How many photos are selected, as [`Mirror::is_selected`] tells them: the viewer bar's
    /// count.
    pub fn selected_count(&self) -> usize {
        self.shown_selection().len()
    }

    /// Whether `path` is selected alone, as [`Mirror::is_selected`] tells it.
    #[cfg(test)]
    pub fn selected_alone(&self, path: &Path) -> bool {
        let photos = self.shown_selection();
        photos.len() == 1 && photos.contains(path)
    }

    /// The app sent `command` as `number`: the selection is shown as the session will make
    /// it, by its rules, laid over each fact until one includes the command, as a value
    /// sent is, so a gesture before that fact reads what the last one did. A range runs
    /// from the photo open or opening as it was sent, or from none should that open fail
    /// first, as the session applies it; taking the active photo
    /// out leaves the app to close it ([`Mirror::closing`]). All, Invert and Unedited, which
    /// need the collection, and Previous, which needs the earlier selection the session
    /// keeps, change nothing shown until the session tells them.
    pub fn selecting(&mut self, command: &SelectionCommand, number: u64) {
        let active = (self.photo.asked().map(Path::to_owned)).zip(self.photo.open());
        self.ask(number, Ask::Select(command.clone(), active));
        if *command == SelectionCommand::Previous {
            self.previous = Some(number);
        }
    }

    /// The first fact to include the Previous Selection numbered `previous` has come: the
    /// photo the session made active is shown, opening under that number as the session
    /// opened it, or none where it closed the one shown; the same photo, nothing. A failed
    /// write the photo left reported before it is kept apart, as one reported after
    /// ([`Saving::left`]).
    fn follow(&mut self, previous: u64) {
        let active = self.selection.active.clone();
        if active.as_deref() == self.photo.asked() {
            return;
        }
        let failed = self.saving.departing();
        match active {
            Some(photo) => self.show_opening(photo, previous, Instant::now()),
            None => self.show_closed(),
        }
        if failed.is_some() {
            self.saving.left = failed;
        }
    }

    /// Whether a selection command that needs the collection or the earlier selection, All,
    /// Invert, Unedited or Previous, is sent and not yet told: what the selection is waits
    /// for the session until then.
    pub fn selection_pending(&self) -> bool {
        self.asked.iter().any(|asked| {
            matches!(
                asked.ask,
                Ask::Select(
                    SelectionCommand::All
                        | SelectionCommand::Invert
                        | SelectionCommand::Unedited
                        | SelectionCommand::Previous,
                    _
                )
            )
        })
    }

    /// Whether Previous Selection is sent and not yet told: the earlier selection the last
    /// fact says the session keeps may be the one it has just put back.
    pub fn previous_pending(&self) -> bool {
        (self.asked.iter())
            .any(|asked| matches!(asked.ask, Ask::Select(SelectionCommand::Previous, _)))
    }

    /// The open numbered `open` failed: it selected nothing, and a range sent after it runs
    /// from no photo, as the session, with none active then, applies it.
    fn open_failed(&mut self, open: u64) {
        let before = self.asked.len();
        self.asked.retain(|asked| asked.number != open);
        let mut changed = self.asked.len() != before;
        // By the open's number, not its photo's: a later open of the same photo stands.
        for asked in &mut self.asked {
            if let Ask::Select(_, active) = &mut asked.ask
                && active.as_ref().is_some_and(|(_, number)| *number == open)
            {
                *active = None;
                changed = true;
            }
        }
        if changed {
            self.lay_over();
        }
    }

    /// Lays `ask`, numbered `number`, over the selection until a fact includes it.
    fn ask(&mut self, number: u64, ask: Ask) {
        self.asked.push(Asked { number, ask });
        self.lay_over();
    }

    /// `selected_shown`: the session's selection, each command not yet told applied in order.
    fn lay_over(&mut self) {
        let mut photos = self.selected.clone();
        for asked in &self.asked {
            asked.ask.apply(&mut photos);
        }
        self.selected_shown = photos;
    }

    /// Whether the photo is live, its controls taking changes and its frames coming.
    pub fn live(&self) -> bool {
        self.live
    }

    /// The photo's file name while it opens and is not yet live, for the disabled controls'
    /// tooltip ("Opening IMG_2041…"); `None` otherwise.
    pub fn opening_name(&self) -> Option<String> {
        let path = match &self.photo {
            PhotoState::Opening { path, .. } => path,
            PhotoState::Open(photo) if !self.live => &photo.path,
            _ => return None,
        };
        Some(path.file_name()?.to_string_lossy().into_owned())
    }

    /// From when the viewer bar says Rendering…, while the photo is not live:
    /// [`RENDERING_AFTER`] after it was asked for (decision 77). `None` once it is live, or
    /// with none asked for.
    pub fn rendering_from(&self) -> Option<Instant> {
        let opening = matches!(self.photo, PhotoState::Opening { .. })
            || matches!(self.photo, PhotoState::Open(_)) && !self.live;
        self.asked_at
            .filter(|_| opening)
            .map(|at| at + RENDERING_AFTER)
    }

    /// Whether the viewer bar says Rendering… at `now`: the photo still not live
    /// [`RENDERING_AFTER`] after it was asked for, so a fast open shows nothing.
    pub fn rendering(&self, now: Instant) -> bool {
        self.rendering_from().is_some_and(|from| now >= from)
    }

    /// The picture and its generation, which a texture made of it remembers.
    pub fn picture(&self) -> Option<(&Picture, u64)> {
        (self.picture.as_ref()).map(|picture| (picture, self.picture_generation))
    }

    fn set_picture(&mut self, picture: Option<Picture>) {
        self.picture = picture;
        self.picture_generation += 1;
    }

    /// The session's picture of the photo opening (decision 77): taken only for the
    /// latest open, before it is live, and only where it is rendered or larger than the
    /// picture shown.
    fn preview(&mut self, preview: OpenPreview) {
        if self.photo.open() != Some(preview.open) || self.live {
            return;
        }
        let Some(image) = preview.picture else {
            return;
        };
        let rendered = preview.source == PreviewSource::Rendered;
        let area = |image: &Thumbnail| u64::from(image.width) * u64::from(image.height);
        let larger = (self.picture.as_ref()).is_none_or(|shown| area(&image) > area(&shown.image));
        if rendered || larger {
            self.set_picture(Some(Picture { image, rendered }));
        }
    }

    /// No photo is open: every tool untouched, no history, no picker and no scopes
    /// reading.
    fn no_photo(&mut self) {
        self.shown = Shown::Nothing;
        self.values = self.untouched.clone();
        self.values_changed();
        self.pending.clear();
        self.history = None;
        self.pickers.forget();
        self.picked.forget();
        self.scopes.forget();
    }

    /// The Export tool was shown, or the photo changed, and it asked for the preview
    /// numbered `number`: an answer to an earlier command is from an earlier showing.
    pub fn previews_from(&mut self, number: u64) {
        self.export_floor = number;
    }

    /// What makes the Export tool's preview stale while its photo and settings stay: a
    /// change of the photo's history, the photo becoming live, which gives its output size,
    /// or the end of an export, whose files are then there. It only grows.
    pub fn preview_generation(&self) -> u64 {
        self.edit_changes + self.jobs.exports_ended()
    }

    /// Shows `pending`, sent as command `number`, until a report includes it. Only this
    /// change is shown over the values as they are, which hold every earlier one: shown
    /// again, a coordinate's change would start from coordinates later changes have moved.
    pub fn pend(&mut self, number: u64, pending: Pending) {
        let image = self.photo.image();
        show(&mut self.values, &pending, self.tools, &image);
        self.values_changed();
        self.pending.push((number, pending));
    }

    /// Takes in one event, but for one an earlier open left on its way while another opens
    /// ([`PhotoState::stale`]), and returns what the app owes in reply.
    pub fn apply(&mut self, event: Event) -> Vec<Reaction> {
        let mut reactions = Vec::new();
        // An open that failed selected nothing, stale or not: its own ask stops showing.
        if let Event::Failed(SessionError::Open { open, .. }) = &event {
            self.open_failed(*open);
        }
        if self.photo.stale(&event) || self.closed && of_a_photo(&event) {
            return reactions;
        }
        // Where the photo open is in the folder changes only with the photo or the listing.
        let moves = matches!(
            event,
            Event::Opened(_) | Event::Library(_) | Event::Failed(SessionError::Open { .. })
        );
        // Settings › Storage's part takes the sizes in, and its Clears' jobs and refusals,
        // which the jobs' mirror sees too.
        self.cache.apply(&event);
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
            // The opening photo's picture (decision 77), for its latest open alone.
            Event::OpenPreview(preview) => self.preview(preview),
            Event::Opened(photo) => {
                self.saving.opened(&photo.path, photo.view_only.is_some());
                self.photo = PhotoState::Open(photo);
            }
            Event::Saved { path, through } => self.saving.saved(&path, through),
            // The open photo's values, which replace the untouched ones in one apply; a
            // session with no photo open sends none. Until the photo is live they show
            // read-only (decision 77).
            Event::Values(values) => {
                if let PhotoState::Open(photo) = &self.photo {
                    // The photo's cell and the Edited sort follow its edit at once.
                    let editable = photo.view_only.is_none();
                    self.folder.edited(&photo.path, values.edited, editable);
                    let live = values.live;
                    self.saving.reported(&values);
                    let image = self.photo.image();
                    self.orientation = crate::zoom::shown(&values, &image);
                    self.values = reported(values, &mut self.pending, self.tools, &image);
                    self.edit_changes += u64::from(live && !self.live);
                    self.live = live;
                    self.shown = if live {
                        Shown::Photo { editable }
                    } else {
                        read_only(&mut self.values);
                        Shown::Opening
                    };
                    self.values_changed();
                }
            }
            // The first live frame takes the picture's place in one swap.
            Event::Frame(frame) => {
                self.frame = Some(frame);
                self.failure = None;
                if self.live
                    && self.picture.take().is_some()
                    && let Some(open) = self.photo.open()
                {
                    let generation = self.picture_generation;
                    reactions.push(Reaction::Swapped { open, generation });
                }
            }
            Event::Scopes(reading) => self.scopes.receive(&reading),
            // The active picker's reading, or `None` as it turns off.
            Event::Picker { through, reading } => {
                if let Some((tool, settings)) = self.picked.apply(reading.as_ref()) {
                    reactions.push(Reaction::Measured { tool, settings });
                }
                self.pickers.apply(through, reading);
            }
            // The Export tool shows the preview, the newest it holds.
            Event::ExportPreview { through, preview } => {
                if through >= self.export_floor {
                    // A preview cancelled in time sends nothing, but one that had finished
                    // sends its answer: only a newer one than this shows.
                    self.export_floor = through;
                    reactions.push(Reaction::Preview { through, preview });
                }
            }
            // The jobs' events are the spinner's and the jobs window's (decisions 57 and 72).
            event @ Event::Job { .. } => {
                let show = self.jobs.apply(&event);
                jobs_reaction(show, &mut reactions);
            }
            // The export's own record is for scripts and the CLI: no row is drawn from it.
            Event::Export { .. } => {}
            // The folder took these in above.
            Event::Library(_) => {}
            // Taken in above, for Settings › Storage.
            Event::Cache(_) => {}
            // The session's selection, the latest by its `through`.
            Event::Selection(selection) => {
                if selection.through >= self.selection.through {
                    self.selected = selection.photos.iter().cloned().collect();
                    // What it includes is the session's; the rest laid over it again.
                    (self.asked).retain(|asked| asked.number > selection.through);
                    self.selection = selection;
                    self.lay_over();
                    // At the first fact to include it, whatever was sent after it.
                    let through = self.selection.through;
                    if let Some(previous) = self.previous.filter(|&number| number <= through) {
                        self.previous = None;
                        self.follow(previous);
                    }
                }
            }
            // The session's view, the latest by its `through`; one numbered lower arrived
            // late.
            Event::View(state) => {
                if (self.view.as_ref()).is_none_or(|view| state.through >= view.through) {
                    self.view = Some(state);
                }
            }
            // The open photo's history, the latest by its `through`, which Undo and Redo
            // name; one numbered lower arrived late.
            Event::History(history) => {
                if matches!(self.photo, PhotoState::Open(_))
                    && self
                        .history
                        .as_ref()
                        .is_none_or(|h| history.through >= h.through)
                {
                    // The same photo's steps or end moved; the first history of a photo is
                    // no change, the photo being a new one for the preview.
                    let moved = self
                        .history
                        .as_ref()
                        .is_some_and(|h| h.steps != history.steps || h.end != history.end);
                    self.edit_changes += u64::from(moved);
                    self.history = Some(history);
                }
            }
            // The session logs a failed save, a tool left out and the scopes turned off as
            // they happen; the rest only reach the app.
            Event::Failed(SessionError::Save {
                path,
                through,
                source,
            }) => {
                self.saving
                    .write_failed(&path, through, &source.to_string());
            }
            // The app hands it over in its own words ([`Mirror::not_saved`]); else the
            // session's.
            Event::Failed(SessionError::NotSaved {
                path,
                through,
                error,
            }) => self.not_saved(&path, through, &error.to_string()),
            // The app hands it over in its own words too; else the session's.
            Event::Failed(error @ SessionError::MoveNotKept { .. }) => {
                self.notice = Some(error.to_string());
            }
            Event::Failed(error) => self.failed(error, &mut reactions),
        }
        if moves {
            self.folder.locate(self.photo.path());
        }
        reactions
    }

    /// The session could not write the edit to `path`, as `why` says, after the first
    /// `through` commands: another program changed the file, and it cannot be written over
    /// (`SessionError::NotSaved`). Told as a failed write is, the open photo's or, as it
    /// closes, the photo left's: Quit asks rather than waits, and no frame clears it.
    pub fn not_saved(&mut self, path: &Path, through: u64, why: &str) {
        self.saving.write_failed(path, through, why);
    }

    fn failed(&mut self, error: SessionError, reactions: &mut Vec<Reaction>) {
        if let SessionError::Refused { command, .. } = &error {
            // It changed nothing: what was shown for it, and a save awaited for it, stop.
            let command = *command;
            self.pending.retain(|(number, _)| *number != command);
            self.pickers.refused(command);
            self.saving.refused(command);
            self.jobs.refused(command);
            // A selection command refused changed nothing: the rest laid over without it.
            if self.asked.iter().any(|asked| asked.number == command) {
                self.asked.retain(|asked| asked.number != command);
                self.lay_over();
            }
            if self.previous == Some(command) {
                self.previous = None;
            }
        }
        // The app sends no change on the edit before the photo is live, its controls drawn
        // disabled, so it never causes this; should one come, nothing is said of it.
        if let SessionError::Refused { error: refused, .. } = &error
            && matches!(**refused, SessionError::NotLive)
        {
            return;
        }
        // A zoom the session cannot render yet keeps its frame at Fit, and the Viewer goes
        // back to Fit with it: logged under `control`, as a refused step is.
        if let SessionError::Refused {
            command,
            error: refused,
        } = &error
            && let SessionError::Zoom(why) = &**refused
        {
            tracing::debug!(target: "control", "zoom refused, back to Fit: {why}");
            reactions.push(Reaction::Unzoomed { command: *command });
            return;
        }
        if !matches!(error, SessionError::Tool { .. } | SessionError::Scopes(_)) {
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
            self.pickers.forget();
            let show = self.jobs.engine_stopped();
            jobs_reaction(show, reactions);
        }
        // A photo opening, its values come or not, until it is live.
        let opening = match &self.photo {
            PhotoState::Opening { path, open } => Some((path.clone(), *open)),
            PhotoState::Open(photo) if !self.live => Some((photo.path.clone(), photo.open)),
            _ => None,
        };
        match opening {
            Some((path, open)) if ends_open => {
                // An open that failed selected nothing; one the engine ended never reported.
                self.open_failed(open);
                let message = format!("{}: {error}", path.display());
                self.photo = PhotoState::Failed { path, message };
                // The session closed the photo left before it tried: none is open.
                self.no_photo();
                self.set_picture(None);
                self.saving.leaving();
            }
            // The scopes failing turns them off, never the frame (decision 19): nothing over
            // the photo.
            _ if matches!(error, SessionError::Scopes(_)) => {}
            _ => self.failure = Some(error.to_string()),
        }
    }
}

/// A command the app sent that changes the selection, laid over the session's until a
/// fact includes it ([`Mirror::selecting`]).
#[derive(Debug)]
struct Asked {
    /// Its number, as the session numbered it.
    number: u64,
    ask: Ask,
}

/// What a command does to the selection, by the session's rules.
#[derive(Debug)]
enum Ask {
    /// A selection command, with the photo open or opening as it was sent and the number
    /// of the open that asked for it, from which a range runs.
    Select(SelectionCommand, Option<(PathBuf, u64)>),
    /// An open: a photo outside the selection is selected alone.
    Open(PathBuf),
}

impl Ask {
    fn apply(&self, photos: &mut HashSet<PathBuf>) {
        match self {
            Ask::Open(path) => {
                if !photos.contains(path) {
                    *photos = HashSet::from([path.clone()]);
                }
            }
            Ask::Select(SelectionCommand::Set(set), _) => *photos = set.iter().cloned().collect(),
            Ask::Select(SelectionCommand::Add(photo), _) => {
                photos.insert(photo.clone());
            }
            Ask::Select(SelectionCommand::Remove(photo), _) => {
                photos.remove(photo);
            }
            Ask::Select(SelectionCommand::Range { to, order }, active) => {
                let active = active.as_ref().map(|(photo, _)| photo.as_path());
                let at = |photo: &Path| order.iter().position(|listed| listed == photo);
                *photos = match active.and_then(at).zip(at(to)) {
                    Some((from, to)) => {
                        let (first, last) = (from.min(to), from.max(to));
                        order[first..=last].iter().cloned().collect()
                    }
                    None => HashSet::new(),
                };
                photos.insert(to.clone());
                photos.extend(active.map(Path::to_owned));
            }
            Ask::Select(SelectionCommand::Clear, _) => photos.clear(),
            // They need the collection, or the earlier selection: as the session tells them.
            Ask::Select(
                SelectionCommand::All
                | SelectionCommand::Invert
                | SelectionCommand::Unedited
                | SelectionCommand::Previous,
                _,
            ) => {}
        }
    }
}

/// Every tool's controls at its definition's defaults, as the Tools panel draws them with
/// no photo open: none can change anything, none is marked, no preset row's option can be
/// applied, and a choice lists every option. A preset row reads the option the defaults
/// match and a view's coordinates are the tool's view of them, both on the no-photo
/// image ([`PhotoState::image`]); a row whose tool cannot say reads Custom, and a view
/// whose tool cannot is left out, as the session leaves it out. Whether each control's
/// row shows, and a value after its label, are the tool's on that image too (decision
/// 88), so Exposure's camera rows, which no photo gives anything to do, are hidden.
fn untouched(tools: &[&dyn ProcessingTool]) -> Values {
    let image = PhotoState::None.image();
    let states = tools.iter().map(|tool| {
        let definition = tool.definition();
        let defaults = definition.defaults();
        let (rows, _) = edit::photo_rows(*tool, &image);
        let controls: Vec<ControlState> = definition
            .controls
            .iter()
            .zip(rows)
            .map(|(control, row)| ControlState {
                control: control.id,
                value: control.default(),
                default: control.default(),
                mark: None,
                available: false,
                options: Vec::new(),
                range: edit::range(*tool, control.id, &defaults).map(|(range, _)| range),
                visible: row.visible,
                label_value: row.label_value,
            })
            .collect();
        let presets = definition
            .preset_rows
            .iter()
            .map(|row| PresetState {
                preset: row.id,
                matching: catch_unwind(AssertUnwindSafe(|| {
                    tool.matching(row.id, &defaults, &image)
                }))
                .unwrap_or_default(),
                available: Vec::new(),
            })
            .collect();
        let mut views = Vec::new();
        for view in definition.views {
            let viewed = catch_unwind(AssertUnwindSafe(|| tool.view(view.id, &defaults, &image)));
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
        ToolState {
            tool: definition.tool.id,
            enabled: !definition.switched,
            edited: false,
            hidden: false,
            controls,
            presets,
            views,
            computed: Vec::new(),
            readouts: Vec::new(),
        }
    });
    Values {
        tools: states.collect(),
        settings: Vec::new(),
        edited: false,
        through: 0,
        extent: [0, 0],
        live: false,
    }
}

/// Whether `event` is a photo's own, which only a photo open or opening sends: its
/// picture, its opening or its failure, its values, history, frames, scopes readings and
/// picker readings.
fn of_a_photo(event: &Event) -> bool {
    matches!(
        event,
        Event::Opened(_)
            | Event::OpenPreview(_)
            | Event::Values(_)
            | Event::History(_)
            | Event::Frame(_)
            | Event::Scopes(_)
            | Event::Picker { .. }
            | Event::Failed(SessionError::Open { .. })
    )
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
    /// The photo asked for, open or opening: the photo shown; none for one that failed.
    pub fn asked(&self) -> Option<&Path> {
        match self {
            PhotoState::Opening { path, .. } => Some(path),
            PhotoState::Open(photo) => Some(&photo.path),
            PhotoState::None | PhotoState::Failed { .. } => None,
        }
    }

    /// The image context a tool's view of the pending values reads ([`Pending::show`]):
    /// the open photo's, as the session reports it in `Photo` (decision 27), so a view
    /// that reads the camera, as White balance's temperature and tint read its matrix,
    /// shows its pending values as the session will. The pending display's availability
    /// reads it too (decision 30), where the pending values move the tool's answer.
    pub fn image(&self) -> ImageContext {
        match self {
            PhotoState::Open(photo) => (*photo.image).clone(),
            _ => ImageContext::full(0, 0),
        }
    }

    /// The number of the open that asked for the photo, opening or open.
    pub fn open(&self) -> Option<u64> {
        match self {
            PhotoState::Opening { open, .. } => Some(*open),
            PhotoState::Open(photo) => Some(photo.open),
            PhotoState::None | PhotoState::Failed { .. } => None,
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
    /// report, history, frame and scopes reading until its own `Opened`.
    fn stale(&self, event: &Event) -> bool {
        // Once open, an earlier open's failure, which may come after this one's `Opened`
        // (decision 77).
        if let PhotoState::Open(photo) = self {
            return matches!(event,
                Event::Failed(SessionError::Open { open, .. }) if *open != photo.open);
        }
        let PhotoState::Opening { open, .. } = self else {
            return false;
        };
        match event {
            Event::Opened(photo) => photo.open != *open,
            Event::Failed(SessionError::Open { open: failed, .. }) => failed != open,
            Event::Values(_)
            | Event::History(_)
            | Event::Frame(_)
            | Event::Scopes(_)
            | Event::Picker { .. } => true,
            _ => false,
        }
    }
}

/// How long a write of the open photo's edit may go unconfirmed before the Toolbar says
/// Saving… (`docs/gui.md`, The edit, saved).
pub const SAVING_AFTER: Duration = Duration::from_secs(2);

/// How long a photo may go unlive after it was asked for before the viewer bar says
/// Rendering… (decision 77): a fast open shows nothing.
pub const RENDERING_AFTER: Duration = Duration::from_secs(1);

/// The picture the Viewer shows of a photo until its first live frame (decision
/// 77), unmarked: a thumbnail, its rendered one or the camera's.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub image: Thumbnail,
    /// Whether it is Apertrust's render of the edit.
    pub rendered: bool,
}

/// Every control, coordinate and preset option of `values` drawn disabled, as a photo's
/// values are until it is live (decision 77), each keeping its value and mark.
fn read_only(values: &mut Values) {
    for tool in &mut values.tools {
        for control in &mut tool.controls {
            control.available = false;
        }
        for coordinate in &mut tool.views {
            coordinate.available = false;
        }
        for preset in &mut tool.presets {
            preset.available.clear();
        }
    }
}

/// Whether the open photo's edit is on disk as it stands, from what the app sent and the
/// session reported, for the Toolbar's save state: nothing while saving works, Saving…
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
    /// The open photo's edit file and why its last write failed, where one did: what
    /// [`Saving::left`] is to keep once the session itself has moved to another photo.
    fn departing(&self) -> Option<(PathBuf, String)> {
        (self.sidecar.clone()).zip(self.failure.clone())
    }

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

    /// What the Toolbar's save state shows at `now`.
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
            (report.tools == values.tools)
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
    tools: &[&dyn ProcessingTool],
    image: &ImageContext,
) -> Values {
    pending.retain(|(number, _)| *number > reported.through);
    for (_, change) in pending.iter() {
        show(&mut reported, change, tools, image);
    }
    reported
}

/// Shows a change sent in `values` as the session will report it ([`Pending::show`]): a
/// control set shows as kept, as the session keeps it, one reset its default, released,
/// and a lossless view's coordinates and controls move together. `image` is what the
/// tool's view reads.
pub fn show(
    values: &mut Values,
    change: &Pending,
    tools: &[&dyn ProcessingTool],
    image: &ImageContext,
) {
    let id = change.tool();
    let tool = tools.iter().find(|tool| tool.definition().tool.id == id);
    let state = values.tools.iter_mut().find(|state| state.tool == id);
    let (Some(tool), Some(state)) = (tool, state) else {
        return;
    };
    change.show(*tool, state, image);
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
            view_only: None,
            exposure_time: None,
            aperture: None,
            focal_length: None,
            image: Box::new(ImageContext::full(84, 62)),
        };
        let values = Event::Values(Values {
            tools: Vec::new(),
            settings: Vec::new(),
            edited: false,
            through: 4,
            extent: [84, 62],
            live: true,
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
        use apertrust_engine::session::{ControlState, ToolState};

        let control = |id, value: f32, mark| ControlState {
            control: id,
            value: ControlValue::Number(value),
            default: ControlValue::Number(0.0),
            mark,
            available: true,
            options: Vec::new(),
            range: None,
            visible: true,
            label_value: None,
        };
        let exposure = |black, black_mark, exposure, exposure_mark| ToolState {
            tool: "exposure",
            enabled: true,
            edited: black != 0.0 || exposure != 0.0,
            hidden: false,
            controls: vec![
                control("black_level", black, black_mark),
                control("exposure", exposure, exposure_mark),
            ],
            presets: Vec::new(),
            views: Vec::new(),
            computed: Vec::new(),
            readouts: Vec::new(),
        };
        let report = |black, value, through| Values {
            tools: vec![exposure(black, None, value, Some(Mark::Kept))],
            settings: Vec::new(),
            edited: true,
            through,
            extent: [84, 62],
            live: true,
        };
        let image = ImageContext::full(84, 62);
        let mirror = |report, pending: &mut Vec<_>| {
            reported(report, pending, apertrust_tools::TOOLS, &image)
        };
        let set_black = |value| Pending::Control {
            tool: "exposure",
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
                    tool: "exposure",
                    control: "exposure",
                },
            ),
        ];
        let shown = mirror(report(0.05, 1.0, 4), &mut pending);
        assert_eq!(shown.tools, [exposure(0.2, Some(Mark::Kept), 0.0, None)]);
        assert_eq!(pending.len(), 3);
        let shown = mirror(report(0.2, 0.0, 7), &mut pending);
        assert_eq!(shown.tools, [exposure(0.2, None, 0.0, Some(Mark::Kept))]);
        assert!(pending.is_empty());

        // The marker and the header's reset show at once too.
        let mut pending = vec![
            (
                8,
                Pending::Keep {
                    tool: "exposure",
                    control: "exposure",
                    kept: false,
                },
            ),
            (9, Pending::ResetTool { tool: "exposure" }),
        ];
        let shown = mirror(report(0.2, 1.0, 7), &mut pending[..1].to_vec());
        assert_eq!(shown.tools, [exposure(0.2, None, 1.0, None)]);
        let shown = mirror(report(0.2, 1.0, 7), &mut pending);
        assert_eq!(shown.tools, [exposure(0.0, None, 0.0, None)]);
    }

    // Feeding `apply` event sequences: the mirror and the reactions it returns.

    pub(crate) fn photo(open: u64) -> Photo {
        Photo {
            path: PathBuf::from("/photos/a.ARW"),
            open,
            camera: String::new(),
            width: 84,
            height: 62,
            view_only: None,
            exposure_time: None,
            aperture: None,
            focal_length: None,
            image: Box::new(ImageContext::full(84, 62)),
        }
    }

    /// The exposure tool with Exposure at `exposure`, reported after `through` commands.
    pub(crate) fn report(exposure: f32, through: u64) -> Values {
        use apertrust_engine::session::{ControlState, ToolState};
        Values {
            tools: vec![ToolState {
                tool: "exposure",
                enabled: true,
                edited: exposure != 0.0,
                hidden: false,
                controls: vec![ControlState {
                    control: "exposure",
                    value: ControlValue::Number(exposure),
                    default: ControlValue::Number(0.0),
                    mark: None,
                    available: true,
                    options: Vec::new(),
                    range: None,
                    visible: true,
                    label_value: None,
                }],
                presets: Vec::new(),
                views: Vec::new(),
                computed: Vec::new(),
                readouts: Vec::new(),
            }],
            settings: Vec::new(),
            edited: exposure != 0.0,
            through,
            extent: [84, 62],
            live: true,
        }
    }

    fn set_exposure(value: f32) -> Pending {
        Pending::Control {
            tool: "exposure",
            control: "exposure",
            value: ControlValue::Number(value),
        }
    }

    /// The Exposure the widgets show.
    fn shown(mirror: &Mirror) -> ControlValue {
        mirror.values.tools[0].controls[0].value.clone()
    }

    pub(crate) fn mirror() -> Mirror {
        Mirror::new(apertrust_tools::TOOLS, jobs::Mirror::new(None))
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
    fn a_refused_view_goes_back_to_fit() {
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        let why = "the fitted viewport renders in tiles".to_owned();
        let refused = Event::Failed(SessionError::Refused {
            command: 4,
            error: Box::new(SessionError::Zoom(why)),
        });
        assert_eq!(m.apply(refused), [Reaction::Unzoomed { command: 4 }]);
        // Nothing said over the photo: its frame stays at Fit, as the zoom goes back to it.
        assert_eq!(m.failure, None);
    }

    #[test]
    fn a_frame_replaces_the_last_and_clears_the_failure() {
        use apertrust_engine::compute::Timings;
        use apertrust_engine::gpu::{GpuContext, GpuOptions};
        use apertrust_engine::session::FramePlace;
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
            place: FramePlace {
                scale: 1.0,
                origin: [0, 0],
                extent: [2, 2],
            },
            overview: None,
            detail_pending: false,
        };
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        // A failure stays until the next frame, which is the photo drawn again; a notice of
        // the history, until another photo is asked for.
        let moved = SessionError::MoveNotKept {
            path: sidecar::path_for(&photo(1).path),
        };
        m.apply(Event::Failed(moved));
        m.apply(Event::Failed(SessionError::Gpu("out of memory".to_owned())));
        assert_eq!(m.failure.as_deref(), Some("the GPU: out of memory"));
        assert!(m.frame.is_none());
        assert_eq!(m.apply(Event::Frame(frame(3))), []);
        assert_eq!(m.frame.as_ref().map(|f| f.through), Some(3));
        assert_eq!(m.failure, None);
        m.apply(Event::Frame(frame(5)));
        assert_eq!(m.frame.as_ref().map(|f| f.through), Some(5));
        assert!(m.notice.as_deref().is_some_and(|n| n.contains("Undo")));
        m.opening(PathBuf::from("/photos/b.ARW"), 2, Instant::now());
        assert_eq!(m.notice, None);
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
        // The failed save is the saving's, not the Viewer's failure.
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

    /// An edit the session could not write over another program's file is told as a failed
    /// write: no longer awaited, so Quit asks rather than waits; the save state's, not the
    /// Viewer's, so no frame clears it; and kept as the photo left's as it closes.
    #[test]
    fn an_edit_not_saved_over_another_program_s_file_is_a_failed_write() {
        let mut m = mirror();
        let edit = sidecar::path_for(&photo(1).path);
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        send(&mut m, 2, 1.0);
        assert!(m.saving.writing());
        let unreadable = SessionError::NotSaved {
            path: edit.clone(),
            through: 2,
            error: Box::new(SessionError::NotLive),
        };
        assert_eq!(m.apply(Event::Failed(unreadable)), []);
        assert!(!m.saving.writing());
        assert!(m.saving.failed().is_some());
        assert!(matches!(
            m.saving.state(Instant::now()),
            SaveState::Failed(_)
        ));
        assert_eq!(m.failure, None);
        // In the app's words where it hands it over.
        m.not_saved(&edit, 2, "can't read it");
        assert_eq!(
            m.saving.state(Instant::now()),
            SaveState::Failed("can't read it")
        );
        // A completed change after it, whose write the session drops as well: answered by
        // the next report, so it is no longer awaited either.
        m.saving.sent(3, true, Instant::now());
        assert!(m.saving.writing());
        m.not_saved(&edit, 3, "can't read it");
        assert!(!m.saving.writing());
        // The photo turned view-only: `Opened` again, then what was not saved, which Quit
        // asks about though the save state says nothing of a photo it never writes.
        send(&mut m, 4, 2.0);
        let mut view_only = photo(1);
        view_only.view_only = Some(sidecar::ViewOnly {
            build: None,
            reason: sidecar::Reason::Newer(sidecar::Newer::Version(3)),
        });
        m.apply(Event::Opened(view_only));
        m.not_saved(&edit, 4, "view only now");
        assert!(!m.saving.writing());
        assert_eq!(m.saving.failed(), Some("view only now"));
        // As the photo closes, another photo asked for: the photo left's.
        m.opening(PathBuf::from("/photos/b.ARW"), 4, Instant::now());
        m.not_saved(&edit, 3, "can't read it");
        assert_eq!(m.saving.left, Some((edit, "can't read it".to_owned())));
    }

    #[test]
    fn a_failed_open_is_the_photo_s_and_a_failure_later_is_the_viewer_s() {
        let mut m = mirror();
        m.opening(PathBuf::from("/photos/b.ARW"), 4, Instant::now());
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
        // With a photo open, a failure is for the Viewer; a scopes failure shows
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
                done: 1,
                total: Some(2),
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
        m.opening(PathBuf::from("/photos/b.ARW"), 2, Instant::now());
        let reactions = m.apply(Event::Failed(SessionError::Stopped));
        assert_eq!(reactions, [Reaction::Jobs(jobs::Show::Update)]);
        assert!(matches!(m.photo, PhotoState::Failed { .. }));
        assert_eq!(m.jobs.view(&words)[0].phase, JobPhase::Failed);
        // No job left to end: nothing more to show.
        assert_eq!(m.apply(Event::Failed(SessionError::Stopped)), []);
    }

    #[test]
    fn a_refused_try_again_leaves_nothing_for_quitting_to_wait_for() {
        let id = jobs::tests::ids(1)[0];
        let mut m = mirror();
        // Try again sent as command 5, which quitting waits for until its job is added.
        m.jobs.starting(5);
        assert!(!m.jobs.running().none());
        m.apply(Event::Failed(SessionError::Refused {
            command: 5,
            error: Box::new(SessionError::NotRetryable { job: id }),
        }));
        assert!(m.jobs.running().none());
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

    /// The photo's picker readings reach the pickers' mirror; a refused `On` stops
    /// showing, and so does one the session turned off again before its first reading, as
    /// a change of its tool does; another photo, and a photo closed, forget it and drop
    /// what it still sends.
    #[test]
    fn the_mirror_keeps_the_photo_s_picker() {
        use apertrust_engine::session::{PickerReading, PickerRef, Region};

        let exposure = PickerRef::Tool {
            tool: "exposure",
            picker: "exposure",
        };
        let reading = |through| Event::Picker {
            through,
            reading: Some(PickerReading {
                through,
                picker: exposure,
                region: Region::Area([0.02, 0.02, 0.98, 0.98]),
                tool: None,
                frame: None,
            }),
        };
        let off = |through| Event::Picker {
            through,
            reading: None,
        };
        let mut m = mirror();
        m.opening(PathBuf::from("/photos/a.ARW"), 1, Instant::now());
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        m.pickers.asked(2, Some((exposure, true)));
        m.apply(refused(2, "no picker"));
        assert_eq!(m.pickers.active(), None);
        // On, then Exposure changed before its first reading: the session turns it off.
        m.pickers.asked(3, Some((exposure, true)));
        send(&mut m, 4, 0.5);
        assert_eq!(m.pickers.active(), Some(exposure));
        m.apply(off(4));
        assert_eq!(m.pickers.active(), None);
        // An end from before an `On` leaves it shown.
        m.pickers.asked(5, Some((exposure, true)));
        m.apply(off(4));
        assert_eq!(m.pickers.active(), Some(exposure));
        m.apply(reading(5));
        assert_eq!(m.pickers.active(), Some(exposure));
        m.apply(off(6));
        assert_eq!(m.pickers.active(), None);
        m.apply(reading(6));
        // Another photo: none, the photo left's readings dropped on their way.
        m.opening(PathBuf::from("/photos/b.ARW"), 7, Instant::now());
        assert_eq!(m.pickers.active(), None);
        m.apply(reading(6));
        assert_eq!(m.pickers.active(), None);
        m.apply(Event::Opened(photo(7)));
        m.apply(Event::Values(report(0.0, 7)));
        m.apply(reading(8));
        assert_eq!(m.pickers.active(), Some(exposure));
        // Closed, and what the session still sends of it.
        m.closing();
        assert_eq!(m.pickers.active(), None);
        m.apply(reading(9));
        assert_eq!(m.pickers.active(), None);
    }

    /// A tool pick's command that completes a change is waited for as any completed
    /// change is: the session answers it with `Saved`, the edit written or, where the pick
    /// wrote nothing, reported held, so a release whose only answer is `Saved`, with no
    /// `Values`, is waited for no more, and a slider's drag after it shows no Saving…; a
    /// failed write of a pick is told.
    #[test]
    fn a_pick_is_waited_for_until_its_save_and_no_longer() {
        let edit = sidecar::path_for(&photo(1).path);
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        // A release, saved in its own batch: `Saved` alone answers it.
        let now = Instant::now();
        m.saving.sent(2, true, now);
        assert!(m.saving.writing());
        m.apply(Event::Saved {
            path: edit.clone(),
            through: 2,
        });
        assert!(!m.saving.writing());
        // A slider's drag after it: its steps wait for no write, and nothing shows.
        for (number, value) in [(3, 0.2), (4, 0.4)] {
            m.pend(number, set_exposure(value));
            m.saving.sent(number, false, now);
            m.apply(Event::Values(report(value, number)));
        }
        let later = now + SAVING_AFTER + Duration::from_millis(1);
        assert_eq!(m.saving.saving_from(), None);
        assert_eq!(m.saving.state(later), SaveState::Saved);

        // A pick whose write fails is told.
        m.saving.sent(5, true, now);
        assert_eq!(m.saving.state(later), SaveState::Saving);
        m.apply(Event::Failed(SessionError::Save {
            path: edit,
            through: 5,
            source: std::io::Error::other("disk full"),
        }));
        assert!(!m.saving.writing());
        assert!(
            m.saving
                .failed()
                .is_some_and(|why| why.contains("disk full"))
        );
    }

    /// Whether `values` are every tool's, untouched and unavailable, as with no photo.
    fn untouched_all(values: &Values) -> bool {
        let ids: Vec<&str> = apertrust_tools::TOOLS
            .iter()
            .map(|tool| tool.definition().tool.id)
            .collect();
        let shown: Vec<&str> = values.tools.iter().map(|state| state.tool).collect();
        let controls = values.tools.iter().flat_map(|state| &state.controls);
        let coordinates = values.tools.iter().flat_map(|state| &state.views);
        let presets = values.tools.iter().flat_map(|state| &state.presets);
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
    fn with_no_photo_every_tool_is_untouched_and_takes_no_edit() {
        let m = mirror();
        assert_eq!(m.shown(), Shown::Nothing);
        assert!(!m.takes_edits() && !m.shown().editable());
        assert!(untouched_all(&m.values));
        // Each control at its definition's default, whatever the photo would make it.
        for (tool, state) in apertrust_tools::TOOLS.iter().zip(&m.values.tools) {
            for (control, shown) in tool.definition().controls.iter().zip(&state.controls) {
                assert_eq!(shown.value, control.default(), "{}", control.id);
            }
        }
        // A lossless view's coordinates are there to draw, as White balance's temperature
        // and tint are.
        let temperature = m.values.tools.iter().find(|s| s.tool == "temperature");
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
        // Another photo is asked for: at once no photo, every tool untouched, no scopes,
        // and nothing sent for them.
        m.pend(2, set_exposure(2.0));
        m.opening(PathBuf::from("/photos/b.ARW"), 3, Instant::now());
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
        assert_eq!(m.values.tools.len(), 1);
        assert_eq!(shown(&m), ControlValue::Number(0.5));
        assert!(m.takes_edits());
    }

    #[test]
    fn a_photo_open_for_viewing_only_takes_no_edit_and_its_resets_are_disabled() {
        let mut m = mirror();
        let mut view_only = photo(1);
        view_only.view_only = Some(sidecar::ViewOnly {
            build: None,
            reason: sidecar::Reason::Newer(sidecar::Newer::Version(9)),
        });
        m.apply(Event::Opened(view_only));
        m.apply(Event::Values(report(0.0, 1)));
        assert_eq!(m.shown(), Shown::Photo { editable: false });
        assert!(!m.takes_edits() && !m.shown().editable());
        m.opening(PathBuf::from("/photos/b.ARW"), 2, Instant::now());
        m.apply(Event::Opened(photo(2)));
        m.apply(Event::Values(report(0.0, 2)));
        assert_eq!(m.shown(), Shown::Photo { editable: true });
    }

    #[test]
    fn a_failed_open_leaves_no_photo_in_the_tools_panel() {
        let mut m = mirror();
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(1.5, 1)));
        m.apply(Event::Scopes(crate::scopes::testing::reading(
            true, false, false,
        )));
        m.opening(PathBuf::from("/photos/b.ARW"), 2, Instant::now());
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
        m.opening(PathBuf::from("/photos/c.ARW"), 3, Instant::now());
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
        m.opening(PathBuf::from("/photos/b.ARW"), 2, Instant::now());
        assert_eq!(m.failure, None);
        assert!(m.frame.is_none());
        assert_eq!(m.saving.state(Instant::now()), SaveState::None);
        assert!(matches!(m.photo, PhotoState::Opening { open: 2, .. }));
        assert_eq!(m.apply(Event::Opened(photo(2))), []);
        assert!(matches!(m.photo, PhotoState::Open(_)));
        assert!(!m.saving.writing());
    }

    /// Reset is available while a tool every photo runs is not applied, the history's end
    /// before its default step, though the edit holds nothing: Reset applies it (decision
    /// 78). A switched tool off is no reason, and the edit's `edited` stays as it was.
    #[test]
    fn reset_is_available_while_a_default_tool_is_not_applied() {
        let mut mirror = mirror();
        assert!(!mirror.values.edited);
        assert!(mirror.values.tools.iter().any(|state| !state.enabled));
        assert!(!mirror.resettable());
        let exposure = (mirror.values.tools.iter_mut())
            .find(|state| state.tool == "exposure")
            .unwrap();
        exposure.enabled = false;
        assert!(mirror.resettable());
        assert!(!mirror.values.edited);
    }

    /// What makes the Export tool's preview stale besides its photo and settings: the photo's
    /// history moving, in its steps or where it ends, the photo becoming live, and an export
    /// ending, whatever its outcome. A history's first, another's `through` alone, a live
    /// photo's later values and jobs that are no exports change nothing.
    #[test]
    fn the_export_preview_goes_stale_when_the_history_moves_the_photo_goes_live_or_an_export_ends()
    {
        use crate::history::tests::{history, step};
        use apertrust_engine::jobs::{Queue, State};

        let steps = |ids: &[u64]| {
            ids.iter()
                .map(|id| step(*id, &["exposure"], None))
                .collect()
        };
        let event = |through, ids: &[u64]| Event::History(history(through, steps(ids), None, None));
        let mut m = mirror();
        m.opening(PathBuf::from("/photos/a.ARW"), 1, Instant::now());
        m.apply(Event::Opened(photo(1)));
        // Still opening: its values are read-only and its output size not yet known. A
        // preview asked now lacks the size; becoming live asks for another, once.
        let opening = m.preview_generation();
        let not_live = Values {
            live: false,
            ..report(0.0, 1)
        };
        m.apply(Event::Values(not_live.clone()));
        assert_eq!(m.preview_generation(), opening);
        m.apply(Event::Values(report(0.0, 1)));
        assert_eq!(m.preview_generation(), opening + 1);
        // Live already: its later values, as every change reports them, ask for nothing.
        m.apply(Event::Values(report(0.5, 2)));
        assert_eq!(m.preview_generation(), opening + 1);
        let before = m.preview_generation();
        // The photo's first history is no change: its photo is new to the preview already.
        m.apply(event(1, &[1]));
        assert_eq!(m.preview_generation(), before);
        // The same steps again, as a command that changed nothing of them sends.
        m.apply(event(2, &[1]));
        assert_eq!(m.preview_generation(), before);
        // A step more (a Reset, a change), then the end moved back (Undo).
        m.apply(event(3, &[1, 2]));
        assert_eq!(m.preview_generation(), before + 1);
        let mut undone = history(4, steps(&[1, 2]), None, None);
        undone.end = 1;
        m.apply(Event::History(undone));
        assert_eq!(m.preview_generation(), before + 2);
        // Another photo's history starts over, with no change.
        m.opening(PathBuf::from("/photos/b.ARW"), 5, Instant::now());
        m.apply(Event::Opened(photo(5)));
        m.apply(Event::Values(report(0.0, 5)));
        let other = m.preview_generation();
        m.apply(event(5, &[1, 2, 3]));
        assert_eq!(m.preview_generation(), other);

        // An export ending, finished or cancelled, once each.
        let ids = jobs::tests::ids(2);
        m.apply(jobs::tests::added(ids[0], Queue::UserExport, Some(1)));
        assert_eq!(m.preview_generation(), other);
        m.apply(jobs::tests::ended(ids[0], State::Finished, None));
        assert_eq!(m.preview_generation(), other + 1);
        m.apply(jobs::tests::added(ids[1], Queue::UserExport, Some(2)));
        m.apply(jobs::tests::ended(ids[1], State::Cancelled, None));
        assert_eq!(m.preview_generation(), other + 2);
        // An end reported again, or of a job never added, counts nothing.
        m.apply(jobs::tests::ended(ids[1], State::Cancelled, None));
        m.apply(jobs::tests::ended(
            jobs::tests::ids(3)[2],
            State::Finished,
            None,
        ));
        assert_eq!(m.preview_generation(), other + 2);
    }

    /// The photo's history (decision 78): the latest by its `through`, an earlier one
    /// arriving late dropped; none while another photo opens, until its own, an earlier
    /// open's dropped; none once the photo closes or an open fails.
    #[test]
    fn the_mirror_keeps_the_photo_s_latest_history() {
        use crate::history::tests::{history, step};
        use apertrust_engine::session::{Move, StepId};

        let one = |through, undo| {
            let steps = vec![step(1, &["exposure"], None)];
            Event::History(history(through, steps, undo, None))
        };
        let undo = |m: &Mirror| m.history.as_ref().map(|h| (h.through, h.undo));
        let mut m = mirror();
        // None before a photo opens.
        m.apply(one(1, None));
        assert_eq!(undo(&m), None);
        m.opening(PathBuf::from("/photos/a.ARW"), 1, Instant::now());
        m.apply(one(1, None));
        assert_eq!(undo(&m), None);
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        m.apply(one(1, None));
        assert_eq!(undo(&m), Some((1, None)));
        let back = Some(Move::Steps {
            first: StepId(1),
            count: 1,
        });
        m.apply(one(4, back));
        assert_eq!(undo(&m), Some((4, back)));
        // An earlier one, late.
        m.apply(one(3, None));
        assert_eq!(undo(&m), Some((4, back)));
        // One with the same `through`, as an open sends again once live where the decode
        // describes the steps anew: the later replaces it (`docs/events.md`, the history).
        m.apply(one(4, None));
        assert_eq!(undo(&m), Some((4, None)));
        m.apply(one(4, back));
        // Another photo: none until its own, the photo left's dropped on its way.
        m.opening(PathBuf::from("/photos/b.ARW"), 5, Instant::now());
        assert_eq!(undo(&m), None);
        m.apply(one(4, back));
        assert_eq!(undo(&m), None);
        m.apply(Event::Opened(photo(5)));
        m.apply(Event::Values(report(0.0, 5)));
        m.apply(one(5, None));
        assert_eq!(undo(&m), Some((5, None)));
        // Closed, and what the session still sends of it.
        m.closing();
        assert_eq!(undo(&m), None);
        m.apply(one(6, back));
        assert_eq!(undo(&m), None);
        // An open that fails.
        m.opening(PathBuf::from("/photos/c.ARW"), 7, Instant::now());
        m.apply(Event::Opened(photo(7)));
        m.apply(one(7, back));
        assert_eq!(undo(&m), Some((7, back)));
        m.apply(Event::Failed(SessionError::Open {
            open: 7,
            error: Box::new(SessionError::NotAMosaic),
        }));
        assert_eq!(undo(&m), None);
    }

    /// The mirror keeps the session's latest selection by its `through`, one arriving late
    /// dropped, whatever photo is opening or closed, and tells which photos it holds
    /// (decision 37).
    #[test]
    fn an_earlier_open_failing_late_stops_showing_its_photo_selected() {
        let a = PathBuf::from("/photos/a.ARW");
        let b = PathBuf::from("/photos/b.ARW");
        let mut m = mirror();
        // A asked for, then B before A's failure: A's failure comes once B is opening.
        m.opening(a.clone(), 1, Instant::now());
        m.opening(b.clone(), 2, Instant::now());
        assert!(m.is_selected(&b));
        m.apply(Event::Failed(SessionError::Open {
            open: 1,
            error: Box::new(SessionError::NotAMosaic),
        }));
        // Stale for the photo, B still opening, but A's open selected nothing.
        assert!(matches!(m.photo, PhotoState::Opening { open: 2, .. }));
        assert!(!m.is_selected(&a));
        assert!(m.is_selected(&b));
    }

    #[test]
    fn a_range_sent_after_an_open_that_fails_runs_from_no_photo() {
        let at = |name: &str| PathBuf::from("/photos").join(name);
        let order: Vec<PathBuf> = ["a.ARW", "b.ARW", "c.ARW"].map(at).to_vec();
        let mut m = mirror();
        m.opening(at("a.ARW"), 1, Instant::now());
        let range = SelectionCommand::Range {
            to: at("c.ARW"),
            order,
        };
        m.selecting(&range, 2);
        assert!(m.is_selected(&at("b.ARW")));
        // A's open fails before the session applies the range: none active, C alone.
        m.apply(Event::Failed(SessionError::Open {
            open: 1,
            error: Box::new(SessionError::NotAMosaic),
        }));
        assert!(!m.is_selected(&at("a.ARW")));
        assert!(!m.is_selected(&at("b.ARW")));
        assert!(m.is_selected(&at("c.ARW")));
    }

    #[test]
    fn a_range_keeps_its_anchor_when_an_earlier_open_of_the_same_photo_fails() {
        let at = |name: &str| PathBuf::from("/photos").join(name);
        let order: Vec<PathBuf> = ["a.ARW", "b.ARW", "c.ARW"].map(at).to_vec();
        let mut m = mirror();
        // A opened, A opened again, then a range from A: the first open's failure is not
        // the second's.
        m.opening(at("a.ARW"), 1, Instant::now());
        m.opening(at("a.ARW"), 2, Instant::now());
        let range = SelectionCommand::Range {
            to: at("c.ARW"),
            order,
        };
        m.selecting(&range, 3);
        m.apply(Event::Failed(SessionError::Open {
            open: 1,
            error: Box::new(SessionError::NotAMosaic),
        }));
        assert!(m.is_selected(&at("b.ARW")));
    }

    #[test]
    fn a_range_drops_its_anchor_when_its_open_fails_after_a_fact_told_the_open() {
        use apertrust_engine::session::Selection;

        let at = |name: &str| PathBuf::from("/photos").join(name);
        let order: Vec<PathBuf> = ["a.ARW", "b.ARW", "c.ARW"].map(at).to_vec();
        let mut m = mirror();
        m.opening(at("a.ARW"), 1, Instant::now());
        let range = SelectionCommand::Range {
            to: at("c.ARW"),
            order,
        };
        m.selecting(&range, 2);
        // The session tells the open, A active, before the range and before A fails.
        m.apply(Event::Selection(Selection {
            photos: vec![at("a.ARW")],
            active: Some(at("a.ARW")),
            previous: false,
            through: 1,
        }));
        m.apply(Event::Failed(SessionError::Open {
            open: 1,
            error: Box::new(SessionError::NotAMosaic),
        }));
        assert!(!m.is_selected(&at("b.ARW")));
        assert!(m.is_selected(&at("c.ARW")));
    }

    /// The mirror keeps the session's latest view by its `through`, one arriving late
    /// dropped, and keeps it as another photo opens (decision 99).
    #[test]
    fn the_mirror_keeps_the_session_s_latest_view() {
        use apertrust_engine::session::{View, ViewScale, ViewState};
        use apertrust_engine::view::place;

        let told = |scale, centre, through| {
            let view = View { scale, centre };
            Event::View(ViewState {
                through,
                view,
                placement: Some(place([6000, 4000], (2400, 1600), &view)),
            })
        };
        let mut m = mirror();
        assert_eq!(m.view, None);
        m.apply(told(ViewScale::Factor(1.0), [0.3, 0.4], 2));
        m.apply(told(ViewScale::Factor(2.0), [0.6, 0.5], 4));
        // An earlier one, late.
        m.apply(told(ViewScale::Fit, [0.5, 0.5], 3));
        let view = m.view.expect("the session's view");
        assert_eq!(view.through, 4);
        assert_eq!(view.view.scale, ViewScale::Factor(2.0));
        assert_eq!(view.placement.map(|p| p.render), Some(1.0));
        // Not a photo's: another photo opening keeps it.
        m.opening(PathBuf::from("/photos/b.ARW"), 5, Instant::now());
        assert_eq!(m.view.map(|v| v.through), Some(4));
    }

    /// The session's selection telling `photos` selected, `active` active and an earlier
    /// selection kept, through `through`, the photos in `/photos`.
    fn told_selection(photos: &[&str], active: Option<&str>, through: u64) -> Event {
        let at = |photo: &str| PathBuf::from("/photos").join(photo);
        Event::Selection(apertrust_engine::session::Selection {
            photos: photos.iter().map(|photo| at(photo)).collect(),
            active: active.map(at),
            previous: true,
            through,
        })
    }

    /// A Previous Selection sent: no edit is sent until the session tells it, since it may
    /// make another photo active; then the photo it made active shows, opening under the
    /// command's number, at the first fact to include it, though a later command sent with
    /// it, a Select All that changes nothing and so tells nothing, is not yet included.
    #[test]
    fn previous_selection_shows_the_session_s_photo_at_the_first_fact_to_include_it() {
        let mut m = mirror();
        m.opening(PathBuf::from("/photos/a.ARW"), 1, Instant::now());
        m.apply(Event::Opened(photo(1)));
        m.apply(Event::Values(report(0.0, 1)));
        assert!(m.takes_edits());
        m.selecting(&SelectionCommand::Previous, 2);
        assert!(!m.takes_edits(), "another photo may be active");
        m.selecting(&SelectionCommand::All, 3);
        m.apply(told_selection(&["b.ARW", "c.ARW"], Some("c.ARW"), 2));
        assert!(matches!(&m.photo,
            PhotoState::Opening { path, open: 2 } if *path == Path::new("/photos/c.ARW")));
        // Its own events then come, not dropped.
        let opened = Photo {
            path: PathBuf::from("/photos/c.ARW"),
            ..photo(2)
        };
        m.apply(Event::Opened(opened));
        assert!(matches!(&m.photo, PhotoState::Open(photo) if photo.open == 2));
        // A photo asked for after it wins; a refused one shows nothing.
        m.selecting(&SelectionCommand::Previous, 4);
        m.opening(PathBuf::from("/photos/d.ARW"), 5, Instant::now());
        m.apply(told_selection(&["a.ARW"], Some("a.ARW"), 4));
        assert!(matches!(&m.photo, PhotoState::Opening { open: 5, .. }));
        m.selecting(&SelectionCommand::Previous, 6);
        let error = Box::new(SessionError::NoPreviousSelection);
        m.apply(Event::Failed(SessionError::Refused { command: 6, error }));
        m.apply(told_selection(&["e.ARW"], Some("e.ARW"), 7));
        assert!(matches!(&m.photo, PhotoState::Opening { open: 5, .. }));
    }

    /// A failed write of the open photo reported before the fact that tells Previous
    /// Selection's other photo is kept apart as the photo left's, its edit not on disk.
    #[test]
    fn a_failed_write_reported_before_previous_selection_s_photo_is_kept() {
        let mut m = mirror();
        let a = PathBuf::from("/photos/a.ARW");
        m.opening(a.clone(), 1, Instant::now());
        m.apply(Event::Opened(photo(1)));
        m.selecting(&SelectionCommand::Previous, 2);
        m.apply(Event::Failed(SessionError::Save {
            path: sidecar::path_for(&a),
            through: 2,
            source: std::io::Error::other("disk full"),
        }));
        m.apply(told_selection(&["b.ARW"], Some("b.ARW"), 2));
        assert!(matches!(m.photo, PhotoState::Opening { open: 2, .. }));
        assert_eq!(m.saving.failed(), Some("disk full"));
        let (left, _) = m.saving.left.as_ref().expect("kept apart");
        assert_eq!(*left, sidecar::path_for(&a));
        // Closed by it too.
        m.apply(Event::Opened(Photo {
            path: PathBuf::from("/photos/b.ARW"),
            ..photo(2)
        }));
        m.selecting(&SelectionCommand::Previous, 3);
        let b = sidecar::path_for(Path::new("/photos/b.ARW"));
        m.apply(Event::Failed(SessionError::Save {
            path: b.clone(),
            through: 3,
            source: std::io::Error::other("read-only"),
        }));
        m.apply(told_selection(&[], None, 3));
        assert!(matches!(m.photo, PhotoState::None));
        assert_eq!(m.saving.left, Some((b, "read-only".to_owned())));
    }

    #[test]
    fn the_mirror_keeps_the_session_s_latest_selection() {
        use apertrust_engine::session::Selection;

        let told = |photos: &[&str], active: Option<&str>, through| {
            let at = |photo: &&str| PathBuf::from("/photos").join(photo);
            Event::Selection(Selection {
                photos: photos.iter().map(at).collect(),
                active: active.as_ref().map(at),
                previous: false,
                through,
            })
        };
        let mut m = mirror();
        assert_eq!(m.selection, Selection::default());
        m.opening(PathBuf::from("/photos/a.ARW"), 1, Instant::now());
        m.apply(told(&["a.ARW"], Some("a.ARW"), 1));
        assert_eq!(m.selection.photos, [PathBuf::from("/photos/a.ARW")]);
        m.apply(told(&["a.ARW", "b.ARW"], Some("a.ARW"), 3));
        // An earlier one, late.
        m.apply(told(&["a.ARW"], Some("a.ARW"), 2));
        assert_eq!(m.selection.photos.len(), 2);
        assert_eq!(m.selection.through, 3);
        // As the Browser's cells mark them.
        assert!(m.is_selected(Path::new("/photos/b.ARW")));
        assert!(!m.is_selected(Path::new("/photos/c.ARW")));
        // The photo closed: the selection the session tells with no active photo.
        m.closing();
        m.apply(told(&["b.ARW"], None, 4));
        assert_eq!(m.selection.photos, [PathBuf::from("/photos/b.ARW")]);
        assert_eq!(m.selection.active, None);
        assert!(!m.is_selected(Path::new("/photos/a.ARW")));
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
                title: Message::new("job-folder"),
                cancellable: true,
                destination: None,
                detail: None,
                progress: Default::default(),
                retries: None,
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
                outcome: None,
                failure: None,
                at: std::time::SystemTime::UNIX_EPOCH,
                offers: Vec::new(),
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
        m.opening(PathBuf::from("/photos/b.ARW"), 3, Instant::now());
        m.apply(folder_opened(2, "/photos"));
        // The listing's first chunk has not reached the photo yet.
        m.apply(listed(2, &["a.ARW"], false));
        assert_eq!(m.folder.active(), None);
        m.apply(listed(2, &["b.ARW", "c.ARW"], true));
        assert_eq!(m.folder.active(), Some(1));
        // Another photo of the folder opens, and then one of another folder.
        m.opening(PathBuf::from("/photos/c.ARW"), 4, Instant::now());
        assert_eq!(m.folder.active(), Some(2));
        m.opening(PathBuf::from("/elsewhere/c.ARW"), 5, Instant::now());
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
        m.opening(PathBuf::from("/photos/a.ARW"), 3, Instant::now());
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

    /// A frame including `through` commands, on a buffer of the GPU's; `None` with no
    /// adapter.
    fn gpu_frame(through: u64) -> Option<Frame> {
        use apertrust_engine::compute::Timings;
        use apertrust_engine::gpu::{GpuContext, GpuOptions};
        use apertrust_engine::session::FramePlace;
        let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default())).ok()?;
        Some(Frame {
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
            place: FramePlace {
                scale: 1.0,
                origin: [0, 0],
                extent: [2, 2],
            },
            overview: None,
            detail_pending: false,
        })
    }

    /// A picture `side` pixels square.
    fn square(side: u32) -> Thumbnail {
        Thumbnail {
            width: side,
            height: side,
            pixels: vec![100; (side * side * 4) as usize].into(),
        }
    }

    /// The picture of the photo opened `open`, from `source`.
    fn open_preview(open: u64, picture: Option<Thumbnail>, source: PreviewSource) -> Event {
        Event::OpenPreview(OpenPreview {
            open,
            picture,
            source,
        })
    }

    /// `report`'s values, not yet live.
    fn not_live(exposure: f32, through: u64) -> Values {
        Values {
            live: false,
            ..report(exposure, through)
        }
    }

    /// A photo opens in stages (decision 77): in the click's frame the largest thumbnail
    /// the folder holds; the session's picture where larger; its values read-only, every
    /// control disabled, nothing taken; then live, the picture gone with the first frame
    /// and the controls enabled in the same apply.
    #[test]
    fn a_photo_opens_in_stages_read_only_until_it_is_live() {
        let mut m = mirror();
        m.folder.opening(PathBuf::from("/photos/Wedding"), 1);
        m.apply(folder_opened(1, "/photos/Wedding"));
        m.apply(listed(1, &["a.ARW"], true));
        for side in [128, 256] {
            m.apply(Event::Library(LibraryEvent::Thumbnail {
                open: 1,
                photo: "a.ARW".to_owned(),
                size: side,
                image: Some(square(side)),
            }));
        }
        let now = Instant::now();
        m.opening(PathBuf::from("/photos/Wedding/a.ARW"), 5, now);
        let shown = |m: &Mirror| m.picture().map(|(p, _)| (p.image.width, p.rendered));
        assert_eq!(shown(&m), Some((256, false)));
        assert_eq!(m.shown(), Shown::Nothing);
        // The session's picture, larger, takes its place.
        m.apply(open_preview(5, Some(square(1024)), PreviewSource::Embedded));
        assert_eq!(shown(&m), Some((1024, false)));
        // Stage 2: the values, read-only.
        m.apply(Event::Opened(Photo {
            path: PathBuf::from("/photos/Wedding/a.ARW"),
            ..photo(5)
        }));
        m.apply(Event::Values(not_live(1.5, 6)));
        assert_eq!(m.shown(), Shown::Opening);
        assert!(!m.takes_edits() && !m.shown().editable() && !m.live());
        let exposure = &m.values.tools[0].controls[0];
        assert_eq!(exposure.value, ControlValue::Number(1.5));
        assert!(!exposure.available);
        assert_eq!(m.opening_name().as_deref(), Some("a.ARW"));
        assert_eq!(shown(&m), Some((1024, false)));
        // Stage 3: live, the controls and the frame in one burst.
        m.apply(Event::Values(report(1.5, 6)));
        let Some(frame) = gpu_frame(6) else {
            return;
        };
        // The swap the Viewer fades, once: an edit's frame after it is not one.
        let generation = m.picture().map(|(_, generation)| generation).unwrap();
        assert_eq!(
            m.apply(Event::Frame(frame)),
            [Reaction::Swapped {
                open: 5,
                generation
            }]
        );
        assert_eq!(m.apply(Event::Frame(gpu_frame(7).unwrap())), []);
        assert_eq!(m.shown(), Shown::Photo { editable: true });
        assert!(m.takes_edits() && m.live());
        assert!(m.values.tools[0].controls[0].available);
        assert_eq!(m.picture(), None);
        assert_eq!(m.opening_name(), None);
        assert!(!m.rendering(now + RENDERING_AFTER));
        // Another photo, with no picture in hand: its first frame replaces the last photo's
        // frame by a cut, never a fade.
        m.opening(PathBuf::from("/photos/Wedding/b.ARW"), 8, now);
        assert!(m.frame.is_none() && m.picture().is_none());
        m.apply(Event::Opened(Photo {
            path: PathBuf::from("/photos/Wedding/b.ARW"),
            ..photo(8)
        }));
        m.apply(Event::Values(report(0.0, 9)));
        assert_eq!(m.apply(Event::Frame(gpu_frame(9).unwrap())), []);
    }

    /// The session's picture is taken for the latest open alone, before it is live: an
    /// earlier open's, and one after the live frame, are dropped; a rendered one replaces
    /// even a larger picture.
    #[test]
    fn a_stale_open_preview_is_dropped() {
        let mut m = mirror();
        m.opening(PathBuf::from("b.ARW"), 4, Instant::now());
        m.opening(PathBuf::from("c.ARW"), 5, Instant::now());
        m.apply(open_preview(4, Some(square(512)), PreviewSource::Embedded));
        assert_eq!(m.picture(), None);
        m.apply(open_preview(5, Some(square(512)), PreviewSource::Embedded));
        m.apply(open_preview(5, Some(square(256)), PreviewSource::Rendered));
        assert_eq!(
            m.picture().map(|(p, _)| (p.image.width, p.rendered)),
            Some((256, true))
        );
        m.apply(Event::Opened(photo(5)));
        m.apply(Event::Values(report(0.0, 6)));
        m.apply(open_preview(5, Some(square(1024)), PreviewSource::Rendered));
        assert_eq!(
            m.picture().map(|(p, _)| p.image.width),
            Some(256),
            "after live"
        );
    }

    /// Rendering… only once the photo is still not live a second after it was asked for,
    /// and never once it is (decision 77).
    #[test]
    fn rendering_shows_only_a_second_after_the_click_while_not_live() {
        let mut m = mirror();
        let asked = Instant::now();
        assert!(!m.rendering(asked + RENDERING_AFTER));
        m.opening(PathBuf::from("b.ARW"), 2, asked);
        assert!(!m.rendering(asked));
        assert!(!m.rendering(asked + RENDERING_AFTER - Duration::from_millis(1)));
        assert!(m.rendering(asked + RENDERING_AFTER));
        assert_eq!(m.rendering_from(), Some(asked + RENDERING_AFTER));
        // Its values, not live: still rendering.
        m.apply(Event::Opened(photo(2)));
        m.apply(Event::Values(not_live(0.0, 3)));
        assert!(m.rendering(asked + 2 * RENDERING_AFTER));
        // Live: gone.
        m.apply(Event::Values(report(0.0, 3)));
        assert!(!m.rendering(asked + 2 * RENDERING_AFTER));
        assert_eq!(m.rendering_from(), None);
    }

    /// A change refused because the photo was not live is dropped with nothing said; an
    /// earlier open's failure is no concern of this one's; a decode failing after `Opened`
    /// fails the open, its picture gone.
    #[test]
    fn a_not_live_refusal_shows_nothing_and_a_late_failure_ends_the_open() {
        let mut m = mirror();
        m.opening(PathBuf::from("b.ARW"), 2, Instant::now());
        m.apply(open_preview(2, Some(square(64)), PreviewSource::Embedded));
        m.apply(Event::Opened(photo(2)));
        m.apply(Event::Values(not_live(0.0, 3)));
        m.apply(Event::Failed(SessionError::Refused {
            command: 3,
            error: Box::new(SessionError::NotLive),
        }));
        assert_eq!(m.failure, None);
        let failed = |open| {
            Event::Failed(SessionError::Open {
                open,
                error: Box::new(SessionError::NotAMosaic),
            })
        };
        m.apply(failed(1));
        assert!(matches!(m.photo, PhotoState::Open(_)));
        m.apply(failed(2));
        assert!(matches!(m.photo, PhotoState::Failed { .. }));
        assert_eq!(m.picture(), None);
        assert_eq!(m.shown(), Shown::Nothing);
        assert_eq!(m.saving.state(Instant::now()), SaveState::None);
    }

    /// A real session on the test's GPU, its raws synthetic (`session::testing`), whose
    /// events the mirror takes in as the app does; `None` with no adapter.
    pub(crate) struct Live {
        pub(crate) session: apertrust_engine::session::Session,
        /// The session's device, which draws its frames as the window does.
        pub(crate) gpu: apertrust_engine::gpu::GpuContext,
        wakes: std::sync::mpsc::Receiver<()>,
        pub(crate) mirror: Mirror,
        /// What the Tools panel showed, whether a picture, and the photo's path, after each
        /// event, a state shown again left out.
        seen: Vec<(Shown, bool, Option<PathBuf>)>,
    }

    impl Live {
        pub(crate) fn start() -> Option<Live> {
            use apertrust_engine::gpu::{GpuContext, GpuOptions};
            let Ok(gpu) = pollster::block_on(GpuContext::new(&GpuOptions::default())) else {
                eprintln!("no GPU adapter: a session needs one");
                return None;
            };
            let (woken, wakes) = std::sync::mpsc::channel();
            let session = apertrust_engine::session::testing::start(
                gpu.clone(),
                apertrust_operations::OPERATIONS,
                apertrust_tools::TOOLS,
                "0.4.0+12~g1a2b3c4",
                move || {
                    let _ = woken.send(());
                },
            );
            session.send(apertrust_engine::session::Command::Resize {
                width: 64,
                height: 48,
            });
            Some(Live {
                session,
                gpu,
                wakes,
                mirror: mirror(),
                seen: Vec::new(),
            })
        }

        /// Asks for the photo at `path`, as the app does.
        pub(crate) fn open(&mut self, path: &Path) -> u64 {
            let command = apertrust_engine::session::Command::Open(path.to_owned());
            let open = self.session.send(command);
            self.mirror.opening(path.to_owned(), open, Instant::now());
            open
        }

        /// Takes in the session's events until the photo is live with its first frame.
        pub(crate) fn until_live(&mut self) {
            let deadline = Instant::now() + Duration::from_secs(600);
            while !(self.mirror.live() && self.mirror.frame.is_some()) {
                assert!(Instant::now() < deadline, "live in time: {:?}", self.seen);
                let _ = self.wakes.recv_timeout(Duration::from_millis(50));
                for event in self.session.events() {
                    self.mirror.apply(event);
                    let m = &self.mirror;
                    let state = (
                        m.shown(),
                        m.picture().is_some(),
                        m.photo.path().map(Path::to_owned),
                    );
                    if self.seen.last() != Some(&state) {
                        self.seen.push(state);
                    }
                }
            }
        }

        /// Takes in the session's events until `done` holds of the mirror.
        pub(crate) fn until(&mut self, done: impl Fn(&Mirror) -> bool) {
            let deadline = Instant::now() + Duration::from_secs(600);
            while !done(&self.mirror) {
                assert!(
                    Instant::now() < deadline,
                    "in time: {:?}",
                    self.mirror.history
                );
                let _ = self.wakes.recv_timeout(Duration::from_millis(50));
                for event in self.session.events() {
                    self.mirror.apply(event);
                }
            }
        }

        /// Exposure's `control` as the session last reported it.
        fn exposure(&self, control: &str) -> Option<ControlValue> {
            let state = (self.mirror.values.tools.iter()).find(|m| m.tool == "exposure")?;
            let shown = state.controls.iter().find(|c| c.control == control)?;
            Some(shown.value.clone())
        }
    }

    /// Through a real session: a photo's steps, saved in its sidecar (step 9.5), are in the
    /// History tab again once it is opened again, after another photo, each a line of its
    /// own, the end on the last.
    #[test]
    fn a_real_session_s_reopened_photo_shows_its_steps_in_the_history_tab() {
        use apertrust_engine::session::Command;

        let Some(mut live) = Live::start() else {
            return;
        };
        let folder = scratch("history-reopened");
        let (path, other) = (folder.join("steps.ARW"), folder.join("other.ARW"));
        live.open(&path);
        live.until_live();
        let set = |control: &str, value| Command::SetControl {
            tool: "exposure".to_owned(),
            control: control.to_owned(),
            value: ControlValue::Number(value),
            done: true,
        };
        // A new photo's default steps, then the two changes'.
        let n = apertrust_engine::history::default_tools(
            apertrust_tools::TOOLS,
            apertrust_operations::OPERATIONS,
        )
        .len();
        live.session.send(set("exposure", 1.0));
        live.session.send(set("black_level", 0.01));
        live.until(|m| m.history.as_ref().is_some_and(|h| h.steps.len() == n + 2));
        let ids: Vec<_> = (live.mirror.history.as_ref().unwrap().steps.iter())
            .map(|step| step.id)
            .collect();
        live.open(&other);
        live.until_live();
        live.open(&path);
        live.until_live();
        live.until(|m| m.history.as_ref().is_some_and(|h| h.steps.len() == n + 2));
        let history = live.mirror.history.clone().expect("the photo's history");
        let read: Vec<_> = history.steps.iter().map(|step| step.id).collect();
        assert_eq!((read, history.end), (ids, n + 2));
        let words = apertrust_ui::Translator::new(apertrust_ui::Locale::EnGb);
        let today = crate::history::today();
        let listed = crate::history::listed(Some(&history), live.mirror.tools(), &words, today);
        let steps = (listed.lines())
            .filter(|line| matches!(line.place, crate::history::Place::Step { .. }))
            .count();
        assert_eq!(steps, n + 2, "{listed:?}");
        // The default steps read back numbered 1 to n, the Defaults group's, the changes'
        // after them.
        let defaults: Vec<usize> = (listed.entries.iter())
            .find_map(|entry| match entry {
                crate::history::Entry::Group {
                    key: crate::history::GroupKey::Defaults,
                    lines,
                    ..
                } => Some(
                    (lines.iter())
                        .filter_map(|line| match line.place {
                            crate::history::Place::Step { number, .. } => Some(number),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .expect("the Defaults group");
        assert_eq!(defaults, (1..=n).rev().collect::<Vec<_>>(), "{listed:?}");
        drop(live);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Through a real session: a plain click on a step in the History section, the app's
    /// own drawing of the session's history, asks to go to it, and the session, sent that,
    /// takes the edit back to that step; Shift-click asks to open its tool and nothing
    /// else.
    #[test]
    fn a_plain_click_on_a_step_takes_a_real_session_s_edit_back_to_it() {
        use apertrust_engine::session::Command;
        use egui_kittest::Harness;
        use egui_kittest::kittest::{NodeT as _, Queryable as _};

        let Some(mut live) = Live::start() else {
            return;
        };
        let folder = scratch("history-click");
        let path = folder.join("steps.ARW");
        live.open(&path);
        live.until_live();
        // Two steps, on two controls, so they never merge.
        let set = |control: &str, value| Command::SetControl {
            tool: "exposure".to_owned(),
            control: control.to_owned(),
            value: ControlValue::Number(value),
            done: true,
        };
        // After a new photo's default steps.
        let n = apertrust_engine::history::default_tools(
            apertrust_tools::TOOLS,
            apertrust_operations::OPERATIONS,
        )
        .len();
        live.session.send(set("exposure", 1.0));
        live.session.send(set("black_level", 0.01));
        live.until(|m| m.history.as_ref().is_some_and(|h| h.steps.len() == n + 2));
        assert!(live.mirror.takes_edits());
        let history = live.mirror.history.clone().expect("the photo's history");
        let first = history.steps[n].id;
        // Black level as the second step found it: its value at the first.
        let before = history.steps[n + 1].changes[0].controls[0].before.clone();
        let (first_name, second_name) = (
            format!("Step {}: Exposure", n + 1),
            format!("Step {}: Exposure", n + 2),
        );

        struct Scene {
            live: Live,
            actions: Vec<crate::history::Action>,
            commands: Vec<Command>,
        }
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(320.0, 600.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, apertrust_ui::Theme::default()))
            .build_ui_state(
                |ui, scene: &mut Scene| {
                    // As the History tab draws it, through its `UtilityTool`.
                    let settings = crate::settings::Settings::default();
                    let words = apertrust_ui::translator(ui.ctx());
                    let mut cx = crate::utility::Context {
                        mirror: &scene.live.mirror,
                        settings: &settings,
                        words: &words,
                        commands: &mut scene.commands,
                    };
                    let mut tool = crate::history::HistoryTool { alone: true };
                    let head = crate::tools::Head {
                        open: true,
                        ..crate::tools::Head::default()
                    };
                    let (_, (_, asked)) = crate::history::tab(ui, head, false, |ui| {
                        crate::utility::rows(ui, &mut tool, &mut cx)
                    });
                    scene.actions.extend(asked);
                },
                Scene {
                    live,
                    actions: Vec::new(),
                    commands: Vec::new(),
                },
            );
        harness.run();
        // A line by its number and tool, whatever it says changed after them.
        fn item<'a>(harness: &'a Harness<'_, Scene>, name: &str) -> egui_kittest::Node<'a> {
            let named = |node: &egui_kittest::Node<'_>| {
                let label = node.accesskit_node().label().unwrap_or_default();
                label.starts_with(&format!("{name}, "))
            };
            (harness.query_all_by_role(eframe::egui::accesskit::Role::TreeItem))
                .find(named)
                .unwrap_or_else(|| panic!("no line {name}"))
        }
        item(&harness, &first_name).click();
        harness.run();
        let go_to = Command::GoTo { step: Some(first) };
        assert_eq!(harness.state().commands, std::slice::from_ref(&go_to));
        assert!(harness.state().actions.is_empty());
        // The app sends it; the session goes there.
        let live = &mut harness.state_mut().live;
        live.session.send(go_to);
        live.until(|m| m.history.as_ref().is_some_and(|h| h.end == n + 1));
        assert_eq!(live.exposure("exposure"), Some(ControlValue::Number(1.0)));
        assert_eq!(live.exposure("black_level"), Some(before));
        // Shift-click: the tool, and no step gone to.
        harness.state_mut().actions.clear();
        harness.run();
        item(&harness, &second_name).click_modifiers(eframe::egui::Modifiers::SHIFT);
        harness.run();
        assert_eq!(
            harness.state().actions,
            [crate::history::Action::Open("exposure")]
        );
        drop(harness);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A folder of the test's own, emptied first.
    pub(crate) fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("apertrust-mirror-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// Through a real session (decision 77): the picture, the values read-only, then live
    /// with the first frame, in that order.
    #[test]
    fn a_real_session_opens_a_photo_in_stages() {
        let Some(mut live) = Live::start() else {
            return;
        };
        let folder = scratch("stages");
        let path = folder.join("embedded.ARW");
        live.open(&path);
        live.until_live();
        let shown: Vec<(Shown, bool)> = live.seen.iter().map(|(s, p, _)| (*s, *p)).collect();
        let opening = shown.iter().position(|s| *s == (Shown::Opening, true));
        let photo = shown
            .iter()
            .position(|s| s.0 == Shown::Photo { editable: true });
        assert!(
            matches!((opening, photo), (Some(o), Some(p)) if o < p),
            "{shown:?}"
        );
        assert_eq!(
            shown.last(),
            Some(&(Shown::Photo { editable: true }, false))
        );
        drop(live);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Through a real session: a photo whose edit a newer Apertrust wrote goes live for
    /// viewing only (decision 43).
    #[test]
    fn a_real_session_s_view_only_photo_ends_not_editable() {
        let Some(mut live) = Live::start() else {
            return;
        };
        let folder = scratch("view-only");
        let path = folder.join("newer.ARW");
        let xml = apertrust_engine::sidecar::to_xml(
            &apertrust_engine::edit::Edit::default(),
            apertrust_operations::OPERATIONS,
            apertrust_tools::TOOLS,
            "9.0.0",
        )
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
        std::fs::write(sidecar::path_for(&path), xml).unwrap();
        live.open(&path);
        live.until_live();
        assert_eq!(live.mirror.shown(), Shown::Photo { editable: false });
        assert!(!live.mirror.takes_edits());
        drop(live);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Through a real session, switching quickly A → B → C: only C is shown, whatever A's
    /// and B's jobs still send.
    #[test]
    fn a_real_session_switched_quickly_shows_only_the_last_photo() {
        let Some(mut live) = Live::start() else {
            return;
        };
        let folder = scratch("switch");
        let [a, b, c] = ["a.ARW", "b.ARW", "c.ARW"].map(|name| folder.join(name));
        live.open(&a);
        live.open(&b);
        let last = live.open(&c);
        live.until_live();
        assert_eq!(live.mirror.photo.open(), Some(last));
        assert!(
            live.seen
                .iter()
                .all(|(_, _, path)| path.as_deref() == Some(c.as_path())),
            "{:?}",
            live.seen
        );
        drop(live);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A photo opened alone (`docs/library.md`, One photo): the Browser holds it by
    /// itself, its cell following its edit, while no folder is shown and nothing reaches
    /// the recents, whatever the photo's events say. Open This Folder then opens its folder,
    /// which joins the recents, the photo active in it.
    #[test]
    fn a_photo_alone_lists_nothing_and_its_folder_opens_with_it_active() {
        let mut m = mirror();
        let path = PathBuf::from("/photos/a.ARW");
        m.folder.alone(1, vec![path.clone()]);
        // No thumbnail until one is asked for and comes.
        let key = crate::folder::CellKey::Path(path.clone());
        assert_eq!(m.folder.thumbnail(&key, 128), None);
        m.opening(path.clone(), 1, Instant::now());
        let mut reactions = m.apply(Event::Opened(photo(1)));
        reactions.extend(m.apply(Event::Values(report(1.0, 1))));
        assert_eq!(reactions, []);
        let [photo] = m.folder.photos() else {
            panic!("not one photo alone: {:?}", m.folder.folder);
        };
        assert_eq!(photo.name, "a.ARW");
        assert_eq!(photo.edit, EditState::Edited);
        assert_eq!(m.folder.active_shown(), Some(0));
        assert_eq!(m.folder.shown(), None);
        // Open This Folder: the photo stays open, active once its folder is listed.
        let (_, folder) = m.folder.alone_active().expect("its folder");
        let folder = folder.to_owned();
        assert_eq!(folder, PathBuf::from("/photos"));
        m.folder.opening(folder, 2);
        assert_eq!(
            m.apply(folder_opened(2, "/photos")),
            [Reaction::FolderOpened(PathBuf::from("/photos"))]
        );
        m.apply(listed(2, &["0.ARW", "a.ARW"], true));
        assert_eq!(m.folder.active(), Some(1));
        assert!(matches!(m.photo, PhotoState::Open(_)));
    }
}
