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

# Claude Code PostToolUse hook: formats a Rust file Claude has just written or edited,
# with the workspace's edition. A file that does not parse yet is left alone; the
# turn-end check (turn-checks.sh) reports it.
set -uo pipefail

command -v jq > /dev/null || { echo "Apertrust's Claude Code hooks need jq" >&2; exit 1; }
file="$(jq -r '.tool_input.file_path // empty')"
[[ "$file" == *.rs && -f "$file" ]] || exit 0

root="$(git -C "$(dirname "$file")" rev-parse --show-toplevel 2> /dev/null)" || exit 0
edition="$(sed -n 's/^edition = "\([0-9]*\)"/\1/p' "$root/Cargo.toml" 2> /dev/null | head -n 1)"
rustfmt --edition "${edition:-2024}" "$file" > /dev/null 2>&1
exit 0
