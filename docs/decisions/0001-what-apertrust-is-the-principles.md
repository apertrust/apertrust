# 0001. What Apertrust is: the principles

## Context

What is Apertrust, beyond a rewrite of darktable, and what do product decisions follow? A free editor needs a promise it can keep and measure, users it is built for, and rules for when goals pull apart. Without them every interface choice is argued from taste.

## Decision

**[principles.md](../principles.md) is the basis of every product decision.** It holds:

- **The promise:** a free, non-destructive, cross-platform raw photo editor that aims for measured image quality, controls that keep to what they say, and help in reaching the look you want. It names no other product.
- **What it is built on:** darktable's trusted colour science, migrated first, then evolved in Apertrust's own way ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).
- **The users:** photographers who care how their pictures look and want to get there quickly. The darktable veteran and the subscription leaver matter equally; photographers delivering hundreds of photos a job, and people editing on phones and tablets, come later.
- **Eight principles in three groups:**
  - Trust: quality is never traded for simplicity; what you see is true; your work is safe, and it is yours.
  - Ease: start from a good picture; every control keeps its promise; one place for each task, in words you know.
  - Help: show or tell the goal and the editor sets the sliders; help is visible, reversible and yours to accept.
- **The quality bar**, which every feature meets: immediate (the picture follows a slider, presets and styles preview on hover, no progress bars while editing, it still runs without a GPU); first-class on Windows, macOS and Linux, with the same results on each; accessible (keyboard, scaling, colour-blind-safe overlays, screen readers as far as the toolkit allows).
- **When principles conflict:** picture quality over convenience; predictable over clever; newcomers succeed without help while veterans' control is one level down, never removed; the user's choice over automation; fewer, better modules over a feature list; local and private over connected.
- **What will not be built:** a darktable skin or a copy of a commercial editor as where Apertrust ends up; reordering the processing, copies of a module outside layers, expert and legacy modules; accounts, subscriptions, cloud processing or telemetry; generative content. Not yet: video, archive-scale photo management, a web version for a home server.
- **First numeric targets** for each promise, measured as the principles' table says.

**Assistants are a core capability but not the headline.** They work through the same modules, history and undo as a person.

**A native mobile app** for phones and tablets, designed for touch, is planned after Phase 3.

**Image-quality results are published** where photographers can see them.

## Options considered

- A darktable rewrite with a friendlier interface as the identity: other editors already offer that, and it gives no reason to switch.
- A commercial editor's layout as the identity: copies a look, not a promise, and inherits its compromises.
- Agents as the headline: help is a means to the photo, not the product.

## Consequences

- Product decisions cite a principle; where principles conflict, the conflict rules decide.
- The interface follows familiar words and better behaviour, not another editor's layout.
- The plan carries the principles' consequences: the matching test ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md)), published results, and the mobile phase after Phase 3.
- Open: the matching test's threshold, set with the test; where and how quality results are published; the mobile phase's scope and exit criteria, set when it is planned.
