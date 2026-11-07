## What this changes and why

<!-- A few plain sentences. -->

## Issue

<!-- "Closes #123". A change that is not small should have an issue, agreed before the work. -->

## Checks

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `tools/check-license-headers.sh`
- [ ] `tools/check-ui-strings.sh`
- [ ] `tools/build-kernels.sh --check`, when a Slang file changed (the generated WGSL is committed)
- [ ] `tools/build-api-docs.sh`
- [ ] `cargo deny check` and `tools/credits.sh --check`, when a dependency changed

For a kernel or a port, paste the results of `apertrust-golden run` (the ΔE2000 numbers) and `apertrust-golden gradient`:

```text

```

## Sign-off

- [ ] Every commit is signed off (`git commit -s`), certifying the [Developer Certificate of Origin](https://developercertificate.org/). I wrote or checked every line and can explain it.
