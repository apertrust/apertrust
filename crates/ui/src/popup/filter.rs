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

//! The dropdown's type-to-filter, as darktable's combo boxes have it (`_popup_draw` and
//! `_popup_key_press` in `bauhaus.c`, decision 42): what is typed matches an option
//! anywhere in its words, whatever the case, and a character that would match no option
//! is dropped.

use std::ops::Range;

use super::{Entry, Item};

/// Where `typed` matches in `text`, as a byte range of `text`, whatever the case; the
/// first place if it matches more than once. Nothing typed matches at the start, with
/// nothing in it.
pub(super) fn find(text: &str, typed: &str) -> Option<Range<usize>> {
    let wanted: Vec<char> = typed.chars().flat_map(char::to_lowercase).collect();
    if wanted.is_empty() {
        return Some(0..0);
    }
    // The text folded the same way, each folded character with the byte range of the
    // character of `text` it came from, since folding can change a character's length.
    let folded: Vec<(char, Range<usize>)> = text
        .char_indices()
        .flat_map(|(at, c)| {
            let end = at + c.len_utf8();
            c.to_lowercase().map(move |lower| (lower, at..end))
        })
        .collect();
    (0..folded.len())
        .find(|&start| {
            folded[start..]
                .iter()
                .map(|(c, _)| c)
                .take(wanted.len())
                .eq(wanted.iter())
        })
        .map(|start| folded[start].1.start..folded[start + wanted.len() - 1].1.end)
}

/// Whether `entry` is an item `typed` matches.
pub(super) fn matches(entry: &Entry, typed: &str) -> bool {
    matches!(entry, Entry::Item(item) if find(&item.text, typed).is_some())
}

/// Whether the item can be chosen and `typed` matches it: what the arrow keys visit.
pub(super) fn reachable(entry: &Entry, typed: &str) -> bool {
    matches!(entry, Entry::Item(Item { enabled: true, .. })) && matches(entry, typed)
}

/// `typed` with `c` added if some option that can be chosen matches the result; otherwise
/// unchanged. As darktable's `_popup_draw` has it, an insensitive entry never counts.
pub(super) fn typed(entries: &[Entry], typed: &str, c: char) -> String {
    let mut tried = typed.to_owned();
    tried.push(c);
    if entries.iter().any(|entry| reachable(entry, &tried)) {
        tried
    } else {
        typed.to_owned()
    }
}

/// The option after `from` among those `reachable`, wrapping round, or the one before it
/// when `by` is negative; from one that is not among them, the next or previous in the
/// list. None when none is reachable.
pub(super) fn step(
    entries: &[Entry],
    typed: &str,
    from: Option<usize>,
    by: isize,
) -> Option<usize> {
    let reach: Vec<usize> = (0..entries.len())
        .filter(|&i| reachable(&entries[i], typed))
        .collect();
    let (first, last) = (*reach.first()?, *reach.last()?);
    Some(if by > 0 {
        reach.into_iter().find(|&i| Some(i) > from).unwrap_or(first)
    } else {
        reach
            .into_iter()
            .rev()
            .find(|&i| Some(i) < from)
            .unwrap_or(last)
    })
}

/// The only option that can be chosen and that `typed` matches, if there is exactly one:
/// what Enter takes.
pub(super) fn only_choice<'a>(entries: &'a [Entry], typed: &str) -> Option<&'a Item> {
    let mut choices = entries.iter().filter_map(|entry| match entry {
        Entry::Item(item) if reachable(entry, typed) => Some(item),
        _ => None,
    });
    let only = choices.next()?;
    choices.next().is_none().then_some(only)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry> {
        vec![
            Item::new("0", "Daylight").into(),
            Item::new("1", "Black body").into(),
            Item::new("2", "Fluorescent F11").enabled(false).into(),
            Item::new("3", "Custom").into(),
        ]
    }

    #[test]
    fn text_matches_anywhere_in_the_words_whatever_the_case() {
        assert_eq!(find("Black body", "ack"), Some(2..5));
        assert_eq!(find("Black body", "BODY"), Some(6..10));
        assert_eq!(find("Black body", "k b"), Some(4..7));
        assert_eq!(find("Daylight", "x"), None);
        // Nothing typed matches everything, at the start.
        assert_eq!(find("Daylight", ""), Some(0..0));
        // The first place, and a match as long as the words.
        assert_eq!(find("banana", "an"), Some(1..3));
        assert_eq!(find("Custom", "custom"), Some(0..6));
        assert_eq!(find("Custom", "customs"), None);
    }

    #[test]
    fn a_match_is_in_the_text_s_own_bytes_in_any_script() {
        assert_eq!(find("Білий баланс", "БАЛ"), Some(11..17));
        assert_eq!(find("Światło dzienne", "dzi"), Some(10..13));
        // A character whose lowercase is longer, matched as a whole.
        assert_eq!(find("İstanbul", "i\u{307}s"), Some(0..3));
    }

    #[test]
    fn a_character_that_matches_nothing_is_dropped() {
        let entries = entries();
        assert_eq!(typed(&entries, "", 'b'), "b");
        assert_eq!(typed(&entries, "b", 'L'), "bL");
        assert_eq!(typed(&entries, "bl", 'z'), "bl");
        assert_eq!(typed(&entries, "", 'q'), "");
        // A disabled option never counts: "f" is only in Fluorescent, which cannot be chosen.
        assert_eq!(typed(&entries, "", 'f'), "");
        assert_eq!(typed(&entries, "", 'c'), "c");
    }

    #[test]
    fn the_arrows_visit_the_options_that_match_and_can_be_chosen() {
        let entries = entries();
        // Nothing typed: every enabled option, wrapping, the disabled one passed over.
        assert_eq!(step(&entries, "", Some(1), 1), Some(3));
        assert_eq!(step(&entries, "", Some(3), 1), Some(0));
        assert_eq!(step(&entries, "", Some(0), -1), Some(3));
        assert_eq!(step(&entries, "", None, 1), Some(0));
        assert_eq!(step(&entries, "", None, -1), Some(3));
        // From the disabled option, the next and the previous around it.
        assert_eq!(step(&entries, "", Some(2), 1), Some(3));
        assert_eq!(step(&entries, "", Some(2), -1), Some(1));
        // Typed: only the matches. "o" is in Black body, Fluorescent and Custom.
        assert_eq!(step(&entries, "o", Some(1), 1), Some(3));
        assert_eq!(step(&entries, "o", Some(3), 1), Some(1));
        // None match and can be chosen: nowhere to go.
        assert_eq!(step(&entries, "fl", Some(1), 1), None);
    }

    #[test]
    fn enter_takes_the_only_match_that_can_be_chosen() {
        let entries = entries();
        let id = |typed| only_choice(&entries, typed).map(|item| item.id.as_str());
        assert_eq!(id("bla"), Some("1"));
        // Two match.
        assert_eq!(id("o"), None);
        // Nothing typed matches all of them.
        assert_eq!(id(""), None);
        // The only match is disabled.
        assert_eq!(id("fluor"), None);
        assert_eq!(id("cus"), Some("3"));
        // Of "o"'s three, the disabled one never counts.
        let two = &entries[..3];
        assert_eq!(
            only_choice(two, "o").map(|item| item.id.as_str()),
            Some("1")
        );
    }
}
