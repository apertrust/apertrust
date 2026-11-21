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

//! The session's view (decision 99): `Command::View`, view state off the edit, one per
//! session and kept across photos, told as `Event::View` once a batch that changed it or
//! its placement; each frame's place; and from 100% on the detail pipeline's frame beside
//! the fitted one, both in one submission, a pan rendering the detail alone, the scopes and
//! the Colour picker reading the fitted frame, and a tiled viewport refusing the zoom; the
//! detail's margin, a pan inside it rendering nothing and one past it updating the detail
//! in place.

use crate::session::history::MERGE_WITHIN;
use crate::settings::registry;
use crate::view::{ANTICIPATE_MOVE, View, ViewScale};

use super::*;

fn zoom(scale: ViewScale, centre: [f32; 2]) -> Command {
    Command::View(View { scale, centre })
}

/// The `View` facts among `events`.
fn views(events: &[Event]) -> Vec<&ViewState> {
    (events.iter())
        .filter_map(|event| match event {
            Event::View(state) => Some(state),
            _ => None,
        })
        .collect()
}

/// The one `View` among `events`.
fn told(events: &[Event]) -> &ViewState {
    match &views(events)[..] {
        [state] => state,
        other => panic!("one View in {events:?}, not {}", other.len()),
    }
}

/// The kinds of `events` the batch's order rules name, in order.
fn kinds(events: &[Event]) -> Vec<&'static str> {
    (events.iter())
        .filter_map(|event| match event {
            Event::Values(_) => Some("values"),
            Event::History(_) => Some("history"),
            Event::View(_) => Some("view"),
            Event::Frame(_) => Some("frame"),
            Event::Failed(_) => Some("failed"),
            Event::Saved { .. } => Some("saved"),
            _ => None,
        })
        .collect()
}

