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

//! The Colour picker's picker through the session (decision 79; `docs/pickers.md`,
//! Tests): every command, its region kept on its part of the scene through every
//! orientation, its reading after the frame it measured and equal to the frame's own
//! statistics under it, the refusals, turning off, and a failure that leaves the frames;
//! and module pickers: read at their operation, applied through their pick as a gesture,
//! turning their module on and off with it, and in the raw stage per CFA colour.

use apertrust_modules::colour_balance_rgb::ColourBalanceRgb;
use apertrust_modules::orientation::Orientation;
use apertrust_modules::{Picker, PickerKind, Quad, Space};

use super::*;
use crate::picker::{DEFAULT_AREA, DEFAULT_POINT, pixels};

fn picker(command: PickerCommand) -> Command {
    Command::Picker(command)
}

fn on(area: bool) -> Command {
    picker(PickerCommand::On {
        picker: PickerRef::ColourPicker,
        area,
        keys: PickKeys::default(),
        tools: Vec::new(),
        channel: None,
    })
}

/// `On` for [`Sample`]'s picker `id`, an area where it takes both.
fn on_module(id: &'static str, area: bool) -> Command {
    on_picker("sample", id, area)
}

/// `On` for the picker `id` of `module`.
fn on_picker(module: &'static str, id: &'static str, area: bool) -> Command {
    picker(PickerCommand::On {
        picker: PickerRef::Module { module, picker: id },
        area,
        keys: PickKeys::default(),
        tools: Vec::new(),
        channel: None,
    })
}

fn point(at: [f32; 2]) -> Command {
    picker(PickerCommand::Point {
        at,
        done: true,
        keys: PickKeys::default(),
    })
}

fn area(from: [f32; 2], to: [f32; 2]) -> Command {
    picker(PickerCommand::Area {
        from,
        to,
        done: true,
        keys: PickKeys::default(),
    })
}

/// The pickers' events in `events`.
fn readings(events: &[Event]) -> Vec<&Option<PickerReading>> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Picker(reading) => Some(reading),
            _ => None,
        })
        .collect()
}

impl Driven {
    /// The pickers' readings still on their way, picked up as the thread picks them up,
    /// looking at the GPU between commands.
    fn picked(&mut self) -> Vec<Event> {
        let start = Instant::now();
        let mut events = Vec::new();
        loop {
            events.extend(self.engine.picked());
            if !self.engine.pickers.pending() {
                return events;
            }
            assert!(start.elapsed() < TEST_WAIT, "no reading");
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    /// The one reading that `commands` bring, measured on the frame shown.
    fn reading(&mut self, commands: impl IntoIterator<Item = Command>) -> PickerReading {
        let events = self.run(commands);
        assert!(failures(&events).is_empty(), "{events:?}");
        let picked = self.picked();
        match &picked[..] {
            [Event::Picker(Some(reading))] => reading.clone(),
            _ => panic!("{events:?} then {picked:?}"),
        }
    }
}

/// darktable's statistics of the frame's `pixels`, of `size`, under `region`, in f64, as
/// the reading's `FrameReading` is made from them; `None` where the region misses it.
fn expected(pixels_: &[[f32; 4]], size: [u32; 2], region: Region) -> Option<FrameReading> {
    let [left, top, right, bottom] = pixels(region, size)?;
    let (mut sum, mut low, mut high) = ([0.0f64; 4], [f32::MAX; 4], [-f32::MAX; 4]);
    for y in top..bottom {
        for x in left..right {
            let v = pixels_[(y * size[0] + x) as usize];
            for c in 0..4 {
                sum[c] += f64::from(v[c]);
                low[c] = low[c].min(v[c]);
                high[c] = high[c].max(v[c]);
            }
        }
    }
    let count = f64::from((right - left) * (bottom - top));
    Some(FrameReading::of(&Stats {
        mean: sum.map(|s| (s / count) as f32),
        min: low,
        max: high,
    }))
}

/// Whether `read` is `want`, the means within float rounding, the rest exactly.
fn agrees(read: Option<FrameReading>, want: Option<FrameReading>) -> bool {
    match (read, want) {
        (None, None) => true,
        (Some(read), Some(want)) => {
            let near = |a: f32, b: f32, by: f32| (a - b).abs() <= by * b.abs().max(1.0);
            read.rgb[1..] == want.rgb[1..]
                && (0..3).all(|c| near(read.rgb[0][c], want.rgb[0][c], 1e-5))
                && (0..3).all(|s| (0..3).all(|c| near(read.lab[s][c], want.lab[s][c], 1e-3)))
        }
        _ => false,
    }
}

/// Whether two regions are the same within float rounding.
fn same(a: Region, b: Region) -> bool {
    let near = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5);
    match (a, b) {
        (Region::Point(a), Region::Point(b)) => near(&a, &b),
        (Region::Area(a), Region::Area(b)) => near(&a, &b),
        _ => false,
    }
}

#[test]
fn every_command_measures_the_frame_shown_under_its_region() {
    let path = scratch("picker-commands");
    let mut driven = Driven::new(&[&Sample]);
    let events = driven.open(&path);
    let (pixels_, _) = frame_pixels(&driven.gpu, &events);
    let size = [40, 30];
    let check = |driven: &mut Driven, commands: Vec<Command>, region: Region| {
        let reading = driven.reading(commands);
        assert!(same(reading.region, region), "{reading:?}");
        assert_eq!(reading.picker, PickerRef::ColourPicker);
        // The frame shown, measured with no render, with every command so far.
        assert_eq!(reading.through, driven.engine.applied);
        let want = expected(&pixels_, size, reading.region);
        assert!(agrees(reading.frame, want), "{reading:?}, {want:?}");
    };

    // On, at darktable's default point; a point, then the default area by a switch.
    check(&mut driven, vec![on(false)], DEFAULT_POINT);
    check(
        &mut driven,
        vec![point([0.25, 0.5])],
        Region::Point([0.25, 0.5]),
    );
    check(
        &mut driven,
        vec![picker(PickerCommand::Switch)],
        DEFAULT_AREA,
    );
    // An area by its corners in any order, then back to the last point.
    let moved = Region::Area([0.1, 0.2, 0.6, 0.7]);
    check(&mut driven, vec![area([0.6, 0.7], [0.1, 0.2])], moved);
    check(
        &mut driven,
        vec![picker(PickerCommand::Switch)],
        Region::Point([0.25, 0.5]),
    );
    // A reset takes the kind measured back to its default, the other kind kept.
    check(
        &mut driven,
        vec![picker(PickerCommand::Reset)],
        DEFAULT_POINT,
    );
    check(&mut driven, vec![picker(PickerCommand::Switch)], moved);
    check(
        &mut driven,
        vec![picker(PickerCommand::Reset)],
        DEFAULT_AREA,
    );
    // A reading that changes nothing a client shows is not sent again.
    let events = driven.run([picker(PickerCommand::Reset)]);
    assert!(
        events.is_empty() && driven.picked().is_empty(),
        "{events:?}"
    );
    // Coordinates outside the photo are clamped to it.
    check(
        &mut driven,
        vec![area([-1.0, 0.5], [f32::NAN, 2.0])],
        Region::Area([0.0, 0.5, 0.0, 1.0]),
    );

    // Off, sent at once, its places kept for its next `On`, each kind at its last.
    check(
        &mut driven,
        vec![point([0.75, 0.25])],
        Region::Point([0.75, 0.25]),
    );
    let events = driven.run([picker(PickerCommand::Off)]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    assert!(driven.picked().is_empty());
    check(
        &mut driven,
        vec![on(true)],
        Region::Area([0.0, 0.5, 0.0, 1.0]),
    );
    check(&mut driven, vec![on(false)], Region::Point([0.75, 0.25]));

    // A point on the photo's right edge falls in no pixel, as darktable's: no frame part.
    let reading = driven.reading([point([1.0, 0.5])]);
    assert_eq!(reading.frame, None);
    remove(&path);
}

#[test]
fn a_reading_follows_the_frame_it_measured() {
    let path = scratch("picker-frames");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.reading([on(true)]);
    // Each frame is measured as it renders, its reading picked up after it, never
    // waited for: before anything looks at the GPU, it has not arrived.
    for (i, value) in [0.2, 0.4, 0.6].into_iter().enumerate() {
        let events = driven.run([set("brightness", ControlValue::Number(value), i == 2)]);
        let through = frame_through(&events);
        assert!(readings(&events).is_empty(), "{events:?}");
        assert!(
            driven
                .engine
                .pickers
                .reducer
                .as_mut()
                .is_some_and(|reducer| reducer.awaits(|sampled| sampled.through == through))
        );
        let (pixels_, _) = frame_pixels(&driven.gpu, &events);
        let picked = driven.picked();
        let [Event::Picker(Some(reading))] = &picked[..] else {
            panic!("{picked:?}");
        };
        assert_eq!(reading.through, through);
        let want = expected(&pixels_, [40, 30], reading.region);
        assert!(agrees(reading.frame, want), "{reading:?}");
    }
    remove(&path);

    // Through a session's thread, with no command after its frame: the frame, then its
    // reading.
    let path = scratch("picker-session");
    let gpu = gpu();
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 40,
        height: 40,
    });
    open_live(&session, &wakes, &path);
    session.send(on(true));
    let sent = session.send(set("brightness", ControlValue::Number(0.3), true));
    let measured = |e: &Event| matches!(e, Event::Picker(Some(r)) if r.through >= sent);
    let events = until(&session, &wakes, measured);
    let frame = events
        .iter()
        .position(|e| matches!(e, Event::Frame(frame) if frame.through == sent));
    let reading = events.iter().position(measured);
    assert!(
        matches!((frame, reading), (Some(frame), Some(reading)) if frame < reading),
        "{events:?}"
    );
    drop(session);
    remove(&path);
}

