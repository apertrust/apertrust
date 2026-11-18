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
//! and tool pickers: read at their operation, applied through their pick as a gesture,
//! turning their tool on and off with it, and in the raw stage per CFA colour.

use apertrust_tools::colour_balance_rgb::ColourBalanceRgb;
use apertrust_tools::orientation::Orientation;
use apertrust_tools::{Picker, PickerKind, Quad, Space};

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
        settings: Vec::new(),
        channel: None,
    })
}

/// `On` for [`Sample`]'s picker `id`, an area where it takes both.
fn on_tool(id: &'static str, area: bool) -> Command {
    on_picker("sample", id, area)
}

/// `On` for the picker `id` of `tool`.
fn on_picker(tool: &'static str, id: &'static str, area: bool) -> Command {
    picker(PickerCommand::On {
        picker: PickerRef::Tool { tool, picker: id },
        area,
        keys: PickKeys::default(),
        settings: Vec::new(),
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

/// The `through` of the picker's end in `events`: a `Picker` with no reading.
fn end(events: &[Event]) -> Option<u64> {
    events.iter().find_map(|e| match e {
        Event::Picker {
            through,
            reading: None,
        } => Some(*through),
        _ => None,
    })
}

/// The pickers' events in `events`.
fn readings(events: &[Event]) -> Vec<&Option<PickerReading>> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Picker { reading, .. } => Some(reading),
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
            [
                Event::Picker {
                    reading: Some(reading),
                    ..
                },
            ] => reading.clone(),
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
        let [
            Event::Picker {
                reading: Some(reading),
                ..
            },
        ] = &picked[..]
        else {
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
    let measured =
        |e: &Event| matches!(e, Event::Picker { reading: Some(r), .. } if r.through >= sent);
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
    // A picker no tool declares, and one of a tool the session lacks.
    assert_eq!(
        refused(&driven.run([on_tool("contrast", true)])),
        "UnknownPicker"
    );
    assert_eq!(
        refused(&driven.run([on_picker("exposure", "exposure", true)])),
        "UnknownPicker"
    );
    // Settings no picker of the tool reads.
    let unread = vec![("target", ControlValue::Number(50.0))];
    let mut tool = on_tool("brightness", true);
    if let Command::Picker(PickerCommand::On {
        settings: asked, ..
    }) = &mut tool
    {
        *asked = unread.clone();
    }
    assert_eq!(refused(&driven.run([tool])), "UnknownSetting(\"target\")");
    let settings = picker(PickerCommand::Settings {
        tool: "sample",
        settings: unread,
    });
    assert_eq!(
        refused(&driven.run([settings])),
        "UnknownSetting(\"target\")"
    );
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
    let newer = brightened("0.9.0").replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
    std::fs::write(sidecar::path_for(&path), newer).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    assert!(matches!(opened.first(), Some(Event::Opened(photo)) if photo.view_only.is_some()));
    assert!(driven.reading([on(false)]).frame.is_some());
    // A tool's picker writes the edit: refused there, the Colour picker's left on.
    let events = driven.run([on_tool("brightness", true)]);
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
        matches!(events.first(), Some(Event::Picker { reading: None, .. })),
        "{events:?}"
    );
    // Its end is through every command applied as it turned off: the open's own number,
    // the resize after it in the batch.
    assert_eq!(end(&events), Some(driven.engine.applied - 1));
    driven.settle();
    assert!(driven.picked().is_empty());
    // Its place is kept for the session, on the new photo too.
    let reading = driven.reading([on(true)]);
    assert!(same(reading.region, Region::Area([0.1, 0.1, 0.5, 0.5])));
    // And off again as the photo closes.
    let events = driven.run([Command::Close]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    assert_eq!(end(&events), Some(driven.engine.applied));
    remove(&path);
    remove(&other);
}

/// darktable's flip bits for each of the Orientation tool's choices: the rows flipped,
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
        tool: "flip".to_owned(),
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
            [
                Event::Picker {
                    reading: Some(reading),
                    ..
                },
            ] => reading,
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
    let [
        Event::Picker {
            reading: Some(reading),
            ..
        },
    ] = &picked[..]
    else {
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
            [
                Event::Failed(SessionError::Pickers(_)),
                Event::Picker { reading: None, .. }
            ]
        ),
        "{events:?}"
    );
    assert_eq!(end(&events), Some(driven.engine.applied));
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

/// The tool parts of the pickers' readings in `events`.
/// The tool parts of the pickers' readings in `events`, each once: a reading whose frame
/// part alone changed carries the tool part the one before it did.
fn tool_parts(events: &[Event]) -> Vec<&ToolReading> {
    let mut parts: Vec<&ToolReading> = readings(events)
        .into_iter()
        .filter_map(|reading| reading.as_ref()?.tool.as_ref())
        .collect();
    parts.dedup();
    parts
}

/// The tool part last sent in `events`.
fn latest(events: &[Event]) -> &ToolReading {
    let parts = tool_parts(events);
    parts
        .last()
        .copied()
        .unwrap_or_else(|| panic!("a tool part in {events:?}"))
}

/// The mean of the first three channels, [`Sample`]'s stand-in for luminance.
fn grey(stats: &Stats) -> f32 {
    (stats.mean[0] + stats.mean[1] + stats.mean[2]) / 3.0
}

fn number(values: &Values, tool: &str, id: &str) -> f32 {
    match control(values, tool, id).value {
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

/// The bits of a tool reading's statistics, compared exactly.
fn bits(stats: &Stats) -> [[u32; 4]; 3] {
    [stats.mean, stats.min, stats.max].map(|v| v.map(f32::to_bits))
}

#[test]
fn a_tool_picker_applies_its_first_reading_and_each_move_never_an_edit_upstream() {
    let path = scratch("picker-tool");
    let mut driven = Driven::new(&[&Orientation, &Sample]);
    driven.open(&path);
    driven.writes();
    // On, with no edit: no frame, a sample-only render at exposure.
    let events = driven.run([on_tool("brightness", true)]);
    assert!(
        failures(&events).is_empty() && !events.iter().any(|e| matches!(e, Event::Frame(_))),
        "{events:?}"
    );
    let picked = driven.picked();
    // Its reading, then the change its pick makes and the frame; then the reading that
    // frame measured at exposure, sent and never applied. The frame's own parts come
    // between them as they arrive.
    let parts = tool_parts(&picked);
    let [part, again] = parts[..] else {
        panic!("{picked:?}");
    };
    let at = |which: &dyn Fn(&Event) -> bool| {
        (picked.iter().position(which)).unwrap_or_else(|| panic!("{picked:?}"))
    };
    let carries = |e: &Event, part: &ToolReading| matches!(e, Event::Picker { reading: Some(r), .. } if r.tool.as_ref() == Some(part));
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
    let brightness_picker = PickerRef::Tool {
        tool: "sample",
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
    let part = (tool_parts(&picked).into_iter())
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
        tool: "flip".to_owned(),
        control: "orientation".to_owned(),
        value: ControlValue::Choice(6),
        done: true,
    }]);
    assert!(driven.engine.pickers.on());
    let picked = driven.picked();
    assert!(
        !picked.is_empty()
            && picked.iter().all(|e| matches!(
                e,
                Event::Picker {
                    reading: Some(_),
                    ..
                }
            )),
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
    driven.run([on_tool("show", false)]);
    let picked = driven.picked();
    let first = latest(&picked);
    let (input, output) = (first.input.unwrap(), first.output.unwrap());
    // A change to the tool, which keeps `show` on: the render measures at exposure.
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
    driven.run([on_tool("brightness", true)]);
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
fn a_tool_picker_turns_its_tool_on_and_off_as_the_tool_changes_unless_it_keeps() {
    let path = scratch("picker-switched");
    let mut driven = Driven::new(&[&Sample, &ColourBalanceRgb]);
    driven.open(&path);
    driven.writes();
    let enabled = |values: &Values| {
        (values.tools.iter())
            .find(|m| m.tool == "colorbalancergb")
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
    let parts = tool_parts(&picked);
    assert!(
        matches!(parts[..], [part] if part.input.is_some() && part.output.is_none()),
        "{picked:?}"
    );
    assert!(!picked.iter().any(|e| matches!(e, Event::Values(_))));
    // A change to its tool turns it off, as darktable's `gui_changed` does.
    let events = driven.run([Command::ResetTool {
        tool: "colorbalancergb".to_owned(),
    }]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    // Its end is through the reset, so a client knows it answers its `On`.
    assert_eq!(end(&events), Some(driven.engine.applied));
    // A change to another tool leaves it on; Sample's `show`, darktable's keep-active,
    // stays on through its own tool's.
    driven.run([on_tool("show", true)]);
    driven.picked();
    let events = driven.run([
        Command::ResetTool {
            tool: "colorbalancergb".to_owned(),
        },
        set("grain", ControlValue::Number(20.0), true),
    ]);
    assert!(
        readings(&events).is_empty() && driven.engine.pickers.on(),
        "{events:?}"
    );
    // `spread` does not keep: off as its tool changes.
    driven.run([on_tool("spread", true)]);
    driven.picked();
    let events = driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(matches!(readings(&events)[..], [None]), "{events:?}");
    assert_eq!(end(&events), Some(driven.engine.applied));
    // `Off` ends it through itself.
    driven.run([on_tool("show", true)]);
    driven.picked();
    let events = driven.run([picker(PickerCommand::Off)]);
    assert_eq!(end(&events), Some(driven.engine.applied));
    remove(&path);
}

#[test]
fn a_pick_reads_the_channel_and_keys_shown_as_it_reads() {
    let path = scratch("picker-channel");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let grain = |events: &[Event]| number(values(events), "sample", "grain");
    let spread = |part: &ToolReading, c: usize| {
        let input = part.input.unwrap();
        100.0 * (input.max[c] - input.min[c])
    };
    // `spread` with the red channel shown: red's spread, denoised, in percent.
    let mut on = on_tool("spread", true);
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
    // No settings: kept, and nothing applied, since no picker of Sample reads one.
    let events = driven.run([picker(PickerCommand::Settings {
        tool: "sample",
        settings: Vec::new(),
    })]);
    assert!(events.is_empty(), "{events:?}");
    remove(&path);
}

/// A tool with one area picker on temperature's input, the mosaic, as White balance's
/// `spot` reads it, which writes nothing: its one control is temperature's red
/// coefficient, which it owns and binds to nothing.
struct Spot;

static SPOT: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition::processing(
        "spot",
        &[Tab {
            id: None,
            rows: &[Row::Slider(Slider::new("red").quad(Quad::Picker("red")))],
        }],
    ),
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
    preset_rows: &[],
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
        settings: &[],
        words_by: None,
    }],
    facts: None,
};

static SPOT_OWNS: [Owned; 1] = [Owned::new("temperature", "red")];

impl ProcessingTool for Spot {
    fn definition(&self) -> &'static ProcessingDefinition {
        &SPOT
    }

    fn owns(&self) -> &'static [Owned] {
        &SPOT_OWNS
    }

    fn bind(&self, _: &ToolValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
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
    use apertrust_tools::exposure::Exposure;

    let corpus = std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    );
    let raw = corpus.join("Sony/ILCE-7RM5/7RM5-LosslessCompressedLarge.ARW");
    let path = scratch("picker-budget").with_extension("ARW");
    std::fs::hard_link(&raw, &path)
        .or_else(|_| std::fs::copy(&raw, &path).map(|_| ()))
        .expect("the corpus raw");
    let cases: [(&str, &'static [&'static dyn ProcessingTool], &str, &str); 2] = [
        (
            "exposure, after the downscale",
            &[&Exposure],
            "exposure",
            "exposure",
        ),
        ("temperature, in the raw stage", &[&Spot], "spot", "red"),
    ];
    for (name, tools, tool, id) in cases {
        let mut driven = Driven::new(tools);
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
        driven.run([on_picker(tool, id, true)]);
        driven.picked();
        let mut latencies = Vec::new();
        for run in 0..60u32 {
            // Never the default area the picker started at, which would move nothing.
            let at = 0.021 + 0.001 * (run % 30) as f32;
            let start = Instant::now();
            driven.run([moved([at, at], [1.0 - at, 1.0 - at], true)]);
            let read = loop {
                let events = driven.engine.picked();
                if !tool_parts(&events).is_empty() {
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
    driven.run([on_tool("brightness", true)]);
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
    // Releasing again where the area is reads nothing and changes nothing: it is
    // answered all the same, the edit reported held.
    let events = driven.run([moved([0.1, 0.1], [0.7, 0.8], true)]);
    assert!(driven.picked().is_empty(), "{events:?}");
    assert_eq!(written(&driven.writes()).len(), 1);
    remove(&path);

    // A release while only a reading an edit caused is on its way, never applied:
    // complete at once, and the next drag's first step still a step.
    let path = scratch("picker-release-edit-reading");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_tool("brightness", true)]);
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
        &driven.engine.tools,
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
    // Its reading has no tool part, and nothing is applied.
    let picked = driven.picked();
    assert!(
        matches!(&picked[..], [Event::Picker { reading: Some(reading), .. }] if reading.tool.is_none()),
        "{picked:?}"
    );
    remove(&path);
}

#[test]
fn a_release_whose_reading_misses_completes_the_drag_s_change() {
    let path = scratch("picker-release-missed");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_tool("brightness", true)]);
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

/// A tool with one area picker on exposure's input, which reads the setting `gain`:
/// its pick sets `level` to the gain times the mean's first channel, and with no gain
/// writes `other`, which its picker does not write, as a tool's bug would; it shows a
/// patch of the gain. It owns
/// exposure's exposure and binds nothing.
struct Configured;

static CONFIGURED: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition::processing(
        "configured",
        &[Tab {
            id: None,
            rows: &[
                Row::Slider(Slider::new("level").quad(Quad::Picker("level"))),
                Row::Slider(Slider::new("other")),
                Row::Setting(apertrust_tools::SettingRow {
                    id: "gain",
                    kind: ControlKind::Number {
                        min: 0.0,
                        max: 4.0,
                        step: 0.1,
                        default: 1.0,
                    },
                    decimals: 1,
                    track: apertrust_tools::Track::Plain,
                    key: None,
                    words: None,
                }),
            ],
        }],
    ),
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
    preset_rows: &[],
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
        settings: &["gain"],
        words_by: None,
    }],
    facts: None,
};

