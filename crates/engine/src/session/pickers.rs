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

//! The session's colour pickers (decision 79; `docs/pickers.md`, Through the session):
//! view state, never in the edit. One active picker, a module's or the Colour picker
//! module's, and each picker's last point and area, kept in the original image's
//! coordinates for the session's life ([`crate::picker::Place`]).
//!
//! The active picker is measured on the frame, in a submission of its own after each
//! frame's copy and the scopes' count, and on the frame shown when its region changes
//! with no render. A module's picker is measured at its operation in the live viewport
//! pipe too: right after the operation's last pass in a render's encoder while what it
//! reads changes, or, where its area moves with no edit, by a sample-only render from the
//! newest cache point at or before the operation, its operation the edited instance.
//! Readings come back without the thread waiting, as the scopes' do, and go out as
//! [`Event::Picker`] when they change what a client shows: after the frame they measured.
//!
//! The session applies a module's reading itself, through the module's pure
//! [`apertrust_modules::Module::pick`]: the first after the picker turns on and each after
//! its region moves, never one an edit upstream caused, a drag's readings one gesture. A
//! failure turns the pickers off for the session, never the frame.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};

use apertrust_modules::pick::Picked;
use apertrust_modules::{ControlValue, Controls, Picker, PickerKind};
use apertrust_plugin::{Format, ImageContext, ImageDesc};

use crate::compute::ComputeError;
use crate::edit::{self, EditError, panic_message};
use crate::history::{StepSource, Wrote};
use crate::picker::{
    self, Cfa, DEFAULT_AREA, DEFAULT_POINT, FrameReading, Geometries, Group, Layout, ModuleReading,
    PickKeys, PickerCommand, PickerReading, PickerRef, Place, Reading, Recorded, Reducer, Region,
    Stats,
};
use crate::pipeline::{Instance, Plan, Probe};

use super::{Engine, Event, SessionError};

/// The pickers' state, which the session keeps for its life.
#[derive(Default)]
pub(super) struct Pickers {
    /// The picker on, if any.
    active: Option<Active>,
    /// Each picker's last point and area this session, kept as it turns off for its next
    /// `On`.
    places: BTreeMap<PickerRef, Places>,
    /// The frame's reduction and its readback, made with the first measurement.
    pub(super) reducer: Option<Reducer<Sampled>>,
    /// The reduction at a module picker's operation, made as the first turns on.
    probe: Option<Reducer<Probed>>,
    /// Why the pickers failed, which turns them off for the session.
    failed: Option<String>,
    /// Readings to hand over, in the order of the commands they measured: those that
    /// needed no GPU, of a region that misses what it reads, and those arrived while one
    /// measured before them is still on its way.
    ready: Vec<Arrived>,
    /// The reading last sent while the active picker is on.
    sent: Option<PickerReading>,
    /// The active picker's latest module part and frame part.
    module: Option<ModuleReading>,
    frame: Option<FrameReading>,
    /// The tool settings each module's pickers read, as the client last sent them.
    tools: BTreeMap<&'static str, Vec<(&'static str, ControlValue)>>,
    /// Each module's last reading applied, for the photo, as RGB levels' `last_picked_color`:
    /// what its pick takes as `last`.
    last: BTreeMap<&'static str, Last>,
    /// Each module's latest reading measured while its picker was on, applied or not, for
    /// the photo: darktable's `picked_color`, which every run of the pipe overwrites and
    /// a tool setting's change applies again.
    latest: BTreeMap<&'static str, Last>,
    /// The drag a module picker's moves belong to: `On` begins one, and so does a move
    /// after the last one's release. Each drag's picks are one change, saved once.
    drag: u64,
    /// The newest drag released, which completes it and every one before it.
    released: u64,
    /// The drag whose picks wrote changes the writer has yet to be handed.
    gesture: Option<u64>,
    /// A module picker's commands that complete a change, `On`, `Switch`, `Reset` or a
    /// move's release, with no answer yet, each by its number and its drag, oldest first.
    /// One is answered by the first write handed over once its drag is complete, released
    /// with no reading of it still to come, or then by the edit reported held where it has
    /// nothing to write ([`Engine::settle`]), so a client always gets a `Saved` through it,
    /// and never one before its pick reaches the edit ([`Pickers::owed_through`]).
    pub(super) owed: Vec<(u64, u64)>,
}

/// The picker on, and whether it measures its area rather than its point.
#[derive(Debug, Clone, Copy)]
struct Active {
    picker: PickerRef,
    area: bool,
    /// A module's picker's state; `None` for the Colour picker's.
    module: Option<OnModule>,
}

/// A module's picker on.
#[derive(Debug, Clone, Copy)]
struct OnModule {
    module: &'static str,
    picker: &'static Picker,
    /// The channel tab the module shows.
    channel: Option<&'static str>,
    /// What its last measurement at the operation read: [`Probing::key`].
    measured: Option<u64>,
    /// The next reading to apply, with the command that asked for it.
    due: Option<Due>,
}

/// A reading to apply: the command that asked for it, the drag it belongs to, and the
/// keys held.
#[derive(Debug, Clone, Copy)]
pub(super) struct Due {
    command: u64,
    drag: u64,
    keys: PickKeys,
}

/// A module's last reading applied, with its picker and the keys held.
#[derive(Debug, Clone, Copy)]
struct Last {
    picker: &'static Picker,
    reading: Reading,
    keys: PickKeys,
}

/// A picker's last point and area, in the original image's coordinates.
#[derive(Debug, Default)]
struct Places {
    point: Option<Place>,
    area: Option<Place>,
}

impl Places {
    /// The place of the kind `area` names, darktable's default for it, mapped back
    /// through `geometries`, where it has none yet.
    fn of_kind(&mut self, area: bool, geometries: &Geometries) -> Place {
        let (place, default) = if area {
            (&mut self.area, DEFAULT_AREA)
        } else {
            (&mut self.point, DEFAULT_POINT)
        };
        *place.get_or_insert_with(|| Place::of(default, geometries))
    }
}

/// What the session keeps of a frame measured until its reading arrives.
#[derive(Debug, Clone, Copy)]
pub(super) struct Sampled {
    pub(super) through: u64,
    picker: PickerRef,
    region: Region,
}

/// What the session keeps of a module picker's measurement at its operation until its
/// reading arrives.
#[derive(Debug, Clone, Copy)]
pub(super) struct Probed {
    pub(super) through: u64,
    picker: PickerRef,
    region: Region,
    /// The reading's application, where it is due.
    due: Option<Due>,
}

/// A reading arrived: the frame's statistics under its region, or the operation's input's
/// and output's, `None` where the region misses what it reads.
enum Arrived {
    Frame(Sampled, Option<Stats>),
    Module(Probed, Option<Vec<Stats>>),
    /// No reading at the operation, which lies in the viewport's tiles.
    Unavailable(Probed),
}

