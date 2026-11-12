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

//! The staging pair the scopes and the colour pickers read their results back through
//! (decisions 19 and 79; `docs/engine.md`, Scopes; `docs/pickers.md`, When it runs),
//! without the engine thread ever waiting for the GPU.
//!
//! A [`Readback`] takes a submission whose work leaves a result in a buffer of its
//! owner's, copies that result into whichever of its two staging buffers is free, maps it
//! and returns. A mapping that has completed is picked up by [`Readback::poll`], which
//! never waits. A staging buffer stays busy from its copy until its result has been read
//! and it is unmapped; taking it back sooner would abort its mapping and drop that result,
//! and during a drag, when each frame comes before the last one's mapping completes,
//! could drop every result. So two take turns, and while both are busy the result stays
//! in its owner's buffer, each submission's replacing the last's, and the latest is
//! copied as soon as one frees: the last result always arrives.

use std::sync::mpsc;

use crate::compute::ComputeError;
use crate::gpu::GpuContext;

/// A result submitted, with what its owner keeps of it.
struct Sent<T> {
    measured: T,
    /// The staging bytes its copy fills.
    bytes: u64,
    /// The submissions so far when it was submitted, which results arrive in.
    sequence: u64,
    /// The readback's generation when it was submitted: one [`Readback::discard`] has
    /// passed is dropped when it arrives.
    generation: u64,
}

enum Staging<T> {
    Free,
    Mapping {
        sent: Sent<T>,
        done: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
        /// The mapping's result, once its callback has been seen.
        result: Option<Result<(), wgpu::BufferAsyncError>>,
    },
}

impl<T> Staging<T> {
    /// Whether the mapping's callback has fired, looked at without waiting.
    fn arrived(&mut self) -> bool {
        let Staging::Mapping { done, result, .. } = self else {
            return false;
        };
        if result.is_none() {
            *result = match done.try_recv() {
                Ok(mapped) => Some(mapped),
                Err(mpsc::TryRecvError::Empty) => None,
                // wgpu calls every callback, so this should not happen: count the mapping
                // failed.
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(wgpu::BufferAsyncError)),
            };
        }
        result.is_some()
    }
}

/// Two staging buffers taking turns, each result read back without waiting; `T` is what
/// the owner keeps of a result until it arrives.
pub(crate) struct Readback<T> {
    staging: [(wgpu::Buffer, Staging<T>); 2],
    /// The latest result submitted while both staging buffers were mapping, still in its
    /// owner's buffer.
    waiting: Option<Sent<T>>,
    /// The submissions so far.
    sequence: u64,
    generation: u64,
}

impl<T> Readback<T> {
    /// Two staging buffers of `size` bytes, named after `label`.
    pub(crate) fn new(gpu: &GpuContext, label: &str, size: u64) -> Self {
        let staging = |index: usize| {
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{label} staging {index}")),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            (buffer, Staging::Free)
        };
        Self {
            staging: [staging(0), staging(1)],
            waiting: None,
            sequence: 0,
            generation: 0,
        }
    }

    /// Submits `encoder`, whose work leaves a result in its owner's buffers, with `copy`
    /// recording the copy of its first `bytes` into a free staging buffer, which is then mapped; while both
    /// are busy, submits `encoder` alone and keeps `measured` waiting for one, in place of
    /// any result waiting before it. Returns without waiting.
    pub(crate) fn submit(
        &mut self,
        gpu: &GpuContext,
        mut encoder: wgpu::CommandEncoder,
        measured: T,
        bytes: u64,
        copy: impl Fn(&mut wgpu::CommandEncoder, &wgpu::Buffer, u64),
    ) {
        self.sequence += 1;
        let sent = Sent {
            measured,
            bytes,
            sequence: self.sequence,
            generation: self.generation,
        };
        match self.free() {
            Some(slot) => {
                copy(&mut encoder, &self.staging[slot].0, bytes);
                gpu.submit([encoder.finish()]);
                self.map(slot, sent);
            }
            None => {
                gpu.submit([encoder.finish()]);
                self.waiting = Some(sent);
            }
        }
    }

