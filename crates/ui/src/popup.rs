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

//! The popup service (`docs/gui.md`, Popups; decision 40): a caller opens a [`Request`], a
//! plain list of entries hung from an anchor, under an id of its choosing, and reads the
//! [`Answer`] on the frame it arrives, a later one, as it would a click. A caller never
//! draws a list itself, and the service knows nothing of actions or the app: an entry is
//! words and a string id.
//!
//! The app draws the open popups once a frame with [`show`], before it reads their
//! answers; their keys are theirs before the frame begins, whatever has the focus.
//!
//! Every popup in the app goes through it, as a plain list with no control inside: the top
//! panel's ☰ and ⋯ and the side panel's » and ⋮ are pull-downs, `Choice`'s list a dropdown.
//!
//! Built so far: the pull-down, drawn from the theme's tokens, which the top panel's ☰
//! opens on Windows and Linux (decision 39), and the dropdown, `Choice`'s list, drawn with
//! darktable's type-to-filter (decision 42). A [`Service`] the app installs shows the
//! requests it takes itself, on the Mac AppKit's menus, and answers them through
//! [`answered`]; the drawn popups show the rest, and every request where none is
//! installed: on Windows and Linux, in the gallery and in the tests. A context menu comes
//! later, as the docs give it.

mod drawn;
mod filter;

use std::sync::Arc;

use egui::{Context, Id, Rect};

/// One line of a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Item(Item),
    /// A line between groups.
    Separator,
    Submenu(Submenu),
}

/// An item that answers with its id when chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    /// Its translated words, shown as given: an `&` is an ampersand.
    pub text: String,
    /// The keys that run it, in the platform's spelling (`Ctrl+O`), shown at its end; the
    /// app's keymap handles them, never the menu.
    pub shortcut: Option<String>,
    /// For an item that turns something on or is the chosen one of a set: whether it is,
    /// shown by a check mark. None for a command.
    pub checked: Option<bool>,
    /// A disabled item is drawn so, reached by the keys and read as unavailable, and
    /// cannot be chosen.
    pub enabled: bool,
    /// What a pointer resting on the item says, translated: why a disabled one cannot be
    /// chosen.
    pub tip: Option<String>,
}

impl Item {
    /// An enabled command; `text` is translated.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Item {
            id: id.into(),
            text: text.into(),
            shortcut: None,
            checked: None,
            enabled: true,
            tip: None,
        }
    }

    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }

    /// Makes it an item that can be checked, checked while `on`.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// What a pointer resting on it says, translated.
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }
}

/// A submenu, opening to the side of its line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submenu {
    /// Unique in its request, for [`Request::expanded`].
    pub id: String,
    /// Its translated words. At a menu's top, an `&` marks its access key, the letter
    /// after it (`&File`), and `&&` is an ampersand; the `&` is never shown. Below the
    /// top, the words are shown as given.
    pub text: String,
    pub entries: Vec<Entry>,
}

impl Submenu {
    pub fn new(id: impl Into<String>, text: impl Into<String>, entries: Vec<Entry>) -> Self {
        Submenu {
            id: id.into(),
            text: text.into(),
            entries,
        }
    }

    /// Its access key, the letter after its `&`, if it has one.
    pub fn access_key(&self) -> Option<char> {
        words(&self.text).1.map(|(_, key)| key)
    }

    /// Its words as shown at a menu's top, without the `&` that marks its access key.
    pub fn shown(&self) -> String {
        words(&self.text).0
    }
}

impl From<Item> for Entry {
    fn from(item: Item) -> Self {
        Entry::Item(item)
    }
}

impl From<Submenu> for Entry {
    fn from(submenu: Submenu) -> Self {
        Entry::Submenu(submenu)
    }
}

/// What kind of popup a request is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A menu from a button, opening under it: the top panel's ☰.
    PullDown,
    /// A list of options for a value, opening under its control with its end edge on the
    /// control's and at least as wide: `Choice`'s. Its entries are items, the chosen one
    /// checked, which the highlight starts on; typed text filters it as darktable's combo
    /// boxes do (decision 42): the options it matches anywhere, whatever the case, show it
    /// in SemiBold and the others at half strength, and a character that matches none is
    /// dropped. Backspace takes the last back; Enter chooses the only match that can be,
    /// and the highlighted option when nothing is typed; ↑ and ↓ move among the options
    /// that can be chosen and match. On the Mac it is AppKit's pop-up menu, the chosen
    /// option over the control, with AppKit's type-select in place of the filter.
    Dropdown,
}

