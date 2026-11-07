# What Apertrust is

**Status:** architecture decision 1 records this page, and decision 2 how Apertrust gets there: darktable's colour science and modules first, rewritten as darktable has them in Apertrust's interface, then Apertrust's own way once the whole project is migrated. Product decisions follow this page, and `docs/roadmap.md` carries what it means for the plan. Where the first pass differs, this page says so.

This page is about what photographers get from Apertrust and how it should feel to use. How it is built is covered elsewhere:

- `docs/architecture.md`: the architecture in overview, linking each area's document, `docs/engine.md` the engine among them;
- `docs/modules/README.md`: the modules.

## The promise

**Apertrust is a free, non-destructive, cross-platform raw photo editor. It aims for measured image quality, controls that keep to what they say, and help in reaching the look you want.**

For photographers who care how their pictures look, Apertrust gets them there quickly and predictably, on their own computer: the picture is right, every control behaves as promised, and the editor does the slider work towards the look they want.

## What it is built on

A major part of Apertrust is built on darktable's trusted colour science. It comes in two stages.

- **The migration, first.** darktable's processing and modules are rewritten for the GPU, and every step is compared with darktable's own render until it matches. The migration ends once, for the whole project, when every module of the first pass is in the app and matches, at the end of Phase 3.
- **Then Apertrust's own way.** From there Apertrust's own renders are its reference, and every change to how a photo looks is deliberate, measured and reviewed. Its processing may then evolve on its own, and it does not claim to reproduce darktable.

Nothing of darktable is loaded or linked at any stage, and edits made with the migrated modules keep looking exactly as they did.

## What only Apertrust does

1. **Image quality that is measured, not judged by eye.**
   - Every step of the processing is checked against a reference on every change.
   - Colour, highlights and shadows come out right, and look the same on every computer.
2. **Controls with guarantees.**
   - Every control's behaviour is written down and tested: what it changes, what it leaves alone, and what its default does.
   - The same slider works the same on the whole photo and on a layer.
3. **Editing towards a goal.** Show a photo, pick a few of your own edits, set an Auto target, keep a series consistent, match two cameras, or describe the change in words. The editor finds the slider values:
   - you get ordinary settings back, never a filter;
   - sliders you have already set stay as they are.
4. **Learns your look privately.** From a handful of your edits, on your own computer, with no cloud and no subscription.
5. **Assistants that check their own work.** A script or an AI assistant edits through the same sliders and history as you, and verifies the result by measurement before showing it.

## The opportunity

- **New editors look alike.** A dozen open-source raw editors appeared in 2026, most written in weeks. They copy the same layout and the same AI features, and photographers find them interchangeable. Where they fall short is the picture itself: highlights, shadows, colour.
- **darktable rewards experience.** Its picture quality is excellent, and it offers many modules and ways of working, so newcomers, especially those used to commercial editors, can take a while to find where to start.
- **Commercial editors keep their best new features in the cloud.** Learning a photographer's style and editing towards a goal need a subscription and thousands of past edits, or match only roughly.

Nobody gives photographers top picture quality, controls they can predict, and help that works the way they do. That is the gap Apertrust fills.

## Who we build for

Photographers who care how their pictures look and want to get there quickly. Two groups matter equally. When their needs pull apart, the rule under "When principles conflict" decides.

### The darktable veteran

- **Who:** knows what a raw file can give, and gets it from darktable.
- **Struggles with:**
  - hunting through dozens of modules;
  - three ways to do the same thing;
  - settings that interact in ways they must remember.
- **Wants:** the same results in a fraction of the time, without giving up control when a photo needs it.
- **Keeps:**
  - every parameter, in each module's tabs and collapsible groups;
  - curves;
  - typed-in values;
  - keyboard shortcuts;
  - layers wherever darktable needed a second instance of a module.
- **In the first pass:** darktable's own modules and parameters, without its legacy ones, in Apertrust's interface, so the results come without relearning.

### The subscription leaver

- **Who:** edits in a commercial editor and is tired of paying monthly and sending photos to the cloud.
- **Wants:**
  - a familiar way of working;
  - their looks to carry over;
  - no step down in quality.
- **Struggles with:** open-source editors that feel either like a cockpit or like a toy.
- **In the first pass:** darktable's modules, in darktable's words, in Apertrust's interface.

### Later

- photographers who deliver hundreds of photos per job: events, weddings, real estate;
- people who edit on phones and tablets.

## What they need to get done

1. **Develop a photo:** from the raw file to how it looked to me, quickly.
2. **Fix one part:** a face, the sky, a corner, without upsetting the rest.
3. **Make a series consistent:** one shoot, one look, even when the light changed.
4. **Reproduce a look:** my own style, or a photo I admire.
5. **Trust the result:** what I see is what I export, on any computer, today and next year.
6. **Hand off the routine:** to a preset, a batch, a script or an AI assistant, without losing control.

