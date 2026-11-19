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

//! The system calls of the single-instance hand-off (decision 89; `docs/gui.md`, Instances
//! and windows) that the standard library does not make: the app's `instance` module holds
//! the lock, the message and the joins, and asks these.
//!
//! - **macOS and Linux:** the user the process runs as (`user_id`), the user at the other
//!   end of a Unix socket (`peer_user`: `SO_PEERCRED` on Linux, `getpeereid` on macOS,
//!   since the standard library's `peer_cred` is unstable), and whether a directory's
//!   access-control list grants another account a right (`acl_grants_others`: POSIX ACLs
//!   on Linux, macOS's extended ACLs). Each is a call into the system's C library, which
//!   the standard library links already.
//! - **Windows:** the instance's named pipe (`PipeListener`), created as the first
//!   instance of its name and open to the user's own account alone, a second start's end of
//!   it (`connect_pipe`), whether its server runs as the same user
//!   (`pipe_server_is_own`), and the right to bring a window forward handed to the
//!   instance ([`allow_foreground`]).
//! - **Wayland:** a window activated with another process's activation token
//!   (`activate_with_token`), `xdg_activation_v1.activate`, which winit 0.30 makes only
//!   for a window it creates.

use std::io;

/// Lets the process `id` bring its window forward (Windows' `AllowSetForegroundWindow`): a
/// second start calls it with the instance's process id before handing off, since only the
/// process the person started may take the foreground. Does nothing elsewhere, where the
/// instance comes forward by its activation token or on its own.
pub fn allow_foreground(id: u32) -> io::Result<()> {
    #[cfg(windows)]
    return windows::allow_foreground(id);
    #[cfg(not(windows))]
    {
        let _ = id;
        Ok(())
    }
}

/// Lets go of what `activate_with_token` keeps over winit's display, while that display is
/// still connected: call it as the event loop ends, on its thread, before winit's display
/// goes. Does nothing where nothing is kept, and off Linux.
pub fn end_activation() {
    #[cfg(target_os = "linux")]
    linux::end_activation();
}

#[cfg(unix)]
pub use unix::{acl_grants_others, peer_user, user_id};

#[cfg(target_os = "linux")]
pub use linux::{activate_with_token, posix_acl_grants_others};

#[cfg(windows)]
pub use windows::{PipeListener, connect_pipe, pipe_server_is_own};

#[cfg(unix)]
mod unix {
    use std::ffi::{CString, c_int};
    use std::io;
    use std::os::fd::AsRawFd as _;
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::net::UnixStream;
    use std::path::Path;

    unsafe extern "C" {
        fn geteuid() -> u32;
    }

    /// The effective user id the process runs as, which owns what it creates.
    pub fn user_id() -> u32 {
        // SAFETY: a plain call, which cannot fail.
        unsafe { geteuid() }
    }

    /// The effective user id of the process at the other end of `stream`, as the system
    /// recorded it when the connection was made.
    pub fn peer_user(stream: &UnixStream) -> io::Result<u32> {
        super::peer::user(stream.as_raw_fd())
    }

    /// Whether the access-control list of the directory at `path` grants another account
    /// than its owner any right (Linux's POSIX ACL; on macOS any entry that allows, which
    /// a directory of the user's own never has). False where the file system keeps none.
    pub fn acl_grants_others(path: &Path) -> io::Result<bool> {
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a NUL in the path"))?;
        super::acl::grants_others(&path)
    }

    /// Whether the last error is one that says there is no ACL to read.
    #[cfg_attr(not(any(target_os = "linux", target_os = "macos")), allow(dead_code))]
    pub(super) fn no_acl(error: &io::Error, codes: &[c_int]) -> bool {
        error
            .raw_os_error()
            .is_some_and(|code| codes.contains(&code))
    }
}

#[cfg(target_os = "linux")]
mod peer {
    use std::ffi::{c_int, c_void};
    use std::io;

    unsafe extern "C" {
        fn getsockopt(
            socket: c_int,
            level: c_int,
            name: c_int,
            value: *mut c_void,
            length: *mut u32,
        ) -> c_int;
    }

    /// `struct ucred`, the same on every Linux architecture.
    #[repr(C)]
    #[derive(Default)]
    struct Credentials {
        pid: i32,
        uid: u32,
        gid: u32,
    }

