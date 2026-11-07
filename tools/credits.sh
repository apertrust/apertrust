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

# Writes CREDITS.md's list of components: every library the programs Apertrust ships, the
# app and the command line, are built from, on every platform, with its licence, as
# Cargo.lock has them. The rest of the file is written by hand. With --check, changes
# nothing and fails if the list is out of date.
set -euo pipefail

cd "$(dirname "$0")/.."

file=CREDITS.md
begin='<!-- components: written by tools/credits.sh, not by hand -->'
end='<!-- end of components -->'

# One line a library: `- name version: licence`. Not the workspace's own crates, which
# cargo shows with their path.
list=$(
    cargo tree --locked --edges normal --target all -p apertrust -p apertrust-cli \
        --prefix none --format '{p}|{l}' |
        sed -e 's/ (\*)$//' -e 's/ (proc-macro)//' |
        grep -v ' (/' |
        sort -u |
        awk -F'|' '$1 != "" { split($1, p, " v"); printf "- %s %s: %s\n", p[1], p[2], $2 }'
)

new=$(LIST="$list" awk -v begin="$begin" -v end="$end" '
    $0 == begin { print; print ""; print ENVIRON["LIST"]; print ""; skip = 1; next }
    $0 == end { skip = 0 }
    !skip { print }
' "$file")

if [[ ${1:-} == --check ]]; then
    if [[ "$new" != "$(cat "$file")" ]]; then
        echo "$file's components are out of date: run tools/credits.sh" >&2
        exit 1
    fi
    exit 0
fi
printf '%s\n' "$new" >"$file"
echo "$file: $(printf '%s\n' "$list" | wc -l | tr -d ' ') components"
