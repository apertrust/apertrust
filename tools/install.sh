#!/bin/bash
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

# Installs or updates Apertrust's latest release, or with --nightly Apertrust Nightly, on
# macOS or Linux from GitHub (docs/packaging.md, Installing with the script):
#
#   curl -q --proto '=https' --proto-redir '=https' -fsSL https://raw.githubusercontent.com/apertrust/apertrust/main/tools/install.sh | bash
#   curl -q --proto '=https' --proto-redir '=https' -fsSL https://raw.githubusercontent.com/apertrust/apertrust/main/tools/install.sh | bash -s -- --nightly
#
# A pipe reports bash's status, not curl's, so a download that fails (a 404, no network)
# looks like success; for a script, or an unattended run, download first, then run:
#
#   curl -q --proto '=https' --proto-redir '=https' -fsSL URL -o install.sh && bash install.sh --nightly
#
# It chooses the package for this system from the release's SHA256SUMS and checks it
# against it. On macOS, the disk image for this Mac's chip, copied to /Applications; on
# Linux, the .deb with apt, the .rpm with dnf or zypper, or else the tarball: the programs
# into ~/.local/bin, the menu entry, icons and file types into $XDG_DATA_HOME or ~/.local/share.
# The packages are signed ad hoc, not yet with a developer's certificate, nor notarised:
# running this script trusts them, and on macOS it removes quarantine from this package
# and app only.
#
# It asks nothing: sudo alone asks for a password, where one is needed, on the terminal.
# Everything is in functions, run by the last line with no stdin, so that bash has read
# the whole script before anything runs, as it reads it from curl, and no command reads
# the rest of it or waits for an answer.

set -euo pipefail

usage="usage: install.sh [--nightly]"

# The channel's names, set by choose_channel.
NAME=""     # Apertrust, or Apertrust Nightly
CHANNEL=""  # Release, or Nightly
PACKAGE=""  # apertrust or apertrust-nightly: each file's prefix and the app's executable
RELEASE=""  # where the release's files are downloaded from

TMP=""
PKG_HASH=""  # the package chosen from SHA256SUMS
PKG_NAME=""
VERSION=""
FILE=""  # the package downloaded and checked, in $TMP
UPDATED_FROM=""  # the version installed before, when one was

# The terminal's look, set by setup_ui, and the step running.
BOLD="" DIM="" RED="" GREEN="" YELLOW="" ORANGE="" RESET="" CLEAR=""
MARK_RUN="[..]" MARK_OK="[ok]" MARK_FAIL="[fail]" MARK_WARN="[warn]" ARROW="->"
STEP=""
FAILED=false
CHANGE_NOTE=""  # what a failure from here on may have changed

# macOS's state, for the cleanup.
DEST="/Applications"
STAGE=""
INSTALLED=false  # the new app is in place, so the previous one may go
KEEP_STAGE=false  # the stage holds what a stopped install left, to keep
REPLACE_ARCH=""  # set when the installed app lacks this Mac's architecture
NEED_SUDO=false
MOUNTED=false

# Colour and Unicode marks on a terminal, unless NO_COLOR is set or the terminal is dumb.
setup_ui() {
    if [[ -t 1 && -z "${NO_COLOR:-}" && "${TERM:-dumb}" != dumb ]]; then
        BOLD=$'\033[1m' DIM=$'\033[2m' RED=$'\033[31m' GREEN=$'\033[32m'
        YELLOW=$'\033[33m' RESET=$'\033[0m' CLEAR=$'\r\033[K'
        MARK_RUN="›" MARK_OK="✓" MARK_FAIL="✗" MARK_WARN="!" ARROW="→"
        # The logo's orange, #D66A20, in 24-bit colour where the terminal says it has it.
        case "${COLORTERM:-}" in
        truecolor | 24bit) ORANGE=$'\033[38;2;214;106;32m' ;;
        *) ORANGE=$'\033[38;5;166m' ;;
        esac
    fi
}