static CONFIGURED_OWNS: [Owned; 1] = [Owned::new("exposure", "exposure")];

impl ProcessingTool for Configured {
    fn definition(&self) -> &'static ProcessingDefinition {
        &CONFIGURED
    }

    fn owns(&self) -> &'static [Owned] {
        &CONFIGURED_OWNS
    }

    fn bind(&self, _: &ToolValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        Vec::new()
    }

    fn pick(
        &self,
        _: &str,
        reading: &apertrust_tools::pick::Reading,
        _: &Controls,
        _: &ImageContext,
        settings: &[(&str, ControlValue)],
        _: Option<&str>,
        _: Option<&apertrust_tools::pick::Reading>,
        _: PickKeys,
    ) -> Option<apertrust_tools::pick::Picked> {
        let gain = settings
            .iter()
            .find_map(|&(setting, ref value)| match (setting, value) {
                ("gain", &ControlValue::Number(gain)) => Some(gain),
                _ => None,
            })
            .unwrap_or(1.0);
        let mut writes = Controls::new();
        let control = if gain == 0.0 { "other" } else { "level" };
        writes.set(control, ControlValue::Number(gain * reading.input.mean[0]));
        // A patch of the gain, so each gain shows something of its own.
        let patch = apertrust_tools::pick::Patch {
            srgb: [gain / 4.0; 3],
            lch: [25.0 * gain, 0.0, 0.0],
        };
        Some(apertrust_tools::pick::Picked {
            writes,
            // The gain it measured, as a measure mode gives its settings back.
            shown: apertrust_tools::pick::Shown {
                patch: Some(patch),
                settings: vec![("gain", ControlValue::Number(gain + 1.0))],
                ..Default::default()
            },
        })
    }
}

