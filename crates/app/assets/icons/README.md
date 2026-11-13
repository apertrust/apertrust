# Icons

Apertrust's app icon, the Slider A: a serif A, a hairline and a heavy stem on bracketed feet under a beaked apex, whose crossbar is a slider's track running past both legs, its marker held in the counter. The track breaks round the marker, and the break is a cut, painted with nothing: whatever is behind shows through it, and it sets the marker apart without colour.

- **The drawing** is on a 64-unit grid: the apex at 6, the baseline at 58.5, the track straight from 3 to 61 with round ends beyond, reaching 1.5 and 62.5 (1 and 63 in the simplified drawing); in the logo its right end is straight to 57, reaching 58.5. At 32 px and below each icon is drawn from its `-small` SVG, the simplified drawing: a heavier hairline, plain slab feet, a thicker track and a larger marker.
- **The colours:** the A ink #1E1A16 on light grounds and cream #F4EEE2 on dark ones; the marker #D66A20 on light, #DC7028 on dark. The single-colour files draw the marker in the A's colour, the cut keeping it apart.

## SVG sources (`svg/`)

| File | Use |
| --- | --- |
| `apertrust-macos.svg` | macOS app icon, 1024 canvas, 824 plate centred with shadow (Apple's grid): a flat cream plate (#F4EEE2) with a hairline rim, the mark's grid 72% of the plate's width |
| `apertrust-macos-dark.svg` | The same on a dark plate (#1E1A16), the A cream, for macOS's dark icon appearance |
| `apertrust-macos-small.svg`, `apertrust-macos-dark-small.svg` | The same with the simplified drawing, for 32 px and below |
| `apertrust-plate.svg`, `apertrust-plate-dark.svg` | The plates alone, full-bleed, for design tools and the web |
| `apertrust-square.svg`, `apertrust-square-small.svg` | The Windows icon: the mark on a rounded cream plate (#FBF8F1) with an edge (#D9CDB6), so the ink A holds on a dark taskbar |
| `apertrust-linux.svg`, `apertrust-linux-small.svg` | The Linux icon: the mark on the cream plate standing on a lip of its edge colour |
| `apertrust-mark.svg` | The mark alone, tight bounds, transparent background, ink and #D66A20: in the app, the web and docs |
| `apertrust-mark-mono.svg` | The mark in one colour, `currentColor`, with its cut: UI, favicons, monochrome print |
| `apertrust-logo.svg` | The full logo: "Apertrust" with the Slider A as its A, ink letters, for light backgrounds: the web, docs, the README |
| `apertrust-logo-dark.svg` | The same with cream letters and the #DC7028 marker, for dark backgrounds |
| `apertrust-logo-mono.svg` | The full logo in one colour, `currentColor`, the cut as in the mono mark |

The short logo is the mark, the full one the logo.

- **The logo's letters,** "pertrust", are Fraunces 600 at the optical size 144, 75 units for the 64-unit mark, from x 64 on the mark's baseline, 58.5, a unit of letter spacing taken out; the track stops at 57. They are outlined, so no font is needed to draw them.
- **Fraunces** is © The Fraunces Project Authors, under the SIL Open Font License 1.1, taken from Google Fonts' repository (`ofl/fraunces`, `Fraunces[SOFT,WONK,opsz,wght].ttf`, commit `4024282d9b0c`) and outlined with fontTools. Only the outlines are here; the font is not shipped.
- **Where it goes:** never place the icon or the mark beside the logo, where the A would repeat; leave clear space of the marker's width, 9 of the mark's 64 units, on every side; draw the logo at least 24 px tall. Smaller, the mark then the name, side by side, takes its place.

## PNG and platform files

Every file below is rendered from the SVGs by `tools/build-icons.sh` (the app's `icons` example, resvg and nothing else), each PNG at its own size; run it after an SVG changes and commit what it writes.

- `png/macos/AppIcon.iconset/`: every size macOS expects, 16 to 1024 with the @2x names, from `apertrust-macos.svg` and, at 32 px and below, `apertrust-macos-small.svg`.
- `png/macos/AppIcon-dark.iconset/`: the same for the dark appearance. An `.icns` holds one appearance only, so the dark set waits for packaging with an asset catalog.
- `apertrust.icns`: the light iconset's PNGs, as `iconutil` would pack them but without its additions.
- `png/windows/`: 16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 px, from `apertrust-square.svg` and its `-small` drawing.
- `apertrust.ico`: the Windows icon, holding all the sizes above as PNGs.
- `png/linux/hicolor/`: the freedesktop icon theme's layout, 16 to 512 px from `apertrust-linux.svg` and its `-small` drawing, and `scalable/apps/` (a copy of `apertrust-linux.svg`), each file named after the app id, `org.apertrust.Apertrust`, as the desktop file's `Icon=` names it.
- `png/mark/`: the mark alone, 256, 512 and 1024 px high, transparent.

## In the app

- **The window's icon** is compiled into the binary (`src/main.rs`):
  - on a Mac, the macOS iconset's 512 px plate, which eframe makes the Dock icon of an unbundled run;
  - on Windows, the Windows icon at 256 px;
  - elsewhere, the Linux icon at 256 px.
- **Packages** use the platform files here instead ([docs/packaging.md](../../../../docs/packaging.md)): the `.icns` in the app bundle, the `.ico` in the Windows executable and its installer, the hicolor tree on Linux.

## Metadata

The SVGs and PNGs carry no embedded metadata, and the `.icns` and `.ico` hold only those PNGs: the script writes no chunk but the image's own. Keep it that way when replacing an SVG: strip C2PA, XMP and text.
