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

//! `apertrust-base`: what every Apertrust program sets up as it starts, and nothing else
//! (architecture decision 43).
//!
//! Its rule: only what a program needs before its first real work: its logging with the
//! startup banner and the version, its folders and its settings. Nothing
//! about images, colour, the GPU or the interface: no egui, no wgpu, no engine, so that
//! every client, the design system included, can depend on it.
//!
//! - `log`, behind the `log` feature, which the design system never turns on: darktable's
//!   `-d` domains, short lines and the startup banner (decision 44), and
//!   `program!()`, a binary with its version, which it hands the banner, About and the
//!   edits' `x:xmptk`.
//! - `build`, behind the `build` feature, which only the binaries' build-dependencies turn
//!   on: `build::version()`, which reads the version and the banner's versions in a
//!   binary's own build script for `program!()`, so that a commit rebuilds no library.
//! - `folders`, behind the `folders` feature, which only the binaries turn on: `Folders`,
//!   the user data folder and how it was chosen, by `--user-data-dir` or its aliases, by
//!   `APERTRUST_USER_DATA_DIR` or as the platform's, the one place that decides it; the
//!   cache folder joins it with the first cache.
//! - `jobs`, behind the `jobs` feature, which the engine and the library turn on:
//!   darktable's control jobs, its queues, priorities, workers, progress and cancel
//!   (decision 57, which widened this crate's rule to hold them), knowing nothing of
//!   images or the GPU.
//! - `fs`, behind the `fs` feature, which the sidecar, the export and the library turn on
//!   (decision 53): the safe write, a file replaced whole through a hidden temporary and a
//!   rename, with the temporary's naming and the removal of old ones.
//! - [`settings`]: the settings schema's types, a setting and its kinds and [`Choice`]
//!   (`settings::Choice`); the [`Store`] (`settings::Store`), which reads and writes
//!   `settings.toml` through a [`Storage`] (`settings::Storage`); and, behind the `codegen`
//!   feature, the generator that each owner's build script runs on the schema,
//!   `data/settings.schema.toml` in the workspace's top-level `data/` (decision 45).
//!
//! [`Choice`]: settings::Choice
//! [`Store`]: settings::Store
//! [`Storage`]: settings::Storage

#[cfg(any(feature = "build", test))]
pub mod build;
#[cfg(any(feature = "folders", test))]
pub mod folders;
#[cfg(any(feature = "fs", test))]
pub mod fs;
#[cfg(any(feature = "jobs", test))]
pub mod jobs;
#[cfg(feature = "log")]
pub mod log;
pub mod settings;