/// The last `History` fact among `events`.
fn last_history(events: &[Event]) -> &History {
    (events.iter().rev())
        .find_map(|event| match event {
            Event::History(history) => Some(history),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a History in {events:?}"))
}

/// The frame among `events`.
fn the_frame(events: &[Event]) -> &Frame {
    (events.iter())
        .find_map(|event| match event {
            Event::Frame(frame) => Some(frame),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a frame in {events:?}"))
}

#[test]
fn a_view_is_no_command_on_the_edit() {
    let path = scratch("zoom-edit");
    let mut driven = Driven::new(&[&Sample]);
    driven.run([
        Command::Open(path.clone()),
        Command::Resize {
            width: 40,
            height: 40,
        },
    ]);
    // Taken while the photo opens, before it is live: never `NotLive`, and no values.
    let opening = driven.run([zoom(ViewScale::Factor(1.0), [0.2, 0.2])]);
    assert!(driven.engine.opening.is_some());
    assert_eq!(kinds(&opening), ["view"], "{opening:?}");
    let opened = driven.settle();
    assert!(driven.engine.opening.is_none());
    let steps = last_history(&opened).steps.len();
    driven.writes();

    // Between a gesture's changes, it leaves the gesture under way: one step, not two,
    // though the changes are apart enough never to merge. Zoomed past 100% and past the
    // region rendered at 100%, it renders the detail's frame.
    let start = Instant::now();
    driven.run_at(start, [set("brightness", ControlValue::Number(0.2), false)]);
    let viewed = driven.run_at(
        start + MERGE_WITHIN * 2,
        [zoom(ViewScale::Factor(2.0), [1.0, 0.5])],
    );
    assert_eq!(kinds(&viewed), ["view", "frame"], "{viewed:?}");
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_some());
    assert!(driven.writes().is_empty());
    let done = driven.run_at(
        start + MERGE_WITHIN * 4,
        [set("brightness", ControlValue::Number(0.4), true)],
    );
    assert_eq!(last_history(&done).steps.len(), steps + 1, "{done:?}");

    // On a photo open for viewing only, taken too, writing nothing.
    let only = scratch("zoom-only");
    let newer =
        brightened_log("0.9.0").replace(r#"apertrust:version="2""#, r#"apertrust:version="3""#);
    std::fs::write(sidecar::path_for(&only), newer).unwrap();
    driven.open(&only);
    assert!(driven.engine.open.as_ref().unwrap().view_only.is_some());
    driven.writes();
    // From the 200% kept across photos to Fill, which renders Fit for now.
    let viewed = driven.run([zoom(ViewScale::Fill, [0.5, 0.5])]);
    assert_eq!(kinds(&viewed), ["view", "frame"], "{viewed:?}");
    assert!(driven.writes().is_empty());
    remove(&path);
    remove(&only);
}

#[test]
fn a_view_is_kept_across_photos() {
    let path = scratch("zoom-kept");
    let mut driven = Driven::new(&[]);
    driven.open(&path);
    let kept = View {
        scale: ViewScale::Factor(1.0),
        centre: [0.3, 0.6],
    };
    let state = *told(&driven.run([Command::View(kept)]));
    // 84 × 62 at 100% in 40 × 40: the centre inside the photo stays.
    assert_eq!(state.view, kept);
    // Another photo, turned upright, 62 × 84: the same view, its centre clamped to it.
    let portrait = scratch("zoom-kept-portrait").with_file_name("portrait.ARW");
    let events = driven.run([Command::Open(portrait.clone())]);
    assert!(views(&events).is_empty(), "{events:?}");
    let events = driven.settle();
    assert_eq!(values(&events).extent, [62, 84]);
    let state = told(&events);
    let placed = crate::view::place([62, 84], (40, 40), &kept);
    assert_eq!(state.view.scale, ViewScale::Factor(1.0));
    assert_eq!(state.view.centre, placed.centre);
    let told = crate::view::place([62, 84], (40, 40), &state.view);
    assert_eq!(state.placement, Some(told));
    assert!(placed.centre[0] > 0.3 && placed.centre[1] == 0.6);
    assert_eq!(driven.engine.view, state.view);
    // Closed, the view stays the session's, and nothing is told of it.
    let closed = driven.run([Command::Close]);
    assert!(views(&closed).is_empty(), "{closed:?}");
    assert_eq!(driven.engine.view, state.view);
    remove(&path);
    remove(&portrait);
}

#[test]
fn a_view_change_sends_the_view_fact_once_a_batch() {
    let path = scratch("zoom-batch");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let events = driven.run([
        zoom(ViewScale::Factor(1.0), [0.3, 0.4]),
        set("brightness", ControlValue::Number(0.3), false),
        zoom(ViewScale::Fill, [0.5, 0.5]),
        zoom(ViewScale::Factor(2.0), [0.6, 0.55]),
    ]);
    // After the batch's values and before its frame, with the last view.
    assert_eq!(kinds(&events), ["values", "view", "frame"], "{events:?}");
    let state = told(&events);
    let last = View {
        scale: ViewScale::Factor(2.0),
        centre: [0.6, 0.55],
    };
    assert_eq!(state.view, last);
    assert_eq!(state.through, driven.engine.applied);
    assert_eq!(
        state.placement,
        Some(crate::view::place([84, 62], (40, 40), &last))
    );
    // A batch with none, and the same view again, send none.
    let events = driven.run([set("brightness", ControlValue::Number(0.4), true)]);
    assert!(views(&events).is_empty(), "{events:?}");
    let events = driven.run([Command::View(last)]);
    assert!(events.is_empty(), "{events:?}");
    remove(&path);
}

#[test]
fn a_resize_that_moves_the_placement_sends_the_view_fact() {
    let path = scratch("zoom-resize");
    let mut driven = Driven::new(&[]);
    driven.open(&path);
    let larger = Command::Resize {
        width: 300,
        height: 200,
    };
    let events = driven.run([larger.clone()]);
    assert_eq!(kinds(&events), ["view", "frame"], "{events:?}");
    let placement = told(&events).placement.expect("a placement");
    // The photo fits at 100%.
    assert_eq!((placement.render, placement.size), (1.0, [84, 62]));
    assert!(driven.run([larger]).is_empty());
    // At Fill, a resize stays Fill.
    driven.run([zoom(ViewScale::Fill, [0.5, 0.5])]);
    let events = driven.run([Command::Resize {
        width: 100,
        height: 100,
    }]);
    let state = told(&events);
    assert_eq!(state.view.scale, ViewScale::Fill);
    let shown = state.placement.expect("a placement").shown;
    assert_eq!(shown, 100.0 / 62.0);
    remove(&path);
}

#[test]
fn a_centre_clamped_at_an_edge_is_told_once() {
    let path = scratch("zoom-edge");
    let mut driven = Driven::new(&[]);
    driven.open(&path);
    // At 100% in 40 × 40, the corner clamps the centre to where the view touches the
    // photo's edges, which an `f32` holds only to its rounding.
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.0, 0.0])]);
    let state = *told(&events);
    let placement = state.placement.expect("a placement");
    assert_eq!(
        placement,
        crate::view::place([84, 62], (40, 40), &state.view)
    );
    // A batch that moves nothing tells nothing.
    let events = driven.run([Command::Scopes(None)]);
    assert!(views(&events).is_empty(), "{events:?}");
    remove(&path);
}

