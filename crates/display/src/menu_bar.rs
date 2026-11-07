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

//! The Mac's menu bar where muda cannot reach (`docs/gui.md`, The menus; decision 39):
//! AppKit's own full-screen item kept out of the app's menus, which have their own, and
//! whether the key event a menu item answers is a key's repeat. Elsewhere these do
//! nothing.

/// Keeps AppKit from adding its own Enter Full Screen item to the app's menus, which have
/// their own: registers `NSFullScreenMenuItemEverywhere` as false for this run, never
/// written to the app's defaults. Call it before the event loop exists, so it holds before
/// AppKit first looks. Nothing elsewhere.
pub fn no_system_full_screen_item() {
    #[cfg(target_os = "macos")]
    macos::no_full_screen_item();
}

/// Whether the event the app is handling is a key held down and repeating: on macOS,
/// NSApp's current event is a key-down with `isARepeat`, as when AppKit performs a menu
/// item's key equivalent for each of a key's repeats. False for any other event, off the
/// main thread, and elsewhere.
pub fn handling_key_repeat() -> bool {
    #[cfg(target_os = "macos")]
    return macos::key_repeat();
    #[cfg(not(target_os = "macos"))]
    false
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSApplication, NSEventType};
    use objc2_foundation::{NSDictionary, NSNumber, NSString, NSUserDefaults};

    pub(super) fn no_full_screen_item() {
        let key = NSString::from_str("NSFullScreenMenuItemEverywhere");
        let no = NSNumber::new_bool(false);
        let value: &AnyObject = &no;
        let defaults: Retained<NSDictionary<NSString, AnyObject>> =
            NSDictionary::from_slices(&[&*key], &[value]);
        // SAFETY: the dictionary maps a string to a number, as defaults are.
        unsafe { NSUserDefaults::standardUserDefaults().registerDefaults(&defaults) };
    }

    pub(super) fn key_repeat() -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        let Some(event) = NSApplication::sharedApplication(mtm).currentEvent() else {
            return false;
        };
        // Only a key event may be asked: AppKit raises an exception for any other.
        event.r#type() == NSEventType::KeyDown && event.isARepeat()
    }
}
