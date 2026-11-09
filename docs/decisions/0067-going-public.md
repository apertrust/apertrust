# 0067. Going public

## Context

Apertrust is developed in the open. Where does the project live, how is AI assistance in its development disclosed, how is the origin of each contribution certified, and what else does a public project need?

## Decision

**The organisation `github.com/apertrust`,** with the main repository `apertrust/apertrust`, described as "A free, non-destructive, cross-platform raw photo editor". Two-factor authentication is required and only owners create repositories.

**The domain apertrust.org.** The sidecars' XMP namespaces are on it ([0048](0048-the-apxmp-format.md)).

**AI assistance is disclosed once, at repository level:** the README and [AI_POLICY.md](../../AI_POLICY.md) say Apertrust is built with heavy AI assistance, its darktable ports above all. Every contribution is judged by the same rules, however it was made: the contributor is accountable for every line and able to explain it, changes are small and scoped to an issue, CI and the golden checks pass, ports keep darktable's provenance ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)). An `Assisted-by:` or `Co-Authored-By:` trailer is allowed but never required. AGENTS.md and the project's skills help AI-assisted contributors meet the rules.

**The DCO:** every commit carries a human's `Signed-off-by:` (`git commit -s`). An agent never signs off in an AI's name; it adds the maintainer's sign-off only to a commit it makes for him. `.github/workflows/dco.yml` fails a pull request with a commit lacking its author's sign-off, bots' commits exempt.

**The repository's documents and checks:** [CONTRIBUTING.md](../../CONTRIBUTING.md), [SECURITY.md](../../SECURITY.md) (private vulnerability reports), [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) (the Contributor Covenant 2.1), [GOVERNANCE.md](../../GOVERNANCE.md); issue forms and a pull request template in `.github/`; CodeQL for Rust, the OpenSSF Scorecard, and SonarQube Cloud's automatic analysis; every action pinned by its commit SHA, Dependabot updating cargo and the actions weekly.

**Two websites:** `apertrust/apertrust.org`, the site at apertrust.org, and later `apertrust/apertrust-docs` at docs.apertrust.org (planned). Both are built with Astro, the documentation with Starlight for its documentation features, languages and interactive components, served by GitHub Pages with the organisation's domain verified. Their content is CC BY-SA 4.0 and their code MIT, the name and logo excluded; npm dependencies are kept few, the lockfile committed, Dependabot on and actions pinned.

**The agent kit,** the skills, agents and commands, is a public repository of its own, `apertrust/agent-kit`, optional for contributors; the rules every contribution must meet are in AGENTS.md and are checked by CI and review, with the kit or without it. The hooks stay in the main repository (`.claude/settings.json`, `.claude/hooks/`): they run the build's own checks, which CI enforces for everyone.

**Working material stays out of the repository:** research, deliberation before a decision, session reports, product ideas and usability-test notes. What is decided is recorded here: the decision records, the specs, the plan.

## Options considered

- A trailer on every AI-assisted commit: a missing one proves nothing and cannot be enforced.
- Banning AI-assisted contributions: unenforceable, and against how the project is built.
- A CLA in place of the DCO: heavier for contributors, with nothing the GPL project needs.
- Zola, Hugo or mdBook for the websites: weaker documentation features, languages and components than Starlight.
- Hosting the websites on a registrar's free plan: small, uploaded by FTP, with no versioning.
- The agent kit inside the main repository: tooling for one kind of contributor in everyone's checkout.

## Consequences

- AGENTS.md's writing rules require the sign-off; `/draft-commit` and `/draft-pr` say so.
- Nothing personal or private belongs in the repository's files or commits.