fn gain(gain: f32) -> Command {
    picker(PickerCommand::Settings {
        tool: "configured",
        settings: vec![("gain", ControlValue::Number(gain))],
    })
}

#[test]
fn settings_apply_the_latest_reading_measured_and_a_refused_pick_turns_the_picker_off() {
    let path = scratch("picker-settings");
    let mut driven = Driven::new(&[&Configured]);
    driven.open(&path);
    driven.run([on_picker("configured", "level", true)]);
    let picked = driven.picked();
    let shown = |events: &[Event]| match readings(events).last() {
        Some(Some(reading)) => reading.tool.as_ref().and_then(|m| m.picked.clone()),
        _ => None,
    };
    // A reading's pick hands its settings back.
    assert_eq!(shown(&picked).unwrap().settings.len(), 1, "{picked:?}");
    let (last, latest) = driven.engine.pickers.readings_of("configured");
    assert_eq!(last, latest);
    // A larger viewport: exposure's input measured again, never applied, so the latest
    // reading is no longer the last applied.
    driven.run([Command::Resize {
        width: 64,
        height: 64,
    }]);
    driven.picked();
    let (last, latest) = driven.engine.pickers.readings_of("configured");
    let (last, latest) = (last.unwrap(), latest.unwrap());
    assert_ne!(last.input.mean[0], latest.input.mean[0]);
    // A new gain applies the latest, darktable's `picked_color`, and sends the reading.
    let events = driven.run([gain(2.0)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(
        number(values(&events), "configured", "level"),
        (2.0 * latest.input.mean[0]).clamp(0.0, 10.0)
    );
    assert!(matches!(readings(&events)[..], [Some(_)]), "{events:?}");
    // The settings the client has just sent are never handed back.
    let shown = shown(&events).unwrap();
    assert!(
        shown.patch.is_some() && shown.settings.is_empty(),
        "{shown:?}"
    );
    assert_eq!(
        driven.engine.pickers.readings_of("configured").0,
        Some(latest)
    );
    // A pick its tool gets wrong is refused, the picker off, the last reading kept.
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
    assert_eq!(
        driven.engine.pickers.readings_of("configured").0,
        Some(latest)
    );
    remove(&path);
}

#[test]
fn a_waiting_reading_replaced_passes_its_application_on() {
    let path = scratch("picker-keep-due");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_tool("brightness", true)]);
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
    driven.run([on_tool("show", true)]);
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
    driven.run([on_tool("show", true)]);
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
    driven.run([on_tool("brightness", true)]);
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
    driven.run([on_tool("brightness", true)]);
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
    driven.run([on_tool("brightness", true)]);
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
fn settings_save_a_drag_under_way_before_their_pick() {
    let path = scratch("picker-settings-mid-drag");
    let mut driven = Driven::new(&[&Configured]);
    driven.open(&path);
    driven.run([on_picker("configured", "level", true)]);
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
    // The release, where the step left it, has nothing left to save: it is answered, the
    // edit reported held.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    driven.picked();
    assert_eq!(written(&driven.writes()).len(), 1);
    assert!(!driven.engine.needs_save());
    remove(&path);
}

/// Every completing command of a tool's picker is answered with a `Saved` through it,
/// a pick that wrote nothing included, the edit reported held (decision 79): `show`'s
/// `On` and a release of its, neither of which writes.
#[test]
fn a_pick_that_writes_nothing_is_answered_with_a_save() {
    let path = scratch("picker-answered");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.writes();
    let saved = |writes: &[Event]| {
        (writes.iter())
            .filter_map(|e| match e {
                Event::Saved { through, .. } => Some(*through),
                _ => None,
            })
            .max()
    };
    driven.run([on_tool("show", true)]);
    driven.picked();
    let on = driven.engine.applied;
    assert!(saved(&driven.writes()).is_some_and(|t| t >= on));
    driven.run([area([0.2, 0.2], [0.4, 0.4])]);
    driven.picked();
    let release = driven.engine.applied;
    assert!(saved(&driven.writes()).is_some_and(|t| t >= release));
    assert!(driven.engine.pickers.owed.is_empty());
    remove(&path);
}

/// A pick that writes nothing, after the edit's last write failed, writes the edit again as
/// it answers: held, as after a cancel, which writes only a failed write's edit.
#[test]
fn a_pick_that_writes_nothing_writes_again_after_a_failed_write() {
    let folder = scratch("picker-missing").with_file_name("picker not yet");
    let path = folder.join("synthetic.ARW");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.4), true)]);
    let writes = driven.writes();
    assert!(
        matches!(&writes[..], [Event::Failed(SessionError::Save { .. })]),
        "{writes:?}"
    );
    std::fs::create_dir_all(&folder).unwrap();
    driven.run([on_tool("show", true)]);
    driven.picked();
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    assert!(sidecar.exists());
    std::fs::remove_dir_all(folder.parent().unwrap()).unwrap();
}

