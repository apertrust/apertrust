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

# Claude Code turn hooks. `start` (UserPromptSubmit) records fingerprints of the Rust
# sources and of everything a golden result depends on, and prints nothing: its output
# would reach Claude. `stop` (Stop) compares them and, for what changed during the
# turn, runs:
#   - Rust: cargo fmt --check and cargo clippy -D warnings;
#   - kernels, the golden harness or the engine: apertrust-golden run (ΔE against
#     darktable) and gradient, or only gradient --synthetic when APERTRUST_TEST_IMAGES
#     is not set.
# Changes made before the turn, such as the user's own work in progress, trigger
# nothing. A failure sends Claude back with the output, but only while the sources
# keep changing (at most three times a turn), so Claude can stop to report a failure
# it should not fix.
set -uo pipefail

command -v jq > /dev/null || { echo "Apertrust's Claude Code hooks need jq" >&2; exit 1; }
input="$(cat)"
session="$(jq -r '.session_id // "session"' <<< "$input")"
cwd="$(jq -r '.cwd // empty' <<< "$input")"
root="$(git -C "${cwd:-.}" rev-parse --show-toplevel 2> /dev/null)" || exit 0
[[ -f "$root/Cargo.toml" ]] || exit 0
cd "$root" || exit 0
state="target/claude-hooks"
turn="$state/turn-$session"
blocked="$state/blocked-$session"
max_blocks=3

if command -v sha256sum > /dev/null; then hash=(sha256sum); else hash=(shasum -a 256); fi
# A hash of the named files' contents, tracked or untracked but not ignored.
fingerprint() {
  git ls-files -z --cached --others --exclude-standard -- "$@" \
    | xargs -0 "${hash[@]}" 2> /dev/null | "${hash[@]}" | cut -d' ' -f1
}
rust_sources() { fingerprint '*.rs' '*Cargo.toml' Cargo.lock '*clippy.toml' '*rustfmt.toml'; }
golden_inputs() {
  fingerprint crates/op crates/color crates/plugin \
    crates/operations crates/golden crates/engine Cargo.lock
}

case "${1:-}" in
  start)
    mkdir -p "$state"
    printf '%s %s\n' "$(rust_sources)" "$(golden_inputs)" > "$turn"
    rm -f "$blocked"
    exit 0
    ;;
  stop) ;;
  *) exit 0 ;;
esac

[[ -f "$turn" ]] || exit 0
read -r rust_before golden_before < "$turn"
rust_now="$(rust_sources)"
golden_now="$(golden_inputs)"
[[ "$rust_now" != "$rust_before" ]] && rust_changed=1 || rust_changed=0
[[ "$golden_now" != "$golden_before" ]] && golden_changed=1 || golden_changed=0
((rust_changed || golden_changed)) || exit 0

log="$(mktemp)"
trap 'rm -f "$log"' EXIT
failures="" # the failed checks' names, for the user
details=""  # their output, for Claude
note=""

# Runs a check; on failure records its name and the start of its output.
check() {
  local name="$1"
  shift
  "$@" > "$log" 2>&1 && return 0
  failures+="${failures:+, }$name"
  details+="## $name"$'\n'"$(head -n 80 "$log")"$'\n\n'
  return 1
}

if ((rust_changed)); then
  check "cargo fmt --all --check" cargo fmt --all -- --check -l
  # The harness needs a clean build; it runs at the next stop, once clippy passes.
  check "cargo clippy --workspace --all-targets -- -D warnings" \
    cargo clippy -q --workspace --all-targets --message-format short -- -D warnings \
    || golden_changed=0
fi
if ((golden_changed)); then
  golden=(cargo run -q --release -p apertrust-golden --)
  if [[ -d "${APERTRUST_TEST_IMAGES:-}" ]]; then
    check "apertrust-golden run" "${golden[@]}" run --rounds 1 --iterations 1 --warmup-ms 0
    check "apertrust-golden gradient" "${golden[@]}" gradient
  else
    check "apertrust-golden gradient --synthetic" "${golden[@]}" gradient --synthetic
    note="Golden ΔE check skipped: APERTRUST_TEST_IMAGES is not set to darktable's"
    note+=" integration-test images, so only the synthetic gradient check ran."
  fi
fi

if [[ -z "$failures" ]]; then
  [[ -z "$note" ]] || jq -n --arg message "$note" '{systemMessage: $message}'
  exit 0
fi

# Unchanged sources since the last block mean Claude chose to stop: let it.
sources="$rust_now $golden_now"
blocks=0
if [[ -f "$blocked" ]]; then
  read -r blocks last_sources < "$blocked"
  if [[ "$last_sources" == "$sources" ]] || ((blocks >= max_blocks)); then
    jq -n --arg message "Turn-end checks still fail: $failures.${note:+ $note}" \
      '{systemMessage: $message}'
    exit 0
  fi
fi
echo "$((blocks + 1)) $sources" > "$blocked"

reason="Turn-end checks (.claude/hooks/turn-checks.sh) fail after this turn's changes.

$details"
reason+="Fix what this turn's changes caused. If a failure is in code this turn did not touch,
or comes from the environment (no GPU adapter, a darktable-cli other than the reference release), tell
the user instead: the checks will not stop you again while the sources are unchanged.
Never change a golden case, tolerance or baseline to make a check pass.${note:+

$note}"
jq -n --arg reason "$reason" --arg message "Turn-end checks failed: $failures" \
  '{decision: "block", reason: $reason, systemMessage: $message}'
