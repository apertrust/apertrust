#!/usr/bin/env bash
# Apertrust, a free, non-destructive, cross-platform raw photo editor
# Copyright (C) 2026  Apertrust developers
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

# Builds Apertrust's packages for this computer's platform from packaging/Packager.toml
# with cargo-packager (docs/packaging.md): on macOS the app bundle and its disk image, on
# Windows the NSIS installer and a portable zip, on Linux the AppImage and a tarball, each
# with the app, the CLI and the licences, into target/package/<target>/out.
#
#   tools/package.sh [--target TRIPLE] [--finder-layout] [--dev]
#
# --dev packages Apertrust Dev, a development build that installs and keeps its settings
# beside Apertrust, as the nightly builds are (docs/packaging.md): the binaries are built
# with APERTRUST_DEV=1 and named apertrust-dev and apertrust-dev-cli, and the packages
# take the name, the id and the files packaging/dev-identity.txt gives.
#
# On macOS it packages each architecture apart, a disk image for Apple silicon and one for
# Intel, into target/package/aarch64-apple-darwin/out and x86_64-apple-darwin/out, for
# each Rust target installed (`rustup target add aarch64-apple-darwin x86_64-apple-darwin`);
# --target packages one. The bundles and the images are signed ad hoc, not notarised.
# Each image's window gets its background and Finder's layout of it from packaging/macos/dmg,
# added to the image cargo-packager made with hdiutil alone, so no Finder is driven.
# --finder-layout also has create-dmg arrange the window in Finder first, which needs a
# logged-in session and may ask to control Finder; the layout added after it replaces its own.
#
# The version is the build's, as `--version` prints it: from git, or APERTRUST_VERSION
# when a packager sets it (decision 44).
set -euo pipefail

cd "$(dirname "$0")/.."

# The cargo-packager this script and the config are written for.
PACKAGER=0.11.8
# The oldest macOS the bundle runs on, as packaging/macos/Info.plist says, for both
# architectures' builds.
export MACOSX_DEPLOYMENT_TARGET=11.0

finder_layout=false
dev=false
only=
while [[ $# -gt 0 ]]; do
    case "$1" in
    --finder-layout) finder_layout=true ;;
    --dev) dev=true ;;
    --target)
        only=${2:?--target needs a target triple}
        shift
        ;;
    *)
        echo "usage: tools/package.sh [--target TRIPLE] [--finder-layout] [--dev]" >&2
        exit 2
        ;;
    esac
    shift
done

# The package's identity: its files' and binaries' name, the name it shows, its id.
if [[ "$dev" = true ]]; then
    # Read by apertrust-base's build script: the binaries name themselves Apertrust Dev
    # and keep their settings in its folder.
    export APERTRUST_DEV=1
    package=apertrust-dev
    product="Apertrust Dev"
    id=org.apertrust.Apertrust.Dev
else
    package=apertrust
    product=Apertrust
    id=org.apertrust.Apertrust
fi

# The file $1 with the package's identity: as it is, or for --dev with each
# `FROM => TO` of packaging/dev-identity.txt replaced wherever it occurs.
with_identity() {
    if [[ "$dev" = false ]]; then
        cat "$1"
        return
    fi
    local text line from to
    text=$(cat "$1")
    while IFS= read -r line; do
        case "$line" in
        "" | "#"*) continue ;;
        esac
        from=${line%% => *}
        to=${line#* => }
        text=${text//"$from"/"$to"}
    done <packaging/dev-identity.txt
    printf '%s\n' "$text"
}

installed=$(cargo packager --version 2>/dev/null | awk '{ print $2 }') || true
if [[ "$installed" != "$PACKAGER" ]]; then
    echo "error: cargo-packager $PACKAGER is needed (found: ${installed:-none}):" >&2
    echo "  cargo install --locked cargo-packager --version $PACKAGER" >&2
    exit 1
fi

# The top folder as the platform's programs read it: C:/... on Windows, not /c/...
root=$(pwd -W 2>/dev/null || pwd)
target_dir=${CARGO_TARGET_DIR:-$root/target}
host=$(rustc -vV | sed -n 's/^host: //p')

case "$(uname -s)" in
Darwin) platform=macos ;;
Linux) platform=linux ;;
MINGW* | MSYS* | CYGWIN*) platform=windows ;;
*)
    echo "error: no package for $(uname -s)" >&2
    exit 1
    ;;
esac

build() {
    cargo build --release --locked -p apertrust -p apertrust-cli "$@"
}