/// A turn that panics loses the events it made, a `View` among them: the session tells the
/// view again, with no photo, and the next photo's placement, though of the same extent.
#[test]
fn a_panicked_turn_tells_the_view_again() {
    let path = scratch("zoom-panic");
    let mut driven = Driven::new(&[]);
    driven.open(&path);
    driven.run([zoom(ViewScale::Factor(1.0), [0.4, 0.5])]);
    let payload: Box<dyn std::any::Any + Send> = Box::new("a bug");
    let taken = driven.engine.applied;
    let events = driven.engine.recover(payload.as_ref(), taken);
    let state = told(&events);
    assert_eq!(
        (state.view.scale, state.placement),
        (ViewScale::Factor(1.0), None)
    );
    assert_eq!(state.through, taken);
    let events = driven.open(&path);
    let state = told(&events);
    assert_eq!(
        state.placement,
        Some(crate::view::place([84, 62], (40, 40), &state.view))
    );
    remove(&path);
}

#[test]
fn a_frame_at_fit_says_its_place() {
    let path = scratch("zoom-frame");
    let mut driven = Driven::new(&[&Sample]);
    let events = driven.open(&path);
    let extent = values(&events).extent;
    let fitted = FramePlace {
        scale: 40.0 / 84.0,
        origin: [0, 0],
        extent,
    };
    assert_eq!(extent, [84, 62]);
    assert_eq!(the_frame(&events).place, fitted);
    // Its pixels are the viewport's as it rendered them before the view.
    assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &[], &[]));
    // At Fill, between Fit and 100%, it is still the whole photo at Fit, rendered as
    // before, until the detail renders below 100%.
    let events = driven.run([
        zoom(ViewScale::Fill, [0.1, 0.1]),
        set("brightness", ControlValue::Number(0.5), false),
    ]);
    assert_eq!(the_frame(&events).place, fitted);
    assert!(the_frame(&events).overview.is_none());
    let exposure = ("exposure", "exposure", Value::Float(0.7 + 0.5));
    assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &[], &[exposure]));
    remove(&path);
}

/// The default pipeline on the synthetic photo with `settings`, as [`direct`] renders it.
fn default_instances(settings: &[(&str, &str, Value)]) -> Vec<Instance<'static>> {
    let ids = [
        "rawprepare",
        "temperature",
        "highlights",
        "demosaic",
        "exposure",
        "colorin",
        "channelmixerrgb",
        "agx",
        "colorout",
    ];
    (ids.iter())
        .map(|id| {
            let operation = apertrust_operations::operation(id).expect("an operation");
            let mut params = ParamValues::new();
            for &(op, name, value) in settings {
                if op == *id {
                    params.set(name, value);
                }
            }
            Instance { operation, params }
        })
        .collect()
}

/// A fresh detail pipeline's frame of `region` at 100% with `settings`, over a fresh
/// fitted viewport's demosaic image, as [`direct`] renders the fitted frame: its bytes.
fn direct_detail(gpu: &GpuContext, settings: &[(&str, &str, Value)], region: [u32; 4]) -> Vec<u8> {
    let instances = default_instances(settings);
    let image = testing::image();
    let input = GpuImage::upload_mosaic(gpu, 96, 72, &testing::mosaic()).unwrap();
    let limits = Limits::of(gpu);
    let pipe = Pipe::Viewport {
        width: 40,
        height: 40,
    };
    let plan = pipeline::plan(&instances, &image, input.desc(), pipe, &limits).unwrap();
    let mut fitted = pollster::block_on(Pipeline::new(gpu, plan, input)).unwrap();
    fitted.render(gpu).unwrap();
    let (source, key) = fitted.demosaiced().expect("demosaic's image");
    let pipe = Pipe::Detail { region, scale: 1.0 };
    let plan = pipeline::plan(&instances, &image, source.desc(), pipe, &limits)
        .unwrap()
        .keyed_on(key);
    let mut detail = pollster::block_on(Pipeline::new(gpu, plan, source)).unwrap();
    detail.render(gpu).unwrap();
    bytemuck::cast_slice(&detail.output().download(gpu).unwrap()).to_vec()
}

