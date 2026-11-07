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

//! The system's alert (`docs/gui.md`, Dialogs and alerts; decision 41): on macOS an
//! `NSAlert` shown as a sheet on the window it rises from, the main window or a dialog's
//! sheet. Elsewhere there is none yet: Windows' task dialog and Linux's own are planned,
//! and [`begin_alert`] says so.
//!
//! The sheet is started from the run loop, as [`crate::begin_sheet`] is, since AppKit
//! animates it in a loop of its own, and the answer comes back through a call that runs on
//! the main thread once the sheet has gone. The words are the caller's, translated: a
//! button never takes the system's own OK or Cancel, and every key is set on its button
//! here, not left to AppKit, which gives Escape to a button titled "Cancel" in English
//! whatever the language.

use raw_window_handle::HasWindowHandle;

use crate::Error;

/// What a button is for; the same three the design system's request has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonRole {
    /// An ordinary button.
    Default,
    /// Escape presses it, and Cmd+. on the Mac.
    Cancel,
    /// Marked as destroying something, drawn as the system draws such a button.
    Destructive,
}

/// One button of an alert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlertButton<'a> {
    pub title: &'a str,
    pub role: ButtonRole,
    /// Return presses it. At most one button is the default, and with none the first is;
    /// it need not be the first listed, and may be the cancel, which Escape presses too.
    pub default: bool,
}

/// The index of the default button: the first flagged, else the first.
fn default_of(buttons: &[AlertButton<'_>]) -> usize {
    buttons.iter().position(|b| b.default).unwrap_or(0)
}

/// The buttons by index in the order they are added to the alert: the default first, which
/// the Mac puts at the end of the row, then the others as they were listed.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn add_order(buttons: &[AlertButton<'_>]) -> Vec<usize> {
    let default = default_of(buttons);
    let others = (0..buttons.len()).filter(|i| *i != default);
    (0..buttons.len())
        .filter(|i| *i == default)
        .chain(others)
        .collect()
}

/// Shows an alert as a sheet on `parent`'s window (macOS), from the run loop once the event
/// under way is handled, and calls `answered` with the index of the button pressed once the
/// sheet has gone, on the main thread; `None` if it ended some other way. The index is that
/// of the list, whatever order the buttons are shown in. The default button, which Return
/// presses, is shown where the platform puts it; the one with the cancel role, if any, is
/// Escape's, the default as well or not.
/// With `suppression`, the alert shows a box with those words (`NSAlert`'s suppression
/// button, unticked), and `answered` is told with the answer whether it was ticked.
/// `parent` must stay until then; a sheet on a window that has one already waits behind it,
/// so a dialog's alert rises from the dialog's own window. Returns false where the platform
/// has no such alert and nothing was done, never calling `answered` then.
pub fn begin_alert(
    parent: &impl HasWindowHandle,
    title: &str,
    message: &str,
    buttons: &[AlertButton<'_>],
    suppression: Option<&str>,
    answered: impl FnOnce(Option<usize>, bool) + 'static,
) -> Result<bool, Error> {
    let handle = parent.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::begin(handle, title, message, buttons, suppression, answered).map(|()| true);
    }
    let _ = (handle, title, message, buttons, suppression, answered);
    Ok(false)
}

/// The index of the button a modal response names, `NSAlertFirstButtonReturn` and up in the
/// order the buttons were added (`order`, [`add_order`]), in the caller's list; none for
/// any other response.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn button_of(response: isize, order: &[usize]) -> Option<usize> {
    let place = usize::try_from(response.checked_sub(FIRST_BUTTON)?).ok()?;
    order.get(place).copied()
}