    // `SOL_SOCKET` and `SO_PEERCRED`: the generic values, but for the architectures that
    // number them their own way.
    #[cfg(any(target_arch = "mips", target_arch = "mips64"))]
    const LEVEL: c_int = 0xffff;
    #[cfg(any(target_arch = "mips", target_arch = "mips64"))]
    const PEER_CREDENTIALS: c_int = 18;
    #[cfg(any(target_arch = "sparc", target_arch = "sparc64"))]
    const LEVEL: c_int = 0xffff;
    #[cfg(any(target_arch = "sparc", target_arch = "sparc64"))]
    const PEER_CREDENTIALS: c_int = 0x40;
    #[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
    const LEVEL: c_int = 1;
    #[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
    const PEER_CREDENTIALS: c_int = 21;
    #[cfg(not(any(
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "sparc",
        target_arch = "sparc64",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )))]
    const LEVEL: c_int = 1;
    #[cfg(not(any(
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "sparc",
        target_arch = "sparc64",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )))]
    const PEER_CREDENTIALS: c_int = 17;

    pub(super) fn user(socket: c_int) -> io::Result<u32> {
        let mut credentials = Credentials::default();
        let mut length = size_of::<Credentials>() as u32;
        // SAFETY: the socket is a live descriptor its stream owns for the call, and the
        // value and its length point at a `ucred` of this process's, which the call fills.
        let done = unsafe {
            getsockopt(
                socket,
                LEVEL,
                PEER_CREDENTIALS,
                (&raw mut credentials).cast(),
                &mut length,
            )
        };
        if done != 0 {
            return Err(io::Error::last_os_error());
        }
        if length as usize != size_of::<Credentials>() {
            return Err(io::Error::other("the peer's credentials are not a ucred"));
        }
        Ok(credentials.uid)
    }
}

#[cfg(target_os = "macos")]
mod peer {
    use std::ffi::c_int;
    use std::io;

    unsafe extern "C" {
        fn getpeereid(socket: c_int, user: *mut u32, group: *mut u32) -> c_int;
    }

    pub(super) fn user(socket: c_int) -> io::Result<u32> {
        let (mut user, mut group) = (u32::MAX, u32::MAX);
        // SAFETY: the socket is a live descriptor its stream owns for the call, and both
        // pointers are this function's own, which the call fills.
        if unsafe { getpeereid(socket, &mut user, &mut group) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(user)
    }
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
mod peer {
    use std::ffi::c_int;
    use std::io;

    pub(super) fn user(_socket: c_int) -> io::Result<u32> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no peer credentials on this system",
        ))
    }
}

#[cfg(target_os = "linux")]
mod acl {
    use std::ffi::{CStr, c_char, c_void};
    use std::io;

    unsafe extern "C" {
        fn getxattr(
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: usize,
        ) -> isize;
    }

    /// `ENODATA` and `EOPNOTSUPP`: no ACL kept, or none on this file system.
    const NONE: [i32; 2] = [61, 95];

    pub(super) fn grants_others(path: &CStr) -> io::Result<bool> {
        let name = c"system.posix_acl_access";
        let mut value = vec![0u8; 1024];
        loop {
            // SAFETY: both strings are terminated and outlive the call, and the buffer is
            // this function's own, its length given.
            let read = unsafe {
                getxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    value.as_mut_ptr().cast(),
                    value.len(),
                )
            };
            if let Ok(read) = usize::try_from(read) {
                value.truncate(read);
                return Ok(super::linux::posix_acl_grants_others(&value));
            }
            let error = io::Error::last_os_error();
            if super::unix::no_acl(&error, &NONE) {
                return Ok(false);
            }
            // `ERANGE`: a longer list than the buffer holds.
            if error.raw_os_error() == Some(34) && value.len() < 1 << 20 {
                value.resize(value.len() * 4, 0);
                continue;
            }
            return Err(error);
        }
    }
}

#[cfg(target_os = "macos")]
mod acl {
    use std::ffi::{CStr, c_char, c_int, c_void};
    use std::io;
    use std::ptr;

