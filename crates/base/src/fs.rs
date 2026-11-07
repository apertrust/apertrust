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

//! The safe write (decision 53), behind the `fs` feature, with no `unsafe`.
//!
//! Every file Apertrust writes into a person's folder is written whole to a hidden
//! temporary file beside its name and renamed into place, so a NAS, a cloud folder or a
//! crash only ever shows a complete file under a photo's name:
//!
//! - [`write()`] replaces a file with new bytes: written, flushed, synced and renamed, not
//!   at all when the file already holds them, with nothing left behind and the old file
//!   untouched on an error;
//! - [`create`] makes the hidden temporary (`.~NAME.XXXXXX.tmp`, `NAME` cut to fit a file
//!   system's 255 bytes), for a writer that fills the file itself, as the export's does;
//!   [`is_temporary_name`] tells such a name, and [`remove_if_stale`] removes those a
//!   crash left;
//! - [`persist_retrying`] is the rename, tried again for about a second where Windows
//!   refuses it because an indexer or a virus scanner holds the file.
//!
//! Bulk listing and cloud placeholders' flags, which need foreign calls, are planned.

mod temporary;
mod write;

pub use temporary::{STALE_AFTER, create, is_temporary_name, remove_if_stale};
pub use write::{Retry, Written, persist_retrying, write};
