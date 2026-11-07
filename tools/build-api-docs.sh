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
for arg in "$@"; do
  if [[ "$arg" == --open ]]; then open=1; else args+=("$arg"); fi
done

RUSTDOCFLAGS="-D warnings ${RUSTDOCFLAGS:-}" \
  cargo doc --workspace --no-deps --document-private-items ${args[@]+"${args[@]}"}

index="$root/target/doc/apertrust_engine/index.html"
echo "API docs: $index"
if ((open)); then
  case "$(uname -s)" in
    Darwin) open "$index" ;;
    MINGW* | MSYS* | CYGWIN*) start "" "$index" ;;
    *) xdg-open "$index" ;;
  esac
fi
