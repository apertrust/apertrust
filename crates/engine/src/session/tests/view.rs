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
//! its placement; and each frame's place, the whole photo at Fit for now.

use crate::session::history::MERGE_WITHIN;
use crate::view::{View, ViewScale};

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
    let path = scratch("view-edit");
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
    // though the changes are apart enough never to merge.
    let start = Instant::now();
    driven.run_at(start, [set("brightness", ControlValue::Number(0.2), false)]);
    let viewed = driven.run_at(
        start + MERGE_WITHIN * 2,
        [zoom(ViewScale::Factor(2.0), [0.5, 0.5])],
    );
    assert_eq!(kinds(&viewed), ["view"], "{viewed:?}");
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_some());
    assert!(driven.writes().is_empty());
    let done = driven.run_at(
        start + MERGE_WITHIN * 4,
        [set("brightness", ControlValue::Number(0.4), true)],
    );
    assert_eq!(last_history(&done).steps.len(), steps + 1, "{done:?}");

    // On a photo open for viewing only, taken too, writing nothing.
    let only = scratch("view-only");
    let newer =
        brightened_log("0.9.0").replace(r#"apertrust:version="2""#, r#"apertrust:version="3""#);
    std::fs::write(sidecar::path_for(&only), newer).unwrap();
    driven.open(&only);
    assert!(driven.engine.open.as_ref().unwrap().view_only.is_some());
    driven.writes();
    let viewed = driven.run([zoom(ViewScale::Fill, [0.5, 0.5])]);
    assert_eq!(kinds(&viewed), ["view"], "{viewed:?}");
    assert!(driven.writes().is_empty());
    remove(&path);
    remove(&only);
}

#[test]
fn a_view_is_kept_across_photos() {
    let path = scratch("view-kept");
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
    let portrait = scratch("view-kept-portrait").with_file_name("portrait.ARW");
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
    let path = scratch("view-batch");
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
    let path = scratch("view-resize");
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
    let path = scratch("view-edge");
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
    let path = scratch("view-panic");
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
    let path = scratch("view-frame");
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
    // Zoomed, it is still the whole photo at Fit, rendered as before.
    let events = driven.run([
        zoom(ViewScale::Factor(2.0), [0.1, 0.1]),
        set("brightness", ControlValue::Number(0.5), false),
    ]);
    assert_eq!(the_frame(&events).place, fitted);
    let exposure = ("exposure", "exposure", Value::Float(0.7 + 0.5));
    assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &[], &[exposure]));
    remove(&path);
}