# A banner: the logo's orange square, the name, the installer and its channel, in a box
# as wide as its text, which the plain one measures, with two spaces each side.
header() {
    local plain="# Apertrust   installer - $CHANNEL" line="" i
    for ((i = 0; i < ${#plain} + 4; i++)); do
        line="$line-"
    done
    if [[ -z "$CLEAR" ]]; then
        printf '+%s+\n|  %s  |\n+%s+\n\n' "$line" "$plain" "$line"
        return
    fi
    line="${line//-/─}"
    printf '%s╭%s╮%s\n' "$DIM" "$line" "$RESET"
    printf '%s│%s  %s■%s %sApertrust%s   %sinstaller · %s%s  %s│%s\n' "$DIM" "$RESET" \
        "$ORANGE" "$RESET" "$BOLD" "$RESET" "$DIM" "$CHANNEL" "$RESET" "$DIM" "$RESET"
    printf '%s╰%s╯%s\n\n' "$DIM" "$line" "$RESET"
}

# A step that prints nothing: its line is shown on a terminal until ok replaces it.
begin() {
    STEP="$1"
    if [[ -n "$CLEAR" ]]; then
        printf '  %s%s%s %s' "$DIM" "$MARK_RUN" "$RESET" "$1"
    fi
}

# A step whose commands print, below its line.
begin_loud() {
    STEP="$1"
    printf '  %s%s%s %s\n' "$DIM" "$MARK_RUN" "$RESET" "$1"
}

ok() {
    printf '%s  %s%s%s %s\n' "$CLEAR" "$GREEN" "$MARK_OK" "$RESET" "${1:-$STEP}"
    STEP=""
}

warn() {
    local line
    printf '  %s%s%s %s\n' "$YELLOW" "$MARK_WARN" "$RESET" "$1"
    shift
    for line in "$@"; do
        printf '    %s\n' "$line"
    done
}

# Stops with the step that failed, or the first line, and the reason's lines, on stderr.
fail() {
    local line
    printf '%s' "$CLEAR"
    {
        if [[ -n "$STEP" ]]; then
            printf '  %s%s%s %s\n' "$RED" "$MARK_FAIL" "$RESET" "$STEP"
        else
            printf '  %s%s%s %s\n' "$RED" "$MARK_FAIL" "$RESET" "$1"
            shift
        fi
        for line in "$@"; do
            printf '    %s\n' "$line"
        done
    } >&2
    STEP=""
    FAILED=true
    exit 1
}

choose_channel() {
    case "$#:${1:-}" in
    0:)
        NAME="Apertrust"
        CHANNEL="Release"
        PACKAGE="apertrust"
        RELEASE="https://github.com/apertrust/apertrust/releases/latest/download"
        ;;
    1:--nightly)
        NAME="Apertrust Nightly"
        CHANNEL="Nightly"
        PACKAGE="apertrust-nightly"
        RELEASE="https://github.com/apertrust/apertrust/releases/download/nightly"
        ;;
    *) fail "$usage" ;;
    esac
}

# Refuses while the app runs. A path in another program's arguments, a folder named
# apertrust given to an editor, is not the app.
require_closed() {
    local status=0
    if [[ "$(uname -s)" = Darwin ]]; then
        running_macos || status=$?
    else
        running_linux || status=$?
    fi
    case "$status" in
    0) fail "$NAME is running." "Quit it and run this script again." ;;
    1) ;;
    *) fail "Could not check whether $NAME is running." ;;
    esac
}

# Succeeds when the app runs on macOS, as pgrep: a process whose executable, which ps
# gives as a full path without the arguments, spaces and all, is $PACKAGE in any folder.
running_macos() {
    local names
    names="$(ps -axo comm=)" || return 2
    grep -qE "(^|/)$PACKAGE\$" <<<"$names"
}

# Succeeds when the app runs on Linux, as pgrep: the processes whose name is the
# executable's, cut to Linux's 15 characters, whose executable is $PACKAGE in any folder,
# or replaced since it started; one of another user's, which cannot be read, counts.
running_linux() {
    local pids pid exe
    pids="$(pgrep -x "${PACKAGE:0:15}")" || return
    for pid in $pids; do
        exe="$(readlink "/proc/$pid/exe" 2>/dev/null)" || return 0
        exe="${exe% (deleted)}"
        [[ "${exe##*/}" != "$PACKAGE" ]] || return 0
    done
    return 1
}

