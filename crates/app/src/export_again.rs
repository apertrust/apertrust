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

//! File › Export Again (`docs/gui.md`, Export Again): darktable's export with the settings
//! as last kept, with the Export tool left as it is. darktable's Ctrl+E is its export
//! button's own `_export_with_current_settings`, which asks the storage for a
//! confirmation first (File on disk: "export in overwrite mode?") and then exports; so this
//! does what the tool's Export button does, from the settings the tool left in `export.*`.
//!
//! The question is darktable's: asked when On conflict is Overwrite and the setting
//! `export.ask_before_overwrite` is on, whether or not a file is there
//! ([`crate::export::asks_before`]), so nothing waits for the session: [`export`] makes the
//! command, and [`Again`] holds it while the question is up.

use std::path::{Path, PathBuf};

use apertrust_engine::session::{Command, Places};

use crate::export::Values;

/// The export of `photos` with `values`, the settings as the tool left them: Export
/// Again's, and the Export button's, each the selected photos or the photo open
/// (`selection::exported`).
pub fn export(photos: Vec<PathBuf>, values: &Values, places: &Places) -> Command {
    Command::Export {
        photos,
        settings: values.settings(),
        places: places.clone(),
    }
}

/// The export waiting for the person's answer to the overwrite question.
#[derive(Debug, Default)]
pub struct Again {
    /// The export to send on a yes, and whether the Export tool's button asked for it, which
    /// then says it started.
    asked: Option<(Command, bool)>,
}

/// `command`, a held export, without `photo`, the photo open for viewing only if there is
/// one, which no export takes, the engine not refusing it: none where no photo is left to
/// send.
pub fn without(mut command: Command, photo: Option<&Path>) -> Option<Command> {
    if let (Command::Export { photos, .. }, Some(photo)) = (&mut command, photo) {
        photos.retain(|p| p != photo);
        if photos.is_empty() {
            return None;
        }
    }
    Some(command)
}

impl Again {
    /// Holds `export`, File › Export Again's, while the question is asked. An earlier one
    /// still held is dropped.
    pub fn hold(&mut self, export: Command) {
        self.asked = Some((export, false));
    }

    /// Holds `export`, the Export tool's, while the question is asked.
    pub fn hold_for_tool(&mut self, export: Command) {
        self.asked = Some((export, true));
    }

    /// The person said yes to the question: the export to send.
    #[cfg(test)]
    pub fn confirmed(&mut self) -> Option<Command> {
        self.confirmed_by().map(|(command, _)| command)
    }

    /// The person said yes to the question: the export to send, and whether the Export
    /// tool asked for it.
    pub fn confirmed_by(&mut self) -> Option<(Command, bool)> {
        self.asked.take()
    }

    /// The person said no, or the question could not be asked: nothing is sent.
    pub fn declined(&mut self) {
        self.asked = None;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    use apertrust_export::settings::OnConflict;

    use super::*;

    pub(crate) const PHOTO: &str = "/Users/anna/Pictures/IMG_2041.ARW";

    pub(crate) fn places() -> Places {
        Places {
            home: Some(PathBuf::from("/Users/anna")),
            pictures: None,
            desktop: None,
            user_name: "anna".to_owned(),
        }
    }

    pub(crate) fn values(on_conflict: OnConflict) -> Values {
        Values {
            path: "$(FILE_FOLDER)/kept/$(FILE_NAME)".to_owned(),
            on_conflict,
            jpeg_quality: 71,
            ..Values::default()
        }
    }

    /// A held export loses only the photo that turned view-only; with none left, nothing
    /// is sent.
    #[test]
    fn a_held_export_keeps_its_other_photos_when_one_turns_view_only() {
        let other = PathBuf::from("/Users/anna/Pictures/IMG_2042.ARW");
        let kept = values(OnConflict::Overwrite);
        let both = vec![PathBuf::from(PHOTO), other.clone()];
        let held = export(both, &kept, &places());
        let left = without(held.clone(), Some(Path::new(PHOTO)));
        assert_eq!(left, Some(export(vec![other], &kept, &places())));
        assert_eq!(without(held.clone(), None), Some(held));
        let alone = export(vec![PathBuf::from(PHOTO)], &kept, &places());
        assert_eq!(without(alone, Some(Path::new(PHOTO))), None);
    }

    #[test]
    fn it_exports_the_settings_as_last_kept_for_the_photo_open() {
        let kept = values(OnConflict::Overwrite);
        let Command::Export {
            photos,
            settings,
            places: sent,
        } = export(vec![PathBuf::from(PHOTO)], &kept, &places())
        else {
            panic!("an export");
        };
        assert_eq!(photos, [PathBuf::from(PHOTO)]);
        assert_eq!(settings, kept.settings());
        assert_eq!(sent, places());
    }

    #[test]
    fn a_yes_sends_the_held_export_once_and_a_no_sends_nothing() {
        let kept = values(OnConflict::Overwrite);
        let mut again = Again::default();
        again.hold(export(vec![PathBuf::from(PHOTO)], &kept, &places()));
        let Some(Command::Export { settings, .. }) = again.confirmed() else {
            panic!("a yes sends the export");
        };
        assert_eq!(settings, kept.settings());
        assert_eq!(again.confirmed(), None, "once");

        again.hold(export(vec![PathBuf::from(PHOTO)], &kept, &places()));
        again.declined();
        assert_eq!(again.confirmed(), None, "a no sends nothing");
    }

    #[test]
    fn a_yes_says_whether_the_export_tool_asked() {
        let kept = values(OnConflict::Overwrite);
        let command = || export(vec![PathBuf::from(PHOTO)], &kept, &places());
        let mut again = Again::default();
        again.hold_for_tool(command());
        assert_eq!(again.confirmed_by(), Some((command(), true)));
        again.hold(command());
        assert_eq!(again.confirmed_by(), Some((command(), false)));
        // A no says nothing at all: the tool notes no start.
        again.hold_for_tool(command());
        again.declined();
        assert_eq!(again.confirmed_by(), None);
    }

    #[test]
    fn holding_another_export_replaces_the_one_held() {
        let mut again = Again::default();
        again.hold(export(
            vec![PathBuf::from(PHOTO)],
            &values(OnConflict::CreateUniqueFilename),
            &places(),
        ));
        let newer = values(OnConflict::Overwrite);
        again.hold(export(vec![PathBuf::from(PHOTO)], &newer, &places()));
        let Some(Command::Export { settings, .. }) = again.confirmed() else {
            panic!("the export");
        };
        assert_eq!(settings, newer.settings());
    }
}