    unsafe extern "C" {
        fn acl_get_file(path: *const c_char, kind: c_int) -> *mut c_void;
        fn acl_get_entry(acl: *mut c_void, id: c_int, entry: *mut *mut c_void) -> c_int;
        fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
        fn acl_free(object: *mut c_void) -> c_int;
    }

    /// `ACL_TYPE_EXTENDED`, macOS's one kind of ACL.
    const EXTENDED: c_int = 0x100;
    /// `ACL_FIRST_ENTRY` and `ACL_NEXT_ENTRY`.
    const FIRST: c_int = 0;
    const NEXT: c_int = -1;
    /// `ACL_EXTENDED_ALLOW`: an entry that grants a right.
    const ALLOW: c_int = 1;
    /// `ENOENT`: no ACL on the file.
    const NONE: [i32; 1] = [2];

    pub(super) fn grants_others(path: &CStr) -> io::Result<bool> {
        // SAFETY: the path is terminated and outlives the call.
        let acl = unsafe { acl_get_file(path.as_ptr(), EXTENDED) };
        if acl.is_null() {
            let error = io::Error::last_os_error();
            return if super::unix::no_acl(&error, &NONE) {
                Ok(false)
            } else {
                Err(error)
            };
        }
        let mut grants = false;
        let mut entry = ptr::null_mut();
        let mut id = FIRST;
        // SAFETY: `acl` is the list `acl_get_file` returned, live until it is freed below,
        // and each entry it gives is read before the next is asked for.
        while unsafe { acl_get_entry(acl, id, &mut entry) } == 0 {
            id = NEXT;
            let mut tag = 0;
            // SAFETY: an entry of the live list, and the tag this function's own.
            if unsafe { acl_get_tag_type(entry, &mut tag) } == 0 && tag == ALLOW {
                grants = true;
                break;
            }
        }
        // SAFETY: the list `acl_get_file` returned, freed once.
        unsafe { acl_free(acl) };
        Ok(grants)
    }
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
mod acl {
    use std::ffi::CStr;
    use std::io;

    pub(super) fn grants_others(_path: &CStr) -> io::Result<bool> {
        Ok(false)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::cell::RefCell;
    use std::mem::ManuallyDrop;

    use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle as _};
    use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
    use wayland_client::globals::{GlobalListContents, registry_queue_init};
    use wayland_client::protocol::{wl_registry::WlRegistry, wl_surface::WlSurface};
    use wayland_client::{
        Connection, Dispatch, EventQueue, Proxy, QueueHandle,
        backend::{Backend, ObjectId, WaylandError},
    };
    use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

    use crate::Error;

    /// Whether the POSIX ACL `value`, as Linux keeps it in `system.posix_acl_access`, grants
    /// another account than the owner a right: a named user's or group's entry, or the
    /// owning group's or others', with any permission. The mask grants nothing itself, and
    /// is not counted, so an entry it masks off still refuses the directory.
    pub fn posix_acl_grants_others(value: &[u8]) -> bool {
        // The header, the format's version, then entries of a tag, permissions and an id.
        const USER: u16 = 0x02;
        const GROUP_OWNER: u16 = 0x04;
        const GROUP: u16 = 0x08;
        const OTHER: u16 = 0x20;
        let Some(entries) = value.get(4..) else {
            return false;
        };
        entries.as_chunks::<8>().0.iter().any(|entry| {
            let tag = u16::from_le_bytes([entry[0], entry[1]]);
            let permissions = u16::from_le_bytes([entry[2], entry[3]]);
            matches!(tag, USER | GROUP_OWNER | GROUP | OTHER) && permissions != 0
        })
    }

    /// The registry's state, which keeps no events: the activation global has none, and the
    /// registry's later ones are not needed.
    struct Activation;

