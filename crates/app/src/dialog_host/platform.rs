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
//! | A modal window's parent | Its sheet's window | Its owner | None set yet: no `WM_TRANSIENT_FOR` | `xdg_toplevel.set_parent` ([`made`]) |
//! | An alert | AppKit's `NSAlert`, a sheet | None yet: no task dialog | Drawn by the host in a modal window ([`draws_alerts`]) | As X11 |
//! | Frame | AppKit's | Drawn by the host ([`draws_frame`]) | Drawn by the host | Drawn by the host |
//! | Border and shadow | AppKit's | The system's undecorated shadow | None: a transparent margin can show black without a compositing manager | Drawn by the host round a transparent margin ([`shadowed`]) |
//! | Bringing a window forward | `focus_window` | The same | The same | An xdg-activation request ([`activate`]) |
//!
//! Under a modal dialog or a sheet the app keeps the main window's input from it on every
//! platform (the app's `raw_input_hook`), and a click on the main window brings a modal
//! window forward ([`super::Dialogs::raise`]). Still to build (decision 41; `docs/gui.md`,
//! Still to build): Windows' owner disabled while it is open (winit's `set_enable`), and
//! X11's `WM_TRANSIENT_FOR` and `_NET_WM_STATE_MODAL`, which winit 0.30 does not set (its
//! parent window is a child confined to the parent, not a transient), `apertrust-display`'s.

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

/// Whether the host can ask an alert on `os`: the Mac's `NSAlert`, and the alert it draws
/// itself where it does ([`draws_alerts`]). Windows' task dialog is still to build (decision
/// 41), so there an ask answers [`super::Ask::NoAlerts`], and a dialog leaves out what would
/// need one.
pub fn shows_alerts(os: OperatingSystem) -> bool {
    os == OperatingSystem::Mac || draws_alerts(os)
}

/// Whether the host draws an alert itself on `os`, `components::AlertBody` in a modal window
/// of its own, rather than asking the system's: on Linux, which has no system alert for an
/// app outside GTK and Qt (decision 41).
pub fn draws_alerts(os: OperatingSystem) -> bool {
    os == OperatingSystem::Nix
}

/// Whether a dialog's window has a border and a drop shadow the host draws round a
/// transparent outer margin, on `os`, where the app runs on Wayland (`wayland`, from the
/// main window, [`wayland`]): every hosted window on Linux under Wayland, modal or
/// modeless, whose window has no decorations ([`draws_frame`]). Not on X11, where the
/// transparent margin can show black without a compositing manager. The Mac's sheets and
/// windows are AppKit's, and Windows' have the system's undecorated shadow ([`attributes`]).
pub fn shadowed(os: OperatingSystem, wayland: bool) -> bool {
    os == OperatingSystem::Nix && wayland
}

/// Whether the host draws a dialog window's frame on `os`: a strip at the window's top with
/// its close button, in the main window's caption style, and the window's title but an
/// alert's ([`super::draw`]). On Windows, where the main window's top panel is its title
/// bar, so a dialog's frame matches it, with no icon and no system caption; on Linux,
/// where GNOME on Wayland draws none and winit's fallback frame shows minimise and maximise
/// as bare squares. Its window then has no decorations, on X11 too, so the frame is the
/// same on every desktop. The Mac's frames are the system's.
pub fn draws_frame(os: OperatingSystem) -> bool {
    matches!(os, OperatingSystem::Windows | OperatingSystem::Nix)
}