impl Arrived {
    /// The commands it measured the result of.
    fn through(&self) -> u64 {
        match self {
            Arrived::Frame(sampled, _) => sampled.through,
            Arrived::Module(probed, _) | Arrived::Unavailable(probed) => probed.through,
        }
    }

    /// The application it carries, where due.
    fn due(&self) -> Option<Due> {
        match self {
            Arrived::Frame(..) => None,
            Arrived::Module(probed, _) | Arrived::Unavailable(probed) => probed.due,
        }
    }
}

/// What a module's picker measures at its operation in a plan.
pub(super) struct Probing {
    /// The operation, as an index into the plan's operations.
    pub(super) op: usize,
    /// What it reads: the input's and output's keys, the boxes, the denoise.
    key: u64,
    /// Its input's box, and its output's with the picker's `output`; `None` where the
    /// region misses the input, or where nothing is read.
    targets: Option<Vec<Target>>,
    /// The operation lies in the viewport's tiles, each holding one tile's part of its
    /// image at a time: no reading there.
    unavailable: bool,
    tag: Probed,
}

/// A buffer a module picker reads and its box.
struct Target {
    size: [u32; 2],
    /// The mosaic's pattern, for a mosaic.
    cfa: Option<Cfa>,
    denoise: bool,
    bounds: [u32; 4],
}

impl Probing {
    /// Whether the region misses the operation's input, so it reads nothing on the GPU.
    pub(super) fn missed(&self) -> bool {
        self.targets.is_none()
    }

    /// The reduction's buffers: the input, and the output where it reads it.
    fn groups<'a>(
        &'a self,
        targets: &'a [Target],
        buffers: [&'a wgpu::Buffer; 2],
    ) -> Vec<Group<'a>> {
        targets
            .iter()
            .zip(buffers)
            .map(|(target, buffer)| Group {
                buffer,
                size: target.size,
                layout: match &target.cfa {
                    Some(cfa) => Layout::Mosaic(cfa),
                    None => Layout::Rgba {
                        denoise: target.denoise,
                    },
                },
                boxes: std::slice::from_ref(&target.bounds),
            })
            .collect()
    }
}

/// What a picker command does beyond the pickers' own state.
#[derive(Debug, Default)]
pub(super) struct PickerEffect {
    /// The active picker's region changed: the frame shown is to be measured again.
    pub(super) moved: bool,
    /// The edit changed: the values are reported.
    pub(super) values: bool,
    /// The image may have changed.
    pub(super) render: bool,
    /// The change is complete: the edit is saved.
    pub(super) save: bool,
    /// A command owed an answer has nothing to write: where the edit needs no save, it is
    /// reported held, as after a cancel ([`Pickers::owed`]).
    pub(super) answer: bool,
}

impl Pickers {
    /// The active picker off, with what is on its way of it: nothing sent.
    pub(super) fn clear(&mut self) {
        self.active = None;
        self.sent = None;
        self.module = None;
        self.frame = None;
        self.ready.clear();
        self.gesture = None;
        if let Some(reducer) = &mut self.reducer {
            reducer.discard();
        }
        if let Some(probe) = &mut self.probe {
            probe.discard();
        }
    }

    /// Whether a picker is on.
    pub(super) fn on(&self) -> bool {
        self.active.is_some()
    }

    /// Whether a module's picker is on, measured at its operation in each render.
    pub(super) fn probing(&self) -> bool {
        self.active.is_some_and(|active| active.module.is_some())
    }

    /// The reduction at a module picker's operation, once one has turned on.
    pub(super) fn probe_mut(&mut self) -> Option<&mut Reducer<Probed>> {
        self.probe.as_mut()
    }

    /// Whether a reading is on its way, so the thread looks at the GPU between commands.
    pub(super) fn pending(&self) -> bool {
        !self.ready.is_empty()
            || self.reducer.as_ref().is_some_and(Reducer::pending)
            || self.probe.as_ref().is_some_and(Reducer::pending)
    }

    /// Drops the readings on their way, of a frame a GPU error dropped: the next render
    /// measures again at the operation.
    pub(super) fn discard(&mut self) {
        // The newest reading to apply, the one due or on its way, is asked for again, by
        // the next measurement, which stands for all of them: whether its drag is
        // complete is the drag's, kept apart from any reading.
        let held = (self.ready.iter().filter_map(Arrived::due))
            .chain((self.probe.iter()).flat_map(|probe| probe.on_its_way().filter_map(|p| p.due)))
            .chain(self.module_on().and_then(|on| on.due))
            .max_by_key(|due| (due.drag, due.command));
        self.ready.clear();
        if let Some(reducer) = &mut self.reducer {
            reducer.discard();
        }
        if let Some(probe) = &mut self.probe {
            probe.discard();
        }
        if let Some(on) = self
            .active
            .as_mut()
            .and_then(|active| active.module.as_mut())
        {
            on.measured = None;
            on.due = held;
        }
    }

    /// A module picker's reading that arrived while the History tab's eye hides a module,
    /// not applied: its application, `due`, waits, the newest of it and any due already
    /// kept, and what it read is measured again (decision 78).
    fn wait(&mut self, due: Option<Due>) {
        if let Some(on) = self
            .active
            .as_mut()
            .and_then(|active| active.module.as_mut())
        {
            on.measured = None;
            on.due = on
                .due
                .into_iter()
                .chain(due)
                .max_by_key(|due| (due.drag, due.command));
        }
    }

    /// The module's picker on, if one is.
    fn module_on(&self) -> Option<OnModule> {
        self.active.and_then(|active| active.module)
    }

    /// Whether a reading to apply of `drag` is still to come: due, on its way from the
    /// operation, or yet to be handed over.
    fn of_drag_to_come(&self, drag: u64) -> bool {
        let of = |due: Option<Due>| due.is_some_and(|due| due.drag == drag);
        of(self.module_on().and_then(|on| on.due))
            || self.ready.iter().any(|arrived| of(arrived.due()))
            || (self.probe.iter()).any(|probe| probe.on_its_way().any(|p| of(p.due)))
    }

    /// Whether `drag` is complete: released, with no reading of it still to come.
    fn complete(&self, drag: u64) -> bool {
        drag <= self.released && !self.of_drag_to_come(drag)
    }

    /// Whether a command owed an answer can have it: its drag complete.
    pub(super) fn answerable(&self) -> bool {
        (self.owed.iter()).any(|&(_, drag)| self.complete(drag))
    }

