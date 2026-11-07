#!/usr/bin/env bash
# Apertrust, a free, non-destructive, cross-platform raw photo editor
# Copyright (C) 2026  Apertrust developers
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

# Renders the app icon's platform files from its SVG sources, into crates/app/assets/icons
# (its README lists them): the macOS iconsets and .icns, the Windows sizes and .ico, the
# Linux hicolor tree named after the app id, and the mark's PNGs. Run it after an SVG
# changes and commit what it writes; packages and the window's icon take those files.
# Rust only, through the app's `icons` example: no system tool is needed.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo run --locked --quiet -p apertrust --example icons -- crates/app/assets/icons
git status --short crates/app/assets/icons