# -q first, so that the user's .curlrc cannot weaken the checks (insecure, a proxy
# over http, other protocols): the downloads are https alone, redirects included.
fetch() {
    # A stalled server or proxy ends the run rather than holding it: 30 s to connect, and
    # a transfer under 1 KB/s for a minute is abandoned; a large one still moving never is.
    curl -q -fL --proto '=https' --proto-redir '=https' \
        --connect-timeout 30 --speed-limit 1024 --speed-time 60 "$@"
}

# Sets PKG_HASH and PKG_NAME from the one line of the SHA256SUMS $1 naming
# $PACKAGE-<version>$2; the version starts with a digit, so apertrust- never matches
# apertrust-nightly-'s files.
choose_package() {
    local hash name count=0
    while read -r hash name; do
        name="${name%$'\r'}"
        name="${name#\*}"
        [[ "$name" == "$PACKAGE"-[0-9]*"$2" ]] || continue
        [[ "$hash" =~ ^[0-9a-f]{64}$ && "$name" =~ ^[A-Za-z0-9._-]+$ ]] ||
            fail "The release's list of packages is damaged. Try again later."
        PKG_HASH="$hash"
        PKG_NAME="$name"
        count=$((count + 1))
    done <"$1"
    case "$count" in
    0) fail "$NAME has no package for this computer ($PACKAGE-<version>$2)." ;;
    1) ;;
    *) fail "$NAME has more than one package for this computer. Try again later." ;;
    esac
}

# Finds the package named $PACKAGE-<version>$1 and says whether it installs or updates
# the version $2 installed, stopping when it is that one; then downloads it into $TMP, as
# FILE, and checks it against the release's SHA256SUMS. Versions are compared as they
# are, never ordered: the channel's current build installs, whichever is newer.
download() {
    local code status=0 progress=-sS
    begin "Finding $NAME's package"
    code="$(fetch -s -o "$TMP/SHA256SUMS" -w '%{http_code}' "$RELEASE/SHA256SUMS")" ||
        status=$?
    if [[ "$status" -ne 0 ]]; then
        if [[ "$code" != 404 ]]; then
            fail "Could not reach GitHub (curl's exit status $status)." \
                "Check your connection and try again."
        elif [[ "$PACKAGE" = apertrust ]]; then
            fail "No Apertrust release with a SHA256SUMS is published; install the nightly" \
                "with --nightly."
        else
            fail "The nightly isn't available right now. Try again in a few minutes."
        fi
    fi
    choose_package "$TMP/SHA256SUMS" "$1"
    VERSION="${PKG_NAME#"$PACKAGE"-}"
    VERSION="${VERSION%"$1"}"
    # The package's version, X.Y.Z+N, which the release publishes with a . for the +.
    if [[ "$VERSION" =~ ^([0-9]+\.[0-9]+\.[0-9]+)\.([0-9]+)$ ]]; then
        VERSION="${BASH_REMATCH[1]}+${BASH_REMATCH[2]}"
    fi
    if [[ -z "$2" ]]; then
        ok "Installing $NAME $VERSION"
    elif [[ "$2" = "$VERSION" && -n "$REPLACE_ARCH" ]]; then
        ok "Replacing $NAME $VERSION, which has no $REPLACE_ARCH build, with its $REPLACE_ARCH build"
    elif [[ "$2" = "$VERSION" ]]; then
        ok "$NAME $VERSION is already installed."
        if [[ "$1" = *.tar.gz ]]; then
            # The record stays, so that a later run refreshes them again.
            CHANGE_NOTE="Only the menu entry or the file types' caches may have changed."
            refresh_desktop
        fi
        exit 0
    else
        UPDATED_FROM="$2"
        ok "Updating $NAME $2 $ARROW $VERSION"
    fi
    # Before the download, and again before the install.
    require_closed

    begin_loud "Downloading $PKG_NAME"
    # curl's progress bar on a terminal.
    [[ ! -t 2 ]] || progress=--progress-bar
    status=0
    fetch "$progress" -o "$TMP/$PKG_NAME" "$RELEASE/$PKG_NAME" || status=$?
    [[ "$status" -eq 0 ]] || fail "The download failed (curl's exit status $status). Try again."
    ok "Downloaded $PKG_NAME"

    begin "Verifying SHA-256"
    if [[ "$(uname -s)" = Darwin ]]; then
        (cd "$TMP" && printf '%s  %s\n' "$PKG_HASH" "$PKG_NAME" | shasum -a 256 -c - \
            >/dev/null 2>&1) || fail "The download does not match its checksum. Try again."
    else
        (cd "$TMP" && printf '%s  %s\n' "$PKG_HASH" "$PKG_NAME" | sha256sum -c - \
            >/dev/null 2>&1) || fail "The download does not match its checksum. Try again."
    fi
    ok "Verified SHA-256"
    FILE="$TMP/$PKG_NAME"
}