    /// What a write handed over now answers, as the commands it is through: every one
    /// applied, `applied`, but for a command owed an answer whose pick is still to come,
    /// which only a later write answers. Those it answers are owed no more.
    pub(super) fn owed_through(&mut self, applied: u64) -> u64 {
        let owed = std::mem::take(&mut self.owed);
        self.owed = owed
            .into_iter()
            .filter(|&(_, drag)| !self.complete(drag))
            .collect();
        self.owed
            .first()
            .map_or(applied, |&(command, _)| command - 1)
    }

    /// Whether the drag whose picks the writer has yet to be handed is complete, released
    /// with no reading of it still to come: then it is handed over, once.
    fn settled(&mut self) -> bool {
        let settled = self
            .gesture
            .is_some_and(|drag| drag <= self.released && !self.of_drag_to_come(drag));
        if settled {
            self.gesture = None;
        }
        settled
    }

    /// The commands the oldest reading still on its way measured: a reading measured
    /// after them waits for it, so each is handed over in order.
    fn horizon(&self) -> Option<u64> {
        let frame =
            (self.reducer.iter()).flat_map(|reducer| reducer.on_its_way().map(|s| s.through));
        let module = (self.probe.iter()).flat_map(|probe| probe.on_its_way().map(|p| p.through));
        frame.chain(module).min()
    }

    /// The module's last reading applied and its latest measured, for a test.
    #[cfg(test)]
    pub(super) fn readings_of(&self, module: &str) -> (Option<Reading>, Option<Reading>) {
        (
            self.last.get(module).map(|last| last.reading),
            self.latest.get(module).map(|latest| latest.reading),
        )
    }

    /// The tag of the reading at the operation waiting for a staging buffer, for a test.
    #[cfg(test)]
    pub(super) fn waiting_due(&self) -> Option<Option<(u64, u64)>> {
        let probed = self.probe.as_ref()?.waiting()?;
        Some(probed.due.map(|due| (due.command, due.drag)))
    }

    /// The drag the module picker's moves belong to, and the newest released, for a test.
    #[cfg(test)]
    pub(super) fn drags(&self) -> (u64, u64) {
        (self.drag, self.released)
    }

    /// Forgets what the photo gave: each module's last reading, as another photo opens or
    /// the photo closes.
    pub(super) fn forget_photo(&mut self) {
        self.last.clear();
        self.latest.clear();
    }

    /// The module picker's measurement that `recorded` made, now submitted: what it read
    /// becomes the last measured, and its reading arrives later. A reduction that could
    /// not be recorded is why the pickers fail.
    fn submitted(
        &mut self,
        probing: Probing,
        recorded: Option<Result<Recorded, ComputeError>>,
    ) -> Option<String> {
        let recorded = match recorded? {
            Ok(recorded) => recorded,
            Err(error) => return Some(error.to_string()),
        };
        if let Some(probe) = &mut self.probe {
            probe.submitted(recorded, probing.tag);
        }
        self.measured(probing.key);
        None
    }

    /// What the module picker on reads now is `key`: its due reading is on its way.
    fn measured(&mut self, key: u64) {
        if let Some(on) = self
            .active
            .as_mut()
            .and_then(|active| active.module.as_mut())
        {
            on.measured = Some(key);
            on.due = None;
        }
    }
}

/// The measurement the module picker on makes at its operation in `plan`, where what it
/// reads differs from what it read last; `instances` bind the edit, which `plan` plans,
/// for the photo `image`. `None` with no module picker on, nothing new to read, or its
/// operation not in the plan.
pub(super) fn probing(
    pickers: &mut Pickers,
    plan: &Plan,
    instances: &[Instance],
    image: &ImageContext,
    through: u64,
) -> Option<Probing> {
    let active = pickers.active?;
    let on = active.module?;
    let operation = on.picker.operation;
    let op = plan
        .operations
        .iter()
        .position(|planned| planned.id == operation)?;
    let before = instances
        .iter()
        .position(|instance| instance.operation.manifest().id == operation)?;
    let all = Geometries::of(instances, image).ok()?;
    let upstream = Geometries::of(&instances[..before], image).ok()?;
    let place = pickers
        .places
        .entry(active.picker)
        .or_default()
        .of_kind(active.area, &all);
    let at = place.region(&upstream);
    let tag = Probed {
        through,
        picker: active.picker,
        region: place.region(&all),
        due: on.due,
    };
    // In a tiled viewport the raw stage's operations from its split on run once a tile,
    // each over its tile's part of the image: none holds the whole of what the picker
    // reads, so it reads nothing there.
    if plan.tiled.as_ref().is_some_and(|tiles| tiles.contains(&op)) {
        let mut hasher = DefaultHasher::new();
        (op, "tiled").hash(&mut hasher);
        let key = hasher.finish();
        if on.measured == Some(key) {
            return None;
        }
        tracing::debug!(
            target: "pipe",
            "the picker {} reads nothing: {operation} runs in the viewport's tiles",
            on.picker.id
        );
        return Some(Probing {
            op,
            key,
            targets: None,
            unavailable: true,
            tag,
        });
    }
    let planned = &plan.operations[op];
    let input = match op.checked_sub(1) {
        Some(before) => (
            plan.operations[before].output_desc,
            plan.operations[before].key,
        ),
        None => (plan.input, 0),
    };
    let mut sides = vec![input];
    if on.picker.output {
        sides.push((planned.output_desc, planned.key));
    }
    let target = |desc: &ImageDesc| -> Option<Target> {
        let cfa = match desc.format {
            Format::Rgba => None,
            Format::Mosaic => Some(Cfa::of(image, desc)?),
            Format::Mosaic16 => return None,
        };
        Some(Target {
            size: [desc.width, desc.height],
            cfa,
            denoise: on.picker.denoise,
            bounds: picker::boxed(at, desc, image)?,
        })
    };
    let targets: Option<Vec<Target>> = sides.iter().map(|(desc, _)| target(desc)).collect();
    let mut hasher = DefaultHasher::new();
    (op, on.picker.denoise, on.picker.output).hash(&mut hasher);
    for ((_, key), target) in sides.iter().zip(targets.iter().flatten()) {
        (key, target.bounds).hash(&mut hasher);
    }
    targets.is_some().hash(&mut hasher);
    let key = hasher.finish();
    if on.measured == Some(key) {
        return None;
    }
    Some(Probing {
        op,
        key,
        targets,
        unavailable: false,
        tag,
    })
}

