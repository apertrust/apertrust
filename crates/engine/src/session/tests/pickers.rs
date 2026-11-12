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
//! statistics under it, the refusals, turning off, and a failure that leaves the frames.

use apertrust_modules::orientation::Orientation;

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
    // A picker no module declares.
    let module = picker(PickerCommand::On {
        picker: PickerRef::Module {
            module: "sample",
            picker: "brightness",
        },
        area: true,
        keys: PickKeys::default(),
    });
    assert_eq!(refused(&driven.run([module])), "UnknownPicker");
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
