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

//! A dialog's window where winit cannot make it what it is (`docs/gui.md`, Dialogs and
//! alerts; decision 41): on macOS, presented as a sheet on the window it rises from, where
//! macOS 27 centres it, the window dimmed behind it, and kept out of other windows' tabs.
//! On Linux, [`set_parent`] sets a Wayland dialog's parent while the app keeps its modality.
//! Windows' owner disabling and X11's native modality are planned (`docs/gui.md`, Still to
//! build).
//!
//! AppKit animates a sheet in and out inside `beginSheet:completionHandler:` and
//! `endSheet:`, each in a run loop of its own, 177 to 206 ms on the M4 (spike D1). Called
//! inside winit's event handling, that time is the handler's: the window draws nothing new
//! and the engine's submit gate stays held throughout. So [`begin_sheet`] and [`end_sheet`]
//! only schedule the call, on the main run loop in its default mode, as [`crate::zoom_window`]
//! schedules a zoom: it runs once the event under way is handled, outside winit's handler,
//! which then delivers the windows' events while AppKit animates, and each calls back once
//! AppKit's call has returned.

use raw_window_handle::HasWindowHandle;

use crate::Error;

/// Presents `sheet`'s window as a sheet on `parent`'s (macOS), from the run loop once the
/// event under way is handled, and calls `begun` once AppKit has animated it in: AppKit
/// then keeps `parent`'s input from it and gives the sheet the focus. Both windows must
/// stay until then. Returns false where the platform has no sheets, and never calls
/// `begun` there.
pub fn begin_sheet(
    parent: &impl HasWindowHandle,
    sheet: &impl HasWindowHandle,
    begun: impl FnOnce() + 'static,
) -> Result<bool, Error> {
    let (parent, sheet) = (parent.window_handle()?, sheet.window_handle()?);
    #[cfg(target_os = "macos")]
    if let (
        raw_window_handle::RawWindowHandle::AppKit(parent),
        raw_window_handle::RawWindowHandle::AppKit(sheet),
    ) = (parent.as_raw(), sheet.as_raw())
    {
        return macos::begin(parent, sheet, begun).map(|()| true);
    }
    let _ = (parent, sheet, begun);
    Ok(false)
}

/// Ends the sheet [`begin_sheet`] presented (macOS), from the run loop once the event under
/// way is handled, and calls `ended` once AppKit has animated it away: `parent` takes its
/// input and the focus back. Both windows must stay until then, since AppKit closes no
/// window a sheet hangs on. Returns false where the platform has no sheets, and never calls
/// `ended` there.
pub fn end_sheet(
    parent: &impl HasWindowHandle,
    sheet: &impl HasWindowHandle,
    ended: impl FnOnce() + 'static,
) -> Result<bool, Error> {
    let (parent, sheet) = (parent.window_handle()?, sheet.window_handle()?);
    #[cfg(target_os = "macos")]
    if let (
        raw_window_handle::RawWindowHandle::AppKit(parent),
        raw_window_handle::RawWindowHandle::AppKit(sheet),
    ) = (parent.as_raw(), sheet.as_raw())
    {
        return macos::end(parent, sheet, ended).map(|()| true);
    }
    let _ = (parent, sheet, ended);
    Ok(false)
}

/// Keeps `window` out of other windows' tabs (macOS), before it first shows: with
/// System Settings' "Prefer tabs when opening documents" set to Always, AppKit would open
/// a dialog's window as a tab of the main window. Does nothing elsewhere.
pub fn keep_out_of_tabs(window: &impl HasWindowHandle) -> Result<(), Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::untabbed(handle);
    }
    let _ = handle;
    Ok(())
}

/// Shows `window`, still hidden, in front of the others without making it the key window
/// (macOS): winit's `set_visible(true)` is `makeKeyAndOrderFront:`, which would take the
/// keyboard from the window the person is working in. Returns false where the platform has
/// no such call and nothing was done: the caller shows the window as winit does. Windows'
/// (`SW_SHOWNOACTIVATE`) and Linux's way are planned (decision 41).
pub fn show_without_focus(window: &impl HasWindowHandle) -> Result<bool, Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::ordered_front(handle).map(|()| true);
    }
    let _ = handle;
    Ok(false)
}

/// Parents `dialog` to `parent` on Wayland, while the app keeps the dialog modal and both
/// windows independently movable. Returns false on X11.
#[cfg(target_os = "linux")]
pub fn set_parent(
    parent: &winit::window::Window,
    dialog: &winit::window::Window,
) -> Result<bool, Error> {
    linux::set_parent(parent, dialog)
}

#[cfg(target_os = "linux")]
mod linux {
    use raw_window_handle::{HasDisplayHandle as _, RawDisplayHandle};
    use wayland_client::{
        Connection, Proxy,
        backend::{Backend, InvalidId, ObjectId, WaylandError},
    };
    use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
    use winit::{platform::wayland::WindowExtWayland as _, window::Window};

    use crate::Error;