impl Engine {
    /// [`super::Command::Picker`]: what it changes beyond the pickers' state.
    /// A `Picker` with no reading goes into `events` as a picker turns off.
    pub(super) fn picker(
        &mut self,
        command: PickerCommand,
        events: &mut Vec<Event>,
    ) -> Result<PickerEffect, SessionError> {
        let command = match command {
            PickerCommand::Off => {
                self.picker_off(events);
                return Ok(PickerEffect::default());
            }
            PickerCommand::Channel { channel } => {
                if let Some(on) = (self.pickers.active.as_mut()).and_then(|a| a.module.as_mut()) {
                    on.channel = channel;
                }
                return Ok(PickerEffect::default());
            }
            PickerCommand::Tools { module, tools } => {
                return self.picker_tools(module, tools, events);
            }
            PickerCommand::On {
                picker,
                area,
                keys,
                tools,
                channel,
            } => return self.picker_on(picker, area, keys, tools, channel, events),
            command => command,
        };
        let mut active = self.pickers.active.ok_or(SessionError::NoPicker)?;
        let geometries = self.geometries()?;
        let places = self.pickers.places.entry(active.picker).or_default();
        let before = (active.area, places.of_kind(active.area, &geometries));
        let (done, keys) = match command {
            PickerCommand::Point { at, done, keys } => {
                places.point = Some(Place::of(Region::Point(at), &geometries));
                active.area = false;
                (done, keys)
            }
            PickerCommand::Area {
                from,
                to,
                done,
                keys,
            } => {
                let [from, to] = [from, to].map(picker::clamped);
                let area = [
                    from[0].min(to[0]),
                    from[1].min(to[1]),
                    from[0].max(to[0]),
                    from[1].max(to[1]),
                ];
                places.area = Some(Place::of(Region::Area(area), &geometries));
                active.area = true;
                (done, keys)
            }
            PickerCommand::Switch => {
                active.area = !active.area;
                (true, PickKeys::default())
            }
            PickerCommand::Reset if active.area => {
                places.area = Some(Place::of(DEFAULT_AREA, &geometries));
                (true, PickKeys::default())
            }
            PickerCommand::Reset => {
                places.point = Some(Place::of(DEFAULT_POINT, &geometries));
                (true, PickKeys::default())
            }
            PickerCommand::On { .. }
            | PickerCommand::Off
            | PickerCommand::Tools { .. }
            | PickerCommand::Channel { .. } => unreachable!("handled above"),
        };
        let after = (active.area, places.of_kind(active.area, &geometries));
        let mut effect = PickerEffect {
            moved: true,
            ..Default::default()
        };
        if let Some(on) = &mut active.module {
            let pickers = &mut self.pickers;
            if after != before {
                // A move after the last drag's release begins the next.
                if pickers.released == pickers.drag {
                    pickers.drag += 1;
                }
                // darktable's `_record_point_area`: a reading after a move is applied.
                on.due = Some(Due {
                    command: self.applied,
                    drag: pickers.drag,
                    keys,
                });
            }
            if done {
                pickers.released = pickers.drag;
                pickers.owed.push((self.applied, pickers.drag));
            }
        }
        self.pickers.active = Some(active);
        // A release with no reading of its drag still to come completes the drag with the
        // values already written; otherwise the drag's newest reading does, as it arrives.
        effect.save = self.pickers.settled();
        // One with nothing to write is answered all the same, the edit reported held.
        effect.answer = self.pickers.answerable();
        Ok(effect)
    }

