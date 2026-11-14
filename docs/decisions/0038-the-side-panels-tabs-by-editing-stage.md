# 0038. The side panel's tabs, by editing stage

## Context

Which tabs does the Edit workspace's side panel have, which tool goes in which, and in what order?

darktable's darkroom shows its modules in groups. Its default "workflow: scene-referred" preset (`src/libs/modulegroups.c` at the reference release) groups them by the kind of processing, base, color, correct and effect, and its right panel lists each group's modules in pipeline order, the last at the top. Those groups do not follow the order people edit in, so a normal edit crosses every tab: white balance and exposure in base, colour in color, denoise and lens correction in correct, then back to base for the tone mapper and local contrast. Within a tab the first tool of an edit is at the bottom.

The first pass mirrors darktable's modules ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), in Apertrust's own interface (principles, What we will not build). Tabs are display order only: what a tab lists never changes the processing order ([0022](0022-pipeline-order-and-the-default-pipeline.md), [0083](0083-the-side-panels-layout-dragged-and-floating.md)).

## Decision

**The Edit workspace's tabs are Photos, Adjust, Detail, Geometry, Effects, Raw and History, grouped by editing stage so that a normal edit stays in one tab.** A built-in darktable workspace keeps darktable's scene-referred groups for people coming from darktable (planned, with named workspaces). Edit stays the default.

### Edit's tabs

- **Order:** Photos first ([0037](0037-one-view-the-photos-tab.md)), the five editing tabs, History last. Ctrl+1 to Ctrl+5 reach Photos to Effects. Raw and History have no default key, since Ctrl+6 is Ctrl+− on French AZERTY keyboards, whose 6 key types −; the settings file's `keys` table can give each one (`panel-tab-6`, `panel-tab-7`).
- **What each holds,** top to bottom in editing order, the order a person reaches for them:
  - Adjust: White balance, Colour calibration, Exposure, the edit's tone mapper, Tone equalizer, Local contrast, Haze removal, RGB curve (the Tone curve at its place until RGB curve replaces it), RGB levels, Colour balance RGB, Colour equalizer, RGB primaries;
  - Detail: Denoise (profiled), Sharpen, Diffuse or sharpen, Retouch (retouch layers, Phase 4: [0074](0074-masks-ported-from-darktables-master.md));
  - Geometry: Crop, Rotate and perspective, Orientation, Lens correction, Chromatic aberrations;
  - Effects: Vignetting, Grain, Soften, Bloom;
  - Raw, in processing order: Raw black/white point, Highlight reconstruction, Demosaic, Input colour profile, Output colour profile.
- **Why these groups:**
  - Adjust holds light and colour together, because a scene-referred edit interleaves them: the white balances, exposure, the tone mapper, then contrast and colour.
  - Detail holds what works at the scale of pixels, Geometry what changes the frame and corrects the lens, Effects the finishing looks.
  - Raw holds the raw stage's tools and the colour profiles, set up rather than edited and rarely touched, in a tab of their own, out of the way; their processing order is how they are set up.
- **Display order only.** The processing order stays the pipeline's whatever a tab lists. These lists are the Edit workspace's defaults: a person can reorder, pin, move and float sections on screen ([0083](0083-the-side-panels-layout-dragged-and-floating.md)), and the side panel's ⋮ › Processing Order sorts a tab's tools into pipeline order.
- **Labels:** one plain word each, Apertrust's own, in British English and sentence case: Adjust, Detail, Geometry, Effects, Raw; American English has the same words. Ukrainian and Polish translations follow, reviewed as every translation is.
- **Ids,** as settings keep the tab: `adjust`, `detail`, `geometry`, `effects` and `raw`, beside `photos` and `history`; their messages are `tab-` and the id.
- **Icons:** one colour, in Lucide's style, drawn at 22 points, each with a filled form for the chosen tab ([0081](0081-icon-only-buttons-draw-no-plates-on-is-the-filled-icon.md)); drawn with the wireframe.
- **Edit opens on Adjust.** A tab id in settings that this build does not know, or one drawn disabled, opens Adjust with a warning, and the id stays in the file until a tab is chosen. darktable's ids, `base`, `color`, `correct` and `effect`, which earlier builds kept for Edit, and `library`, which they kept for Photos, are such ids in Edit, never given to another tab.
- **Scopes** are pinned over the five editing tabs' tools by default, in neither the Photos tab nor History; Layers join them when they come.
- **No quick access panel and no active-modules tab.** darktable's module search may come later as its own piece of work.
- **A tab with nothing behind it yet** is drawn disabled in its place, saying it is to come, with no key.
- **Styles** go in the Photos tab, as darktable's styles module is the lighttable's.

