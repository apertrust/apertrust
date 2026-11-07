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

//! Each platform's part of each [`Form`] (decision 41; `docs/gui.md`, Dialogs and alerts),
//! in one place: which form a dialog gets ([`form`]) and what its window is made with
//! ([`attributes`]), as far as winit 0.30 goes without `unsafe`. What winit cannot do is
//! `apertrust-display`'s, as the Mac's sheet is.
//!
//! | | macOS | Windows | Linux, X11 | Linux, Wayland |
//! | --- | --- | --- | --- | --- |
//! | Sheet | AppKit's sheet | As Modal | As Modal | As Modal |
//! | Modal | A window centred on the main window | The same, owned by the main window: above it, minimised with it, no taskbar button of its own | The same, of the dialog window type | The same, placed by the compositor |
//! | Modeless | A window where it was last left | The same | The same | Placed by the compositor |
//!
//! Under a modal dialog or a sheet the app keeps the main window's input from it on every
//! platform (the app's `raw_input_hook`), and a click on the main window brings a modal
//! window forward ([`super::Dialogs::raise`]). Still to build (decision 41; `docs/gui.md`,
//! Still to build): Windows' owner disabled while it is open (winit's `set_enable`); X11's
//! `WM_TRANSIENT_FOR` and `_NET_WM_STATE_MODAL`, which winit 0.30 does not set (its parent
//! window is a child confined to the parent, not a transient), and Wayland's
//! `xdg_toplevel.set_parent` and xdg-dialog's modal hint, which winit 0.30 does not offer,
//! both `apertrust-display`'s.

use eframe::egui::os::OperatingSystem;
use egui_winit::winit::window::{Window, WindowAttributes};

use super::Form;

/// The form a dialog that asks for `wanted` gets on `os`: a sheet only on the Mac, the one
/// platform with sheets, and elsewhere what a sheet becomes ([`unsheeted`]); a modal or a
/// modeless window as asked everywhere.
pub fn form(wanted: Form, os: OperatingSystem) -> Form {
    if os == OperatingSystem::Mac {
        wanted
    } else {
        unsheeted(wanted)
    }
}

/// What `form` becomes where no sheet can be presented, on Windows and Linux, or on a Mac
/// where AppKit refuses one: a modal window centred on the main window, which keeps its
/// input as a sheet does.
pub fn unsheeted(form: Form) -> Form {
    match form {
        Form::Sheet => Form::Modal,
        form => form,
    }
}

/// Whether the host can ask an alert on `os`: only the Mac's, `NSAlert`, is built. Windows'
/// task dialog and Linux's drawn alert are still to build (decision 41), so there an ask
/// answers [`super::Ask::NoAlerts`], and a dialog leaves out what would need one.
pub fn shows_alerts(os: OperatingSystem) -> bool {
    os == OperatingSystem::Mac
}

/// Whether the host draws a dialog window's frame on `os`: on Linux, where GNOME on
/// Wayland draws none and winit's fallback frame shows minimise and maximise as bare
/// squares, a strip at the window's top with its close button, in the main window's
/// caption style ([`super::draw`]). Its window then has no decorations, on X11 too, so
/// the frame is the same on every desktop. The Mac's and Windows' frames are the system's.
pub fn draws_frame(os: OperatingSystem) -> bool {
    os == OperatingSystem::Nix
}

/// `attributes` for the window of a dialog shown as `form` over `main`: on Linux no
/// decorations ([`draws_frame`]); and what the platform gives a modal one at creation, on
/// Windows its owner, the main window, on X11 the dialog window type, which Wayland
/// ignores. A modeless window takes nothing more there, nor the Mac's windows.
pub(super) fn attributes(
    attributes: WindowAttributes,
    form: Form,
    main: &Window,
) -> WindowAttributes {
    #[cfg(target_os = "linux")]
    let attributes = attributes.with_decorations(false);
    if !form.modal() {
        return attributes;
    }
    #[cfg(target_os = "windows")]
    let attributes = owned(attributes, main);
    #[cfg(target_os = "linux")]
    let attributes = {
        use egui_winit::winit::platform::x11::{WindowAttributesExtX11 as _, WindowType};
        attributes.with_x11_window_type(vec![WindowType::Dialog])
    };
    let _ = main;
    attributes
}

