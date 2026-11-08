#!/usr/bin/env python3
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

"""Writes layout.DS_Store and layout-dev.DS_Store, Finder's view of the disk image's
window for Apertrust and for Apertrust Dev (docs/packaging.md).

Run by hand, on any system, only when the layout changes, and commit what it writes;
packaging copies the file into each image and never needs Python:

    pip install ds_store==1.3.3 mac_alias==2.2.3   # both MIT, in a virtual environment
    python3 packaging/macos/dmg/generate.py

Each file is written from nothing, so it holds nothing of the machine that ran this: no
path, user, disk or date but the fixed one below. Finder finds the background through an
alias that holds only the volume's name and /.background/background.tiff, so the volume
must be named as LAYOUTS has it, with no version, as cargo-packager names it after the
product; the app's icon is placed by its bundle's name.
"""

import datetime
import os

from ds_store import DSStore
from mac_alias import ALIAS_EJECTABLE_DISK, ALIAS_KIND_FILE, Alias, TargetInfo, VolumeInfo

HERE = os.path.dirname(os.path.abspath(__file__))

# Each layout's file and its volume, the product's name, the app's bundle named after
# it. Held to background.svg's comment, tools/package.sh and packaging/dev-identity.txt.
LAYOUTS = {"layout.DS_Store": "Apertrust", "layout-dev.DS_Store": "Apertrust Dev"}
BACKGROUND = ".background/background.tiff"
WIDTH, HEIGHT = 660, 400  # the window's content, in points: the background's 1x size
TITLE_BAR = 32  # Finder's title bar with no toolbar, above the content
ICON_SIZE = 128
TEXT_SIZE = 13
# Each icon's centre in the content area: the app's, then the Applications link's.
APP_AT, APPLICATIONS_AT = (170, 196), (490, 196)
# Where the window opens on the screen, its top-left corner.
ORIGIN = (200, 120)
# The one date the file holds, in the alias. Finder matches the alias by name and path,
# never by this, since every image is a new volume.
DATE = datetime.datetime(2026, 11, 7, tzinfo=datetime.timezone.utc)


def background_alias(volume_name):
    """An alias to /.background/background.tiff on the volume `volume_name`, as Finder
    records one on a disk image, with every hint of a real volume (its id, its folders'
    ids) left as a placeholder Finder does not need."""
    folder, name = BACKGROUND.split("/")
    volume = VolumeInfo(volume_name, DATE, b"H+", ALIAS_EJECTABLE_DISK, 0, b"\0\0")
    volume.posix_path = f"/Volumes/{volume_name}"
    target = TargetInfo(ALIAS_KIND_FILE, name, 0, 0, DATE, b"\0\0\0\0", b"\0\0\0\0")
    target.folder_name = folder
    target.carbon_path = f"{volume_name}:{folder}:\0{name}"
    target.posix_path = f"/{BACKGROUND}"
    return Alias(volume=volume, target=target).to_bytes()


def write(out, volume):
    """Writes the layout of the window of the volume `volume` to `out`."""
    if os.path.exists(out):
        os.remove(out)
    with DSStore.open(out, "w+") as store:
        x, y = ORIGIN
        store["."]["bwsp"] = {
            "WindowBounds": f"{{{{{x}, {y}}}, {{{WIDTH}, {HEIGHT + TITLE_BAR}}}}}",
            "ShowToolbar": False,
            "ShowStatusBar": False,
            "ShowTabView": False,
            "ShowSidebar": False,
            "ContainerShowSidebar": False,
        }
        store["."]["icvp"] = {
            "viewOptionsVersion": 1,
            "backgroundType": 2,  # a picture
            "backgroundImageAlias": background_alias(volume),
            "backgroundColorRed": 1.0,
            "backgroundColorGreen": 1.0,
            "backgroundColorBlue": 1.0,
            "arrangeBy": "none",
            "gridOffsetX": 0.0,
            "gridOffsetY": 0.0,
            "gridSpacing": 100.0,
            "iconSize": float(ICON_SIZE),
            "textSize": float(TEXT_SIZE),
            "labelOnBottom": True,
            "showIconPreview": True,
            "showItemInfo": False,
        }
        store["."]["vSrn"] = ("long", 1)
        for name, position in {f"{volume}.app": APP_AT, "Applications": APPLICATIONS_AT}.items():
            store[name]["Iloc"] = position


def main():
    for out, volume in LAYOUTS.items():
        write(os.path.join(HERE, out), volume)


if __name__ == "__main__":
    main()
