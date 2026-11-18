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

# The paths whose change runs a pull request's tests on Windows, .github/windows-paths.txt
# (decision 70).
#
#   tools/check-windows-paths.sh                  fails if a file with a platform cfg is
#                                                 not covered by the list (CI's lint)
#   tools/check-windows-paths.sh FILE...          the same for the files given, from the
#                                                 repository's root, as pre-commit passes
#                                                 them; for every file when the list is
#                                                 among them
#   tools/check-windows-paths.sh --changed REVS   prints the files `git diff REVS` changes
#                                                 that the list covers, e.g. main...HEAD
set -euo pipefail
# sort and comm in one order, the bytes', whatever the locale: under a UTF-8 one they
# can disagree, and comm then fails.
export LC_ALL=C

cd "$(dirname "$0")/.."

list=.github/windows-paths.txt

# Each line a glob pathspec, comments and blank lines left out.
pathspecs=()
while IFS= read -r glob; do
  pathspecs+=(":(glob)$glob")
done < <(sed -e 's/#.*//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e '/^$/d' "$list")

if [[ "${1:-}" == --changed ]]; then
  shift
  git diff --name-only "$@" -- "${pathspecs[@]}"
  exit 0
fi

# A cfg attribute or macro naming a platform: cfg(windows), cfg!(target_os = "macos"),
# cfg_attr(unix, …), inside any() or not() too; in Rust sources and Cargo manifests.
platform='cfg(_attr)?!?\(.*(target_os|target_family|target_vendor|target_env|windows|unix)'

# The files to look at: the Rust sources and manifests among those given, or every one. A
# change to the list can leave any file out, so with the list among them, every one too.
targets=('*.rs' '*.toml')
if (($#)) && ! printf '%s\n' "$@" | grep -qxF "$list"; then
  targets=()
  for file in "$@"; do
    case "$file" in
      *.rs | *.toml) targets+=("$file") ;;
    esac
  done
  [[ -n "${targets[*]-}" ]] || exit 0
fi

missing=$(comm -23 \
  <(git grep -l --untracked -E "$platform" -- "${targets[@]}" | sort) \
  <(git ls-files --cached --others --exclude-standard -- "${pathspecs[@]}" | sort))

if [[ -n "$missing" ]]; then
  echo "$missing"
  echo "These files have a platform cfg but are not in $list: add them, so a pull request that changes them runs the tests on Windows."
  exit 1
fi
