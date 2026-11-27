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

//! The live frame in bounded submissions through the session (decision 112;
//! `docs/engine.md`, The live frame in bounded submissions): at Fit, a frame in chunks
//! equal to the one submission's, each chunk a turn at the submit gate, the copy and the
//! timestamps in the last; a tool picker's reading equal to the one submission's; and a
//! command sent between a frame's chunks rendered in the next frame, with the window's
//! turn at the gate between the chunks. In the zoomed view (decision 99), a change's
//! detail chunks first and the fitted frame's in the next frame, and where the two join
//! the fitted chunks first, one frame, every fitted chunk done before a round's first tile;
//! `view.rs` checks its frames in chunks.

use super::*;
use crate::pipeline::Chunking;
use crate::view::{View, ViewScale};

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

/// The submit gate's turns the last render of the open photo's fitted pipeline and of its
/// detail each took, a turn a chunk.
fn chunk_turns(driven: &Driven) -> (Vec<u64>, Vec<u64>) {
    let open = driven.engine.open.as_ref().expect("a photo open");
    let fitted = open.pipeline.as_ref().expect("its pipeline").chunk_turns();
    let detail = open
        .detail
        .as_ref()
        .expect("the detail")
        .pipeline
        .chunk_turns();
    (fitted.to_vec(), detail.to_vec())
}

/// The `n` turns at the submit gate after the `before`th, one after another.
fn after(before: u64, n: usize) -> Vec<u64> {
    (before + 1..=before + n as u64).collect()
}

/// Zoomed to 100%, every pass a chunk of its own: a change's frame is the detail's chunks
/// alone, one after another, and the next turn's the fitted frame's after them; once the
/// fitted frame has followed a drag for `OVERVIEW_LAG`, one frame takes the fitted
/// chunks first, then the detail's, and nothing else.
#[test]
fn zoomed_the_detail_s_chunks_come_first_and_the_fitted_frame_s_after_them() {
    let path = scratch("chunks-zoomed");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.engine.chunking = Some(Chunking::Bounded(0.0));
    driven.run([Command::View(View {
        scale: ViewScale::Factor(1.0),
        centre: [0.5, 0.5],
    })]);
    let brightness = |value| set("brightness", ControlValue::Number(value), false);
    let frame = |events: &[Event]| {
        (events.iter())
            .find_map(|event| match event {
                Event::Frame(frame) => Some(frame.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("a frame: {events:?}"))
    };

    // A change: the detail's chunks, the frame's every turn.
    let before = driven.gpu.gate.turns();
    let events = driven.run([brightness(0.3)]);
    assert!(frame(&events).overview.is_none(), "the detail first");
    let (_, detail) = chunk_turns(&driven);
    assert!(detail.len() > 1, "{detail:?}");
    assert_eq!(detail, after(before, detail.len()));
    assert_eq!(driven.gpu.gate.turns(), before + detail.len() as u64);
    // The next turn: the fitted frame's chunks, after the detail's, its every turn.
    let before = driven.gpu.gate.turns();
    let events = driven.engine.turn(Vec::new(), None);
    assert!(frame(&events).overview.is_some(), "the fitted frame");
    let (fitted, _) = chunk_turns(&driven);
    assert!(fitted.len() > 1, "{fitted:?}");
    assert_eq!(fitted, after(before, fitted.len()));
    assert_eq!(driven.gpu.gate.turns(), before + fitted.len() as u64);

    // A detail frame answering a command sent the lag ago, then the drag's next command:
    // the fitted chunks first, then the detail's, one frame.
    let long_ago = Instant::now() - OVERVIEW_LAG;
    let events = driven.run_at(long_ago, [brightness(0.5)]);
    assert!(frame(&events).overview.is_none(), "the detail first");
    let before = driven.gpu.gate.turns();
    let events = driven.run([brightness(0.6)]);
    assert!(frame(&events).overview.is_some(), "both in one frame");
    let (fitted, detail) = chunk_turns(&driven);
    assert!(
        fitted.len() > 1 && detail.len() > 1,
        "{fitted:?}, {detail:?}"
    );
    assert_eq!(fitted, after(before, fitted.len()));
    assert_eq!(detail, after(before + fitted.len() as u64, detail.len()));
    let both = (fitted.len() + detail.len()) as u64;
    assert_eq!(driven.gpu.gate.turns(), before + both);
    remove(&path);
}

/// Zoomed to 100% with the tone equalizer, whose mask is a round over the whole image: a
/// frame that renders the fitted passes, every one a chunk of its own, and the round has
/// every fitted chunk done before the round's first tile, a submission outside the two in
/// flight, so no third is ever queued. Order, not time: the chunks stay tracked until
/// waited for, however fast the GPU.
#[test]
fn zoomed_a_round_waits_for_the_fitted_chunks() {
    static TOOLS: [&dyn ProcessingTool; 2] =
        [&Sample, &apertrust_tools::tone_equalizer::ToneEqualizer];
    let path = scratch("chunks-round");
    let mut driven = Driven::new(&TOOLS);
    driven.open(&path);
    driven.run([Command::SetControl {
        tool: "toneequal".into(),
        control: "blending".into(),
        value: ControlValue::Number(10.0),
        done: true,
    }]);
    driven.engine.chunking = Some(Chunking::Bounded(0.0));
    driven.run([Command::View(View {
        scale: ViewScale::Factor(1.0),
        centre: [0.5, 0.5],
    })]);
    // Exposure, before the tone equalizer, each change done: the round again each time;
    // the second, once the fitted frame has waited `OVERVIEW_LAG`, with the fitted passes.
    let brightness = |value| set("brightness", ControlValue::Number(value), true);
    let long_ago = Instant::now() - OVERVIEW_LAG;
    driven.run_at(long_ago, [brightness(0.3)]);
    driven.engine.in_flight_at_rounds.clear();
    let events = driven.run([brightness(0.6)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let joined = (events.iter()).any(|e| matches!(e, Event::Frame(f) if f.overview.is_some()));
    assert!(joined, "both in one frame: {events:?}");
    let open = driven.engine.open.as_ref().expect("a photo open");
    let fitted = open.pipeline.as_ref().expect("its pipeline").chunks();
    assert!(fitted > 2, "{fitted} fitted chunks");
    assert_eq!(
        driven.engine.in_flight_at_rounds,
        [0],
        "chunks tracked at the round"
    );
    remove(&path);
}
