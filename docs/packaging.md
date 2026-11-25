# Packaging

How Apertrust becomes a package a person installs on macOS, Windows and Linux: what each package holds, how to build one, and what is still missing (decision 68).

One tool builds every package, [cargo-packager](https://github.com/crabnebula-dev/cargo-packager), from one file, `packaging/Packager.toml`, but the `.rpm`, which cargo-packager does not make: [cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm) builds it from `packaging/linux/rpm.toml`. `tools/package.sh` builds the release binaries for the computer it runs on, fills in the version and runs them. Both are build tools, installed on the machine that packages; nothing of them is compiled into Apertrust or shipped with it.

## The packages

| Platform | Package | What it holds |
| --- | --- | --- |
| macOS | `Apertrust.app` | `Contents/MacOS/apertrust` (the app) and `apertrust-cli`; `Contents/Resources/` with the icon, `LICENSE`, `CREDITS.md` and `licences/`; the bundle's `Info.plist` |
| macOS | `apertrust-<version>-macos-arm64.dmg` for Apple silicon and `apertrust-<version>-macos-x86_64.dmg` for Intel | The app and a link to Applications, with the icon as the volume's, in a window of Apertrust's own with a background (below), signed ad hoc; one image per architecture, so each download holds only its own code |
| Windows | `apertrust-<version>-windows-x64-setup.exe`, or `-windows-arm64-setup.exe` for Windows on Arm | An NSIS installer for the person installing, with no administrator's rights, into `%LOCALAPPDATA%\Programs\Apertrust`: both executables and the licences, a Start menu shortcut, the raw file types in Open With, the edit's as Apertrust's own, and an uninstaller (below, Windows) |
| Windows | `apertrust-<version>-windows-x64-portable.zip`, or `-windows-arm64-portable.zip` | An `Apertrust` folder with both executables and the licences, to run from anywhere |
| Linux | `apertrust-<version>-linux-x86_64.AppImage`, or `-linux-aarch64.AppImage` for Arm | The app and the CLI in `usr/bin`, the desktop file, the AppStream metadata, the MIME types, the icons and the licences (in `usr/lib/apertrust`) |
| Linux | `apertrust-<version>-linux-amd64.deb`, or `-linux-arm64.deb` for Arm | A Debian package for Debian, Ubuntu and their derivatives: the app and the CLI in `/usr/bin`, the rest in `/usr/share` (below, Linux) |
| Linux | `apertrust-<version>-linux-x86_64.rpm`, or `-linux-aarch64.rpm` | An RPM package for Fedora, RHEL 10 and openSUSE, with the same files |
| Linux | `apertrust-<version>-linux-x86_64.tar.gz`, or `-linux-aarch64.tar.gz` | The same laid out as a prefix (`bin/`, `share/applications`, `share/metainfo`, `share/mime/packages`, `share/icons/hicolor`, `share/doc/apertrust`), to copy into `~/.local` or `/usr/local` |

**The file names** are `<name>-<version>-<os>-<arch>[-<kind>].<ext>`, all lower case but the AppImage's extension, `.AppImage`, with `-` between every part: the name `apertrust`, or `apertrust-nightly` (below); the system `macos`, `windows` or `linux`; the architecture in the system's own words, `arm64` or `x86_64` on macOS, `x64` or `arm64` on Windows, `x86_64` or `aarch64` on Linux, but Debian's own, `amd64` or `arm64`, in the `.deb`, as its control file names it; and the kind on Windows, `setup` for the installer and `portable` for the zip. So `apertrust-0.2.0+615-macos-arm64.dmg`, `apertrust-nightly-0.2.0+615-linux-aarch64.AppImage`, `apertrust-0.2.0+615-linux-amd64.deb`, `apertrust-0.2.0+615-linux-x86_64.rpm`. cargo-packager has no template for its files' names, so `tools/package.sh` renames the disk image, the installer, the AppImage and the `.deb` it writes, mapping the target to `<os>-<arch>` in one function, and names the `.rpm`, the zip and the tarball itself.

**The licences** in every package: `LICENSE` (the GPL), `CREDITS.md` (every component with its licence, and the fonts' and icons' notices, which About also shows), `licences/OFL.txt` (the fonts' licence: the binary compiles IBM Plex Sans in, decision 64) and `licences/LICENSE-Lucide` (the interface's icons).

**Nothing else is bundled.** The app's data, the settings schema among it, is compiled into the binaries (decision 45), so no package has a data folder. On Linux the platform's libraries stay the system's (decision 5): the Vulkan loader and drivers, libwayland, libxkbcommon, the X11 libraries and libdbus are loaded at run time, left out of the AppImage and named as dependencies of the `.deb` and the `.rpm`.

**The version** is the build's (decision 44), as `--version` prints it, `0.2.0+12~gabc1234`. A package's own version is the release's three numbers and the count of commits, `0.2.0+615`, since installers take numbers only: the bundle's `CFBundleShortVersionString` is `0.2.0` and its `CFBundleVersion` `615`, the Windows executable's version resource `0.2.0.615` with the full version as its text, the `.deb`'s `Version` and the `.rpm`'s `Version` `0.2.0+615` (with the `Release` `1`), and the files are named with `0.2.0+615`. Both formats take it as it is: it has only digits, dots and `+`, never Debian's revision separator `-`, which RPM's version forbids, nor `~`, which both sort before the version it follows; the script drops the build's `~gHASH` with the rest after the three numbers. A `+` sorts after, so `0.2.0+615` is newer than `0.2.0` to apt and to rpm alike, and newer than `0.2.0+614`. With `APERTRUST_VERSION` set, the version is the packager's and the package's is its leading `X.Y.Z`, which it must have.

## The metadata

In `packaging/`, beside the configuration:

- `macos/Info.plist`: the bundle's keys, merged over those cargo-packager writes: the name, the version, macOS 11 at least (the oldest that runs on Apple silicon, for both architectures), the photography category, the interface's languages (en-GB, uk, pl), and the raw document types, Apertrust an editor of them at the Alternate rank, so it is offered in Open With and never takes over the raws another app opens; and folders, as a viewer at the rank None, so a folder dropped on the Dock icon opens while Apertrust is never offered as a folder's app. Apertrust's own type, the edit (`.apxmp`), is exported as `org.apertrust.edit`, conforming to `public.xml` and `public.data`, with Apertrust its editor at the rank Owner, so a double-click opens it; its icon is macOS's own document page with the word APXMP on it (`UTTypeIconText`), no icon file, which macOS draws once the bundle is signed, as the script signs it ad hoc. Each type is the system's where macOS declares one; seven formats macOS does not name (ARI, CRM, DCS, KDC, MEF, QTK, X3F) are imported as `org.apertrust.raw-image.<extension>`. The binary embeds `crates/app/assets/macos/Info.plist` for a run without a bundle; its keys agree with these.
- `linux/org.apertrust.Apertrust.desktop`: the desktop entry, with the MIME types of every raw format and the edit's, `application/x-apertrust-edit`, its `Exec` passing `%F`, the files alone ([Opening a raw from the system](#opening-a-raw-from-the-system)); `Icon=` and the window's Wayland app id are the app's id, so the compositor finds the window's icon.
- `linux/org.apertrust.Apertrust.metainfo.xml`: the AppStream metadata: the name, the summary, the GPL-3.0-or-later licence, the homepage and the repository.
- `linux/org.apertrust.Apertrust.mime.xml`: the MIME types for the five formats shared-mime-info does not name (ARI, CRM, DCS, ORI, QTK), and the edit's, `application/x-apertrust-edit`, an XML file named `*.apxmp`, which Apertrust is then the one app to declare, so a double-click opens its photo; installed as `share/mime/packages/org.apertrust.Apertrust.xml`.
- **Windows**: the executable's icon and version resource are compiled by the app's `build.rs` through winresource and the SDK's `rc.exe`. `windows/installer.nsi` is the installer's script, cargo-packager's own template at 0.11.8 with Apertrust's changes (below, Windows), which registers every raw extension in `Packager.toml`'s list, and the edit's, `.apxmp`, its second association.

**The file types are tested.** `crates/app/tests/packaging.rs` holds the bundle's document types, the desktop file's MIME types, the MIME package and the installer's extensions, in the config and as its template registers and removes them, to `apertrust_raw::extensions()`, the formats rawler decodes, so a rawler update that adds one fails until the packages open it too; and the edit's type, its document entry, its registration and its removal, and its MIME type in the MIME package and the desktop file. It also holds the app's id to be the same in every file.

**The disk image's window**, what Finder shows when the image is mounted, is Apertrust's in `packaging/macos/dmg`: a 660 × 400 point window on the dark theme's ground, the logo at the top, the app's icon and the Applications link side by side with a thin arrow between them, and "Drag Apertrust to Applications" under them. Finder draws the icons' labels in black over a background picture, in dark mode too, so each sits on a light plate. Its files:

- `background.svg`: the source, in the dark theme's tokens, with the layout in its comment; the logo is linked and the caption outlined, so it renders with no fonts. To change the words, set them as text, outline them with usvg and replace the path.
- `background.tiff`: the background at 1× (660 × 400, 72 dpi) and 2× (1320 × 800, 144 dpi) in one file, as Finder takes a background for Retina screens; written by `tools/build-dmg-background.sh`, which renders the SVG with resvg through the app's `dmg_background` example and joins the two with macOS's `tiffutil`. Run it on a Mac after the SVG or the logo changes and commit the TIFF.
- `layout.DS_Store`: Finder's view of the window: its size, icon view with no toolbar, icons of 128 points with labels of 13, the app at (170, 196) and Applications at (490, 196), and the background. `layout-nightly.DS_Store` is the same for Apertrust Nightly's image (below), its volume and its bundle named `Apertrust Nightly`. `generate.py` writes both from nothing, holding no path, user or disk of the machine that ran it; run it by hand when the layout changes, with `pip install ds_store==1.3.3 mac_alias==2.2.3` (both MIT) in a virtual environment, and commit the files, so packaging never needs Python.

`tools/package.sh` adds both to each image after cargo-packager makes it, with `hdiutil` alone: the image made writable, attached with no Finder window, given `.background/background.tiff` and `.DS_Store`, compressed again and signed again. The layout finds its background by the volume's name, so the volume is the product's, `Apertrust` or `Apertrust Nightly`, with no version, as cargo-packager names it; the script stops on any other.

**The icons** are rendered from the mark's SVGs by `tools/build-icons.sh` into `crates/app/assets/icons`, beside their sources and the window's icon, which the binary compiles from the same tree: the `.icns`, the `.ico` (16 to 256 px) and the hicolor tree, named after the app id ([its README](../crates/app/assets/icons/README.md)). The edit's document icon on Windows, `apertrust-apxmp.ico`, is the executable's icon resource 2, beside the app's, 1 (`crates/app/build.rs`); on macOS the system draws the edit's.

## Windows

- **A windowed program.** `apertrust.exe` is built for Windows' window subsystem, in a release build and a debug one alike (`#![windows_subsystem = "windows"]`), so no console opens beside it from Explorer or the Start menu. `apertrust-cli.exe` stays a console program.
- **Its terminal.** Started from one, the app attaches to its console first thing (`AttachConsole(ATTACH_PARENT_PROCESS)`, in `apertrust-display`), before the log is set up, and points stdout and stderr at it where they have no handle; one redirected to a file or a pipe is kept. So `--version`, `-d`'s lines and its errors print there; started from Explorer, nothing is printed. cmd and PowerShell do not wait for a windowed program, so its lines come after their prompt; `start /wait apertrust --version` in cmd, or a pipe (`apertrust --version | more`), waits.
- **The install folder** is `%LOCALAPPDATA%\Programs\Apertrust`, Windows' place for a person's programs; `%LOCALAPPDATA%\Apertrust` is left to the cache folder (`docs/settings.md`).
- **Open With only.** For each raw extension, in `HKCU\Software\Classes`, the installer writes the ProgID `Apertrust.RawImage` (its name, icon and `shell\open\command`), the ProgID's name under `.<ext>\OpenWithProgids`, and the app's own entry, `Applications\apertrust.exe`, with its name, icon, command and the extensions under `SupportedTypes`. It never writes a raw extension's default value, so Apertrust is offered in Open With and becomes a type's default only when the person chooses it. Each `shell\open` verb, the ProgIDs' and the `Applications` entry's, has `MultiSelectModel` = `Document` ([decision 89](decisions/0089-one-instance-and-one-window-per-user-data-folder.md)): where the person makes Apertrust a type's default, Open on several files starts the app once per file, each start handing its file to the running instance. `Player` would invoke it once, its `%1` the first file alone. Windows shows a command-line verb with `Document` for up to 15 selected files and hides it past that, so larger selections go through Send To.
- **Send To.** Explorer offers Open With for one file alone, so the installer adds `Apertrust.lnk` (Apertrust Nightly's `Apertrust Nightly.lnk`) to the person's Send To folder (`%APPDATA%\Microsoft\Windows\SendTo`): Send To on several raws starts the app once with every file on its command line, without making it any type's default. Windows limits a command line to 32,767 characters, a few hundred files with typical paths, so a larger selection cannot all arrive this way. It is part of the file types' registration, not one of the shortcuts `/NS` or the Start menu page's "Do not create shortcuts" leaves out. Explorer offers it for any selection, folders included, and the app takes what it passes as a command line's paths: raws and edits open one photo, a folder opens folder mode, and anything else is refused with the alert. Windows 11 shows Send To under Show more options. The uninstaller removes it.
- **The edit, Apertrust's own.** `.apxmp` gets the ProgID `Apertrust.Edit`, named "Apertrust edit", its `DefaultIcon` the executable's document icon, `"<install folder>\apertrust.exe",-2`, and the same `shell\open\command`, which opens the edit's photo; the ProgID's name under `.apxmp\OpenWithProgids`, `.apxmp` in the `Applications` entry's `SupportedTypes`, and, as Apertrust owns the extension, `.apxmp`'s default value, `Apertrust.Edit`.
- **The uninstaller** removes exactly those: `.apxmp`'s default where it is still `Apertrust.Edit`, the ProgIDs' names from each `OpenWithProgids` (the key, and the extension's, only where nothing else is left in it), the ProgIDs, the `Applications` entry, the Start menu entry (in the folder chosen at install, which the installer's key keeps), the desktop shortcut, the Send To shortcut, the files it installed with the install folder once empty, the uninstall entry and the installer's own key. The person's settings and edits, elsewhere, stay. A default the person chose for a type is the system's to forget.

## Linux

The `.deb` and the `.rpm` install into `/usr`, from the tree `tools/package.sh` lays out for the tarball:

- `/usr/bin/apertrust` and `/usr/bin/apertrust-cli`;
- `/usr/share/applications/org.apertrust.Apertrust.desktop`, `/usr/share/metainfo/org.apertrust.Apertrust.metainfo.xml` and `/usr/share/mime/packages/org.apertrust.Apertrust.xml`;
- the icons in `/usr/share/icons/hicolor/<size>/apps/org.apertrust.Apertrust.png`, and the scalable `.svg`;
- the licences in `/usr/share/doc/apertrust/`: `LICENSE`, `CREDITS.md` and `licences/`.

cargo-packager's `.deb` also holds what it puts in every Linux package, the AppImage's too: the licences again in `/usr/lib/apertrust` and the icons again under the binary's name, `apertrust.png`. Its own desktop file, named after the binary, is turned off (`generateDesktopEntry = false`), since the desktop file must be named after the app id, which the window's Wayland app id and the metadata's `launchable` name; the AppImage takes ours under the binary's name, as cargo-packager links it at the AppImage's top.

**No maintainer scripts.** The system's triggers refresh the desktop, MIME and icon caches when either package installs or removes its files: dpkg's on Debian and Ubuntu (desktop-file-utils, shared-mime-info, hicolor-icon-theme), rpm's file triggers on Fedora, RHEL and openSUSE. The `.rpm` needs no shell (`require-sh = false`).

**Dependencies**, those the binaries link and those they load at run time ([0005](decisions/0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)); `crates/app/tests/packaging.rs` holds the two lists to the same libraries, which the AppImage leaves out:

| Library | `.deb`: `Depends` (Ubuntu 22.04 and later, Debian 12 and later) | `.rpm`: `Requires` (Fedora, RHEL, openSUSE) |
| --- | --- | --- |
| glibc | `libc6 (>= 2.35)`, the newest version the binaries' symbols need, read with `objdump` | `libc.so.6(GLIBC_…)(64bit)` and `libm.so.6(GLIBC_…)(64bit)`, found by cargo-generate-rpm |
| libgcc | `libgcc-s1` | `libgcc_s.so.1()(64bit)`, found by cargo-generate-rpm |
| The Vulkan loader | `libvulkan1` | `libvulkan.so.1()(64bit)` |
| libwayland | `libwayland-client0` | `libwayland-client.so.0()(64bit)` |
| libxkbcommon | `libxkbcommon0`, `libxkbcommon-x11-0` | `libxkbcommon.so.0()(64bit)`, `libxkbcommon-x11.so.0()(64bit)` |
| The X11 libraries winit loads | `libx11-6`, `libx11-xcb1`, `libxcb1`, `libxcursor1`, `libxi6` | `libX11.so.6()(64bit)`, `libX11-xcb.so.1()(64bit)`, `libxcb.so.1()(64bit)`, `libXcursor.so.1()(64bit)`, `libXi.so.6()(64bit)` |
| libdbus, for the file dialogs' portal | `libdbus-1-3` | `libdbus-1.so.3()(64bit)` |
| A Vulkan driver, recommended | through `libvulkan1`, which recommends `mesa-vulkan-drivers \| vulkan-icd` | `Recommends: (mesa-vulkan-drivers or libvulkan_lvp or libvulkan_radeon or libvulkan_intel or libvulkan_nouveau)` |

- **Debian's names** are each library's package, the same from Ubuntu 22.04 and Debian 12 on. cargo-packager 0.11.8 writes `Depends` alone, no `Recommends`; libvulkan1's own recommendation brings a driver, which apt installs by default.
- **RPM's capabilities** name each library by its file, as rpm itself does, so one `.rpm` installs on Fedora, RHEL and openSUSE, whose packages of them are named differently. rpm marks every 64-bit library `(64bit)`, on aarch64 as on x86_64, so the list is the same for both.
- **The build base.** The nightly builds make the Linux packages in an Ubuntu 22.04 container: a binary needs the glibc it was linked against or newer, and Ubuntu 22.04's, 2.35, is the oldest of the systems the packages are for, so one package of each runs on Ubuntu 22.04 and later, Debian 12 and later, Fedora, openSUSE Leap 15.6 and Tumbleweed, and RHEL 10. RHEL 9's glibc is 2.34: the binaries need 2.35 for `hypot`, whose symbol glibc 2.35 versioned anew, so neither package installs there.

**Installing:**

```sh
# From where the package was downloaded, ~/Downloads here.
sudo apt install ~/Downloads/apertrust-*-linux-amd64.deb       # Debian, Ubuntu
sudo dnf install ~/Downloads/apertrust-*-linux-x86_64.rpm      # Fedora, RHEL 10
sudo zypper install --allow-unsigned-rpm ~/Downloads/apertrust-*-linux-x86_64.rpm   # openSUSE
```

`sudo apt remove apertrust`, `sudo dnf remove apertrust` or `sudo zypper remove apertrust` removes it. `tools/install.sh` downloads and installs the right one itself ([Installing with the script](#installing-with-the-script)).

**Checked** on x86_64 in containers of Ubuntu 22.04 and 24.04, Debian 12, Fedora 43, AlmaLinux 10 (RHEL 10's), and openSUSE Leap 15.6 and Tumbleweed: installed with its dependencies, both programs run, `apertrust-cli gpu` finds lavapipe, the Vulkan driver the dependencies brought (but on openSUSE, whose container images install no recommended package), the caches name its desktop file, MIME types and icons, and removing it removes its files and its desktop file's entries, Apertrust and Apertrust Nightly installed side by side.

## Apertrust Nightly

The nightly builds are Apertrust Nightly, a development build that installs and runs beside Apertrust: its own name, id, executables, package files and user data folder ([settings.md](settings.md), Development builds), so neither replaces the other.

**Which builds are Nightly:** the nightly packages are always Nightly; the nightly job says so explicitly. Every other build, from source or `cargo run`, stays Apertrust while no release tag exists, and becomes Nightly once one does: a build not on a release tag, when the history has one.

`tools/package.sh --nightly` packages it. The binaries are built with `APERTRUST_NIGHTLY=1`, which makes them name themselves Apertrust Nightly and keep their settings in its folder, and are copied as `apertrust-nightly` and `apertrust-nightly-cli`: the Windows installer registers Open With and finds a running app by the executable's name, and the tarball's `bin/` shares a prefix with Apertrust's. The packaging files are given the identity `packaging/nightly-identity.txt` lists, each line a text and what replaces it; without `--nightly` they are used as they are. The Linux tree the script lays out names the MIME types, the icons and the licences' folder after Apertrust Nightly's id and name itself.

| Platform | Apertrust Nightly's packages |
| --- | --- |
| macOS | `apertrust-nightly-<version>-macos-arm64.dmg` and `apertrust-nightly-<version>-macos-x86_64.dmg`, the volume `Apertrust Nightly`, laid out by `layout-nightly.DS_Store`, holding `Apertrust Nightly.app`: the bundle id `org.apertrust.Apertrust.Nightly`, its name `Apertrust Nightly`, the executables `apertrust-nightly` and `apertrust-nightly-cli`, the edit's type at the rank Alternate, so it opens edits but never takes them from Apertrust |
| Windows | `apertrust-nightly-<version>-windows-x64-setup.exe` and `-windows-arm64-setup.exe`, installing into `%LOCALAPPDATA%\Programs\Apertrust Nightly` with its own uninstall entry, Start menu shortcut `Apertrust Nightly`, ProgIDs `Apertrust.Nightly.RawImage` and `Apertrust.Nightly.Edit`, the edit's in Open With alone, never `.apxmp`'s default, and Open With entry `Applications\apertrust-nightly.exe`; `apertrust-nightly-<version>-windows-x64-portable.zip` and `-windows-arm64-portable.zip`, an `Apertrust Nightly` folder |
| Linux | `apertrust-nightly-<version>-linux-x86_64.AppImage`, `-linux-amd64.deb`, `-linux-x86_64.rpm` and `-linux-x86_64.tar.gz`, and `-linux-aarch64` (`-linux-arm64.deb`) for Arm, each AppImage with its `.zsync` (below, Nightly builds), with `apertrust-nightly` and `apertrust-nightly-cli`, the desktop entry `org.apertrust.Apertrust.Nightly.desktop` (named Apertrust Nightly), the metadata `org.apertrust.Apertrust.Nightly.metainfo.xml`, the MIME types `org.apertrust.Apertrust.Nightly.xml`, the icons under its id, `org.apertrust.Apertrust.Nightly.png`, the same picture, and the licences in `share/doc/apertrust-nightly`, so the `.deb` and the `.rpm` install beside Apertrust's with no file in common |

`crates/app/tests/packaging.rs` applies `nightly-identity.txt` to the files as the script does, and checks that each line still matches and that every package names Apertrust Nightly by its own id.

## Opening a raw from the system

Decision 46. A raw opened with Apertrust from the system, by Open With or a double-click, opens that photo alone, [one photo](library.md#one-photo), as `apertrust FILE` from a terminal, File › Open… and a drop do; several opened together open together. An edit, `IMG.ARW.apxmp`, opens its photo the same way, the name without its lower-case `.apxmp` (`sidecar::photo_for`); one whose photo is not beside it is refused with the alert, which names the photo. Each platform delivers them its own way:

- **macOS** sends Open With, a double-click and a drop on the Dock icon as an open-documents event, not arguments, at the launch or while the app runs; `apertrust-display` receives it (`receive_opened_files`). The bundle's document types, `CFBundleDocumentTypes` in `Info.plist`, declare the raw types, the edit's, and folders at the rank None, for the Dock icon alone. Files that come while the window is minimised bring it back and to the front.
- **Windows:** the ProgID's and the `Applications` entry's `shell\open\command` pass `"%1"`, the file alone.
- **Linux:** the desktop entry's `Exec` passes `%F`, every file chosen, alone; started from a menu it passes none, and nothing opens. Apertrust Nightly's entry declares the edit's type too, Linux having no rank to keep it from a desktop's default; where both are installed, a desktop that takes the first entry in its cache's order takes Apertrust's, `org.apertrust.Apertrust.desktop`.

The app takes the paths on its command line as a drop's, whoever passed them, Windows' Send To among them, whatever its selection holds, so it never needs to tell a launcher from a terminal: looked at off the interface's thread once the window is open, a file that opens nothing refused with an alert in the window, a launcher having no terminal, its reason also on stderr; `apertrust IMG.ARW.apxmp` from a terminal opens its photo, as a double-click on the edit does. **`--in-folder`**, which the registrations passed while Open With opened the photo's folder and a terminal the photo alone, is accepted and ignored, left out of `--help`, until the first release is tagged, so a desktop entry or a registration an earlier nightly build wrote still opens its files; it is removed in that release, whose registrations never passed it.

**A folder from the system** opens in folder mode from a drop on the Mac's Dock icon, from Windows' Send To, which Explorer offers for a folder as for any selection, and from a terminal, `apertrust FOLDER`. Neither Windows' registrations nor the Linux desktop entry declare folders (no `inode/directory`), so Apertrust is never offered in a folder's Open With or as its default app; a folder passed on the command line, as a script may give, opens it too.

`tests/packaging.rs` holds the desktop entry and both installer lines to passing the files alone, with no `--in-folder`, and the folder's document type to the rank None.

**One instance and one window per user data folder** ([decision 89](decisions/0089-one-instance-and-one-window-per-user-data-folder.md); [gui.md](gui.md#instances-and-windows)). On Windows and Linux a start that finds the app running on its user data folder hands its paths, already absolute, and its activation token to that instance through a named pipe or an abstract Unix socket, and exits; the instance opens them as an open from outside, replacing its window's place. By platform:

- **macOS:** LaunchServices sends Open With, a double-click and a Dock drop to the running app as the open-documents event, and activates it; there is no channel. A start of the binary that finds the lock held, from a terminal say, is refused on stderr with exit status 1. An instance on another user data folder takes an open-documents event LaunchServices delivers to it, as any instance does.
- **Windows:** Open With takes one file. Several come through Send To, one start with every file, or through Open where Apertrust is the type's default, one start per file, each handing off. A start that finds the instance hands off, calling `AllowSetForegroundWindow` so the instance may come to the front, and the instance joins the files, each arriving within 500 ms of the one before, into one open, so raws opened together open together while their hand-offs arrive less than 500 ms apart; a later one is an open of its own. While the instance is starting, every hand-off joins its own command line. The switch test opens 15 raws with Open, the most Explorer offers it for with `Document`, Apertrust the type's default, on Windows 11, the app closed and running, and records the largest gap between the hand-offs' arrivals at the instance, the gap the join times; Send To cannot stand in, being one start. Above 250 ms, Explorer's `DelegateExecute` verb replaces the join before the first release.
- **Linux:** the desktop entry passes every file in one start, which hands off with `XDG_ACTIVATION_TOKEN` on Wayland, or `DESKTOP_STARTUP_ID` on X11, so the compositor lets the window come forward.

Every user data folder is the same application: one app id, `org.apertrust.Apertrust`, one icon and one set of file associations, opens from outside reaching, on Windows and Linux, the instance on the folder a start with no `--user-data-dir` uses: `APERTRUST_USER_DATA_DIR`'s where the desktop session sets it, else the platform's. Apertrust Nightly, with its own identity, is the one separate application.

**Checked by hand** on each platform, since no test drives the system's opening:

- macOS: a double-click on a raw with the app closed and with it running; a double-click on an edit, and on one whose photo is gone; the edit's icon in Finder, the word APXMP on the page; Open With; raws and a folder dropped on the Dock icon; several raws from one folder and from two; a raw opened while the window is minimised, which brings it back; one opened under About or Settings, which opens once it closes.
- Windows: Open With and a double-click on a raw after installing, the registry's command passing the file alone; Send To on 20 raws opening them together in one window; Open on up to 15, Apertrust the type's default, doing the same (still to check, #346); a folder through Send To opening in folder mode; a raw opened while the app runs coming to the front; a double-click on an edit, its icon in Explorer; a file that opens nothing refused with the alert; after uninstalling, `.apxmp`'s default gone where it was still Apertrust's.
- Linux: a file manager's Open With with one file and several, a double-click on an edit opening its photo, and the app started from the menu with none; a file opened while the app runs coming to the front on GNOME and KDE, on Wayland and X11.

## Building one

[packaging/README.md](../packaging/README.md) is the short version: what each platform needs, the command, and how to try each package.

```sh
cargo install --locked cargo-packager --version 0.11.8   # once; the script checks the version
cargo install --locked cargo-generate-rpm --version 0.21.1   # once, on Linux; checked too
tools/package.sh                 # this platform's packages into target/package/<target>/out
tools/package.sh --nightly       # Apertrust Nightly's, as the nightly builds are
APERTRUST_VERSION=0.2.0 tools/package.sh                  # a packager's version
```

- **macOS**: Xcode's command-line tools (`lipo`, `codesign`, `hdiutil`). The script packages each Apple target installed, a disk image for Apple silicon and one for Intel (`rustup target add aarch64-apple-darwin x86_64-apple-darwin` installs both); `--target aarch64-apple-darwin` or `--target x86_64-apple-darwin` packages one. The disk image is made without driving Finder, its window laid out by the committed `.DS_Store` (above); `--finder-layout` also has create-dmg arrange it in Finder first, which needs a logged-in session and may ask to let the terminal control Finder, and the committed layout then replaces its own.
- **Windows**: the MSVC toolchain with the Windows SDK, whose `rc.exe` compiles the resources, from Git Bash; the portable zip is written by PowerShell's `Compress-Archive`. An x64 or Arm system packages its own; `--target aarch64-pc-windows-msvc` names Arm's. From PowerShell, the script runs through Git's bash:

  ```powershell
  & "C:\Program Files\Git\bin\bash.exe" tools/package.sh --nightly
  ```
- **Linux**: an x86_64 or aarch64 system with `objdump` (binutils), which the `.deb`'s glibc is read with, and `ldd`, which cargo-generate-rpm finds the `.rpm`'s libraries with. The packages need the glibc they are built on or newer, so a build for others is made on the oldest system they are for, as the nightly builds are (above, Linux). The AppImage runs linuxdeploy without FUSE; in a container with no FUSE at all, `APPIMAGE_EXTRACT_AND_RUN=1` has linuxdeploy's AppImage plugin extract itself too.
- **What cargo-packager downloads** into its tools folder the first time: the create-dmg script, pinned by its commit, on macOS; NSIS and its plugins, checked by SHA-1, on Windows; linuxdeploy, its AppImage plugin and AppRun on Linux.

## The CLI

Every package holds `apertrust-cli` beside the app:

- **macOS**: `Apertrust.app/Contents/MacOS/apertrust-cli`. To put it on the `PATH`: `ln -s /Applications/Apertrust.app/Contents/MacOS/apertrust-cli /usr/local/bin/apertrust-cli` (or a folder of one's own on the `PATH`).
- **Windows**: `apertrust-cli.exe` in the install folder, `%LOCALAPPDATA%\Programs\Apertrust`, or in the zip's folder; add the folder to the user's `Path` to run it anywhere.
- **Linux**: `/usr/bin/apertrust-cli` from the `.deb` or the `.rpm`, on the `PATH`; the tarball's `bin/apertrust-cli`. The AppImage starts the app; its CLI is in `usr/bin` once extracted with `--appimage-extract`.

Planned with MCP (Phase 6; decision 108): `apertrust-mcp`, the MCP server an agent's client launches, beside `apertrust-cli` in every package, in the same places. In Flatpak and Snap the assistants' channel is a socket file in a runtime folder the sandbox shares, decided with those packages ([session-protocol.md](session-protocol.md#the-local-channel)).

## Unsigned packages

The packages are not signed with a developer's certificate yet, so each system warns the first time:

- **macOS**: the app and the disk image are signed ad hoc (`codesign --sign -`), with the hardened runtime, and not notarised, so Gatekeeper refuses a downloaded copy as from an unidentified developer, or calls it damaged. The disk image itself is refused before it opens. Removing the download's quarantine from the image, before it is opened, always works, and the app copied out of it opens too: `xattr -d com.apple.quarantine ~/Downloads/apertrust*.dmg`; for an app already copied from a quarantined image, `xattr -dr com.apple.quarantine /Applications/Apertrust*.app`. For the unidentified developer alone, opening it once and then choosing Open Anyway in System Settings › Privacy & Security also works (macOS 15 and later), as does Control-clicking the app and choosing Open (macOS 14 and earlier).
- **Windows**: SmartScreen says "Windows protected your PC" for the installer and the executables: More info › Run anyway.
- **Linux**: no package is signed; make the AppImage executable (`chmod +x`) and run it. apt and dnf install a local `.deb` or `.rpm` without a signature; zypper asks for `--allow-unsigned-rpm`.

Apertrust Nightly's are the same, under its names: `/Applications/Apertrust Nightly.app`, `apertrust-nightly-*-linux-x86_64.AppImage`.

## Installing with the script

`tools/install.sh` installs or updates Apertrust on macOS or Linux from GitHub: the latest release by default, or with `--nightly` Apertrust Nightly, its only option. No release is published yet, so without `--nightly` it finds no `SHA256SUMS`, says so and stops.

```sh
curl -q --proto '=https' --proto-redir '=https' -fsSL https://raw.githubusercontent.com/apertrust/apertrust/main/tools/install.sh | bash -s -- --nightly
curl -q --proto '=https' --proto-redir '=https' -fsSL https://raw.githubusercontent.com/apertrust/apertrust/main/tools/install.sh | bash   # the latest release
```

`-q`, first, keeps a `~/.curlrc` from changing how the script is fetched, and `--proto` and `--proto-redir` keep it, redirects included, on HTTPS, as the script's own downloads are. A pipe reports bash's status, never curl's, so a download that fails (a 404, no network) looks like a success. For a script or an unattended run, download first and run it only if that worked:

```sh
curl -q --proto '=https' --proto-redir '=https' -fsSL https://raw.githubusercontent.com/apertrust/apertrust/main/tools/install.sh -o install.sh && bash install.sh --nightly
```

- **The package** is chosen from the release's `SHA256SUMS` ([Nightly builds](#nightly-builds)), the one file named `apertrust-<version>` or `apertrust-nightly-<version>` and this system's ending, its version starting with a digit so neither channel takes the other's; it is downloaded over https alone, redirects too, by a `curl -q` that reads no `.curlrc`, so a user's configuration cannot weaken that, giving up on a server that takes 30 s to connect or sends under 1 KB/s for a minute (a large download still moving is never cut off), and checked against its line (`shasum -a 256 -c` on macOS, `sha256sum -c` on Linux). A release's `SHA256SUMS` comes from `releases/latest/download/`, GitHub's newest release that is not a pre-release, the nightly's from `releases/download/nightly/`. With no file, none of this system's, or more than one, it stops and says why.
- **The version**: the package's, `X.Y.Z+N`, its `+` published as `.` in the file's name. Before downloading, the script reads the version installed, from the app's `Info.plist` (`CFBundleShortVersionString`+`CFBundleVersion`, or the first alone where both are the same, as a packager's `APERTRUST_VERSION` makes them) on macOS, `dpkg-query` (installed and with no error, held or not) or `rpm -q` for the `.deb` or the `.rpm`, for the tarball the record the script writes once it has unpacked it whole, `doc/<package>/installed-version` in the data directory (the tarball row below), the package's version and the binary's `--version`, counted while the binary in `~/.local/bin` still prints that `--version` (its count of commits starts at the last release tag, the package's at the first commit), none where it cannot be read, and says what it does: `Installing Apertrust Nightly 0.0.0+392`, or `Updating Apertrust Nightly 0.0.0+380 → 0.0.0+392`; with the same version installed it says so and stops, changing nothing but, on macOS, where the installed executable has no build for this Mac's chip (read with `file`, part of macOS, an Intel-only app on Apple silicon): it replaces it, saying so, and an executable it cannot read is left to the version check; and, for the tarball, the menu and file types, refreshed again. Versions are compared as they are, never ordered, so the channel's current build installs over a newer one.
- **Running**: once the version decides it installs, it refuses while the app runs, `apertrust` or `apertrust-nightly`, and checks again just before installing: on macOS a process whose executable, as `ps` gives its path, is the app's, a bundle's wherever it is; on Linux a process of that name whose executable, in `/proc`, is the app's, so another program given a folder named `apertrust` is not taken for it.
- **macOS**: the disk image for the Mac's chip, `arm64` or `x86_64` from `uname -m`, `arm64` too for a shell under Rosetta (`sysctl.proc_translated`). Its quarantine is removed, since the packages are signed ad hoc and not notarised ([Unsigned packages](#unsigned-packages)); it is mounted read-only, the app's signature verified with `codesign --verify --deep --strict`, copied into a staging folder in `/Applications` and verified again, its quarantine removed, and only then put in place. The previous app is set aside in the staging folder and removed only once the new app is confirmed in place; it is put back if anything stops the install before, and if something appears at the app's path in between, the script stops and keeps the staging folder, with the new app and the previous one. sudo is asked for only where `/Applications` is not writable.
- **Linux**, by the package manager present, each architecture in its format's own words, sudo only where not root:

  | Found | Format | Architecture | Installed with |
  | --- | --- | --- | --- |
  | `apt-get` (Debian, Ubuntu) | `.deb` | `dpkg --print-architecture`: `amd64` or `arm64` | `apt-get install -y --allow-downgrades` as `env DEBIAN_FRONTEND=noninteractive` through sudo, with dpkg's `--force-confdef` and `--force-confold`, so that no question is asked |
  | `dnf` (Fedora, RHEL 10) | `.rpm` | `rpm --eval '%{_arch}'`: `x86_64` or `aarch64` | `dnf install -y`, then `dnf downgrade -y` where a newer one stays |
  | `zypper` (openSUSE) | `.rpm` | the same | `zypper --non-interactive install --allow-unsigned-rpm --oldpackage` |
  | none of them | `.tar.gz` | `uname -m`: `x86_64` or `aarch64` (`arm64` read as `aarch64`) | unpacked as the README does it, with no sudo: `bin/` into `~/.local/bin`, `share/` into the data directory, `$XDG_DATA_HOME` where that is an absolute path, else `~/.local/share`; then `update-desktop-database` and `update-mime-database` on it where present. The menu entry runs the program by its name, as the README's own steps leave it, so it needs `~/.local/bin` on the desktop session's `PATH`, as the terminal does; the script says when `~/.local/bin` is not on the `PATH`. A symlink for `~/.local/bin` or the data directory is followed, never replaced; one for a folder below them stops it before anything is unpacked |

  It never installs the AppImage.
- **Refused**: any other system than macOS and Linux, any other architecture, each naming what it found. Windows has its installer; nothing is uninstalled.
- **Unattended**: it asks nothing. sudo alone asks for a password, where one is needed, on the terminal; everything else runs with no stdin, since under `curl | bash` stdin is the script.
- **Its output**: a banner in a box, the logo's orange square with the name and the channel, then one line a step (the system, the architecture and the format with why, the package found, the download with curl's progress bar, the checksum, the install's own steps), each marked done, failed with its reason, or a warning, and a summary: what was installed, where, its version, the one it updated, and how to open it, the command by its full path, with a warning where the one on the `PATH` is another copy, an older tarball's in `~/.local/bin` say, which it never removes. A failure goes to stderr and ends with what was changed: nothing; on macOS the previous app put back, or where it is if it could not be; for the tarball, that `~/.local` may hold part of its files; for apt, dnf or zypper, that their output says whether they changed anything. Colour and the marks ✓, ✗ and ! only on a terminal, with `NO_COLOR` unset and `TERM` not `dumb`; otherwise `[ok]`, `[fail]` and `[warn]`.

## Nightly builds

`.github/workflows/nightly-builds.yml` builds Apertrust Nightly's packages each night at 00:00 UTC, each on its own platform, and publishes them as the `nightly` pre-release on GitHub, [releases/tag/nightly](https://github.com/apertrust/apertrust/releases/tag/nightly).

- **When**: only when main has changed since the commit the `nightly` tag points at. A run by hand from the Actions tab builds the same way, and its `force` input builds and publishes an unchanged main too. One run at a time; a second waits.
- **The jobs**: `check` compares main with the tag; then five builds, each `tools/package.sh --nightly` with cargo-packager 0.11.8 (and cargo-generate-rpm 0.21.1 on Linux), from a full clone so the version is computed. Each uploads its packages as an artifact, kept seven days.

| Job | Runner | Files |
| --- | --- | --- |
| `macos` | `macos-15` | Both chips' disk images, Intel's cross-built with `--target x86_64-apple-darwin` |
| `windows` | `windows-2025` | The x64 installer and zip |
| `windows-arm` | `windows-11-arm` | The arm64 installer and zip, with `--target aarch64-pc-windows-msvc`; rustup, pinned and checked by its SHA-256, is installed first should the image lack it |
| `linux` | `ubuntu-24.04`, in an `ubuntu:22.04` container | The x86_64 AppImage, its `.zsync`, the `.deb`, the `.rpm` and the tarball |
| `linux-arm` | `ubuntu-24.04-arm`, in an `ubuntu:22.04` container | The aarch64 AppImage, its `.zsync`, the `.deb`, the `.rpm` and the tarball |

- **The Linux container** is Ubuntu 22.04's image, pinned by its digest, for its glibc (above, Linux). Its first step installs what the runner's image has and the container lacks, before the checkout, which needs git for a full clone: `ca-certificates`, `curl`, `git`, `build-essential`, `file` and `zsync` from apt, and rustup, pinned and checked by its SHA-256 as on Windows on Arm. git is told the checkout, owned by the runner's user and not the container's root, is safe, so the version is computed. `APPIMAGE_EXTRACT_AND_RUN=1` runs linuxdeploy, its plugin and appimagetool without FUSE.

- **Updates of the AppImage**: each Linux job embeds the AppImage's update information, `gh-releases-zsync|apertrust|apertrust|latest-pre|apertrust-nightly-*-linux-<arch>.AppImage.zsync` (`x86_64` or `aarch64`), and publishes `<AppImage>.zsync` beside it, so AppImageUpdate fetches only the blocks that changed. It names the latest pre-release, `latest-pre` in the AppImage update-information specification, rather than the tag `nightly`: Gear Lever ignores a fixed tag and asks GitHub for the latest release, which is never a pre-release. AppImageUpdate takes `latest-pre` from `2.0.0-alpha-1-20251018` and its `continuous` build; an earlier one treats any tag holding "latest" as the latest release and finds no update, so update `appimageupdatetool` first. A release of Apertrust must therefore not be a pre-release, or nightly AppImages would look there for a file of their name and find none (#151). After packaging, the job gives the AppImage its published name, its `+` a `.`, so the `.zsync`'s URL names the published file; extracts it with `--appimage-extract`; and packs it again with appimagetool, `-u` and the update information, which writes the `.zsync` with zsyncmake (apt's `zsync`). appimagetool and its runtime are pinned to a release, appimagetool 1.9.1 and the type2 runtime 20251108, each checked against the SHA-256 the workflow holds. An AppImage downloaded while the files were named `apertrust-nightly_<version>_<arch>.AppImage` looks for a `.zsync` of that name, which the release no longer holds, so it finds no update: download the new AppImage once, and it updates from then on. In Gear Lever, one downloaded before its update information named `latest-pre` finds none either: download it once, or set its update source by hand (GitHub, `apertrust/apertrust`, the file name `apertrust-nightly-*-linux-<arch>.AppImage`, pre-releases allowed).
- **Publishing**: once every build has succeeded, `publish`, the one job that may write, makes the `nightly` release again: it deletes the release with its tag, then creates it through GitHub's Releases API on the commit built, with the night's files, titled `Nightly build <date>`. Beside them, `SHA256SUMS` gives each file's SHA-256 in `sha256sum`'s format (`<hash>  <file name>`), one line per file, written in this job from the files it uploads and replaced with them each night; `sha256sum -c --ignore-missing SHA256SUMS` (`shasum -a 256 -c --ignore-missing SHA256SUMS` on a Mac) checks a download beside it: its line in the output names the file with `OK`, and a renamed download is skipped, not checked; `tools/install.sh` chooses its package from it (above). The API makes the tag, since GitHub refuses the job's own token a `git push` of a tag whose commit changes workflow files. The release is missing for the minute between. The release's notes give the commit and the date, say that nightlies are untested development builds, and point to the README's [Installation](../README.md#installation) for installing and opening them unsigned. It is a pre-release, never the latest release. A file's version `+` is published as `.` (`apertrust-nightly-0.0.0.615-windows-x64-setup.exe`), as GitHub would otherwise rename it.
- **On failure**: when any build fails, nothing is published and the last nightly stays. A run from another branch builds but never publishes.
- **What is not there**: tests, which CI runs on every push to main, signing (above), and an APT or RPM repository: the `.deb` and the `.rpm` update when a newer one is installed by hand.

## Signing and notarisation, planned

- **macOS**: a Developer ID Application certificate (the Apple Developer Program) for `macos.signingIdentity`, and notarisation through `notarytool`, which cargo-packager runs once the certificate and an App Store Connect API key are given as `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_API_ISSUER`, `APPLE_API_KEY` and `APPLE_API_KEY_PATH`; the ticket is then stapled to the disk image.
- **Windows**: a code-signing certificate (an organisation's, or a cloud signing service's) for the installer and both executables, through `windows.certificateThumbprint` or `windows.signCommand`. An MSIX package for the Microsoft Store is planned beside the installer.
- **Linux**: Flatpak, through Flathub, with its runtime giving the platform's libraries; the desktop file, the metadata and the icons here are already named after the app id it needs.
- **CI**: a release workflow for version tags, building every package on its own platform as the nightly builds do and publishing `SHA256SUMS` beside them the same way, for `tools/install.sh`, comes next.

## Known gaps

- **Windows is untested**: the installer, the Open With entries, the edit's type and icon, the uninstaller and the terminal attach wait for the first build on Windows.
- **The dark icon**: an `.icns` holds one appearance; macOS's dark and tinted icons need an asset catalog.
- **RHEL 9**, and any system with a glibc older than 2.35, installs neither the `.deb` nor the `.rpm` (above, Linux); the tarball's and the AppImage's binaries fail to start there too.
- **The `.rpm`'s folders**: cargo-generate-rpm 0.21.1 cannot make a package own a folder (its `dir` assets fail), so removing the package leaves `/usr/share/doc/apertrust` and its `licences` empty on Fedora, RHEL and openSUSE Leap.
- **Older Linux systems**: the desktop file relies on shared-mime-info for 24 of the formats, and an older release may lack some of the newer ones (CR3, NRW, IIQ among them); such files then do not offer Apertrust until it is updated.
