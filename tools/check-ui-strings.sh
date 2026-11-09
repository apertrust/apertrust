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

# Fails if interface code passes an English literal to a label, button, heading or
# tooltip, instead of a translated message (docs/gui.md, Translation). Test modules are
# skipped, from their `#[cfg(test)]` on.
set -euo pipefail

cd "$(dirname "$0")/.."

# Not checked: the design system's gallery and the viewport spike's section in the app,
# tools for developers; the module renderer replaces the spike (docs/gui.md, build order
# step 6).
allowed='^(crates/ui/src/gallery\.rs|crates/app/src/spike\.rs)$'

# A quoted string as the first argument, or after the first, of the egui calls that show
# text, and of the widgets' constructors.
pattern='\.(label|button|small_button|heading|small|monospace|strong|weak|selectable_label|checkbox|radio|radio_value|toggle_value|hyperlink_to|on_hover_text|on_disabled_hover_text)\(([^()"]*, *)?"|(Label|Button|RichText|Checkbox|SelectableLabel|Hyperlink)::new\("'

found=0
while IFS= read -r file; do
  [[ "$file" =~ $allowed ]] && continue
  hits=$(awk '/^ *#\[cfg\(test\)\]/ { exit } { print FILENAME ":" FNR ": " $0 }' "$file" | grep -E "$pattern" || true)
  if [[ -n "$hits" ]]; then
    echo "$hits"
    found=1
  fi
done < <(git ls-files --cached --others --exclude-standard 'crates/ui/src/*.rs' 'crates/app/src/*.rs')

if [[ "$found" -ne 0 ]]; then
  echo "Interface text must be a message id: tr(ctx, \"…\") or Translator::text (crates/ui/locales)."
fi
exit "$found"
