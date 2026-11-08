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

# The documentation a pull request may change and still skip the code's checks,
# .github/docs-only-paths.txt (decision 70).
#
#   tools/check-docs-paths.sh                fails if a file the list matches is read by
#                                            code: named in a string in crates/, include_str!
#                                            among them, or in tools/ (CI's lint)
#   tools/check-docs-paths.sh --code REVS    prints the files `git diff REVS` changes that
#                                            the list does not match: none, docs only
set -euo pipefail

cd "$(dirname "$0")/.."

list=.github/docs-only-paths.txt

# Each line a glob pathspec, a `!` one excluding; comments and blank lines left out.
pathspecs=()
while IFS= read -r glob; do
  case "$glob" in
    '!'*) pathspecs+=(":(exclude,glob)${glob#!}") ;;
    *) pathspecs+=(":(glob)$glob") ;;
  esac
done < <(sed -e 's/#.*//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e '/^$/d' "$list")

if [[ "${1:-}" == --code ]]; then
  shift
  # Both sides of a rename, so a file moved out of the code counts.
  comm -23 \
    <(git diff --name-only --no-renames "$@" | sort) \
    <(git diff --name-only --no-renames "$@" -- "${pathspecs[@]}" | sort)
  exit 0
fi

# The Markdown files code names, outside comments: in crates/, any string literal ending
# in .md, include_str!'s path among them; in tools/, any word ending in .md. Each without
# its leading ./, ../ or /, so a relative path matches the end of the file's.
named=$(
  {
    git grep -h -F .md -- 'crates/*.rs' | grep -v -E '^[[:space:]]*//' | grep -o -E '"[^"]*\.md"' | tr -d '"' || true
    git grep -h -F .md -- tools | grep -v -E '^[[:space:]]*(#|//)' | grep -o -E '[A-Za-z0-9_./-]+\.md' || true
  } | sed -E 's#^((\.\.?)?/)+##' | sort -u
)

docs=$(git ls-files --cached --others --exclude-standard -- "${pathspecs[@]}")

read_by_code=$(awk '
  NR == FNR { if ($0 != "") named[$0]; next }
  { for (n in named) if ($0 == n || substr($0, length($0) - length(n)) == "/" n) print $0 " (code names \"" n "\")" }
' <(echo "$named") <(echo "$docs"))

if [[ -n "$read_by_code" ]]; then
  echo "$read_by_code"
  echo "Code reads these files, so a change to them must run the checks: take them out of $list with a ! line."
  exit 1
fi