    /// [`PickerCommand::On`].
    fn picker_on(
        &mut self,
        picker: PickerRef,
        area: bool,
        keys: PickKeys,
        tools: Vec<(&'static str, ControlValue)>,
        channel: Option<&'static str>,
        events: &mut Vec<Event>,
    ) -> Result<PickerEffect, SessionError> {
        if !self.live() {
            return Err(SessionError::NotLive);
        }
        if let Some(failed) = &self.pickers.failed {
            return Err(SessionError::Pickers(failed.clone()));
        }
        let mut effect = PickerEffect {
            moved: true,
            ..Default::default()
        };
        let mut module = None;
        if let PickerRef::Module {
            module: id,
            picker: name,
        } = picker
        {
            let open = self.open.as_ref().ok_or(SessionError::NotLive)?;
            if let Some(view_only) = &open.view_only {
                return Err(SessionError::ViewOnly(view_only.clone()));
            }
            // What it would read is the view's while the eye hides a module, and it writes
            // the edit (decision 78).
            if !open.hidden.is_empty() {
                return Err(SessionError::Hidden);
            }
            let found = edit::module(&self.modules, id).map_err(|_| SessionError::UnknownPicker)?;
            let declared = found
                .definition()
                .picker(name)
                .ok_or(SessionError::UnknownPicker)?;
            check_tools(found.definition().pickers, &tools)?;
            // Its module on, as darktable turns it on with its picker: its operation must
            // then be in the plan. With the history's end among the default steps it is the
            // first change, which applies the modules not applied (decision 78).
            let mut edit = open.edit.clone();
            edit.unapplied.clear();
            edit.turn_on(found);
            let bound = edit::instances(&edit, self.operations, &self.modules, &open.image);
            let planned = bound
                .instances
                .iter()
                .any(|instance| instance.operation.manifest().id == declared.operation);
            if !planned {
                return Err(SessionError::UnknownPicker);
            }
            if self.pickers.probe.is_none() {
                match pollster::block_on(Reducer::new(&self.gpu)) {
                    Ok(mut reducer) => {
                        reducer.keeping(keep_due);
                        self.pickers.probe = Some(reducer);
                    }
                    Err(error) => {
                        let error = error.to_string();
                        self.pickers_fail(error.clone(), events);
                        return Err(SessionError::Pickers(error));
                    }
                }
            }
            // Owed an answer from here, its drag still to come: no write handed over
            // while it turns on, the drag under way's or the picker it replaces', answers it.
            let owed = (self.applied, self.pickers.drag + 1);
            self.pickers.owed.push(owed);
            // A drag under way is saved before the module turns on, never with it.
            if self.pickers.gesture.take().is_some() {
                self.save_picks();
            }
            // A slider's gesture under way ends as a step of its own source first.
            self.end_gesture();
            let applies = (self.open.as_ref()).is_some_and(|open| !open.edit.unapplied.is_empty());
            let changed = self.change(|edit, _, _, _| {
                edit.unapplied.clear();
                edit.turn_on(found);
                Ok(())
            });
            let changed = match changed {
                Ok(changed) => changed,
                Err(error) => {
                    self.pickers.owed.retain(|&o| o != owed);
                    return Err(error);
                }
            };
            if changed {
                if let Some(open) = &mut self.open {
                    open.gesture = None;
                }
                // With the end among the default steps it applies them, darktable's first
                // change: a step of its module, as darktable's picker turning its module on
                // is a history item, saved with the log.
                if applies {
                    if let Some(open) = &mut self.open {
                        let wrote = vec![Wrote::Module(found.definition().id.to_owned())];
                        open.history.wrote(None, wrote, std::time::Instant::now());
                        open.unsaved = true;
                    }
                    self.step(StepSource::You, None);
                }
                (effect.values, effect.render, effect.save) = (true, true, true);
            }
            self.pickers.tools.insert(found.definition().id, tools);
            // The picker it replaces off first, its own commands answered; then a drag of
            // its own, complete as it begins, answered once its first reading is applied or
            // dropped.
            if self.pickers.active.is_some() {
                self.picker_off(events);
            }
            self.pickers.drag += 1;
            self.pickers.released = self.pickers.drag;
            let area = match declared.kind {
                PickerKind::Point => false,
                PickerKind::Area => true,
                PickerKind::PointArea => area,
            };
            module = Some((
                area,
                OnModule {
                    module: found.definition().id,
                    picker: declared,
                    channel,
                    measured: None,
                    // Its first reading is applied, darktable's forced `changed`.
                    due: Some(Due {
                        command: self.applied,
                        drag: self.pickers.drag,
                        keys,
                    }),
                },
            ));
        }
        if self.pickers.active.is_some() {
            self.picker_off(events);
        }
        let (area, module) = match module {
            Some((area, on)) => (area, Some(on)),
            None => (area, None),
        };
        let geometries = self.geometries()?;
        let places = self.pickers.places.entry(picker).or_default();
        places.of_kind(area, &geometries);
        self.pickers.active = Some(Active {
            picker,
            area,
            module,
        });
        Ok(effect)
    }

    /// [`PickerCommand::Tools`]: the settings kept, and the module's latest reading measured
    /// applied again where the picker that read it reads one of them, as one completed
    /// change, as darktable's settings callbacks apply `picked_color` again
    /// (`exposure.c:1171-1175`). A pick refused turns the module's picker off where it is
    /// on, as a reading's refused pick does.
    fn picker_tools(
        &mut self,
        id: &'static str,
        tools: Vec<(&'static str, ControlValue)>,
        events: &mut Vec<Event>,
    ) -> Result<PickerEffect, SessionError> {
        let module = edit::module(&self.modules, id)?;
        check_tools(module.definition().pickers, &tools)?;
        let id = module.definition().id;
        let changed: BTreeSet<&str> = tools.iter().map(|(tool, _)| *tool).collect();
        self.pickers.tools.insert(id, tools);
        // A completed change, answered whether it applies a pick or not.
        let unapplied = PickerEffect {
            answer: true,
            ..PickerEffect::default()
        };
        // No reading applies while the History tab's eye hides a module (decision 78).
        if self.hiding() {
            return Ok(unapplied);
        }
        let Some(latest) = self.pickers.latest.get(id).copied() else {
            return Ok(unapplied);
        };
        let editable = self
            .open
            .as_ref()
            .is_some_and(|open| open.view_only.is_none());
        if !self.live() || !editable || !latest.picker.tools.iter().any(|t| changed.contains(t)) {
            return Ok(unapplied);
        }
        let due = Due {
            command: self.applied,
            drag: self.pickers.drag,
            keys: latest.keys,
        };
        let on = |pickers: &Pickers| {
            pickers.active.and_then(|active| {
                let on = active.module?;
                (on.module == id && on.picker.id == latest.picker.id).then_some((active.picker, on))
            })
        };
        // A drag under way is saved before the pick, never with it.
        if self.pickers.gesture.take().is_some() {
            self.save_picks();
        }
        let picked = self
            .pick(id, latest.picker, &latest.reading, due)
            .and_then(|picked| {
                let Some(picked) = picked else {
                    return Ok(None);
                };
                // The client has just sent its tool settings: a pick applied again never
                // hands them back, so measure mode, which writes only them, does nothing
                // here, as darktable's settings callback re-runs only correction
                // (`exposure.c:1170-1175`).
                let mut shown = picked.shown.clone();
                shown.tools.clear();
                Ok(Some((self.write_pick(id, latest.picker, picked)?, shown)))
            });
        let (changed, shown) = match picked {
            Ok(Some(written)) => written,
            Ok(None) => return Ok(unapplied),
            Err(error) => {
                if on(&self.pickers).is_some() {
                    self.picker_off(events);
                }
                return Err(error);
            }
        };
        self.pickers.last.insert(id, latest);
        if let Some((picker, _)) = on(&self.pickers)
            && let Some(part) = &mut self.pickers.module
        {
            part.picked = Some(shown);
            let region = self.pickers.sent.as_ref().map(|sent| sent.region);
            if let Some(region) = region {
                self.send(self.applied, picker, region, events);
            }
        }
        // A completed change, answered whether it wrote or not.
        Ok(PickerEffect {
            moved: false,
            values: true,
            render: changed,
            save: true,
            answer: true,
        })
    }

    /// Turns the active picker off, as [`PickerCommand::Off`] does, as another photo opens
    /// and as the photo closes: a `Picker` with no reading where one was on, and a pick's gesture
    /// under way closed, its edit handed to the writer.
    pub(super) fn picker_off(&mut self, events: &mut Vec<Event>) {
        if self.pickers.active.is_some() {
            let gesture = self.pickers.gesture.is_some();
            let owed = !self.pickers.owed.is_empty();
            self.pickers.clear();
            events.push(Event::Picker {
                through: self.applied,
                reading: None,
            });
            self.answer(gesture, owed);
        }
    }

    /// After a picker turned off or failed: a drag under way is saved, what it wrote one
    /// step of source Auto, and a command still owed an answer with nothing to write gets
    /// the edit reported held.
    fn answer(&mut self, gesture: bool, owed: bool) {
        if gesture {
            self.step(StepSource::Auto, None);
        }
        if gesture && self.needs_save() {
            self.save();
        } else if owed && !self.unsaved() {
            self.save_held();
        }
    }

    /// Turns the active picker off where it is a module's, which writes the edit, as a
    /// photo turns view-only; the Colour picker's stays on.
    pub(super) fn module_picker_off(&mut self, events: &mut Vec<Event>) {
        if self
            .pickers
            .active
            .is_some_and(|active| active.module.is_some())
        {
            self.picker_off(events);
        }
    }

    /// Turns a module's picker off as a command on the edit changes its module, as
    /// darktable's `dt_iop_gui_changed` and `gui_update` do: `module` names the module the
    /// command changed, `None` the whole edit. A picker that is darktable's "keep-active"
    /// stays on.
    pub(super) fn picker_module_changed(&mut self, module: Option<&str>, events: &mut Vec<Event>) {
        let Some(on) = self.pickers.active.and_then(|active| active.module) else {
            return;
        };
        if !on.picker.keep && module.is_none_or(|module| module == on.module) {
            self.picker_off(events);
        }
    }

    /// Measures the frame in `buffer`, of `size` pixels, under the active picker, in a
    /// submission of its own after everything before it: the frame's copy, and the
    /// scopes' count. Its reading arrives through [`Self::picked`]. A reduction that
    /// cannot be made, or that the GPU refuses, turns the pickers off, never the frame.
    pub(super) fn sample(
        &mut self,
        buffer: &wgpu::Buffer,
        size: [u32; 2],
        events: &mut Vec<Event>,
    ) {
        let Some(active) = self.pickers.active else {
            return;
        };
        let geometries = match self.geometries() {
            Ok(geometries) => geometries,
            Err(error) => {
                tracing::debug!(target: "pipe", "the colour picker measures nothing: {error}");
                return;
            }
        };
        let pickers = &mut self.pickers;
        let places = pickers.places.entry(active.picker).or_default();
        let region = places.of_kind(active.area, &geometries).region(&geometries);
        let sampled = Sampled {
            through: self.applied,
            picker: active.picker,
            region,
        };
        let Some(bounds) = picker::pixels(region, size) else {
            // Nothing to measure, and the frame readings on their way are older.
            if let Some(reducer) = &mut pickers.reducer {
                reducer.discard();
            }
            pickers
                .ready
                .retain(|arrived| !matches!(arrived, Arrived::Frame(..)));
            pickers.ready.push(Arrived::Frame(sampled, None));
            return;
        };
        // Frame readings yet to be handed over are of older frames: the frame part never
        // describes one.
        (pickers.ready).retain(|arrived| !matches!(arrived, Arrived::Frame(..)));
        let (measured, error) = self.caught(|engine| -> Result<(), ComputeError> {
            let gpu = &engine.gpu;
            if engine.pickers.reducer.is_none() {
                engine.pickers.reducer = Some(pollster::block_on(Reducer::new(gpu))?);
            }
            if let Some(reducer) = &mut engine.pickers.reducer {
                reducer.measure_buffer(gpu, buffer, size, &[bounds], sampled);
            }
            Ok(())
        });
        if let Some(error) = measured.err().map(|e| e.to_string()).or(error) {
            self.pickers_fail(error, events);
        }
    }

    /// The probe a render records at the module picker's operation for `probing`, with
    /// where its reduction goes once recorded; `None` with nothing to measure.
    pub(super) fn probe<'a>(
        reducer: Option<&'a mut Reducer<Probed>>,
        gpu: &'a crate::gpu::GpuContext,
        probing: Option<&'a Probing>,
        recorded: &'a mut Option<Result<Recorded, ComputeError>>,
    ) -> Option<Probe<'a>> {
        let (reducer, probing) = (reducer?, probing?);
        let targets = probing.targets.as_ref()?;
        Some(Probe {
            op: probing.op,
            record: Box::new(move |encoder, input, output| {
                let groups = probing.groups(targets, [input, output]);
                *recorded = Some(reducer.record(gpu, encoder, &groups));
            }),
        })
    }

