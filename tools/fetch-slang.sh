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

# Downloads the pinned Slang compiler into external/slang/<version>/ and prints its bin
# directory. Generated WGSL is committed, so only people editing .slang kernels need it.
set -euo pipefail

VERSION=2026.20
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/external/slang/$VERSION"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) asset=macos-aarch64 sha=07fa8833fdbb343c3674c84fdd885910f495f3daaaa0c732b32980151e8b58c4 ;;
  Darwin-x86_64) asset=macos-x86_64 sha=b4f01f615dc625f60c593d7d0a4bbd38e05dfa2a34bfd3973017c309d866feaa ;;
  Linux-x86_64) asset=linux-x86_64 sha=e863e568c204810d6e8bc31852f056aef6763c55ea9925db2db7affa060347c9 ;;
  Linux-aarch64) asset=linux-aarch64 sha=b0345743e2153d8c2bd33f56fb5696bcef63fc03513f054a1ae1b8d3dd0152a9 ;;
  MINGW*-x86_64 | MSYS*-x86_64) asset=windows-x86_64 sha=7b9bb838b105de0bb86390196ac5bcb02efc610c0542517204105cf87ec98399 ;;
  *) echo "no pinned Slang build for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

if [[ -x "$dest/bin/slangc" || -x "$dest/bin/slangc.exe" ]]; then
  echo "$dest/bin"
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
archive="$tmp/slang.tar.gz"
curl --proto '=https' -fsSL -o "$archive" \
  "https://github.com/shader-slang/slang/releases/download/v$VERSION/slang-$VERSION-$asset.tar.gz"

if command -v sha256sum > /dev/null; then
  actual="$(sha256sum "$archive" | cut -d' ' -f1)"
else
  actual="$(shasum -a 256 "$archive" | cut -d' ' -f1)"
fi
if [[ "$actual" != "$sha" ]]; then
  echo "checksum mismatch for slang-$VERSION-$asset: got $actual" >&2
  exit 1
fi

mkdir -p "$dest"
tar -xzf "$archive" -C "$dest"
echo "$dest/bin"
