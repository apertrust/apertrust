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

//! The migrations of `settings.toml` ([`MIGRATIONS`]), each made on the file's table
//! before it is read and before a write merges into it.

use toml::{Table, Value};

use super::Migration;

/// Every migration, in order from version 1.
pub const MIGRATIONS: &[Migration] = &[Migration {
    from: 1,
    migrate: parts_named_by_role,
}];

/// The window's parts' tables, old and new, and the settings version 1 kept in each
/// (decision 95): only these move, so a key no build declared stays where it is.
const TABLES: [(&str, &str, &[&str]); 3] = [
    ("side_panel", "tools_panel", &["tab_names", "width"]),
    (
        "central_panel",
        "viewer",
        &["margin", "proof_margin", "background"],
    ),
    (
        "grid_panel",
        "browser",
        &[
            "sort",
            "descending",
            "thumbnails",
            "labels",
            "square_thumbnails",
            "position",
            "rows",
            "columns",
        ],
    ),
];

/// The action ids `[keys]` holds by id, old and new.
const ACTIONS: [(&str, &str); 4] = [
    ("show-browse", "show-collections"),
    ("grid-position-left", "browser-position-left"),
    ("grid-position-right", "browser-position-right"),
    ("grid-position-bottom", "browser-position-bottom"),
];

/// Version 1 to 2, the window's parts named by their role (decision 95): each key of the
/// Tools panel's, the Viewer's and the Browser's tables, each action id in `[keys]`, and
/// Browse's entries in `edit.sections` and `edit.groups`, which become Collections'.
fn parts_named_by_role(table: &mut Table) {
    for (old, new, keys) in TABLES {
        for key in keys {
            rename(table, &[old, key], &[new, key]);
        }
        remove_if_empty(table, &[old]);
    }
    for (old, new) in ACTIONS {
        rename(table, &["keys", old], &["keys", new]);
    }
    rename(
        table,
        &["edit", "sections", "browse"],
        &["edit", "sections", "collections"],
    );
    move_entries(
        table,
        &["edit", "groups", "browse"],
        &["edit", "groups", "collections"],
    );
}

/// Moves each entry of the table at the dotted path `from` into the table at `to`, one by
/// one as [`rename`] moves a value, so an entry already at `to` wins and the others join
/// it; `from` goes once it is empty. Nothing moves where `from` is no table.
fn move_entries(table: &mut Table, from: &[&str], to: &[&str]) {
    let keys: Vec<String> = match table_at(table, from) {
        Some(entries) => entries.keys().cloned().collect(),
        None => return,
    };
    for key in &keys {
        let (old, new) = (
            [from, &[key.as_str()]].concat(),
            [to, &[key.as_str()]].concat(),
        );
        rename(table, &old, &new);
    }
    remove_if_empty(table, from);
}

/// Removes the table at the dotted path `path` where it is empty.
fn remove_if_empty(table: &mut Table, path: &[&str]) {
    if let Some((last, parents)) = path.split_last()
        && let Some(parent) = table_at(table, parents)
        && parent
            .get(*last)
            .and_then(Value::as_table)
            .is_some_and(Table::is_empty)
    {
        parent.remove(*last);
    }
}

/// Moves the value at the dotted path `from` to `to`, making the tables on `to`'s path. One
/// already at `to` wins, and the old is dropped, with a line under `-d settings` naming
/// it; a value on `to`'s path that is no table keeps the old where it is.
fn rename(table: &mut Table, from: &[&str], to: &[&str]) {
    let (Some((old, old_parents)), Some((new, new_parents))) = (from.split_last(), to.split_last())
    else {
        return;
    };
    let Some(value) = table_at(table, old_parents).and_then(|parent| parent.remove(*old)) else {
        return;
    };
    let (old_id, new_id) = (from.join("."), to.join("."));
    let Some(parent) = made_table_at(table, new_parents) else {
        tracing::debug!(target: "settings", "{old_id} not moved: {new_id}'s table is something else");
        if let Some(parent) = table_at(table, old_parents) {
            parent.insert((*old).to_owned(), value);
        }
        return;
    };
    if parent.contains_key(*new) {
        tracing::debug!(target: "settings", "{old_id} dropped: {new_id} is set too, and kept");
    } else {
        parent.insert((*new).to_owned(), value);
    }
}

/// The table at `path` in `table`, if every part of it is one.
fn table_at<'a>(table: &'a mut Table, path: &[&str]) -> Option<&'a mut Table> {
    path.iter()
        .try_fold(table, |table, key| table.get_mut(*key)?.as_table_mut())
}

/// The table at `path` in `table`, made where it is missing; None where a part of it is
/// something else.
fn made_table_at<'a>(table: &'a mut Table, path: &[&str]) -> Option<&'a mut Table> {
    path.iter().try_fold(table, |table, key| {
        table
            .entry((*key).to_owned())
            .or_insert_with(|| Table::new().into())
            .as_table_mut()
    })
}

