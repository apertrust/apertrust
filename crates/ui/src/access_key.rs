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

//! Access keys on Windows and Linux (decision 39; `docs/gui.md`, The menu bar, Sheets): an
//! `&` in translated words marks the letter after it, `&&` an ampersand that is shown, and
//! the letter is underlined while Alt is held. A top-level menu's opens the menu button's
//! menu at that menu; a sheet's footer button's presses it ([`Button`]). The Mac shows
//! no `&` and has none.
//!
//! [`Button`]: crate::components::Button

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Key, Modifiers, Stroke};

use crate::Theme;

/// Takes out of the frame's input the first press of Alt with a letter, and the text it
/// typed, if `find` finds what it is the access key of, and returns what it found. The
/// letter is the one the press typed on the current layout, as ф on a Ukrainian keyboard,
/// which egui reports as its physical key, A; where `find` finds nothing for it, or nothing
/// was typed, the key's own Latin letter, so an English interface's Alt+F opens File on a
/// Cyrillic layout too. Only Alt counts, so AltGr, which egui never sees as Alt, finds
/// nothing. A key's repeat that `find` finds is taken out too, with its text, so a held key
/// types nothing into a field, but returns nothing, acting once. A press `find` finds
/// nothing for is left alone. `find` runs outside egui's lock, so it may read the context.
pub fn pressed<T>(ctx: &egui::Context, mut find: impl FnMut(char) -> Option<T>) -> Option<T> {
    let (at, typed, own, repeat) = ctx.input(|input| {
        input.events.iter().enumerate().find_map(|(i, event)| {
            let egui::Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = *event
            else {
                return None;
            };
            if !(alt_only(modifiers) && is_letter(key)) {
                return None;
            }
            let typed = match input.events.get(i + 1) {
                Some(egui::Event::Text(text)) => text.chars().next(),
                _ => None,
            };
            let own = key.name().chars().next()?;
            Some((i, typed, own, repeat))
        })
    })?;
    let found = typed.and_then(&mut find).or_else(|| find(own))?;
    ctx.input_mut(|input| {
        let end = (at + 1 + usize::from(typed.is_some())).min(input.events.len());
        input.events.drain(at..end);
    });
    (!repeat).then_some(found)
}

/// Whether `a` and `b` are the same letter, whatever their case.
pub fn same(a: char, b: char) -> bool {
    a.to_lowercase().eq(b.to_lowercase())
}

/// `text` as shown, without its `&`, and its access key with the key's index among the
/// shown text's characters.
pub(crate) fn words(text: &str) -> (String, Option<(usize, char)>) {
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

/// `text` in `colour` at the body size, the character at `key`'s index underlined.
pub(crate) fn underlined(
    text: &str,
    key: Option<(usize, char)>,
    t: &Theme,
    colour: Color32,
) -> LayoutJob {
    let plain = TextFormat::simple(FontId::proportional(t.text.body), colour);
    let mut job = LayoutJob::default();
    let Some((at, _)) = key else {
        job.append(text, 0.0, plain);
        return job;
    };
    let split = |n: usize| text.char_indices().nth(n).map_or(text.len(), |(i, _)| i);
    let (start, end) = (split(at), split(at + 1));
    job.append(&text[..start], 0.0, plain.clone());
    job.append(
        &text[start..end],
        0.0,
        TextFormat {
            underline: Stroke::new(t.shape.hairline, colour),
            ..plain.clone()
        },
    );
    job.append(&text[end..], 0.0, plain);
    job
}

fn alt_only(m: Modifiers) -> bool {
    m.alt && !m.shift && !m.ctrl && !m.command && !m.mac_cmd
}

fn is_letter(key: Key) -> bool {
    matches!(key.name().as_bytes(), [b'A'..=b'Z'])
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

    /// What [`pressed`] finds for `events`, with something for each of `letters`, and the
    /// events it leaves.
    fn access(events: Vec<egui::Event>, letters: &str) -> (Option<char>, Vec<egui::Event>) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut found = None;
        let mut left = Vec::new();
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx();
            found = pressed(ctx, |letter| {
                let letter = letter.to_lowercase().next()?;
                letters.contains(letter).then_some(letter)
            });
            left = ctx.input(|i| i.events.clone());
        });
        // No painter takes the font atlas.
        output.textures_delta.clear();
        (found, left)
    }

    fn key_event(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn alt_with_an_access_key_is_read_by_the_letter_it_typed() {
        let text = |t: &str| egui::Event::Text(t.to_owned());
        let alt_f = key_event(Key::F, Modifiers::ALT);
        // Taken, with what it typed, so a focused field types nothing.
        let (found, left) = access(vec![alt_f.clone(), text("f")], "fe");
        assert_eq!(found, Some('f'));
        assert!(left.is_empty(), "{left:?}");
        // Without its text, by the key's own letter.
        assert_eq!(access(vec![alt_f.clone()], "f").0, Some('f'));
        // A Ukrainian layout's ф, on the key egui reports as A.
        let alt_a = key_event(Key::A, Modifiers::ALT);
        assert_eq!(access(vec![alt_a.clone(), text("ф")], "фa").0, Some('ф'));
        // On a Cyrillic layout with an English interface, the key's own Latin letter.
        let (found, left) = access(vec![alt_f.clone(), text("а")], "f");
        assert_eq!(found, Some('f'));
        assert!(left.is_empty(), "{left:?}");
        assert_eq!(access(vec![alt_a, text("ф")], "e").0, None);
        // A letter nothing has is left alone.
        let (found, left) = access(vec![alt_f.clone(), text("f")], "e");
        assert_eq!(found, None);
        assert_eq!(left.len(), 2);
        // Only Alt alone: neither with Ctrl or Shift, nor AltGr, nor a plain letter.
        for modifiers in [
            Modifiers::ALT | Modifiers::COMMAND,
            Modifiers::ALT | Modifiers::SHIFT,
            Modifiers::NONE,
        ] {
            let found = access(vec![key_event(Key::F, modifiers), text("f")], "f").0;
            assert_eq!(found, None, "{modifiers:?}");
        }
        assert_eq!(
            access(vec![key_event(Key::F4, Modifiers::ALT)], "f").0,
            None
        );
    }
}
