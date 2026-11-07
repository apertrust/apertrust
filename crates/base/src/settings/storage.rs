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

//! Where settings are stored (decision 35): [`Storage`], and the desktop's, a file.

use std::fs::{File, Permissions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Where settings are stored: a file on the desktop, the browser's storage on the web, a
/// folder of a test's own in tests.
pub trait Storage: Send {
    /// Where, for the log.
    fn location(&self) -> String;

    /// What is stored, or None when nothing is yet. Text that is not UTF-8 is an
    /// [`io::ErrorKind::InvalidData`] error.
    fn read(&self) -> io::Result<Option<String>>;

    /// Stores `text` in place of what was there, whole or not at all.
    fn write(&mut self, text: &str) -> io::Result<()>;

    /// Moves what is stored aside, where the user can still find it, and says where.
    fn set_aside(&mut self) -> io::Result<String>;

    /// Whether what is stored is to be read and never written, as a file a person made
    /// read-only.
    fn read_only(&self) -> bool {
        false
    }

    /// Removes what writes left behind, as the temporary file of a process that ended
    /// in the middle of one, before anything is read; says how many it removed.
    fn tidy(&mut self) -> usize {
        0
    }
}

/// How many writes this process has begun, of every [`FileStorage`] in it, which names
/// their temporary files, so no two ever share one.
static WRITES: AtomicU64 = AtomicU64::new(0);

/// How old a temporary file must be to be taken for one a process left behind: a write
/// takes milliseconds, so another Apertrust's that is under way is never removed.
const STALE: Duration = Duration::from_secs(60);

/// Settings in a file: `settings.toml`, set aside as `settings.toml.bak`.
pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    pub fn new(path: PathBuf) -> Self {
        FileStorage { path }
    }

    /// The file itself: where `settings.toml` points if it is a symbolic link (or a link
    /// to one), so that writing keeps the link.
    fn file(&self) -> PathBuf {
        let mut file = self.path.clone();
        // Links of links, but not a loop of them.
        for _ in 0..8 {
            let Ok(target) = std::fs::read_link(&file) else {
                break;
            };
            file = match file.parent() {
                Some(folder) => folder.join(target),
                None => target,
            };
        }
        file
    }
}

/// `path` with `suffix` after its whole name.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Whether `name` is one of `file`'s temporary files: `settings.toml.<process>-<write>.tmp`.
fn temporary(file: &str, name: &str) -> bool {
    let middle = name
        .strip_prefix(file)
        .and_then(|rest| rest.strip_prefix('.'))
        .and_then(|rest| rest.strip_suffix(".tmp"));
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    middle
        .and_then(|middle| middle.split_once('-'))
        .is_some_and(|(process, write)| digits(process) && digits(write))
}

/// Creates `path`, which must not exist, to write; on Unix already with the mode of
/// `permissions`, the file's it is to replace, so its text is never readable by more than
/// that file's was.
fn create(path: &Path, permissions: Option<&Permissions>) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if let Some(permissions) = permissions {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(permissions.mode() & 0o7777);
    }
    #[cfg(not(unix))]
    let _ = permissions;
    options.open(path)
}

impl Storage for FileStorage {
    fn location(&self) -> String {
        self.path.display().to_string()
    }

    fn read(&self) -> io::Result<Option<String>> {
        match std::fs::read(&self.path) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Writes a temporary file of this write's own beside the file, with the file's
    /// permissions, syncs it to the disk and renames it into place, so neither a failed
    /// write nor a power cut leaves half a file, two Apertrusts writing at once never share
    /// one, and a file only its owner may read stays so.
    fn write(&mut self, text: &str) -> io::Result<()> {
        let file = self.file();
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let write = WRITES.fetch_add(1, Ordering::Relaxed) + 1;
        let suffix = format!(".{}-{write}.tmp", std::process::id());
        let temporary = beside(&file, &suffix);
        let permissions = std::fs::metadata(&file).ok().map(|m| m.permissions());
        let written = create(&temporary, permissions.as_ref())
            .and_then(|mut f| f.write_all(text.as_bytes()).and_then(|()| f.sync_all()))
            // Whole, as the process's umask may have taken some away as it was created.
            .and_then(|()| match permissions {
                Some(permissions) => std::fs::set_permissions(&temporary, permissions),
                None => Ok(()),
            })
            .and_then(|()| std::fs::rename(&temporary, &file));
        if written.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        written
    }

    /// Whether the file, or what it links to, is read-only: a rename would replace it all
    /// the same, since that asks only its folder.
    fn read_only(&self) -> bool {
        std::fs::metadata(&self.path).is_ok_and(|m| m.permissions().readonly())
    }

    /// Removes the file's temporary files that are more than a minute old, so another
    /// Apertrust's write under way keeps its own.
    fn tidy(&mut self) -> usize {
        let file = self.file();
        let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
            return 0;
        };
        let name = name.to_string_lossy();
        let Ok(entries) = std::fs::read_dir(folder) else {
            return 0;
        };
        let stale = |entry: &std::fs::DirEntry| {
            let modified = entry.metadata().and_then(|m| m.modified());
            let age = modified.ok().and_then(|modified| modified.elapsed().ok());
            age.is_some_and(|age| age >= STALE)
        };
        let mut removed = 0;
        for entry in entries.flatten() {
            let ours = temporary(&name, &entry.file_name().to_string_lossy());
            if ours && stale(&entry) && std::fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
        removed
    }

    /// Renames the file to `settings.toml.bak`, replacing an older one.
    fn set_aside(&mut self) -> io::Result<String> {
        let file = self.file();
        let aside = beside(&file, ".bak");
        std::fs::rename(&file, &aside)?;
        Ok(aside.display().to_string())
    }
}
