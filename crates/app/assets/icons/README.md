# Icons

Apertrust's app icon, "Folded A": a folded-paper A with a light amber face and a darker rust face, a cut between the faces and a small rust hexagonal aperture in its notch. The cut is a mask on the amber face, the rust face widened, so it stays transparent on any background and tells the faces apart without colour. It stays clear down to 16 px, so there is no separate small-size artwork.

## SVG sources (`svg/`)

| File | Use |
| --- | --- |
| `apertrust-macos.svg` | macOS app icon, 1024 canvas, 824 plate centred with shadow (Apple's grid); the A at 1.4 times its drawn size, 78% of the plate, close to the Mac's own icons' glyphs |
| `apertrust-macos-dark.svg` | The same on a dark plate, for macOS's dark icon appearance, its hexagon a lighter rust (#D0632A) |
| `apertrust-plate.svg`, `apertrust-plate-dark.svg` | The plates alone, full-bleed, for design tools and the web |
| `apertrust-square.svg` | The free-standing A on a square canvas: the Windows and Linux icon |
| `apertrust-mark.svg` | The A alone, tight bounds, transparent background: in the app, the web and docs |
| `apertrust-mark-mono.svg` | Single-colour mark using `currentColor`, both faces solid, the cut between them: UI, favicons, monochrome print |
| `apertrust-logo.svg` | The full logo: the A as the first letter of "Apertrust", ink letters (#1B1B1E), for light backgrounds: the web, docs, the README |
| `apertrust-logo-dark.svg` | The same with paper letters (#F4F2EF) and the lighter hexagon, for dark backgrounds |
| `apertrust-logo-mono.svg` | The full logo in one colour, `currentColor`, the cut as in the mono mark |

The short logo is the mark, the full one the logo. The logo's letters are Archivo Bold at 110% width, outlined, so no font is needed to draw them. As the logo canvas sets out: never place the icon or the mark beside the logo, where the A would repeat; leave clear space of at least half the A's height on every side; draw the logo at least 120 px wide.

## PNG and platform files

Every file below is rendered from the SVGs by `tools/build-icons.sh` (the app's `icons` example, resvg and nothing else), each PNG at its own size; run it after an SVG changes and commit what it writes.

- `png/macos/AppIcon.iconset/`: every size macOS expects, 16 to 1024 with the @2x names, from `apertrust-macos.svg`.
- `png/macos/AppIcon-dark.iconset/`: the same for the dark appearance. An `.icns` holds one appearance only, so the dark set waits for packaging with an asset catalog.
- `apertrust.icns`: the light iconset's PNGs, as `iconutil` would pack them but without its additions.
- `png/windows/`: 16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 px, from `apertrust-square.svg`.
- `apertrust.ico`: the Windows icon, holding all the sizes above as PNGs.
- `png/linux/hicolor/`: the freedesktop icon theme's layout, 16 to 512 px and `scalable/apps/` (the free-standing A), each file named after the app id, `org.apertrust.Apertrust`, as the desktop file's `Icon=` names it.
- `png/mark/`: the mark alone, 256, 512 and 1024 px high, transparent.

## In the app

- **The window's icon** is compiled into the binary (`src/main.rs`):
  - on a Mac, the macOS iconset's 512 px plate, which eframe makes the Dock icon of an unbundled run;
  - elsewhere, the free-standing A at 256 px.
- **Packages** use the platform files here instead ([docs/packaging.md](../../../../docs/packaging.md)): the `.icns` in the app bundle, the `.ico` in the Windows executable and its installer, the hicolor tree on Linux.

## Metadata

The SVGs and PNGs carry no embedded metadata, and the `.icns` and `.ico` hold only those PNGs: the script writes no chunk but the image's own. Keep it that way when replacing an SVG: strip C2PA, XMP and text.
