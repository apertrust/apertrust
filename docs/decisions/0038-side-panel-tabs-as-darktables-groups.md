# 0038. Side panel tabs as darktable's groups

## Context

The first pass presents darktable's modules ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)). Which tabs does the side panel have, which module goes in which, and in what order? darktable's darkroom shows its modules in groups, and its default "workflow: scene-referred" preset is what its users know.

## Decision

**The Edit workspace's tabs are Library, Base, Colour, Correct and Effect, then History:** darktable's scene-referred groups (`src/libs/modulegroups.c` at the reference release) between Apertrust's own Library and History.

- **Labels** are darktable's words in British English and sentence case. Their ids, as settings keep the tab, are darktable's names: `base`, `color`, `correct`, `effect`, beside `library` and `history`. Ukrainian and Polish start from darktable's own translations.
- **Icons** are darktable's shapes (`src/dtgtk/paint.c`) as one-colour icons drawn at 22 points, in the style of Lucide, the icons' foundation ([gui.md](../gui.md), Fonts and icons), with a filled form for the chosen tab: Base a circle; Colour darktable's circle, its gradient suggested by line work; Correct two half circles offset sideways; Effect a circle with three sparkles.
- **Order:** Library first ([0037](0037-one-view-the-library-as-a-tab.md)), the four groups in darktable's order, History last, so Ctrl+1 to Ctrl+5 reach the Library and the four groups.
- **Each module sits in darktable's group** and a tab lists its modules as darktable's right panel does, in pipeline order with the last module at the top:
  - Base: Local contrast, the edit's tone mapper, Crop, Tone equalizer, Exposure, Orientation, Rotate and perspective, Highlight reconstruction, White balance;
  - Colour: Colour balance RGB, Colour equalizer, RGB primaries, Colour calibration;
  - Correct: Sharpen, Retouch, Haze removal, Chromatic aberrations, Lens correction, Denoise (profiled);
  - Effect: Vignetting, Grain, Diffuse or sharpen.
- **Modules the preset puts in no group** take the tab darktable's other layouts give them: Raw black/white point, Demosaic, RGB curve and RGB levels in Base; Input and Output colour profile in Colour; Soften and Bloom in Effect. The lists are kept in [modules/README.md](../modules/README.md), and a test holds each tab's list to `OPERATIONS` reversed.
- **No quick access panel and no active-modules tab.** darktable's module search may come later as its own piece of work.
- **A tab with nothing behind it yet** (History until it is built) is drawn disabled in its place, saying it is to come, with no key.
- **Edit opens on Base.** A tab id in settings that this build does not know, or one drawn disabled, opens Base with a warning, and the id stays in the file until a tab is chosen.
- **Scopes** are pinned over every tab's modules but the Library's; Layers join the four groups when they come.
- **Styles** go in the Library tab, as darktable's styles module is the lighttable's.

## Options considered

- Apertrust's own tabs (a Profile, Adjust, Optics and Detail split): unlike darktable's, against the rewrite.
- History beside the Library: it takes Effect's Ctrl+5.
- The groups before the Library: the Library is first in the one view.
- Not drawing tabs that are not built: Ctrl+1 would move when the Library comes.
- darktable's coloured Colour icon: against the one-colour icon rule.
- The preset's listing order, which darktable never shows on screen.
- Opening on Colour, where darktable's default lands only by a renumbering.
- The modules in no group reached through search only: it would need search built first and break the rule that a workspace holds every module.

## Consequences

- The app's `TABS` and `TAB_MODULES` hold these lists; a module added to `OPERATIONS` gets its tab by this rule.
- A port's module page names its tab from darktable's preset.
