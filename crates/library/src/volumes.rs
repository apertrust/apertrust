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

//! The volumes mounted, for Import's Source and its preview (decision 85): the cards and
//! drives a person imports from, each by the name the system's file manager shows, and a
//! path's volume with its free space.
//!
//! The mounts come from `sysinfo`'s disk part ([`mounts`]); what is told of them is pure
//! and tested on every system against the mounts each system reports ([`shown`], [`name`],
//! [`volume_for`]): the system's own volumes, the boot volume's system parts, recovery,
//! snapshots and virtual file systems, are left out. A card is the card rule's
//! ([`crate::source::card_at`]). Nothing is written on any volume.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A volume a person would import from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Volume {
    /// Its name as the system's file manager shows it: its label, such as `EOS_DIGITAL`.
    pub name: String,
    pub mount_point: PathBuf,
    /// Whether it holds a `DCIM` folder at its top ([`crate::source::card_at`]).
    pub card: bool,
    /// Whether the system takes it for removable, a card in a reader or a USB drive.
    pub removable: bool,
    /// Its free and its whole space, in bytes.
    pub available: u64,
    pub total: u64,
}

/// A path's volume and its free space, for Import's preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeSpace {
    /// The volume's name, as [`Volume::name`].
    pub volume: String,
    /// Its free and its whole space, in bytes.
    pub available: u64,
    pub total: u64,
}

/// A mount as the system reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// The name the system gives it: the volume's label on macOS and Windows, the device's
    /// on Linux.
    pub name: String,
    pub mount_point: PathBuf,
    /// Its file system's name, as the system reports it (`apfs`, `ext4`, `NTFS`).
    pub file_system: String,
    pub removable: bool,
    pub available: u64,
    pub total: u64,
}

/// The systems whose mounts are told apart, each by its own rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    MacOs,
    Linux,
    Windows,
}

impl System {
    /// The system this build runs on; Linux's rules elsewhere.
    pub const THIS: System = if cfg!(target_os = "macos") {
        System::MacOs
    } else if cfg!(windows) {
        System::Windows
    } else {
        System::Linux
    };
}

/// File systems that hold no one's files: the kernel's own, memory, snapshots and the
/// automounter's.
const VIRTUAL: &[&str] = &[
    "autofs",
    "binfmt_misc",
    "bpf",
    "cgroup",
    "cgroup2",
    "configfs",
    "debugfs",
    "devfs",
    "devpts",
    "devtmpfs",
    "efivarfs",
    "fusectl",
    "fuse.portal",
    "hugetlbfs",
    "mqueue",
    "nsfs",
    "nullfs",
    "overlay",
    "proc",
    "pstore",
    "ramfs",
    "securityfs",
    "squashfs",
    "sysfs",
    "tmpfs",
    "tracefs",
    "fuse.gvfsd-fuse",
];

/// The system's own folders at the top of a Linux file system, a mount under which is the
/// system's: boot and EFI partitions, snaps, run-time and virtual file systems.
const LINUX_SYSTEM: &[&str] = &[
    "boot", "dev", "efi", "etc", "home", "nix", "opt", "proc", "root", "run", "snap", "sys", "tmp",
    "usr", "var",
];

/// Whether a person would import from `mount` on `system`: not a virtual file system,
/// and on macOS the boot volume or one under `/Volumes`, a Time Machine snapshot left
/// out; on Linux the root or one outside the system's own folders, removable media under
/// `/run/media` and `/media` among them; on Windows a drive or a volume mounted in a
/// folder.
pub fn shown(mount: &Mount, system: System) -> bool {
    if VIRTUAL.contains(&mount.file_system.as_str()) {
        return false;
    }
    let point = &mount.mount_point;
    match system {
        System::MacOs => {
            let in_volumes = point.parent() == Some(Path::new("/Volumes"))
                && !(point.file_name()).is_some_and(|name| {
                    name.to_string_lossy().starts_with("com.apple.")
                        || name.to_string_lossy().starts_with(".timemachine")
                });
            point == Path::new("/") || in_volumes
        }
        System::Linux => {
            let mut parts = point.components().skip(1).map(Component::as_os_str);
            match parts.next() {
                None => point == Path::new("/"),
                Some(top) if top == OsStr::new("run") => {
                    parts.next() == Some(OsStr::new("media")) && parts.next().is_some()
                }
                Some(top) => !LINUX_SYSTEM.iter().any(|system| top == OsStr::new(system)),
            }
        }
        System::Windows => !point.as_os_str().is_empty(),
    }
}

