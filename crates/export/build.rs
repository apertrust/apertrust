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

//! Generates the export settings, those the settings schema gives `owner = "export"`
//! (`data/settings.schema.toml`), as typed statics in `OUT_DIR/settings.rs`, which
//! `src/settings.rs` includes as its `registry` (decisions 43 and 56). The generator
//! embeds the schema, so a change to it reruns this script; one it refuses fails the
//! build, naming the line.

fn main() {
    apertrust_base::settings::codegen::build("export");
}
