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

//! A file written and read back past the system's cache, so a read-back checks what the
//! disk holds, not the pages the write has just filled (decision 101; the method each
//! system takes, and its cost, are in library.md's Move to… and Copy to…):
//!
//! - macOS: `F_NOCACHE` on the handle that writes, so its pages are not kept, and on the
//!   one that reads; `F_NOCACHE` on the read alone is answered from the pages still there.
//!   The sync is `F_FULLFSYNC`, or `fsync` where a share refuses it, as SMB does;
//! - Linux: `fsync`, then, as the read-back opens the file, `posix_fadvise(POSIX_FADV_DONTNEED)`,
//!   which drops the pages the sync has made clean, on every file system, where `O_DIRECT`
//!   is refused by some and asks for aligned buffers;
//! - Windows: `FlushFileBuffers`, then a read with `FILE_FLAG_NO_BUFFERING`, in whole
//!   sectors into an aligned buffer.
//!
//! [`uncache_writes`] before the bytes are written, [`sync`] once they are, then
//! [`read_uncached`].

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// How the reads of [`read_uncached`] are aligned, and what their length is a multiple of:
/// a page, and a multiple of every sector size in use, 512 bytes and 4 KiB.
const ALIGN: usize = 4096;

/// Asks the system not to keep the pages of what is written through `file` (macOS's
/// `F_NOCACHE`); nothing elsewhere, where [`read_uncached`] drops them or passes them.
pub fn uncache_writes(file: &File) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    rustix::fs::fcntl_nocache(file, true)?;
    #[cfg(not(target_vendor = "apple"))]
    let _ = file;
    Ok(())
}

/// Puts what was written through `file` on the disk: on macOS `F_FULLFSYNC`, which std's
/// `sync_all` is, falling back to `fsync` where the file system refuses it, as a share over
/// SMB does; elsewhere `sync_all`. The pages stay in the system's cache; [`read_uncached`]
/// passes them.
pub fn sync(file: &File) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    match rustix::fs::fcntl_fullfsync(file) {
        Err(rustix::io::Errno::NOTSUP | rustix::io::Errno::INVAL) => rustix::fs::fsync(file)?,
        done => done?,
    }
    #[cfg(not(target_vendor = "apple"))]
    file.sync_all()?;
    Ok(())
}

/// Reads the file at `path` from start to end past the system's cache, handing `each` the
/// bytes in order, at most `chunk` (rounded up to 4 KiB) at a time; the bytes read.
pub fn read_uncached(
    path: &Path,
    chunk: usize,
    mut each: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<u64> {
    let mut file = open_uncached(path)?;
    let chunk = chunk.max(1).next_multiple_of(ALIGN);
    // A buffer one alignment longer, its aligned part used: Windows' unbuffered reads need
    // an address and a length in whole sectors.
    let mut buffer = vec![0u8; chunk + ALIGN];
    let start = buffer.as_ptr().align_offset(ALIGN).min(ALIGN);
    let buffer = &mut buffer[start..start + chunk];
    let mut total = 0;
    loop {
        let read = match file.read(buffer) {
            Ok(0) => return Ok(total),
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        each(&buffer[..read])?;
        total += read as u64;
        // Unbuffered, a read shorter than asked is the file's end, and the position after
        // it no longer whole sectors, which the next read would be refused for.
        if cfg!(windows) && read < buffer.len() {
            return Ok(total);
        }
    }
}

fn open_uncached(path: &Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        /// `FILE_FLAG_NO_BUFFERING`, from `winbase.h`.
        const NO_BUFFERING: u32 = 0x2000_0000;
        File::options()
            .read(true)
            .custom_flags(NO_BUFFERING)
            .open(path)
    }
    #[cfg(not(windows))]
    {
        let file = File::open(path)?;
        #[cfg(target_vendor = "apple")]
        rustix::fs::fcntl_nocache(&file, true)?;
        // The file's pages, clean once [`sync`] has put them on the disk, dropped for every
        // handle on it, so the reads below come from the disk; advice, so best effort.
        #[cfg(any(target_os = "linux", target_os = "android"))]
        rustix::fs::fadvise(&file, 0, None, rustix::fs::Advice::DontNeed)?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    /// The bytes come back whole and in order, of any length, through every step: the
    /// method's effect on the cache is measured, not tested (library.md).
    #[test]
    fn a_file_written_uncached_reads_back_whole() {
        let folder = tempfile::tempdir().expect("a folder");
        for len in [0, 1, ALIGN - 1, ALIGN, ALIGN + 1, 3 * ALIGN + 17, 1 << 20] {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 31 % 251) as u8).collect();
            let path = folder.path().join(format!("{len}.bin"));
            let mut file = File::create(&path).unwrap();
            uncache_writes(&file).unwrap();
            file.write_all(&bytes).unwrap();
            sync(&file).unwrap();
            drop(file);
            for chunk in [1, ALIGN, 64 * 1024] {
                let mut read = Vec::new();
                let total = read_uncached(&path, chunk, |part| {
                    assert!(part.len() <= chunk.next_multiple_of(ALIGN));
                    read.extend_from_slice(part);
                    Ok(())
                })
                .unwrap();
                assert_eq!(total, len as u64);
                assert_eq!(read, bytes, "{len} bytes in chunks of {chunk}");
            }
        }
    }

    #[test]
    fn an_error_of_the_reader_stops_the_read() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("a.bin");
        std::fs::write(&path, vec![1u8; 3 * ALIGN]).unwrap();
        let mut calls = 0;
        let result = read_uncached(&path, ALIGN, |_| {
            calls += 1;
            Err(io::ErrorKind::InvalidData.into())
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(calls, 1);
        assert_eq!(
            read_uncached(&folder.path().join("missing"), ALIGN, |_| Ok(()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
}
