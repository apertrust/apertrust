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

# Compiles every Slang kernel to WGSL: each operation's, in crates/op/*/slang/,
# and the engine's own, in crates/engine/slang/, each into the generated/
# directory beside its slang/. Kernels import the shared libraries in
# crates/color/slang/ (colour maths) and crates/plugin/slang/ (the
# pass header, and the component writes that stand in for run-time indices).
# Commit the output: builds and CI use the generated WGSL and never need slangc.
# Each output starts with its source's licence header, the leading comment block.
# With --check, it compiles into a temporary directory instead and lists every
# generated file that is stale or has no source, failing if there is one; generated/
# is left as it is.
set -euo pipefail
shopt -s nullglob

check=0
[[ "${1:-}" == --check ]] && check=1

root="$(cd "$(dirname "$0")/.." && pwd)"
bin="$("$root/tools/fetch-slang.sh")"
libs=(-I "$root/crates/color/slang" -I "$root/crates/plugin/slang")

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

stale=0
for src in "$root"/crates/op/*/slang "$root/crates/engine/slang"; do
  crate="$(dirname "$src")"
  rel="${crate#"$root"/}"
  out="$crate/generated"
  dest="$out"
  ((check)) && dest="$tmp/$rel/generated"
  mkdir -p "$dest"
  for kernel in "$src"/*.slang; do
    name="$(basename "$kernel" .slang)"
    "$bin/slangc" "$kernel" "${libs[@]}" -target wgsl -o "$tmp/$name.wgsl"
    {
      awk '/^\/\//{ print; next } { exit }' "$kernel"
      echo
      echo "// Generated from slang/$name.slang by tools/build-kernels.sh. Do not edit."
      echo
      cat "$tmp/$name.wgsl"
    } > "$dest/$name.wgsl"
    ((check)) || echo "$rel/generated/$name.wgsl"
  done

  ((check)) || continue
  for file in "$dest"/*.wgsl; do
    name="$(basename "$file")"
    if ! cmp -s "$file" "$out/$name"; then
      echo "stale: $rel/generated/$name"
      stale=1
    fi
  done
  for file in "$out"/*.wgsl; do
    name="$(basename "$file")"
    if [[ ! -e "$dest/$name" ]]; then
      echo "no source: $rel/generated/$name"
      stale=1
    fi
  done
done
exit "$stale"
