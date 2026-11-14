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

//! The Mac's native popup menus (`docs/gui.md`, Popups; decision 40): a plain list shown as
//! an `NSMenu`, a dropdown's chosen option over its control as the Mac's pop-up buttons
//! place it, a pull-down under its button. Elsewhere there is none, and [`pop_up_menu`]
//! says so: the app draws its popups there.
//!
//! The menu is described in plain data, [`PopUpMenu`], which the app makes from the popup
//! service's request; this crate knows nothing of the design system. Its items answer with
//! their ids and run nothing: an item's keys are shown as its key equivalent, but the app's
//! keymap runs them.
//!
//! AppKit tracks a menu in a loop of its own inside `popUpMenuPositioningItem:atLocation:
//! inView:`, which returns once the menu closes. Called inside winit's event handling, the
//! window would draw nothing until then, and the engine's submit gate would stay held, so
//! [`pop_up_menu`] only schedules the call on the main run loop in its default mode, as
//! [`crate::begin_sheet`] does: it runs once the event under way is handled, outside
//! winit's handler, which meanwhile delivers the window's events with the gate held round
//! each as always (decision 18), and calls back once AppKit's call has returned.

use raw_window_handle::HasWindowHandle;

use crate::Error;

/// One line of a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item(MenuItem),
    /// A line between groups.
    Separator,
    /// A submenu, its words and its lines.
    Submenu {
        title: String,
        entries: Vec<MenuEntry>,
    },
}

/// An item, which answers with its id when chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub id: String,
    /// Its words, translated, shown as given.
    pub title: String,
    /// A disabled item is shown so and cannot be chosen; AppKit's own enabling is off.
    pub enabled: bool,
    /// Shown with a check mark: on, or the chosen option.
    pub on: bool,
    /// What a pointer resting on it says.
    pub tip: Option<String>,
    /// The keys that run it, shown at its end in AppKit's glyphs. The app runs them; the
    /// menu only shows them, and answers with the item's id if they choose it while open.
    pub keys: Option<KeyEquivalent>,
    /// The item AppKit shows in its place while Option is held, an alternate item
    /// (`NSMenuItem.alternate`), answering with its own id; its own `keys` and alternate
    /// are not used: it takes the item's keys with Option added.
    pub alternate: Option<Box<MenuItem>>,
}

/// A key equivalent as AppKit takes one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEquivalent {
    /// The key: a lowercase character, or one of AppKit's characters for a key that types
    /// none, as [`KeyEquivalent::RETURN`] or [`KeyEquivalent::function`]'s.
    pub key: String,
    pub command: bool,
    pub shift: bool,
    pub option: bool,
    pub control: bool,
}

impl KeyEquivalent {
    pub const RETURN: &str = "\r";
    pub const TAB: &str = "\t";
    pub const ESCAPE: &str = "\u{1b}";
    /// The key Macs print Delete on, Backspace elsewhere.
    pub const BACKSPACE: &str = "\u{8}";
    /// Forward Delete.
    pub const DELETE: &str = "\u{7f}";
    pub const UP: &str = "\u{f700}";
    pub const DOWN: &str = "\u{f701}";
    pub const LEFT: &str = "\u{f702}";
    pub const RIGHT: &str = "\u{f703}";
    pub const HOME: &str = "\u{f729}";
    pub const END: &str = "\u{f72b}";
    pub const PAGE_UP: &str = "\u{f72c}";
    pub const PAGE_DOWN: &str = "\u{f72d}";

    /// F`n`, AppKit's `NSF1FunctionKey` and on, for 1 to 35.
    pub fn function(n: u32) -> Option<String> {
        (1..=35)
            .contains(&n)
            .then(|| char::from_u32(0xf704 + n - 1))
            .flatten()
            .map(String::from)
    }
}

/// Whether the menu follows the light or the dark look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

/// A menu to show from a control in a window.
#[derive(Clone, Debug, PartialEq)]
pub struct PopUpMenu {
    pub entries: Vec<MenuEntry>,
    /// The control it hangs from, in points from the window content's top-left corner, as
    /// winit's logical coordinates: its left, top, width and height. The menu is at least
    /// as wide.
    pub anchor: [f64; 4],
    /// The item at its top placed over the control, a dropdown's chosen option, by index
    /// among `entries`. With none, the menu opens under the control, its start on the
    /// control's.
    pub over: Option<usize>,
    pub appearance: Appearance,
    /// The interface's scale: the menu's words are the system's menu font times it.
    pub scale: f64,
}

/// Shows `menu` from its control in `window` (macOS), from the run loop once the event
/// under way is handled, and calls `chosen` once the menu has closed, on the main thread,
/// with the id of the item chosen, or none. `window` must stay until then. Returns false
/// where the platform has no such menus and nothing was done, never calling `chosen` then.
pub fn pop_up_menu(
    window: &impl HasWindowHandle,
    menu: PopUpMenu,
    chosen: impl FnOnce(Option<String>) + 'static,
) -> Result<bool, Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::show(handle, menu, chosen).map(|()| true);
    }
    let _ = (handle, menu, chosen);
    Ok(false)
}

