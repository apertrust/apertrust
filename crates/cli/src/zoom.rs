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

//! `preview --zoom`: the zoomed view (decision 99) through an in-process session, as the
//! app sends it: a `Resize` to the bounds, then a `View`. It times entering the zoom, pans
//! (`--pans`), each inside the rendered region's margin rendering nothing, and exposure
//! changes there, each frame rendering the fitted viewport and the detail in one
//! submission, and writes the zoomed frame with `--png`. The photo is opened through a link
//! in a folder of its own, so its edit is never written beside it.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use apertrust_engine::gpu::GpuContext;
use apertrust_engine::session::{Command, Event, Frame, Session, SessionError, View, ViewScale};
use apertrust_engine::settings as engine;
use apertrust_engine::sidecar;
use apertrust_export::{Format, png};
use apertrust_tools::ControlValue;

use crate::{EXPOSURE, OPERATIONS, TOOLS, beside, median, write_file};

/// What `preview --zoom` runs.
pub(crate) struct Options {
    /// The viewport's bounds, as `Command::Resize` gives them.
    pub bounds: (u32, u32),
    /// The zoom, 1 being 100%.
    pub factor: f32,
    /// The point shown at the bounds' centre, in the photo's coordinates.
    pub centre: [f32; 2],
    /// The Exposure tool's Exposure to start from; the saved edit's when `None`.
    pub exposure: Option<f32>,
    /// The pans to time once zoomed, each a tenth of the view across.
    pub pans: u32,
    /// The exposure changes to time once zoomed.
    pub changes: u32,
    /// Where to write the zoomed frame, as export writes a PNG.
    pub png: Option<PathBuf>,
}

/// The session, as this harness drives it.
struct Client {
    session: Session,
    wakes: mpsc::Receiver<()>,
    /// The latest frame, and when it arrived.
    frame: Option<(Frame, Instant)>,
    /// The Exposure tool's Exposure as the session last reported it.
    exposure: Option<f32>,
    opened: bool,
    /// Whether the photo opened for viewing only: a newer Apertrust wrote its edit, and
    /// the session refuses every change.
    view_only: bool,
    /// The commands the latest view fact includes.
    viewed: u64,
}

impl Client {
    /// Takes what has arrived, waiting at most `timeout` for the first.
    fn pump(&mut self, timeout: Duration) -> Result<()> {
        let _ = self.wakes.recv_timeout(timeout);
        for event in self.session.events() {
            match event {
                Event::Opened(photo) => {
                    self.opened = true;
                    self.view_only = photo.view_only.is_some();
                }
                Event::View(state) => self.viewed = state.through,
                Event::Frame(frame) => self.frame = Some((frame, Instant::now())),
                Event::Values(values) => {
                    let control = (values.tools.iter())
                        .filter(|tool| tool.tool == EXPOSURE.0)
                        .flat_map(|tool| &tool.controls)
                        .find(|control| control.control == EXPOSURE.1);
                    if let Some(ControlValue::Number(ev)) = control.map(|c| &c.value) {
                        self.exposure = Some(*ev);
                    }
                }
                Event::Failed(SessionError::Stopped) => bail!("the engine stopped"),
                Event::Failed(error) => bail!("{error}"),
                _ => {}
            }
        }
        Ok(())
    }

    /// The frame that includes the command numbered `sent`, waiting two minutes at most.
    fn frame(&mut self, sent: u64) -> Result<(Frame, Instant)> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if let Some((frame, at)) = &self.frame
                && self.opened
                && frame.through >= sent
            {
                return Ok((frame.clone(), *at));
            }
            if Instant::now() > deadline {
                bail!("no frame within two minutes");
            }
            self.pump(Duration::from_millis(50))?;
        }
    }

    /// Waits for the view fact that includes the command numbered `sent`, two minutes at
    /// most.
    fn view(&mut self, sent: u64) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(120);
        while self.viewed < sent {
            if Instant::now() > deadline {
                bail!("no view within two minutes");
            }
            self.pump(Duration::from_millis(50))?;
        }
        Ok(())
    }

    /// The Exposure tool's Exposure set to `ev`, as a drag sends it.
    fn change(&self, ev: f32, done: bool) -> u64 {
        self.session.send(Command::SetControl {
            tool: EXPOSURE.0.to_owned(),
            control: EXPOSURE.1.to_owned(),
            value: ControlValue::Number(ev),
            done,
        })
    }
}

/// The GPU time of `frame`'s render, both pipelines' passes, in milliseconds.
fn gpu_ms(frame: &Frame) -> f64 {
    frame.timings.passes.iter().map(|(_, ms)| ms).sum()
}

