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

# Runs clippy over the workspace, failing on any warning.
#   tools/clippy.sh         this machine's platform, as `cargo clippy` checks it
#   tools/clippy.sh --all   Linux, Windows and macOS too, the platforms CI checks, from any
#                           machine (pre-commit's manual stage)
# Clippy compiles but never links, so another platform needs only its standard library,
# which rustup adds to the pinned toolchain the first time; its build has a folder of its
# own in target/, about 2 GB. The machine's own platform is checked without --target, so
# it shares cargo's usual build. With --all every platform is checked, and the script
# fails if any of them has a warning.
# A build script is compiled for this machine, whatever --target says, so its code for
# another platform is checked only where cfg!() chooses it at run time, not #[cfg]:
# crates/app/build.rs's Windows resources (winresource) are, and run on Windows alone.
set -euo pipefail

cd "$(dirname "$0")/.."

host="$(rustc -vV | sed -n 's/^host: //p')"
case "${1:-}" in
  # CI's runners: Linux and Windows on x86-64, macOS on Apple silicon.
  --all) targets=(x86_64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-apple-darwin) ;;
  "") targets=("$host") ;;
  *)
    echo "usage: tools/clippy.sh [--all]" >&2
    exit 2
    ;;
esac

failed=()
for target in "${targets[@]}"; do
  echo "clippy: $target"
  if [[ "$target" == "$host" ]]; then
    cargo clippy --workspace --all-targets -- -D warnings || failed+=("$target")
  else
    rustup target add "$target"
    cargo clippy --workspace --all-targets --target "$target" -- -D warnings || failed+=("$target")
  fi
done

if ((${#failed[@]})); then
  echo "clippy failed for: ${failed[*]}" >&2
  exit 1
fi
