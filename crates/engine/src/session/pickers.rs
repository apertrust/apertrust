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
//! view state, never in the edit. One active picker, the Colour picker module's for now,
//! and each picker's last point and area, kept in the original image's coordinates for the
//! session's life ([`crate::picker::Place`]).
//!
//! The active picker is measured on the frame, in a submission of its own after each
//! frame's copy and the scopes' count, and on the frame shown when its region changes
//! with no render. Its reading comes back without the thread waiting, as the scopes'
//! does, and goes out as [`Event::Picker`] when it changes what a client shows: after the
//! frame it measured. A failure turns the pickers off for the session, never the frame.

use std::collections::BTreeMap;

use crate::compute::ComputeError;
use crate::edit;
use crate::picker::{
    self, DEFAULT_AREA, DEFAULT_POINT, FrameReading, Geometries, PickerCommand, PickerReading,
    PickerRef, Place, Reducer, Region,
};

use super::{Engine, Event, SessionError};

/// The pickers' state, which the session keeps for its life.
#[derive(Default)]
pub(super) struct Pickers {
    /// The picker on, if any.
    active: Option<Active>,
    /// Each picker's last point and area this session, kept as it turns off for its next
    /// `On`.
    places: BTreeMap<PickerRef, Places>,
    /// The reduction and its readback, made with the first measurement.
    pub(super) reducer: Option<Reducer<Sampled>>,
    /// Why the pickers failed, which turns them off for the session.
    failed: Option<String>,
    /// A reading that needed no GPU, of a region that misses the frame, sent with the
    /// readings picked up next.
    ready: Option<PickerReading>,
    /// The reading last sent while the active picker is on.
    sent: Option<PickerReading>,
}

/// The picker on, and whether it measures its area rather than its point.
#[derive(Debug, Clone, Copy)]
struct Active {
    picker: PickerRef,
    area: bool,
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

impl Pickers {
    /// The active picker off, with what is on its way of it: nothing sent.
    pub(super) fn clear(&mut self) {
        self.active = None;
        self.sent = None;
        self.ready = None;
        if let Some(reducer) = &mut self.reducer {
            reducer.discard();
        }
    }

    /// Whether a picker is on.
    pub(super) fn on(&self) -> bool {
        self.active.is_some()
    }

    /// Whether a reading is on its way, so the thread looks at the GPU between commands.
    pub(super) fn pending(&self) -> bool {
        self.ready.is_some() || self.reducer.as_ref().is_some_and(Reducer::pending)
    }

    /// Drops the readings on their way, of a frame a GPU error dropped.
    pub(super) fn discard(&mut self) {
        self.ready = None;
        if let Some(reducer) = &mut self.reducer {
            reducer.discard();
        }
    }
}

impl Engine {
    /// [`super::Command::Picker`]; whether the frame shown is to be measured again, the
    /// active picker's region having changed. `Picker(None)` goes into `events` as a
    /// picker turns off.
    pub(super) fn picker(
        &mut self,
        command: PickerCommand,
        events: &mut Vec<Event>,
    ) -> Result<bool, SessionError> {
        let mut active = match command {
            PickerCommand::Off => {
                self.picker_off(events);
                return Ok(false);
            }
            PickerCommand::On { picker, area, .. } => {
                if !self.live() {
                    return Err(SessionError::NotLive);
                }
                if let Some(failed) = &self.pickers.failed {
                    return Err(SessionError::Pickers(failed.clone()));
                }
                // No module declares a picker yet (`docs/pickers.md`, The plan).
                if picker != PickerRef::ColourPicker {
                    return Err(SessionError::UnknownPicker);
                }
                if self
                    .pickers
                    .active
                    .is_some_and(|active| active.picker != picker)
                {
                    self.picker_off(events);
                }
                Active { picker, area }
            }
            _ => self.pickers.active.ok_or(SessionError::NoPicker)?,
        };
        let geometries = self.geometries()?;
        let places = self.pickers.places.entry(active.picker).or_default();
        match command {
            PickerCommand::On { .. } | PickerCommand::Off => {}
            PickerCommand::Point { at, .. } => {
                places.point = Some(Place::of(Region::Point(at), &geometries));
                active.area = false;
            }
            PickerCommand::Area { from, to, .. } => {
                let [from, to] = [from, to].map(picker::clamped);
                let area = [
                    from[0].min(to[0]),
                    from[1].min(to[1]),
                    from[0].max(to[0]),
                    from[1].max(to[1]),
                ];
                places.area = Some(Place::of(Region::Area(area), &geometries));
                active.area = true;
            }
            PickerCommand::Switch => active.area = !active.area,
            PickerCommand::Reset if active.area => {
                places.area = Some(Place::of(DEFAULT_AREA, &geometries));
            }
            PickerCommand::Reset => places.point = Some(Place::of(DEFAULT_POINT, &geometries)),
        }
        places.of_kind(active.area, &geometries);
        self.pickers.active = Some(active);
        Ok(true)
    }