/// `NSAlertFirstButtonReturn`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const FIRST_BUTTON: isize = 1000;

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::Cell;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, sel};
    use objc2_app_kit::{NSAlert, NSButton, NSControlStateValueOn};
    use objc2_foundation::{NSRect, NSString};
    use raw_window_handle::AppKitWindowHandle;

    use super::{AlertButton, ButtonRole, add_order, button_of, default_of};
    use crate::Error;
    use crate::dialog::macos::{later, window};

    /// Return and Escape as AppKit spells key equivalents.
    const RETURN: &str = "\r";
    const ESCAPE: &str = "\u{1b}";

    pub(super) fn begin(
        parent: AppKitWindowHandle,
        title: &str,
        message: &str,
        buttons: &[AlertButton<'_>],
        suppression: Option<&str>,
        answered: impl FnOnce(Option<usize>, bool) + 'static,
    ) -> Result<(), Error> {
        let mtm = MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let parent = window(parent)?;
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(message));
        // The default is added first, which AppKit shows at the row's end; the others follow
        // as listed. Return is the default's, Escape the cancel's; a button that is both
        // takes Return here and Escape from a helper (`escape_too`).
        let order = add_order(buttons);
        let default = default_of(buttons);
        let mut cancel_too = None;
        for &i in &order {
            let button = &buttons[i];
            let added = alert.addButtonWithTitle(&NSString::from_str(button.title));
            let key = match (i == default, button.role) {
                (true, role) => {
                    if role == ButtonRole::Cancel {
                        cancel_too = Some(added.clone());
                    }
                    RETURN
                }
                (false, ButtonRole::Cancel) => ESCAPE,
                (false, _) => "",
            };
            added.setKeyEquivalent(&NSString::from_str(key));
            added.setHasDestructiveAction(button.role == ButtonRole::Destructive);
        }
        // The box and its title first: `alert.window()`, which the helper touches, can make
        // AppKit lay the alert out early, and the box must be in by then.
        if let Some(label) = suppression {
            alert.setShowsSuppressionButton(true);
            if let Some(button) = alert.suppressionButton() {
                button.setTitle(&NSString::from_str(label));
            }
        }
        if let Some(cancel) = cancel_too {
            escape_too(mtm, &alert, &cancel);
        }
        // Once the buttons, the box and the helper are all in.
        alert.layout();
        later(move || {
            let answered = Cell::new(Some(answered));
            // The block keeps the alert alive until AppKit calls it and lets it go.
            let kept = alert.clone();
            let block = RcBlock::new(move |response: isize| {
                let _ = &kept;
                if let Some(answered) = answered.take() {
                    answered(button_of(response, &order), ticked(&kept));
                }
            });
            alert.beginSheetModalForWindow_completionHandler(&parent, Some(&block));
        })
    }

    /// Makes Escape press `cancel`, which already has Return: a button takes one key
    /// equivalent, and AppKit's own rule for Escape knows only a button titled "Cancel" in
    /// the system's language. A button of no size in the alert's window takes Escape and
    /// clicks `cancel`.
    fn escape_too(mtm: MainThreadMarker, alert: &NSAlert, cancel: &Retained<NSButton>) {
        let helper = NSButton::new(mtm);
        helper.setFrame(NSRect::ZERO);
        helper.setBordered(false);
        helper.setKeyEquivalent(&NSString::from_str(ESCAPE));
        // `performClick:` is a plain action taking the sender.
        unsafe { helper.setTarget(Some(cancel)) };
        unsafe { helper.setAction(Some(sel!(performClick:))) };
        if let Some(content) = alert.window().contentView() {
            content.addSubview(&helper);
        }
    }

    /// Whether the alert's suppression box is ticked; false for an alert with none.
    fn ticked(alert: &NSAlert) -> bool {
        alert
            .suppressionButton()
            .is_some_and(|button| button.state() == NSControlStateValueOn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_names_the_button_added_in_that_place_by_its_place_in_the_list() {
        assert_eq!(button_of(1000, &[0, 1]), Some(0));
        assert_eq!(button_of(1001, &[0, 1]), Some(1));
        // Added in another order than listed: the answer is the list's index.
        assert_eq!(button_of(1000, &[1, 0]), Some(1));
        assert_eq!(button_of(1001, &[1, 0]), Some(0));
        // Not one of the buttons, or not a button's response at all.
        assert_eq!(button_of(1002, &[0, 1]), None);
        assert_eq!(button_of(999, &[0, 1]), None);
        assert_eq!(button_of(-1000, &[0, 1]), None);
        assert_eq!(button_of(isize::MIN, &[0, 1]), None);
        assert_eq!(button_of(1000, &[]), None);
    }

    fn button(title: &str, role: ButtonRole, default: bool) -> AlertButton<'_> {
        AlertButton {
            title,
            role,
            default,
        }
    }

    #[test]
    fn the_default_is_added_first_and_the_others_as_listed() {
        let overwrite = [
            button("Overwrite", ButtonRole::Destructive, false),
            button("Cancel", ButtonRole::Cancel, true),
        ];
        assert_eq!(default_of(&overwrite), 1);
        assert_eq!(add_order(&overwrite), [1, 0]);
        // None flagged: the first, as before.
        let plain = [
            button("Quit", ButtonRole::Destructive, false),
            button("Cancel", ButtonRole::Cancel, false),
        ];
        assert_eq!(default_of(&plain), 0);
        assert_eq!(add_order(&plain), [0, 1]);
        let three = [
            button("A", ButtonRole::Default, false),
            button("B", ButtonRole::Default, true),
            button("C", ButtonRole::Cancel, false),
        ];
        assert_eq!(add_order(&three), [1, 0, 2]);
        assert_eq!(add_order(&[]), Vec::<usize>::new());
    }

    // The alert itself is AppKit's and cannot be shown in a unit test: it needs a window,
    // the main thread and a run loop. It is a hand check on the Mac (`docs/gui.md`, Hand
    // checks on the Mac).
}
