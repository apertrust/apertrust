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

//! A drawn alert in its own modal dialog window, on Linux (decision 41).

use std::cell::Cell;
use std::rc::Rc;

use apertrust_ui::alert::Request;
use apertrust_ui::components::AlertBody;
use eframe::egui::{self, accesskit::Role, os::OperatingSystem};

use super::{Answer, Asks, Dialog, Form, Keys, Look};

#[derive(Debug)]
pub(super) struct AlertDialog {
    request: Request,
    ticked: bool,
    answer: Rc<Cell<Option<Answer>>>,
    wake: egui::Context,
}

impl AlertDialog {
    pub(super) fn new(
        request: &Request,
        answer: Rc<Cell<Option<Answer>>>,
        wake: &egui::Context,
    ) -> AlertDialog {
        AlertDialog {
            request: request.clone(),
            ticked: false,
            answer,
            wake: wake.clone(),
        }
    }
}

impl Dialog for AlertDialog {
    fn id(&self) -> &'static str {
        "alert"
    }

    fn title(&self, _ctx: &egui::Context) -> String {
        self.request.title.clone()
    }

    fn form(&self, _os: OperatingSystem) -> Form {
        Form::Modal
    }

    /// The window's node is the alert dialog, named by the title once.
    fn role(&self) -> Role {
        Role::AlertDialog
    }

    fn look(&self, t: &apertrust_ui::Theme) -> Look {
        Look {
            background: t.colors.raised,
            margin: t.spacing.l,
        }
    }

    fn keys(&self, _os: OperatingSystem) -> Keys {
        Keys {
            escape: true,
            close_window: true,
            cancel: true,
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) -> Asks {
        let answer = AlertBody::new(&self.request, &mut self.ticked)
            .in_alert_window()
            .show(ui);
        let Some(answer) = answer else {
            return Asks::default();
        };
        self.answer
            .set(Some(Answer::of(Some(answer.pressed), answer.ticked)));
        Asks {
            close: true,
            ..Asks::default()
        }
    }
}

impl Drop for AlertDialog {
    fn drop(&mut self) {
        if self.answer.get().is_none() {
            self.answer.set(Some(Answer::Dismissed));
            self.wake.request_repaint();
        }
    }
}
