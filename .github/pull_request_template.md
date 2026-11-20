## What this changes and why

<!-- A few plain sentences. -->

## Issue

<!-- "Closes #123". A change that is not small should have an issue, agreed before the work. -->

## Checks

- [ ] `pre-commit run --hook-stage manual --all-files`, every check CI runs but the tests (CONTRIBUTING.md, Checks)
- [ ] `cargo test --workspace`

For a kernel or a port, paste the results of `apertrust-golden run` (the ΔE2000 numbers) and `apertrust-golden gradient`:

```text

```

## Sign-off

- [ ] Every commit is signed off (`git commit -s`), certifying the [Developer Certificate of Origin](https://developercertificate.org/). I wrote or checked every line and can explain it.
