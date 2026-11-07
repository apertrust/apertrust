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

# Downloads the raw decoding corpus, the raw.pixls.us samples listed in
# crates/raw/tests/corpus/files.sha256 (about 1.8 GB), into
# test-images/raw.pixls.us, which git ignores. Files already present with the right
# checksum are skipped.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
list="$root/crates/raw/tests/corpus/files.sha256"
dest="$root/test-images/raw.pixls.us"

if command -v sha256sum >/dev/null; then
  sha256() { sha256sum "$1" | cut -d' ' -f1; }
else
  sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
fi

failed=0
while read -r digest name; do
  # A checkout with CRLF line endings (core.autocrlf on Windows) leaves a \r on each name.
  name="${name%$'\r'}"
  file="$dest/$name"
  if [[ -f "$file" && "$(sha256 "$file")" == "$digest" ]]; then
    continue
  fi
  mkdir -p "$(dirname "$file")"
  echo "fetching $name"
  curl -fsSL --globoff -o "$file.part" "https://raw.pixls.us/data/${name// /%20}"
  if [[ "$(sha256 "$file.part")" != "$digest" ]]; then
    echo "checksum mismatch: $name" >&2
    rm -f "$file.part"
    failed=1
    continue
  fi
  mv "$file.part" "$file"
done < "$list"
exit "$failed"
