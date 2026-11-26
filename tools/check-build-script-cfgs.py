#!/usr/bin/env python3
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

# Fails if a build script hides code for a platform behind #[cfg], which clippy then
# checks on that platform alone: a build script is compiled for the machine building,
# whatever --target says (decision 70). Choose it at run time with cfg!() instead, so it
# is compiled everywhere, as crates/app/build.rs does its Windows resources. Where a
# dependency builds on one platform alone and #[cfg] is needed, the line before says why:
#   // host-only: <reason>
#
#   tools/check-build-script-cfgs.py FILE...   the build scripts among them, a build.rs
#                                              beside a Cargo.toml (pre-commit, in CI too)

import re
import sys
from pathlib import Path

# The start of #[cfg(...)], #![cfg(...)] or #[cfg_attr(...)], which may run over lines.
ATTRIBUTE = re.compile(r"#!?\[cfg(_attr)?\(")
# What names a platform in one: its system or its architecture.
PLATFORM = re.compile(
    r"\b(windows|unix|target_(os|family|env|vendor|arch|pointer_width|endian))\b"
)
MARKED = re.compile(r"//\s*host-only:\s*\S")


def is_build_script(path: Path) -> bool:
    return path.name == "build.rs" and (path.parent / "Cargo.toml").is_file()


def attribute_end(text: str, start: int) -> int:
    """Just past the `]` that closes the attribute starting at `start`."""
    depth = 0
    for i in range(text.index("[", start), len(text)):
        if text[i] == "[":
            depth += 1
        elif text[i] == "]":
            depth -= 1
            if depth == 0:
                return i + 1
    return len(text)


def unmarked_cfgs(root: Path, name: str) -> list[str]:
    """The platform cfgs of `name`, by their first line, with no `// host-only:` on the
    line before."""
    text = (root / name).read_text(encoding="utf-8")
    lines = text.splitlines()
    found = []
    for attribute in ATTRIBUTE.finditer(text):
        start = attribute.start()
        # One a line comment mentions is no attribute.
        if "//" in text[text.rfind("\n", 0, start) + 1 : start]:
            continue
        if not PLATFORM.search(text, start, attribute_end(text, start)):
            continue
        number = text.count("\n", 0, start) + 1
        previous = lines[number - 2] if number > 1 else ""
        if not MARKED.search(previous):
            found.append(f"{name}:{number}: {lines[number - 1].strip()}")
    return found


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    found = [
        line
        for name in sys.argv[1:]
        if is_build_script(root / name)
        for line in unmarked_cfgs(root, name)
    ]
    for line in found:
        print(line, file=sys.stderr)
    if found:
        print(
            "a platform's #[cfg] in a build script is never compiled elsewhere: choose it with"
            " cfg!(), or say why it must be #[cfg] on the line before, // host-only: <reason>",
            file=sys.stderr,
        )
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