/// A popup to show: its entries, where it hangs from and its name for screen readers.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub kind: Kind,
    /// The control it hangs from, in points.
    pub anchor: Rect,
    /// Where a press is not outside the popup: the anchor, or more of the control it hangs
    /// from, such as a dropdown's whole row, so that a press there reaches the control
    /// and does not dismiss the popup before the control's own click can close it.
    pub inside: Rect,
    /// What screen readers call it, translated.
    pub name: String,
    pub entries: Vec<Entry>,
    /// The submenu at its top shown open as it opens, its first line chosen, by id: Alt
    /// with a menu's access key opens the ☰'s menu at that menu. None opens it at its top.
    pub expanded: Option<String>,
    /// The control that opened it, which the focus goes back to as it closes. None gives
    /// it back to what had the focus as it opened, which a press leaves with nothing.
    pub opener: Option<Id>,
}

impl Request {
    /// A menu opening under `anchor`, at its top.
    pub fn pull_down(anchor: Rect, name: impl Into<String>, entries: Vec<Entry>) -> Self {
        Request {
            kind: Kind::PullDown,
            anchor,
            inside: anchor,
            name: name.into(),
            entries,
            expanded: None,
            opener: None,
        }
    }

    /// A list of `items` for the value of the control at `anchor`, the chosen one checked.
    /// Choosing one answers with its id.
    pub fn dropdown(anchor: Rect, name: impl Into<String>, items: Vec<Item>) -> Self {
        Request {
            kind: Kind::Dropdown,
            anchor,
            inside: anchor,
            name: name.into(),
            entries: items.into_iter().map(Entry::Item).collect(),
            expanded: None,
            opener: None,
        }
    }

    /// A press anywhere in `control`, which holds the anchor, is not outside the popup.
    pub fn inside(mut self, control: Rect) -> Self {
        self.inside = control;
        self
    }

    /// Opens it with the submenu `id`, one at its top, already open.
    pub fn expanded(mut self, id: impl Into<String>) -> Self {
        self.expanded = Some(id.into());
        self
    }

    /// The focus goes back to `control`, the one that opened it, as it closes: a control
    /// pressed with the pointer has no focus to give back otherwise.
    pub fn opened_by(mut self, control: Id) -> Self {
        self.opener = Some(control);
        self
    }

    /// The index of its checked item at its top: a dropdown's chosen option.
    pub fn checked(&self) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| matches!(entry, Entry::Item(item) if item.checked == Some(true)))
    }

    /// The submenu at its top whose access key is `letter`, whatever its case: what Alt
    /// with that letter opens.
    pub fn access_key(&self, letter: char) -> Option<&str> {
        let wanted = lowercase(letter);
        self.entries.iter().find_map(|entry| match entry {
            Entry::Submenu(submenu) if submenu.access_key().map(lowercase) == Some(wanted) => {
                Some(submenu.id.as_str())
            }
            _ => None,
        })
    }
}

/// How a popup closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// An item was chosen, by its id.
    Chosen(String),
    /// It closed with nothing chosen: Escape, Tab, a press outside it, or [`close`].
    Dismissed,
}

/// A popup service that shows some requests itself (decision 40): on the Mac the app's,
/// which hands plain lists to AppKit. The app installs it with [`install`]; the drawn
/// popups show what it does not take.
pub trait Service: Send + Sync {
    /// Whether it shows `request` itself.
    fn takes(&self, request: &Request) -> bool;
    /// Shows `request` as the popup `id`, later if it must, and calls [`answered`] once it
    /// closes. [`open`] calls it inside a frame, holding nothing of egui's.
    fn open(&self, id: Id, request: Request);
    /// The app closed the popup `id` before it answered: forget it if it is still to show.
    /// An answer that comes all the same is dropped.
    fn close(&self, id: Id) {
        let _ = id;
    }
}

/// Installs `service` in `ctx`, in place of any before it: the requests it takes are its
/// own from then on.
pub fn install(ctx: &Context, service: Arc<dyn Service>) {
    drawn::with(ctx, |popups| popups.service = Some(service));
}

/// The installed service's answer for the popup `id` it showed: [`answer`] gives it on
/// the next frame, which this asks for, and the focus goes back to the popup's opener, or
/// what had the focus as it opened, chosen or not, as the window kept the focus while the
/// menu was open. Called between frames, as AppKit's menu answers once its own loop has
/// returned. Nothing for a popup no longer open.
pub fn answered(ctx: &Context, id: Id, answer: Answer) {
    drawn::with(ctx, |popups| popups.answered(id, answer));
    ctx.request_repaint();
}

