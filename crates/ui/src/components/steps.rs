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

//! What a control's input does, read as it came: each key press with the modifiers held as
//! it was pressed, not those held as the pass ends, in the order the presses came, so a
//! control applies and clamps each before the next; a button's activation with the
//! modifiers of the event that activated it; a drag ended where its button was let go.
//! Where egui's own interaction tells it (a press's origin, a drag's start and stop), a
//! control asks egui instead.

use egui::os::OperatingSystem;
use egui::{Event, InputState, Key, Modifiers, PointerButton, Pos2, Rect};

use super::wheel::key_scale;

/// What one key press asks of a control that moves along a range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Press {
    /// A step up, positive, or down, times what its own modifiers multiply a step by
    /// ([`key_scale`]): → and ←.
    Step(f64),
    /// The range's start: Home.
    Start,
    /// The range's end: End.
    End,
}

/// The presses of → and ←, Home and End in this pass, in order, each consumed and handed
/// to `press`; one with an app's modifiers is left for the app's shortcut.
pub(crate) fn presses(i: &mut InputState, os: OperatingSystem, mut press: impl FnMut(Press)) {
    i.events.retain(|event| {
        let Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        else {
            return true;
        };
        let Some(scale) = key_scale(os, *modifiers) else {
            return true;
        };
        let asked = match key {
            Key::ArrowRight => Press::Step(scale),
            Key::ArrowLeft => Press::Step(-scale),
            Key::Home => Press::Start,
            Key::End => Press::End,
            _ => return true,
        };
        press(asked);
        false
    });
}

/// The modifiers of the event that activated the button at `rect` this pass, the last of
/// them: a primary press on it, or Space or Enter while it had the keyboard, `focused`;
/// those held now where none came, as a hold repeats.
pub(crate) fn activation_modifiers(i: &InputState, rect: Rect, focused: bool) -> Modifiers {
    i.events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                pos,
                modifiers,
            } if rect.contains(*pos) => Some(*modifiers),
            Event::Key {
                key: Key::Space | Key::Enter,
                pressed: true,
                modifiers,
                ..
            } if focused => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(i.modifiers)
}

/// Whether `key` was pressed this pass with no modifiers of its own, as its event says, not
/// as they are held as the pass ends: the first such press consumed.
pub(crate) fn plain(i: &mut InputState, key: Key) -> bool {
    let at = i.events.iter().position(|event| {
        matches!(event, Event::Key { key: k, pressed: true, modifiers, .. }
            if *k == key && modifiers.is_none())
    });
    at.map(|at| i.events.remove(at)).is_some()
}

/// Where the primary button was last let go this pass, though the pointer moved on after.
pub(crate) fn released_at(i: &InputState) -> Option<Pos2> {
    i.events.iter().rev().find_map(|event| match event {
        Event::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            pos,
            ..
        } => Some(*pos),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use egui::{RawInput, pos2, vec2};

    use super::*;

    fn key(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        }
    }

    fn input(events: Vec<Event>) -> InputState {
        let raw = RawInput {
            events,
            ..RawInput::default()
        };
        InputState::default().begin_pass(raw, false, 1.0, egui::InputOptions::default())
    }

    /// Each press in order with its own modifiers, Home and End among the arrows, an app's
    /// modifiers left alone, and other keys untouched. Each case: the presses, then what
    /// was asked and how many events are left.
    #[test]
    fn the_presses_come_in_order_with_their_own_modifiers() {
        let (none, shift, app) = (Modifiers::NONE, Modifiers::SHIFT, Modifiers::ALT);
        let cases: [(Vec<Event>, Vec<Press>, usize); 4] = [
            (
                vec![
                    key(Key::ArrowRight, true, shift),
                    key(Key::ArrowLeft, true, none),
                ],
                vec![Press::Step(10.0), Press::Step(-1.0)],
                0,
            ),
            (
                vec![key(Key::End, true, none), key(Key::Home, true, none)],
                vec![Press::End, Press::Start],
                0,
            ),
            (
                vec![key(Key::ArrowRight, true, app), key(Key::A, true, none)],
                vec![],
                2,
            ),
            (
                vec![
                    key(Key::ArrowRight, false, none),
                    key(Key::ArrowRight, true, none),
                ],
                vec![Press::Step(1.0)],
                1,
            ),
        ];
        for (events, asked, left) in cases {
            let mut i = input(events.clone());
            let mut seen = Vec::new();
            presses(&mut i, OperatingSystem::Windows, |press| seen.push(press));
            assert_eq!(seen, asked, "{events:?}");
            assert_eq!(i.events.len(), left, "{events:?}");
        }
    }

    /// A button's activation takes its own event's modifiers: a press on it, or Space while
    /// it has the keyboard, never a later press elsewhere; held, those held now.
    #[test]
    fn an_activation_takes_its_own_event_s_modifiers() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0));
        let press = |pos, modifiers| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers,
        };
        let shift = Modifiers::SHIFT;
        let i = input(vec![
            key(Key::Space, true, shift),
            press(pos2(100.0, 100.0), Modifiers::NONE),
        ]);
        assert_eq!(activation_modifiers(&i, rect, true), shift);
        let i = input(vec![press(pos2(10.0, 10.0), shift)]);
        assert_eq!(activation_modifiers(&i, rect, false), shift);
        let i = input(vec![key(Key::Space, true, shift)]);
        assert_eq!(activation_modifiers(&i, rect, false), Modifiers::NONE);
        let release = Event::PointerButton {
            pos: pos2(5.0, 6.0),
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        };
        let i = input(vec![release, Event::PointerMoved(pos2(50.0, 60.0))]);
        assert_eq!(released_at(&i), Some(pos2(5.0, 6.0)));
    }

    /// A plain key is one pressed with no modifiers of its own, whatever is held as the
    /// pass ends.
    #[test]
    fn a_plain_key_is_its_own_event_s() {
        let mut i = input(vec![
            key(Key::Delete, true, Modifiers::SHIFT),
            Event::ModifiersChanged(Modifiers::NONE),
        ]);
        assert!(!plain(&mut i, Key::Delete));
        let mut i = input(vec![
            key(Key::Delete, true, Modifiers::NONE),
            Event::ModifiersChanged(Modifiers::SHIFT),
        ]);
        assert!(plain(&mut i, Key::Delete));
        assert!(!plain(&mut i, Key::Delete), "consumed");
    }
}
