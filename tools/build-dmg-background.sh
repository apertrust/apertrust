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

# Renders the disk image's background, packaging/macos/dmg/background.svg, at 1x and 2x
# through the app's `dmg_background` example (resvg), and joins the two into
# packaging/macos/dmg/background.tiff with tiffutil, as Finder takes a HiDPI background:
# one TIFF of 660 x 400 at 72 dpi and 1320 x 800 at 144 dpi. Run it on a Mac after the SVG
# or the logo changes and commit the TIFF; tools/package.sh puts it in every disk image.
set -euo pipefail

cd "$(dirname "$0")/.."

dmg=packaging/macos/dmg
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cargo run --locked --quiet -p apertrust --example dmg_background -- \
    "$dmg/background.svg" "$work/background.png" "$work/background@2x.png"
tiffutil -cathidpicheck "$work/background.png" "$work/background@2x.png" \
    -out "$dmg/background.tiff"
tiffutil -info "$dmg/background.tiff" | grep -E 'Image Width|Resolution:'
git status --short "$dmg"
