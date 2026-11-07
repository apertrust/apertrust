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

//! The GPU's share of the session's jobs (decision 57; `docs/engine.md`, Jobs, The
//! engine's part). A job runs on one of the control's workers ([`apertrust_base::jobs`]),
//! and never submits: its body hands its GPU work to the engine thread as a [`GpuTask`]
//! through [`Tasks`], and waits for its result through [`Context::wait`], which returns at
//! cancel. The engine thread steps the tasks that wait, one step a turn after the live
//! frame (decision 37), chosen by darktable's rule over their jobs' queues
//! ([`schedule::pick`]): each step is scheduled as a job darktable adds to that queue, at
//! a FIFO's end and on the stack's top, so the foreground's steps come first and a
//! background one's after its head has lost five turns, as darktable's jobs do. Only one
//! step's work is on the GPU at a time, so a change to the live photo waits for one step
//! at most. A cancelled job's task is dropped before its next step.

use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use apertrust_base::jobs::Control;
use apertrust_base::jobs::schedule::{self, Added, Identity, Queues, Scheduled};
pub use apertrust_base::jobs::{
    Argument, Context, Interrupted, Job, JobEvent, JobId, Message, Outcome, Queue, State,
};
use thiserror::Error;

use crate::edit::panic_message;
use crate::gpu::GpuContext;
use crate::session::Inbox;

/// A job's GPU work, which its body hands to the engine thread ([`Tasks::hand`]): a plan
/// and its state, stepped by the engine thread one turn at a time, between the live
/// photo's frames. A step makes one submission at most, through [`GpuContext::submit`],
/// or looks at a readback it mapped, never waiting for the GPU; the engine waits for a
/// step's work to finish, polling, before it steps any task again.
pub trait GpuTask: Send + 'static {
    /// What the task hands back to its body once done.
    type Output: Send + 'static;

    /// One step: [`Step::Again`] while there is more, [`Step::Done`] with the result.
    /// A failure, in plain words, ends the task and goes to its body as
    /// [`TaskError::Failed`], as does a panic or an error the GPU reports for the step.
    fn step(&mut self, gpu: &GpuContext) -> Result<Step<Self::Output>, String>;
}

/// What a [`GpuTask`]'s step did.
#[derive(Debug, Clone, PartialEq)]
pub enum Step<T> {
    /// It stepped and has more: it waits for another turn.
    Again,
    /// It is done: this goes to its body.
    Done(T),
}

/// Why a [`GpuTask`] gave its body no result.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TaskError {
    /// The job was cancelled: its body stops, and the engine drops the task before its
    /// next step.
    #[error("cancelled")]
    Cancelled,
    /// The session ended, or dropped the task, before it was done.
    #[error("the engine stopped")]
    Stopped,
    /// A step failed, panicked or caused a GPU error, as this says.
    #[error("{0}")]
    Failed(String),
}

/// A job's way to the engine thread, given to its body as the session adds it: what it
/// hands goes to the engine thread with the job's queue.
#[derive(Clone)]
pub struct Tasks {
    inbox: Sender<Inbox>,
    queue: Queue,
}

impl Tasks {
    pub(crate) fn new(inbox: Sender<Inbox>, queue: Queue) -> Self {
        Self { inbox, queue }
    }

    /// Hands `task` to the engine thread, which steps it once it may; its result comes
    /// through what this returns, so the body may do its own work meanwhile.
    pub fn hand<T: GpuTask>(&self, context: &Context<'_>, task: T) -> Handed<T::Output> {
        let (answer, answered) = mpsc::channel();
        let waiting = Waiting {
            job: context.job(),
            queue: self.queue,
            task: Box::new(Handing {
                task,
                output: None,
                answer: Some(answer),
            }),
        };
        // An engine thread that has gone drops the task, and its answer with it: the
        // body hears `Stopped`.
        let _ = self.inbox.send(Inbox::Task(waiting));
        Handed { answered }
    }

    /// Hands `task` to the engine thread and waits for its result, returning at cancel.
    pub fn run<T: GpuTask>(&self, context: &Context<'_>, task: T) -> Result<T::Output, TaskError> {
        self.hand(context, task).wait(context)
    }
}

/// A [`GpuTask`] handed to the engine thread, whose result its body waits for.
pub struct Handed<T> {
    answered: Receiver<Result<T, TaskError>>,
}

