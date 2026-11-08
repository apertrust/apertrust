# Testing

How Apertrust knows its images are right: image quality judged by measurement, never by eye. The golden-image harness came before any engine code, because image quality, not feature count, is the hard part. How to run the checks is in AGENTS.md and the `golden-tests` skill; what each part of the engine tests is [engine.md](engine.md#testing)'s.

**Status.** Built: the golden-image harness against darktable-cli with per-operation cases, the raw stage's checks, the whole-pipeline exit test, the gradient check, and every module's invariants tables as tests. Planned: homelab GPUs as self-hosted runners, published results, and the baselines after the migration.

**The checks:**

- **Golden cases:** each ported operation against darktable-cli by ΔE2000 ([below](#the-golden-image-harness-during-the-migration)).
- **The whole-pipeline test,** Phase 1's exit: the raw.pixls.us samples through the whole pipeline against darktable-cli's 16-bit render (decision 62; [engine.md](engine.md#testing), The whole pipeline).
- **The gradient check:** every operation's derivatives against finite differences, and forward against reverse mode ([ai.md](ai.md#the-differentiable-pipeline), rule 5).
- **Invariants:** each module's invariants table, every row a test ([modules/README.md](modules/README.md)), and the engine's own invariants, a module at its defaults or off leaving the render byte for byte as it was.
- **The raw corpus:** rawler's output pinned on the 40 samples, so each update is deliberate.

## The migration, then Apertrust's own way

Decision 2. Apertrust migrates darktable's colour science in Phases 0 to 3. The migration ends once, for the whole project, when every darktable module of the first pass is in the app and passes its comparisons, at the end of Phase 3 ([roadmap.md](roadmap.md)).

- Until then darktable-cli's renders are the references, as below.
- From then on the baselines are Apertrust's own renders, taken at the commit that completes the migration, and each change to one is a deliberate, reviewed decision, its size measured and stated. Apertrust may then evolve its processing on its own and claims no dependency on darktable.
- Nothing of darktable is ever loaded or linked, before or after, and edits made with the migrated modules carry over exactly.

## The golden-image harness, during the migration

Decision 61. For each ported operation, the same input buffer and parameters go to `darktable-cli` and to the engine, and the outputs are compared with a ΔE2000 threshold. darktable's integration-test images provide starting cases, and any darktable edit can become one, since the harness decodes darktable's parameters itself. Porting an operation becomes a well-defined task an agent loops on until the test passes.

- **References** come from darktable-cli at the reference release (decision 3), pinned by one constant: the harness refuses any other version. darktable renders each case twice, with the module off and on, as full-resolution linear Rec2020 float images; the port runs on the first and must match the second. Later tone mappers are switched off in both renders, and masks removed, so the two differ only by the module.
- **Where it runs:** locally, with renders cached in `target/`, because hosted CI cannot install the pinned darktable. Hosted CI runs lint and the tests on software GPUs, in three layers (decision 70): lavapipe on Linux on every pull request; WARP on Windows when a pull request changes a Windows path or is labelled `full-ci`, Metal on macOS with the label; all three every night. Later, homelab machines join as self-hosted runners with darktable installed and run the suite on NVIDIA, AMD, Apple and Intel hardware.
- **Operations of several passes,** or with scratch buffers, run their cases as a one-instance plan of the engine on the case's input, and their gradient check through the engine's dispatch of the derivative entry points of the pass that writes the output, over the scratch the forward passes left (decision 15).
- **Tolerances** are per operation, not bit-exact output; a change in one vendor's result over time is a regression signal. Each GPU's ΔE results are kept as a baseline so such a change shows up; the Radeon 780M already differs slightly from NVIDIA and Metal on colour balance rgb.
- **Timing:** each variant warms up for a fixed time, variants alternate instead of running in order, and GPU timestamp queries replace wall-clock time. Timing variants in order once made the second look 40% slower on the RTX 5060.
- **Results are published** where photographers can see them, not only in the repository (decision 1): during the migration, each operation's ΔE2000 against darktable and the whole-pipeline test, per GPU as the homelab runners produce them; after it, each baseline change with its size. Where and in what form is not settled yet.
- **After the migration,** each case's baseline is Apertrust's render at the commit that completes it, checked by the same tolerances, per GPU. Where those baselines are kept, and what each case's input becomes, are designed before the end of Phase 3 ([Open questions](#open-questions)).

## Departures from darktable

Decision 63, a rule of the migration. After it, a processing change is a baseline change, and no darktable behaviour is kept behind a harness setting.

- A port's export departs from what darktable-cli exports reproducibly, which the golden cases compare, only by an agreed difference.
- darktable's behaviour then stays behind a setting only the harness reaches, never in a schema, an edit or a module, so the golden cases still check everything else against darktable and an invariant test checks the departure. The spec and the decision record state its size, measured, and the published results name it.
- The agreed differences are listed in their specs: grain's seed and vignetting's raised-cosine fall-off (both planned), and AgX's dynamic range scaling and Curve y gamma. The tone equalizer has none and renders darktable's curve (decision 22).
- How smaller views compute an operation follows principle 2, the same processing at their scale measured against Apertrust's own export, not darktable's preview pipe, so it is not a departure in this sense. A branch darktable takes on a length scaled to the pipe is taken from the full-resolution length, as darktable-cli's export takes it at any size.
- A default can differ too: it is not processing, so the comparison gives darktable Apertrust's value and needs no setting of its own. None differs today; an unedited photo's illuminant is darktable's own, snapped as `reload_defaults` snaps it (decision 27). Colour balance rgb is not such a default: it runs only while its module is used, as in darktable (decision 32).
- Differences within a case's tolerance (fp32 redesigns, guards, the order of arithmetic), and choices where darktable's own result is not reproducible (it depends on how its binary rounds, or its solve fails), are not departures in this sense; their specs record them.

## Open questions

- **The golden harness after the migration** (decision 2), designed before the end of Phase 3: where Apertrust's own baselines are kept, per GPU, since full-resolution renders are too large to commit (an artefact store with checksums, or re-rendered from the commit that completes the migration); what each case's input becomes, today darktable's render with the module off; and how a baseline change is reviewed and recorded. A decision of its own, with its options, once the first pass's last modules are in sight.
- **The port's departure rule** (decision 63): a port departs from darktable only where a difference is agreed, and its spec lists each one. Where one changes what the export renders, darktable's behaviour stays behind a setting only the golden harness reaches and the difference's size is measured and stated; smaller views follow the export at their scale (principle 2). The agreed differences are grain's seed, vignetting's raised-cosine fall-off and AgX's dynamic range scaling and Curve y gamma; the tone equalizer has none. The rule itself is open to review.