    /// After a render that may have measured the module picker's operation, `recorded`
    /// what it recorded there: the reading on its way, or, where the region misses the
    /// operation's image, one that needs no GPU. A reduction that could not be recorded
    /// turns the pickers off.
    pub(super) fn probed(
        &mut self,
        probing: Option<Probing>,
        recorded: Option<Result<Recorded, ComputeError>>,
        events: &mut Vec<Event>,
    ) {
        let Some(probing) = probing else {
            return;
        };
        if probing.targets.is_none() {
            self.pickers.measured(probing.key);
            let arrived = if probing.unavailable {
                Arrived::Unavailable(probing.tag)
            } else {
                Arrived::Module(probing.tag, None)
            };
            self.pickers.ready.push(arrived);
            return;
        }
        if let Some(error) = self.pickers.submitted(probing, recorded) {
            self.pickers_fail(error, events);
        }
    }

    /// The module picker's reading at its operation where no render measured it: a
    /// sample-only render, from the newest cache point at or before the operation, as its
    /// region moved or it turned on with no edit, its operation then the edited instance.
    /// Where it reads what it read last, a reading due is not, and a release completes
    /// the gesture with the values already written.
    pub(super) fn probe_alone(&mut self, events: &mut Vec<Event>) {
        // Nothing measured while the eye hides a module: its reading would be the view's.
        if !self.pickers.probing() || self.hiding() {
            return;
        }
        let through = self.applied;
        let Some(open) = &mut self.open else {
            return;
        };
        let Some(pipeline) = &open.pipeline else {
            // Nothing rendered yet: the first frame measures it.
            return;
        };
        let bound = edit::instances(&open.shown(), self.operations, &self.modules, &open.image);
        let probing = probing(
            &mut self.pickers,
            pipeline.plan(),
            &bound.instances,
            &open.image,
            through,
        );
        let Some(probing) = probing else {
            // Nothing new to read: no reading is due, and a drag released is complete.
            if let Some(on) = (self.pickers.active.as_mut()).and_then(|a| a.module.as_mut()) {
                on.due = None;
            }
            self.settle();
            return;
        };
        let Some(targets) = &probing.targets else {
            self.probed(Some(probing), None, events);
            return;
        };
        let output = targets.len() > 1;
        let path = open.sidecar.with_extension("");
        (self.rendered_thumbnails).before_sample(
            &path,
            &self.jobs.control,
            pipeline,
            (probing.op, output),
        );
        let mut recorded = None;
        let (sampled, error) = self.caught(|engine| {
            let Some(pipeline) = engine.open.as_mut().and_then(|open| open.pipeline.as_mut())
            else {
                return false;
            };
            let (gpu, reducer) = (&engine.gpu, engine.pickers.probe.as_mut());
            let Some(reducer) = reducer else {
                return false;
            };
            pipeline.sample(gpu, probing.op, output, |encoder, input, written| {
                let buffers = [input, written.unwrap_or(input)];
                let groups = probing.groups(targets, buffers);
                recorded = Some(reducer.record(gpu, encoder, &groups));
            })
        });
        if let Some(error) = error {
            // A sample-only render the GPU refused: the pickers off, the next frame
            // rendered from scratch.
            self.pickers_fail(error, events);
            if let Some(open) = &mut self.open {
                open.pipeline = None;
            }
            return;
        }
        if sampled {
            self.probed(Some(probing), recorded, events);
        }
    }

