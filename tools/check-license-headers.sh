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

# Fails if a source file lacks the GPLv3 licence header described in AGENTS.md.
set -euo pipefail

cd "$(dirname "$0")/.."
# Keep the canonical text in this script's own GPL header. Extra attribution lines
# are allowed, but cannot replace the project's copyright or the licence paragraphs.
header() {
  awk '
    NR == 1 && /^#!/ { next }
    /^\/\// { sub(/^\/\/ ?/, ""); print; if ($0 ~ /www.gnu.org\/licenses/) exit; next }
    /^#/ { sub(/^# ?/, ""); print; if ($0 ~ /www.gnu.org\/licenses/) exit; next }
    { exit }
  ' "$1"
}
expected=$(header "$0" | sed '2,/^$/ { /^$/!d; }')
missing=0
while IFS= read -r file; do
  actual=$(header "$file")
  body=$(printf '%s\n' "$actual" | sed '2,/^$/ { /^$/!d; }')
  if [[ "$body" != "$expected" ]] ||
    ! grep -Eq '^Copyright \(C\) .*Apertrust developers$' <<< "$actual"; then
    echo "missing or incomplete licence header: $file"
    missing=1
  fi
done < <(git ls-files --cached --others --exclude-standard '*.rs' '*.wgsl' '*.slang' '*.sh')
exit "$missing"