/// The name the system's file manager shows `mount` by on `system`: its label; on Linux,
/// where the system gives the device's name, and for a volume with no label, the mount
/// point's last part, which the desktop's automounter names by the label: a Windows drive's
/// letter, `D:`, and `/` for the root. Told from the mount point's text, so each system's
/// are tested on every system.
pub fn name(mount: &Mount, system: System) -> String {
    if system != System::Linux && !mount.name.is_empty() {
        return mount.name.clone();
    }
    let point = mount.mount_point.to_string_lossy();
    let separator = |c: char| c == '/' || (system == System::Windows && c == '\\');
    let last = point.split(separator).rfind(|part| !part.is_empty());
    last.map_or_else(|| point.clone().into_owned(), str::to_owned)
}

/// The mount `path` lies on: the one whose mount point is its longest prefix, by whole
/// parts of the path.
pub fn volume_for<'a>(path: &Path, mounts: &'a [Mount]) -> Option<&'a Mount> {
    (mounts.iter())
        .filter(|mount| path.starts_with(&mount.mount_point))
        .max_by_key(|mount| mount.mount_point.components().count())
}

/// The mounts the system reports, through `sysinfo`'s disk part; none where it cannot list
/// them, which is logged. A blocking call: it asks each mount for its space.
pub fn mounts() -> Vec<Mount> {
    let disks = match sysinfo::Disks::new_with_refreshed_list() {
        Ok(disks) => disks,
        Err(error) => {
            tracing::warn!("the volumes cannot be listed: {error}");
            return Vec::new();
        }
    };
    (disks.iter())
        .map(|disk| Mount {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_owned(),
            file_system: disk.file_system().to_string_lossy().into_owned(),
            removable: disk.is_removable(),
            available: disk.available_space(),
            total: disk.total_space(),
        })
        .collect()
}

/// The volumes a person would import from among `mounts` on `system`, in the order the
/// system lists them, each told a card or not; a mount point that cannot be read is left
/// out and logged. A blocking call: it looks at each volume's top.
pub fn volumes(mounts: &[Mount], system: System) -> Vec<Volume> {
    (mounts.iter())
        .filter(|mount| shown(mount, system))
        .filter(|mount| match fs::metadata(&mount.mount_point) {
            Ok(_) => true,
            Err(error) => {
                let point = mount.mount_point.display();
                tracing::warn!("{point}: left out of the volumes, cannot be read: {error}");
                false
            }
        })
        .map(|mount| Volume {
            name: name(mount, system),
            mount_point: mount.mount_point.clone(),
            card: crate::source::card_at(&mount.mount_point, &mount.mount_point).is_some(),
            removable: mount.removable,
            available: mount.available,
            total: mount.total,
        })
        .collect()
}

/// The free space of the volume `path` lies on ([`volume_for`]) among the system's mounts,
/// and the volume's name; `None` where no mount holds it or the mounts cannot be listed. A
/// blocking call.
pub fn free_space(path: &Path) -> Option<FreeSpace> {
    let mounts = mounts();
    let mount = volume_for(path, &mounts)?;
    Some(FreeSpace {
        volume: name(mount, System::THIS),
        available: mount.available,
        total: mount.total,
    })
}

#[cfg(test)]
mod tests;
