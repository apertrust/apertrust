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

//! The system's preference for less motion (`docs/gui.md`, The central panel's states):
//! with it on, the app cuts where it would fade.
//!
//! - **macOS:** Accessibility › Display › Reduce motion,
//!   `NSWorkspace.accessibilityDisplayShouldReduceMotion`.
//! - **Windows:** Settings › Accessibility › Visual effects › Animation effects off,
//!   `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION)`.
//! - **Linux:** not read: GNOME's `enable-animations` is a GSettings key, reached only
//!   through GIO or D-Bus, which the app does not link, so the app animates there.

/// Whether the system asks apps to reduce motion. Cheap enough to call as each animation
/// begins; false where the platform's preference is not read.
pub fn reduce_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
    }
    #[cfg(windows)]
    {
        windows::reduce_motion()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        false
    }
}

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::Foundation::TRUE;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
    };
    use windows_sys::core::BOOL;

    /// Client-area animations off: what Windows' Animation effects switch turns off.
    pub(super) fn reduce_motion() -> bool {
        let mut animate: BOOL = TRUE;
        // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL through the pointer, which
        // points at `animate`, alive for the call.
        let read = unsafe {
            SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut animate).cast(), 0)
        };
        read != 0 && animate == 0
    }
}
