# 0003. The reference is darktable's latest stable release

## Context

During the migration ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)) every port is written from darktable's source and compared with darktable-cli's renders. Which darktable is the reference, and where does its version number live? A number written throughout the prose and code would have to be rewritten at every release, and references from two releases must never mix.

## Decision

**The reference is darktable's latest stable release,** today 5.6.2, followed until the migration ends.

**Prose, commands and comments say "the reference release"**, "darktable-cli" or just "darktable". A version number appears only as data:

1. `REFERENCE_VERSION` in `crates/golden/src/darktable.rs`: the one constant the golden harness checks darktable-cli against, refusing any other version, and hashes into its caches. `tools/compare-cpu.sh` reads it, and the `port-module` skill builds the source tag from it (`$DT_TAG`, `release-` and the number).
2. Each operation manifest's provenance (`release`, `commit`): the release its port was written from.
3. darktable's own file paths, blob ids, checksums and commit ids.

**A port reads darktable's source at the reference release's tag,** never a newer checkout. Masks and retouch are the one exception: they are ported from darktable's master, at a commit their port spec names ([0074](0074-masks-ported-from-darktables-master.md)).

**Following a new release is its own piece of work:**

1. Re-render the golden references with the new release's darktable-cli.
2. Review each ported module's upstream changes, the diff between the two tags, and follow their bug fixes and behaviour changes.
3. Rerun the checks.
4. Update the constant, each manifest's provenance and each port spec's header.

Until that is done the harness refuses the new darktable-cli, so references from two releases never mix.

The rule ends with the migration.

## Options considered

- A pinned release named everywhere: says one version where the intent is the latest trusted one, and every mention changes at each release.
- darktable's master: moves between releases (new parameter versions the stable release cannot read), so the references would shift under the baselines.

## Consequences

- Behaviour does not depend on how the docs name the release; the harness renders against the constant's version.
- The `port-module` skill's "Following an upstream darktable change" holds the steps above.
- AGENTS.md names the current number once, beside the constant.
