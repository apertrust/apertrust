# 0068. Packages built by cargo-packager

## Context

Apertrust has to be installable on macOS, Windows and Linux, as two programs, the app and the CLI ([0006](0006-two-programs-on-three-desktop-platforms.md)): a signed app bundle, an installer and a Linux package in the end, each with its icon, version, file types and licences. Each platform has its own formats and tools. How are the packages built, with what installer on Windows, and from how many descriptions of the app?

## Decision

**One tool, cargo-packager, from one file, `packaging/Packager.toml`.** cargo-packager is in Rust, under MIT and Apache-2.0, and runs only on the machine that packages: nothing of it is compiled into Apertrust or shipped with it, so decision 5 does not reach it. `tools/package.sh` builds the release binaries for its own platform (on macOS each architecture apart, one disk image for Apple silicon and one for Intel, so each download holds only its own code), fills the config's placeholders (the repository's path, the version, the binaries' folder) and runs it. Each platform's package is built on that platform.

**The packages:**

- **macOS**: `Apertrust.app` and a disk image, the app and the CLI in `Contents/MacOS`, macOS 11 at least, signed ad hoc with the hardened runtime until a Developer ID and notarisation come.
- **Windows**: an NSIS installer for the person installing, needing no administrator, and a portable zip.
- **Linux**: an AppImage and a tarball laid out as a prefix. Flatpak follows, through Flathub.

Every package ships the GPL, `CREDITS.md`, the fonts' OFL and Lucide's licence, and nothing else: data is compiled in ([0045](0045-a-top-level-data-folder.md)), and the platform's libraries stay the system's ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).

**NSIS, not WiX, on Windows:** a per-user install without administrator's rights, file associations in cargo-packager's script, and an installer that builds from Linux and macOS too. An MSI is for managed deployments, which can come later; MSIX, for the Microsoft Store, is planned beside the installer.

**The metadata lives beside the config and is tested.** `packaging/macos/Info.plist` (merged over the keys cargo-packager writes), the Linux desktop file, AppStream metadata and MIME types, named after the app id `org.apertrust.Apertrust`, and the Windows executable's icons and version resource, compiled by the app's build script through winresource and the SDK's `rc.exe`. Every platform's file types are held to the extensions rawler decodes by a test, and the edit's type to its registrations below. The icons are rendered from the mark's SVGs by a Rust example, with no system tool, and committed beside them.

**`.apxmp` is the one type Apertrust owns.** Raws are offered in Open With and never taken from the app that opens them; the edit beside a photo is Apertrust's own format, and opening one opens its photo.

- **macOS:** the bundle exports `org.apertrust.edit`, conforming to `public.xml` and `public.data`, its icon the system's document page with the text APXMP (`UTTypeIconText`, no icon file), and Apertrust is its editor at the rank Owner.
- **Windows:** the installer makes the ProgID `Apertrust.Edit` `.apxmp`'s default, its icon the executable's icon resource 2 (`apertrust.exe,-2`); the uninstaller clears that default only while it is still `Apertrust.Edit`.
- **Apertrust Dev** opens edits but never takes them: the rank Alternate on macOS, and `Apertrust.Dev.Edit` in Open With alone on Windows.

**The version is the build's** ([0044](0044-logging-the-banner-and-versions.md)): the package's is the release's three numbers with the count of commits, since installers take numbers only, and the full version stays in the binaries' banner and the Windows version resource's text.

## Options considered

- **A script per platform** (`hdiutil` and `codesign` on macOS, a WiX or Inno Setup project on Windows, `appimagetool` on Linux), as LightCraft packages its builds: full control, but three descriptions of one app that drift, and every format's details written by hand.
- **cargo-dist:** made for command-line tools' archives and shell installers, with release automation for GitHub; no app bundle, disk image or AppImage of its own.
- **cargo-bundle:** the tool cargo-packager descends from, little maintained, with no NSIS or AppImage.
- **Tauri's bundler:** the same lineage as cargo-packager, but tied to Tauri's configuration and CLI.
- **WiX:** MSI packages, needing Windows and the WiX toolset to build, and administrator's rights for the usual per-machine install.
- **Inno Setup:** a good installer, but not one cargo-packager builds, so a second description of the app.
- **MSIX now:** the Store's format, which needs a signing certificate to install at all; later.
- **One universal macOS image** (both architectures joined by `lipo`): one download for every Mac, but twice the size for each.

## Consequences

- `cargo install --locked cargo-packager --version 0.11.8` on a machine that packages; the script checks the version, and a new one is adopted deliberately, with the config checked against it.
- cargo-packager downloads its helpers the first time on each platform (the create-dmg script, NSIS, linuxdeploy), from pinned addresses; those builds need the network once.
- cargo-packager 0.11.8's quirks are worked around in the config and the script: `--binaries-dir` is ignored, so the folder is in the config; a config file with no `name` makes it look for a `Cargo.toml` beside it.
- The installer's script is Apertrust's own, `packaging/windows/installer.nsi`: cargo-packager 0.11.8's template with the per-user folder `%LOCALAPPDATA%\Programs\Apertrust`, the raw types registered under Open With alone (cargo-packager's makes Apertrust their default handler), `.apxmp`'s default `Apertrust.Edit`, and an uninstaller that removes them (cargo-packager's leaves them behind); a new cargo-packager's template is compared with it.
- The Windows app is a windowed program, so no console opens beside it; started from a terminal it attaches to that terminal's console, so `--version`, `-d` logs and errors print there ([packaging.md](../packaging.md), Windows). The CLI stays a console program.
- The nightly builds package Apertrust Dev, which installs beside Apertrust, each package on its own platform (`tools/package.sh --dev`, `nightly-builds.yml`).
- Planned: a release workflow that builds every package on its own platform; a Developer ID and notarisation on macOS, a code-signing certificate on Windows, Flatpak on Linux ([packaging.md](../packaging.md)).