/// The bytes of `frame`'s buffer.
fn frame_bytes(gpu: &GpuContext, frame: &Frame) -> Vec<u8> {
    read_buffer(gpu, &frame.buffer, frame.buffer.size()).expect("it reads back")
}

/// The bytes of a frame's overview, the fitted frame beside it.
fn overview(gpu: &GpuContext, frame: &Frame) -> Vec<u8> {
    let overview = frame
        .overview
        .as_ref()
        .expect("the fitted frame beside the detail");
    read_buffer(gpu, &overview.buffer, overview.buffer.size()).expect("it reads back")
}

/// The region the detail renders for `view` of the synthetic photo, 84 × 62, in 40 × 40,
/// with darktable's margin, from no region kept.
fn region_of(scale: ViewScale, centre: [f32; 2]) -> [u32; 4] {
    let view = View { scale, centre };
    let placed = crate::view::zoomed([84, 62], (40, 40), &view).expect("a detail");
    crate::view::rendered(&placed, ANTICIPATE_MOVE, u64::MAX).expect("a region")
}

/// The labels a render gives the passes of `plan`, in dispatch order.
fn labels(plan: &pipeline::Plan) -> Vec<&'static str> {
    (plan.passes())
        .map(|(op, pass)| {
            if pass.entry_point == "main" {
                op.id
            } else {
                pass.entry_point
            }
        })
        .collect()
}

/// The passes `frame`'s render ran, by their labels.
fn ran(frame: &Frame) -> Vec<&'static str> {
    frame.timings.passes.iter().map(|(name, _)| *name).collect()
}

#[test]
fn a_view_at_100_renders_both_pipelines_in_one_submission() {
    let path = scratch("detail-both");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    // 40 × 40 of the 84 × 62 photo round its centre, at scale 1, with its margin: 48 × 48
    // rounded up to the grid, 64, then to the photo's height, against its top left.
    let region = [0, 0, 64, 62];
    assert_eq!(region_of(ViewScale::Factor(1.0), [0.5, 0.5]), region);
    let frame = the_frame(&events);
    let place = FramePlace {
        scale: 1.0,
        origin: [0, 0],
        extent: [84, 62],
    };
    assert_eq!((frame.place, frame.width, frame.height), (place, 64, 62));
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &[], region));

    // A change of the edit while zoomed: both pipelines from exposure, in one submission.
    let turns = driven.gpu.gate.turns();
    let events = driven.run([set("brightness", ControlValue::Number(0.5), false)]);
    assert_eq!(driven.gpu.gate.turns() - turns, 1, "{events:?}");
    let frame = the_frame(&events);
    assert_eq!((frame.place, frame.width, frame.height), (place, 64, 62));
    let passes = ran(frame);
    assert_eq!(passes.iter().filter(|&&name| name == "exposure").count(), 2);
    assert!(!passes.contains(&"region"), "{passes:?}");
    // The frame is a fresh detail pipeline's, its overview a fresh fitted viewport's.
    let exposure = [("exposure", "exposure", Value::Float(0.7 + 0.5))];
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &exposure, region));
    let fitted = frame.overview.as_ref().expect("an overview");
    assert_eq!((fitted.width, fitted.height), (40, 30));
    assert!(overview(&driven.gpu, frame) == direct(&driven.gpu, &[], &exposure));
    // Back at Fit, the fitted frame alone.
    let events = driven.run([zoom(ViewScale::Fit, [0.5, 0.5])]);
    let frame = the_frame(&events);
    assert!(frame.overview.is_none());
    assert_eq!(frame.place, FramePlace::fitted([84, 62], (40, 40)));
    assert!(frame_bytes(&driven.gpu, frame) == direct(&driven.gpu, &[], &exposure));
    remove(&path);
}

