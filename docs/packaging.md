# Packaging

How Apertrust becomes a package a person installs on macOS, Windows and Linux: what each package holds, how to build one, and what is still missing (decision 68).

One tool builds every package, [cargo-packager](https://github.com/crabnebula-dev/cargo-packager), from one file, `packaging/Packager.toml`. `tools/package.sh` builds the release binaries for the computer it runs on, fills in the version and runs it. cargo-packager is a build tool, installed on the machine that packages; nothing of it is compiled into Apertrust or shipped with it.

## The packages

| Platform | Package | What it holds |
| --- | --- | --- |
| macOS | `Apertrust.app` | `Contents/MacOS/apertrust` (the app) and `apertrust-cli`; `Contents/Resources/` with the icon, `LICENSE`, `CREDITS.md` and `licences/`; the bundle's `Info.plist` |
| macOS | `Apertrust_<version>_aarch64.dmg` for Apple silicon and `Apertrust_<version>_x64.dmg` for Intel | The app and a link to Applications, with the icon as the volume's, in a window of Apertrust's own with a background (below), signed ad hoc; one image per architecture, so each download holds only its own code |
| Windows | `apertrust_<version>_x64-setup.exe` | An NSIS installer for the person installing, with no administrator's rights, into `%LOCALAPPDATA%\Programs\Apertrust`: both executables and the licences, a Start menu shortcut, the raw file types in Open With and an uninstaller (below, Windows) |
| Windows | `apertrust_<version>_x64-portable.zip` | An `Apertrust` folder with both executables and the licences, to run from anywhere |
| Linux | `apertrust_<version>_x86_64.AppImage` | The app and the CLI in `usr/bin`, the desktop file, the AppStream metadata, the MIME types, the icons and the licences (in `usr/lib/apertrust`) |
| Linux | `apertrust_<version>_x86_64.tar.gz` | The same laid out as a prefix (`bin/`, `share/applications`, `share/metainfo`, `share/mime/packages`, `share/icons/hicolor`, `share/doc/apertrust`), to copy into `~/.local` or `/usr/local` |

**The licences** in every package: `LICENSE` (the GPL), `CREDITS.md` (every component with its licence, and the fonts' and icons' notices, which About also shows), `licences/OFL.txt` (the fonts' licence: the binary compiles IBM Plex Sans in, decision 64) and `licences/LICENSE-Lucide` (the interface's icons).

**Nothing else is bundled.** The app's data, the settings schema among it, is compiled into the binaries (decision 45), so no package has a data folder. On Linux the platform's libraries stay the system's (decision 5): the Vulkan loader and drivers, libwayland, libxkbcommon, the X11 libraries and libdbus are loaded at run time and left out of the AppImage.

**The version** is the build's (decision 44), as `--version` prints it, `0.2.0+12~gabc1234`. A package's own version is the release's three numbers and the count of commits, `0.2.0+615`, since installers take numbers only: the bundle's `CFBundleShortVersionString` is `0.2.0` and its `CFBundleVersion` `615`, the Windows executable's version resource `0.2.0.615` with the full version as its text, and the files are named with `0.2.0+615`. With `APERTRUST_VERSION` set, the version is the packager's and the package's is its leading `X.Y.Z`, which it must have.

## The metadata

In `packaging/`, beside the configuration:

- `macos/Info.plist`: the bundle's keys, merged over those cargo-packager writes: the name, the version, macOS 11 at least (the oldest that runs on Apple silicon, for both architectures), the photography category, the interface's languages (en-GB, uk, pl), and the raw document types, Apertrust an editor of them at the Alternate rank, so it is offered in Open With and never takes over the raws another app opens. Each type is the system's where macOS declares one; seven formats macOS does not name (ARI, CRM, DCS, KDC, MEF, QTK, X3F) are imported as `org.apertrust.raw-image.<extension>`. The binary embeds `crates/app/assets/macos/Info.plist` for a run without a bundle; its keys agree with these.
- `linux/org.apertrust.Apertrust.desktop`: the desktop entry, with the MIME types of every raw format, its `Exec` to pass `--in-folder %F` (planned, [Opening a raw from the system](#opening-a-raw-from-the-system)); `Icon=` and the window's Wayland app id are the app's id, so the compositor finds the window's icon.
- `linux/org.apertrust.Apertrust.metainfo.xml`: the AppStream metadata: the name, the summary, the GPL-3.0-or-later licence, the homepage and the repository.
- `linux/org.apertrust.Apertrust.mime.xml`: the MIME types for the five formats shared-mime-info does not name (ARI, CRM, DCS, ORI, QTK), installed as `share/mime/packages/org.apertrust.Apertrust.xml`.
- **Windows**: the executable's icon and version resource are compiled by the app's `build.rs` through winresource and the SDK's `rc.exe`. `windows/installer.nsi` is the installer's script, cargo-packager's own template at 0.11.8 with Apertrust's changes (below, Windows), which registers every raw extension in `Packager.toml`'s list.

**The file types are tested.** `crates/app/tests/packaging.rs` holds the bundle's document types, the desktop file's MIME types, the MIME package and the installer's extensions, in the config and as its template registers and removes them, to `apertrust_raw::extensions()`, the formats rawler decodes, so a rawler update that adds one fails until the packages open it too. It also holds the app's id to be the same in every file.

**The disk image's window**, what Finder shows when the image is mounted, is Apertrust's in `packaging/macos/dmg`: a 660 × 400 point window on the dark theme's ground, the logo at the top, the app's icon and the Applications link side by side with a thin arrow between them, and "Drag Apertrust to Applications" under them. Finder draws the icons' labels in black over a background picture, in dark mode too, so each sits on a light plate. Its files:

- `background.svg`: the source, in the dark theme's tokens, with the layout in its comment; the logo is linked and the caption outlined, so it renders with no fonts. To change the words, set them as text, outline them with usvg and replace the path.
- `background.tiff`: the background at 1× (660 × 400, 72 dpi) and 2× (1320 × 800, 144 dpi) in one file, as Finder takes a background for Retina screens; written by `tools/build-dmg-background.sh`, which renders the SVG with resvg through the app's `dmg_background` example and joins the two with macOS's `tiffutil`. Run it on a Mac after the SVG or the logo changes and commit the TIFF.
- `layout.DS_Store`: Finder's view of the window: its size, icon view with no toolbar, icons of 128 points with labels of 13, the app at (170, 196) and Applications at (490, 196), and the background. `layout-dev.DS_Store` is the same for Apertrust Dev's image (below), its volume and its bundle named `Apertrust Dev`. `generate.py` writes both from nothing, holding no path, user or disk of the machine that ran it; run it by hand when the layout changes, with `pip install ds_store==1.3.3 mac_alias==2.2.3` (both MIT) in a virtual environment, and commit the files, so packaging never needs Python.

`tools/package.sh` adds both to each image after cargo-packager makes it, with `hdiutil` alone: the image made writable, attached with no Finder window, given `.background/background.tiff` and `.DS_Store`, compressed again and signed again. The layout finds its background by the volume's name, so the volume is the product's, `Apertrust` or `Apertrust Dev`, with no version, as cargo-packager names it; the script stops on any other.

**The icons** are rendered from the mark's SVGs by `tools/build-icons.sh` into `crates/app/assets/icons`, beside their sources and the window's icon, which the binary compiles from the same tree: the `.icns`, the `.ico` (16 to 256 px) and the hicolor tree, named after the app id ([its README](../crates/app/assets/icons/README.md)).

## Windows

- **A windowed program.** `apertrust.exe` is built for Windows' window subsystem, in a release build and a debug one alike (`#![windows_subsystem = "windows"]`), so no console opens beside it from Explorer or the Start menu. `apertrust-cli.exe` stays a console program.
- **Its terminal.** Started from one, the app attaches to its console first thing (`AttachConsole(ATTACH_PARENT_PROCESS)`, in `apertrust-display`), before the log is set up, and points stdout and stderr at it where they have no handle; one redirected to a file or a pipe is kept. So `--version`, `-d`'s lines and its errors print there; started from Explorer, nothing is printed. cmd and PowerShell do not wait for a windowed program, so its lines come after their prompt; `start /wait apertrust --version` in cmd, or a pipe (`apertrust --version | more`), waits.
- **The install folder** is `%LOCALAPPDATA%\Programs\Apertrust`, Windows' place for a person's programs; `%LOCALAPPDATA%\Apertrust` is left to the cache folder (`docs/settings.md`).
- **Open With only.** For each raw extension, in `HKCU\Software\Classes`, the installer writes the ProgID `Apertrust.RawImage` (its name, icon and `shell\open\command`), the ProgID's name under `.<ext>\OpenWithProgids`, and the app's own entry, `Applications\apertrust.exe`, with its name, icon, command and the extensions under `SupportedTypes`. It never writes an extension's default value, so Apertrust is offered in Open With and becomes a type's default only when the person chooses it.
- **The uninstaller** removes exactly those: the ProgID's name from each `OpenWithProgids` (the key, and the extension's, only where nothing else is left in it), the ProgID, the `Applications` entry, the Start menu entry (in the folder chosen at install, which the installer's key keeps), the desktop shortcut, the files it installed with the install folder once empty, the uninstall entry and the installer's own key. The person's settings and edits, elsewhere, stay. A default the person chose for a type is the system's to forget.

## Apertrust Dev

The nightly builds are Apertrust Dev, a development build that installs and runs beside Apertrust: its own name, id, executables, package files and user data folder ([settings.md](settings.md), Development builds), so neither replaces the other.

**Which builds are Dev:** the nightly packages are always Dev; the nightly job says so explicitly. Every other build, from source or `cargo run`, stays Apertrust while no release tag exists, and becomes Dev once one does: a build not on a release tag, when the history has one.

`tools/package.sh --dev` packages it. The binaries are built with `APERTRUST_DEV=1`, which makes them name themselves Apertrust Dev and keep their settings in its folder, and are copied as `apertrust-dev` and `apertrust-dev-cli`: the Windows installer registers Open With and finds a running app by the executable's name, and the tarball's `bin/` shares a prefix with Apertrust's. The packaging files are given the identity `packaging/dev-identity.txt` lists, each line a text and what replaces it; without `--dev` they are used as they are.

| Platform | Apertrust Dev's packages |
| --- | --- |
| macOS | `Apertrust-dev_<version>_aarch64.dmg` and `Apertrust-dev_<version>_x64.dmg`, the volume `Apertrust Dev`, laid out by `layout-dev.DS_Store`, holding `Apertrust Dev.app`: the bundle id `org.apertrust.Apertrust.Dev`, its name `Apertrust Dev`, the executables `apertrust-dev` and `apertrust-dev-cli` |
| Windows | `apertrust-dev_<version>_x64-setup.exe`, installing into `%LOCALAPPDATA%\Programs\Apertrust Dev` with its own uninstall entry, Start menu shortcut `Apertrust Dev`, ProgID `Apertrust.Dev.RawImage` and Open With entry `Applications\apertrust-dev.exe`; `apertrust-dev_<version>_x64-portable.zip`, an `Apertrust Dev` folder |
| Linux | `apertrust-dev_<version>_x86_64.AppImage` and `apertrust-dev_<version>_x86_64.tar.gz`, with `apertrust-dev` and `apertrust-dev-cli`, the desktop entry `org.apertrust.Apertrust.Dev.desktop` (named Apertrust Dev) and the metadata `org.apertrust.Apertrust.Dev.metainfo.xml`; the icons and the MIME types are Apertrust's, the same files in both |

`crates/app/tests/packaging.rs` applies `dev-identity.txt` to the files as the script does, and checks that each line still matches and that every package names Apertrust Dev by its own id.

## Opening a raw from the system

Planned (decision 46). A raw opened with Apertrust from the system, by Open With or a double-click, opens its folder with that photo active, where `apertrust FILE` from a terminal opens the photo alone ([library.md](library.md#folder-mode)). The app tells the two apart as each platform delivers them:

- **macOS** sends Open With and a double-click as an open-documents event, not arguments, so the bundle's document types stay as they are.
- **Windows:** the ProgID's and the `Applications` entry's `shell\open\command` is to pass `--in-folder "%1"` before the file.
- **Linux:** the desktop entry's `Exec` is to pass `--in-folder %F`, every file chosen.

The registrations change with the app's side of it, the library's step 9b ([roadmap.md](roadmap.md#the-librarys-steps)); `tests/packaging.rs` is to hold each to the flag.

## Building one

[packaging/README.md](../packaging/README.md) is the short version: what each platform needs, the command, and how to try each package.

```sh
cargo install --locked cargo-packager --version 0.11.8   # once; the script checks the version
tools/package.sh                 # this platform's packages into target/package/<target>/out
tools/package.sh --dev           # Apertrust Dev's, as the nightly builds are
APERTRUST_VERSION=0.2.0 tools/package.sh                  # a packager's version
```

- **macOS**: Xcode's command-line tools (`lipo`, `codesign`, `hdiutil`). The script packages each Apple target installed, a disk image for Apple silicon and one for Intel (`rustup target add aarch64-apple-darwin x86_64-apple-darwin` installs both); `--target aarch64-apple-darwin` or `--target x86_64-apple-darwin` packages one. The disk image is made without driving Finder, its window laid out by the committed `.DS_Store` (above); `--finder-layout` also has create-dmg arrange it in Finder first, which needs a logged-in session and may ask to let the terminal control Finder, and the committed layout then replaces its own.
- **Windows**: the MSVC toolchain with the Windows SDK, whose `rc.exe` compiles the resources, from Git Bash; the portable zip is written by PowerShell's `Compress-Archive`.
- **Linux**: an x86_64 or aarch64 system; the AppImage runs linuxdeploy without FUSE.
- **What cargo-packager downloads** into its tools folder the first time: the create-dmg script, pinned by its commit, on macOS; NSIS and its plugins, checked by SHA-1, on Windows; linuxdeploy, its AppImage plugin and AppRun on Linux.

## The CLI

Every package holds `apertrust-cli` beside the app:

- **macOS**: `Apertrust.app/Contents/MacOS/apertrust-cli`. To put it on the `PATH`: `ln -s /Applications/Apertrust.app/Contents/MacOS/apertrust-cli /usr/local/bin/apertrust-cli` (or a folder of one's own on the `PATH`).
- **Windows**: `apertrust-cli.exe` in the install folder, `%LOCALAPPDATA%\Programs\Apertrust`, or in the zip's folder; add the folder to the user's `Path` to run it anywhere.
- **Linux**: the tarball's `bin/apertrust-cli`. The AppImage starts the app; its CLI is in `usr/bin` once extracted with `--appimage-extract`.

## Unsigned packages

The packages are not signed with a developer's certificate yet, so each system warns the first time:

- **macOS**: the app and the disk image are signed ad hoc (`codesign --sign -`), with the hardened runtime, and not notarised, so Gatekeeper refuses a downloaded copy as from an unidentified developer. To open it once: Control-click the app in Applications and choose Open (macOS 14 and earlier), or open System Settings › Privacy & Security and choose Open Anyway (macOS 15); or remove the download's quarantine with `xattr -dr com.apple.quarantine /Applications/Apertrust.app`.
- **Windows**: SmartScreen says "Windows protected your PC" for the installer and the executables: More info › Run anyway.
- **Linux**: neither the AppImage nor the tarball is signed; make the AppImage executable (`chmod +x`) and run it.

Apertrust Dev's are the same, under its names: `/Applications/Apertrust Dev.app`, `apertrust-dev_*_x86_64.AppImage`.

## Nightly builds

`.github/workflows/nightly-builds.yml` builds Apertrust Dev's packages each night at 01:00 UTC, each on its own platform, and publishes them as the `nightly` pre-release on GitHub, [releases/tag/nightly](https://github.com/apertrust/apertrust/releases/tag/nightly).

- **When**: only when main has changed since the commit the `nightly` tag points at. A run by hand from the Actions tab builds the same way, and its `force` input builds and publishes an unchanged main too. One run at a time; a second waits.
- **The jobs**: `check` compares main with the tag; then three builds, each `tools/package.sh --dev` with cargo-packager 0.11.8, from a full clone so the version is computed: `macos` on `macos-15` (both chips' disk images, Intel's cross-built with `--target x86_64-apple-darwin`), `windows` on `windows-2025` (the installer and the zip), `linux` on `ubuntu-24.04` (the AppImage and the tarball). Each uploads its packages as an artifact, kept seven days.
- **Publishing**: once every build has succeeded, `publish`, the one job that may write, makes the `nightly` release again: it deletes the release with its tag, then creates it through GitHub's Releases API on the commit built, with the night's files and `SHA256SUMS.txt`, their SHA-256 sums. The API makes the tag, since GitHub refuses the job's own token a `git push` of a tag whose commit changes workflow files. The release is missing for the minute between. The release's notes give the commit and the date, and say that nightlies are untested development builds and unsigned (above). It is a pre-release, never the latest release. A file's version `+` is published as `.` (`apertrust-dev_0.0.0.615_x64-setup.exe`), as GitHub would otherwise rename it after the sums were taken.
- **On failure**: when any build fails, nothing is published and the last nightly stays. A run from another branch builds but never publishes.
- **What is not there**: tests, which CI runs on every push to main, and signing (above).

## Signing and notarisation, planned

- **macOS**: a Developer ID Application certificate (the Apple Developer Program) for `macos.signingIdentity`, and notarisation through `notarytool`, which cargo-packager runs once the certificate and an App Store Connect API key are given as `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_API_ISSUER`, `APPLE_API_KEY` and `APPLE_API_KEY_PATH`; the ticket is then stapled to the disk image.
- **Windows**: a code-signing certificate (an organisation's, or a cloud signing service's) for the installer and both executables, through `windows.certificateThumbprint` or `windows.signCommand`. An MSIX package for the Microsoft Store is planned beside the installer.
- **Linux**: Flatpak, through Flathub, with its runtime giving the platform's libraries; the desktop file, the metadata and the icons here are already named after the app id it needs.
- **CI**: a release workflow for version tags, building every package on its own platform as the nightly builds do, comes next.

## Known gaps

- **Windows is untested**: the installer, the Open With entries, the uninstaller and the terminal attach wait for the first build on Windows.
- **The dark icon**: an `.icns` holds one appearance; macOS's dark and tinted icons need an asset catalog.
- **Older Linux systems**: the desktop file relies on shared-mime-info for 24 of the formats, and an older release may lack some of the newer ones (CR3, NRW, IIQ among them); such files then do not offer Apertrust until it is updated.