    impl Dispatch<WlRegistry, GlobalListContents> for Activation {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: <WlRegistry as Proxy>::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<XdgActivationV1, ()> for Activation {
        fn event(
            _: &mut Self,
            _: &XdgActivationV1,
            _: <XdgActivationV1 as Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    /// The activation global, bound once over winit's display and kept until the loop
    /// ends ([`end_activation`]), with the queue its registry lives on: binding it each time
    /// would leave a registry behind each time.
    struct Bound {
        display: usize,
        connection: Connection,
        activation: XdgActivationV1,
        _queue: EventQueue<Activation>,
    }

    // Never dropped with the thread: by then winit's display may be gone, and dropping the
    // proxies and queue kept over it would touch it. Only `end_activation` drops it; a
    // start that ends without it leaves it to the process's end.
    thread_local! {
        static BOUND: RefCell<Option<ManuallyDrop<Bound>>> = const { RefCell::new(None) };
    }

    /// [`super::end_activation`] on Linux: the global destroyed and what was kept dropped,
    /// while winit's display is connected.
    pub(super) fn end_activation() {
        let bound = BOUND.with_borrow_mut(Option::take);
        if let Some(bound) = bound {
            let bound = ManuallyDrop::into_inner(bound);
            bound.activation.destroy();
            let _ = bound.connection.flush();
        }
    }

    /// Activates `window` with `token`, another process's activation token, as a second
    /// start hands it over (`xdg_activation_v1.activate`): the compositor brings it
    /// forward, or marks it as asking for attention, as its policy says. Returns false on
    /// X11, or where the compositor has no activation global. Call it on the thread that
    /// runs winit's loop, from inside it.
    pub fn activate_with_token(window: &winit::window::Window, token: &str) -> Result<bool, Error> {
        let RawDisplayHandle::Wayland(display) = window.display_handle()?.as_raw() else {
            return Ok(false);
        };
        let RawWindowHandle::Wayland(surface) = window.window_handle()?.as_raw() else {
            return Ok(false);
        };
        let pointer = display.display.as_ptr() as usize;
        BOUND.with_borrow_mut(|bound| {
            // One bound over another display, which winit never makes, is left as it is
            // rather than dropped over a display that may be gone.
            if bound.as_ref().is_none_or(|bound| bound.display != pointer) {
                *bound = bind(display.display.as_ptr())?.map(ManuallyDrop::new);
            }
            let Some(bound) = bound.as_ref() else {
                return Ok(false);
            };
            // SAFETY: the surface is the window's own `wl_surface`, which winit destroys only
            // as the window is dropped; the window is borrowed for this whole call, beyond
            // which neither the id nor the proxy made from it is used.
            let id = unsafe {
                ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast())
            }?;
            let surface = WlSurface::from_id(&bound.connection, id)?;
            bound.activation.activate(token.to_owned(), &surface);
            match bound.connection.flush() {
                // Queued, not lost: winit's loop sends it with its own next flush.
                Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                result => result?,
            }
            Ok(true)
        })
    }

    /// Binds the activation global over the display at `display`, winit's: none where the
    /// compositor has none.
    fn bind(display: *mut std::ffi::c_void) -> Result<Option<Bound>, Error> {
        // SAFETY: the `wl_display` is winit's, connected for as long as its event loop runs,
        // and this is called from inside it. The `Backend` is a guest on it; kept with the
        // proxies made over it until `end_activation`, called as the loop ends, it is only
        // used and dropped from inside the loop, where the display is connected.
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<Activation>(&connection)?;
        let activation = match globals.bind::<XdgActivationV1, _, _>(&queue.handle(), 1..=1, ()) {
            Ok(activation) => activation,
            Err(_) => return Ok(None),
        };
        Ok(Some(Bound {
            display: display as usize,
            connection,
            activation,
            _queue: queue,
        }))
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle};
    use std::ptr;

