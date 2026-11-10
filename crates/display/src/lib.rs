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

//! The window's platform code: tags the window surface's colour space where wgpu does
//! not (`docs/gui.md`, Display colour), and does for the window's
//! frame what winit cannot (`docs/gui.md`, The window frame and menus; decision 39).
//!
//! On macOS, wgpu 30 leaves the Metal layer of an sRGB surface without a colour space,
//! and resets it to none whenever it configures the surface. macOS shows an untagged
//! layer without colour management, so on a wide-gamut display every colour, the UI's
//! included, comes out oversaturated (the viewport spike). [`tag_window`] gives
//! the layer its colour space back. Other platforms need nothing yet.
//!
//! On macOS the app's top bar is the window's title bar: [`place_window_buttons`] centres
//! the traffic lights in it, [`content_never_moves_window`] and [`start_window_drag`]
//! leave moving the window to the app, [`title_bar_double_click`] reads what a
//! double-click on it should do, and [`zoom_window`] zooms.
//! Windows' and Linux's frames are still the system's.
//!
//! On macOS the menu bar is muda's, and for what muda cannot do
//! [`no_system_full_screen_item`] keeps AppKit's own full-screen item out of it and
//! [`handling_key_repeat`] tells a menu item chosen by a key's repeat.
//!
//! On macOS a dialog's window can be a sheet on the main window (decision 41):
//! [`begin_sheet`] and [`end_sheet`] present it and take it away from the run loop, and
//! [`keep_out_of_tabs`] keeps a dialog's own window from opening as a tab. A confirmation is
//! the system's alert, on the Mac an `NSAlert` as a sheet ([`begin_alert`]); Linux's is
//! drawn by the app's dialog host instead.
//!
//! On Wayland a modal dialog's window takes the window it rises from as its parent,
//! `xdg_toplevel.set_parent`, which winit 0.30 does not offer: `set_parent` sends it
//! through wayland-client over winit's own display and toplevels, the app keeping the
//! dialog modal.
//!
//! On macOS the popup service's plain lists are AppKit's menus (decision 40):
//! [`pop_up_menu`] shows one from the run loop and answers with the item chosen.
//!
//! On macOS the files Finder opens with the app, by a double-click, Open With or a drop on
//! the Dock icon, come as an open-documents event that winit 0.30 does not deliver:
//! [`receive_opened_files`] receives them and [`take_opened_files`] hands them to the app;
//! a click on the Dock icon of the running app likewise, through [`receive_reopen`] and
//! [`take_reopened`].
//!
//! On Windows the app is a windowed program, with no console of its own:
//! [`attach_to_terminal`] attaches it to the terminal it was started from, so what it
//! prints shows there.

#![allow(
    unsafe_code,
    reason = "Objective-C calls to tag the window's Metal layer, place its buttons and start its moves, register a default for the menu bar, present sheets and popup menus from the run loop, and methods given to AppKit's and winit's classes, the open-documents and reopen events' among them; on Windows, the console calls that attach the app to its terminal; on Wayland, wayland-client's views of winit's display and toplevels, to set a dialog's parent"
)]

mod alert;
mod console;
mod dialog;
mod menu_bar;
mod opened;
mod popup_menu;
mod title_bar;

pub use alert::{AlertButton, ButtonRole, begin_alert};
pub use console::attach_to_terminal;
#[cfg(target_os = "linux")]
pub use dialog::set_parent;
pub use dialog::{begin_sheet, end_sheet, keep_out_of_tabs, show_without_focus};
pub use menu_bar::{handling_key_repeat, no_system_full_screen_item};
pub use opened::{
    on_opened_files, on_reopen, receive_opened_files, receive_reopen, take_opened_files,
    take_reopened,
};
pub use popup_menu::{Appearance, KeyEquivalent, MenuEntry, MenuItem, PopUpMenu, pop_up_menu};
pub use title_bar::{
    DoubleClick, DragEvent, DragStart, WindowButtons, content_never_moves_window,
    place_window_buttons, start_window_drag, title_bar_double_click, zoom_window,
};

use raw_window_handle::{HandleError, HasWindowHandle};

/// The colour space the window's contents are encoded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Srgb,
    DisplayP3,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no window handle: {0}")]
    Handle(#[from] HandleError),
    #[cfg(target_os = "linux")]
    #[error("Wayland object: {0}")]
    WaylandObject(#[from] wayland_client::backend::InvalidId),
    #[cfg(target_os = "linux")]
    #[error("Wayland connection: {0}")]
    WaylandConnection(#[from] wayland_client::backend::WaylandError),
    #[error("no Metal layer")]
    NoMetalLayer,
    #[error("the view is in no window")]
    NoWindow,
    #[error("off the main thread, where AppKit cannot be used")]
    NotMainThread,
    #[error("the content still moves the window")]
    ContentMovesWindow,
    #[error("the content is not winit's view")]
    UnknownView,
    #[error("no mouse event to start the drag with")]
    NoEvent,
    #[error("winit's class has its own {0}, so winit decides")]
    OwnMethod(&'static str),
    #[error("the application has no delegate")]
    NoDelegate,
    #[error("the application's delegate is not winit's")]
    UnknownDelegate,
}

/// Tags the window's surface as holding `space`, so the system colour-manages it to the
/// display. wgpu clears the tag whenever it configures the surface, on every resize
/// too, so call this every frame; it changes nothing while the tag is right, and does
/// nothing on platforms that need no tag.
pub fn tag_window(window: &impl HasWindowHandle, space: ColorSpace) -> Result<(), Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::tag(handle, space);
    }
    let _ = (handle, space);
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::rc::Retained;
    use objc2_app_kit::NSView;
    use objc2_core_foundation::CFString;
    use objc2_core_graphics::{CGColorSpace, kCGColorSpaceDisplayP3, kCGColorSpaceSRGB};
    use objc2_quartz_core::{CALayer, CAMetalLayer};
    use raw_window_handle::AppKitWindowHandle;

    use super::{ColorSpace, Error};

    pub(super) fn tag(handle: AppKitWindowHandle, space: ColorSpace) -> Result<(), Error> {
        // SAFETY: the handle comes from a live window, and eframe calls the app on the
        // main thread, where AppKit objects may be used.
        let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
        let layer = metal_layer(view).ok_or(Error::NoMetalLayer)?;
        // SAFETY: the colour space names are constants CoreGraphics defines.
        let name: &CFString = unsafe {
            match space {
                ColorSpace::Srgb => kCGColorSpaceSRGB,
                ColorSpace::DisplayP3 => kCGColorSpaceDisplayP3,
            }
        };
        let current = layer
            .colorspace()
            .and_then(|current| CGColorSpace::name(Some(&current)));
        if current.as_deref() != Some(name) {
            let colorspace = CGColorSpace::with_name(Some(name));
            layer.setColorspace(colorspace.as_deref());
        }
        Ok(())
    }

    /// The Metal layer wgpu draws into: the view's own layer, or, as wgpu attaches it,
    /// one of its sublayers.
    fn metal_layer(view: &NSView) -> Option<Retained<CAMetalLayer>> {
        let layer: Retained<CALayer> = view.layer()?;
        if let Ok(metal) = layer.clone().downcast::<CAMetalLayer>() {
            return Some(metal);
        }
        // SAFETY: reading a live layer's sublayers on the main thread.
        let sublayers = unsafe { layer.sublayers() }?;
        sublayers
            .iter()
            .find_map(|sublayer| sublayer.downcast::<CAMetalLayer>().ok())
    }
}