#[test]
fn a_view_change_renders_the_detail_alone() {
    let path = scratch("detail-alone");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    // A pan past the margin: one view, one frame, from the detail's copy of its new
    // region, with no pass of the fitted pipeline.
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.8, 0.5])]);
    assert_eq!(kinds(&events), ["view", "frame"], "{events:?}");
    let frame = the_frame(&events);
    let open = driven.engine.open.as_ref().expect("a photo open");
    let detail = open.detail.as_ref().expect("the detail pipeline");
    assert_eq!(ran(frame), labels(detail.pipeline.plan()));
    let region = region_of(ViewScale::Factor(1.0), [0.8, 0.5]);
    assert_eq!(frame.place.origin, [region[0], region[1]]);
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &[], region));
    // The same view again renders nothing, and is answered so: no later render refuses it.
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.8, 0.5])]);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(driven.engine.view_asked, None);
    remove(&path);
}

/// The scopes' readings and the Colour picker's readings that have arrived, waiting for
/// those on their way.
fn measured(driven: &mut Driven) -> (Vec<Reading>, Vec<PickerReading>) {
    let scopes = driven.readings();
    let start = Instant::now();
    let mut picks = Vec::new();
    loop {
        for event in driven.engine.picked() {
            if let Event::Picker {
                reading: Some(reading),
                ..
            } = event
            {
                picks.push(reading);
            }
        }
        if !driven.engine.pickers.pending() {
            return (scopes, picks);
        }
        assert!(start.elapsed() < TEST_WAIT, "no reading");
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn the_scopes_and_the_colour_picker_read_the_overview() {
    let path = scratch("detail-measured");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let colour_picker = Command::Picker(PickerCommand::On {
        picker: PickerRef::ColourPicker,
        area: true,
        keys: PickKeys::default(),
        settings: Vec::new(),
        channel: None,
    });
    driven.run([Command::Scopes(Some(Graphs::default())), colour_picker]);
    measured(&mut driven);
    // The same edit at 100% and at Fit: the same counts and the same sample, each of
    // the whole photo at the fitted scale.
    let brightness = |value| set("brightness", ControlValue::Number(value), false);
    let mut readings = Vec::new();
    for scale in [ViewScale::Factor(1.0), ViewScale::Fit] {
        driven.run([zoom(scale, [0.3, 0.6])]);
        measured(&mut driven);
        driven.run([brightness(0.2)]);
        measured(&mut driven);
        let events = driven.run([brightness(0.5)]);
        assert_eq!(
            the_frame(&events).overview.is_some(),
            scale != ViewScale::Fit
        );
        let (scopes, picks) = measured(&mut driven);
        let ([scope], [pick]) = (&scopes[..], &picks[..]) else {
            panic!("one of each: {scopes:?}, {picks:?}");
        };
        assert_eq!((scope.width, scope.height), (40, 30));
        readings.push((scope.totals, pick.frame));
    }
    assert_eq!(readings[0], readings[1]);
    remove(&path);
}

#[test]
fn a_white_balance_change_at_100_runs_the_raw_stage_once() {
    let path = scratch("detail-white-balance");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let centre = [0.6, 0.4];
    driven.run([zoom(ViewScale::Factor(1.0), centre)]);
    let red = ("temperature", "red", Value::Float(1.6));
    let events = driven.run([Command::Set {
        operation: red.0.to_owned(),
        param: red.1.to_owned(),
        value: red.2,
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let frame = the_frame(&events);
    let passes = ran(frame);
    for raw_stage in ["rawprepare", "temperature", "opposed_mask", "downscale"] {
        let times = passes.iter().filter(|&&name| name == raw_stage).count();
        assert_eq!(times, 1, "{raw_stage} in {passes:?}");
    }
    let region = region_of(ViewScale::Factor(1.0), centre);
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &[red], region));
    assert!(overview(&driven.gpu, frame) == direct(&driven.gpu, &[], &[red]));
    remove(&path);
}

/// The keys of the whole-image results the open photo's detail holds.
fn held(driven: &Driven) -> Vec<(&'static str, Option<u64>)> {
    let open = driven.engine.open.as_ref().expect("a photo open");
    open.detail.as_ref().expect("the detail").results.keys()
}

#[test]
fn a_view_at_100_with_the_tone_equalizer_renders_its_round() {
    // The tone equalizer's mask in a round over the whole image, between the fitted
    // pipeline's passes and the detail's, held by a key with no region in it: a change
    // upstream runs it again, and a pan none (decision 99).
    let path = scratch("detail-round");
    let mut driven = Driven::new(&[&Sample, &apertrust_tools::tone_equalizer::ToneEqualizer]);
    driven.open(&path);
    driven.run([Command::SetControl {
        tool: "toneequal".into(),
        control: "blending".into(),
        value: ControlValue::Number(10.0),
        done: true,
    }]);
    let at_100 = |events: &[Event]| {
        assert!(failures(events).is_empty(), "{events:?}");
        let frame = the_frame(events);
        assert_eq!(frame.place.scale, 1.0, "the detail's frame");
    };
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    at_100(&events);
    let [("toneequal", Some(first))] = held(&driven)[..] else {
        panic!("the mask held: {:?}", held(&driven));
    };
    // Exposure, before the tone equalizer: the round again, into the same buffers.
    let turns = driven.gpu.gate.turns();
    let events = driven.run([set("brightness", ControlValue::Number(0.5), false)]);
    at_100(&events);
    assert!(
        driven.gpu.gate.turns() - turns > 1,
        "the round after the fitted passes"
    );
    let [("toneequal", Some(changed))] = held(&driven)[..] else {
        panic!("the mask held: {:?}", held(&driven));
    };
    assert_ne!(changed, first);
    // A pan past the margin: the region's passes alone, in one submission.
    let turns = driven.gpu.gate.turns();
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.8, 0.5])]);
    at_100(&events);
    assert_eq!(driven.gpu.gate.turns() - turns, 1);
    assert_eq!(held(&driven), [("toneequal", Some(changed))]);
    remove(&path);
}

