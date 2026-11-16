# 0068. Packages built by cargo-packager

## Context

Apertrust has to be installable on macOS, Windows and Linux, as two programs, the app and the CLI ([0006](0006-two-programs-on-three-desktop-platforms.md)): a signed app bundle, an installer and a Linux package in the end, each with its icon, version, file types and licences. Each platform has its own formats and tools. How are the packages built, with what installer on Windows, and from how many descriptions of the app?

## Decision

**One tool, cargo-packager, from one file, `packaging/Packager.toml`,** and for the one format it does not make, the `.rpm`, cargo-generate-rpm from `packaging/linux/rpm.toml`. Both are in Rust, cargo-packager under MIT and Apache-2.0 and cargo-generate-rpm under MIT, and run only on the machine that packages: nothing of them is compiled into Apertrust or shipped with it, so decision 5 does not reach them. `tools/package.sh` builds the release binaries for its own platform (on macOS each architecture apart, one disk image for Apple silicon and one for Intel, so each download holds only its own code), fills the config's placeholders (the repository's path, the version, the binaries' folder) and runs it. Each platform's package is built on that platform.

**The packages:**

- **macOS**: `Apertrust.app` and a disk image, the app and the CLI in `Contents/MacOS`, macOS 11 at least, signed ad hoc with the hardened runtime until a Developer ID and notarisation come.
- **Windows**: an NSIS installer for the person installing, needing no administrator, and a portable zip.
- **Linux**: an AppImage, a `.deb`, an `.rpm` and a tarball laid out as a prefix, the `.deb` and the `.rpm` installing into `/usr` from the tarball's tree and depending on the libraries the app loads at run time, by Debian's package names and by RPM's library capabilities, so one `.rpm` serves Fedora, RHEL and openSUSE. Flatpak follows, through Flathub.

**The Linux packages are built on Ubuntu 22.04** (glibc 2.35), in a container on the runner, not on the runner's newer system: a binary needs the glibc it was linked against or newer, so the oldest base of the systems they are for gives one package of each that runs on Ubuntu 22.04 and later, Debian 12 and later, Fedora, openSUSE and RHEL 10. RHEL 9, with glibc 2.34, is left out.

**The file names:** `<name>-<version>-<os>-<arch>[-<kind>].<ext>`, all lower case but `.AppImage`, `-` between every part, each platform's own words for its architecture: `apertrust-<version>-macos-arm64.dmg` and `-macos-x86_64.dmg`, `-windows-x64-setup.exe` and `-windows-arm64-setup.exe`, `-windows-x64-portable.zip` and `-windows-arm64-portable.zip`, `-linux-x86_64.AppImage` and `-linux-aarch64.AppImage` (with its `.zsync` for Apertrust Nightly), `-linux-amd64.deb` and `-linux-arm64.deb` in Debian's words, `-linux-x86_64.rpm` and `-linux-aarch64.rpm` in RPM's, `-linux-x86_64.tar.gz` and `-linux-aarch64.tar.gz`; Apertrust Nightly's the same with the name `apertrust-nightly`. cargo-packager has no template for its files' names, so the script renames what it writes.

Every package ships the GPL, `CREDITS.md`, the fonts' OFL and Lucide's licence, and nothing else: data is compiled in ([0045](0045-a-top-level-data-folder.md)), and the platform's libraries stay the system's ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).

**NSIS, not WiX, on Windows:** a per-user install without administrator's rights, file associations in cargo-packager's script, and an installer that builds from Linux and macOS too. An MSI is for managed deployments, which can come later; MSIX, for the Microsoft Store, is planned beside the installer.

**The metadata lives beside the config and is tested.** `packaging/macos/Info.plist` (merged over the keys cargo-packager writes), the Linux desktop file, AppStream metadata and MIME types, named after the app id `org.apertrust.Apertrust`, and the Windows executable's icons and version resource, compiled by the app's build script through winresource and the SDK's `rc.exe`. Every platform's file types are held to the extensions rawler decodes by a test, and the edit's type to its registrations below. The icons are rendered from the mark's SVGs by a Rust example, with no system tool, and committed beside them.

**`.apxmp` is the one type Apertrust owns.** Raws are offered in Open With and never taken from the app that opens them; the edit beside a photo is Apertrust's own format, and opening one opens its photo.

- **macOS:** the bundle exports `org.apertrust.edit`, conforming to `public.xml` and `public.data`, its icon the system's document page with the text APXMP (`UTTypeIconText`, no icon file), and Apertrust is its editor at the rank Owner.
- **Windows:** the installer makes the ProgID `Apertrust.Edit` `.apxmp`'s default, its icon the executable's icon resource 2 (`apertrust.exe,-2`); the uninstaller clears that default only while it is still `Apertrust.Edit`.
- **Apertrust Nightly** opens edits but never takes them: the rank Alternate on macOS, and `Apertrust.Nightly.Edit` in Open With alone on Windows.

