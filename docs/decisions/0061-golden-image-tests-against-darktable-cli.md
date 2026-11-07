# 0061. Golden-image tests against darktable-cli

## Context

Image quality must be judged by measurement, never by eye (principle 1). During the migration ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)) each port must render what darktable renders, and the check needs darktable-cli and its test images, which hosted CI should not have to download or run on every push. Results also differ slightly between GPUs.

## Decision

**Each port is checked by ΔE2000 against darktable-cli at the reference release** ([0003](0003-the-reference-is-darktables-latest-stable-release.md)), with a tolerance per case, by the golden-image harness, `apertrust-golden`:

- a case has darktable render a module's input and expected output from darktable's integration-test images (`APERTRUST_TEST_IMAGES`), cached in `target/golden`; the port passes when its kernel's output is within the case's ΔE2000 tolerance; heatmaps show where it differs;
- the raw stage has its own checks: `raw-passthrough`, `raw-run` and `raw-defaults`; the whole pipeline has `pipeline` ([0062](0062-the-whole-pipeline-comparison.md));
- `gradient` checks every kernel's derivatives against finite differences and forward against reverse mode ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md));
- a case whose operation has several passes or scratch runs as a one-instance engine plan, so the harness has no executor of its own;
- a baseline or tolerance never changes automatically: a change is deliberate and reviewed, and a failing case is explained, not re-rendered.

**The golden suite runs locally,** where darktable-cli and the images are; the `golden-tests` skill says how. Self-hosted runners may run it later.

**CI on GitHub runs the rest** (`.github/workflows/ci.yml`): formatting, clippy, cargo-deny, licence headers, the UI-string check, and the tests on Linux (lavapipe), Windows (WARP) and macOS (Metal), the synthetic gradient check among them. `timing.yml`, run by hand, times a 61 MP export and exposure changes on lavapipe and WARP.

**Baselines that are not darktable's:** the raw decoder's output is pinned by a corpus of 40 raw.pixls.us samples (`crates/raw/tests/corpus/expected.txt`), blessed only on purpose ([0023](0023-rawler-its-camera-data-fixed-upstream.md)); the design system's gallery snapshots allow 2/255 a channel and 2 pixels between GPUs ([0036](0036-the-design-system.md)).

**Measuring:** ΔE baselines are kept per GPU. Benchmarks warm up for a fixed time, alternate the variants they compare and use timestamp queries.

**After the migration** the baselines are Apertrust's own renders, taken at the commit that completes it; each change to one is a reviewed decision with its size ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Options considered

- Visual review: not reproducible, and blind to small shifts.
- The golden suite on hosted CI: darktable-cli and the images on every push, for checks that change only with kernels.
- One baseline for every GPU: drivers differ within the tolerance, and a shared baseline would hide which.
- darktable as the reference for good: it would tie every later improvement to darktable's behaviour.

## Consequences

- A port is done only with its golden case and gradient check passing, its numbers recorded in its spec.
- The turn-end hooks run the golden checks where `APERTRUST_TEST_IMAGES` is set, otherwise the synthetic gradient check.