    /// The active picker's readings that have arrived, never waiting, handed over in the
    /// order of the commands they measured, one waiting for any measured before it still
    /// on its way. Each is sent where it changes the region, the module's part or the
    /// frame's part the client was sent last; a module's reading due is applied through
    /// its pick after it is sent, its change's values and frame after it, and a drag
    /// released is saved once no reading of it is still to come.
    pub(super) fn picked(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut failed = None;
        if self.pickers.probe.as_ref().is_some_and(Reducer::pending) {
            let mut reduced = Vec::new();
            let (polled, error) = self.caught(|engine| match &mut engine.pickers.probe {
                Some(probe) => probe.poll(&engine.gpu, &mut reduced),
                None => Ok(()),
            });
            failed = polled.err().map(|e| e.to_string()).or(error);
            self.pickers.ready.extend(
                (reduced.into_iter())
                    .map(|reduced| Arrived::Module(reduced.tag, Some(reduced.stats))),
            );
        }
        if failed.is_none() && self.pickers.reducer.as_ref().is_some_and(Reducer::pending) {
            let mut reduced = Vec::new();
            let (polled, error) = self.caught(|engine| match &mut engine.pickers.reducer {
                Some(reducer) => reducer.poll(&engine.gpu, &mut reduced),
                None => Ok(()),
            });
            failed = polled.err().map(|e| e.to_string()).or(error);
            self.pickers.ready.extend(
                reduced
                    .into_iter()
                    .map(|reduced| Arrived::Frame(reduced.tag, reduced.stats.first().copied())),
            );
        }
        // Stable, by `through`.
        self.pickers.ready.sort_by_key(Arrived::through);
        // Those a pick's render adds as these are handed over wait for the next call.
        for _ in 0..self.pickers.ready.len() {
            let Some(first) = self.pickers.ready.first() else {
                break;
            };
            if (self.pickers.horizon()).is_some_and(|horizon| horizon < first.through()) {
                break;
            }
            let arrival = self.pickers.ready.remove(0);
            match arrival {
                Arrived::Frame(sampled, stats) => {
                    if self.on(sampled.picker) {
                        self.pickers.frame = stats.as_ref().map(FrameReading::of);
                        self.send(sampled.through, sampled.picker, sampled.region, &mut events);
                    }
                }
                Arrived::Module(probed, stats) => {
                    if self.on(probed.picker) {
                        self.arrived(probed, stats, &mut events);
                    }
                }
                // No reading at the operation, and none applied: the module part goes.
                Arrived::Unavailable(probed) => {
                    if self.on(probed.picker) {
                        self.pickers.module = None;
                        self.send(probed.through, probed.picker, probed.region, &mut events);
                    }
                }
            }
            // A reading of a drag released, applied or not, may be its last.
            self.settle();
        }
        if let Some(error) = failed {
            self.pickers_fail(error, &mut events);
        }
        events
    }

    /// Hands the writer the picks of a drag complete: released, with no reading of it
    /// still to come; and where it has nothing to write, answers a command owed one with
    /// the edit reported held, so a `Saved` through it always comes. Held, an edit whose
    /// last write failed is written again; one with a change of its own still unsaved, as
    /// a slider's gesture under way, waits for that change's save, which answers it.
    fn settle(&mut self) {
        let settled = self.pickers.settled();
        // What the drag's picks changed, one step of source Auto, as the log the writer is
        // handed holds only steps.
        if settled {
            self.step(StepSource::Auto, None);
        }
        if settled && self.needs_save() {
            self.save();
        } else if self.pickers.answerable() && !self.unsaved() {
            self.save_held();
        }
    }

    /// Saves a drag's picks: what they changed one step of source Auto (`docs/history.md`,
    /// What makes a step), as the log the writer is handed holds only steps, then the log
    /// written where it changed. Merging a drag's steps by its picker comes with the
    /// pickers' own steps.
    fn save_picks(&mut self) {
        self.step(StepSource::Auto, None);
        if self.needs_save() {
            self.save();
        }
    }

    /// A module picker's reading at its operation, `stats` its input's and output's:
    /// sent, then, where due, applied through the module's pick.
    fn arrived(&mut self, probed: Probed, stats: Option<Vec<Stats>>, events: &mut Vec<Event>) {
        let Some(on) = self.pickers.active.and_then(|active| active.module) else {
            return;
        };
        // Applied once no module is hidden: until then its application waits, and the
        // next render once all are shown measures again (decision 78).
        if self.hiding() {
            self.pickers.wait(probed.due);
            return;
        }
        let reading = stats.as_ref().and_then(|stats| {
            Some(Reading {
                input: *stats.first()?,
                output: stats.get(1).copied(),
            })
        });
        let previous = self
            .pickers
            .module
            .as_ref()
            .and_then(|part| part.picked.clone());
        // darktable's `picked_color`, which every reading overwrites.
        if let Some(reading) = reading {
            let keys = (probed.due.map(|due| due.keys))
                .or_else(|| self.pickers.latest.get(on.module).map(|latest| latest.keys))
                .unwrap_or_default();
            let latest = Last {
                picker: on.picker,
                reading,
                keys,
            };
            self.pickers.latest.insert(on.module, latest);
        }
        let mut picked = None;
        if let Some(due) = probed.due {
            let made = match &reading {
                Some(reading) => self.pick(on.module, on.picker, reading, due),
                None => Ok(None),
            };
            match made {
                Ok(Some(made)) => picked = Some((made, due)),
                // Nothing to apply, the reading missing or the pick dropped: a drag
                // released is complete with the values its earlier picks wrote, as
                // `picked` settles it.
                Ok(None) => {}
                Err(error) => {
                    self.picker_refused(due.command, error, events);
                    return;
                }
            }
        }
        self.pickers.module = Some(ModuleReading {
            input: reading.map(|reading| reading.input),
            output: reading.and_then(|reading| reading.output),
            picked: match &picked {
                Some((made, _)) => Some(made.shown.clone()),
                None => previous,
            },
        });
        self.send(probed.through, probed.picker, probed.region, events);
        let (Some((made, due)), Some(reading)) = (picked, reading) else {
            return;
        };
        let applied = self.guarded(|engine| {
            let mut events = Vec::new();
            if let Err(error) = engine.apply_pick(on, made, due, reading, &mut events) {
                engine.picker_refused(due.command, error, &mut events);
            }
            events
        });
        events.extend(applied);
    }

    /// The module's pick of `reading` by its picker `picker`, for the reading `due` asked
    /// for: darktable's `color_picker_apply`, which the module's last reading becomes. A
    /// pick that panicked is the module's failure; a number it gives that is not finite is
    /// left out, the rest of the pick and what it shows kept.
    fn pick(
        &mut self,
        id: &'static str,
        picker: &'static Picker,
        reading: &Reading,
        due: Due,
    ) -> Result<Option<Picked>, SessionError> {
        let module = edit::module(&self.modules, id)?;
        let open = self.open.as_ref().ok_or(SessionError::NotLive)?;
        let values = open.edit.every(module, &open.image);
        let tools = self.pickers.tools.get(id).cloned().unwrap_or_default();
        let channel = (self.pickers.active)
            .and_then(|active| active.module)
            .filter(|on| on.module == id)
            .and_then(|on| on.channel);
        let last = self.pickers.last.get(id).map(|last| last.reading);
        let picked = catch_unwind(AssertUnwindSafe(|| {
            module.pick(
                picker.id,
                reading,
                &values,
                &open.image,
                &tools,
                channel,
                last.as_ref(),
                due.keys,
            )
        }));
        let picked = picked.map_err(|panic| {
            SessionError::Edit(EditError::PickFailed {
                module: id,
                picker: picker.id,
                message: panic_message(panic.as_ref()),
            })
        })?;
        // darktable's rule may leave no value, as `log2` of nothing: that write is dropped,
        // and what the pick shows kept, as darktable shows its patch and L before it sets
        // anything (`exposure.c:885-897`).
        let finite = |value: &ControlValue| match value {
            ControlValue::Number(number) => number.is_finite(),
            _ => true,
        };
        Ok(picked.map(|mut picked| {
            let mut writes = Controls::new();
            for (control, value) in picked.writes.iter().filter(|(_, v)| finite(v)) {
                writes.set(control, value.clone());
            }
            picked.writes = writes;
            picked.shown.tools.retain(|(_, value)| finite(value));
            picked
        }))
    }

