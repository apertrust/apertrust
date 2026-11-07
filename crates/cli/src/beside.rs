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

//! `preview --beside-export`, spike X1's harness:
//! exposure changes sent to an in-process session as a drag sends them, one every
//! interval, their frames' latency timed with nothing else to do and while the session's
//! export job exports the same photo, and the export's own time alone and beside them. The
//! photo is opened through a link in a folder of its own, so neither its edit nor the
//! exports touch the folder it is in.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use apertrust_engine::gpu::GpuContext;
use apertrust_engine::jobs::{JobEvent, JobId};
use apertrust_engine::session::{self, Command, Event, ExportSettings, Session, SessionError};
use apertrust_engine::settings as engine;
use apertrust_export::settings::OnConflict;
use apertrust_export::size::Request;
use apertrust_export::{Format, Places};
use apertrust_modules::ControlValue;

use crate::{EXPOSURE, MODULES, OPERATIONS};

/// What `preview --beside-export` runs.
pub(crate) struct Options {
    /// The viewport's bounds, as `Command::Resize` gives them.
    pub bounds: (u32, u32),
    /// The exposure changes timed alone in each round.
    pub changes: u32,
    /// Between two changes, as a drag sends one a display frame.
    pub interval: Duration,
    /// Rounds of: changes alone, an export alone, an export beside changes.
    pub rounds: u32,
    /// The export's format.
    pub format: Format,
}

/// A frame as the client received it.
struct Sample {
    through: u64,
    latency_ms: f64,
    /// Its render's wall time on the engine thread, submit to done.
    render_ms: f64,
}

/// The session, as this harness drives it.
struct Client {
    session: Session,
    wakes: mpsc::Receiver<()>,
    frames: Vec<Sample>,
    /// The Exposure module's Exposure as the session last reported it.
    exposure: Option<f32>,
    /// The export job started by the command of this number, once added, and whether it
    /// has ended.
    export: Option<(u64, Option<JobId>, bool)>,
    opened: bool,
}

impl Client {
    /// Takes what has arrived, waiting at most `timeout` for the first.
    fn pump(&mut self, timeout: Duration) -> Result<()> {
        let _ = self.wakes.recv_timeout(timeout);
        for event in self.session.events() {
            match event {
                Event::Opened(_) => self.opened = true,
                Event::Frame(frame) => self.frames.push(Sample {
                    through: frame.through,
                    latency_ms: frame.latency_ms,
                    render_ms: frame.timings.wall_ms,
                }),
                Event::Values(values) => {
                    let control = values
                        .modules
                        .iter()
                        .filter(|module| module.module == EXPOSURE.0)
                        .flat_map(|module| &module.controls)
                        .find(|control| control.control == EXPOSURE.1);
                    if let Some(ControlValue::Number(ev)) = control.map(|c| &c.value) {
                        self.exposure = Some(*ev);
                    }
                }
                Event::Job { event, command } => match (event, &mut self.export) {
                    (JobEvent::Added { job, .. }, Some((sent, added, _)))
                        if command == Some(*sent) =>
                    {
                        *added = Some(job);
                    }
                    (JobEvent::Ended { job, failure, .. }, Some((_, Some(own), ended)))
                        if job == *own =>
                    {
                        if let Some(failure) = failure {
                            bail!("the export failed: {}", failure.id);
                        }
                        *ended = true;
                    }
                    _ => {}
                },
                Event::Export {
                    status:
                        session::ExportStatus::Done {
                            result: session::PhotoResult::Failed(failure),
                            ..
                        },
                    ..
                } => bail!("the export failed: {}", failure.message),
                Event::Failed(SessionError::Stopped) => bail!("the engine stopped"),
                Event::Failed(error) => bail!("{error}"),
                _ => {}
            }
        }
        Ok(())
    }

    /// Waits until `done` holds of what has arrived, for two minutes at most.
    fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(120);
        while !done(self) {
            if Instant::now() > deadline {
                bail!("no {what} within two minutes");
            }
            self.pump(Duration::from_millis(50))?;
        }
        Ok(())
    }

    /// One change of the Exposure module's Exposure, as a drag sends it.
    fn change(&self, ev: f32) -> u64 {
        self.session.send(Command::SetControl {
            module: EXPOSURE.0.to_owned(),
            control: EXPOSURE.1.to_owned(),
            value: ControlValue::Number(ev),
            done: false,
        })
    }

    /// The frames that include commands after `after`, up to `through`.
    fn frames_in(&self, after: u64, through: u64) -> Vec<&Sample> {
        self.frames
            .iter()
            .filter(|frame| frame.through > after && frame.through <= through)
            .collect()
    }

    fn export_ended(&self) -> bool {
        matches!(self.export, Some((_, _, true)))
    }
}

/// Spike X1's measurement of `path`, as [`Options`] says.
pub(crate) fn run(path: &Path, options: Options, engine: engine::Options) -> Result<()> {
    let folder = std::env::temp_dir().join(format!("apertrust-x1-{}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
    let result = measure(&linked(path, &folder)?, &folder, &options, engine);
    let _ = std::fs::remove_dir_all(&folder);
    result
}

/// `path` linked into `folder` under its own name, so its edit is written there.
fn linked(path: &Path, folder: &Path) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .with_context(|| format!("{}", path.display()))?;
    let link = folder.join(path.file_name().context("a raw file")?);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&path, &link)?;
    #[cfg(not(unix))]
    std::fs::copy(&path, &link)?;
    Ok(link)
}

