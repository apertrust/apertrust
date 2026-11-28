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

//! The Import tool's declaration (decisions 85, 86 and 93; `docs/gui.md`, The Import
//! tool): darktable's copy & import module (`src/libs/import.c`), the Import sheet's one
//! tool, living there alone ([`Lives::ImportSheet`]).
//!
//! - **Its groups,** collapsible, in this order: Source, Destination, Naming, Metadata,
//!   Filters and After import; Metadata and Filters have no rows until 20b, drawn disabled
//!   and closed.
//! - **Its text settings are custom rows** naming their `import.*` keys ([`KEYS`]), drawn
//!   by the app with the design system's path and text field rows, as Export's path line
//!   is: a setting row holds a number, a choice or a boolean, never a text. Source's lists
//!   are custom rows too.
//! - **Its other rows are setting rows** with Import's own words (`import-*`) and no key
//!   yet, each a later slice's, drawn disabled at its default, the value 20a1 works with:
//!   Keep original file name on, Open the destination when the import ends, Verify the
//!   copies off. Their settings' own defaults come with their keys (20a2, 20b).
//! - **Its header:** ↺ its own, Default settings; the presets button, drawn disabled until
//!   presets come (20c); ⋯ with no item until Open Folder Instead (20a2).

use crate::{
    ControlKind, Group, GroupKind, Lives, Reset, Row, SettingRow, SettingWords, Tab,
    ToolDefinition, Track, Widget,
};

use super::UtilityDefinition;

/// Its id, `section-import` its title.
pub const ID: &str = "import";

/// The settings its custom rows show, by their keys in full: Import name, Folder and
/// Subfolder. The app binds each to the import settings the library declares.
pub const KEYS: &[(Widget, &str)] = &[
    (Widget::ImportName, "import.job_code"),
    (Widget::ImportFolder, "import.base_folder"),
    (Widget::ImportSubfolder, "import.subfolder"),
];

/// The key its custom row `widget` shows, where it shows one.
pub fn key(widget: Widget) -> Option<&'static str> {
    KEYS.iter()
        .find(|(shown, _)| *shown == widget)
        .map(|(_, key)| *key)
}

/// A setting row of Import's with no key yet, `kind` with `words`.
const fn setting(id: &'static str, kind: ControlKind, words: &'static SettingWords) -> Row {
    Row::Setting(SettingRow {
        id,
        kind,
        decimals: 0,
        track: Track::Plain,
        key: None,
        words: Some(words),
        end: &[],
    })
}

/// `label`'s words, with `options`.
const fn words(label: &'static str, options: &'static [crate::Word]) -> SettingWords {
    SettingWords {
        label,
        full: None,
        promise: None,
        options,
    }
}

const INCLUDE_SUBFOLDERS: SettingWords = words("import-include-subfolders", &[]);
const SELECT_NEW: SettingWords = words("import-select-new-photos", &[]);
const KEEP_ORIGINAL_NAME: SettingWords = words("import-keep-original-name", &[]);
const OPEN_DESTINATION: SettingWords = words(
    "import-open-destination",
    &[
        crate::Word::Message("import-open-when-starts"),
        crate::Word::Message("import-open-when-ends"),
        crate::Word::Message("import-open-no"),
    ],
);
const VERIFY: SettingWords = words("import-verify", &[]);

/// A custom row of Import's own widget.
const fn custom(widget: Widget) -> Row {
    Row::Custom {
        widget,
        controls: &[],
    }
}

/// A collapsible group of Import's.
const fn group(id: &'static str, rows: &'static [Row]) -> Row {
    Row::Group(Group {
        id,
        kind: GroupKind::Collapsible,
        rows,
    })
}

/// Import, the Import sheet's one tool: its ↺ its own, Default settings, and its presets
/// in its header; its page Source, Destination, Naming, Metadata, Filters and After
/// import.
pub const IMPORT: UtilityDefinition = UtilityDefinition {
    tool: ToolDefinition {
        id: ID,
        tabs: &[Tab {
            id: None,
            rows: &[
                group(
                    "source",
                    &[
                        custom(Widget::ImportPlaces),
                        custom(Widget::ImportFolders),
                        // darktable's "recursive directory" and "select only new images".
                        setting(
                            "include_subfolders",
                            ControlKind::Bool { default: false },
                            &INCLUDE_SUBFOLDERS,
                        ),
                        setting(
                            "select_new",
                            ControlKind::Bool { default: true },
                            &SELECT_NEW,
                        ),
                    ],
                ),
                group(
                    "destination",
                    &[
                        custom(Widget::ImportName),
                        custom(Widget::ImportFolder),
                        custom(Widget::ImportSubfolder),
                    ],
                ),
                group(
                    "naming",
                    &[
                        setting(
                            "keep_original_name",
                            ControlKind::Bool { default: true },
                            &KEEP_ORIGINAL_NAME,
                        ),
                        custom(Widget::ImportFileName),
                    ],
                ),
                group("metadata", &[]),
                group("filters", &[]),
                group(
                    "after_import",
                    &[
                        setting(
                            "open_destination",
                            ControlKind::Choice {
                                options: &["when_starts", "when_ends", "no"],
                                default: 1,
                                unported: &[],
                            },
                            &OPEN_DESTINATION,
                        ),
                        setting("verify", ControlKind::Bool { default: false }, &VERIFY),
                    ],
                ),
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::Own,
        presets: true,
        open_at_first: true,
        menus: &[],
        toolbar: &[],
        cursors: &[],
        lives: Lives::ImportSheet,
    },
    measures: None,
    titles: &[
        ("source", "import-group-source"),
        ("destination", "import-group-destination"),
        ("naming", "import-group-naming"),
        ("metadata", "import-group-metadata"),
        ("filters", "import-group-filters"),
        ("after_import", "import-group-after-import"),
    ],
    open: &["source", "destination", "naming", "after_import"],
    rows_shown: false,
    tips: &[],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Import holds together as a utility, lives in the sheet alone, and each of its text
    /// rows names its key once.
    #[test]
    fn import_is_a_utility_of_the_sheet_whose_text_rows_name_their_keys() {
        assert_eq!(IMPORT.check(), Ok(()));
        assert_eq!(IMPORT.tool.lives, Lives::ImportSheet);
        fn customs(rows: &'static [Row], into: &mut Vec<Widget>) {
            for row in rows {
                if let Row::Custom { widget, .. } = row {
                    into.push(*widget);
                }
                customs(row.rows(), into);
            }
        }
        let mut shown = Vec::new();
        customs(IMPORT.tool.tabs[0].rows, &mut shown);
        for (widget, key) in KEYS {
            assert_eq!(shown.iter().filter(|w| *w == widget).count(), 1, "{key}");
            assert!(key.starts_with("import."), "{key}");
        }
        assert_eq!(key(Widget::ImportPlaces), None);
    }
}