exe=
if [[ "$platform" = windows ]]; then
    exe=.exe
fi
# The targets to package: on macOS each Apple target installed, one image each; elsewhere
# the host's.
if [[ -n "$only" ]]; then
    triples=$only
elif [[ "$platform" = macos ]]; then
    installed_targets=$(rustup target list --installed 2>/dev/null || true)
    triples=
    for arch in aarch64 x86_64; do
        if grep -qx "$arch-apple-darwin" <<<"$installed_targets"; then
            triples="$triples $arch-apple-darwin"
        fi
    done
    triples=${triples:-$host}
else
    triples=$host
fi

# The disk image's window (docs/packaging.md): its background, .background/background.tiff,
# and Finder's view of it, .DS_Store, from packaging/macos/dmg, added to the image that
# cargo-packager made. The image is made writable, attached where no Finder window opens,
# given both files, compressed again as cargo-packager compresses it, and signed again as
# packaging/Packager.toml signs it. The layout finds its background by the volume's name,
# so the volume must be the product's, Apertrust or Apertrust Dev, with no version, each
# with its own layout, which also places the app by its bundle's name.
dress_dmg() {
    dmg=$1
    work=$2
    rw=$work/dmg-rw.dmg
    mount=$work/dmg-mount
    rm -rf "$rw" "$mount"
    mkdir -p "$mount"
    hdiutil convert -quiet "$dmg" -format UDRW -o "$rw"
    hdiutil attach -quiet -nobrowse -noverify -noautoopen -mountpoint "$mount" "$rw"
    volume=$(diskutil info "$mount" | sed -n 's/^ *Volume Name: *//p')
    if [[ "$volume" != "$product" ]]; then
        hdiutil detach -quiet "$mount"
        echo "error: $dmg's volume is \"$volume\", not $product, which its layout needs" >&2
        exit 1
    fi
    layout=layout.DS_Store
    [[ "$dev" = true ]] && layout="layout-dev.DS_Store"
    mkdir -p "$mount/.background"
    cp packaging/macos/dmg/background.tiff "$mount/.background/background.tiff"
    cp "packaging/macos/dmg/$layout" "$mount/.DS_Store"
    rm -rf "$mount/.fseventsd"
    hdiutil detach -quiet "$mount"
    rm -f "$dmg"
    hdiutil convert -quiet "$rw" -format UDZO -imagekey zlib-level=9 -o "$dmg"
    rm -rf "$rw" "$mount"
    identity=$(sed -n 's/^signingIdentity = "\(.*\)"$/\1/p' packaging/Packager.toml)
    codesign --force --sign "${identity:?packaging/Packager.toml names no signingIdentity}" "$dmg"
}