#[test]
fn going_back_to_100_compiles_nothing() {
    let path = scratch("detail-compiles");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    let compiled = driven.engine.compiled;
    let events = driven.run([zoom(ViewScale::Fit, [0.5, 0.5])]);
    assert!(the_frame(&events).overview.is_none());
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.2, 0.7])]);
    assert!(the_frame(&events).overview.is_some());
    assert_eq!(driven.engine.compiled, compiled);
    remove(&path);
}

#[test]
fn a_tiled_viewport_refuses_the_zoom() {
    let path = scratch("detail-tiled");
    // A budget smaller than demosaic's image, which the viewport demosaics in tiles.
    let mut driven = Driven::within(&[&Sample], 64 << 10);
    driven.open(&path);
    let open = driven.engine.open.as_ref().expect("a photo open");
    let fitted = open.pipeline.as_ref().expect("a pipeline");
    assert!(fitted.plan().tiled.is_some());
    let asked = View {
        scale: ViewScale::Factor(1.0),
        centre: [0.5, 0.5],
    };
    let events = driven.run([Command::View(asked)]);
    // Refused, before the batch's facts, the view the session's all the same, and the
    // frame at Fit.
    assert_eq!(kinds(&events), ["failed", "view", "frame"], "{events:?}");
    let refused = |event: &Event| {
        matches!(event, Event::Failed(SessionError::Refused { command, error })
            if *command == driven.engine.applied && matches!(**error, SessionError::Zoom(_)))
    };
    assert!(events.iter().any(refused), "{events:?}");
    let state = told(&events);
    assert_eq!(state.view, asked);
    assert_eq!(state.placement.map(|placed| placed.render), Some(1.0));
    let frame = the_frame(&events);
    assert_eq!(frame.place, FramePlace::fitted([84, 62], (40, 40)));
    assert!(frame.overview.is_none());
    // An edit renders at Fit, refusing nothing.
    let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(the_frame(&events).place.scale, 40.0 / 84.0);
    // An edit and a pan in one batch: the pan's refusal comes before the values.
    let events = driven.run([
        set("brightness", ControlValue::Number(0.3), true),
        zoom(ViewScale::Factor(1.0), [0.4, 0.5]),
    ]);
    let order: Vec<&str> = (kinds(&events).into_iter())
        .filter(|kind| *kind != "saved")
        .collect();
    assert_eq!(
        order,
        ["failed", "values", "history", "view", "frame"],
        "{events:?}"
    );
    remove(&path);
}