fn measure(photo: &Path, folder: &Path, options: &Options, engine: engine::Options) -> Result<()> {
    let gpu = pollster::block_on(GpuContext::new(&engine.gpu))?;
    println!("GPU: {}", gpu.adapter.get_info().name);
    let (woken, wakes) = mpsc::channel();
    let session = Session::start(
        gpu,
        OPERATIONS,
        MODULES,
        apertrust_base::program!().version,
        engine.budget,
        move || {
            let _ = woken.send(());
        },
    );
    let mut client = Client {
        session,
        wakes,
        frames: Vec::new(),
        exposure: None,
        export: None,
        opened: false,
    };
    client.session.send(Command::Open(photo.to_owned()));
    let (width, height) = options.bounds;
    let resize = client.session.send(Command::Resize { width, height });
    client.until("first frame", |c| {
        c.opened && c.frames.last().is_some_and(|f| f.through >= resize)
    })?;
    let base = client.exposure.context("the Exposure module's values")?;
    // A hundredth of a stop, back from +4 rather than past it, as `preview` moves.
    let step = if base + 0.01 > 4.0 { -0.01 } else { 0.01 };
    let mut sent = 0u32;
    let mut next_ev = || {
        sent += 1;
        base + if sent % 2 == 1 { step } else { 0.0 }
    };
    let settings = ExportSettings {
        path: format!("{}/$(FILE_NAME)", folder.display()),
        on_conflict: OnConflict::Overwrite,
        format: options.format,
        size: Request {
            width: 0,
            height: 0,
            scale: None,
            upscale: false,
            high_quality: true,
            dialog: false,
        },
    };
    let export = |client: &mut Client| {
        let command = client.session.send(Command::Export {
            photos: vec![photo.to_owned()],
            settings: settings.clone(),
            places: Places::default(),
        });
        client.export = Some((command, None, false));
        command
    };

    // Warming up: the kernels a change runs, and the GPU's clocks.
    let mut last = 0;
    for _ in 0..10 {
        last = client.change(next_ev());
        client.pump(options.interval)?;
    }
    client.until("warm-up frame", |c| {
        c.frames.last().is_some_and(|f| f.through >= last)
    })?;

    let (mut alone, mut beside) = (Vec::new(), Vec::new());
    let (mut export_alone, mut export_beside) = (Vec::new(), Vec::new());
    for round in 1..=options.rounds {
        // Changes with nothing else to do.
        let after = last;
        let paced = Instant::now();
        for k in 1..=options.changes {
            last = client.change(next_ev());
            wait_until(&mut client, paced + options.interval * k)?;
        }
        client.until("frame", |c| {
            c.frames.last().is_some_and(|f| f.through >= last)
        })?;
        let frames = client.frames_in(after, last);
        let latencies: Vec<f64> = frames.iter().map(|f| f.latency_ms).collect();
        let render: Vec<f64> = frames.iter().map(|f| f.render_ms).collect();
        println!(
            "Round {round}, alone: {} changes, {} frames, latency {}, render {}",
            options.changes,
            frames.len(),
            stats(&latencies),
            stats(&render)
        );
        alone.extend(latencies);

        // The export with nothing else to do.
        let start = Instant::now();
        export(&mut client);
        client.until("export", Client::export_ended)?;
        let ms = start.elapsed().as_secs_f64() * 1e3;
        println!("Round {round}, export alone: {ms:.0} ms");
        export_alone.push(ms);

        // The export beside changes, sent until it ends.
        let after = last;
        let start = Instant::now();
        export(&mut client);
        let mut changes = 0;
        while !client.export_ended() {
            changes += 1;
            last = client.change(next_ev());
            wait_until(&mut client, start + options.interval * changes)?;
        }
        let ms = start.elapsed().as_secs_f64() * 1e3;
        client.until("frame", |c| {
            c.frames.last().is_some_and(|f| f.through >= last)
        })?;
        // The export's command is one of those after `after`; its frames are the changes'.
        let frames = client.frames_in(after, last);
        let latencies: Vec<f64> = frames.iter().map(|f| f.latency_ms).collect();
        let render: Vec<f64> = frames.iter().map(|f| f.render_ms).collect();
        println!(
            "Round {round}, beside the export: {changes} changes, {} frames, latency {}, render {}; export {ms:.0} ms",
            frames.len(),
            stats(&latencies),
            stats(&render)
        );
        beside.extend(latencies);
        export_beside.push(ms);
    }
    println!(
        "All rounds: latency alone {}; beside the export {}",
        stats(&alone),
        stats(&beside)
    );
    println!(
        "Export: alone {}; beside the changes {}",
        stats(&export_alone),
        stats(&export_beside)
    );
    Ok(())
}

/// Takes events until `at`.
fn wait_until(client: &mut Client, at: Instant) -> Result<()> {
    loop {
        let now = Instant::now();
        if now >= at {
            return Ok(());
        }
        client.pump(at - now)?;
    }
}

/// `values`' median, 95th percentile and worst, in milliseconds.
fn stats(values: &[f64]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
    format!(
        "median {:.2}, p95 {:.2}, worst {:.2} ms",
        at(0.5),
        at(0.95),
        sorted[sorted.len() - 1]
    )
}