### The darktable workspace

Planned, with named workspaces in Phase 3: a built-in workspace beside Edit, id `darktable`, named "darktable" in every language, chosen from View › Workspace as any workspace is.

- **Its tabs** are Photos, Base, Colour, Correct, Effect and History: darktable's "workflow: scene-referred" groups between Apertrust's own Photos and History. Ctrl+1 to Ctrl+5 reach Photos and the four groups; History has no default key. It opens on Base.
- **Labels** are darktable's words in British English and sentence case; their ids are darktable's names, `base`, `color`, `correct` and `effect`, their messages `tab-base`, `tab-color`, `tab-correct` and `tab-effect`. Ukrainian and Polish start from darktable's own translations.
- **Icons** are darktable's shapes (`src/dtgtk/paint.c`) as one-colour icons drawn at 22 points in Lucide's style, with a filled form for the chosen tab: Base a circle; Colour darktable's circle, its gradient suggested by line work; Correct two half circles offset sideways; Effect a circle with three sparkles.
- **Each tool sits in darktable's group,** and a tab lists its tools as darktable's right panel does, in pipeline order with the last tool at the top. A tool the preset puts in no group takes the tab darktable's other layouts give it: Raw black/white point, Demosaic, RGB curve and RGB levels in Base; Input and Output colour profile in Colour; Soften and Bloom in Effect.
  - Base: Local contrast, the edit's tone mapper, RGB levels, RGB curve, Crop, Tone equalizer, Exposure, Orientation, Rotate and perspective, Demosaic, Highlight reconstruction, White balance, Raw black/white point;
  - Colour: Output colour profile, Colour balance RGB, Colour equalizer, RGB primaries, Colour calibration, Input colour profile;
  - Correct: Sharpen, Retouch (retouch layers, Phase 4), Haze removal, Chromatic aberrations, Lens correction, Denoise (profiled);
  - Effect: Vignetting, Soften, Grain, Bloom, Diffuse or sharpen.
- **Scopes** are pinned over the four groups' tools; Layers join them when they come.

## Options considered

- **darktable's groups as Edit's tabs:** a normal edit switches tab at every stage, and each tab reads bottom to top. Kept as the darktable workspace instead.
- **Tabs by stage with light and colour apart:** a scene-referred edit still crosses tabs, white balance, exposure and colour calibration going back and forth.
- **One long list of every tool:** too long at more than 60 tools once the migration ends.
- **The raw stage's tools as a collapsed group at the bottom of Adjust:** they would still lengthen the tab every edit uses; a tab of their own keeps them out of the way.
- **Other names for Geometry:** "Crop", "Lens" and "Frame" each name one tool or one cause and miss the others, and "Frame" reads as a border; "Shape" is vague and meets the masks' shapes. Geometry names what every tool there changes: where the pixels land.
- **Names borrowed from one editor's layout:** Apertrust's interface is its own, not a lookalike (principles); each name is a plain word.
- **History beside Photos:** it would take a key from an editing tab.
- **Opening on another tab:** Adjust holds the tools every edit starts with.
- **The tools darktable's preset puts in no group reached through search only,** in the darktable workspace: it would need search built first, and the workspace would lack tools it should hold.
- **darktable's coloured Colour icon** in the darktable workspace: against the one-colour icon rule.

## Consequences

- The app's `TABS` and `TAB_TOOLS` hold Edit's lists, the defaults the layout shown starts from ([0083](0083-the-side-panels-layout-dragged-and-floating.md)); a test holds them as recorded here and each first-pass tool in exactly one of Edit's tabs. A tool added to `OPERATIONS` gets its tab in this record and in [tools/README.md](../tools/README.md).
- The darktable workspace's lists, the ones `TAB_TOOLS` holds as built, are kept with the test holding them to `OPERATIONS` reversed; the workspace is chosen once named workspaces come.
- The tab ids, labels and icons in four languages: `tab-adjust` to `tab-raw` in Fluent, five icons, the settings schema's tab ids.
- A tool page names its tab from [tools/README.md](../tools/README.md).
