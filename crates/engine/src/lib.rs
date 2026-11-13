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

//! Apertrust processing engine.
//!
//! Runs the scene-referred float32 pipeline on the GPU through wgpu. The engine has no UI
//! dependencies: the desktop UI, web UI, MCP and scripts all reach it through the session
//! protocol. Nor does it open a folder or a file of settings: the binaries read its
//! settings and hand it plain options (`settings`, decision 43). See
//! `docs/architecture.md`.

// The generated settings name this crate's choice types by their paths in the schema,
// `apertrust_engine::…`, as another crate would.
extern crate self as apertrust_engine;

pub mod compute;
pub mod edit;
pub mod export;
pub mod export_job;
pub mod export_preview;
pub mod gpu;
pub mod history;
pub mod jobs;
pub mod names;
pub mod picker;
pub mod pipeline;
pub mod raw;
mod readback;
pub mod resample;
pub mod scopes;
pub mod session;
pub mod settings;
pub mod sidecar;
