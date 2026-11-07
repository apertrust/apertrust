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

//! The popup service with a service installed (`docs/gui.md`, Popups; decision 40), as the
//! app installs the Mac's: a recording service takes the requests, nothing is drawn, and
//! its answers reach the component through `popup::answer` on the next frame, the focus
//! back on the control that opened it. A service that takes nothing leaves every popup
//! drawn.

use std::sync::{Arc, Mutex};

use apertrust_ui::components::{Choice, ChoiceAction};
use apertrust_ui::popup::{self, Answer, Kind, Request, Service};
use apertrust_ui::{Theme, install};
use egui::accesskit::Role;
use egui::{Id, Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const OPTIONS: [&str; 3] = ["Daylight", "Black body", "Custom"];
const ROW: &str = "Adaptation illuminant";

/// A service that records what it is asked, taking every request or none.
struct Recording {
    takes: bool,
    opened: Mutex<Vec<(Id, Request)>>,
    closed: Mutex<Vec<Id>>,
}

impl Recording {
    fn new(takes: bool) -> Arc<Recording> {
        Arc::new(Recording {
            takes,
            opened: Mutex::new(Vec::new()),
            closed: Mutex::new(Vec::new()),
        })
    }

    fn opened(&self) -> Vec<(Id, Request)> {
        self.opened.lock().unwrap().clone()
    }
}

impl Service for Recording {
    fn takes(&self, _: &Request) -> bool {
        self.takes
    }

    fn open(&self, id: Id, request: Request) {
        self.opened.lock().unwrap().push((id, request));
    }

    fn close(&self, id: Id) {
        self.closed.lock().unwrap().push(id);
    }
}

struct State {
    selected: usize,
    reported: Vec<ChoiceAction>,
}

/// A choice row, with `service` installed.
fn row(service: Arc<Recording>) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            install(ctx, Theme::default());
            popup::install(ctx, service.clone());
        })
        .build_ui_state(
            |ui, state: &mut State| {
                let action = Choice::new("Illuminant", &OPTIONS, state.selected)
                    .name(ROW)
                    .show(ui);
                if let Some(action) = action {
                    if let ChoiceAction::Set(i) = action {
                        state.selected = i;
                    }
                    state.reported.push(action);
                }
            },
            State {
                selected: 0,
                reported: Vec::new(),
            },
        );
    harness.run();
    harness
}

fn focused(harness: &Harness<'_, State>) -> Option<String> {
    harness
        .query_by(|node| node.is_focused())
        .and_then(|node| node.accesskit_node().label())
}

#[test]
fn a_service_takes_the_request_and_its_answer_arrives_on_a_later_frame() {
    let service = Recording::new(true);
    let mut harness = row(service.clone());
    harness.get_by_role_and_label(Role::ComboBox, ROW).click();
    harness.run();
    // Nothing drawn: the service has it, as asked, a dropdown with the chosen option
    // checked and the row as its opener.
    assert!(harness.query_by_role(Role::Menu).is_none());
    let opened = service.opened();
    assert_eq!(opened.len(), 1);
    let (id, request) = &opened[0];
    assert_eq!(request.kind, Kind::Dropdown);
    assert_eq!(request.checked(), Some(0));
    assert!(request.opener.is_some());
    assert!(popup::is_open(&harness.ctx, *id));
    assert_eq!(
        popup::anchor(&harness.ctx, *id),
        Some(request.anchor),
        "its control is told it is open"
    );
    assert!(harness.state().reported.is_empty());

    // The menu closes with an option chosen, between frames, as AppKit's answers.
    popup::answered(&harness.ctx, *id, Answer::Chosen("1".to_owned()));
    harness.run();
    assert_eq!(harness.state().reported, vec![ChoiceAction::Set(1)]);
    assert_eq!(harness.state().selected, 1);
    assert!(!popup::is_open(&harness.ctx, *id));
    // The focus is back on the row, though a press opened it.
    assert_eq!(focused(&harness).as_deref(), Some(ROW));
    // A second answer for a popup no longer open is dropped.
    popup::answered(&harness.ctx, *id, Answer::Chosen("2".to_owned()));
    harness.run();
    assert_eq!(harness.state().selected, 1);
}

#[test]
fn a_dismissed_service_popup_changes_nothing_and_gives_the_focus_back() {
    let service = Recording::new(true);
    let mut harness = row(service.clone());
    harness.get_by_role_and_label(Role::ComboBox, ROW).click();
    harness.run();
    let (id, _) = service.opened()[0].clone();
    popup::answered(&harness.ctx, id, Answer::Dismissed);
    harness.run();
    assert!(harness.state().reported.is_empty());
    assert_eq!(focused(&harness).as_deref(), Some(ROW));
    // It opens again from the row.
    harness.get_by_role_and_label(Role::ComboBox, ROW).click();
    harness.run();
    assert_eq!(service.opened().len(), 2);
}

#[test]
fn a_service_that_takes_nothing_leaves_the_popup_drawn() {
    let service = Recording::new(false);
    let mut harness = row(service.clone());
    harness.get_by_role_and_label(Role::ComboBox, ROW).click();
    harness.run();
    assert!(service.opened().is_empty());
    assert!(harness.query_by_role(Role::Menu).is_some());
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Black body")
        .click();
    harness.run();
    assert_eq!(harness.state().reported, vec![ChoiceAction::Set(1)]);
}

#[test]
fn closing_a_service_popup_answers_at_once_and_tells_the_service() {
    /// What the next frame does to the popup, and the answers read.
    #[derive(Default)]
    struct Caller {
        open: bool,
        close: bool,
        answers: Vec<Answer>,
    }
    let service = Recording::new(true);
    let id = Id::unique("popup");
    let request = Request::pull_down(Rect::NOTHING, "More actions", Vec::new());
    let ctx_service = service.clone();
    let opening = request.clone();
    let mut harness = Harness::builder()
        .with_context_setup(move |ctx| {
            install(ctx, Theme::default());
            popup::install(ctx, ctx_service.clone());
        })
        .build_ui_state(
            move |ui, caller: &mut Caller| {
                let ctx = ui.ctx();
                if std::mem::take(&mut caller.open) {
                    popup::open(ctx, id, opening.clone());
                }
                if std::mem::take(&mut caller.close) {
                    popup::close(ctx, id);
                }
                caller.answers.extend(popup::answer(ctx, id));
            },
            Caller::default(),
        );
    harness.state_mut().open = true;
    harness.run();
    assert_eq!(service.opened(), vec![(id, request)]);
    assert!(popup::is_open(&harness.ctx, id));
    // The app closes it: answered in that frame, and the service told.
    harness.state_mut().close = true;
    harness.step();
    assert!(!popup::is_open(&harness.ctx, id));
    assert_eq!(*service.closed.lock().unwrap(), vec![id]);
    assert_eq!(harness.state().answers, vec![Answer::Dismissed]);
    // AppKit's answer coming all the same is dropped.
    popup::answered(&harness.ctx, id, Answer::Chosen("export".to_owned()));
    harness.run();
    assert_eq!(harness.state().answers, vec![Answer::Dismissed]);
}
