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
missing=0
while IFS= read -r file; do
  if ! head -n 20 "$file" | grep -q "GNU General Public License"; then
    echo "missing licence header: $file"
    missing=1
  fi
done < <(git ls-files --cached --others --exclude-standard '*.rs' '*.wgsl' '*.slang' '*.sh')
exit "$missing"