#[test]
fn on_is_refused_before_the_photo_is_live_and_a_move_with_no_picker_on() {
    let path = scratch("picker-refused");
    let mut driven = Driven::new(&[&Sample]);
    let refused = |events: &[Event]| match &failures(events)[..] {
        [error] => format!("{error:?}"),
        _ => panic!("{events:?}"),
    };
    // No photo, then one still opening.
    assert_eq!(refused(&driven.run([on(false)])), "NotLive");
    driven.run([
        Command::Open(path.clone()),
        Command::Resize {
            width: 40,
            height: 40,
        },
    ]);
    assert_eq!(refused(&driven.run([on(false)])), "NotLive");
    driven.settle();
    // A picker no module declares, and one of a module the session lacks.
    assert_eq!(
        refused(&driven.run([on_module("contrast", true)])),
        "UnknownPicker"
    );
    assert_eq!(
        refused(&driven.run([on_picker("exposure", "exposure", true)])),
        "UnknownPicker"
    );
    // Tool settings no picker of the module reads.
    let tools = vec![("target", ControlValue::Number(50.0))];
    let mut module = on_module("brightness", true);
    if let Command::Picker(PickerCommand::On { tools: asked, .. }) = &mut module {
        *asked = tools.clone();
    }
    assert_eq!(refused(&driven.run([module])), "UnknownTool(\"target\")");
    let settings = picker(PickerCommand::Tools {
        module: "sample",
        tools,
    });
    assert_eq!(refused(&driven.run([settings])), "UnknownTool(\"target\")");
    // A move, a switch or a reset with no picker on; an `Off` changes nothing.
    for command in [
        point([0.5, 0.5]),
        area([0.1, 0.1], [0.2, 0.2]),
        picker(PickerCommand::Switch),
        picker(PickerCommand::Reset),
    ] {
        assert_eq!(refused(&driven.run([command])), "NoPicker");
    }
    assert!(driven.run([picker(PickerCommand::Off)]).is_empty());
    assert!(driven.picked().is_empty());
    remove(&path);

    // On a photo open for viewing only, the Colour picker's picker measures.
    let path = scratch("picker-view-only");
    let newer = brightened("0.9.0").replace(r#"apertrust:version="1""#, r#"apertrust:version="2""#);
    std::fs::write(sidecar::path_for(&path), newer).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    assert!(matches!(opened.first(), Some(Event::Opened(photo)) if photo.view_only.is_some()));
    assert!(driven.reading([on(false)]).frame.is_some());
    // A module's picker writes the edit: refused there, the Colour picker's left on.
    let events = driven.run([on_module("brightness", true)]);
    assert!(
        matches!(failures(&events)[..], [SessionError::ViewOnly(_)]),
        "{events:?}"
    );
    assert!(driven.engine.pickers.on());
    remove(&path);
}

#[test]
fn the_picker_turns_off_as_another_photo_opens_and_its_readings_never_arrive() {
    let path = scratch("picker-photos");
    let other = scratch("picker-photos-other");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.reading([on(true), area([0.1, 0.1], [0.5, 0.5])]);
    // A frame measured, its reading on its way as another photo opens: off at once.
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let events = driven.run([
        Command::Open(other.clone()),
        Command::Resize {
            width: 40,
            height: 40,
        },
    ]);
    assert!(
        matches!(events.first(), Some(Event::Picker(None))),
        "{events:?}"
    );
    driven.settle();
    assert!(driven.picked().is_empty());
    // Its place is kept for the session, on the new photo too.
    let reading = driven.reading([on(true)]);
    assert!(same(reading.region, Region::Area([0.1, 0.1, 0.5, 0.5])));
    // And off again as the photo closes.
    let events = driven.run([Command::Close]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    remove(&path);
    remove(&other);
}

/// darktable's flip bits for each of the Orientation module's choices: the rows flipped,
/// the columns flipped, the axes swapped. None comes last, from transverse, so each choice
/// changes the photo.
const ORIENTATIONS: [(u32, u8); 8] = [
    (2, 1),
    (3, 2),
    (4, 3),
    (5, 4),
    (6, 5),
    (7, 6),
    (8, 7),
    (1, 0),
];

/// `region`, an area on the photo as it develops, as the orientation `bits` shows it.
fn oriented([left, top, right, bottom]: [f32; 4], bits: u8) -> Region {
    let [x0, x1] = if bits & 2 != 0 {
        [1.0 - right, 1.0 - left]
    } else {
        [left, right]
    };
    let [y0, y1] = if bits & 1 != 0 {
        [1.0 - bottom, 1.0 - top]
    } else {
        [top, bottom]
    };
    Region::Area(if bits & 4 != 0 {
        [y0, x0, y1, x1]
    } else {
        [x0, y0, x1, y1]
    })
}

#[test]
fn an_area_stays_on_its_part_of_the_scene_through_every_orientation() {
    let path = scratch("picker-orientation");
    let mut driven = Driven::new(&[&Sample, &Orientation]);
    driven.open(&path);
    let orient = |choice: u32| Command::SetControl {
        module: "flip".to_owned(),
        control: "orientation".to_owned(),
        value: ControlValue::Choice(choice),
        done: true,
    };
    let scene = [0.1, 0.2, 0.4, 0.7];
    driven.run([orient(1)]);
    driven.reading([on(true), area([0.1, 0.2], [0.4, 0.7])]);
    for (choice, bits) in ORIENTATIONS {
        // Each turn renders a frame, measured under the area where it now lies.
        let events = driven.run([orient(choice)]);
        let (pixels_, through) = frame_pixels(&driven.gpu, &events);
        let Some(Event::Frame(frame)) = events.iter().find(|e| matches!(e, Event::Frame(_))) else {
            unreachable!()
        };
        let size = [frame.width, frame.height];
        let picked = driven.picked();
        let reading = match &picked[..] {
            [Event::Picker(Some(reading))] => reading,
            _ => panic!("{picked:?}"),
        };
        assert_eq!(reading.through, through);
        assert!(
            same(reading.region, oriented(scene, bits)),
            "{choice}: {reading:?}"
        );
        let want = expected(&pixels_, size, reading.region);
        assert!(
            agrees(reading.frame, want),
            "{choice}: {reading:?}, {want:?}"
        );
    }
    // A point set on the turned photo maps back, so it lies at the same place of the
    // scene when the photo is upright again: turned clockwise, the scene's (x, y) shows
    // at (1 − y, x).
    driven.run([orient(6)]);
    driven.picked();
    let reading = driven.reading([point([0.75, 0.25])]);
    assert!(
        same(reading.region, Region::Point([0.75, 0.25])),
        "{reading:?}"
    );
    driven.run([orient(1)]);
    let picked = driven.picked();
    let [Event::Picker(Some(reading))] = &picked[..] else {
        panic!("{picked:?}");
    };
    assert!(
        same(reading.region, Region::Point([0.25, 0.25])),
        "{reading:?}"
    );
    remove(&path);
}

#[test]
fn a_failure_of_the_pickers_is_reported_once_and_the_frames_go_on() {
    let path = scratch("picker-failed");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.reading([on(false)]);
    // A reduction the GPU refuses: a frame larger than its buffer.
    let small = driven.gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let mut events = Vec::new();
    driven.engine.sample(&small, [40, 30], &mut events);
    assert!(
        matches!(
            &events[..],
            [Event::Failed(SessionError::Pickers(_)), Event::Picker(None)]
        ),
        "{events:?}"
    );
    // The frames go on, with nothing measured or reported; `On` is refused from then on.
    let events = driven.run([set("brightness", ControlValue::Number(0.4), true)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    frame_pixels(&driven.gpu, &events);
    assert!(driven.picked().is_empty());
    let events = driven.run([on(false)]);
    assert!(
        matches!(failures(&events)[..], [SessionError::Pickers(_)]),
        "{events:?}"
    );
    remove(&path);
}

/// The module parts of the pickers' readings in `events`.
/// The module parts of the pickers' readings in `events`, each once: a reading whose frame
/// part alone changed carries the module part the one before it did.
fn module_parts(events: &[Event]) -> Vec<&ModuleReading> {
    let mut parts: Vec<&ModuleReading> = readings(events)
        .into_iter()
        .filter_map(|reading| reading.as_ref()?.module.as_ref())
        .collect();
    parts.dedup();
    parts
}

/// The module part last sent in `events`.
fn latest(events: &[Event]) -> &ModuleReading {
    let parts = module_parts(events);
    parts
        .last()
        .copied()
        .unwrap_or_else(|| panic!("a module part in {events:?}"))
}

/// The mean of the first three channels, [`Sample`]'s stand-in for luminance.
fn grey(stats: &Stats) -> f32 {
    (stats.mean[0] + stats.mean[1] + stats.mean[2]) / 3.0
}

fn number(values: &Values, module: &str, id: &str) -> f32 {
    match control(values, module, id).value {
        ControlValue::Number(number) => number,
        ref value => panic!("{value:?}"),
    }
}

fn moved(from: [f32; 2], to: [f32; 2], done: bool) -> Command {
    picker(PickerCommand::Area {
        from,
        to,
        done,
        keys: PickKeys::default(),
    })
}

/// The bits of a module reading's statistics, compared exactly.
fn bits(stats: &Stats) -> [[u32; 4]; 3] {
    [stats.mean, stats.min, stats.max].map(|v| v.map(f32::to_bits))
}

#[test]
fn a_module_picker_applies_its_first_reading_and_each_move_never_an_edit_upstream() {
    let path = scratch("picker-module");
    let mut driven = Driven::new(&[&Orientation, &Sample]);
    driven.open(&path);
    driven.writes();
    // On, with no edit: no frame, a sample-only render at exposure.
    let events = driven.run([on_module("brightness", true)]);
    assert!(
        failures(&events).is_empty() && !events.iter().any(|e| matches!(e, Event::Frame(_))),
        "{events:?}"
    );
    let picked = driven.picked();
    // Its reading, then the change its pick makes and the frame; then the reading that
    // frame measured at exposure, sent and never applied. The frame's own parts come
    // between them as they arrive.
    let parts = module_parts(&picked);
    let [part, again] = parts[..] else {
        panic!("{picked:?}");
    };
    let at = |which: &dyn Fn(&Event) -> bool| {
        (picked.iter().position(which)).unwrap_or_else(|| panic!("{picked:?}"))
    };
    let carries = |e: &Event, part: &ModuleReading| matches!(e, Event::Picker(Some(r)) if r.module.as_ref() == Some(part));
    let order = [
        at(&|e| carries(e, part)),
        at(&|e| matches!(e, Event::Values(_))),
        at(&|e| matches!(e, Event::Frame(_))),
        at(&|e| carries(e, again)),
    ];
    assert!(order.is_sorted(), "{picked:?}");
    let changes = picked.iter().filter(|e| matches!(e, Event::Values(_)));
    assert_eq!(changes.count(), 1, "{picked:?}");
    let applied = values(&picked);
    let first = readings(&picked)[0].as_ref().expect("a reading");
    let brightness_picker = PickerRef::Module {
        module: "sample",
        picker: "brightness",
    };
    assert_eq!(first.picker, brightness_picker);
    assert!(same(first.region, DEFAULT_AREA), "{first:?}");
    let output = part.output.expect("its output read");
    assert!(part.input.is_some());
    assert!(
        part.picked
            .as_ref()
            .is_some_and(|shown| shown.patch.is_some())
    );
    let brightness = number(applied, "sample", "brightness");
    let expected = (0.18 / grey(&output)).log2();
    assert!(
        (brightness - expected).abs() < 1e-5,
        "{brightness} against {expected}"
    );
    // Exposure's output under the area is middle grey now; the panel still shows the pick.
    assert!(
        (grey(&again.output.unwrap()) / 0.18 - 1.0).abs() < 1e-3,
        "{again:?}"
    );
    assert_eq!(again.input, part.input);
    assert_eq!(again.picked, part.picked);
    // One completed change, saved.
    assert_eq!(written(&driven.writes()).len(), 1);

    // A move: the reading of the new area applied the same way.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    let picked = driven.picked();
    let moved_to = number(values(&picked), "sample", "brightness");
    let part = (module_parts(&picked).into_iter())
        .find(|part| {
            part.output
                .is_some_and(|output| (grey(&output) / 0.18 - 1.0).abs() > 1e-3)
        })
        .expect("the moved area's reading");
    let expected = brightness + (0.18 / grey(&part.output.unwrap())).log2();
    assert!(
        (moved_to - expected).abs() < 1e-5,
        "{moved_to} against {expected}"
    );

    // A turn upstream moves what exposure reads under the area: measured and sent, never
    // applied, so the pick never feeds on its own write.
    let events = driven.run([Command::SetControl {
        module: "flip".to_owned(),
        control: "orientation".to_owned(),
        value: ControlValue::Choice(6),
        done: true,
    }]);
    assert!(driven.engine.pickers.on());
    let picked = driven.picked();
    assert!(
        !picked.is_empty() && picked.iter().all(|e| matches!(e, Event::Picker(Some(_)))),
        "{events:?} then {picked:?}"
    );
    assert_ne!(latest(&picked).output, part.output);
    assert_eq!(number(values(&events), "sample", "brightness"), moved_to);
    remove(&path);
}

#[test]
fn a_sample_only_reading_is_the_one_a_render_measures() {
    let path = scratch("picker-sample-only");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    // `show` reads exposure's input and output and writes nothing: its first reading,
    // from a sample-only render.
    driven.run([on_module("show", false)]);
    let picked = driven.picked();
    let first = latest(&picked);
    let (input, output) = (first.input.unwrap(), first.output.unwrap());
    // A change to the module, which keeps `show` on: the render measures at exposure.
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert!(driven.engine.pickers.on());
    let picked = driven.picked();
    let brighter = latest(&picked);
    assert_eq!(bits(&brighter.input.unwrap()), bits(&input));
    assert_ne!(bits(&brighter.output.unwrap()), bits(&output));
    // Back where it was: the render's reading is the sample-only one, bit for bit.
    driven.run([set("brightness", ControlValue::Number(0.0), true)]);
    let picked = driven.picked();
    let back = latest(&picked);
    assert_eq!(bits(&back.output.unwrap()), bits(&output));
    // Moves away and back, each a sample-only render from the edit cache, give the
    // render's values for the same box.
    driven.run([point([0.3, 0.6])]);
    driven.picked();
    driven.run([point([0.5, 0.5])]);
    let picked = driven.picked();
    let moved_back = latest(&picked);
    assert_eq!(bits(&moved_back.input.unwrap()), bits(&input));
    assert_eq!(bits(&moved_back.output.unwrap()), bits(&output));
    remove(&path);
}

#[test]
fn a_drag_is_one_change_completed_by_its_release_or_by_turning_off() {
    let path = scratch("picker-drag");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.writes();
    // A drag's steps are applied, the edit not saved until the release.
    for to in [[0.3, 0.3], [0.4, 0.5]] {
        driven.run([moved([0.1, 0.1], to, false)]);
        let picked = driven.picked();
        assert!(
            picked.iter().any(|e| matches!(e, Event::Values(_))),
            "{picked:?}"
        );
        assert!(driven.writes().is_empty());
    }
    // A release where the drag left the area: nothing more to read, the change complete.
    let events = driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    assert!(driven.picked().is_empty(), "{events:?}");
    assert_eq!(written(&driven.writes()).len(), 1);
    // A step, then the picker off: the change it began is complete.
    driven.run([moved([0.2, 0.2], [0.6, 0.6], false)]);
    driven.picked();
    assert!(driven.writes().is_empty());
    let events = driven.run([picker(PickerCommand::Off)]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    assert_eq!(written(&driven.writes()).len(), 1);
    remove(&path);
}

#[test]
fn a_module_picker_turns_its_module_on_and_off_as_the_module_changes_unless_it_keeps() {
    let path = scratch("picker-switched");
    let mut driven = Driven::new(&[&Sample, &ColourBalanceRgb]);
    driven.open(&path);
    driven.writes();
    let enabled = |values: &Values| {
        (values.modules.iter())
            .find(|m| m.module == "colorbalancergb")
            .is_some_and(|m| m.enabled)
    };
    // Colour balance RGB, switched and off: its picker turns it on, a change of its own.
    let events = driven.run([on_picker("colorbalancergb", "global_hue", true)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert!(enabled(values(&events)));
    assert!(events.iter().any(|e| matches!(e, Event::Frame(_))));
    assert_eq!(written(&driven.writes()).len(), 1);
    // Its operation's input read; its pick, none yet, writes nothing.
    let picked = driven.picked();
    let parts = module_parts(&picked);
    assert!(
        matches!(parts[..], [part] if part.input.is_some() && part.output.is_none()),
        "{picked:?}"
    );
    assert!(!picked.iter().any(|e| matches!(e, Event::Values(_))));
    // A change to its module turns it off, as darktable's `gui_changed` does.
    let events = driven.run([Command::ResetModule {
        module: "colorbalancergb".to_owned(),
    }]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    // A change to another module leaves it on; Sample's `show`, darktable's keep-active,
    // stays on through its own module's.
    driven.run([on_module("show", true)]);
    driven.picked();
    let events = driven.run([
        Command::ResetModule {
            module: "colorbalancergb".to_owned(),
        },
        set("grain", ControlValue::Number(20.0), true),
    ]);
    assert!(
        readings(&events).is_empty() && driven.engine.pickers.on(),
        "{events:?}"
    );
    // `spread` does not keep: off as its module changes.
    driven.run([on_module("spread", true)]);
    driven.picked();
    let events = driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    remove(&path);
}

#[test]
fn a_pick_reads_the_channel_and_keys_shown_as_it_reads() {
    let path = scratch("picker-channel");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let grain = |events: &[Event]| number(values(events), "sample", "grain");
    let spread = |part: &ModuleReading, c: usize| {
        let input = part.input.unwrap();
        100.0 * (input.max[c] - input.min[c])
    };
    // `spread` with the red channel shown: red's spread, denoised, in percent.
    let mut on = on_module("spread", true);
    if let Command::Picker(PickerCommand::On { channel, .. }) = &mut on {
        *channel = Some("red");
    }
    driven.run([on]);
    let picked = driven.picked();
    let part = latest(&picked);
    assert_eq!(grain(&picked), spread(part, 0).clamp(0.0, 100.0));
    // Another channel shown changes nothing, until the next reading applied.
    let events = driven.run([picker(PickerCommand::Channel {
        channel: Some("blue"),
    })]);
    assert!(
        events.is_empty() && driven.picked().is_empty(),
        "{events:?}"
    );
    let keys = PickKeys {
        command: false,
        shift: true,
    };
    driven.run([picker(PickerCommand::Area {
        from: [0.2, 0.2],
        to: [0.7, 0.7],
        done: true,
        keys,
    })]);
    let picked = driven.picked();
    let part = latest(&picked);
    assert_eq!(grain(&picked), spread(part, 2).clamp(0.0, 100.0));
    // With Shift held, coarse grain.
    assert_eq!(
        control(values(&picked), "sample", "size").value,
        ControlValue::Choice(2)
    );
    // No tool settings: kept, and nothing applied, since no picker of Sample reads one.
    let events = driven.run([picker(PickerCommand::Tools {
        module: "sample",
        tools: Vec::new(),
    })]);
    assert!(events.is_empty(), "{events:?}");
    remove(&path);
}

/// A module with one area picker on temperature's input, the mosaic, as White balance's
/// `spot` reads it, which writes nothing: its one control is temperature's red
/// coefficient, which it owns and binds to nothing.
struct Spot;

static SPOT: Definition = Definition {
    id: "spot",
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &[Control {
        id: "red",
        kind: ControlKind::Number {
            min: 0.0,
            max: 8.0,
            step: 0.01,
            default: 1.0,
        },
        unit: None,
        unported: false,
    }],
    tabs: &[Tab {
        id: None,
        rows: &[Row::Slider(Slider::new("red").quad(Quad::Picker("red")))],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
    pickers: &[Picker {
        id: "red",
        operation: "temperature",
        kind: PickerKind::Area,
        denoise: false,
        output: false,
        space: Space::Operation,
        keep: false,
        writes: &[],
        tools: &[],
    }],
};

static SPOT_OWNS: [Owned; 1] = [Owned::new("temperature", "red")];

impl Module for Spot {
    fn definition(&self) -> &'static Definition {
        &SPOT
    }

    fn owns(&self) -> &'static [Owned] {
        &SPOT_OWNS
    }

    fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        Vec::new()
    }
}

#[test]
fn a_picker_in_the_raw_stage_reads_the_mosaic_per_cfa_colour() {
    let path = scratch("picker-mosaic");
    let mut driven = Driven::new(&[&Spot]);
    driven.open(&path);
    driven.run([on_picker("spot", "red", true)]);
    let picked = driven.picked();
    let input = latest(&picked).input.expect("its input read");
    // darktable's box of the default area on the 84 × 62 developed image, [1, 82) by
    // [1, 60), lies on the sensor from the declared area's corner, (6, 4): the
    // photosites [7, 88) by [5, 64) of the synthetic mosaic, prepared as value / 4000.
    let mosaic = testing::mosaic();
    let (mut sum, mut count) = ([0.0f64; 4], [0u32; 4]);
    let (mut low, mut high) = ([f32::MAX; 4], [-f32::MAX; 4]);
    for y in 5..64u32 {
        for x in 7..88u32 {
            let c = [0, 1, 1, 2][((y % 2) * 2 + x % 2) as usize];
            let v = f32::from(mosaic[(y * 96 + x) as usize]) / 4000.0;
            sum[c] += f64::from(v);
            count[c] += 1;
            low[c] = low[c].min(v);
            high[c] = high[c].max(v);
        }
    }
    for c in 0..3 {
        let mean = sum[c] / f64::from(count[c]);
        assert!(
            (f64::from(input.mean[c]) - mean).abs() < 1e-5,
            "{c}: {input:?}"
        );
        assert!((input.min[c] - low[c]).abs() < 1e-6, "{c}: {input:?}");
        assert!((input.max[c] - high[c]).abs() < 1e-6, "{c}: {input:?}");
    }
    // No photosite of a fourth colour: darktable's empty channel.
    assert_eq!(
        (input.mean[3], input.min[3], input.max[3]),
        (0.0, f32::MAX, -f32::MAX)
    );
    // A move reruns rawprepare, with no frame, and reads the new box.
    let events = driven.run([moved([0.5, 0.5], [0.6, 0.6], true)]);
    assert!(
        !events.iter().any(|e| matches!(e, Event::Frame(_))),
        "{events:?}"
    );
    let picked = driven.picked();
    let moved = latest(&picked).input.expect("its input read");
    assert!(moved.mean[0] > input.mean[0], "{picked:?}");
    remove(&path);
}

/// The moved area's budgets (`docs/pickers.md`, Cost): with the 7R V's 61 MP raw open at
/// 2400×1600, each of 50 moves of an area, after 10 to warm up, from its command to its
/// reading at the operation picked up as the thread picks it up, polling as it does:
/// Exposure's picker after the downscale, its operation the edited instance, and a picker
/// on temperature's input in the raw stage, as White balance's reads it.
#[test]
#[ignore = "a measurement with the raw.pixls.us corpus: cargo test --release -p \
            apertrust-engine --lib -- moved_area --ignored --nocapture"]
fn a_moved_area_s_reading_at_its_operation_on_the_7r_v() {
    use apertrust_modules::exposure::Exposure;

    let corpus = std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    );
    let raw = corpus.join("Sony/ILCE-7RM5/7RM5-LosslessCompressedLarge.ARW");
    let path = scratch("picker-budget").with_extension("ARW");
    std::fs::hard_link(&raw, &path)
        .or_else(|_| std::fs::copy(&raw, &path).map(|_| ()))
        .expect("the corpus raw");
    let cases: [(&str, &'static [&'static dyn Module], &str, &str); 2] = [
        (
            "exposure, after the downscale",
            &[&Exposure],
            "exposure",
            "exposure",
        ),
        ("temperature, in the raw stage", &[&Spot], "spot", "red"),
    ];
    for (name, modules, module, id) in cases {
        let mut driven = Driven::new(modules);
        driven.engine.load = open::files();
        let mut opened = driven.run([
            Command::Open(path.clone()),
            Command::Resize {
                width: 2400,
                height: 1600,
            },
        ]);
        opened.extend(driven.settle());
        let photo = opened.iter().find_map(|e| match e {
            Event::Opened(photo) => Some((photo.width, photo.height)),
            _ => None,
        });
        assert_eq!(photo, Some((9504, 6336)), "{opened:?}");
        driven.run([on_picker(module, id, true)]);
        driven.picked();
        let mut latencies = Vec::new();
        for run in 0..60u32 {
            // Never the default area the picker started at, which would move nothing.
            let at = 0.021 + 0.001 * (run % 30) as f32;
            let start = Instant::now();
            driven.run([moved([at, at], [1.0 - at, 1.0 - at], true)]);
            let read = loop {
                let events = driven.engine.picked();
                if !module_parts(&events).is_empty() {
                    break start.elapsed();
                }
                std::thread::sleep(POLL_INTERVAL);
            };
            if run >= 10 {
                latencies.push(read.as_secs_f64() * 1e3);
            }
        }
        latencies.sort_by(f64::total_cmp);
        println!(
            "{name}: a moved area's reading {:.2} ms after its command (median of {}; min {:.2}, max {:.2})",
            latencies[latencies.len() / 2],
            latencies.len(),
            latencies[0],
            latencies[latencies.len() - 1],
        );
    }
    remove(&path);
}

#[test]
fn a_release_while_a_drag_s_reading_is_on_its_way_completes_with_that_reading() {
    let path = scratch("picker-release-in-flight");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.writes();
    // A step, its reading still on its way as the release comes where it left the area:
    // nothing saved yet, and the reading, once applied, completes the change.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    assert!(driven.writes().is_empty());
    let picked = driven.picked();
    assert!(
        picked.iter().any(|e| matches!(e, Event::Values(_))),
        "{picked:?}"
    );
    assert_eq!(written(&driven.writes()).len(), 1);
    // The next drag's first step is a step: nothing saved.
    driven.run([moved([0.2, 0.2], [0.6, 0.6], false)]);
    let picked = driven.picked();
    assert!(
        picked.iter().any(|e| matches!(e, Event::Values(_))),
        "{picked:?}"
    );
    assert!(driven.writes().is_empty());
    // Its release where it left the area saves it.
    driven.run([moved([0.2, 0.2], [0.6, 0.6], true)]);
    assert_eq!(written(&driven.writes()).len(), 1);

    // Two steps' readings on their way as the release comes: the newer completes the
    // change, after both are applied, and nothing is left to save.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.run([moved([0.1, 0.1], [0.7, 0.8], false)]);
    driven.run([moved([0.1, 0.1], [0.7, 0.8], true)]);
    assert!(driven.writes().is_empty());
    let picked = driven.picked();
    let changes = picked.iter().filter(|e| matches!(e, Event::Values(_)));
    assert_eq!(changes.count(), 2, "{picked:?}");
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(!driven.engine.needs_save());
    // Releasing again where the area is reads nothing, and writes nothing.
    let events = driven.run([moved([0.1, 0.1], [0.7, 0.8], true)]);
    assert!(driven.picked().is_empty(), "{events:?}");
    assert!(driven.writes().is_empty());
    remove(&path);

    // A release while only a reading an edit caused is on its way, never applied:
    // complete at once, and the next drag's first step still a step.
    let path = scratch("picker-release-edit-reading");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.picked();
    driven.writes();
    // A larger viewport: exposure's buffer, and so the box, change, and the render
    // measures again, applying nothing.
    driven.run([Command::Resize {
        width: 60,
        height: 60,
    }]);
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    assert_eq!(written(&driven.writes()).len(), 1);
    let picked = driven.picked();
    assert!(
        !picked.iter().any(|e| matches!(e, Event::Values(_))),
        "{picked:?}"
    );
    driven.writes();
    driven.run([moved([0.3, 0.3], [0.7, 0.7], false)]);
    let picked = driven.picked();
    assert!(
        picked.iter().any(|e| matches!(e, Event::Values(_))),
        "{picked:?}"
    );
    assert!(driven.writes().is_empty());
    remove(&path);
}

#[test]
fn a_picker_on_an_operation_in_the_viewport_s_tiles_reads_nothing_and_applies_nothing() {
    let path = scratch("picker-tiled");
    let budget = 64 << 10;
    let mut driven = Driven::within(&[&Spot], budget);
    // Open with no viewport yet, so nothing renders and the reading `On` asks for is
    // still to be taken.
    driven.run([Command::Open(path.clone())]);
    driven.settle();
    let events = driven.run([on_picker("spot", "red", true)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    // A viewport in tiles with no highlight reconstruction, whose whole-image passes
    // would keep the raw stage whole: temperature runs once a tile.
    let gpu = driven.gpu.clone();
    let open = driven.engine.open.as_ref().expect("a photo open");
    let bound = edit::instances(
        &open.edit,
        driven.engine.operations,
        &driven.engine.modules,
        &open.image,
    );
    let instances: Vec<Instance> = (bound.instances.into_iter())
        .filter(|instance| instance.operation.manifest().id != "highlights")
        .collect();
    let limits = Limits {
        budget,
        ..Limits::of(&gpu)
    };
    let pipe = Pipe::Viewport {
        width: 40,
        height: 40,
    };
    let plan = pipeline::plan(&instances, &open.image, open.sensor, pipe, &limits).expect("a plan");
    let temperature = (plan.operations.iter())
        .position(|op| op.id == "temperature")
        .expect("temperature");
    assert!(
        plan.tiled
            .as_ref()
            .is_some_and(|tiles| tiles.contains(&temperature)),
        "{:?}",
        plan.tiled
    );
    let applied = driven.engine.applied;
    let probing = crate::session::pickers::probing(
        &mut driven.engine.pickers,
        &plan,
        &instances,
        &open.image,
        applied,
    )
    .expect("a reading due");
    assert!(probing.missed());
    let mut events = Vec::new();
    driven.engine.probed(Some(probing), None, &mut events);
    assert!(events.is_empty(), "{events:?}");
    // Its reading has no module part, and nothing is applied.
    let picked = driven.picked();
    assert!(
        matches!(&picked[..], [Event::Picker(Some(reading))] if reading.module.is_none()),
        "{picked:?}"
    );
    remove(&path);
}

#[test]
fn a_release_whose_reading_misses_completes_the_drag_s_change() {
    let path = scratch("picker-release-missed");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.writes();
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.picked();
    assert!(driven.writes().is_empty());
    // Released on the photo's right edge: an area no pixel holds, so nothing to pick, and
    // the change the step wrote is complete.
    driven.run([moved([1.0, 0.1], [1.0, 0.5], true)]);
    let picked = driven.picked();
    assert!(
        !picked.iter().any(|e| matches!(e, Event::Values(_))),
        "{picked:?}"
    );
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(!driven.engine.needs_save());
    remove(&path);
}

/// A module with one area picker on exposure's input, which reads the tool setting `gain`:
/// its pick sets `level` to the gain times the mean's first channel, and with no gain
/// writes `other`, which its picker does not write, as a module's bug would; it shows a
/// patch of the gain. It owns
/// exposure's exposure and binds nothing.
struct Tooled;

static TOOLED: Definition = Definition {
    id: "tooled",
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &[
        Control {
            id: "level",
            kind: ControlKind::Number {
                min: 0.0,
                max: 10.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "other",
            kind: ControlKind::Number {
                min: 0.0,
                max: 10.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        },
    ],
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Slider(Slider::new("level").quad(Quad::Picker("level"))),
            Row::Slider(Slider::new("other")),
            Row::ToolSetting(apertrust_modules::ToolSetting {
                id: "gain",
                kind: ControlKind::Number {
                    min: 0.0,
                    max: 4.0,
                    step: 0.1,
                    default: 1.0,
                },
                decimals: 1,
            }),
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
    pickers: &[Picker {
        id: "level",
        operation: "exposure",
        kind: PickerKind::Area,
        denoise: false,
        output: false,
        space: Space::Operation,
        keep: false,
        writes: &["level"],
        tools: &["gain"],
    }],
};

static TOOLED_OWNS: [Owned; 1] = [Owned::new("exposure", "exposure")];

impl Module for Tooled {
    fn definition(&self) -> &'static Definition {
        &TOOLED
    }

    fn owns(&self) -> &'static [Owned] {
        &TOOLED_OWNS
    }

    fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        Vec::new()
    }

    fn pick(
        &self,
        _: &str,
        reading: &apertrust_modules::pick::Reading,
        _: &Controls,
        _: &ImageContext,
        tools: &[(&str, ControlValue)],
        _: Option<&str>,
        _: Option<&apertrust_modules::pick::Reading>,
        _: PickKeys,
    ) -> Option<apertrust_modules::pick::Picked> {
        let gain = tools
            .iter()
            .find_map(|&(tool, ref value)| match (tool, value) {
                ("gain", &ControlValue::Number(gain)) => Some(gain),
                _ => None,
            })
            .unwrap_or(1.0);
        let mut writes = Controls::new();
        let control = if gain == 0.0 { "other" } else { "level" };
        writes.set(control, ControlValue::Number(gain * reading.input.mean[0]));
        // A patch of the gain, so each gain shows something of its own.
        let patch = apertrust_modules::pick::Patch {
            srgb: [gain / 4.0; 3],
            lch: [25.0 * gain, 0.0, 0.0],
        };
        Some(apertrust_modules::pick::Picked {
            writes,
            shown: apertrust_modules::pick::Shown {
                patch: Some(patch),
                tools: Vec::new(),
            },
        })
    }
}

fn gain(gain: f32) -> Command {
    picker(PickerCommand::Tools {
        module: "tooled",
        tools: vec![("gain", ControlValue::Number(gain))],
    })
}

#[test]
fn tools_apply_the_latest_reading_measured_and_a_refused_pick_turns_the_picker_off() {
    let path = scratch("picker-tools");
    let mut driven = Driven::new(&[&Tooled]);
    driven.open(&path);
    driven.run([on_picker("tooled", "level", true)]);
    driven.picked();
    let (last, latest) = driven.engine.pickers.readings_of("tooled");
    assert_eq!(last, latest);
    // A larger viewport: exposure's input measured again, never applied, so the latest
    // reading is no longer the last applied.
    driven.run([Command::Resize {
        width: 64,
        height: 64,
    }]);
    driven.picked();
    let (last, latest) = driven.engine.pickers.readings_of("tooled");
    let (last, latest) = (last.unwrap(), latest.unwrap());
    assert_ne!(last.input.mean[0], latest.input.mean[0]);
    // A new gain applies the latest, darktable's `picked_color`, and sends the reading.
    let events = driven.run([gain(2.0)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(
        number(values(&events), "tooled", "level"),
        (2.0 * latest.input.mean[0]).clamp(0.0, 10.0)
    );
    assert!(matches!(readings(&events)[..], [Some(_)]), "{events:?}");
    assert_eq!(driven.engine.pickers.readings_of("tooled").0, Some(latest));
    // A pick its module gets wrong is refused, the picker off, the last reading kept.
    driven.run([Command::Resize {
        width: 50,
        height: 50,
    }]);
    driven.picked();
    let events = driven.run([gain(0.0)]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::Edit(EditError::PickFailed { .. })]
        ),
        "{events:?}"
    );
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    assert!(!driven.engine.pickers.on());
    assert_eq!(driven.engine.pickers.readings_of("tooled").0, Some(latest));
    remove(&path);
}

#[test]
fn a_waiting_reading_replaced_passes_its_application_on() {
    let path = scratch("picker-keep-due");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    // Three steps and nothing polled: two staging buffers mapping, the third waiting.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.run([moved([0.1, 0.1], [0.5, 0.6], false)]);
    driven.run([moved([0.1, 0.1], [0.6, 0.7], false)]);
    let (drag, _) = driven.engine.pickers.drags();
    let third = (driven.engine.applied, drag);
    assert_eq!(driven.engine.pickers.waiting_due(), Some(Some(third)));
    // A render's reading, applying nothing, replaces it: the step's application with it.
    driven.run([Command::Resize {
        width: 64,
        height: 64,
    }]);
    assert_eq!(driven.engine.pickers.waiting_due(), Some(Some(third)));
    let picked = driven.picked();
    let changes = picked.iter().filter(|e| matches!(e, Event::Values(_)));
    assert_eq!(changes.count(), 3, "{picked:?}");
    remove(&path);
}

#[test]
fn a_gpu_error_retries_without_the_reduction_only_where_one_was_in_the_render() {
    let path = scratch("picker-retry");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("show", true)]);
    driven.picked();
    // A render with no reduction at exposure, grain after it changing: the error is the
    // frame's, and the picker stays on.
    driven.engine.poison = Some(Poison::Always);
    let events = driven.run([set("grain", ControlValue::Number(20.0), true)]);
    assert!(
        matches!(failures(&events)[..], [SessionError::Gpu(_)]),
        "{events:?}"
    );
    assert!(driven.engine.pickers.on());
    // A render with one, failing only with it: again without it, the frame shown and the
    // pickers off.
    driven.engine.poison = Some(Poison::WithReduction);
    let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert!(
        matches!(failures(&events)[..], [SessionError::Pickers(_)]),
        "{events:?}"
    );
    assert!(events.iter().any(|e| matches!(e, Event::Frame(_))));
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    assert!(!driven.engine.pickers.on());
    remove(&path);

    // One that fails with and without the reduction: the frame's error, the picker on.
    let path = scratch("picker-retry-both");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("show", true)]);
    driven.picked();
    driven.engine.poison = Some(Poison::Always);
    let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert!(
        matches!(failures(&events)[..], [SessionError::Gpu(_)]),
        "{events:?}"
    );
    assert!(driven.engine.pickers.on());
    remove(&path);
}

#[test]
fn a_release_that_misses_waits_for_its_drag_s_step_on_its_way() {
    let path = scratch("picker-missed-in-flight");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.writes();
    // A step, its reading still on its way, then a release on the photo's right edge,
    // which reads nothing: the step's pick is applied first, and the drag saved once.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.run([moved([1.0, 0.1], [1.0, 0.5], true)]);
    let picked = driven.picked();
    let changes = picked.iter().filter(|e| matches!(e, Event::Values(_)));
    assert_eq!(changes.count(), 1, "{picked:?}");
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(!driven.engine.needs_save());
    // The last reading sent is the release's region.
    let last = readings(&picked)
        .into_iter()
        .flatten()
        .last()
        .map(|r| r.region);
    assert!(
        last.is_some_and(|region| same(region, Region::Area([1.0, 0.1, 1.0, 0.5]))),
        "{picked:?}"
    );
    remove(&path);
}

#[test]
fn a_release_and_the_next_drag_s_first_step_in_one_batch_save_apart() {
    let path = scratch("picker-release-and-step");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.writes();
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.picked();
    // The release, moving, and the next drag's first step in one batch: the first drag
    // saved before the second's step applies.
    driven.run([
        moved([0.1, 0.1], [0.5, 0.6], true),
        moved([0.2, 0.2], [0.7, 0.7], false),
    ]);
    driven.picked();
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(driven.engine.needs_save());
    // The second drag's release, where its step left it, saves it on its own.
    driven.run([moved([0.2, 0.2], [0.7, 0.7], true)]);
    driven.picked();
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(!driven.engine.needs_save());
    remove(&path);
}

#[test]
fn a_gpu_error_keeps_a_release_that_missed() {
    let path = scratch("picker-discard-release");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_module("brightness", true)]);
    driven.picked();
    driven.writes();
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.picked();
    // A second step on its way, then the release off the photo with a render that fails:
    // the readings dropped, the next render measures again and completes the drag.
    driven.run([moved([0.1, 0.1], [0.5, 0.6], false)]);
    driven.engine.poison = Some(Poison::Always);
    let events = driven.run([
        moved([1.0, 0.1], [1.0, 0.5], true),
        Command::Resize {
            width: 64,
            height: 64,
        },
    ]);
    assert!(!failures(&events).is_empty(), "{events:?}");
    driven.engine.poison = None;
    driven.picked();
    driven.run([Command::Resize {
        width: 48,
        height: 48,
    }]);
    driven.picked();
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(!driven.engine.needs_save());
    remove(&path);
}

#[test]
fn tools_save_a_drag_under_way_before_their_pick() {
    let path = scratch("picker-tools-mid-drag");
    let mut driven = Driven::new(&[&Tooled]);
    driven.open(&path);
    driven.run([on_picker("tooled", "level", true)]);
    driven.picked();
    driven.writes();
    // A step applied, unsaved, then a new gain: the step saved, then the gain's pick.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.picked();
    assert!(driven.writes().is_empty());
    // Counted as handed to the writer, which writes the two as one where the second comes
    // before it has taken the first.
    let handed = driven.engine.handed;
    let events = driven.run([gain(2.0)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(driven.engine.handed - handed, 2);
    let writes = driven.writes();
    assert!(
        matches!(written(&writes)[..], [(_, true)] | [(_, true), (_, true)]),
        "{writes:?}"
    );
    // The release, where the step left it, has nothing left to save.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    driven.picked();
    assert!(driven.writes().is_empty());
    assert!(!driven.engine.needs_save());
    remove(&path);
}
