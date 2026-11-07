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

# Claude Code PostToolUse hook for kernel sources. After an edit to a Slang kernel or
# one of its libraries, regenerates the WGSL (tools/build-kernels.sh) and validates
# every kernel with naga. Failures go back to Claude (exit 2). Generated WGSL is never
# edited by hand.
set -uo pipefail

command -v jq > /dev/null || { echo "Apertrust's Claude Code hooks need jq" >&2; exit 1; }
file="$(jq -r '.tool_input.file_path // empty')"
case "$file" in
  */crates/*/slang/*.slang) ;;
  */crates/*/generated/*.wgsl)
    echo "generated/ is written by tools/build-kernels.sh, and this hook regenerates it" \
      "after every .slang edit, overwriting hand edits: change the kernel's .slang" \
      "source instead." >&2
    exit 2
    ;;
  *) exit 0 ;;
esac

root="$(git -C "$(dirname "$file")" rev-parse --show-toplevel 2> /dev/null)" || exit 0
cd "$root" || exit 0
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

fail() {
  { echo "$1"; head -n 80 "$log"; } >&2
  exit 2
}

# build-kernels.sh would download slangc if it is missing; leave that to Claude.
version="$(sed -n 's/^VERSION=//p' tools/fetch-slang.sh)"
if ! compgen -G "external/slang/$version/bin/slangc*" > /dev/null; then
  echo "slangc $version is not installed, so the WGSL was not regenerated:" \
    "run tools/fetch-slang.sh, then tools/build-kernels.sh." >&2
  exit 2
fi
tools/build-kernels.sh > "$log" 2>&1 || fail "tools/build-kernels.sh failed:"
cargo test -q -p apertrust-operations -p apertrust-engine --test naga > "$log" 2>&1 \
  || fail "naga validation failed (cargo test -p apertrust-operations -p apertrust-engine --test naga):"

context="Hook: regenerated generated/*.wgsl from the Slang sources; naga validates every kernel."
jq -n --arg context "$context" \
  '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $context}}'