    /// Turns the active picker off, as [`PickerCommand::Off`] does, as another photo opens
    /// and as the photo closes: `Picker(None)` where one was on.
    pub(super) fn picker_off(&mut self, events: &mut Vec<Event>) {
        if self.pickers.active.is_some() {
            self.pickers.clear();
            events.push(Event::Picker(None));
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
            // Nothing to measure, and what is on its way is older.
            pickers.discard();
            pickers.ready = Some(reading(sampled, None));
            return;
        };
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

    /// The active picker's readings that have arrived, never waiting, each sent where it
    /// changes the region or the frame's reading the client was sent last.
    pub(super) fn picked(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut readings: Vec<PickerReading> = self.pickers.ready.take().into_iter().collect();
        let mut failed = None;
        if self.pickers.reducer.as_ref().is_some_and(Reducer::pending) {
            let mut reduced = Vec::new();
            let (polled, error) = self.caught(|engine| match &mut engine.pickers.reducer {
                Some(reducer) => reducer.poll(&engine.gpu, &mut reduced),
                None => Ok(()),
            });
            failed = polled.err().map(|e| e.to_string()).or(error);
            readings.extend(
                reduced
                    .into_iter()
                    .map(|reduced| reading(reduced.tag, reduced.stats.first())),
            );
        }
        let pickers = &mut self.pickers;
        for reading in readings {
            let on = pickers
                .active
                .is_some_and(|active| active.picker == reading.picker);
            let changed = pickers.sent.as_ref().is_none_or(|sent| {
                (sent.picker, sent.region, sent.frame)
                    != (reading.picker, reading.region, reading.frame)
            });
            if on && changed {
                pickers.sent = Some(reading.clone());
                events.push(Event::Picker(Some(reading)));
            }
        }
        if let Some(error) = failed {
            self.pickers_fail(error, &mut events);
        }
        events
    }

    /// Turns the pickers off for the rest of the session, reporting why once, and
    /// `Picker(None)` where one was on: the frames go on without them.
    fn pickers_fail(&mut self, error: String, events: &mut Vec<Event>) {
        tracing::error!(target: "pipe", "pickers off: {error}");
        let on = self.pickers.active.is_some();
        self.pickers.clear();
        self.pickers.reducer = None;
        self.pickers.failed = Some(error.clone());
        events.push(Event::Failed(SessionError::Pickers(error)));
        if on {
            events.push(Event::Picker(None));
        }
    }

    /// Whether the photo is live (decision 77): open, its raw uploaded, no open under way.
    fn live(&self) -> bool {
        self.opening.is_none() && self.open.as_ref().is_some_and(|open| open.input.is_some())
    }

    /// The maps between the open photo's coordinates and its original image's, as its
    /// edit gives them.
    fn geometries(&self) -> Result<Geometries, SessionError> {
        let open = self.open.as_ref().ok_or(SessionError::NotLive)?;
        let bound = edit::instances(&open.edit, self.operations, &self.modules, &open.image);
        Ok(Geometries::of(&bound.instances, &open.image)?)
    }
}

/// The reading of `sampled`, given the frame's statistics under it, if measured.
fn reading(sampled: Sampled, stats: Option<&picker::Stats>) -> PickerReading {
    PickerReading {
        through: sampled.through,
        picker: sampled.picker,
        region: sampled.region,
        frame: stats.map(FrameReading::of),
    }
}