/// `--pans`: from `view`, whose frame is `entered`, the centre moved across by a tenth of
/// the view `options.pans` times, back from the photo's edge, each waited for: its view
/// fact, and its frame where the visible rectangle leaves the region the latest frame
/// holds, as a client draws it. Prints how many rendered and each render's time; stops
/// where the photo's width fits the bounds, which no pan across moves.
fn pan(client: &mut Client, view: View, entered: &Frame, options: &Options) -> Result<()> {
    let place = entered.place;
    let mut region = [
        place.origin[0],
        place.origin[1],
        entered.width,
        entered.height,
    ];
    let placed = apertrust_engine::view::place(place.extent, options.bounds, &view);
    let step = placed.visible[2] / 10.0 / f64::from(placed.size[0]);
    let mut centre = placed.centre;
    let mut direction = 1.0;
    let mut renders = Vec::new();
    for pan in 1..=options.pans {
        let mut to = view;
        let mut placed = placed;
        for _ in 0..2 {
            to.centre = [(f64::from(centre[0]) + direction * step) as f32, centre[1]];
            placed = apertrust_engine::view::place(place.extent, options.bounds, &to);
            if placed.centre != centre {
                break;
            }
            // At the photo's edge: back the other way.
            direction = -direction;
        }
        // Where the photo's width fits the bounds, no pan across moves the view.
        if placed.centre == centre {
            println!("The photo's width fits the bounds: a pan across moves nothing");
            return Ok(());
        }
        let sent = client.session.send(Command::View(to));
        centre = placed.centre;
        if apertrust_engine::view::covers(region, placed.visible) {
            client.view(sent)?;
        } else {
            client.frame(sent)?;
        }
        // A frame for the pan, expected or not, is a render.
        if let Some((frame, _)) = &client.frame
            && frame.through >= sent
        {
            let place = frame.place;
            region = [place.origin[0], place.origin[1], frame.width, frame.height];
            tracing::debug!(
                target: "perf",
                "zoom: pan {pan}: rendered {}×{} at {},{}: {:.2} ms to the frame, {:.2} ms GPU",
                frame.width,
                frame.height,
                place.origin[0],
                place.origin[1],
                frame.latency_ms,
                gpu_ms(frame)
            );
            renders.push((pan, frame.latency_ms, gpu_ms(frame)));
        }
    }
    println!(
        "Pans of a tenth of the view: {} of {} rendered",
        renders.len(),
        options.pans
    );
    for (pan, latency, gpu) in renders {
        println!("  pan {pan}: {latency:.2} ms to the frame, {gpu:.2} ms GPU");
    }
    Ok(())
}

/// `preview --zoom` of `path`, as [`Options`] says, from its saved edit: the raw linked
/// and its sidecar copied into a folder this run makes, so what the session saves never
/// reaches the photo's own.
pub(crate) fn run(path: &Path, options: Options, engine: engine::Options) -> Result<()> {
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let name = format!("apertrust-zoom-{}-{started}", std::process::id());
    let folder = std::env::temp_dir().join(name);
    // Its own, and on Unix its owner's alone, as it holds a copy of the edit.
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut made = std::fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut made, 0o700);
    made.create(&folder)
        .with_context(|| format!("{}", folder.display()))?;
    let staged = beside::linked(path, &folder).and_then(|photo| {
        let saved = sidecar::path_for(path);
        if saved.is_file() {
            std::fs::copy(&saved, sidecar::path_for(&photo))
                .with_context(|| format!("{}", saved.display()))?;
        }
        Ok(photo)
    });
    let result = staged.and_then(|photo| zoom(&photo, &options, engine));
    let _ = std::fs::remove_dir_all(&folder);
    result
}