/// A write answering an earlier command never answers a picker's `On` whose first pick is
/// still to come: `Settings` with no reading kept, answered held in the batch, then `On` for
/// a tool already on, the `Saved` through `Settings` alone until the pick arrives.
#[test]
fn a_write_never_answers_a_pick_still_to_come() {
    let path = scratch("picker-pick-to-come");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.writes();
    let saved = |writes: &[Event]| {
        (writes.iter())
            .filter_map(|e| match e {
                Event::Saved { through, .. } => Some(*through),
                _ => None,
            })
            .max()
    };
    let settings = picker(PickerCommand::Settings {
        tool: "sample",
        settings: Vec::new(),
    });
    driven.run([settings, on_tool("show", true)]);
    let on = driven.engine.applied;
    assert_eq!(saved(&driven.writes()), Some(on - 1));
    driven.picked();
    assert!(saved(&driven.writes()).is_some_and(|t| t >= on));
    assert!(driven.engine.pickers.owed.is_empty());
    remove(&path);
}

/// A tool picker's `On` replacing one with a drag under way is never answered by the
/// writes that close the drag and turn the old picker off: those are through the command
/// before it, and only its pick's answer covers it.
#[test]
fn replacing_a_picker_never_answers_the_new_one_early() {
    let path = scratch("picker-replaced");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let saved = |writes: &[Event]| {
        (writes.iter())
            .filter_map(|e| match e {
                Event::Saved { through, .. } => Some(*through),
                _ => None,
            })
            .max()
    };
    driven.run([on_tool("brightness", true)]);
    driven.picked();
    // A step written, unsaved: a drag under way.
    driven.run([moved([0.1, 0.1], [0.4, 0.5], false)]);
    driven.picked();
    assert!(driven.engine.unsaved());
    driven.writes();
    driven.run([on_tool("show", true)]);
    let on = driven.engine.applied;
    assert_eq!(saved(&driven.writes()), Some(on - 1));
    driven.picked();
    assert!(saved(&driven.writes()).is_some_and(|t| t >= on));
    assert!(driven.engine.pickers.owed.is_empty());
    remove(&path);
}