impl<T> Handed<T> {
    /// Waits for the task's result through [`Context::wait`]: it returns at cancel,
    /// [`TaskError::Cancelled`], and when the engine has dropped the task,
    /// [`TaskError::Stopped`], or `Cancelled` where it dropped it for the job's cancel,
    /// which it may see before the body does.
    pub fn wait(self, context: &Context<'_>) -> Result<T, TaskError> {
        wait(context, &self.answered)?
    }
}

/// Waits through [`Context::wait`] for what the engine thread sends `answer`, as a body
/// waits for its task: [`TaskError::Cancelled`] at cancel, and where the sender went
/// without an answer, `Stopped`, or `Cancelled` where it went for the job's cancel.
pub(crate) fn wait<T>(context: &Context<'_>, answer: &Receiver<T>) -> Result<T, TaskError> {
    match context.wait(answer) {
        Ok(value) => Ok(value),
        Err(Interrupted::Cancelled) => Err(TaskError::Cancelled),
        Err(Interrupted::Disconnected) if context.cancelled() => Err(TaskError::Cancelled),
        Err(Interrupted::Disconnected) => Err(TaskError::Stopped),
    }
}

/// A task on its way to the engine thread, or waiting there for a step.
pub(crate) struct Waiting {
    job: JobId,
    queue: Queue,
    task: Box<dyn Stepping>,
}

/// A [`GpuTask`] with its answer, its output type out of sight, so tasks of every kind
/// wait together.
trait Stepping: Send {
    /// One step; whether the task is over, its output then kept for [`Stepping::answer`],
    /// so a GPU error the step caused, heard only after it, still fails it.
    fn step(&mut self, gpu: &GpuContext) -> Result<bool, String>;
    /// Sends the body the output of the step that ended the task.
    fn answer(&mut self);
    /// Ends the task with `failure`, telling its body, its output dropped.
    fn fail(&mut self, failure: String);
}

struct Handing<T: GpuTask> {
    task: T,
    /// The last step's output, until the step's error scopes are heard.
    output: Option<T::Output>,
    answer: Option<Sender<Result<T::Output, TaskError>>>,
}

impl<T: GpuTask> Stepping for Handing<T> {
    fn step(&mut self, gpu: &GpuContext) -> Result<bool, String> {
        match self.task.step(gpu)? {
            Step::Again => Ok(false),
            Step::Done(output) => {
                self.output = Some(output);
                Ok(true)
            }
        }
    }

    fn answer(&mut self) {
        // A body that has stopped waiting hears nothing.
        if let (Some(answer), Some(output)) = (self.answer.take(), self.output.take()) {
            let _ = answer.send(Ok(output));
        }
    }

    fn fail(&mut self, failure: String) {
        self.output = None;
        if let Some(answer) = self.answer.take() {
            let _ = answer.send(Err(TaskError::Failed(failure)));
        }
    }
}

/// A waiting task as the queues hold it: its own identity, a number no other task has,
/// so `SystemForeground`'s deduplication, which is for jobs, never takes one for another.
struct Queued {
    identity: Identity,
    waiting: Waiting,
    /// Whether it has stepped, so the stack's limit never takes it.
    stepped: bool,
}

impl Scheduled for Queued {
    fn identity(&self) -> &Identity {
        &self.identity
    }
}

/// The last step's work on the GPU, and what `-d perf`'s trace tells of it (spike X1).
struct InFlight {
    /// Hears when a poll, the engine thread's or a frame's wait, found the step's work
    /// done, and everything submitted before it.
    heard: Receiver<Instant>,
    /// The task, as its queue names it.
    description: String,
    /// When its step ended, its submission made.
    submitted: Instant,
    /// How long its step took on the engine thread, recording and submitting.
    stepped: Duration,
    /// The looks at the GPU while it ran, and their time.
    polls: u32,
    polling: Duration,
}

/// The tasks waiting for the engine thread, in their jobs' queues, and the work of the
/// last step still on the GPU.
#[derive(Default)]
pub(crate) struct Background {
    queues: Queues<Queued>,
    /// The last step's work while it, or anything submitted before it, is on the GPU.
    in_flight: Option<InFlight>,
    /// The next task's number.
    next: u64,
}

