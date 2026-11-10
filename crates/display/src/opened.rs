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

//! What the system asks the app to open: files (`docs/packaging.md`, Opening a raw from the
//! system), and on macOS the app itself again, by a click on its Dock icon (`docs/gui.md`,
//! The start window).
//!
//! On macOS a double-click in Finder, Open With and a drop on the Dock icon reach the app
//! as an open-documents event, never as arguments, at its launch or while it runs. AppKit
//! hands it to the application delegate's `application:openURLs:`, which winit 0.30's
//! delegate does not have: [`receive_opened_files`] gives winit's delegate class that
//! method, before the event loop runs, so the files of the launch arrive too. Each event's
//! files are kept until the app takes them with [`take_opened_files`], the latest event's
//! only, and [`on_opened_files`] wakes the app as they come.
//!
//! A click on the Dock icon of a running app reaches the delegate's
//! `applicationShouldHandleReopen:hasVisibleWindows:`, which winit 0.30's delegate does not
//! have either: [`receive_reopen`] gives winit's class that method, which notes the click
//! for [`take_reopened`] and wakes the app ([`on_reopen`]), then lets AppKit do what it
//! does by default, bringing a minimised window back.
//!
//! Windows and Linux pass the files as arguments (`--in-folder`), which the app reads
//! itself; there nothing arrives here, and no app runs without a window.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// The files of the latest event not yet taken, and what wakes the app.
struct Opened {
    files: Option<Vec<PathBuf>>,
    wake: Option<Box<dyn Fn() + Send>>,
}

static OPENED: Mutex<Opened> = Mutex::new(Opened {
    files: None,
    wake: None,
});

fn opened() -> std::sync::MutexGuard<'static, Opened> {
    OPENED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Keeps `files`, an event's, in place of those of an earlier one not yet taken, and
/// wakes the app. An event with no file keeps nothing.
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn deliver(files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    let mut opened = opened();
    opened.files = Some(files);
    if let Some(wake) = &opened.wake {
        wake();
    }
}

/// The files the system last asked the app to open, once each, in the order it gave them;
/// none where it has asked nothing since.
pub fn take_opened_files() -> Option<Vec<PathBuf>> {
    opened().files.take()
}

/// `wake` is called as files arrive, so the app takes them though it is idle; it is
/// called on the main thread, with nothing of this module's held but its lock.
pub fn on_opened_files(wake: impl Fn() + Send + 'static) {
    opened().wake = Some(Box::new(wake));
}

/// Whether the Dock icon was clicked since the last call, and what wakes the app then.
static REOPENED: AtomicBool = AtomicBool::new(false);
static REOPEN_WAKE: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

/// Notes a click on the Dock icon, and wakes the app.
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn reopened() {
    REOPENED.store(true, Ordering::SeqCst);
    let wake = REOPEN_WAKE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(wake) = wake.as_ref() {
        wake();
    }
}

/// Whether the Dock icon was clicked since the last call, once each.
pub fn take_reopened() -> bool {
    REOPENED.swap(false, Ordering::SeqCst)
}

/// `wake` is called as the Dock icon is clicked, so the app looks though it is idle and has
/// no window shown; it is called on the main thread.
pub fn on_reopen(wake: impl Fn() + Send + 'static) {
    *REOPEN_WAKE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(wake));
}

/// Makes the app hear a click on its Dock icon while it runs, on macOS; does nothing
/// elsewhere. To be called once, as [`receive_opened_files`] is.
pub fn receive_reopen() -> Result<(), crate::Error> {
    #[cfg(target_os = "macos")]
    return macos::receive_reopen();
    #[cfg(not(target_os = "macos"))]
    Ok(())
}