#[test]
fn above_100_the_detail_renders_at_1() {
    let path = scratch("detail-above");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    // At 400% the bounds show 10 × 10 photo pixels: the frame is those at 100%, with
    // their margin rounded up to the grid, which the client enlarges.
    let events = driven.run([zoom(ViewScale::Factor(4.0), [0.5, 0.5])]);
    let frame = the_frame(&events);
    let region = [0, 0, 64, 62];
    assert_eq!(region_of(ViewScale::Factor(4.0), [0.5, 0.5]), region);
    assert_eq!((frame.width, frame.height), (64, 62));
    let place = FramePlace {
        scale: 1.0,
        origin: [0, 0],
        extent: [84, 62],
    };
    assert_eq!(frame.place, place);
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &[], region));
    remove(&path);
}

#[test]
fn a_view_sent_while_the_photo_opens_is_answered_as_it_goes_live() {
    let opening = |driven: &mut Driven, path: &PathBuf| {
        driven.run([
            Command::Open(path.clone()),
            Command::Resize {
                width: 40,
                height: 40,
            },
        ]);
        let asked = driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
        assert_eq!(kinds(&asked), ["view"], "{asked:?}");
        let view = driven.engine.applied;
        (view, driven.settle())
    };
    // In a viewport the detail can read: the photo's first frame is the detail's.
    let path = scratch("detail-opening");
    let mut driven = Driven::new(&[&Sample]);
    let (_, live) = opening(&mut driven, &path);
    assert!(failures(&live).is_empty(), "{live:?}");
    assert_eq!(the_frame(&live).place.scale, 1.0);
    remove(&path);

    // In a tiled one: refused as the photo goes live, before its facts, the frame at Fit.
    let path = scratch("detail-opening-tiled");
    let mut driven = Driven::within(&[&Sample], 64 << 10);
    let (view, live) = opening(&mut driven, &path);
    let refused = (live.iter()).position(|event| {
        matches!(event, Event::Failed(SessionError::Refused { command, error })
            if *command == view && matches!(**error, SessionError::Zoom(_)))
    });
    // The live stage's values, after the metadata stage's.
    let values = (live.iter()).rposition(|event| matches!(event, Event::Values(_)));
    assert!(refused.is_some() && refused < values, "{live:?}");
    assert_eq!(
        the_frame(&live).place,
        FramePlace::fitted([84, 62], (40, 40))
    );
    // Answered once: the next batch refuses nothing.
    let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    remove(&path);
}

/// The detail pipeline's output buffer and its region, from the session.
fn kept(driven: &Driven) -> (wgpu::Buffer, [u32; 4]) {
    let open = driven.engine.open.as_ref().expect("a photo open");
    let detail = open.detail.as_ref().expect("the detail pipeline");
    (detail.pipeline.output().buffer().clone(), detail.region)
}

#[test]
fn a_pan_inside_the_margin_renders_nothing() {
    let path = scratch("detail-pan-inside");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    let (_, region) = kept(&driven);
    let turns = driven.gpu.gate.turns();
    // Twenty small moves left, each inside the region: no frame and no submission, and
    // each batch's view told.
    for step in 1..=20 {
        let centre = [0.5 - step as f32 * 0.01, 0.5 - step as f32 * 0.005];
        let events = driven.run([zoom(ViewScale::Factor(1.0), centre)]);
        assert_eq!(kinds(&events), ["view"], "{step}: {events:?}");
        let state = told(&events);
        let placement = state.placement.expect("a placement");
        assert!(crate::view::covers(region, placement.visible), "{step}");
    }
    assert_eq!(driven.gpu.gate.turns(), turns);
    assert_eq!(kept(&driven).1, region);
    remove(&path);
}