#[cfg(test)]
mod tests {
    use toml::Table;

    use super::parts_named_by_role;

    fn migrated(text: &str) -> Table {
        let mut table: Table = text.parse().expect("TOML");
        parts_named_by_role(&mut table);
        table
    }

    #[test]
    fn every_part_s_key_action_and_entry_moves_with_its_value() {
        let table = migrated(
            "version = 1\n\
             [side_panel]\ntab_names = false\nwidth = 400\n\
             [central_panel]\nmargin = 24\nproof_margin = 0.3\nbackground = \"white\"\n\
             [grid_panel]\nsort = \"file-name\"\ndescending = true\nthumbnails = \"large\"\n\
             labels = false\nsquare_thumbnails = true\nposition = \"bottom\"\nrows = 3\n\
             columns = 2\n\
             [keys]\nshow-browse = [\"B\"]\ngrid-position-left = [\"Ctrl+1\"]\n\
             grid-position-right = [\"Ctrl+2\"]\ngrid-position-bottom = [\"Ctrl+3\"]\n\
             undo = [\"Ctrl+Z\"]\n\
             [edit.sections]\nbrowse = false\nexposure = true\n\
             [edit.groups.browse]\nfolders = false\n",
        );
        let expected: Table = "version = 1\n\
             [tools_panel]\ntab_names = false\nwidth = 400\n\
             [viewer]\nmargin = 24\nproof_margin = 0.3\nbackground = \"white\"\n\
             [browser]\nsort = \"file-name\"\ndescending = true\nthumbnails = \"large\"\n\
             labels = false\nsquare_thumbnails = true\nposition = \"bottom\"\nrows = 3\n\
             columns = 2\n\
             [keys]\nshow-collections = [\"B\"]\nbrowser-position-left = [\"Ctrl+1\"]\n\
             browser-position-right = [\"Ctrl+2\"]\nbrowser-position-bottom = [\"Ctrl+3\"]\n\
             undo = [\"Ctrl+Z\"]\n\
             [edit.sections]\ncollections = false\nexposure = true\n\
             [edit.groups.collections]\nfolders = false\n"
            .parse()
            .expect("TOML");
        assert_eq!(table, expected);
    }

    #[test]
    fn a_new_key_set_too_wins_and_the_old_is_dropped() {
        let table = migrated(
            "[side_panel]\nwidth = 400\n[tools_panel]\nwidth = 300\n\
             [keys]\nshow-browse = [\"B\"]\nshow-collections = [\"C\"]\n\
             [edit.sections]\nbrowse = false\ncollections = true\n",
        );
        assert_eq!(table["tools_panel"]["width"].as_integer(), Some(300));
        assert!(!table.contains_key("side_panel"));
        assert_eq!(table["keys"]["show-collections"][0].as_str(), Some("C"));
        assert!(
            !table["keys"]
                .as_table()
                .unwrap()
                .contains_key("show-browse")
        );
        assert_eq!(
            table["edit"]["sections"]["collections"].as_bool(),
            Some(true)
        );
        assert!(
            !table["edit"]["sections"]
                .as_table()
                .unwrap()
                .contains_key("browse")
        );
    }

    #[test]
    fn browse_s_groups_join_collections_s_each_new_one_winning() {
        let table = migrated(
            "[edit.groups.browse]\nfolders = false\nalbums = true\n\
             [edit.groups.collections]\nalbums = false\nrecents = true\n",
        );
        let expected: Table = "[edit.groups.collections]\nfolders = false\nalbums = false\n\
             recents = true\n"
            .parse()
            .expect("TOML");
        assert_eq!(table["edit"], expected["edit"]);
    }

    #[test]
    fn a_new_table_that_is_something_else_keeps_the_old_key() {
        let table = migrated("viewer = 3\n[central_panel]\nmargin = 24\n");
        assert_eq!(table["central_panel"]["margin"].as_integer(), Some(24));
        assert_eq!(table["viewer"].as_integer(), Some(3));
    }

    #[test]
    fn a_key_no_build_declared_stays_in_its_old_table() {
        let table = migrated(
            "version = 1\n\
             [grid_panel]\nrows = 3\nfuture_density = \"compact\"\n\
             [browser]\nfuture_density = \"roomy\"\n",
        );
        let browser = table["browser"].as_table().expect("a table");
        assert_eq!(browser["rows"].as_integer(), Some(3));
        assert_eq!(browser["future_density"].as_str(), Some("roomy"));
        let old = table["grid_panel"].as_table().expect("kept, not empty");
        assert_eq!(old["future_density"].as_str(), Some("compact"));
        assert!(!old.contains_key("rows"));
    }

    #[test]
    fn a_file_without_the_old_keys_is_left_as_it_is() {
        let text = "scale = 1.5\n[tools_panel]\nwidth = 300\n[keys]\nundo = [\"Ctrl+Z\"]\n";
        assert_eq!(migrated(text), text.parse::<Table>().expect("TOML"));
    }
}
