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

# Fails if a crate depends on a workspace crate its place in the layers does not allow,
# or on a library a rule keeps out of it (docs/architecture.md, Crates; docs/crates.md,
# How they depend on each other). It reads the dependencies each manifest declares:
# normal and build dependencies count; dev-dependencies are the tests' and may reach up
# the layers, as the tools' invariant tests reach the engine. What a library brings in
# with it is not looked at.
#   tools/check-crate-deps.sh   every crate (CI's lint, pre-commit when a manifest changes)
set -euo pipefail

cd "$(dirname "$0")/.."

# Each crate's rule: `may`, the workspace crates it may depend on, or "any" for a binary,
# which puts the others together; `never`, the crates it may not depend on, whatever
# `may` says. `apertrust-op-*` stands for every operation crate. A crate with no rule
# fails, so a new crate states its place when it lands (decision 65).
rules='{
  "apertrust": {
    "may": "any",
    "never": ["apertrust-library", "apertrust-raw"],
    "why": "the app reaches the library and the raw through the session (decisions 54 and 58)"
  },
  "apertrust-cli": {"may": "any"},
  "apertrust-golden": {"may": "any"},
  "apertrust-engine": {
    "may": ["apertrust-plugin", "apertrust-color", "apertrust-raw", "apertrust-tools",
            "apertrust-base", "apertrust-export", "apertrust-library"],
    "never": ["egui", "eframe", "winit"],
    "why": "the engine never depends on an operation crate or a client, nor on UI code (decisions 20 and 58)"
  },
  "apertrust-plugin": {
    "may": [],
    "never": ["wgpu"],
    "why": "the operation interface is plain data, with no wgpu (decision 21)"
  },
  "apertrust-color": {"may": [], "why": "shared colour maths, under the operations (decision 65)"},
  "apertrust-op-*": {
    "may": ["apertrust-plugin", "apertrust-color"],
    "why": "an operation is its kernel, setup, manifest and schema behind the interface, its shared maths in apertrust-color (decisions 20 and 65)"
  },
  "apertrust-operations": {
    "may": ["apertrust-op-*", "apertrust-plugin"],
    "why": "the list of built-in operations (decision 20)"
  },
  "apertrust-tools": {
    "may": ["apertrust-plugin", "apertrust-color", "apertrust-base"],
    "never": ["egui", "eframe", "wgpu"],
    "why": "tools depend on no egui, wgpu, engine or operation crate (decision 28)"
  },
  "apertrust-library": {
    "may": ["apertrust-base", "apertrust-raw"],
    "never": ["wgpu"],
    "why": "the library never depends on the engine, tools, plugin or wgpu (decision 54)"
  },
  "apertrust-export": {
    "may": ["apertrust-base", "apertrust-color"],
    "never": ["wgpu"],
    "why": "what an export writes and where, with no GPU (decision 56)"
  },
  "apertrust-raw": {"may": [], "why": "raw decoding through rawler, in its own types (decision 23)"},
  "apertrust-base": {
    "may": [],
    "never": ["wgpu", "egui", "eframe", "winit"],
    "why": "nothing about images, colour, the GPU or the UI (decision 43)"
  },
  "apertrust-ui": {
    "may": ["apertrust-base"],
    "why": "the design system has no engine or tool types, and uses apertrust-base for settings::Choice alone (decision 36)"
  },
  "apertrust-display": {"may": [], "why": "the window platform code, an FFI crate (decision 65)"}
}'

found=$(
  cargo metadata --format-version 1 --no-deps --offline |
  jq -r --argjson rules "$rules" '
    def matches($pattern):
      if ($pattern | endswith("*")) then startswith($pattern[:-1]) else . == $pattern end;
    def rule:
      . as $name
      | $rules[$name] // (if startswith("apertrust-op-") then $rules["apertrust-op-*"] else null end);
    .packages[]
    | .name as $crate
    | ($crate | rule) as $rule
    | if $rule == null then
        "\($crate) has no rule: add the workspace crates it may depend on to tools/check-crate-deps.sh"
      else
        .dependencies[]
        | select(.kind != "dev")
        | .name as $dep
        | if any(($rule.never // [])[]; $dep == .) then
            "\($crate) must not depend on \($dep): \($rule.why)"
          elif .path != null and $rule.may != "any"
            and (any($rule.may[]; . as $allowed | $dep | matches($allowed)) | not) then
            "\($crate) must not depend on \($dep): \($rule.why)"
          else empty end
      end
  ' |
  sort -u
)

if [[ -n "$found" ]]; then
  echo "$found" >&2
  exit 1
fi