    /// Writes what the module's picker picked as one change, and says whether the edit
    /// changed: a change on the edit, so a slider's gesture under way ends, a step of its
    /// own source first, never the pick's.
    fn write_pick(
        &mut self,
        id: &'static str,
        picker: &'static Picker,
        picked: Picked,
    ) -> Result<bool, SessionError> {
        self.end_gesture();
        let changed = self.change(|edit, modules, _, image| {
            edit.pick(edit::module(modules, id)?, picker, &picked.writes, image)
        })?;
        if changed && let Some(open) = &mut self.open {
            open.gesture = None;
        }
        Ok(changed)
    }

    /// Applies a reading's pick, `made` of `reading` for the reading `due` asked for, as
    /// one change: one of its drag's, saved once the drag is released and no reading of
    /// it is still to come, never with the next drag's; then the values, and the frame.
    /// Written, `reading` is the module's last applied, as RGB levels' `last_picked_color`.
    fn apply_pick(
        &mut self,
        on: OnModule,
        made: Picked,
        due: Due,
        reading: Reading,
        events: &mut Vec<Event>,
    ) -> Result<(), SessionError> {
        let started = std::time::Instant::now();
        // An earlier drag's picks are saved before a later one's, never merged with them.
        if self.pickers.gesture.is_some_and(|drag| drag < due.drag) {
            self.pickers.gesture = None;
            self.save_picks();
        }
        let changed = self.write_pick(on.module, on.picker, made)?;
        let last = Last {
            picker: on.picker,
            reading,
            keys: due.keys,
        };
        self.pickers.last.insert(on.module, last);
        if changed {
            self.pickers.gesture = Some(due.drag);
        }
        self.settle();
        if let Some(open) = &mut self.open {
            events.push(Event::Values(open.values(
                self.operations,
                &self.modules,
                self.applied,
            )));
        }
        // The step the drag's picks made, as it settled, after the values, as a batch sends
        // it.
        events.extend(self.history_event());
        if changed {
            match self.render(started, events) {
                Ok(Some(frame)) => events.push(Event::Frame(frame)),
                Ok(None) => {}
                Err(error) => events.push(Event::Failed(error)),
            }
            self.probe_alone(events);
        }
        Ok(())
    }

    /// A pick refused, writing nothing: the picker off, reported as refusing the command
    /// that asked for the reading.
    fn picker_refused(&mut self, command: u64, error: SessionError, events: &mut Vec<Event>) {
        tracing::error!(target: "pipe", "a pick refused: {error}");
        events.push(Event::Failed(SessionError::Refused {
            command,
            error: Box::new(error),
        }));
        self.picker_off(events);
    }

    /// Sends the active picker's reading, its parts the latest, where it changes what the
    /// client was sent last.
    fn send(&mut self, through: u64, picker: PickerRef, region: Region, events: &mut Vec<Event>) {
        let pickers = &mut self.pickers;
        let reading = PickerReading {
            through,
            picker,
            region,
            module: pickers.module.clone(),
            frame: pickers.frame,
        };
        let changed = pickers.sent.as_ref().is_none_or(|sent| {
            (sent.picker, sent.region, &sent.module, sent.frame)
                != (
                    reading.picker,
                    reading.region,
                    &reading.module,
                    reading.frame,
                )
        });
        if changed {
            pickers.sent = Some(reading.clone());
            events.push(Event::Picker {
                through: reading.through,
                reading: Some(reading),
            });
        }
    }

    /// Whether `picker` is the one on.
    fn on(&self, picker: PickerRef) -> bool {
        self.pickers
            .active
            .is_some_and(|active| active.picker == picker)
    }

    /// Turns the pickers off for the rest of the session, reporting why once, and
    /// a `Picker` with no reading where one was on: the frames go on without them.
    pub(super) fn pickers_fail(&mut self, error: String, events: &mut Vec<Event>) {
        tracing::error!(target: "pipe", "pickers off: {error}");
        let on = self.pickers.active.is_some();
        let gesture = self.pickers.gesture.is_some();
        let owed = !self.pickers.owed.is_empty();
        self.pickers.clear();
        self.pickers.reducer = None;
        self.pickers.probe = None;
        self.pickers.failed = Some(error.clone());
        events.push(Event::Failed(SessionError::Pickers(error)));
        if on {
            events.push(Event::Picker {
                through: self.applied,
                reading: None,
            });
        }
        self.answer(gesture, owed);
    }

    /// Whether the photo is live (decision 77): open, its raw uploaded, no open under way.
    fn live(&self) -> bool {
        self.opening.is_none() && self.open.as_ref().is_some_and(|open| open.input.is_some())
    }

    /// The maps between the open photo's coordinates and its original image's, as its
    /// edit gives them in the live frame, the modules the eye hides as if reset.
    fn geometries(&self) -> Result<Geometries, SessionError> {
        let open = self.open.as_ref().ok_or(SessionError::NotLive)?;
        let bound = edit::instances(&open.shown(), self.operations, &self.modules, &open.image);
        Ok(Geometries::of(&bound.instances, &open.image)?)
    }
}

/// What a reading at the operation waiting for a staging buffer passes on to the newer one
/// replacing it: its due application, with its drag, where the newer has none. Where both
/// have one, the newer's applies; one of an earlier drag leaves that drag with no reading
/// still to come, so it is saved before the newer drag's applies.
fn keep_due(old: &Probed, new: &mut Probed) {
    if new.due.is_none() {
        new.due = old.due;
    }
}

/// Refuses tool settings that none of `pickers` reads.
fn check_tools(
    pickers: &[Picker],
    tools: &[(&'static str, ControlValue)],
) -> Result<(), SessionError> {
    let read: BTreeSet<&str> = pickers
        .iter()
        .flat_map(|picker| picker.tools)
        .copied()
        .collect();
    match tools.iter().find(|(tool, _)| !read.contains(tool)) {
        Some(&(tool, _)) => Err(SessionError::UnknownTool(tool)),
        None => Ok(()),
    }
}
