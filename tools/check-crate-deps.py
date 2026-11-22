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

# Fails if a crate depends on a workspace crate its place in the layers does not allow,
# or on a library a rule keeps out of it (docs/architecture.md, Crates; docs/crates.md,
# How they depend on each other). The rules are data, each crate's in the root Cargo.toml's
# [workspace.metadata.crate-deps]. It reads the dependencies each manifest declares:
# normal and build dependencies count; dev-dependencies are the tests' and may reach up
# the layers, as the tools' invariant tests reach the engine. What a library brings in
# with it is not looked at.
#   tools/check-crate-deps.py   every crate (pre-commit when a manifest changes, in CI too)

import json
import subprocess
import sys
from pathlib import Path

OPERATION = "apertrust-op-"


def matches(name, pattern):
    """A crate name against a rule's entry, where "apertrust-op-*" stands for every operation crate."""
    if pattern.endswith("*"):
        return name.startswith(pattern[:-1])
    return name == pattern


def rule_of(rules, crate):
    if crate in rules:
        return rules[crate]
    if crate.startswith(OPERATION):
        return rules.get(OPERATION + "*")
    return None


def main():
    root = Path(__file__).resolve().parent.parent
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        ).stdout
    )
    rules = (metadata.get("metadata") or {}).get("crate-deps", {})
    # With --no-deps, the packages listed are the workspace's members alone.
    members = {package["name"] for package in metadata["packages"]}
    declared = {
        dependency["name"] for package in metadata["packages"] for dependency in package["dependencies"]
    }

    def is_member(pattern):
        return any(matches(member, pattern) for member in members)

    # A name that matches nothing is misspelled, or stale.
    found = set()
    for crate, rule in rules.items():
        # rule_of() takes a crate's own name, or the operations' wildcard alone.
        if crate not in members and not (crate == OPERATION + "*" and is_member(crate)):
            found.add(f"[workspace.metadata.crate-deps] has a rule for {crate}, which is no workspace crate")
        if rule.get("may") != "any":
            for name in rule.get("may", []):
                if not is_member(name):
                    found.add(f"{crate}'s rule names {name} in may, which is no workspace crate")
        for name in rule.get("never", []):
            if name not in members | declared:
                found.add(f"{crate}'s rule names {name} in never, which no crate depends on")

    for package in metadata["packages"]:
        crate = package["name"]
        rule = rule_of(rules, crate)
        if rule is None:
            found.add(f"{crate} has no rule: add its place to [workspace.metadata.crate-deps] in Cargo.toml")
            continue
        may = rule.get("may", [])
        for dependency in package["dependencies"]:
            if dependency["kind"] == "dev":
                continue
            name = dependency["name"]
            never = name in rule.get("never", [])
            not_allowed = (
                name in members and may != "any" and not any(matches(name, allowed) for allowed in may)
            )
            if never or not_allowed:
                why = rule.get("why", "its rule in Cargo.toml says no why, so add one")
                found.add(f"{crate} must not depend on {name}: {why}")

    for line in sorted(found):
        print(line, file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