/// Exposure's Picker target at Lightness, at this target lightness (decision 88).
fn mapping(lightness: f32) -> Vec<(&'static str, ControlValue)> {
    vec![
        ("target", ControlValue::Choice(1)),
        ("lightness", ControlValue::Number(lightness)),
    ]
}

/// A pick that gives a value that is not finite, Exposure's at a target lightness of 0 %:
/// that write is left out, Exposure as it was, and what the pick shows, its patch and L,
/// still sent, as darktable shows them before it sets anything (`exposure.c:885-897`).
/// The same reading at 50 % writes Exposure.
#[test]
fn a_value_a_pick_gives_that_is_not_finite_is_left_out_and_what_it_shows_kept() {
    use apertrust_tools::exposure::Exposure;
    let path = scratch("picker-not-finite");
    let mut driven = Driven::new(&[&Exposure]);
    let opened = driven.open(&path);
    let before = number(values(&opened), "exposure", "exposure");
    let mut events = driven.run([picker(PickerCommand::On {
        picker: PickerRef::Tool {
            tool: "exposure",
            picker: "exposure",
        },
        area: true,
        keys: PickKeys::default(),
        settings: mapping(0.0),
        channel: None,
    })]);
    events.extend(driven.picked());
    assert!(failures(&events).is_empty(), "{events:?}");
    let shown = readings(&events)
        .into_iter()
        .rev()
        .find_map(|reading| reading.as_ref()?.tool.as_ref()?.picked.clone())
        .unwrap_or_else(|| panic!("a pick shown in {events:?}"));
    let patch = shown.patch.expect("the input patch");
    assert!(patch.lch[0] > 0.0, "{patch:?}");
    for event in &events {
        if let Event::Values(values) = event {
            assert_eq!(number(values, "exposure", "exposure"), before);
        }
    }
    // The latest reading again, by a target that gives a value.
    let events = driven.run([picker(PickerCommand::Settings {
        tool: "exposure",
        settings: mapping(50.0),
    })]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let after = number(values(&events), "exposure", "exposure");
    assert!(
        after.is_finite() && after != before,
        "{after} after {before}"
    );
    remove(&path);
}

/// Exposure's Picker target at Reference, with the reference at `lightness`, or none.
fn toward_reference(lightness: Option<f32>) -> Vec<(&'static str, ControlValue)> {
    let mut settings = vec![("target", ControlValue::Choice(2))];
    if let Some(lightness) = lightness {
        settings.push(("reference_lightness", ControlValue::Number(lightness)));
    }
    settings
}

/// Exposure's `picker` turned on as an area with these settings.
fn on_exposure(id: &'static str, settings: Vec<(&'static str, ControlValue)>) -> Command {
    picker(PickerCommand::On {
        picker: PickerRef::Tool {
            tool: "exposure",
            picker: id,
        },
        area: true,
        keys: PickKeys::default(),
        settings,
        channel: None,
    })
}