/// Opens `request` as the popup `id`, in place of any open under that id. A request the
/// installed [`Service`] takes is the service's until it answers. Any other is drawn: it
/// shows from [`show`] on, takes the keyboard focus, and gives it back to its opener, or
/// to what had it, when it closes by the keys or a choice; opened in place of itself, to
/// what had it first. A frame that does not [`show`] a drawn one closes it, so it never
/// holds the keys unseen.
pub fn open(ctx: &Context, id: Id, request: Request) {
    ctx.add_plugin(drawn::Drawn);
    let restore = request.opener.or_else(|| ctx.memory(|m| m.focused()));
    let service = drawn::with(ctx, |popups| popups.service.clone());
    if let Some(service) = service.filter(|service| service.takes(&request)) {
        drawn::with(ctx, |popups| {
            popups.open_native(id, request.anchor, restore)
        });
        service.open(id, request);
        return;
    }
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let pass = ctx.cumulative_pass_nr();
    drawn::with(ctx, |popups| {
        popups.open(id, request, restore, pointer, pass)
    });
    ctx.request_repaint();
}

/// Draws the open popups and takes what the pointer does to them: once a frame, before
/// their answers are read. A second call in the same pass does nothing, so a component
/// that opens a popup, as `Choice` does, may call it to have it drawn and answered in
/// the pass it opens in.
pub fn show(ctx: &Context) {
    drawn::show(ctx);
}

/// Closes the popup `id`, if it is open, with [`Answer::Dismissed`] and the focus given
/// back.
pub fn close(ctx: &Context, id: Id) {
    if let Some(Some(restore)) = shut(ctx, id) {
        ctx.memory_mut(|m| m.request_focus(restore));
    }
}

/// Closes the popup `id`, if it is open, with [`Answer::Dismissed`], leaving the focus
/// where it is: for a popup closing because something else takes the focus, as a dialog
/// an action opened.
pub fn dismiss(ctx: &Context, id: Id) {
    shut(ctx, id);
}

/// Closes `id`, drawn or the service's, with [`Answer::Dismissed`], telling the service;
/// where the focus would go back, if it was open.
fn shut(ctx: &Context, id: Id) -> Option<Option<Id>> {
    let (restore, service) = drawn::with(ctx, |popups| {
        let native = popups.is_native(id);
        let restore = popups.close(id, Answer::Dismissed);
        (restore, popups.service.clone().filter(|_| native))
    });
    if let Some(service) = service {
        service.close(id);
    }
    restore
}

/// The id a pull-down hung from the button `button` opens under, so that the button can
/// tell, from its own id, that its menu is open.
pub fn button_menu(button: Id) -> Id {
    button.with("popup")
}

pub fn is_open(ctx: &Context, id: Id) -> bool {
    anchor(ctx, id).is_some()
}

/// Where the open popup `id` hangs from; None while it is closed.
pub fn anchor(ctx: &Context, id: Id) -> Option<Rect> {
    drawn::with(ctx, |popups| popups.anchor(id))
}

/// How the popup `id` closed, on the frame it did, once: read, it is gone.
pub fn answer(ctx: &Context, id: Id) -> Option<Answer> {
    drawn::with(ctx, |popups| popups.answer(id))
}

/// `text` as shown, without its `&`, and its access key with the key's index among the
/// shown text's characters.
fn words(text: &str) -> (String, Option<(usize, char)>) {
    let mut shown = String::with_capacity(text.len());
    let mut key = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '&' {
            shown.push(c);
            continue;
        }
        match chars.next() {
            Some('&') => shown.push('&'),
            Some(next) => {
                if key.is_none() {
                    key = Some((shown.chars().count(), next));
                }
                shown.push(next);
            }
            None => {}
        }
    }
    (shown, key)
}

fn lowercase(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ampersand_marks_the_access_key_and_two_make_one() {
        assert_eq!(words("&File"), ("File".to_owned(), Some((0, 'F'))));
        assert_eq!(words("Ed&it"), ("Edit".to_owned(), Some((2, 'i'))));
        assert_eq!(words("Copy && Paste"), ("Copy & Paste".to_owned(), None));
        assert_eq!(words("&Довідка"), ("Довідка".to_owned(), Some((0, 'Д'))));
        assert_eq!(words("Help&"), ("Help".to_owned(), None));
    }

    #[test]
    fn a_letter_finds_the_menu_at_the_top_with_its_access_key() {
        let request = Request::pull_down(
            Rect::NOTHING,
            "Menu",
            vec![
                Submenu::new("file", "&File", vec![]).into(),
                Item::new("about", "&About").into(),
                Submenu::new("view", "&View", vec![]).into(),
                Submenu::new("help", "Довідк&а", vec![]).into(),
            ],
        );
        assert_eq!(request.access_key('f'), Some("file"));
        assert_eq!(request.access_key('V'), Some("view"));
        assert_eq!(request.access_key('А'), Some("help"));
        // Only submenus carry access keys.
        assert_eq!(request.access_key('a'), None);
    }
}