fn zoom(photo: &Path, options: &Options, engine: engine::Options) -> Result<()> {
    let gpu = pollster::block_on(GpuContext::new(&engine.gpu))?;
    println!("GPU: {}", gpu.adapter.get_info().name);
    let (woken, wakes) = mpsc::channel();
    let session = Session::start(
        gpu.clone(),
        OPERATIONS,
        TOOLS,
        apertrust_base::program!().version,
        engine.view(),
        // The CLI writes no disk cache (decision 43).
        None,
        move || {
            let _ = woken.send(());
        },
    );
    let mut client = Client {
        session,
        wakes,
        frame: None,
        exposure: None,
        opened: false,
        view_only: false,
        viewed: 0,
    };
    client.session.send(Command::Open(photo.to_owned()));
    let (width, height) = options.bounds;
    let resize = client.session.send(Command::Resize { width, height });
    let (fitted, _) = client.frame(resize)?;
    println!(
        "At Fit: {}×{}, {:.1} ms GPU",
        fitted.width,
        fitted.height,
        gpu_ms(&fitted)
    );
    // An --exposure the saved edit has already changes nothing, and renders no frame. A
    // photo open for viewing only takes no change.
    let saved = client.exposure.context("the Exposure tool's values")?;
    if client.view_only && options.exposure.is_some_and(|ev| ev != saved) {
        bail!(
            "--exposure: the photo is open for viewing only, as a newer Apertrust wrote its edit"
        );
    }
    let base = match options.exposure {
        Some(ev) if ev != saved => {
            let sent = client.change(ev, true);
            client.frame(sent)?;
            ev
        }
        Some(_) | None => saved,
    };

    // Entering the zoom: the detail pipeline built and rendered beside the fitted frame.
    let view = View {
        scale: ViewScale::Factor(options.factor),
        centre: options.centre,
    };
    let sent_at = Instant::now();
    let viewed = client.session.send(Command::View(view));
    // Only a view the engine renders past Fit sends a frame: from 100% on, for now.
    let placed = apertrust_engine::view::place(fitted.place.extent, options.bounds, &view);
    let zoomed = placed.render >= 1.0 && fitted.place.scale < 1.0;
    if !zoomed {
        client.view(viewed)?;
        println!(
            "The frame stays at Fit at {:.0}%: the detail renders from 100% so far.",
            placed.shown * 100.0
        );
    } else {
        let (entered, arrived) = client.frame(viewed)?;
        let entered_ms = arrived.duration_since(sent_at).as_secs_f64() * 1e3;
        tracing::debug!(
            target: "perf",
            "zoom: entered in {entered_ms:.1} ms, GPU: {:.1} ms",
            gpu_ms(&entered)
        );
        let place = entered.place;
        // The factor as the session holds it, clamped to 16.
        println!(
            "Zoomed to {}: {}×{} at {},{} of {}×{}, scale {}; entered in {entered_ms:.1} ms ({:.1} ms GPU)",
            placed.shown,
            entered.width,
            entered.height,
            place.origin[0],
            place.origin[1],
            place.extent[0],
            place.extent[1],
            place.scale,
            gpu_ms(&entered)
        );
        if options.pans > 0 {
            pan(&mut client, view, &entered, options)?;
        }
    }

    // Exposure changes, as a slider drag sends them, each waited for: zoomed, both
    // pipelines from exposure in one submission; at Fit, the fitted one.
    let step = if base + 0.01 > 4.0 { -0.01 } else { 0.01 };
    let (mut times, mut latency) = (Vec::new(), Vec::new());
    // The latest frame's: the zoomed one, or at Fit any.
    let mut last = if zoomed { viewed } else { 0 };
    let changes = if client.view_only { 0 } else { options.changes };
    if client.view_only && options.changes > 0 {
        println!("No changes timed: the photo is open for viewing only.");
    }
    for change in 1..=changes {
        let ev = base + if change % 2 == 1 { step } else { 0.0 };
        last = client.change(ev, false);
        let (frame, _) = client.frame(last)?;
        tracing::debug!(
            target: "perf",
            "zoom: change {change}: GPU: {:.2} ms, {} passes{}",
            gpu_ms(&frame),
            frame.timings.passes.len(),
            if frame.overview.is_some() {
                ", both pipelines"
            } else {
                ""
            }
        );
        times.push(gpu_ms(&frame));
        latency.push(frame.latency_ms);
    }
    if !times.is_empty() {
        let changes = times.len();
        let (gpu, gpu_min) = median(times);
        let (latency, latency_min) = median(latency);
        println!(
            "Per exposure change, median of {changes}: {gpu:.2} ms GPU (min {gpu_min:.2}); {latency:.2} ms latency (min {latency_min:.2})"
        );
    }

    if let Some(png_path) = &options.png {
        // The frame shown at the exposure it started from: an odd number of changes left
        // it a step away.
        if changes % 2 == 1 {
            last = client.change(base, false);
        }
        let (frame, _) = client.frame(last)?;
        let pixels = frame.download(&gpu)?;
        write_file(
            png_path,
            &Format::Png(png::Options::default()),
            &pixels,
            frame.width,
            frame.height,
        )?;
        println!(
            "Wrote {}×{} to {}",
            frame.width,
            frame.height,
            png_path.display()
        );
    }
    Ok(())
}
