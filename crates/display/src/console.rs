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

//! The terminal a windowed program was started from (docs/packaging.md, Windows).
//!
//! On Windows the app is a windowed program, so Explorer opens no console beside it; and
//! a windowed program has no console of its own, so what it prints goes nowhere, even
//! when a terminal started it. [`attach_to_terminal`] attaches it to the console of the
//! process that started it, where there is one, and points stdout and stderr at it,
//! unless they were redirected, so `--version`, `-d`'s lines and the errors print there
//! as a console program's do. Started from Explorer there is no such console, and
//! nothing is printed. Rust's stdout and stderr ask Windows for the standard handle at
//! every write, so setting it once, before anything is written, is enough.

/// What a standard stream's handle is as the program starts.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stream {
    /// No handle, or one that names nothing: a windowed program's, as Explorer and the
    /// terminals start one.
    Missing,
    /// A character device: a console's, handed on by a parent that passes its handles to
    /// the program it starts, as cargo does, or the null device.
    Character,
    /// A file or a pipe, which the person redirected it to: kept as it is.
    Redirected,
}

/// Whether to attach to the parent's console: unless both streams are redirected, since
/// a console handle handed on is only sure to work in a process attached to its console.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn attach_wanted(streams: [Stream; 2]) -> bool {
    streams.iter().any(|stream| *stream != Stream::Redirected)
}

/// Which of the streams, as they are once attached or not, to point at the console: those
/// still missing, once attached. One redirected, or a console's handed on, is kept.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn reopened(streams: [Stream; 2], attached: bool) -> [bool; 2] {
    streams.map(|stream| attached && stream == Stream::Missing)
}

/// Attaches the program to the console of the process that started it, where it has one,
/// and points stdout and stderr at it where they have no handle. Call it first in `main`,
/// before anything is printed or the log is set up. It does nothing where no process
/// with a console started the program, as from Explorer, and nothing off Windows.
pub fn attach_to_terminal() {
    #[cfg(windows)]
    windows::attach();
}

#[cfg(windows)]
mod windows {
    use std::ptr;

    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_CHAR, FILE_TYPE_UNKNOWN,
        GetFileType, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE,
        STD_OUTPUT_HANDLE, SetStdHandle,
    };

    use super::{Stream, attach_wanted, reopened};

    const STREAMS: [STD_HANDLE; 2] = [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE];

    pub(super) fn attach() {
        let streams = STREAMS.map(stream);
        if !attach_wanted(streams) {
            return;
        }
        // SAFETY: a plain call; it fails where the parent has no console, or where this
        // process has one already.
        let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;
        let reopen = reopened(STREAMS.map(stream), attached);
        if !reopen.contains(&true) {
            return;
        }
        // The console's screen buffer, readable too: Rust tells a console from a file by
        // reading its mode, which needs read access.
        let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
        // SAFETY: the name is a terminated UTF-16 string that outlives the call, and the
        // other arguments are constants or null, as the call allows.
        let console = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if console == INVALID_HANDLE_VALUE {
            return;
        }
        // The handle is kept open for the process's life, as a standard handle is.
        for (id, reopen) in STREAMS.into_iter().zip(reopen) {
            if reopen {
                // SAFETY: a valid handle, set before any thread of the program writes.
                unsafe { SetStdHandle(id, console) };
            }
        }
    }

    fn stream(id: STD_HANDLE) -> Stream {
        // SAFETY: a plain call; the handle it returns may be null or stale.
        let handle: HANDLE = unsafe { GetStdHandle(id) };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Stream::Missing;
        }
        // SAFETY: a handle that is not null; a stale one answers FILE_TYPE_UNKNOWN.
        match unsafe { GetFileType(handle) } {
            FILE_TYPE_UNKNOWN => Stream::Missing,
            FILE_TYPE_CHAR => Stream::Character,
            _ => Stream::Redirected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stream::{Character, Missing, Redirected};
    use super::{attach_wanted, reopened};

    #[test]
    fn started_from_explorer_or_a_terminal() {
        // Explorer and cmd start a windowed program with no handles: it attaches where
        // the parent has a console, and points both streams at it.
        assert!(attach_wanted([Missing, Missing]));
        assert_eq!(reopened([Missing, Missing], true), [true, true]);
        // Explorer has no console, so the attach fails and nothing is printed.
        assert_eq!(reopened([Missing, Missing], false), [false, false]);
    }

    #[test]
    fn a_redirected_stream_is_kept() {
        // `apertrust --version > out.txt`: stdout stays the file, stderr the console.
        assert!(attach_wanted([Redirected, Missing]));
        assert_eq!(reopened([Redirected, Missing], true), [false, true]);
        // Both redirected, or both pipes, as Git Bash's terminal gives: no attach at all.
        assert!(!attach_wanted([Redirected, Redirected]));
    }

    #[test]
    fn a_console_handed_on_is_kept() {
        // cargo run hands its console's handles on: attached to that console, they work.
        assert!(attach_wanted([Character, Character]));
        assert_eq!(reopened([Character, Character], true), [false, false]);
    }
}
