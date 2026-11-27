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

//! The live frame at Fit in bounded submissions through the session (decision 112;
//! `docs/engine.md`, The live frame in bounded submissions): a frame in chunks equal to
//! the one submission's, each chunk a turn at the submit gate, the copy and the
//! timestamps in the last; a tool picker's reading equal to the one submission's; and a
//! command sent between a frame's chunks rendered in the next frame, with the window's
//! turn at the gate between the chunks.

use super::*;
use crate::pipeline::Chunking;

/// The live pipeline of the photo `driven` has open.
fn live(driven: &mut Driven) -> &mut Pipeline {
    let open = driven.engine.open.as_mut().expect("a photo open");
    open.pipeline.as_mut().expect("its pipeline")
}

/// `On` for [`Sample`]'s picker `show`, which reads exposure's input and output.
fn show() -> Command {
    Command::Picker(PickerCommand::On {
        picker: PickerRef::Tool {
            tool: "sample",
            picker: "show",
        },
        area: false,
        keys: PickKeys::default(),
        settings: Vec::new(),
        channel: None,
    })
}

/// A frame of the synthetic photo, a change of [`Sample`]'s brightness rendered with
/// `chunking`, and, with `picking`, `show`'s reading of it: its bytes, its timings, its
/// chunks, its turns at the submit gate, and the reading's input and output.
#[allow(clippy::type_complexity, reason = "what a frame gives, for one test")]
fn rendered(
    name: &str,
    chunking: Chunking,
    picking: bool,
) -> (Vec<u8>, Timings, usize, u64, Option<(Stats, Stats)>) {
    let path = scratch(name);
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    if picking {
        driven.run([show()]);
        driven.picked();
    }
    live(&mut driven).set_chunking(chunking);
    let turns = driven.gpu.gate.turns();
    let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let turns = driven.gpu.gate.turns() - turns;
    let chunks = live(&mut driven).chunks();
    let bytes = frame(&driven.gpu, &events);
    let timings = (events.iter())
        .find_map(|event| match event {
            Event::Frame(frame) => Some(frame.timings.clone()),
            _ => None,
        })
        .expect("a frame");
    let reading = picking.then(|| {
        let picked = driven.picked();
        let parts: Vec<&ToolReading> = (picked.iter())
            .filter_map(|event| match event {
                Event::Picker {
                    reading: Some(reading),
                    ..
                } => reading.tool.as_ref(),
                _ => None,
            })
            .collect();
        let part = parts.last().unwrap_or_else(|| panic!("{picked:?}"));
        (
            part.input.expect("its input"),
            part.output.expect("its output"),
        )
    });
    remove(&path);
    (bytes, timings, chunks, turns, reading)
}

/// The bits of a reading's statistics, compared exactly.
fn bits(stats: &Stats) -> [[u32; 4]; 3] {
    [stats.mean, stats.min, stats.max].map(|v| v.map(f32::to_bits))
}

#[test]
fn a_frame_in_chunks_equals_one_submission_s_each_chunk_a_turn_at_the_gate() {
    let (whole, whole_timings, one, turns, _) = rendered("chunks-whole", Chunking::Whole, false);
    assert_eq!((one, turns), (1, 1));
    // The bound at nothing: every pass a chunk of its own.
    let (cut, timings, chunks, turns, _) = rendered("chunks-cut", Chunking::Bounded(0.0), false);
    assert!(chunks > 1, "{chunks} chunks");
    assert_eq!(turns, chunks as u64, "a turn at the submit gate a chunk");
    // The copy into the display buffer in the last chunk, the frame byte for byte.
    assert!(cut == whole, "the frame in chunks differs");
    // The timestamps' resolve and copy in the last chunk, so every pass is timed as in
    // one submission.
    let names =
        |timings: &Timings| -> Vec<&str> { timings.passes.iter().map(|(name, _)| *name).collect() };
    assert_eq!(names(&timings), names(&whole_timings));
    let timed = |timings: &Timings| -> Vec<bool> {
        (timings.passes.iter())
            .map(|(_, ms)| ms.is_finite())
            .collect()
    };
    assert_eq!(timed(&timings), timed(&whole_timings));
}

#[test]
fn a_tool_picker_reads_in_chunks_what_it_reads_in_one_submission() {
    let (whole, _, _, _, read) = rendered("chunks-picker-whole", Chunking::Whole, true);
    let (cut, _, chunks, _, read_cut) = rendered("chunks-picker-cut", Chunking::Bounded(0.0), true);
    assert!(chunks > 1, "{chunks} chunks");
    assert!(cut == whole, "the frame in chunks differs");
    let ((input, output), (input_cut, output_cut)) =
        (read.expect("a reading"), read_cut.expect("a reading"));
    assert_eq!(bits(&input_cut), bits(&input));
    assert_eq!(bits(&output_cut), bits(&output));
}

/// Through a session's thread: a command that arrives while a frame's chunks are
/// submitted waits in the channel, so the frame is the earlier edit's alone and the
/// command renders in the next frame; and a window's turn at the submit gate comes
/// between the chunks, the next chunk waiting behind it (order, not time).
#[test]
fn a_command_sent_between_a_frame_s_chunks_renders_in_the_next_frame() {
    let gpu = gpu();
    let path = scratch("chunks-between");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 40,
        height: 40,
    });
    let opened = open_live(&session, &wakes, &path);
    if !opened.iter().any(|e| matches!(e, Event::Frame(_))) {
        until(&session, &wakes, |e| matches!(e, Event::Frame(_)));
    }
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldChunks(hold));
    let earlier = session.send(set("brightness", ControlValue::Number(0.5), false));
    begun
        .recv_timeout(TEST_WAIT)
        .expect("the frame's first chunk submitted");
    // Between the chunks the window's turn is its at once.
    let window = gpu.gate.hold();
    let later = session.send(set("brightness", ControlValue::Number(0.9), false));
    permit.send(()).expect("the engine waits");
    // The frame's next chunk waits behind the window's turn, no frame out before it.
    gpu.gate.until_asked(2);
    let mut events = session.events();
    assert!(
        !events.iter().any(|e| matches!(e, Event::Frame(_))),
        "{events:?}"
    );
    drop(window);
    let last = |e: &Event| matches!(e, Event::Frame(frame) if frame.through == later);
    if !events.iter().any(last) {
        events.extend(until(&session, &wakes, last));
    }
    let frames: Vec<&Frame> = (events.iter())
        .filter_map(|event| match event {
            Event::Frame(frame) => Some(frame),
            _ => None,
        })
        .collect();
    let through: Vec<u64> = frames.iter().map(|frame| frame.through).collect();
    assert_eq!(through, [earlier, later], "{events:?}");
    let shown = |frame: &Frame| read_buffer(&gpu, &frame.buffer, frame.buffer.size()).unwrap();
    let (first, second) = (shown(frames[0]), shown(frames[1]));
    drop(session);
    remove(&path);

    // The first frame is the earlier edit's, the second the later's, as a session that
    // took each alone renders them.
    let alone = |value: f32, name: &str| {
        let path = scratch(name);
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let events = driven.run([set("brightness", ControlValue::Number(value), false)]);
        let bytes = frame(&driven.gpu, &events);
        remove(&path);
        bytes
    };
    assert!(
        first == alone(0.5, "chunks-between-earlier"),
        "the earlier edit's"
    );
    assert!(
        second == alone(0.9, "chunks-between-later"),
        "the later edit's"
    );
}
