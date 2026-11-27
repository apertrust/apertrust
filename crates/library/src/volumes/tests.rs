// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! The volumes told from the mounts each system reports, on every system.

use tempfile::TempDir;

use super::*;

/// A mount named `name` at `point`, of the file system `file_system`.
fn mount(name: &str, point: &str, file_system: &str, removable: bool) -> Mount {
    Mount {
        name: name.to_owned(),
        mount_point: PathBuf::from(point),
        file_system: file_system.to_owned(),
        removable,
        available: 6_200_000_000,
        total: 412_000_000_000,
    }
}

/// The mounts as each system reports them, each with whether a person imports from it and
/// the name the file manager shows.
fn reported(system: System) -> Vec<(Mount, Option<&'static str>)> {
    match system {
        System::MacOs => vec![
            (
                mount("Macintosh HD", "/", "apfs", false),
                Some("Macintosh HD"),
            ),
            (mount("Data", "/System/Volumes/Data", "apfs", false), None),
            (
                mount("Preboot", "/System/Volumes/Preboot", "apfs", false),
                None,
            ),
            (mount("VM", "/System/Volumes/VM", "apfs", false), None),
            (
                mount("Update", "/System/Volumes/Update", "apfs", false),
                None,
            ),
            (mount("xarts", "/System/Volumes/xarts", "apfs", false), None),
            (mount("devfs", "/dev", "devfs", false), None),
            (
                mount("EOS_DIGITAL", "/Volumes/EOS_DIGITAL", "exfat", true),
                Some("EOS_DIGITAL"),
            ),
            (
                mount("Backup", "/Volumes/Backup", "apfs", true),
                Some("Backup"),
            ),
            (mount("", "/Volumes/NAS", "smbfs", false), Some("NAS")),
            (
                mount(
                    "snapshot",
                    "/Volumes/com.apple.TimeMachine.localsnapshots",
                    "apfs",
                    false,
                ),
                None,
            ),
            (
                mount("Recovery", "/System/Volumes/Recovery", "apfs", false),
                None,
            ),
            (
                mount(
                    "iOS 18",
                    "/Library/Developer/CoreSimulator/Volumes/iOS_22A",
                    "apfs",
                    false,
                ),
                None,
            ),
        ],
        System::Linux => vec![
            (mount("/dev/nvme0n1p2", "/", "ext4", false), Some("/")),
            (mount("/dev/nvme0n1p1", "/boot/efi", "vfat", false), None),
            (mount("/dev/nvme0n1p3", "/home", "ext4", false), None),
            (
                mount("/dev/loop3", "/snap/core22/1380", "squashfs", false),
                None,
            ),
            (mount("tmpfs", "/run/user/1000", "tmpfs", false), None),
            (
                mount("/dev/sdb1", "/run/media/ana/EOS_DIGITAL", "exfat", true),
                Some("EOS_DIGITAL"),
            ),
            (
                mount("/dev/sdc1", "/media/ana/Backup", "ext4", true),
                Some("Backup"),
            ),
            (
                mount("//nas/photos", "/mnt/photos", "cifs", false),
                Some("photos"),
            ),
            (mount("/dev/sdd1", "/data", "btrfs", false), Some("data")),
            (
                mount(
                    "overlay",
                    "/var/lib/docker/overlay2/x/merged",
                    "overlay",
                    false,
                ),
                None,
            ),
            (mount("/dev/sda2", "/var", "ext4", false), None),
        ],
        System::Windows => vec![
            (mount("Windows", "C:\\", "NTFS", false), Some("Windows")),
            (
                mount("EOS_DIGITAL", "E:\\", "exFAT", true),
                Some("EOS_DIGITAL"),
            ),
            (mount("", "D:\\", "NTFS", false), Some("D:")),
            (mount("", "C:\\Mounts\\Card", "exFAT", true), Some("Card")),
        ],
    }
}

/// Each system's own volumes are left out, and the rest are named as its file manager
/// names them.
#[test]
fn the_system_s_own_volumes_are_left_out_and_the_rest_named() {
    for system in [System::MacOs, System::Linux, System::Windows] {
        for (mount, expected) in reported(system) {
            let told = shown(&mount, system).then(|| name(&mount, system));
            assert_eq!(told.as_deref(), expected, "{system:?}: {mount:?}");
        }
    }
}

/// A path's volume is the mount whose mount point is its longest prefix, by whole parts.
#[test]
fn a_path_s_volume_is_its_longest_mount_point() {
    let mounts: Vec<Mount> = reported(System::MacOs)
        .into_iter()
        .map(|(m, _)| m)
        .collect();
    let on = |path: &str| volume_for(Path::new(path), &mounts).map(|m| m.name.as_str());
    assert_eq!(on("/Users/ana/Pictures/Apertrust"), Some("Macintosh HD"));
    assert_eq!(on("/System/Volumes/Data/Users/ana"), Some("Data"));
    assert_eq!(
        on("/Volumes/EOS_DIGITAL/DCIM/100CANON"),
        Some("EOS_DIGITAL")
    );
    // A prefix of the name alone is no mount point.
    assert_eq!(on("/Volumes/EOS_DIGITAL_2/DCIM"), Some("Macintosh HD"));
    assert_eq!(volume_for(Path::new("relative"), &mounts), None);
}

/// A volume with `DCIM` at its top is a card; one that cannot be read is left out.
#[test]
fn a_volume_with_dcim_at_its_top_is_a_card() {
    let card = TempDir::new().unwrap();
    fs::create_dir(card.path().join("DCIM")).unwrap();
    let drive = TempDir::new().unwrap();
    let point = |dir: &TempDir| dir.path().to_string_lossy().into_owned();
    let gone = drive.path().join("gone").to_string_lossy().into_owned();
    let mounts = [
        mount("EOS_DIGITAL", &point(&card), "exfat", true),
        mount("Backup", &point(&drive), "ext4", true),
        mount("Gone", &gone, "ext4", true),
    ];
    let told: Vec<(String, bool)> = (volumes(&mounts, System::Windows).into_iter())
        .map(|volume| (volume.name, volume.card))
        .collect();
    assert_eq!(
        told,
        [
            ("EOS_DIGITAL".to_owned(), true),
            ("Backup".to_owned(), false)
        ]
    );
}