#[test]
fn a_pan_past_the_margin_updates_the_detail_in_place() {
    let path = scratch("detail-pan-past");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    let (buffer, region) = kept(&driven);
    assert_eq!(region, [0, 0, 64, 62]);
    let compiled = driven.engine.compiled;
    // To the right edge: past the region, the same size moved against the edge.
    let events = driven.run([zoom(ViewScale::Factor(1.0), [1.0, 0.5])]);
    assert_eq!(kinds(&events), ["view", "frame"], "{events:?}");
    let frame = the_frame(&events);
    let moved = [20, 0, 64, 62];
    assert_eq!(region_of(ViewScale::Factor(1.0), [1.0, 0.5]), moved);
    assert_eq!(
        (frame.place.origin, frame.width, frame.height),
        ([20, 0], 64, 62)
    );
    // In place: the same buffers, nothing compiled; its pixels a fresh detail's there.
    assert_eq!(kept(&driven), (buffer, moved));
    assert_eq!(driven.engine.compiled, compiled);
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &[], moved));
    remove(&path);
}

#[test]
fn a_scale_change_renders_again_where_the_region_falls_short() {
    let path = scratch("detail-scale");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    // Bounds of 80 × 60: at 400% 20 × 15 visible, a region of 64 × 62; at 100% the whole
    // photo, 80 × 60 with its margin being more than it.
    driven.run([Command::Resize {
        width: 80,
        height: 60,
    }]);
    let events = driven.run([zoom(ViewScale::Factor(4.0), [0.3, 0.5])]);
    let frame = the_frame(&events);
    assert_eq!((frame.width, frame.height), (64, 62));
    // To 100%: the region falls short of the visible rectangle, so the detail renders the
    // new one, at the scale the view renders at, 1, never reusing what it holds.
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.3, 0.5])]);
    let frame = the_frame(&events);
    let whole = [0, 0, 84, 62];
    assert_eq!(
        (frame.place.origin, frame.width, frame.height),
        ([0, 0], 84, 62)
    );
    assert_eq!(kept(&driven).1, whole);
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &[], whole));
    // Back to 400%, rendered at the same scale, 1: the region holds it, and nothing renders.
    let events = driven.run([zoom(ViewScale::Factor(4.0), [0.7, 0.4])]);
    assert_eq!(kinds(&events), ["view"], "{events:?}");
    assert_eq!(kept(&driven).1, whole);
    remove(&path);
}

#[test]
fn an_edit_keeps_the_region() {
    let path = scratch("detail-kept");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
    // A pan inside the region, then an edit: the region the detail rendered, not one round
    // the new centre.
    let events = driven.run([zoom(ViewScale::Factor(1.0), [0.3, 0.5])]);
    assert_eq!(kinds(&events), ["view"], "{events:?}");
    let events = driven.run([set("brightness", ControlValue::Number(0.5), false)]);
    let frame = the_frame(&events);
    let region = [0, 0, 64, 62];
    assert_eq!(
        (frame.place.origin, frame.width, frame.height),
        ([0, 0], 64, 62)
    );
    assert_eq!(kept(&driven).1, region);
    let exposure = [("exposure", "exposure", Value::Float(0.7 + 0.5))];
    assert!(frame_bytes(&driven.gpu, frame) == direct_detail(&driven.gpu, &exposure, region));
    remove(&path);
}

#[test]
fn the_anticipate_setting_reaches_the_session() {
    use apertrust_base::settings::{Overrides, Store};

    let path = scratch("detail-anticipate");
    // Bounds of 60 × 40 at 100%: with darktable's margin 72 wide, rounded up to the grid
    // past the photo, so the whole of its width; with none, the visible 60 rounded up to
    // the grid, 64.
    let overrides = Overrides::read(["viewer.anticipate_move=1.0"], registry::ALL).expect("read");
    let store = Store::nowhere(registry::ALL).with_overrides(&overrides);
    let options = crate::settings::Options::read(&store, &crate::settings::Flags::default());
    assert_eq!(options.anticipate_move, 1.0);
    for (view, width) in [(options.view(), 64), (ViewOptions::default(), 84)] {
        let mut driven = Driven::with(&[&Sample], view);
        driven.open(&path);
        driven.run([Command::Resize {
            width: 60,
            height: 40,
        }]);
        let events = driven.run([zoom(ViewScale::Factor(1.0), [0.5, 0.5])]);
        let frame = the_frame(&events);
        assert_eq!((frame.width, frame.height), (width, 62), "{view:?}");
    }
    remove(&path);
}