## Principles

Eight principles, following the three parts of the promise. Each one helps choose between two reasonable options. It says what it means in practice and what it rules out.

### Trust: the picture is right, and so is everything around it

#### 1. Quality is never traded for simplicity

- Simple controls sit on top of reference-grade processing. Making something easier never makes the picture worse.
- Power stays reachable, one level down, inside the same module.
- *Rules out:* simplifying by removing capability. Users resented this when darktable's forks tried it.

#### 2. What you see is true

- At 100% the screen shows exactly what the export will be. Smaller views use the same processing, scaled down.
- Clipping and out-of-gamut warnings, before and after, and the histogram tell the truth about the picture.
- An edit looks the same on every computer and after every update.
- *Rules out:*
  - previews made by cheaper processing than the export;
  - flattering displays that hide problems;
  - updates that quietly change old edits.

#### 3. Your work is safe, and it is yours

- The original file is never touched.
- Every change saves as you make it, and history and undo cover everything.
- Snapshots keep versions you want to compare or return to.
- Everything runs on your computer, with no account, cloud or subscription.
- Edits sit beside the photo in a readable file, so leaving is as easy as arriving.
- *Rules out:* a crash that costs work, uploading photos, telemetry, and lock-in formats.

### Ease: predictable, and quick to learn

#### 4. Start from a good picture

- Opening a raw file gives a pleasing, natural result straight away. Editing is refining, not rescuing.
- *Rules out:* flat or grey starting points that need ten modules before they look like a photo.

#### 5. Every control keeps to what it says

- Each control is a darktable parameter and does what darktable's does, checked against darktable.
- A module that is off leaves the photo untouched, as darktable's does: a module darktable leaves off on a new photo stays off until one of its controls changes, then works from darktable's own defaults, and its reset turns it off again (architecture decision 32). A module every photo runs is untouched at its defaults.
- No control depends on hidden settings or on the order it was used in. These hold now, and are tested.
- Local work uses the same sliders. Click the subject or the sky, brush, or drag a gradient, then adjust as you would anywhere else; a slider will behave the same on a layer as on the whole photo when layers come.
- *Rules out:*
  - controls whose effect depends on hidden settings, or on the order they were used in;
  - masks built inside each module's settings.

#### 6. One module for each tool, its depth inside it

- The modules are darktable's, one per algorithm, without its legacy and overlapping ones, in darktable's words.
- Depth comes from inside the module, in its tabs and collapsible groups.
- *Rules out:* duplicate modules, legacy alternatives, an expert maze, and copying a layout for its own sake.

### Help: it can set the sliders for you

#### 7. Show or tell the goal; the editor sets the sliders

You can show or tell it what you want:

- a photo to match;
- a few of your own edits to learn from;
- an Auto target, such as natural exposure and neutral white;
- a series to keep consistent;
- another camera to match;
- an instruction, such as "warmer, with more open shadows".

The editor then sets the sliders, working around what you have already decided:

- Sliders you have set yourself stay fixed until you release them.
- Matching has a strength control.
- *Rules out:*
  - one-click filters that produce pixels you cannot adjust;
  - automation that overwrites your choices.

#### 8. Help is visible, reversible and yours to accept

- Whatever the editor or an assistant changes appears as ordinary slider values in the history, with a short note on what it did. One undo reverts it.
- **Assistants** are a core capability, but not the headline:
  - scripts and AI assistants work through the same modules, layers, history and undo as a person;
  - they check their results before showing them;
  - they connect only when you allow it.
- **AI tools** (masks, denoise, erase) run on your computer. The editor is complete without them.
- *Rules out:*
  - hidden adjustments;
  - an automation interface that can do what the editor cannot show;
  - AI that needs the cloud;
  - generated content as a core feature.

## The quality bar

Requirements every feature meets. They are not principles, because nobody would choose the opposite.

- **Immediate.**
  - A slider moves and the picture follows, on a large file or a modest laptop.
  - Presets and styles preview on hover.
  - There are no progress bars while editing.
  - Without a GPU it still runs, more slowly.
- **First-class on Windows, macOS and Linux:** the same features and the same results on each.
- **Accessible.**
  - Everything can be reached by keyboard, and the interface scales.
  - Warnings and overlays stay readable for colour-blind users.
  - Screen readers work as far as the interface toolkit allows.

## When principles conflict

