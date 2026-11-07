# Packaging

Everything needed to build Apertrust's packages. `tools/package.sh` builds the release binaries for the computer it runs on and packages them with cargo-packager from `Packager.toml`. A package is built on its own platform. [docs/packaging.md](../docs/packaging.md) says what each package holds and why.

## Once, on any platform

```sh
cargo install --locked cargo-packager --version 0.11.8   # the script checks the version
```

The first run downloads cargo-packager's own tools: create-dmg on macOS, NSIS on Windows, linuxdeploy on Linux.

## Build

```sh
tools/package.sh                              # this platform's packages
APERTRUST_VERSION=0.2.0 tools/package.sh      # with a version of your own
```

The packages land in `target/package/<target>/out`. The version comes from the newest `release-X.Y.Z` tag, as `0.0.0+N~gHASH` before the first.

| Platform | Needs | Packages |
| --- | --- | --- |
| macOS | Xcode's command-line tools; `rustup target add aarch64-apple-darwin x86_64-apple-darwin` for both chips | `Apertrust.app` and a disk image for each chip |
| Windows | The MSVC toolchain with the Windows SDK; run the script from Git Bash | An NSIS installer and a portable zip |
| Linux | An x86_64 or aarch64 system | An AppImage and a tarball laid out as a prefix |

- **macOS:** `--target aarch64-apple-darwin` or `--target x86_64-apple-darwin` packages one chip. `--finder-layout` also has create-dmg arrange the disk image's window in Finder first; it needs a logged-in session.
- **Windows:** the installer installs for the person, needing no administrator, into `%LOCALAPPDATA%\Programs\Apertrust`.

## Try a package

- **macOS:** open the disk image and drag Apertrust to Applications. The app is signed ad hoc, not notarised, so the first time: Control-click it › Open (macOS 14 and earlier), or System Settings › Privacy & Security › Open Anyway (macOS 15 and later).
- **Windows:** run the installer; SmartScreen asks first (More info › Run anyway). Apertrust then appears under Open With for raw files.
- **Linux, the AppImage:**

  ```sh
  chmod +x apertrust_*_x86_64.AppImage
  ./apertrust_*_x86_64.AppImage PHOTO.RAW
  ```

- **Linux, the tarball** into `~/.local`, for the menu entry, the icons and Open With:

  ```sh
  tar -xzf apertrust_*_x86_64.tar.gz -C ~/.local --strip-components=1
  update-desktop-database ~/.local/share/applications
  update-mime-database ~/.local/share/mime
  gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
  ```

- **The metadata** on Linux: `desktop-file-validate linux/org.apertrust.Apertrust.desktop` and `appstreamcli validate linux/org.apertrust.Apertrust.metainfo.xml`.

## What is here

| File | What it is |
| --- | --- |
| `Packager.toml` | cargo-packager's configuration for every platform |
| `macos/Info.plist` | The bundle's keys, merged over cargo-packager's |
| `macos/dmg/` | The disk image's background and Finder layout; `tools/build-dmg-background.sh` writes the background |
| `windows/installer.nsi` | The installer's script, with the Open With entries and the uninstaller |
| `linux/org.apertrust.Apertrust.desktop` | The desktop entry, with every raw format's MIME type |
| `linux/org.apertrust.Apertrust.metainfo.xml` | The AppStream metadata |
| `linux/org.apertrust.Apertrust.mime.xml` | MIME types for the raw formats shared-mime-info does not name |

The app icon's PNGs, `.icns` and `.ico` are made from its SVGs by `tools/build-icons.sh`.
