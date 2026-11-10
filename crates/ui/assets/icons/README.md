# Icons

The interface's icons, one SVG file each, drawn by `apertrust-ui` (`src/icons.rs`, `docs/gui.md`). They are Apertrust's own files under Apertrust's names. Most are derived from [Lucide](https://lucide.dev)'s icons, release 1.70.0; where Lucide has no icon that reads as the thing, one is drawn for Apertrust in Lucide's style.

## Licences

- **Derived from Lucide:** Lucide's icons are under the ISC licence, © Lucide Icons and Contributors, and those Lucide derived from the Feather project under the MIT licence, © 2013-present Cole Bemis. `LICENSE-Lucide` is Lucide's `LICENSE` verbatim, with both notices. Each derived file's first line names its source and licence: `<!-- Derived from Lucide's "zoom-in" (https://lucide.dev), itself derived from Feather, MIT licence; mirrored left to right -->`, or `…, ISC licence` for one not from Feather. `CREDITS.md`, which About shows, carries both notices.
- **Drawn for Apertrust:** under the GPL like the code, with no Lucide notice.

| Icon | Source |
| --- | --- |
| `chevron`, `chevron-end` | Lucide's `chevron-right`, from Feather |
| `chevron-down` | Lucide's `chevron-down`, from Feather |
| `back` | Lucide's `chevron-left`, from Feather |
| `up-down` | Lucide's `chevrons-up-down` |
| `wand` | Lucide's `wand` |
| `reset` | Lucide's `rotate-ccw` |
| `presets`, `menu` | Lucide's `menu` |
| `options` | Lucide's `ellipsis` |
| `options-vertical` | Lucide's `ellipsis-vertical` |
| `more` | Lucide's `chevrons-right`, from Feather |
| `pick` | Lucide's `pipette` |
| `line` | Drawn for Apertrust |
| `undo` | Lucide's `undo-2` |
| `redo` | Lucide's `redo-2` |
| `copy`, `apply` | Drawn for Apertrust: an arrow out and an arrow in, Copy edit's and Apply edit's |
| `import` | Lucide's `download`, from Feather |
| `export` | Lucide's `upload`, from Feather |
| `settings` | Lucide's `settings` |
| `photos-selected`, `photos-selected-filled` | Lucide's `layout-grid` |
| `proof-margin`, `proof-margin-filled` | Drawn for Apertrust: a photo inside a wide margin, the proof margin's |
| `before-after`, `before-after-filled` | Lucide's `square-split-horizontal` |
| `grid`, `grid-filled` | Lucide's `grid-3x3` |
| `plus` | Lucide's `plus`, from Feather |
| `zoom-in`, `zoom-out` | Lucide's `zoom-in` and `zoom-out`, from Feather, mirrored left to right: the lens at the top right, the handle at the bottom left |
| `sort-ascending`, `sort-descending` | Lucide's `arrow-up-narrow-wide` and `arrow-down-wide-narrow` |
| `check` | Lucide's `check`, from Feather |
| `warning` | Lucide's `triangle-exclamation-point` |
| `lock` | Lucide's `lock`, from Feather |
| `edited` | Lucide's `pencil` |
| `labels`, `labels-filled` | Lucide's `eye` |
| `star`, `star-filled` | Lucide's `star` |
| `reject` | Lucide's `circle-x` |
| `rotate-left`, `rotate-right` | Lucide's `rotate-ccw-square` and `rotate-cw-square` |
| `flip-horizontal`, `flip-vertical` | Lucide's `triangles-centerline-dashed-vertical` and `triangles-centerline-dashed-horizontal` |
| `history`, `history-filled` | Lucide's `rotate-ccw-clock` |
| `folder`, `folder-filled` | Lucide's `folder` |
| `metadata`, `metadata-filled` | Lucide's `tag` |
| `base`, `color`, `correct`, `effect` and their filled forms | Drawn for Apertrust: darktable's module group shapes (decision 38) |
| `histogram`, `waveform`, `parade`, `vectorscope`, `log-scale`, `clip-shadows`, `clip-highlights` | Drawn for Apertrust: the scopes' graphs and toggles |
| `minimise`, `maximise`, `restore`, `close` | Drawn for Apertrust: the window's caption buttons, as the systems draw theirs |
| `mark` | Drawn for Apertrust: the app's mark |

`Icon::file` in `src/icons.rs` is the one place an icon is mapped to its file, its size and the Lucide icon it is derived from.

## Rules

Every icon follows the same rules, so it takes the theme's colour and stays sharp at any scale:

- **One colour, `currentColor`.** Strokes, and the fills of dots and filled forms, use `currentColor`; no other colour appears. Nothing is partly transparent: no `opacity`, `fill-opacity` or `stroke-opacity`, so every part is solid or an outline. The app renders the file with resvg at the exact pixel size it needs, in white, and tints it with a theme token.
- **Lucide's grid:** `viewBox="0 0 24 24"` with `width` and `height` 24; shapes a unit inside the box, their strokes' centres between 2 and 22; round caps and joins; outlines; Lucide's corner radii, 2 units, 1 on small rectangles; about 2 units between distinct parts. The caption buttons and the mark keep boxes of their own, the size they are drawn at: 10 and 20.
- **Sizes:** `Icon::size` is the size an icon is drawn at, in points: 14 for header and control icons, 16 for the top panel's and the central bar's, 22 for the side panel's tabs; 10 and 12 for chevrons, the check, the caption's lock, the grid's marks, the rating's stars and reject, and the scopes' clipping toggles.
- **Stroke:** every line is drawn 1.25 points wide whatever the icon's size (`icons::STROKE`, Lucide's `absoluteStrokeWidth`). The files keep Lucide's 2-unit stroke, set once on the `<svg>` element and nowhere else, and the renderer sets it to 1.25 × 24 ÷ the size drawn in points: 3 units at 10 points, 2.14 at 14, 1.36 at 22. The caption buttons keep their 1-unit strokes on half-unit coordinates, thin and sharp at every whole scale, and the mark its own widths.
- **Shapes:** `<path>`, `<circle>`, `<rect>` and `<line>`; no text, images, gradients, filters or masks, but the exceptions below.
- **Filled forms**, `<name>-filled.svg`, only to show a state: the chosen side panel tab, a central bar or grid bar toggle that is on, and a rating's points (`Icon::filled`). Each is its outline's shapes, in its box and stroke, filled. Where a line crosses the filled shape, a mask cuts it out, so the icon stays one colour on any background: one `<mask id="cut">`, a white square with the lines in black, applied with `mask="url(#cut)"`.
- **The mark**, `mark.svg`, the top panel's: the app's mono mark (`crates/app/assets/icons/svg/apertrust-mark-mono.svg`) redrawn in a 20-unit box, the one icon of filled shapes rather than lines. Its corners are rounded by its shapes' own strokes, in its own widths, and its cut is a mask as a filled form's is.

A new icon starts from Lucide's where Lucide has one that reads as the thing: its shapes under Apertrust's name, the `<svg>` element as the others have it, adapted where needed, with the first line naming its source and licence (and the table above). Otherwise draw it in Lucide's style ([its design guide](https://lucide.dev/guide/design/icon-design-guide)). Either way it gets a variant in `Icon`, its line in `Icon::file` and a place in `Icon::ALL`. The tests read every file, check the rules above, render it, hold every line to 1.25 points, and check each derived file's notice against `LICENSE-Lucide`.