impl Background {
    /// Whether a task waits or a step's work is on the GPU, so the engine thread looks
    /// for commands without blocking for long.
    pub(crate) fn waiting(&self) -> bool {
        !self.queues.is_empty() || self.in_flight.is_some()
    }

    /// Queues a task handed by a body, as darktable adds a job to its queue.
    pub(crate) fn add(&mut self, waiting: Waiting) {
        self.next += 1;
        let queue = waiting.queue;
        let identity = Identity {
            kind: "gpu task",
            key: Some(self.next.to_le_bytes().to_vec()),
            description: format!("gpu task {} of job {}", self.next, waiting.job),
        };
        let queued = Queued {
            identity,
            waiting,
            stepped: false,
        };
        // darktable's stack discards its bottom job past 30, never one that has started,
        // since a running job has left its queue; a task that has stepped waits in it, so
        // the oldest that has not stepped goes in its place, or the new one if none has
        // not (difference J17). The stack then never passes 30, and `schedule::add`
        // discards nothing.
        if queue == Queue::SystemForeground && self.queues.list(queue).len() >= schedule::MAX_JOBS {
            let oldest = self
                .queues
                .list(queue)
                .iter()
                .rev()
                .find(|e| !e.job.stepped);
            let Some(key) = oldest.map(|entry| entry.job.identity.key.clone()) else {
                let new = &queued.identity.description;
                tracing::debug!(target: "control", "dropped: {new}, past 30 in {queue}");
                return;
            };
            if let Some(dropped) = self.queues.remove(queue, |q| q.identity.key == key) {
                let old = &dropped.job.identity.description;
                tracing::debug!(target: "control", "dropped: {old}, past 30 in {queue}");
            }
        }
        self.queue(queue, queued);
    }

    /// [`schedule::add`]: a FIFO's end, or the stack's top, at the queue's priority.
    fn queue(&mut self, queue: Queue, queued: Queued) {
        match schedule::add(&mut self.queues, &[], queue, queued) {
            Added::Queued { dropped: None } => {}
            // The stack is kept within 30, and no two tasks are equal, so none of these
            // can happen; the task's body would hear it stopped.
            Added::Queued {
                dropped: Some(dropped),
            } => {
                tracing::warn!(target: "control", "dropped: {}, past 30 in {queue}", dropped.identity.description);
            }
            Added::Moved { new } | Added::Running { new } => {
                tracing::warn!(target: "control", "dropped: {}, equal to another", new.identity.description);
            }
        }
    }

    /// One step of one waiting task, once the last step's work is done on the GPU:
    /// the tasks of jobs no longer running dropped, then the head darktable's rule picks,
    /// every other head gaining one ([`schedule::pick`]). A task with more to do is added
    /// to its queue again, as a new job of it would be, at the priority its queue gives.
    pub(crate) fn step(&mut self, gpu: &GpuContext, control: &Control) {
        if !self.idle(gpu) {
            return;
        }
        self.sweep(control);
        let Some((queue, entry)) = schedule::pick(&mut self.queues, false) else {
            return;
        };
        let Queued {
            identity,
            mut waiting,
            ..
        } = entry.job;
        let description = identity.description.clone();
        let start = Instant::now();
        let outcome = stepped(gpu, waiting.task.as_mut());
        let stepped = start.elapsed();
        match outcome {
            // Its next step is a job of its queue again.
            Ok(false) => {
                let stepped = true;
                self.queue(
                    queue,
                    Queued {
                        identity,
                        waiting,
                        stepped,
                    },
                );
            }
            // Done, and the GPU refused nothing of its last step: the body hears it.
            Ok(true) => waiting.task.answer(),
            Err(failure) => {
                tracing::error!(target: "control", "{} failed: {failure}", identity.description);
                waiting.task.fail(failure);
            }
        }
        let (done, heard) = mpsc::channel();
        gpu.queue.on_submitted_work_done(move || {
            let _ = done.send(Instant::now());
        });
        self.in_flight = Some(InFlight {
            heard,
            description,
            submitted: Instant::now(),
            stepped,
            polls: 0,
            polling: Duration::ZERO,
        });
    }