/// Exposure's picker toward a reference with none set cannot start: its `On` is refused,
/// `NotReady`, and no picker is on (decision 88). With one set it starts; clearing the
/// reference by `Settings` turns it off, applying nothing.
#[test]
fn exposure_s_picker_waits_for_a_reference_and_turns_off_when_it_goes() {
    use apertrust_tools::exposure::Exposure;
    let path = scratch("picker-not-ready");
    let mut driven = Driven::new(&[&Exposure]);
    let opened = driven.open(&path);
    let before = number(values(&opened), "exposure", "exposure");
    let events = driven.run([on_exposure("exposure", toward_reference(None))]);
    match &failures(&events)[..] {
        [error] => assert_eq!(format!("{error:?}"), "NotReady"),
        _ => panic!("{events:?}"),
    }
    assert!(!driven.engine.pickers.on());
    // With a reference it starts and picks toward it.
    let mut events = driven.run([on_exposure("exposure", toward_reference(Some(62.0)))]);
    events.extend(driven.picked());
    assert!(failures(&events).is_empty(), "{events:?}");
    assert!(driven.engine.pickers.on());
    let picked = number(values(&events), "exposure", "exposure");
    assert_ne!(picked, before);
    // The reference cleared: off, nothing applied.
    let events = driven.run([picker(PickerCommand::Settings {
        tool: "exposure",
        settings: toward_reference(None),
    })]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert!(!driven.engine.pickers.on());
    for event in &events {
        if let Event::Values(values) = event {
            assert_eq!(number(values, "exposure", "exposure"), picked);
        }
    }
    remove(&path);
}

/// A tool picker's reading carries its operation's input's maximum, each channel's, which
/// Exposure's pick reads clipped pixels by (decision 88), and the pick shows its Last pick.
#[test]
fn a_reading_carries_its_operation_s_input_maximum() {
    use apertrust_tools::exposure::Exposure;
    let path = scratch("picker-input-maximum");
    let mut driven = Driven::new(&[&Exposure]);
    driven.open(&path);
    let mut events = driven.run([on_exposure("exposure", Vec::new())]);
    events.extend(driven.picked());
    assert!(failures(&events).is_empty(), "{events:?}");
    let open = driven.engine.open.as_ref().unwrap();
    let plan = open.pipeline.as_ref().expect("a live pipeline").plan();
    let at = (plan.operations.iter())
        .position(|operation| operation.id == "exposure")
        .unwrap();
    let expected = match at.checked_sub(1) {
        Some(before) => plan.operations[before].output_desc.maximum,
        None => plan.input.maximum,
    };
    let reading = driven.engine.pickers.latest_reading("exposure").unwrap();
    assert_eq!(reading.input_maximum, expected);
    assert!(expected.iter().all(|channel| *channel > 0.0));
    let shown = readings(&events)
        .into_iter()
        .rev()
        .find_map(|reading| reading.as_ref()?.tool.as_ref()?.picked.clone())
        .unwrap_or_else(|| panic!("a pick shown in {events:?}"));
    assert!(shown.last.is_some(), "{shown:?}");
    remove(&path);
}

/// The reference row's picker measures the area and hands its L back as
/// `reference_lightness`; it writes nothing and makes no history step (E54).
#[test]
fn the_reference_picker_hands_its_lightness_back_and_makes_no_step() {
    use apertrust_tools::exposure::Exposure;
    let path = scratch("picker-reference");
    let mut driven = Driven::new(&[&Exposure]);
    let opened = driven.open(&path);
    let before = number(values(&opened), "exposure", "exposure");
    let steps = |driven: &Driven| {
        let open = driven.engine.open.as_ref().unwrap();
        open.history.log().steps().len()
    };
    let start = steps(&driven);
    let mut events = driven.run([on_exposure("reference", toward_reference(None))]);
    events.extend(driven.picked());
    assert!(failures(&events).is_empty(), "{events:?}");
    let shown = readings(&events)
        .into_iter()
        .rev()
        .find_map(|reading| reading.as_ref()?.tool.as_ref()?.picked.clone())
        .unwrap_or_else(|| panic!("a pick shown in {events:?}"));
    let [("reference_lightness", ControlValue::Number(l))] = shown.settings[..] else {
        panic!("{:?}", shown.settings)
    };
    assert!((0.0..=100.0).contains(&l), "{l}");
    for event in &events {
        if let Event::Values(values) = event {
            assert_eq!(number(values, "exposure", "exposure"), before);
        }
    }
    driven.run([picker(PickerCommand::Off)]);
    assert_eq!(steps(&driven), start);
    remove(&path);
}

/// A pick that ends a person's slider gesture under way, on another tool, makes the
/// gesture a step of its own source first, You, never the pick's Auto.
#[test]
fn a_pick_ends_a_slider_s_gesture_as_a_step_of_its_own_source() {
    use crate::history::{Change, Entry, StepSource};
    let path = scratch("picker-ends-gesture");
    let mut driven = Driven::new(&[&Sample, &apertrust_tools::soften::Soften]);
    driven.open(&path);
    driven.run([on_tool("brightness", true)]);
    driven.picked();
    driven.writes();
    driven.run([Command::SetControl {
        tool: "soften".to_owned(),
        control: "amount".to_owned(),
        value: ControlValue::Number(70.0),
        done: false,
    }]);
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    driven.picked();
    let open = driven.engine.open.as_ref().unwrap();
    let of = |tool: &str| {
        let entry = |step: &&crate::history::Step| matches!(&step.change, Change::Entry(Entry::Tool { tool: m, .. }) if m == tool);
        (open.history.log().steps().iter())
            .rfind(entry)
            .map(|step| step.source)
    };
    assert_eq!(of("soften"), Some(StepSource::You));
    assert_eq!(of("sample"), Some(StepSource::Auto));
    remove(&path);
}

/// A drag's picks settling as their reading arrives, with no batch after it, send the
/// step they made as `History`, after their `Values`, so Undo is named at once.
#[test]
fn a_settled_pick_sends_its_history() {
    let path = scratch("picker-history");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([on_tool("brightness", true)]);
    driven.picked();
    driven.writes();
    driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    let picked = driven.picked();
    let at = |kind: fn(&Event) -> bool| picked.iter().rposition(kind);
    let history = at(|e| matches!(e, Event::History(_)));
    let values = at(|e| matches!(e, Event::Values(_)));
    assert!(
        history.is_some_and(|h| values.is_some_and(|v| v < h)),
        "{picked:?}"
    );
    remove(&path);
}

fn hide(tool: &str, hidden: bool) -> Command {
    Command::Hide {
        tool: tool.to_owned(),
        hidden,
    }
}

/// While the History tab's eye hides a tool, a tool's picker already on measures and
/// applies nothing (decision 78): what it read would be the view's, and what it wrote the
/// edit's. A `Hide` leaves it on; a move meanwhile writes nothing to the edit or its file,
/// its reading waiting; shown again, the next reading applies as usual. `On` for a
/// tool's picker is refused while a tool is hidden, `Hidden`; the Colour
/// picker's, which writes nothing, still turns on.
#[test]
fn a_tool_picker_applies_no_reading_while_a_tool_is_hidden() {
    let path = scratch("picker-hidden");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(&[&Orientation, &Sample]);
    driven.open(&path);
    driven.run([on_tool("brightness", true)]);
    let before = number(values(&driven.picked()), "sample", "brightness");
    driven.writes();
    let file = std::fs::read(&sidecar).unwrap();
    let edit = driven.engine.open.as_ref().unwrap().edit.clone();
    // Hidden: the picker stays on.
    let hidden = driven.run([hide("flip", true)]);
    assert!(failures(&hidden).is_empty(), "{hidden:?}");
    assert_eq!(end(&hidden), None);
    // A move measures and applies nothing, writing nothing.
    let moved_events = driven.run([moved([0.1, 0.1], [0.4, 0.5], true)]);
    assert!(failures(&moved_events).is_empty(), "{moved_events:?}");
    let picked = driven.picked();
    assert!(
        !picked.iter().any(|e| matches!(e, Event::Values(_))),
        "{picked:?}"
    );
    assert_eq!(driven.engine.open.as_ref().unwrap().edit, edit);
    driven.writes();
    assert_eq!(std::fs::read(&sidecar).unwrap(), file);
    // `On` for a tool's picker is refused while hidden.
    let refused = driven.run([on_tool("spread", true)]);
    assert!(
        matches!(failures(&refused)[..], [SessionError::Hidden]),
        "{refused:?}"
    );
    // Shown again: the moved area's reading applies.
    let shown = driven.run([hide("flip", false)]);
    assert!(failures(&shown).is_empty(), "{shown:?}");
    let picked = driven.picked();
    let after = number(values(&picked), "sample", "brightness");
    assert!((after - before).abs() > 1e-4, "{before} then {after}");
    assert_ne!(driven.engine.open.as_ref().unwrap().edit, edit);
    // The Colour picker's turns on while a tool is hidden.
    driven.run([hide("flip", true)]);
    let reading = driven.reading([on(false)]);
    assert_eq!(reading.picker, PickerRef::ColourPicker);
    remove(&path);
}

/// The tool of each step of the open photo's log, in order.
fn steps_of(driven: &Driven) -> Vec<String> {
    use crate::history::{Change, Entry};
    let open = driven.engine.open.as_ref().unwrap();
    (open.history.log().steps().iter())
        .map(|step| match &step.change {
            Change::Entry(Entry::Tool { tool, .. }) => tool.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

/// White balance's picker (`docs/tools/white-balance.md`, After a pick): Settings offers
/// from image area as soon as the photo can be edited; `On` through it applies its first
/// reading as one step of source Auto, the coefficients darktable's `color_picker_apply`
/// gives for the mosaic's means and `preset` from image area, and Settings reads from
/// image area. Shown again, as darktable's `gui_update` redraws the tool, it reads user
/// modified: after Undo then Redo, and reopened, never a step of its own and nothing
/// written for it, the file holding from image area through another tool's change until
/// White balance's own next step. A Settings option while the picker is on turns it off.
#[test]
fn white_balance_s_picker_reads_from_image_area_then_user_modified_when_shown_again() {
    use apertrust_tools::exposure::Exposure;
    use apertrust_tools::white_balance::{SETTINGS, WhiteBalance};

    use crate::history::StepSource;
    const TOOLS: &[&dyn ProcessingTool] = &[&WhiteBalance, &Exposure];
    let path = scratch("picker-white-balance");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(TOOLS);
    let row = |values: &Values| {
        let tool = (values.tools.iter())
            .find(|tool| tool.tool == "temperature")
            .unwrap();
        (tool.presets.iter())
            .find(|row| row.preset == "settings")
            .unwrap()
            .clone()
    };
    let preset = |option| {
        let at = SETTINGS.iter().position(|o| *o == option).unwrap();
        ControlValue::Choice(at as u32)
    };
    let settings = |values: &Values| {
        let value = control(values, "temperature", "preset").value.clone();
        (row(values).matching, value)
    };
    // What the file holds of `preset`, once the writer is done.
    let saved = |driven: &Driven| {
        driven.writes();
        let read = sidecar::read(&sidecar, OPERATIONS, TOOLS, &[]).unwrap();
        let values = read.edit.tool("temperature").cloned().unwrap_or_default();
        values.get("preset").cloned()
    };
    let steps = |driven: &Driven| {
        let open = driven.engine.open.as_ref().unwrap();
        (open.history.log().steps().iter())
            .map(|step| step.source)
            .collect::<Vec<_>>()
    };
    let opened = driven.open(&path);
    assert!(row(values(&opened)).available.contains(&"spot"));
    let before = steps(&driven);

    // The pick: the coefficients that make the area neutral, from image area.
    let mut events = driven.run([on_picker("temperature", "spot", true)]);
    events.extend(driven.picked());
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(settings(values(&events)), (Some("spot"), preset("spot")));
    let m = latest(&events).input.expect("its input read").mean;
    let inverse = |mean: f32| if mean > 0.001 { 1.0 / mean } else { 1.0 };
    for (c, id) in ["red", "green", "blue"].into_iter().enumerate() {
        let expected = match c {
            1 => 1.0,
            _ => (inverse(m[c]) / inverse(m[1])).clamp(0.0, 8.0),
        };
        let picked = number(values(&events), "temperature", id);
        assert!(
            (picked - expected).abs() <= 1e-6,
            "{id}: {picked} against {expected}"
        );
    }
    let after = steps(&driven);
    assert_eq!(after.len(), before.len() + 1, "{after:?}");
    assert_eq!(after.last(), Some(&StepSource::Auto));
    assert_eq!(saved(&driven), Some(preset("spot")));
    driven.run([picker(PickerCommand::Off)]);

    // Undo, then Redo: shown again, user modified, the file still holding from image area.
    let undone = driven.run([Command::Undo]);
    assert_eq!(settings(values(&undone)).0, Some("d65_late"));
    let redone = driven.run([Command::Redo]);
    assert!(failures(&redone).is_empty(), "{redone:?}");
    assert_eq!(settings(values(&redone)), (Some("user"), preset("user")));
    assert_eq!(saved(&driven), Some(preset("spot")));
    assert_eq!(steps(&driven), after);

    // Another photo, closing this one, then this one again: user modified, nothing
    // written.
    driven.open(&scratch("picker-white-balance-other"));
    assert_eq!(saved(&driven), Some(preset("spot")));
    let file = std::fs::read(&sidecar).unwrap();
    let reopened = driven.open(&path);
    assert_eq!(settings(values(&reopened)), (Some("user"), preset("user")));
    assert!(written(&driven.writes()).is_empty());
    assert_eq!(std::fs::read(&sidecar).unwrap(), file);
    // The next change, of another tool, is one step of its own, the file still holding
    // from image area.
    let before = steps_of(&driven);
    let events = driven.run([Command::SetControl {
        tool: "exposure".to_owned(),
        control: "exposure".to_owned(),
        value: ControlValue::Number(0.5),
        done: true,
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let after = steps_of(&driven);
    assert_eq!(after[..before.len()], before[..]);
    assert_eq!(after[before.len()..], ["exposure".to_owned()]);
    assert_eq!(saved(&driven), Some(preset("spot")));
    // White balance's own next change, here a coefficient released, saves it.
    let events = driven.run([Command::Keep {
        tool: "temperature".to_owned(),
        control: "blue".to_owned(),
        kept: false,
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(
        steps_of(&driven).last().map(String::as_str),
        Some("temperature")
    );
    assert_eq!(steps_of(&driven).len(), after.len() + 1);
    assert_eq!(saved(&driven), Some(preset("user")));

    // A Settings option while the picker is on turns it off.
    let mut events = driven.run([on_picker("temperature", "spot", true)]);
    events.extend(driven.picked());
    assert_eq!(settings(values(&events)).0, Some("spot"));
    assert!(driven.engine.pickers.on());
    let events = driven.run([Command::ApplyPreset {
        tool: "temperature".to_owned(),
        preset: "settings".to_owned(),
        option: "as_shot".to_owned(),
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert!(end(&events).is_some(), "{events:?}");
    assert!(!driven.engine.pickers.on());
    assert_eq!(
        settings(values(&events)),
        (Some("as_shot"), preset("as_shot"))
    );
    // From image area is never applied.
    let refused = driven.run([Command::ApplyPreset {
        tool: "temperature".to_owned(),
        preset: "settings".to_owned(),
        option: "spot".to_owned(),
    }]);
    assert_eq!(failures(&refused).len(), 1, "{refused:?}");
    driven.writes();
    remove(&path);
}

/// A redraw not yet saved is never laid over a newer file's White balance: picked, then
/// reopened, Settings reads user modified with the file holding from image area; another
/// program then writes White balance's red and as shot, and the session's next change, of
/// Exposure, takes that file in, its White balance standing as the other program wrote it.
#[test]
fn a_redraw_not_yet_saved_never_replaces_a_newer_file_s_white_balance() {
    a_newer_file_s_white_balance_stands("picker-white-balance-newer", true);
}

/// Nor is a pick saved this session, which the edit holds as from image area while its
/// fold as read is user modified: picked, then a newer file with White balance's red and
/// as shot taken in, the newer White balance stands.
#[test]
fn a_pick_saved_this_session_never_replaces_a_newer_file_s_white_balance() {
    a_newer_file_s_white_balance_stands("picker-white-balance-newer-pick", false);
}

/// A pick, then the photo reopened where `reopen`, then another program's White balance
/// taken in with the session's next change, of Exposure: the other program's stands.
fn a_newer_file_s_white_balance_stands(name: &str, reopen: bool) {
    use apertrust_tools::exposure::Exposure;
    use apertrust_tools::white_balance::WhiteBalance;

    use crate::history::{Change, StepSource};
    const TOOLS: &[&dyn ProcessingTool] = &[&WhiteBalance, &Exposure];
    let path = scratch(name);
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    let mut events = driven.run([on_picker("temperature", "spot", true)]);
    events.extend(driven.picked());
    assert!(failures(&events).is_empty(), "{events:?}");
    driven.run([picker(PickerCommand::Off)]);
    driven.writes();
    if reopen {
        driven.open(&scratch(&format!("{name}-other")));
        driven.writes();
        let reopened = driven.open(&path);
        let user = ControlValue::Choice(2);
        assert_eq!(
            control(values(&reopened), "temperature", "preset").value,
            user
        );
    }

    // Another program's step a minute on: red at 2.5 and as shot.
    let open = driven.engine.open.as_ref().unwrap();
    let mut theirs = open.history.log().clone();
    let mut clocks = open.history.clocks().to_vec();
    let last = theirs.steps().last().unwrap().clock.when;
    let clock = Clock::at(last + 60_000, None, "00000000".parse().unwrap());
    let fold = theirs.fold();
    let mut newer = fold.clone();
    let image = testing::image();
    let red = ControlValue::Number(2.5);
    newer
        .set_control(&WhiteBalance, "red", red.clone(), &image)
        .unwrap();
    let as_shot = ControlValue::Choice(0);
    (newer.set_control(&WhiteBalance, "preset", as_shot.clone(), &image)).unwrap();
    theirs.add_change(
        Change::between(&fold, &newer, &[]),
        StepSource::You,
        None,
        clock,
    );
    clocks.push(clock);
    let file = sidecar::Edits::new("");
    sidecar::write_log(
        &sidecar,
        &file,
        &theirs,
        &clocks,
        (OPERATIONS, TOOLS),
        "cli",
    )
    .unwrap();

    // The session's next change, of Exposure, finds it and takes it in.
    let events = driven.run([Command::SetControl {
        tool: "exposure".to_owned(),
        control: "exposure".to_owned(),
        value: ControlValue::Number(0.5),
        done: true,
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    driven.writes();
    let taken = driven.engine.newer_file(false);
    assert!(failures(&taken).is_empty(), "{taken:?}");
    driven.writes();
    let shown = values(&taken);
    assert_eq!(control(shown, "temperature", "red").value, red);
    assert_eq!(control(shown, "temperature", "preset").value, as_shot);
    assert_eq!(number(shown, "exposure", "exposure"), 0.5);
    let read = sidecar::read(&sidecar, OPERATIONS, TOOLS, &[]).unwrap();
    let saved = read.edit.tool("temperature").unwrap();
    assert_eq!(
        (saved.get("red"), saved.get("preset")),
        (Some(&red), Some(&as_shot))
    );
    remove(&path);
}