    pub(super) fn set_parent(parent: &Window, dialog: &Window) -> Result<bool, Error> {
        let (Some(parent_handle), Some(dialog_handle)) =
            (parent.xdg_toplevel(), dialog.xdg_toplevel())
        else {
            return Ok(false);
        };
        let RawDisplayHandle::Wayland(display) =
            parent.display_handle().map(|handle| handle.as_raw())?
        else {
            return Ok(false);
        };
        // SAFETY: the `wl_display` is winit's, connected for as long as its event loop runs,
        // which spans this call, made from inside it with both windows borrowed. The
        // `Backend` is a guest on it and, with the `Connection` over it, is dropped before
        // this returns, so the display outlives both.
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        // SAFETY: `xdg_toplevel` returned the window's own `xdg_toplevel` `wl_proxy`, which
        // winit manages through wayland-rs and destroys only as the window is dropped; the
        // window is borrowed for this whole call, beyond which neither the id nor the proxy
        // made from it is used.
        let parent = unsafe { toplevel(&connection, parent_handle.as_ptr()) }?;
        // SAFETY: as above, for the dialog's window.
        let dialog = unsafe { toplevel(&connection, dialog_handle.as_ptr()) }?;
        dialog.set_parent(Some(&parent));
        match connection.flush() {
            // Queued, not lost: winit's loop sends it with its own next flush.
            Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            result => result?,
        }
        Ok(true)
    }

    /// The `xdg_toplevel` at `pointer` as a proxy on `connection`.
    ///
    /// # Safety
    ///
    /// `pointer` is a live `xdg_toplevel` `wl_proxy` of `connection`'s display, and stays so
    /// while the proxy returned is used.
    unsafe fn toplevel(
        connection: &Connection,
        pointer: *mut std::ffi::c_void,
    ) -> Result<XdgToplevel, InvalidId> {
        // SAFETY: the caller's, above.
        let id = unsafe { ObjectId::from_ptr(XdgToplevel::interface(), pointer.cast()) }?;
        XdgToplevel::from_id(connection, id)
    }
}

#[cfg(target_os = "macos")]
pub(crate) mod macos {
    use std::cell::Cell;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSView, NSWindow, NSWindowTabbingMode};
    use objc2_foundation::{NSArray, NSDefaultRunLoopMode, NSRunLoop};
    use raw_window_handle::AppKitWindowHandle;

    use crate::Error;

    /// The window of the view `handle` names, on the main thread, where AppKit may be used.
    pub(crate) fn window(handle: AppKitWindowHandle) -> Result<Retained<NSWindow>, Error> {
        MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        // SAFETY: the handle comes from a live window, and this runs on the main thread.
        let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
        view.window().ok_or(Error::NoWindow)
    }

    /// Runs `call` from the main run loop in its default mode, once the event under way
    /// is handled, as `performSelector:withObject:afterDelay:` does: never inside a
    /// tracking loop, a menu's or a live resize's.
    ///
    /// Correctness relies on `-[NSRunLoop performInModes:block:]` waking the run loop as
    /// it schedules the block, which it does: the review measured the block run with no
    /// delay while the loop slept. `CFRunLoopPerformBlock` alone would not wake it, and the
    /// block would wait for the next event, a sheet hanging unopened or a Quit unanswered
    /// until the pointer moved.
    pub(crate) fn later(call: impl FnOnce() + 'static) -> Result<(), Error> {
        MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let call = Cell::new(Some(call));
        let block = RcBlock::new(move || {
            if let Some(call) = call.take() {
                call();
            }
        });
        // SAFETY: a constant Foundation defines.
        let default = unsafe { NSDefaultRunLoopMode };
        let modes = NSArray::from_slice(&[default]);
        // SAFETY: AppKit asks for a block that may be sent to another thread, since the
        // run loop may be another thread's. This one is the main thread's, and this runs
        // on the main thread, so the block runs on the thread that made it.
        unsafe { NSRunLoop::mainRunLoop().performInModes_block(&modes, &block) };
        Ok(())
    }

    pub(super) fn ordered_front(handle: AppKitWindowHandle) -> Result<(), Error> {
        let window = window(handle)?;
        window.orderFront(None);
        Ok(())
    }

    pub(super) fn begin(
        parent: AppKitWindowHandle,
        sheet: AppKitWindowHandle,
        begun: impl FnOnce() + 'static,
    ) -> Result<(), Error> {
        let (parent, sheet) = (window(parent)?, window(sheet)?);
        later(move || {
            parent.beginSheet_completionHandler(&sheet, None);
            begun();
        })
    }

    pub(super) fn end(
        parent: AppKitWindowHandle,
        sheet: AppKitWindowHandle,
        ended: impl FnOnce() + 'static,
    ) -> Result<(), Error> {
        let (parent, sheet) = (window(parent)?, window(sheet)?);
        later(move || {
            parent.endSheet(&sheet);
            ended();
        })
    }

    pub(super) fn untabbed(handle: AppKitWindowHandle) -> Result<(), Error> {
        window(handle)?.setTabbingMode(NSWindowTabbingMode::Disallowed);
        Ok(())
    }
}
