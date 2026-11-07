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

//! `apertrust-library`: the library's code, with no GPU and no engine (decisions 46, 47 and 52 to 55). The engine's session serves its operations to clients; the command line's
//! `library scan` still reaches the index directly.
//!
//! Built so far: [`sidecar`], the `.apxmp` file's text layer (layer 1), [`listing`], one
//! folder's raws and their sidecars, listed on a worker thread, [`index`], that listing
//! with each sidecar read, in memory, and [`ops`], the commands and events clients see
//! (layer 4), opening a folder first.

pub mod index;
pub mod listing;
pub mod ops;
pub mod sidecar;
pub mod xmp;

pub use index::{Counts, Edit, Index, Sidecar};

pub use listing::{Error, Event, FileStat, Folder, Listing, Options, Orphan, Photo, Summary};
