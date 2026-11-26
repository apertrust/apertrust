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
# The start of a string literal: "...", b"...", r#"..."#, br#"..."#, c"..." and the like.
STRING = re.compile(r'(?<![\w])(b|c)?(r(#*))?"')
# A character literal, as 'x', '"', '\'' or '\u{e9}', and not a lifetime's 'a.
CHARACTER = re.compile(r"'(\\'|\\[^'\n]+|[^\\'\n])'")


def code_only(text: str) -> str:
    """`text` with its comments and literals blanked, every newline kept, so an offset or
    a line in it is the same in `text`."""
    out = list(text)
    i = 0
    while i < len(text):
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = len(text) if end < 0 else end
        elif text.startswith("/*", i):
            # Block comments nest.
            depth, end = 1, i + 2
            while end < len(text) and depth:
                if text.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif text.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
        elif string := STRING.match(text, i):
            if string.group(2):
                # A raw string ends at its quote and as many #s, and has no escapes.
                close = '"' + string.group(3)
                end = text.find(close, string.end())
                end = len(text) if end < 0 else end + len(close)
            else:
                end = string.end()
                while end < len(text) and text[end] != '"':
                    end += 2 if text[end] == "\\" else 1
                end += 1
        elif character := CHARACTER.match(text, i):
            end = character.end()
        else:
            i += 1
            continue
        for k in range(i, min(end, len(text))):
            if out[k] != "\n":
                out[k] = " "
        i = end
    return "".join(out)


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
    # One a comment or a string holds is no attribute.
    code = code_only(text)
    found = []
    for attribute in ATTRIBUTE.finditer(code):
        start = attribute.start()
        if not PLATFORM.search(code, start, attribute_end(code, start)):
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