package() {
    triple=$1
    if [[ "$platform" = macos ]] || [[ -n "$only" ]]; then
        build --target "$triple"
        bin_dir=$target_dir/$triple/release
    else
        build
        bin_dir=$target_dir/release
    fi

    # The version the build stamped, from the CLI, which every platform runs in a console:
    # "Apertrust CLI 0.0.0+615~ge26ca33 [macos arm64 release]". It is read from the host's
    # own build, since another architecture's binary may not run here (Intel's on Apple
    # silicon without Rosetta), and it is the same for every target.
    cli=$bin_dir/apertrust-cli$exe
    if [[ "$triple" != "$host" ]]; then
        cargo build --release --locked -p apertrust-cli --target "$host"
        cli=$target_dir/$host/release/apertrust-cli$exe
    fi
    # The version is the word before the three of "[macos arm64 release]", as the name may
    # be "Apertrust Dev CLI".
    full=$("$cli" --version | head -1 | awk '{ print $(NF - 3) }')
    short=${full%%[+~-]*}
    if ! grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+' <<<"$short"; then
        echo "error: the version $full does not start with X.Y.Z, which packages need" >&2
        exit 1
    fi
    if [[ -n "${APERTRUST_VERSION:-}" ]]; then
        version=$short
        build_number=$short
    else
        build_number=$(git rev-list --count HEAD)
        version=$short+$build_number
    fi
    echo "version $full, packaged as $version"

    stage=$target_dir/package/$triple
    out=$stage/out
    rm -rf "$out"
    mkdir -p "$out"
    # Apertrust Dev's binaries under their own names, so that its installer and tarball
    # never take Apertrust's place (Windows registers Open With by the executable's name).
    if [[ "$dev" = true ]]; then
        mkdir -p "$stage/bin"
        cp "$bin_dir/apertrust$exe" "$stage/bin/$package$exe"
        cp "$bin_dir/apertrust-cli$exe" "$stage/bin/$package-cli$exe"
        bin_dir=$stage/bin
    fi
    fill() {
        with_identity "$1" | sed -e "s|@ROOT@|$root|g" -e "s|@VERSION@|$version|g" \
            -e "s|@SHORT_VERSION@|$short|g" -e "s|@BUILD@|$build_number|g" \
            -e "s|@BINARIES@|$bin_dir|g" -e "s|@INFO_PLIST@|$stage/Info.plist|g" \
            -e "s|@STAGE@|$stage|g" >"$2"
    }
    fill packaging/Packager.toml "$stage/Packager.toml"
    fill packaging/macos/Info.plist "$stage/Info.plist"
    # The Linux desktop entry and metadata, named after the id, which the AppImage of
    # Apertrust Dev takes from here and the tarball of either.
    with_identity packaging/linux/org.apertrust.Apertrust.desktop >"$stage/$id.desktop"
    with_identity packaging/linux/org.apertrust.Apertrust.metainfo.xml \
        >"$stage/$id.metainfo.xml"

    case "$platform" in
    macos) formats=app,dmg ;;
    windows) formats=nsis ;;
    linux) formats=appimage ;;
    esac
    # create-dmg drives Finder to lay out the image's window unless CI is set.
    if [[ "$platform" = macos ]] && [[ "$finder_layout" = false ]]; then
        export CI=true
    fi
    cargo packager --config "$stage/Packager.toml" --formats "$formats" --target "$triple" \
        --out-dir "$out"

    # The licences, laid out as Packager.toml's resources are.
    licences() {
        mkdir -p "$1/licences"
        cp LICENSE CREDITS.md "$1/"
        cp crates/ui/assets/fonts/OFL.txt crates/ui/assets/icons/LICENSE-Lucide "$1/licences/"
    }

    case "$platform" in
    macos)
        codesign --verify --deep --strict "$out/$product.app"
        for dmg in "$out"/*.dmg; do
            dress_dmg "$dmg" "$stage"
            codesign --verify --strict "$dmg"
            # cargo-packager names the image after the product, "Apertrust Dev_…": the
            # file's name has no space, and -dev as the other packages' have.
            if [[ "$dev" = true ]]; then
                mv "$dmg" "$out/Apertrust-dev_${dmg##*/Apertrust Dev_}"
            fi
        done
        ;;
    linux)
        # A tarball laid out as a prefix, for ~/.local or /usr/local: bin, and share with the
        # desktop file, metadata, MIME types, icons and the licences.
        arch=${triple%%-*}
        name=${package}_${version}_$arch
        tree=$stage/tarball/$name
        rm -rf "$stage/tarball"
        mkdir -p "$tree/bin" "$tree/share/applications" "$tree/share/metainfo" \
            "$tree/share/mime/packages" "$tree/share/icons"
        cp "$bin_dir/$package" "$bin_dir/$package-cli" "$tree/bin/"
        cp "$stage/$id.desktop" "$tree/share/applications/"
        cp "$stage/$id.metainfo.xml" "$tree/share/metainfo/"
        cp packaging/linux/org.apertrust.Apertrust.mime.xml \
            "$tree/share/mime/packages/org.apertrust.Apertrust.xml"
        cp -R crates/app/assets/icons/png/linux/hicolor "$tree/share/icons/"
        licences "$tree/share/doc/$package"
        tar -C "$stage/tarball" -czf "$out/$name.tar.gz" "$name"
        ;;
    windows)
        # A portable zip: the two executables and the licences in one folder, named for the
        # architecture as the installer is.
        arch=${triple%%-*}
        [[ "$arch" = x86_64 ]] && arch=x64
        name=${package}_${version}_$arch-portable
        tree=$stage/zip/$product
        rm -rf "$stage/zip"
        mkdir -p "$tree"
        cp "$bin_dir/$package.exe" "$bin_dir/$package-cli.exe" "$tree/"
        licences "$tree"
        powershell -NoProfile -Command \
            "Compress-Archive -Path '$(cygpath -w "$tree")' -DestinationPath '$(cygpath -w "$out/$name.zip")' -Force"
        ;;
    esac

    echo "packages in $out:"
    ls -1 "$out"
}

for triple in $triples; do
    package "$triple"
done