# Runs a command as root: itself when root already, else through sudo.
as_root() {
    if [[ "$(id -u)" -eq 0 ]]; then
        "$@"
    else
        sudo "$@"
    fi
}

# On macOS, with sudo only where /Applications is not writable.
run_install() {
    if [[ "$NEED_SUDO" = true ]]; then
        sudo "$@"
    else
        "$@"
    fi
}

# The version installed, in the package's form, X.Y.Z+N, or nothing where none is
# installed or it cannot be read. On macOS from the app's Info.plist.
installed_macos() {
    local plist="$DEST/$NAME.app/Contents/Info.plist" short build
    short="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$plist" \
        2>/dev/null)" || return 0
    build="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$plist" 2>/dev/null)" ||
        return 0
    # tools/package.sh gives the bundle X.Y.Z and the commit count, the package's version
    # X.Y.Z+N; or, with a packager's APERTRUST_VERSION, X.Y.Z for both and the package's.
    if [[ "$build" = "$short" ]]; then
        echo "$short"
    else
        echo "$short+$build"
    fi
}

# Whether the installed app's executable has the architecture $1 (arm64 or x86_64), alone
# or in a universal binary, so that an Intel-only build is not taken for this Apple silicon
# Mac's: 0 when it has it, 1 when it is a Mach-O executable without it, 2 when it cannot be
# told. Read with /usr/bin/file, macOS's own, which another file earlier on the PATH would
# word otherwise; never lipo, which may be the developer tools' stand-in and offer to
# install them instead of answering.
installed_macos_has_arch() {
    local exe="$DEST/$NAME.app/Contents/MacOS/$PACKAGE" kinds
    [[ -f "$exe" ]] || return 2
    kinds="$(/usr/bin/file -b "$exe" 2>/dev/null)" || return 2
    [[ "$kinds" = *Mach-O* ]] || return 2
    if grep -q -E "executable $1(\]|$)" <<<"$kinds"; then
        return 0
    fi
    return 1
}

# The .deb's version, where dpkg has it installed: its error and state words "ok
# installed", whatever the selection (install, hold); never unpacked or half-configured.
installed_deb() {
    local status want error state version
    status="$(dpkg-query -W -f='${Status} ${Version}' "$PACKAGE" 2>/dev/null)" || return 0
    read -r want error state version <<<"$status"
    if [[ -n "$want" && "$error" = ok && "$state" = installed && -n "$version" ]]; then
        echo "$version"
    fi
}

# The .rpm's version, where rpm has it installed.
installed_rpm() {
    local version
    version="$(rpm -q --qf '%{VERSION}' "$PACKAGE" 2>/dev/null)" || return 0
    echo "$version"
}

# The tarball's package version, from the record this script writes once it has unpacked
# the tarball whole, when the binary in ~/.local still prints the --version the record
# holds. The binary's own version counts commits from the last release tag, the package's
# all of them, so the one cannot be read from the other.
installed_tarball() {
    local record version printed
    [[ -x "$HOME/.local/bin/$PACKAGE" ]] || return 0
    record="$(tarball_record)"
    [[ -f "$record" ]] || return 0
    version="$(sed -n 1p "$record")"
    printed="$(binary_version)" || return 0
    if [[ -n "$version" && -n "$printed" && "$printed" = "$(sed -n 2p "$record")" ]]; then
        echo "$version"
    fi
}

# The first line of the binary's --version in ~/.local.
binary_version() {
    "$HOME/.local/bin/$PACKAGE" --version 2>/dev/null | sed -n 1p
}

