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

//! Find Tool… (decision 96; `docs/gui.md`, Finding a tool): a state of the Tools panel, not
//! a tool. While it is open the tab bar is its field and the tab shown's sections give way
//! to the tools found, every tool of the Tools panel, hidden ones included, each once.
//!
//! A tool is found by its name in the interface's language, by its British English name,
//! and by darktable's name for its module and darktable's aliases for it, its title's
//! `.search` attribute in the interface's language, `|` between them. Case and accents
//! count for nothing ([`apertrust_ui::i18n::fold`]). The best come first:
//!
//! 1. the tools whose name starts with the text;
//! 2. those whose name has a word starting with it;
//! 3. those whose British English name, darktable's name or an alias holds it, as
//!    darktable's search matches its modules' names and aliases.
//!
//! Within each, processing tools in pipeline order, then utilities, as the Hidden tab lists
//! them ([`layout::hidden_order`]).

use std::collections::{BTreeMap, BTreeSet};

use apertrust_ui::i18n::fold;
use apertrust_ui::{Locale, Translator};

use crate::layout;

/// The title's attribute that holds a tool's other words.
const SEARCH: &str = "search";

/// The message of `tool`'s title: a processing tool's `tool-<id>-title`, a utility's
/// `section-<id>`.
pub fn title(tool: &str) -> String {
    if layout::processing(tool) {
        apertrust_tools::messages::title(tool)
    } else {
        format!("section-{tool}")
    }
}

/// The words every tool of the Tools panel is found by in one language, folded, in the
/// order ties are listed in.
#[derive(Debug)]
pub struct Index {
    locale: Locale,
    tools: Vec<Words>,
}

#[derive(Debug)]
struct Words {
    id: &'static str,
    /// Its name in the interface's language.
    name: String,
    /// Its British English name, then darktable's name and aliases.
    others: Vec<String>,
}

impl Index {
    /// The tools it finds among, as Find Tool… lists them.
    #[cfg(test)]
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.tools.iter().map(|words| words.id)
    }

    /// The words of every tool, its name and `.search` in `words`' language.
    pub fn new(words: &Translator) -> Index {
        let english = Translator::new(Locale::EnGb);
        let mut ids = layout::known();
        ids.sort_by_key(|id| layout::hidden_order(id));
        let tools = ids
            .into_iter()
            .map(|id| {
                let message = title(id);
                let search = words.attribute(&message, SEARCH).unwrap_or_default();
                let others = std::iter::once(english.text(&message))
                    .chain(search.split('|').map(str::to_owned))
                    .map(|word| fold(word.trim()))
                    .filter(|word| !word.is_empty())
                    .collect();
                Words {
                    id,
                    name: fold(&words.text(&message)),
                    others,
                }
            })
            .collect();
        Index {
            locale: words.locale(),
            tools,
        }
    }

    /// The language its words are in.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// The tools `text` finds, best first; none while it holds nothing but spaces.
    pub fn find(&self, text: &str) -> Vec<&'static str> {
        let text = fold(text.trim());
        if text.is_empty() {
            return Vec::new();
        }
        let mut groups: [Vec<&'static str>; 3] = Default::default();
        for tool in &self.tools {
            let group = if tool.name.starts_with(&text) {
                0
            } else if words(&tool.name).any(|word| word.starts_with(&text)) {
                1
            } else if tool.others.iter().any(|other| other.contains(&text)) {
                2
            } else {
                continue;
            };
            groups[group].push(tool.id);
        }
        groups.into_iter().flatten().collect()
    }
}

/// The words of `name`, between its spaces and punctuation.
fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// Find Tool…'s search while it is open: what was typed, and the tools found, as a tab the
/// Tools panel draws in the place of the tab shown, none pinned.
#[derive(Debug)]
pub struct Search {
    pub text: String,
    /// Whether the field takes the focus this frame, as the search opens.
    pub focus: bool,
    /// Whether an open list or a text field below the field had the keys at the last
    /// frame, so Escape is theirs, closing only that.
    pub inner: bool,
    /// The tools found that were opened in the search: every one is listed closed at
    /// first, whatever its section is in its tab, which stays as it was.
    pub opened: BTreeSet<String>,
    /// The collapsible groups opened or closed in the search, by tool and group, over
    /// their tab's: the tab's come back as they were.
    pub groups: BTreeMap<(String, String), bool>,
    index: Option<Index>,
    found: layout::Tab,
}

/// The id of the tab of tools found, which no tab a workspace shows takes.
pub const FOUND: &str = "find-tool";