    /// Whether the last step's work is done on the GPU, polling without waiting.
    fn idle(&mut self, gpu: &GpuContext) -> bool {
        let Some(in_flight) = &mut self.in_flight else {
            return true;
        };
        let start = Instant::now();
        if let Err(error) = gpu.device.poll(wgpu::PollType::Poll) {
            tracing::error!(target: "gpu", "polling the GPU: {error}");
        }
        let heard = in_flight.heard.try_recv();
        in_flight.polls += 1;
        in_flight.polling += start.elapsed();
        let done = match heard {
            Err(TryRecvError::Empty) => return false,
            Ok(done) => Some(done),
            Err(TryRecvError::Disconnected) => None,
        };
        if let (Some(done), true) = (
            done,
            tracing::enabled!(target: "perf", tracing::Level::TRACE),
        ) {
            let ms = |duration: Duration| duration.as_secs_f64() * 1e3;
            tracing::trace!(
                target: "perf",
                "{}: stepped in {:.2} ms, done {:.2} ms after, seen {:.2} ms later, {} polls, {:.3} ms",
                in_flight.description,
                ms(in_flight.stepped),
                ms(done.saturating_duration_since(in_flight.submitted)),
                ms(done.elapsed()),
                in_flight.polls,
                ms(in_flight.polling),
            );
        }
        self.in_flight = None;
        true
    }

    /// Drops every task whose job is no longer running: cancelled, so before its next
    /// step, or ended without waiting for it.
    fn sweep(&mut self, control: &Control) {
        for queue in Queue::ALL {
            while let Some(entry) = self.queues.remove(queue, |queued| {
                control.state(queued.waiting.job) != Some(State::Running)
            }) {
                tracing::debug!(
                    target: "control",
                    "dropped: {}, its job is not running",
                    entry.job.identity.description
                );
            }
        }
    }
}

/// Runs one step inside error scopes of its own, a panic caught: whether the task is
/// over, or why it failed, the GPU's first error for the step included.
fn stepped(gpu: &GpuContext, task: &mut dyn Stepping) -> Result<bool, String> {
    let memory = gpu.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| task.step(gpu)));
    let errors = [validation.pop(), memory.pop()].map(pollster::block_on);
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(payload) => Err(format!("panicked: {}", panic_message(payload.as_ref()))),
    };
    match errors.into_iter().flatten().next() {
        Some(error) => Err(format!("the GPU: {error}")),
        None => outcome,
    }
}

