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
# Windows the NSIS installer and a portable zip, on Linux the AppImage, the .deb, the .rpm
# (from packaging/linux/rpm.toml with cargo-generate-rpm, as cargo-packager makes none)
# and a tarball, each with the app, the CLI and the licences, into
# target/package/<target>/out.
#
#   tools/package.sh [--target TRIPLE] [--finder-layout] [--nightly]
#
# --nightly packages Apertrust Nightly, a development build that installs and keeps its
# settings beside Apertrust, as the nightly builds are (docs/packaging.md): the binaries
# are built with APERTRUST_NIGHTLY=1 and named apertrust-nightly and apertrust-nightly-cli,
# and the packages take the name, the id and the files packaging/nightly-identity.txt
# gives.
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
# Each package is named <package>-<version>-<os>-<arch>[-<kind>].<ext>, lower case but .AppImage,
# with each platform's own words for its architecture, and on Linux each format's, Debian's
# in the .deb (docs/packaging.md): apertrust-0.2.0+615-macos-arm64.dmg,
# apertrust-nightly-0.2.0+615-windows-x64-setup.exe, apertrust-0.2.0+615-linux-amd64.deb,
# apertrust-0.2.0+615-linux-x86_64.rpm.
#
# The version is the build's, as `--version` prints it: from git, or APERTRUST_VERSION
# when a packager sets it (decision 44).
set -euo pipefail

cd "$(dirname "$0")/.."

# The cargo-packager this script and the config are written for, and on Linux the
# cargo-generate-rpm.
PACKAGER=0.11.8
GENERATE_RPM=0.21.1
# The oldest macOS the bundle runs on, as packaging/macos/Info.plist says, for both
# architectures' builds.
export MACOSX_DEPLOYMENT_TARGET=11.0

finder_layout=false
nightly=false
only=
while [[ $# -gt 0 ]]; do
    case "$1" in
    --finder-layout) finder_layout=true ;;
    --nightly) nightly=true ;;
    --target)
        only=${2:?--target needs a target triple}
        shift
        ;;
    *)
        echo "usage: tools/package.sh [--target TRIPLE] [--finder-layout] [--nightly]" >&2
        exit 2
        ;;
    esac
    shift
done

# The package's identity: its files' and binaries' name, the name it shows, its id.
if [[ "$nightly" = true ]]; then
    # Read by apertrust-base's build script: the binaries name themselves Apertrust Nightly
    # and keep their settings in its folder.
    export APERTRUST_NIGHTLY=1
    package=apertrust-nightly
    product="Apertrust Nightly"
    id=org.apertrust.Apertrust.Nightly
else
    package=apertrust
    product=Apertrust
    id=org.apertrust.Apertrust
fi

# The file $1 with the package's identity: as it is, or for --nightly with each
# `FROM => TO` of packaging/nightly-identity.txt replaced wherever it occurs.
with_identity() {
    if [[ "$nightly" = false ]]; then
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
    done <packaging/nightly-identity.txt
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

if [[ "$platform" = linux ]]; then
    installed=$(cargo generate-rpm --version 2>/dev/null | awk '{ print $2 }') || true
    if [[ "$installed" != "$GENERATE_RPM" ]]; then
        echo "error: cargo-generate-rpm $GENERATE_RPM is needed (found: ${installed:-none}):" >&2
        echo "  cargo install --locked cargo-generate-rpm --version $GENERATE_RPM" >&2
        exit 1
    fi
fi

build() {
    cargo build --release --locked -p apertrust -p apertrust-cli "$@"
}

# The target $1's platform and architecture as its packages name them, in the platform's
# own words: macos-arm64 or macos-x86_64, windows-x64 or windows-arm64, linux-x86_64 or
# linux-aarch64 (the AppImage's, the tarball's and RPM's words); with deb as $2, Debian's,
# linux-amd64 or linux-arm64, which the .deb's control file names too.
os_arch() {
    case "$1:${2:-}" in
    aarch64-apple-darwin:) echo macos-arm64 ;;
    x86_64-apple-darwin:) echo macos-x86_64 ;;
    x86_64-pc-windows-*:) echo windows-x64 ;;
    aarch64-pc-windows-*:) echo windows-arm64 ;;
    x86_64-*-linux-*:) echo linux-x86_64 ;;
    aarch64-*-linux-*:) echo linux-aarch64 ;;
    x86_64-*-linux-*:deb) echo linux-amd64 ;;
    aarch64-*-linux-*:deb) echo linux-arm64 ;;
    *)
        echo "error: no package name for the target $1${2:+ in a .$2}" >&2
        return 1
        ;;
    esac
}