impl Search {
    /// A search opening: nothing typed, nothing found, the field taking the focus.
    pub fn new() -> Search {
        Search {
            text: String::new(),
            focus: true,
            inner: false,
            opened: BTreeSet::new(),
            groups: BTreeMap::new(),
            index: None,
            found: layout::Tab {
                id: FOUND.to_owned(),
                title: layout::Title::Own(String::new()),
                icon: apertrust_ui::Icon::Search,
                pinned: Vec::new(),
                order: Vec::new(),
            },
        }
    }

    /// The tools the text finds, found again in `words`' language: its words read again
    /// where the language has changed.
    pub fn refresh(&mut self, words: &Translator) {
        if self.index.as_ref().map(Index::locale) != Some(words.locale()) {
            self.index = Some(Index::new(words));
        }
        if let Some(index) = &self.index {
            self.found.order = index.find(&self.text);
        }
    }

    /// The tools found, as a tab.
    pub fn found(&self) -> &layout::Tab {
        &self.found
    }

    /// Whether anything is typed, so the tools found are listed, or the hint shows.
    pub fn typed(&self) -> bool {
        !self.text.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(locale: Locale, text: &str) -> Vec<&'static str> {
        Index::new(&Translator::new(locale)).find(text)
    }

    #[test]
    fn nothing_typed_finds_nothing() {
        assert!(found(Locale::EnGb, "").is_empty());
        assert!(found(Locale::EnGb, "   ").is_empty());
    }

    #[test]
    fn every_tool_has_other_words_in_every_language() {
        for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
            let words = Translator::new(locale);
            for id in layout::known() {
                let search = words.attribute(&title(id), SEARCH);
                assert!(
                    search.is_some_and(|s| !s.trim().is_empty()),
                    "{}: {id} has no .search",
                    locale.id()
                );
            }
        }
    }

    /// Each language finds a tool by its name, a word of it, its British English name,
    /// darktable's name and an alias, in that order of groups.
    #[test]
    fn each_language_finds_a_tool_by_its_name_english_darktable_s_and_an_alias() {
        // British English: the name's start, a word of it, darktable's name, an alias.
        assert_eq!(found(Locale::EnGb, "expo")[0], "exposure");
        assert!(found(Locale::EnGb, "balance").contains(&"colorbalancergb"));
        assert_eq!(found(Locale::EnGb, "color balance"), ["colorbalancergb"]);
        assert_eq!(found(Locale::EnGb, "dehaze"), ["hazeremoval"]);
        assert_eq!(found(Locale::EnGb, "rotation"), ["flip"]);
        // American English: its own spelling, and British English's.
        assert_eq!(found(Locale::EnUs, "color cal"), ["channelmixerrgb"]);
        assert_eq!(found(Locale::EnUs, "colour cal"), ["channelmixerrgb"]);
        // Ukrainian: the name, the English name, darktable's name and an untranslated
        // alias.
        assert_eq!(found(Locale::Uk, "експоз"), ["exposure"]);
        assert_eq!(found(Locale::Uk, "exposure"), ["exposure"]);
        assert_eq!(found(Locale::Uk, "крива rgb"), ["tone_curve"]);
        assert_eq!(found(Locale::Uk, "smog"), ["hazeremoval"]);
        // Polish: the name, the English name, darktable's name and a translated alias.
        assert_eq!(found(Locale::Pl, "ekspoz"), ["exposure"]);
        assert_eq!(found(Locale::Pl, "vignetting"), ["vignette"]);
        assert_eq!(found(Locale::Pl, "krzywa rgb"), ["tone_curve"]);
        assert_eq!(found(Locale::Pl, "obrót"), ["flip"]);
    }

    #[test]
    fn case_and_accents_count_for_nothing() {
        assert_eq!(found(Locale::Pl, "OBROT"), ["flip"]);
        assert_eq!(found(Locale::Pl, "zmiekczenie"), ["soften"]);
        assert_eq!(found(Locale::EnGb, "ExPoSuRe")[0], "exposure");
    }

    /// Names starting with the text come first, then names with a word starting with it,
    /// then other words; ties in pipeline order, utilities last.
    #[test]
    fn the_best_come_first() {
        let colour = found(Locale::EnGb, "colour");
        // Colour calibration, Colour equalizer and Colour balance RGB start with it, in
        // pipeline order; Input and Output colour profile have a word starting with it.
        assert_eq!(
            colour,
            [
                "channelmixerrgb",
                "colorequal",
                "colorbalancergb",
                "colorin",
                "colorout"
            ]
        );
        // White balance by its name, Colour calibration by darktable's alias after it.
        assert_eq!(
            found(Locale::EnGb, "white balance"),
            ["temperature", "channelmixerrgb"]
        );
        // Utilities after the processing tools: "history" finds History alone.
        assert_eq!(found(Locale::EnGb, "hist"), ["history"]);
    }
}