    use windows_sys::Win32::Foundation::{
        ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE,
        INVALID_HANDLE_VALUE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        EqualSid, GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
        PIPE_WAIT,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

    /// The pipe's buffers, each way: a hand-off is a few hundred bytes.
    const BUFFER: u32 = 4096;

    pub(super) fn allow_foreground(id: u32) -> io::Result<()> {
        // SAFETY: a plain call.
        if unsafe { AllowSetForegroundWindow(id) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// `name` as a terminated UTF-16 string.
    fn wide(name: &str) -> Vec<u16> {
        name.encode_utf16().chain([0]).collect()
    }

    /// A handle the call returned, owned, or the last error where it returned none.
    fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a valid handle the call just returned, owned by no one else.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    /// The token user of the process `process`, its SID inside: a buffer aligned for
    /// `TOKEN_USER`.
    struct TokenOwner(Vec<u64>);

    impl TokenOwner {
        fn of(process: HANDLE) -> io::Result<TokenOwner> {
            let mut token = ptr::null_mut();
            // SAFETY: a process handle valid for the call, and the token this function's
            // own, closed below.
            if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
                return Err(io::Error::last_os_error());
            }
            let token = owned(token)?;
            let mut length = 0;
            // SAFETY: asks the length alone, with no buffer.
            unsafe {
                GetTokenInformation(
                    token.as_raw_handle(),
                    TokenUser,
                    ptr::null_mut(),
                    0,
                    &mut length,
                )
            };
            let mut buffer = vec![0u64; (length as usize).div_ceil(8).max(1)];
            // SAFETY: the buffer is this function's own, at least `length` bytes, aligned
            // for `TOKEN_USER`.
            let read = unsafe {
                GetTokenInformation(
                    token.as_raw_handle(),
                    TokenUser,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 8) as u32,
                    &mut length,
                )
            };
            if read == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(TokenOwner(buffer))
        }

        fn sid(&self) -> *mut c_void {
            // SAFETY: the buffer holds the `TOKEN_USER` the call wrote, its SID within it.
            unsafe { (*self.0.as_ptr().cast::<TOKEN_USER>()).User.Sid }
        }
    }

    /// The pipe's security: its DACL grants the user's own account, and no other, every
    /// right, as SDDL `D:P(A;;GA;;;<SID>)`, protected from inherited entries.
    struct Security(*mut c_void);

    impl Security {
        fn own() -> io::Result<Security> {
            // SAFETY: the current process's pseudo-handle, which needs no closing.
            let owner = TokenOwner::of(unsafe { GetCurrentProcess() })?;
            let mut text = ptr::null_mut();
            // SAFETY: a SID of the live buffer, and the string the call allocates, freed
            // below.
            if unsafe { ConvertSidToStringSidW(owner.sid(), &mut text) } == 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: the call wrote a terminated UTF-16 string at `text`.
            let length = (0..).take_while(|&i| unsafe { *text.add(i) } != 0).count();
            // SAFETY: as above, `length` units before the terminator.
            let sid = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) });
            // SAFETY: the string `ConvertSidToStringSidW` allocated, freed once.
            unsafe { LocalFree(text.cast()) };
            let sddl = wide(&format!("D:P(A;;GA;;;{sid})"));
            let mut descriptor = ptr::null_mut();
            // SAFETY: a terminated string, and the descriptor the call allocates, freed as
            // this drops.
            let made = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            };
            if made == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Security(descriptor))
        }
    }

    impl Drop for Security {
        fn drop(&mut self) {
            // SAFETY: the descriptor the conversion allocated, freed once.
            unsafe { LocalFree(self.0) };
        }
    }

    /// The instance's end of its named pipe: an instance of it waiting for a second start,
    /// the next made as each connects.
    pub struct PipeListener {
        name: Vec<u16>,
        waiting: OwnedHandle,
    }

    impl PipeListener {
        /// Creates the pipe `name` (`\\.\pipe\…`), failing where an instance of that name
        /// exists already, another process's included (`FILE_FLAG_FIRST_PIPE_INSTANCE`):
        /// open to the user's own account alone and to this computer's processes alone.
        pub fn bind(name: &str) -> io::Result<PipeListener> {
            let name = wide(name);
            let waiting = create(&name, true)?;
            Ok(PipeListener { name, waiting })
        }

        /// Waits for a second start to connect, and returns its end as a file to read and
        /// write, the pipe's next instance made first, so one always waits. Where the
        /// connection failed, as for a start that closed its end before it was taken
        /// (`ERROR_NO_DATA`), the waiting instance is disconnected, or replaced where it
        /// cannot be, so the next call waits on it afresh.
        pub fn accept(&mut self) -> io::Result<File> {
            // SAFETY: the waiting instance's handle, valid for the call; no overlapped I/O.
            let connected =
                unsafe { ConnectNamedPipe(self.waiting.as_raw_handle(), ptr::null_mut()) };
            // SAFETY: a plain call, right after the one it reads the error of.
            let error = (connected == 0).then(|| unsafe { GetLastError() });
            if let Some(code) = error.filter(|code| *code != ERROR_PIPE_CONNECTED) {
                // SAFETY: the waiting instance's handle, valid for the call.
                if unsafe { DisconnectNamedPipe(self.waiting.as_raw_handle()) } == 0 {
                    self.waiting = create(&self.name, false)?;
                }
                return Err(io::Error::from_raw_os_error(code as i32));
            }
            let next = create(&self.name, false)?;
            Ok(File::from(std::mem::replace(&mut self.waiting, next)))
        }
    }

    /// An instance of the pipe `name`, the first of its name where `first`.
    fn create(name: &[u16], first: bool) -> io::Result<OwnedHandle> {
        let security = Security::own()?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.0,
            bInheritHandle: 0,
        };
        let flags = if first {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
        // SAFETY: a terminated name and attributes that outlive the call.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | flags,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                BUFFER,
                BUFFER,
                0,
                &attributes,
            )
        };
        owned(handle)
    }

    /// A second start's end of the pipe `name`, as a file to read and write; the server
    /// may only identify this process, never act as it.
    pub fn connect_pipe(name: &str) -> io::Result<File> {
        let name = wide(name);
        // SAFETY: a terminated name that outlives the call, and constants or null.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                ptr::null_mut(),
            )
        };
        owned(handle).map(File::from)
    }

    /// Whether the process serving `pipe` runs as the same user as this one, and its
    /// process id (`GetNamedPipeServerProcessId`, then the process's token).
    pub fn pipe_server_is_own(pipe: &File) -> io::Result<(bool, u32)> {
        let mut id = 0;
        // SAFETY: the pipe's handle, valid while `pipe` lives, and the id this function's.
        if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle(), &mut id) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a plain call; the handle is owned and closed below.
        let process = owned(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, id) })?;
        let theirs = TokenOwner::of(process.as_raw_handle())?;
        // SAFETY: the current process's pseudo-handle.
        let ours = TokenOwner::of(unsafe { GetCurrentProcess() })?;
        // SAFETY: two SIDs of the live buffers.
        let same = unsafe { EqualSid(theirs.sid(), ours.sid()) } != 0;
        Ok((same, id))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::posix_acl_grants_others;

    /// One entry of a POSIX ACL as Linux keeps it.
    fn entry(tag: u16, permissions: u16, id: u32) -> Vec<u8> {
        [
            &tag.to_le_bytes()[..],
            &permissions.to_le_bytes(),
            &id.to_le_bytes(),
        ]
        .concat()
    }

    /// The owner's rights alone grant nothing to another account; a named user's or
    /// group's do, masked off or not, as do the owning group's and others'.
    #[test]
    fn a_posix_acl_grants_another_account_a_right() {
        let header = 2u32.to_le_bytes().to_vec();
        let owner_only = [
            header.clone(),
            entry(0x01, 7, u32::MAX),
            entry(0x04, 0, u32::MAX),
            entry(0x20, 0, u32::MAX),
        ]
        .concat();
        assert!(!posix_acl_grants_others(&owner_only));
        let named = [
            owner_only.clone(),
            entry(0x02, 1, 1001),
            entry(0x10, 0, u32::MAX),
        ]
        .concat();
        assert!(posix_acl_grants_others(&named));
        let group = [owner_only.clone(), entry(0x08, 4, 100)].concat();
        assert!(posix_acl_grants_others(&group));
        let others = [header, entry(0x01, 7, u32::MAX), entry(0x20, 1, u32::MAX)].concat();
        assert!(posix_acl_grants_others(&others));
        assert!(!posix_acl_grants_others(&[]));
    }

    /// Ending the activation with nothing bound, as on X11 or before any second start, does
    /// nothing, however often; with a display it needs a compositor, tested by hand.
    #[test]
    fn ending_an_activation_never_bound_does_nothing() {
        super::end_activation();
        super::end_activation();
    }

    /// A directory the test makes, with no ACL, grants nothing; one the system has not
    /// is an error.
    #[test]
    fn a_directory_without_an_acl_grants_nothing() {
        let folder = std::env::temp_dir().join(format!("apertrust-acl-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        assert!(!super::acl_grants_others(&folder).unwrap());
        std::fs::remove_dir(&folder).unwrap();
        assert!(super::acl_grants_others(&folder).is_err());
    }
}