# Gives the one package cargo-packager wrote into $out ending in $1 the name $2:
# cargo-packager names its packages its own way, with no template to change it.
rename_package() {
    local found=("$out"/*"$1")
    if [[ ${#found[@]} -ne 1 ]] || [[ ! -f "${found[0]}" ]]; then
        echo "error: one *$1 expected in $out, found: ${found[*]##*/}" >&2
        exit 1
    fi
    mv "${found[0]}" "$out/$2"
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
# so the volume must be the product's, Apertrust or Apertrust Nightly, with no version,
# each with its own layout, which also places the app by its bundle's name.
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
    [[ "$nightly" = true ]] && layout="layout-nightly.DS_Store"
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
    target_name=$(os_arch "$triple")
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
    # be "Apertrust Nightly CLI".
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
    # Every package's name but its kind and extension.
    base=$package-$version-$target_name

    stage=$target_dir/package/$triple
    out=$stage/out
    rm -rf "$out"
    mkdir -p "$out"
    # Apertrust Nightly's binaries under their own names, so that its installer and
    # tarball never take Apertrust's place (Windows registers Open With by the
    # executable's name).
    if [[ "$nightly" = true ]]; then
        mkdir -p "$stage/bin"
        cp "$bin_dir/apertrust$exe" "$stage/bin/$package$exe"
        cp "$bin_dir/apertrust-cli$exe" "$stage/bin/$package-cli$exe"
        bin_dir=$stage/bin
    fi
    # The licences, laid out as Packager.toml's resources are.
    licences() {
        mkdir -p "$1/licences"
        cp LICENSE CREDITS.md "$1/"
        cp crates/ui/assets/fonts/OFL.txt crates/ui/assets/icons/LICENSE-Lucide "$1/licences/"
    }

    # The Linux desktop entry and metadata, named after the id, which the AppImage of
    # Apertrust Nightly takes from here.
    with_identity packaging/linux/org.apertrust.Apertrust.desktop >"$stage/$id.desktop"
    with_identity packaging/linux/org.apertrust.Apertrust.metainfo.xml \
        >"$stage/$id.metainfo.xml"

    # On Linux, the tarball's tree, laid out as a prefix before cargo-packager runs, since
    # the .deb's and the .rpm's /usr are its bin and share: the desktop file, the metadata,
    # the MIME types, the icons and the licences, each named after the package's id or
    # name, so that Apertrust Nightly's packages have no file in common with Apertrust's.
    prefix=
    glibc=
    if [[ "$platform" = linux ]]; then
        prefix=$stage/tarball/$base
        rm -rf "$stage/tarball"
        mkdir -p "$prefix/bin" "$prefix/share/applications" "$prefix/share/metainfo" \
            "$prefix/share/mime/packages" "$prefix/share/icons"
        cp "$bin_dir/$package" "$bin_dir/$package-cli" "$prefix/bin/"
        cp "$stage/$id.desktop" "$prefix/share/applications/"
        cp "$stage/$id.metainfo.xml" "$prefix/share/metainfo/"
        cp packaging/linux/org.apertrust.Apertrust.mime.xml "$prefix/share/mime/packages/$id.xml"
        cp -R crates/app/assets/icons/png/linux/hicolor "$prefix/share/icons/"
        if [[ "$nightly" = true ]]; then
            for icon in "$prefix"/share/icons/hicolor/*/apps/org.apertrust.Apertrust.*; do
                mv "$icon" "${icon%/*}/$id.${icon##*.}"
            done
        fi
        licences "$prefix/share/doc/$package"
        # The newest glibc the binaries need, which the .deb depends on; the .rpm's
        # requirements cargo-generate-rpm reads from the binaries itself.
        glibc=$(objdump -T "$bin_dir/$package" "$bin_dir/$package-cli" |
            grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1) || true
        glibc=${glibc#GLIBC_}
        if [[ -z "$glibc" ]]; then
            echo "error: no glibc version in the binaries' symbols" >&2
            exit 1
        fi
    fi

    fill() {
        with_identity "$1" | sed -e "s|@ROOT@|$root|g" -e "s|@VERSION@|$version|g" \
            -e "s|@SHORT_VERSION@|$short|g" -e "s|@BUILD@|$build_number|g" \
            -e "s|@BINARIES@|$bin_dir|g" -e "s|@INFO_PLIST@|$stage/Info.plist|g" \
            -e "s|@STAGE@|$stage|g" -e "s|@PREFIX@|$prefix|g" -e "s|@GLIBC@|$glibc|g" >"$2"
    }
    fill packaging/Packager.toml "$stage/Packager.toml"
    fill packaging/macos/Info.plist "$stage/Info.plist"

    case "$platform" in
    macos) formats=app,dmg ;;
    windows) formats=nsis ;;
    linux) formats=appimage,deb ;;
    esac
    # create-dmg drives Finder to lay out the image's window unless CI is set.
    if [[ "$platform" = macos ]] && [[ "$finder_layout" = false ]]; then
        export CI=true
    fi
    cargo packager --config "$stage/Packager.toml" --formats "$formats" --target "$triple" \
        --out-dir "$out"

    case "$platform" in
    macos)
        codesign --verify --deep --strict "$out/$product.app"
        for dmg in "$out"/*.dmg; do
            dress_dmg "$dmg" "$stage"
            codesign --verify --strict "$dmg"
        done
        rename_package .dmg "$base.dmg"
        ;;
    linux)
        rename_package .AppImage "$base.AppImage"
        deb_name=$(os_arch "$triple" deb)
        rename_package .deb "$package-$version-$deb_name.deb"
        # The .rpm, from packaging/linux/rpm.toml filled in as the Cargo.toml of a folder of
        # its own, as cargo-generate-rpm reads a Cargo manifest, named here directly. It
        # finds the libraries the binaries link and their glibc itself (builtin: with ldd,
        # whether or not rpm is installed), the config naming those loaded at run time.
        rm -rf "$stage/rpm"
        mkdir -p "$stage/rpm"
        fill packaging/linux/rpm.toml "$stage/rpm/Cargo.toml"
        cargo generate-rpm -p "$stage/rpm" --target "$triple" --auto-req builtin \
            -o "$out/$base.rpm"
        # The tarball, for ~/.local or /usr/local.
        tar -C "$stage/tarball" -czf "$out/$base.tar.gz" "$base"
        ;;
    windows)
        rename_package -setup.exe "$base-setup.exe"
        # A portable zip: the two executables and the licences in one folder.
        name=$base-portable
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