# The record of the tarball this script unpacked whole: the package's version, then the
# binary's --version.
tarball_record() {
    echo "$(data_dir)/doc/$PACKAGE/installed-version"
}

# Where the tarball's share/ goes: $XDG_DATA_HOME where it is an absolute path, as the
# specification has it, else ~/.local/share.
data_dir() {
    if [[ "${XDG_DATA_HOME:-}" = /* ]]; then
        echo "${XDG_DATA_HOME%/}"
    else
        echo "$HOME/.local/share"
    fi
}

# Shows the summary: what was installed, where, and how to open it.
summary() {
    if [[ -n "$UPDATED_FROM" ]]; then
        printf '\n  %s%s %s is installed, updated from %s.%s\n' "$BOLD" "$NAME" "$VERSION" \
            "$UPDATED_FROM" "$RESET"
    else
        printf '\n  %s%s %s is installed.%s\n' "$BOLD" "$NAME" "$VERSION" "$RESET"
    fi
    printf '    Where  %s\n' "$1"
    printf '    Open   %s\n' "$2"
    printf '\n'
}

# Warns where the command $PACKAGE on the PATH is another copy than $1, the one
# installed, an older tarball's in ~/.local/bin, say; nothing is removed.
check_command() {
    local found
    found="$(command -v "$PACKAGE" 2>/dev/null)" || return 0
    [[ "$found" -ef "$1" ]] ||
        warn "$PACKAGE on your PATH is another copy, $found;" "the one installed is $1"
}

install_macos() {
    local app="$NAME.app" machine arch chip installed mnt="$TMP/mnt"
    machine="$(uname -m)"
    case "$machine" in
    arm64) arch=arm64 chip="Apple silicon" ;;
    x86_64)
        # An x86_64 shell under Rosetta on Apple silicon installs arm64's.
        if [[ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]]; then
            arch=arm64 chip="Apple silicon, this shell under Rosetta"
        else
            arch=x86_64 chip="Intel"
        fi
        ;;
    *) fail "Unsupported architecture: $machine." ;;
    esac
    ok "macOS on $arch ($chip): the disk image"
    installed="$(installed_macos)"
    if [[ -n "$installed" ]]; then
        # Replaced only when its executable is known to lack this architecture; one that
        # cannot be read is left to the version check.
        local has=0
        installed_macos_has_arch "$arch" || has=$?
        [[ "$has" -ne 1 ]] || REPLACE_ARCH="$arch"
    fi
    download "-macos-$arch.dmg" "$installed"

    begin "Removing the disk image's quarantine (not notarised)"
    if xattr -p com.apple.quarantine "$FILE" >/dev/null 2>&1; then
        xattr -d com.apple.quarantine "$FILE"
    fi
    ok "Removed the disk image's quarantine (not notarised)"

    begin "Mounting the disk image, read-only"
    mkdir -p "$mnt"
    # Set first, so that a signal during the attach still unmounts the image.
    MOUNTED=true
    hdiutil attach "$FILE" -nobrowse -readonly -noautoopen -mountpoint "$mnt" -quiet ||
        fail "Could not mount the disk image."
    ok "Mounted the disk image, read-only"

    # Ad-hoc signatures can be verified, but spctl would reject an unnotarised build.
    begin "Verifying the app's signature"
    codesign --verify --deep --strict "$mnt/$app" 2>/dev/null ||
        fail "The app's signature does not verify."
    ok "Verified the app's signature"

    if [[ ! -w "$DEST" ]]; then
        NEED_SUDO=true
        warn "$DEST is not writable: sudo asks for an administrator's password"
    fi

    # Copy and verify before replacing an installed version. Keep the previous app
    # beside it until the new one is in place, so a failed copy leaves it intact.
    begin "Copying the app beside $DEST/$app"
    STAGE="$(run_install mktemp -d "$DEST/.$PACKAGE.XXXXXX")"
    # Keep verification and rollback checks possible when sudo owns the directory.
    run_install chmod 755 "$STAGE"
    run_install ditto "$mnt/$app" "$STAGE/$app"
    codesign --verify --deep --strict "$STAGE/$app" 2>/dev/null ||
        fail "The copied app's signature does not verify."
    run_install xattr -dr com.apple.quarantine "$STAGE/$app"
    ok "Copied the app and verified its signature"

    begin "Replacing $DEST/$app"
    require_closed
    if [[ -e "$DEST/$app" || -L "$DEST/$app" ]]; then
        CHANGE_NOTE="The previous app was moved aside; see above whether it was restored."
        run_install mv "$DEST/$app" "$STAGE/previous.app"
    fi
    # mv would put the new app inside a folder of that name, and the cleanup would
    # not know the previous app from it: stop, keeping the stage, if one appeared.
    if [[ -e "$DEST/$app" || -L "$DEST/$app" ]]; then
        KEEP_STAGE=true
        CHANGE_NOTE="$DEST/$app was not replaced; the new app is kept at $STAGE/$app."
        if [[ -e "$STAGE/previous.app" || -L "$STAGE/previous.app" ]]; then
            CHANGE_NOTE="$CHANGE_NOTE The previous app is at $STAGE/previous.app."
        fi
        fail "$DEST/$app appeared while it was being replaced." \
            "Remove it and run this script again."
    fi
    run_install mv "$STAGE/$app" "$DEST/$app"
    if [[ ! -e "$DEST/$app/Contents/Info.plist" || -e "$STAGE/$app" ]]; then
        CHANGE_NOTE="$DEST/$app is not the new app: look there, and inside it."
        fail "The new app is not in place at $DEST/$app."
    fi
    INSTALLED=true
    # Only a cleanup that fails can still stop the script: the install stands.
    CHANGE_NOTE="$NAME $VERSION is installed in $DEST."
    ok "Installed $DEST/$app"

    summary "$DEST/$app" "open -a \"$NAME\""
}

# Restores the previous app if installation stopped between the two renames, removes
# the staging folder, with the previous app in it only once the new app is in place, and
# unmounts the disk image: nothing to do but on macOS. Returns 2 when the disk image is
# still mounted, 1 when anything else could not be done.
cleanup_macos() {
    local app="$NAME.app" status=0
    if [[ -n "$STAGE" ]]; then
        if [[ "$INSTALLED" != true ]] &&
            [[ -e "$STAGE/previous.app" || -L "$STAGE/previous.app" ]] &&
            [[ ! -e "$DEST/$app" && ! -L "$DEST/$app" ]]; then
            if run_install mv "$STAGE/previous.app" "$DEST/$app"; then
                CHANGE_NOTE="The previous app was put back; nothing was changed."
            else
                CHANGE_NOTE="The previous app could not be put back: it is at $STAGE/previous.app"
                status=1
            fi
        fi
        if [[ "$KEEP_STAGE" = true ]]; then
            warn "Kept $STAGE, as it holds the new app" >&2
            status=1
        elif [[ "$INSTALLED" = true ]] ||
            [[ ! -e "$STAGE/previous.app" && ! -L "$STAGE/previous.app" ]]; then
            # A previous app with files of root's, installed by another tool, may not
            # go without sudo: the install stands, and the leftover is named.
            if ! run_install rm -rf "$STAGE" 2>/dev/null; then
                warn "Could not remove $STAGE, the previous app's files;" \
                    "remove it with: sudo rm -rf \"$STAGE\"" >&2
            fi
        else
            # The previous app is not back and the new one is not in place: keep both.
            warn "The previous app is kept in $STAGE" >&2
            status=1
        fi
    fi
    # Mounted when the mount point is on another device than its folder.
    if [[ "$MOUNTED" = true ]] &&
        [[ "$(stat -f %d "$TMP/mnt" 2>/dev/null)" != "$(stat -f %d "$TMP")" ]] &&
        ! hdiutil detach "$TMP/mnt" -quiet; then
        warn "Could not unmount $TMP/mnt; its files stay in $TMP" >&2
        return 2
    fi
    return "$status"
}

# A Linux architecture this script has no package for.
unsupported_linux_arch() {
    fail "Unsupported architecture: $1 (the packages are for 64-bit x86 and Arm)."
}

# The .deb with apt, the .rpm with dnf or zypper, by the package manager present, or the
# tarball into ~/.local, each architecture in its format's own words.
install_linux() {
    local arch tool sudo=""
    [[ "$(id -u)" -eq 0 ]] || sudo=", with sudo"
    if command -v apt-get >/dev/null; then
        arch="$(dpkg --print-architecture)"
        case "$arch" in amd64 | arm64) ;; *) unsupported_linux_arch "$arch" ;; esac
        ok "Linux on $arch, with apt-get (Debian, Ubuntu): the .deb"
        download "-linux-$arch.deb" "$(installed_deb)"
        # Readable by apt's own user, which reads the package without root.
        chmod 755 "$TMP"
        chmod 644 "$FILE"
        tool=apt
    elif command -v dnf >/dev/null || command -v zypper >/dev/null; then
        arch="$(rpm --eval '%{_arch}')"
        case "$arch" in x86_64 | aarch64) ;; *) unsupported_linux_arch "$arch" ;; esac
        if command -v dnf >/dev/null; then
            ok "Linux on $arch, with dnf (Fedora, RHEL): the .rpm"
            tool=dnf
        else
            ok "Linux on $arch, with zypper (openSUSE): the .rpm"
            tool=zypper
        fi
        download "-linux-$arch.rpm" "$(installed_rpm)"
    else
        install_tarball
        return
    fi

    begin_loud "Installing with $tool$sudo"
    require_closed
    CHANGE_NOTE="$tool stopped: its output above says whether it changed anything."
    case "$tool" in
    # Each takes the channel's build over a newer one installed, too. apt-get with the
    # frontend set across sudo (which resets the environment) and dpkg told to keep the
    # user's changed configuration files, so that nothing asks a question.
    apt)
        as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y --allow-downgrades \
            -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold "$FILE"
        ;;
    dnf)
        as_root dnf install -y "$FILE"
        # dnf install leaves a newer version in place; dnf downgrade replaces it.
        [[ "$(installed_rpm)" = "$VERSION" ]] || as_root dnf downgrade -y "$FILE"
        ;;
    zypper)
        as_root zypper --non-interactive install --allow-unsigned-rpm --oldpackage "$FILE"
        ;;
    esac
    CHANGE_NOTE=""
    ok "Installed with $tool"
    summary "/usr/bin/$PACKAGE" "the applications menu, or /usr/bin/$PACKAGE"
    check_command "/usr/bin/$PACKAGE"
}

# Refuses where a directory below bin/ or share/ that the tarball writes is a symlink,
# which tar would replace with a real directory: $1 is the tarball's top folder, $2
# where its bin/ goes, $3 where its share/ goes, both already resolved, as a symlink to
# the folder itself, ~/.local/bin often, is fine. (GNU tar's --keep-directory-symlink
# does not do it: it refuses a link that leaves the folder it unpacks into.)
refuse_directory_symlinks() {
    local entry rel dir
    while read -r entry; do
        [[ "$entry" == "$1"/*/ ]] || continue
        rel="${entry#"$1"/}"
        rel="${rel%/}"
        case "$rel" in
        bin/*) dir="$2/${rel#bin/}" ;;
        share/*) dir="$3/${rel#share/}" ;;
        *) continue ;;
        esac
        [[ ! -L "$dir" ]] ||
            fail "$dir is a symbolic link to a directory, which unpacking would replace." \
                "Remove the link, or unpack the tarball by hand where it points."
    done < <(tar -tzf "$FILE")
}

# The tarball, whose one folder at the top is a prefix (bin/, share/): its bin/ into
# ~/.local/bin and its share/ into the data directory, ~/.local/share unless
# $XDG_DATA_HOME says otherwise, as README.md's Installation does it. A symlink for one
# of those folders, ~/.local/bin often, is followed, never replaced.
install_tarball() {
    local arch record printed top bindir datadir
    # shellcheck disable=SC2088  # the words for the person, not a path
    local where="~/.local"
    arch="$(uname -m)"
    case "$arch" in
    x86_64 | aarch64) ;;
    arm64) arch=aarch64 ;;
    *) unsupported_linux_arch "$arch" ;;
    esac
    ok "Linux on $arch, no apt-get, dnf or zypper: the tarball, into ~/.local"
    download "-linux-$arch.tar.gz" "$(installed_tarball)"

    datadir="$(data_dir)"
    # shellcheck disable=SC2088
    [[ "$datadir" = "$HOME/.local/share" ]] || where="~/.local/bin and $datadir"
    begin "Unpacking into $where"
    require_closed
    top="$(tar -tzf "$FILE" | sed -n 1p)"
    top="${top%/}"
    [[ "$top" =~ ^[A-Za-z0-9._+-]+$ ]] || fail "The tarball's layout is not as expected."
    mkdir -p "$HOME/.local/bin" "$datadir"
    bindir="$(cd "$HOME/.local/bin" && pwd -P)"
    datadir="$(cd "$datadir" && pwd -P)"
    refuse_directory_symlinks "$top" "$bindir" "$datadir"
    CHANGE_NOTE="$where may hold part of the tarball's files."
    record="$(tarball_record)"
    rm -f "$record"
    tar -xzf "$FILE" -C "$bindir" --strip-components=2 "$top/bin"
    tar -xzf "$FILE" -C "$datadir" --strip-components=2 "$top/share"
    CHANGE_NOTE="$NAME is unpacked into $where."
    printed="$(binary_version)" || printed=""
    [[ -n "$printed" ]] || fail "The unpacked $PACKAGE does not run on this system."
    mkdir -p "${record%/*}"
    printf '%s\n%s\n' "$VERSION" "$printed" >"$record"
    CHANGE_NOTE="$NAME is unpacked into $where; its menu entry or file types may not be."
    ok "Unpacked into $where"
    refresh_desktop
    CHANGE_NOTE=""

    summary "$HOME/.local/bin/$PACKAGE" \
        "the applications menu, or $(printf '%q' "$HOME/.local/bin/$PACKAGE")"
    case ":$PATH:" in
    *":$HOME/.local/bin:"*) check_command "$HOME/.local/bin/$PACKAGE" ;;
    *) warn "$HOME/.local/bin is not on your PATH:" \
        "add it, for the terminal and for the menu entry, which runs $PACKAGE by its name" ;;
    esac
}

# The menu entry and Open With of the tarball's files in the data directory; run again
# when the same version is installed already, so that a run which stopped here completes.
refresh_desktop() {
    local data
    data="$(data_dir)"
    if command -v update-desktop-database >/dev/null; then
        begin "Updating the menu's entries"
        update-desktop-database "$data/applications" >/dev/null 2>&1 ||
            fail "update-desktop-database failed."
        ok "Updated the menu's entries"
    else
        warn "No update-desktop-database: $NAME may appear in the menu only after" \
            "you log in again"
    fi
    if command -v update-mime-database >/dev/null; then
        begin "Updating the file types"
        update-mime-database "$data/mime" >/dev/null 2>&1 ||
            fail "update-mime-database failed."
        ok "Updated the file types"
    else
        warn "No update-mime-database (shared-mime-info): Open With may not offer" \
            "$NAME for raw files"
    fi
}

cleanup() {
    local status=$? macos=0
    cleanup_macos || macos=$?
    [[ "$macos" -eq 0 ]] || status=1
    # Kept when the disk image is still mounted in it.
    if [[ -n "$TMP" && "$macos" -ne 2 ]]; then
        rm -rf "$TMP"
    fi
    if [[ "$status" -ne 0 ]]; then
        # A step that stopped with no reason of its own; the cleanup's warns name theirs.
        if [[ "$FAILED" != true && ( "$macos" -eq 0 || -n "$STEP" ) ]]; then
            printf '%s' "$CLEAR"
            case "$status" in
            130 | 143) STEP="${STEP:-Installing} (interrupted)" ;;
            *) STEP="${STEP:-Installing} (a command failed with exit status $status)" ;;
            esac
            printf '  %s%s%s %s\n' "$RED" "$MARK_FAIL" "$RESET" "$STEP" >&2
        fi
        printf '\n  %s\n\n' "${CHANGE_NOTE:-Nothing was changed.}" >&2
    fi
    exit "$status"
}

main() {
    setup_ui
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    choose_channel "$@"
    header

    local system
    system="$(uname -s)"
    case "$system" in
    Darwin | Linux) ;;
    *) fail "This installer is for macOS and Linux; this system is $system." ;;
    esac

    TMP="$(mktemp -d)"
    if [[ "$system" = Darwin ]]; then
        install_macos
    else
        install_linux
    fi
}

main "$@" </dev/null
