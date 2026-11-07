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

//! An alert as plain data (`docs/gui.md`, Dialogs and alerts; decision 41): a title, a
//! message and buttons, each with its words and its role. Every confirmation is one of
//! these, answered on a later frame; the platform shows it as the system's alert (`NSAlert`
//! on the Mac, from `apertrust-display`) or, where there is none to use, the design
//! system's own body ([`crate::components::AlertBody`]).
//!
//! The words are handed over as text, translated by the caller: a button never takes the
//! system's own OK or Cancel, which follow the system's language rather than Apertrust's.
//!
//! Like [`crate::i18n`], this module has no egui in it.

/// What a button is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// An ordinary button.
    Default,
    /// The way out: Escape presses it, and Cmd+. on the Mac.
    Cancel,
    /// A button that destroys something the person cannot get back, marked as such.
    Destructive,
}

/// One button: its words and its role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    pub label: String,
    pub role: Role,
}

/// What is wrong with a request, if it is: it cannot be shown as the platforms show an
/// alert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    /// It has no button.
    NoButton,
    /// The default button is none of the buttons.
    BadDefault,
    /// More than one button is the cancel.
    TwoCancels,
}

/// An alert to ask: modal to the window it rises from, the main window or a dialog.
///
/// The buttons are listed in the order of their answers, the one that goes ahead first. The
/// default, which Return presses, is the first unless the request names another
/// ([`Request::default_button`]), and may be the cancel, which is Escape's too; a platform
/// puts the default where it puts it, apart from the order listed. The answer is the index
/// of the button pressed.
///
/// A request may carry a suppression box, "Don't ask again": the platform draws it under the
/// message, unticked, and reports whether it was ticked with the answer. What ticking it
/// means is the caller's, and only for the answer it asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub title: String,
    pub message: String,
    pub buttons: Vec<Button>,
    /// The index of the default button, which Return presses.
    pub default: usize,
    /// The words of the suppression box, if the alert has one.
    pub suppression: Option<String>,
}

impl Request {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Request {
        Request {
            title: title.into(),
            message: message.into(),
            buttons: Vec::new(),
            default: 0,
            suppression: None,
        }
    }

    /// Gives the alert a suppression box with these words, unticked.
    pub fn suppression(mut self, label: impl Into<String>) -> Request {
        self.suppression = Some(label.into());
        self
    }

    /// Adds a button after the others.
    pub fn button(mut self, label: impl Into<String>, role: Role) -> Request {
        self.buttons.push(Button {
            label: label.into(),
            role,
        });
        self
    }

    /// Makes the button at `index` the default, which Return presses, instead of the first.
    pub fn default_button(mut self, index: usize) -> Request {
        self.default = index;
        self
    }

    /// The index of the button Escape presses, if there is a cancel.
    pub fn cancel(&self) -> Option<usize> {
        self.buttons
            .iter()
            .position(|button| button.role == Role::Cancel)
    }

    /// Whether the request can be shown.
    pub fn check(&self) -> Result<(), Invalid> {
        if self.buttons.is_empty() {
            return Err(Invalid::NoButton);
        }
        if self.default >= self.buttons.len() {
            return Err(Invalid::BadDefault);
        }
        let cancels = self
            .buttons
            .iter()
            .filter(|button| button.role == Role::Cancel)
            .count();
        if cancels > 1 {
            return Err(Invalid::TwoCancels);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn confirm() -> Request {
        Request::new("Title", "Message")
            .button("Delete", Role::Destructive)
            .button("Cancel", Role::Cancel)
    }

    #[test]
    fn a_request_holds_its_words_and_its_buttons_in_order() {
        let request = confirm();
        assert_eq!(request.title, "Title");
        assert_eq!(request.message, "Message");
        let buttons: Vec<_> = request
            .buttons
            .iter()
            .map(|b| (b.label.as_str(), b.role))
            .collect();
        assert_eq!(
            buttons,
            [("Delete", Role::Destructive), ("Cancel", Role::Cancel)]
        );
        assert_eq!(request.cancel(), Some(1));
        assert_eq!(request.check(), Ok(()));
    }

    #[test]
    fn a_request_has_a_suppression_box_only_when_given_one() {
        assert_eq!(confirm().suppression, None);
        let request = confirm().suppression("Don't ask again");
        assert_eq!(request.suppression.as_deref(), Some("Don't ask again"));
        assert_eq!(request.check(), Ok(()));
    }

    #[test]
    fn the_default_is_the_first_button_unless_named_and_may_be_the_cancel() {
        assert_eq!(confirm().default, 0);
        let safe = confirm().default_button(1);
        assert_eq!((safe.default, safe.cancel()), (1, Some(1)));
        assert_eq!(safe.check(), Ok(()));
        // A cancel listed first is shown as listed, and is the default as well.
        let first = Request::new("Title", "Message")
            .button("Cancel", Role::Cancel)
            .button("OK", Role::Default);
        assert_eq!(first.check(), Ok(()));
    }

    #[test]
    fn a_request_with_no_cancel_is_valid_and_has_no_escape() {
        let request = Request::new("Title", "Message").button("OK", Role::Default);
        assert_eq!(request.cancel(), None);
        assert_eq!(request.check(), Ok(()));
    }

    #[test]
    fn a_request_the_platforms_could_not_show_is_refused() {
        let none = Request::new("Title", "Message");
        assert_eq!(none.check(), Err(Invalid::NoButton));
        let default = confirm().default_button(2);
        assert_eq!(default.check(), Err(Invalid::BadDefault));
        let two = confirm().button("Stop", Role::Cancel);
        assert_eq!(two.check(), Err(Invalid::TwoCancels));
    }
}