/// The test caller: a synthetic GPU job, whose task makes one small submission a step, told
/// to the test as it goes, and held where the test needs it held.
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::{Arc, Mutex, PoisonError};

    use super::*;

    /// What a synthetic job tells its test, by its label.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Told {
        /// Its body handed its task.
        Handed(&'static str),
        /// Its task's step, from 1, began.
        Began(&'static str, usize),
        /// Its body heard its task's end: its steps, or why not.
        Ended(&'static str, Result<usize, TaskError>),
    }

    /// Where a synthetic job goes wrong.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Fault {
        None,
        /// Its body panics before it hands its task.
        BodyPanics,
        /// This step panics.
        StepPanics(usize),
        /// This step fails.
        StepFails(usize),
        /// This step records a copy the GPU refuses.
        StepInvalid(usize),
    }

    type Gate = Arc<Mutex<Receiver<()>>>;

    /// A synthetic job, as a test starts one with `Command::Synthetic`.
    #[derive(Debug, Clone)]
    pub struct Synthetic {
        pub label: &'static str,
        pub queue: Queue,
        steps: usize,
        cancellable: bool,
        fault: Fault,
        told: Sender<Told>,
        /// Held before it hands its task until the test says go, through its context.
        held: Option<Gate>,
        /// Its first `gated` steps each wait for the test's permit, holding the engine
        /// thread as a long step would; a permit sender dropped lets them all through.
        gated: usize,
        permits: Option<Gate>,
        /// How many tasks its body hands at once, each of `steps` steps, before it waits
        /// for each in turn, telling each one's end.
        tasks: usize,
    }

    impl PartialEq for Synthetic {
        fn eq(&self, other: &Self) -> bool {
            (self.label, self.queue, self.steps) == (other.label, other.queue, other.steps)
        }
    }

    impl Synthetic {
        /// A job of `steps` steps in `queue`, with a progress that cannot be cancelled.
        pub fn new(label: &'static str, queue: Queue, steps: usize, told: &Sender<Told>) -> Self {
            Self {
                label,
                queue,
                steps,
                cancellable: false,
                fault: Fault::None,
                told: told.clone(),
                held: None,
                gated: 0,
                permits: None,
                tasks: 1,
            }
        }

        /// Its body hands `tasks` tasks at once.
        pub fn tasks(self, tasks: usize) -> Self {
            Self { tasks, ..self }
        }

        pub fn cancellable(self) -> Self {
            Self {
                cancellable: true,
                ..self
            }
        }

        pub fn fault(self, fault: Fault) -> Self {
            Self { fault, ..self }
        }

        /// Its body waits for the go this returns before it hands its task.
        pub fn held(self) -> (Self, Sender<()>) {
            let (go, gone) = mpsc::channel();
            let held = Some(Arc::new(Mutex::new(gone)));
            (Self { held, ..self }, go)
        }

        /// Its first `steps` steps each wait for a permit this returns.
        pub fn gated(self, steps: usize) -> (Self, Sender<()>) {
            let (permit, permits) = mpsc::channel();
            let permits = Some(Arc::new(Mutex::new(permits)));
            (
                Self {
                    gated: steps,
                    permits,
                    ..self
                },
                permit,
            )
        }

        /// The job, its task handed through `tasks`.
        pub(crate) fn job(self, tasks: Tasks) -> Job {
            let progress = (Message::new("job-synthetic"), self.cancellable);
            let label = self.label;
            Job::new("synthetic", label, move |context| {
                if self.fault == Fault::BodyPanics {
                    panic!("the synthetic job {label} panicked");
                }
                if let Some(held) = &self.held {
                    let gone = held.lock().unwrap_or_else(PoisonError::into_inner);
                    if context.wait(&gone).is_err() {
                        return Ok(());
                    }
                }
                context.set_message(Message::new("job-synthetic-steps").with("steps", self.steps));
                let handed: Vec<_> = (0..self.tasks)
                    .map(|_| {
                        let task = Steps {
                            label,
                            steps: self.steps,
                            done: 0,
                            gated: self.gated,
                            permits: self.permits.clone(),
                            fault: self.fault,
                            told: self.told.clone(),
                            buffer: None,
                        };
                        tasks.hand(context, task)
                    })
                    .collect();
                let _ = self.told.send(Told::Handed(label));
                let mut failed = None;
                for handed in handed {
                    let result = handed.wait(context);
                    let _ = self.told.send(Told::Ended(label, result.clone()));
                    match result {
                        Ok(_) | Err(TaskError::Cancelled) => {}
                        Err(error) => failed = failed.or(Some(error)),
                    }
                }
                match failed {
                    None if !context.cancelled() => {
                        context.set_fraction(1.0);
                        Ok(())
                    }
                    None => Ok(()),
                    Some(error) => {
                        Err(Message::new("job-synthetic-failed").with("reason", error.to_string()))
                    }
                }
            })
            .progress(progress.0, progress.1)
        }
    }

    /// The synthetic job's task: a buffer cleared a step.
    struct Steps {
        label: &'static str,
        steps: usize,
        done: usize,
        gated: usize,
        permits: Option<Gate>,
        fault: Fault,
        told: Sender<Told>,
        buffer: Option<wgpu::Buffer>,
    }

    impl GpuTask for Steps {
        type Output = usize;

        fn step(&mut self, gpu: &GpuContext) -> Result<Step<usize>, String> {
            let step = self.done + 1;
            let _ = self.told.send(Told::Began(self.label, step));
            if step <= self.gated
                && let Some(permits) = &self.permits
            {
                let _ = permits
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .recv();
            }
            match self.fault {
                Fault::StepPanics(at) if at == step => panic!("step {step} panicked"),
                Fault::StepFails(at) if at == step => return Err(format!("step {step} failed")),
                _ => {}
            }
            let buffer = self.buffer.get_or_insert_with(|| {
                gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("synthetic task"),
                    size: 256,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            });
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            encoder.clear_buffer(buffer, 0, None);
            if self.fault == Fault::StepInvalid(step) {
                // Past the buffer's end: the GPU refuses it.
                encoder.copy_buffer_to_buffer(buffer, 0, buffer, 128, 1024);
            }
            gpu.submit([encoder.finish()]);
            self.done = step;
            Ok(if step == self.steps {
                Step::Done(step)
            } else {
                Step::Again
            })
        }
    }
}