    /// Polls the device without waiting and hands each result whose mapping has
    /// completed to `read`, with its bytes, in the order they were submitted; then, if a
    /// staging buffer has freed and a result is waiting for one, submits its `copy` and
    /// maps it. A mapping that failed is reported after the others are handed over: its
    /// result is lost and its buffer free again.
    pub(crate) fn poll(
        &mut self,
        gpu: &GpuContext,
        copy: impl Fn(&mut wgpu::CommandEncoder, &wgpu::Buffer, u64),
        mut read: impl FnMut(T, &[u8]),
    ) -> Result<(), ComputeError> {
        if !self.pending() {
            return Ok(());
        }
        gpu.device.poll(wgpu::PollType::Poll)?;
        let mut arrived: Vec<usize> = (0..self.staging.len())
            .filter(|&slot| self.staging[slot].1.arrived())
            .collect();
        arrived.sort_by_key(|&slot| match &self.staging[slot].1 {
            Staging::Mapping { sent, .. } => sent.sequence,
            Staging::Free => 0,
        });
        let mut failed = None;
        for slot in arrived {
            let (buffer, staging) = &mut self.staging[slot];
            let Staging::Mapping {
                sent,
                result: Some(result),
                ..
            } = std::mem::replace(staging, Staging::Free)
            else {
                unreachable!("a mapping that arrived has its result");
            };
            if let Err(error) = result {
                failed = Some(error);
                continue;
            }
            match buffer.get_mapped_range(..sent.bytes) {
                Ok(view) => {
                    if sent.generation == self.generation {
                        read(sent.measured, &view);
                    }
                }
                // Mapped, as its callback said, so this should not happen.
                Err(_) => failed = Some(wgpu::BufferAsyncError),
            }
            buffer.unmap();
        }
        if self.waiting.is_some()
            && let Some(slot) = self.free()
            && let Some(sent) = self.waiting.take()
        {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            copy(&mut encoder, &self.staging[slot].0, sent.bytes);
            gpu.submit([encoder.finish()]);
            self.map(slot, sent);
        }
        match failed {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }

    /// Whether a result is still on its way: a staging buffer mapping, or a result
    /// waiting for one.
    pub(crate) fn pending(&self) -> bool {
        self.waiting.is_some()
            || self
                .staging
                .iter()
                .any(|(_, staging)| matches!(staging, Staging::Mapping { .. }))
    }

    /// Drops every result on its way, as when the photo it measures closes: those
    /// mapping still arrive, and are dropped then.
    pub(crate) fn discard(&mut self) {
        self.generation += 1;
        self.waiting = None;
    }

    /// Whether a result `which` picks is yet to arrive: waiting for a staging buffer, or
    /// its mapping's callback not fired yet. Whatever submitted it and returned without
    /// waiting for its mapping leaves it so.
    #[cfg(test)]
    pub(crate) fn awaits(&mut self, which: impl Fn(&T) -> bool) -> bool {
        self.waiting
            .as_ref()
            .is_some_and(|sent| which(&sent.measured))
            || self.staging.iter_mut().any(|(_, staging)| {
                let picked =
                    matches!(staging, Staging::Mapping { sent, .. } if which(&sent.measured));
                picked && !staging.arrived()
            })
    }

    /// A staging buffer that is neither mapping nor mapped.
    fn free(&self) -> Option<usize> {
        self.staging
            .iter()
            .position(|(_, staging)| matches!(staging, Staging::Free))
    }

    /// Maps staging buffer `slot`, which a submitted copy fills with `sent`'s result.
    fn map(&mut self, slot: usize, sent: Sent<T>) {
        let (sender, done) = mpsc::channel();
        let (buffer, staging) = &mut self.staging[slot];
        buffer.map_async(wgpu::MapMode::Read, ..sent.bytes, move |result| {
            // The receiver lives in the staging state until the callback has been seen.
            let _ = sender.send(result);
        });
        *staging = Staging::Mapping {
            sent,
            done,
            result: None,
        };
    }
}
