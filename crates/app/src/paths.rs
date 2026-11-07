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

//! The person's own folders that the app looks in, found through the `directories` crate,
//! never a path built by hand (`desktop-architecture`, App paths and settings): for now
//! their Pictures folder, where the open dialog starts. Apertrust's own folders, the user
//! data folder among them, are `apertrust_base::folders`'s (decision 43).

use std::path::{Path, PathBuf};

use directories::UserDirs;

/// The person's Pictures folder, or None where the system names none. On Linux it is read
/// from `user-dirs.dirs`, so the app reads it once, as it starts.
pub fn pictures() -> Option<PathBuf> {
    UserDirs::new()?.picture_dir().map(Path::to_owned)
}
