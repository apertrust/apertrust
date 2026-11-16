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

# Builds the API docs: rustdoc's pages from the code's doc comments, for every crate
# and binary in the workspace, private items included, into target/doc, which git
# ignores. They are not the design docs in docs/, which are written by hand and
# committed. Dependencies are left out; their names show unlinked.
# Any rustdoc warning fails, as in CI. Arguments go to cargo doc: --open opens the
# engine's page in a browser.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

args=()
open=0
target_dir="${CARGO_TARGET_DIR:-target}"
take_target=0
for arg in "$@"; do
  if ((take_target)); then target_dir="$arg"; take_target=0; fi
  case "$arg" in
    --open) open=1 ;;
    --target-dir) take_target=1; args+=("$arg") ;;
    --target-dir=*) target_dir="${arg#*=}"; args+=("$arg") ;;
    *) args+=("$arg") ;;
  esac
done

RUSTDOCFLAGS="-D warnings ${RUSTDOCFLAGS:-}" \
  cargo doc --workspace --no-deps --document-private-items ${args[@]+"${args[@]}"}

case "$target_dir" in
  /* | [A-Za-z]:*) ;;
  *) target_dir="$root/$target_dir" ;;
esac
index="$target_dir/doc/apertrust_engine/index.html"
echo "API docs: $index"
if ((open)); then
  case "$(uname -s)" in
    Darwin) open "$index" ;;
    MINGW* | MSYS* | CYGWIN*) start "" "$index" ;;
    *) xdg-open "$index" ;;
  esac
fi