/// `attributes` owned by `main` (Windows): the window stays above it, is minimised with it
/// and has no taskbar button of its own, and the system gives `main` the activation back
/// as it is destroyed. winit takes the owner's handle as a number, with no `unsafe`.
#[cfg(target_os = "windows")]
fn owned(attributes: WindowAttributes, main: &Window) -> WindowAttributes {
    use egui_winit::winit::platform::windows::WindowAttributesExtWindows as _;
    use egui_winit::winit::raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
    match main.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(handle)) => attributes.with_owner_window(handle.hwnd.get()),
        Ok(_) => attributes,
        Err(e) => {
            tracing::warn!(target: "gpu", "dialog not owned by the main window: {e}");
            attributes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog_host::Dialog;
    use crate::{export, jobs, settings_dialog};

    const PLATFORMS: [OperatingSystem; 3] = [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ];

    /// The form `dialog` shows in on each of the three platforms.
    fn forms(dialog: &dyn Dialog) -> [Form; 3] {
        PLATFORMS.map(|os| form(dialog.form(os), os))
    }

    #[test]
    fn a_sheet_is_the_mac_s_and_a_modal_window_elsewhere() {
        assert_eq!(
            PLATFORMS.map(|os| form(Form::Sheet, os)),
            [Form::Sheet, Form::Modal, Form::Modal]
        );
        for wanted in [Form::Modal, Form::Modeless] {
            assert_eq!(PLATFORMS.map(|os| form(wanted, os)), [wanted; 3]);
        }
        // Where AppKit refuses a sheet, it is modal still: never modeless.
        assert_eq!(unsheeted(Form::Sheet), Form::Modal);
        assert_eq!(unsheeted(Form::Modeless), Form::Modeless);
    }

    #[test]
    fn only_the_mac_shows_alerts_so_far() {
        assert_eq!(PLATFORMS.map(shows_alerts), [true, false, false]);
    }

    #[test]
    fn the_host_draws_the_frame_on_linux_alone() {
        assert_eq!(PLATFORMS.map(draws_frame), [false, false, true]);
    }

    #[test]
    fn each_dialog_s_form_on_macos_windows_and_linux() {
        let export = export::Export::new(export::Values::default());
        assert_eq!(forms(&export), [Form::Sheet, Form::Modal, Form::Modal]);
        let settings = settings_dialog::Settings {
            rows: Vec::new(),
            page: apertrust_base::settings::Page::General,
            close_button: false,
        };
        // Apple's Settings windows are modeless; elsewhere a dialog is modal.
        assert_eq!(forms(&settings), [Form::Modeless, Form::Modal, Form::Modal]);
        let jobs = jobs::Window::new(Vec::new());
        assert_eq!(forms(&jobs), [Form::Modeless; 3]);
    }

    #[test]
    fn each_dialog_s_keys_on_macos_windows_and_linux() {
        use crate::dialog_host::Keys;
        let all = Keys {
            escape: true,
            close_window: true,
            cancel: true,
        };
        let export = export::Export::new(export::Values::default());
        assert_eq!(PLATFORMS.map(|os| export.keys(os)), [all; 3]);
        let settings = settings_dialog::Settings {
            rows: Vec::new(),
            page: apertrust_base::settings::Page::General,
            close_button: true,
        };
        // On the Mac, Apple's Settings window: Escape does nothing, and Cmd+W is Close
        // Window's, the menu's. Elsewhere Escape cancels it, as every modal dialog.
        let escape = Keys {
            escape: true,
            ..Keys::default()
        };
        assert_eq!(
            PLATFORMS.map(|os| settings.keys(os)),
            [Keys::default(), escape, escape]
        );
        let jobs = jobs::Window::new(Vec::new());
        assert_eq!(PLATFORMS.map(|os| jobs.keys(os)), [Keys::default(); 3]);
    }
}