/// `attributes` for the window of a dialog shown as `form` over `main`: on Windows and
/// Linux no decorations ([`draws_frame`]), on Windows with the system's undecorated shadow
/// and Windows 11's rounded corners, as the main window's; and what the platform gives a
/// modal one at creation, on Windows its owner, the main window, on X11 the dialog window
/// type, which Wayland ignores. A modeless window takes nothing more there, nor the Mac's
/// windows.
pub(super) fn attributes(
    attributes: WindowAttributes,
    form: Form,
    main: &Window,
) -> WindowAttributes {
    #[cfg(target_os = "linux")]
    let attributes = attributes.with_decorations(false);
    #[cfg(target_os = "windows")]
    let attributes = {
        use egui_winit::winit::platform::windows::WindowAttributesExtWindows as _;
        attributes
            .with_decorations(false)
            .with_undecorated_shadow(true)
    };
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

/// What the platform gives the window of a dialog shown as `form` over `parent` once it is
/// made, where winit cannot: on Wayland a modal one's parent, `xdg_toplevel.set_parent`
/// (`apertrust_display::set_parent`), its modality kept by the app. Nothing elsewhere, nor
/// for a modeless window.
pub(super) fn made(window: &Window, form: Form, parent: &Window) {
    #[cfg(target_os = "linux")]
    if form.modal()
        && let Err(e) = apertrust_display::set_parent(parent, window)
    {
        tracing::warn!(target: "gpu", "Wayland dialog parent not set: {e}");
    }
    let _ = (window, form, parent);
}

/// Opens a hosted window's window menu at `at`, in the window's pixels, for a right-click
/// on the frame the host draws: on Linux the compositor's or window manager's. Nothing on
/// Windows, where winit builds the menu itself with Minimise always enabled and Maximise
/// for a window the person resizes, which a dialog has neither of.
pub(super) fn window_menu(window: &Window, at: egui_winit::winit::dpi::PhysicalPosition<f64>) {
    #[cfg(not(target_os = "windows"))]
    window.show_window_menu(at);
    #[cfg(target_os = "windows")]
    let _ = (window, at);
}

/// Brings a hosted window forward when the user asks, with winit's focus call. On Wayland,
/// where that call does nothing, the window asks the compositor to activate it instead,
/// through xdg-activation; a compositor may only mark it as wanting attention.
pub(super) fn activate(window: &Window) {
    window.focus_window();
    #[cfg(target_os = "linux")]
    if wayland(window) {
        use egui_winit::winit::window::UserAttentionType;
        window.request_user_attention(Some(UserAttentionType::Critical));
    }
}

/// Whether `window` is on Wayland rather than X11, known only once a window is up: where
/// winit's `focus_window` does nothing ([`activate`]) and dialogs are shadowed
/// ([`shadowed`]). Never off Linux.
pub(super) fn wayland(window: &Window) -> bool {
    #[cfg(target_os = "linux")]
    {
        use egui_winit::winit::raw_window_handle::{HasDisplayHandle as _, RawDisplayHandle};
        matches!(
            window.display_handle().map(|handle| handle.as_raw()),
            Ok(RawDisplayHandle::Wayland(_))
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        false
    }
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
    fn the_mac_and_linux_show_alerts() {
        assert_eq!(PLATFORMS.map(shows_alerts), [true, false, true]);
    }

    #[test]
    fn the_host_draws_alerts_on_linux_alone() {
        assert_eq!(PLATFORMS.map(draws_alerts), [false, false, true]);
    }

    #[test]
    fn hosted_windows_have_a_shadow_and_border_on_wayland_alone() {
        assert_eq!(PLATFORMS.map(|os| shadowed(os, true)), [false, false, true]);
        // On X11, none.
        assert_eq!(PLATFORMS.map(|os| shadowed(os, false)), [false; 3]);
    }

    #[test]
    fn the_host_draws_the_frame_on_windows_and_linux() {
        assert_eq!(PLATFORMS.map(draws_frame), [false, true, true]);
    }

    #[test]
    fn each_dialog_s_form_on_macos_windows_and_linux() {
        let export = export::Export::new(export::Values::default());
        assert_eq!(forms(&export), [Form::Sheet, Form::Modal, Form::Modal]);
        let (keeper, _) = crate::settings::Keeper::nowhere();
        let settings = settings_dialog::Settings::new(&keeper, false);
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
        let (keeper, _) = crate::settings::Keeper::nowhere();
        let settings = settings_dialog::Settings::new(&keeper, true);
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