/// Makes the app receive the files the system asks it to open, on macOS; does nothing
/// elsewhere. To be called once, after winit's event loop is made, which sets its
/// delegate, and before it runs, which finishes the launch and delivers the launch's files.
pub fn receive_opened_files() -> Result<(), crate::Error> {
    #[cfg(target_os = "macos")]
    return macos::receive();
    #[cfg(not(target_os = "macos"))]
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;

    use std::ffi::{CStr, CString};

    use objc2::Encode as _;
    use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, NSObjectProtocol as _, Sel};
    use objc2::{MainThreadMarker, sel};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSArray, NSURL};

    use crate::Error;

    /// Gives the application delegate's class, winit's, `application:openURLs:`.
    pub(super) fn receive() -> Result<(), Error> {
        let opened: extern "C-unwind" fn(&AnyObject, Sel, &AnyObject, &NSArray<NSURL>) = opened;
        // SAFETY: an IMP is any method's function pointer, called with the signature the
        // encoding gives, `opened`'s: no result, self, the selector, the application and
        // the array of URLs.
        let imp: Imp = unsafe { std::mem::transmute(opened) };
        add(
            sel!(application:openURLs:),
            "application:openURLs:",
            imp,
            c"v@:@@",
        )
    }

    /// Gives the application delegate's class, winit's,
    /// `applicationShouldHandleReopen:hasVisibleWindows:`.
    pub(super) fn receive_reopen() -> Result<(), Error> {
        let reopen: extern "C-unwind" fn(&AnyObject, Sel, &AnyObject, Bool) -> Bool = reopen;
        // SAFETY: as `receive`'s, with `reopen`'s signature: a BOOL, self, the selector, the
        // application and a BOOL, whose encoding is the platform's (`c` or `B`).
        let imp: Imp = unsafe { std::mem::transmute(reopen) };
        let types = format!("{0}@:@{0}", Bool::ENCODING);
        let name = "applicationShouldHandleReopen:hasVisibleWindows:";
        let types = CString::new(types).map_err(|_| Error::OwnMethod(name))?;
        add(
            sel!(applicationShouldHandleReopen:hasVisibleWindows:),
            name,
            imp,
            &types,
        )
    }

    /// Gives winit's application delegate class the method `imp`, named `name`, with the
    /// type encoding `types`, then sets the delegate again, so AppKit, which may note what a
    /// delegate answers to as it is set, sees the method.
    fn add(selector: Sel, name: &'static str, imp: Imp, types: &CStr) -> Result<(), Error> {
        let mtm = MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let app = NSApplication::sharedApplication(mtm);
        let delegate = app.delegate().ok_or(Error::NoDelegate)?;
        // winit's class by name, never another's: a delegate that is not winit's is left
        // as it is, and nothing is received (`main.rs` warns on `control`).
        let class = AnyClass::get(c"WinitApplicationDelegate")
            .filter(|class| delegate.isKindOfClass(class))
            .ok_or(Error::UnknownDelegate)?;
        // SAFETY: the method goes on winit's delegate class, which does not define it
        // (`class_addMethod` adds nothing to a class that does), with the encoding its
        // function's signature has.
        let added = unsafe {
            let class = std::ptr::from_ref::<AnyClass>(class).cast_mut();
            objc2::ffi::class_addMethod(class, selector, imp, types.as_ptr())
        };
        if !added.as_bool() {
            return Err(Error::OwnMethod(name));
        }
        app.setDelegate(None);
        app.setDelegate(Some(&delegate));
        Ok(())
    }

    /// The delegate's `applicationShouldHandleReopen:hasVisibleWindows:`: the click noted for
    /// the app, which shows its start window where it has no window shown; yes, so AppKit
    /// still brings a minimised window back.
    extern "C-unwind" fn reopen(_: &AnyObject, _: Sel, _: &AnyObject, _: Bool) -> Bool {
        super::reopened();
        Bool::YES
    }

    /// The delegate's `application:openURLs:`: the files among `urls`, kept for the app.
    extern "C-unwind" fn opened(_: &AnyObject, _: Sel, _: &AnyObject, urls: &NSArray<NSURL>) {
        super::deliver(files(urls));
    }

    /// The paths of the file URLs among `urls`, in their order; any other URL is left out.
    pub(super) fn files(urls: &NSArray<NSURL>) -> Vec<PathBuf> {
        urls.iter()
            .filter(|url| url.isFileURL())
            .filter_map(|url| url.path())
            .map(|path| PathBuf::from(path.to_string()))
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use objc2_foundation::{NSArray, NSString, NSURL};

        use super::files;

        /// Finder's files as URLs: each file's path, in order, a web address left out.
        #[test]
        fn the_file_urls_are_the_files_in_their_order() {
            let file = |path: &str| NSURL::fileURLWithPath(&NSString::from_str(path));
            let web =
                NSURL::URLWithString(&NSString::from_str("https://apertrust.org")).expect("a URL");
            let urls = NSArray::from_retained_slice(&[
                file("/Users/me/Pictures/Lisbon/IMG_2.CR3"),
                web,
                file("/Users/me/Pictures/Lisbon/IMG 1.CR3"),
            ]);
            assert_eq!(
                files(&urls),
                [
                    "/Users/me/Pictures/Lisbon/IMG_2.CR3",
                    "/Users/me/Pictures/Lisbon/IMG 1.CR3"
                ]
                .map(std::path::PathBuf::from)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{deliver, on_opened_files, on_reopen, reopened, take_opened_files, take_reopened};

    /// The latest event's files, once, the app woken for each event with files; an event
    /// with none keeps nothing. One test, since the files are kept for the whole process.
    #[test]
    fn the_latest_event_s_files_are_taken_once_and_wake_the_app() {
        let woken = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&woken);
        on_opened_files(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(take_opened_files(), None);
        let paths = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
        deliver(paths(&["/a/IMG_1.CR3"]));
        deliver(paths(&["/b/IMG_2.CR3", "/b/IMG_3.CR3"]));
        deliver(Vec::new());
        assert_eq!(woken.load(Ordering::SeqCst), 2);
        assert_eq!(
            take_opened_files(),
            Some(paths(&["/b/IMG_2.CR3", "/b/IMG_3.CR3"]))
        );
        assert_eq!(take_opened_files(), None);
    }

    /// A Dock click, noted once and waking the app each time. One test, since the click is
    /// kept for the whole process.
    #[test]
    fn a_click_on_the_dock_icon_is_taken_once_and_wakes_the_app() {
        let woken = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&woken);
        on_reopen(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        });
        assert!(!take_reopened());
        reopened();
        reopened();
        assert_eq!(woken.load(Ordering::SeqCst), 2);
        assert!(take_reopened());
        assert!(!take_reopened());
    }
}