/// The index in the `NSMenu` of the line at `at` among `entries`: each alternate before it
/// is a line of AppKit's menu too, after its item.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn item_index(entries: &[MenuEntry], at: usize) -> usize {
    let alternates = (entries.iter().take(at))
        .filter(|entry| matches!(entry, MenuEntry::Item(item) if item.alternate.is_some()))
        .count();
    at + alternates
}

/// Where AppKit is asked to put the menu, in the view's points from its top-left corner:
/// over the control's top-left, where the positioning item's goes, or under its bottom-left
/// without one.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn location(menu: &PopUpMenu) -> (f64, f64) {
    let [x, y, _, height] = menu.anchor;
    if menu.over.is_some() {
        (x, y)
    } else {
        (x, y + height)
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::Cell;
    use std::rc::Rc;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSControlStateValueOff, NSControlStateValueOn, NSEventModifierFlags, NSFont, NSMenu,
        NSMenuItem, NSView,
    };
    use objc2_foundation::{NSObject, NSPoint, NSString};
    use raw_window_handle::AppKitWindowHandle;

    use super::{Appearance, KeyEquivalent, MenuEntry, MenuItem, PopUpMenu, item_index, location};
    use crate::Error;
    use crate::dialog::macos::{later, window};

    define_class!(
        /// The target of every item of a menu shown: notes the tag of the one chosen, its
        /// index among the menu's ids.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "ApertrustPopUpMenuTarget"]
        #[ivars = Cell<Option<isize>>]
        struct Target;

        impl Target {
            #[unsafe(method(chosen:))]
            fn chosen(&self, item: &NSMenuItem) {
                self.ivars().set(Some(item.tag()));
            }
        }
    );

    impl Target {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Cell::new(None));
            // SAFETY: NSObject's `init`, on an object just allocated with its ivars set.
            unsafe { msg_send![super(this), init] }
        }
    }

    /// What every level of the menu shares.
    struct Look {
        font: Retained<NSFont>,
        appearance: Option<Retained<NSAppearance>>,
    }

    pub(super) fn show(
        handle: AppKitWindowHandle,
        popup: PopUpMenu,
        chosen: impl FnOnce(Option<String>) + 'static,
    ) -> Result<(), Error> {
        let mtm = MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let view: Retained<NSView> = window(handle)?.contentView().ok_or(Error::NoWindow)?;
        // SAFETY: constants AppKit defines.
        let name = unsafe {
            match popup.appearance {
                Appearance::Light => NSAppearanceNameAqua,
                Appearance::Dark => NSAppearanceNameDarkAqua,
            }
        };
        let look = Look {
            font: NSFont::menuFontOfSize(NSFont::systemFontSize() * popup.scale),
            appearance: NSAppearance::appearanceNamed(name),
        };
        let target = Target::new(mtm);
        let mut ids = Vec::new();
        let menu = build(mtm, &popup.entries, &target, &mut ids, &look);
        menu.setMinimumWidth(popup.anchor[2]);
        let over = popup
            .over
            .map(|at| item_index(&popup.entries, at))
            .and_then(|at| isize::try_from(at).ok())
            .and_then(|at| menu.itemAtIndex(at));
        let (x, y) = location(&popup);
        later(move || {
            // winit's view is flipped, its origin at the top-left, as the anchor's is.
            let y = if view.isFlipped() {
                y
            } else {
                view.bounds().size.height - y
            };
            let picked = menu.popUpMenuPositioningItem_atLocation_inView(
                over.as_deref(),
                NSPoint::new(x, y),
                Some(&view),
            );
            let ids = Rc::new(ids);
            let answer = {
                let (target, ids) = (target.clone(), ids.clone());
                move || {
                    let at = target.ivars().get()?;
                    ids.get(usize::try_from(at).ok()?).cloned()
                }
            };
            match answer() {
                // AppKit sends the item's action before the menu's call returns, so the
                // answer is in; were it ever sent after, it would come with the run loop's
                // next turn.
                None if picked => {
                    let _ = later(move || chosen(answer()));
                }
                id => chosen(id),
            }
        })
    }

    /// The menu for `entries`, each item's tag its id's index in `ids`, its action
    /// `target`'s.
    fn build(
        mtm: MainThreadMarker,
        entries: &[MenuEntry],
        target: &Target,
        ids: &mut Vec<String>,
        look: &Look,
    ) -> Retained<NSMenu> {
        let menu = NSMenu::new(mtm);
        // The request's enabled states stand, whatever the items' targets answer.
        menu.setAutoenablesItems(false);
        // SAFETY: a font AppKit made, for the menu's items.
        unsafe { menu.setFont(Some(&look.font)) };
        menu.setAppearance(look.appearance.as_deref());
        let action: &AnyObject = target;
        for entry in entries {
            let line = match entry {
                MenuEntry::Separator => NSMenuItem::separatorItem(mtm),
                MenuEntry::Item(item) => {
                    let line = item_line(mtm, item, action, ids);
                    // The alternate follows its item, with the same key and Option added
                    // to its modifiers, which is how AppKit pairs them; an item with no
                    // keys takes none, so Option alone shows the alternate.
                    let (key, mask) = match &item.keys {
                        Some(keys) => (keys.key.as_str(), modifiers(keys)),
                        None => ("", NSEventModifierFlags::empty()),
                    };
                    line.setKeyEquivalent(&NSString::from_str(key));
                    line.setKeyEquivalentModifierMask(mask);
                    if let Some(alternate) = &item.alternate {
                        menu.addItem(&line);
                        let other = item_line(mtm, alternate, action, ids);
                        other.setKeyEquivalent(&NSString::from_str(key));
                        other.setKeyEquivalentModifierMask(mask | NSEventModifierFlags::Option);
                        other.setAlternate(true);
                        other
                    } else {
                        line
                    }
                }
                MenuEntry::Submenu { title, entries } => {
                    let title = NSString::from_str(title);
                    let line = NSMenuItem::new(mtm);
                    line.setTitle(&title);
                    let submenu = build(mtm, entries, target, ids, look);
                    submenu.setTitle(&title);
                    line.setSubmenu(Some(&submenu));
                    line
                }
            };
            menu.addItem(&line);
        }
        menu
    }

    /// An item's line, its tag its id's index in `ids` and its action `action`'s, with its
    /// words, state and tip; its keys are the caller's to set.
    fn item_line(
        mtm: MainThreadMarker,
        item: &MenuItem,
        action: &AnyObject,
        ids: &mut Vec<String>,
    ) -> Retained<NSMenuItem> {
        let line = NSMenuItem::new(mtm);
        line.setTitle(&NSString::from_str(&item.title));
        line.setTag(isize::try_from(ids.len()).unwrap_or(isize::MAX));
        ids.push(item.id.clone());
        // SAFETY: the target outlives the menu's tracking, which the caller holds it
        // through, and `chosen:` takes the item that sends it.
        unsafe { line.setTarget(Some(action)) };
        unsafe { line.setAction(Some(sel!(chosen:))) };
        line.setEnabled(item.enabled);
        line.setState(if item.on {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        if let Some(tip) = &item.tip {
            line.setToolTip(Some(&NSString::from_str(tip)));
        }
        line
    }

    fn modifiers(keys: &KeyEquivalent) -> NSEventModifierFlags {
        let mut flags = NSEventModifierFlags::empty();
        for (on, flag) in [
            (keys.command, NSEventModifierFlags::Command),
            (keys.shift, NSEventModifierFlags::Shift),
            (keys.option, NSEventModifierFlags::Option),
            (keys.control, NSEventModifierFlags::Control),
        ] {
            if on {
                flags |= flag;
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(over: Option<usize>) -> PopUpMenu {
        PopUpMenu {
            entries: Vec::new(),
            anchor: [10.0, 20.0, 120.0, 24.0],
            over,
            appearance: Appearance::Dark,
            scale: 1.0,
        }
    }

    #[test]
    fn a_dropdown_opens_over_its_control_and_a_pull_down_under_it() {
        assert_eq!(location(&menu(Some(2))), (10.0, 20.0));
        assert_eq!(location(&menu(None)), (10.0, 44.0));
    }

    #[test]
    fn a_line_s_index_counts_the_alternates_before_it() {
        let item = |id: &str, alternate: Option<MenuItem>| {
            MenuEntry::Item(MenuItem {
                id: id.to_owned(),
                title: id.to_owned(),
                enabled: true,
                on: false,
                tip: None,
                keys: None,
                alternate: alternate.map(Box::new),
            })
        };
        let other = || match item("b", None) {
            MenuEntry::Item(item) => item,
            _ => unreachable!(),
        };
        let entries = [
            item("a", Some(other())),
            MenuEntry::Separator,
            item("c", None),
        ];
        assert_eq!(item_index(&entries, 0), 0);
        assert_eq!(item_index(&entries, 1), 2);
        assert_eq!(item_index(&entries, 2), 3);
    }

    #[test]
    fn function_keys_are_appkit_s_characters() {
        assert_eq!(KeyEquivalent::function(1).as_deref(), Some("\u{f704}"));
        assert_eq!(KeyEquivalent::function(12).as_deref(), Some("\u{f70f}"));
        assert_eq!(KeyEquivalent::function(0), None);
        assert_eq!(KeyEquivalent::function(36), None);
    }

    // The menu itself is AppKit's and cannot be shown in a unit test: it needs a window, the
    // main thread and a run loop. It is a hand check on the Mac (`docs/gui.md`, Popups).
}
