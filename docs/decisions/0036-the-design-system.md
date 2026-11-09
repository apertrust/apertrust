# 0036. The design system

## Context

How is the interface built so that its parts are reused, it can be themed and translated, it is accessible and it can be tested without looking at it? egui ([0035](0035-egui-for-the-desktop-and-the-browser.md)) draws whatever it is told, so consistency has to come from a layer above it.

## Decision

**A design-system crate, `apertrust-ui`, under the app** ([gui.md](../gui.md)). It holds tokens and themes, fonts, icons, every component the interface uses and a gallery. It knows nothing of photos, modules or the engine: a slider reports that it moved, and the app turns that into a session command. It depends on `apertrust-base` only for `settings::Choice` ([0043](0043-settings-and-apertrusts-folders.md)). Module panels are generated from module definitions, with custom widgets from a closed registry.

**Themes** are token sets named by role: colours, the photo's neutral surround, type sizes, density, radii and strokes, overlay colours and motion. They are TOML files in `crates/ui/themes`: dark (the default, a neutral dark grey), light and high contrast built in, and user theme files that extend one. Every theme is checked for WCAG AA contrast when it loads and can be switched live. The interface's scale is a setting (egui's zoom factor). The layout, the components' shapes, the typeface and colours that are data are not themable.

**Type:** IBM Plex Sans in Regular, Medium and SemiBold for the interface, egui's monospace family included. Its digits, `+` and `−` share one width (600 units), so a changing value keeps its place without tabular figures, which egui's text renderer cannot apply. IBM Plex Mono Regular, in a family of its own, sets the log window's lines alone, in columns as the terminal writes them. egui's default fonts are off. The fonts ship unmodified with their licence ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)).

**Icons** are one-colour SVG files, rendered by resvg (pure Rust) at their pixel size and tinted with a theme colour, with no gradients or opacity; `Picture` draws an SVG in its own colours, for the logo.

**Translation** lives in an egui-free `i18n` module: Fluent messages in `crates/ui/locales`, so translators control plurals and selectors, and ICU4X for numbers and dates in each language's CLDR format. British English, the source, with Ukrainian and Polish from the start, American English as an overlay of the messages it spells differently, and a pseudo-locale that catches clipped and untranslated text. Every user-visible string is a message id; `tools/check-ui-strings.sh` refuses English literals in CI. The engine sends error codes and message ids, never words. Plex covers Latin, Cyrillic and Greek; CJK comes with fallback fonts loaded on demand; right-to-left languages once egui lays them out. The module becomes a crate, `apertrust-i18n`, only when the CLI or the server needs translated text.

**Also:**

- one actions registry for buttons, menus, tooltips and rebindable shortcuts ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md));
- AccessKit on, each component reporting its role, full name and value;
- one versioned settings file for the interface (theme, language, scale, keyboard map, workspace), never edits ([0043](0043-settings-and-apertrusts-folders.md));
- tests with `egui_kittest` through AccessKit, and gallery snapshots in every theme and the pseudo-locale on CI's software GPU, allowed 2/255 a channel and 2 pixels between GPUs.

## Options considered

- Components in the app crate: nothing would keep them free of engine types.
- A translation crate before anything outside the interface translates: a crate with one user protects no boundary.
- egui's `Visuals` alone, or colours in code: no roles, no user themes, no contrast check.
- gettext (a binding to C's libintl, or a reader without Fluent's selectors), or key-value files without plurals.
- Shortcuts per widget: no single place to rebind or show them.
- AccessKit off, and checking by eye.
- IBM Plex Mono for values: Plex Sans's equal-width digits make it unnecessary, so Mono is the log window's alone.
- egui's default fonts kept by a licence exception: a typeface the design does not use.

## Consequences

- `crates/ui` holds the components and the gallery (`cargo run -p apertrust-ui --example gallery`); snapshots are reviewed like golden images.
- The app crate wires components to the session; it adds no component of its own.
- Every new string needs its message in all three languages.