1. **Picture quality wins over convenience.**
2. **Predictable wins over clever.** A control that always behaves the same is better than one that guesses.
3. **Newcomers succeed without help; veterans' control is one level down, never removed.**
4. **The user's own choice wins over automation.**
5. **Fewer, better modules win over matching another editor's feature list.**
6. **Local and private wins over connected.**

## What we will not build

- **A darktable skin, or a copy of a commercial editor,** as where Apertrust ends up. The first pass shows darktable's modules on purpose, in Apertrust's own interface, to prove the rewrite; after the migration, Apertrust evolves on its own.
- **darktable's flexibility for its own sake:**
  - reordering the processing;
  - multiple copies of a module outside layers;
  - expert and legacy modules.
- **Accounts, subscriptions, cloud processing or telemetry.**
- **Generative image content.** Removing small distractions by filling from their surroundings is in scope; adding or replacing content is not.
- **Not yet:**
  - video;
  - archive-scale photo management, later;
  - a web version for a home server, later;
  - a native mobile app, planned after the desktop editor is ready for daily use.

## How we will know it works

The numbers are first targets; usability tests with photographers confirm or revise them.

| Promise | Measured by |
| --- | --- |
| Quality | During the migration, a darktable user sees no difference: every processing step matches darktable within a tight tolerance, on every supported GPU. After it, no photo changes its look without a deliberate, reviewed change to Apertrust's own baselines. Already true for the whole Phase 1 pipeline on the Apple M4, except two lossy Sony formats that wait for a decoder fix. Every operation's golden case also passes on an NVIDIA RTX 5060 and an AMD Radeon 780M; the whole-pipeline test is still to run there |
| Predictable controls | Every control's behaviour is written down and tested before it ships. 4 of 5 testers predict what a slider will do before moving it |
| Quick to learn | 4 of 5 testers new to Apertrust, from darktable or a commercial editor, get an edit they are happy with within 15 minutes |
| It sets the sliders | Most matching and Auto results are kept with at most 3 slider changes. An early test on real photo pairs reproduces the reference closely; its threshold is set with the test |
| What you see is true | The export equals the preview at 100%, and an edit renders the same after an update |
| Your work is safe | A crash never loses more than the last change |
| Worth switching | The gate after Phase 3: the author uses Apertrust instead of darktable every day. Later, in usability tests: editing real photos feels better than in darktable |

## What this means for the plan

- **Rewrite first, improve later.** darktable's modules come first, in Apertrust's interface, so the engine, the interface and the edit are proven on modules people already know. Once the whole project is migrated, Apertrust evolves on its own, and edits made before keep looking exactly as they did.
- **The rewrite is as close to darktable as it can be:** its modules, parameters, controls, defaults, order and results. It differs only in these ways: one view, Apertrust's design system, layers in place of instances, darktable's order kept fixed, tone mappers as modules of their own with one per edit, and darktable's legacy and overlapping modules left out. Both of darktable's white balance modules stay as darktable has them (architecture decision 27). Improvements come after the rewrite works.
- **The migration ends for the whole project at once,** when the first pass is complete. Until then darktable's renders are the reference; after it Apertrust's own are (architecture decision 2).
- **Prove "it sets the sliders" early.** A command-line test that matches a reference photo comes at the start of Phase 3.
- **Design the interface around the promise,** not around another editor's layout.
- **Usability tests with photographers,** against the targets above, and fixes from what they find.
- **Publish the quality results** where photographers can see them.
- **A mobile phase** after Phase 3.

## How the architecture delivers it

| Promise | Delivered by | Where |
| --- | --- | --- |
| Quality | During the migration, darktable's algorithms ported one by one, each tested against darktable by ΔE2000; after it, against Apertrust's own baselines (architecture decision 2) | `docs/testing.md`, `golden-tests` skill |
| What you see is true | One GPU engine for preview and export, with the viewport scaled after demosaic; a software GPU when there is no hardware one; pinned, versioned edits | Decisions 7, 12, 13 and 48 |
| Safe and yours | Local processing; `.apxmp` XMP sidecars written atomically; history in the session | Decision 48, `docs/engine.md` |
| Predictable controls | Modules over operations, one owner per parameter, invariants tables with tests; layers as masked operation instances | Decision 4, `tool-bindings` skill, `docs/layers.md` |
| Immediate | A GPU-resident pipeline, a viewport pipe and cache points | `docs/engine.md`, decision 12 |
| Sets the sliders | A differentiable pipeline: Slang derivatives of the operations after demosaic, checked against finite differences: exposure, AgX and colour balance rgb export them; colorin and colorout need theirs for matching | `docs/ai.md` (The differentiable pipeline) |
| Assistants | One session protocol for every client; MCP | `docs/architecture.md` (At a glance), `docs/ai.md` |