**One application for every user data folder** ([0089](0089-one-instance-per-user-data-folder-a-window-per-place.md)): the app id, icon, Dock and taskbar grouping and file associations are the same whatever `--user-data-dir` says; a separate application is Apertrust Nightly's model alone, with its own identity. The Windows installer's Open With verb has `MultiSelectModel=Player` (planned), so Explorer offers it for any number of files, each start handing its file to the running instance.

**The version is the build's** ([0044](0044-logging-the-banner-and-versions.md)): the package's is the release's three numbers with the count of commits, since installers take numbers only, and the full version stays in the binaries' banner and the Windows version resource's text. `0.2.0+615` is a valid version for Debian and for RPM as it is, newer than `0.2.0` to both.

## Options considered

- **A script per platform** (`hdiutil` and `codesign` on macOS, a WiX or Inno Setup project on Windows, `appimagetool` on Linux), as LightCraft packages its builds: full control, but three descriptions of one app that drift, and every format's details written by hand.
- **cargo-dist:** made for command-line tools' archives and shell installers, with release automation for GitHub; no app bundle, disk image or AppImage of its own.
- **cargo-bundle:** the tool cargo-packager descends from, little maintained, with no NSIS or AppImage.
- **Tauri's bundler:** the same lineage as cargo-packager, but tied to Tauri's configuration and CLI.
- **WiX:** MSI packages, needing Windows and the WiX toolset to build, and administrator's rights for the usual per-machine install.
- **Inno Setup:** a good installer, but not one cargo-packager builds, so a second description of the app.
- **MSIX now:** the Store's format, which needs a signing certificate to install at all; later.
- **One universal macOS image** (both architectures joined by `lipo`): one download for every Mac, but twice the size for each.
- **cargo-deb** for the `.deb`: a fuller Debian package (`Recommends`, maintainer scripts), but a second description of the app beside cargo-packager's, which makes one already.
- **rpmbuild with a spec file** for the `.rpm`: rpm's own tool, but a C program needing rpm's build tools on the machine that packages, and a third description of the app.
- **The Linux packages built on the runner's system** (Ubuntu 24.04, glibc 2.39): no container, but packages that need glibc 2.39, leaving out Ubuntu 22.04, Debian 12 and openSUSE Leap 15.6.

## Consequences

- `cargo install --locked cargo-packager --version 0.11.8` on a machine that packages, and on Linux `cargo install --locked cargo-generate-rpm --version 0.21.1`; the script checks both versions, and a new one is adopted deliberately, with the config checked against it.
- cargo-packager downloads its helpers the first time on each platform (the create-dmg script, NSIS, linuxdeploy), from pinned addresses; those builds need the network once.
- cargo-packager 0.11.8's quirks are worked around in the config and the script: `--binaries-dir` is ignored, so the folder is in the config; a config file with no `name` makes it look for a `Cargo.toml` beside it; its packages' file names are its own, so the script gives each its name after packaging; its desktop file would be named after the binary, not the app id, so it is off and each Linux package takes ours. Its `.deb` has no `Recommends` (libvulkan1's own brings a Vulkan driver), and holds the licences again in `/usr/lib/apertrust` and the icons again under the binary's name, as its AppImage does.
- cargo-generate-rpm reads a Cargo manifest, so the script writes the filled-in `rpm.toml` as the `Cargo.toml` of a folder of its own; it cannot own a folder, so removing the `.rpm` leaves `/usr/share/doc/apertrust` empty.
- No package of Apertrust Nightly has a file in common with Apertrust's, so the `.deb`s and `.rpm`s install side by side: its icons and MIME types on Linux are named after its own id.
- The Linux packages' build runs in a container pinned by digest, which installs the tools the runner's image has: git, the compiler and linker, rustup, file and zsync ([packaging.md](../packaging.md), Nightly builds).
- The installer's script is Apertrust's own, `packaging/windows/installer.nsi`: cargo-packager 0.11.8's template with the per-user folder `%LOCALAPPDATA%\Programs\Apertrust`, the raw types registered under Open With alone (cargo-packager's makes Apertrust their default handler), `.apxmp`'s default `Apertrust.Edit`, the open verb's `MultiSelectModel` (planned, [0089](0089-one-instance-per-user-data-folder-a-window-per-place.md)), and an uninstaller that removes them (cargo-packager's leaves them behind); a new cargo-packager's template is compared with it.
- The Windows app is a windowed program, so no console opens beside it; started from a terminal it attaches to that terminal's console, so `--version`, `-d` logs and errors print there ([packaging.md](../packaging.md), Windows). The CLI stays a console program.
- The nightly builds package Apertrust Nightly, which installs beside Apertrust, each package on its own platform (`tools/package.sh --nightly`, `nightly-builds.yml`).
- Planned: a release workflow that builds every package on its own platform; a Developer ID and notarisation on macOS, a code-signing certificate on Windows, Flatpak on Linux ([packaging.md](../packaging.md)).
