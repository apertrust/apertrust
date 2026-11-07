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

//! The window's frame where the app's top bar is its title bar (`docs/gui.md`, The window
//! frame and menus; decision 39), as far as winit cannot do it: on macOS the traffic
//! lights placed in the top bar, the window moved only from the top bar's empty areas,
//! what a double-click on a title bar does by the user's setting, and zooming from the
//! run loop, so each step of the animation is drawn. Elsewhere these do nothing yet.

use raw_window_handle::HasWindowHandle;

use crate::Error;

/// What double-clicking a window's title bar does: on macOS the user's choice under
/// Desktop & Dock, "Double-click a window's title bar to".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleClick {
    /// Zoom the window, or return it to its size before.
    Zoom,
    /// Minimise it into the Dock.
    Minimise,
    Nothing,
}

impl DoubleClick {
    /// The action macOS's global defaults name. `AppleActionOnDoubleClick` is `Maximize`
    /// (zoom), `Minimize`, `None`, or since macOS 15 `Fill`, which has no API of its own
    /// and is taken as zoom: for a window with no size of its own to zoom to, zooming
    /// fills the screen too. Where it is unset, as until the user changes it,
    /// `AppleMiniaturizeOnDoubleClick` (`minimise`), the setting before OS X 10.11,
    /// decides between minimising and zooming. Anything else zooms.
    pub fn from_defaults(action: Option<&str>, minimise: bool) -> DoubleClick {
        match action {
            Some("Minimize") => DoubleClick::Minimise,
            Some("None") => DoubleClick::Nothing,
            Some(_) => DoubleClick::Zoom,
            None if minimise => DoubleClick::Minimise,
            None => DoubleClick::Zoom,
        }
    }
}

/// What double-clicking a title bar does, as the user's setting says now, read each
/// time so a change in System Settings applies at once; zoom where the system has no
/// such setting. Windows and Linux map zoom to maximising or restoring once their
/// frames are built (decision 39).
pub fn title_bar_double_click() -> DoubleClick {
    #[cfg(target_os = "macos")]
    return macos::double_click();
    #[cfg(not(target_os = "macos"))]
    DoubleClick::Zoom
}

/// Where the window's own buttons are once [`place_window_buttons`] has run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowButtons {
    /// The top bar is not the title bar: in full screen, which hides the system's title
    /// bar and its buttons with it, and on platforms whose title bar the system draws.
    None,
    /// Centred in the top bar. The room they take from the window's left edge, in the
    /// window system's points, with a margin after them as wide as the one before; none
    /// where the window hides them.
    Placed(f32),
    /// They show, but the window's views are not as this version of the code knows them,
    /// so they stay where AppKit put them. The room they take there.
    Unplaced(f32),
}

/// Places the window's own buttons (close, minimise and zoom) in the app's top bar,
/// which runs to the window's top edge and is `height` points tall, in the window
/// system's points (egui's points times the interface's scale): centred in it, the close
/// button's centre as far from the window's left edge as from its top, the others after
/// it as the system spaces them.
///
/// The system lays the buttons out again whenever the window resizes, enters or leaves
/// full screen or moves to another display, so call this every frame: it changes
/// nothing while they are in place.
pub fn place_window_buttons(
    window: &impl HasWindowHandle,
    height: f32,
) -> Result<WindowButtons, Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::place(handle, f64::from(height));
    }
    let _ = (handle, height);
    Ok(WindowButtons::None)
}

/// Keeps the system from moving the window by itself when a press on the app's content
/// is dragged (macOS). Under a transparent title bar AppKit moves the window from any
/// view that lets it, and winit's lets it, so a drag that began on a button in the top
/// bar would move the window. Afterwards winit's view lets it move only while
/// [`start_window_drag`] runs, which the app calls for a drag that began on the top
/// bar's empty areas (decision 39), and keeps each press's mouse-down for it. Call it
/// once the window exists; elsewhere it does nothing.
pub fn content_never_moves_window(window: &impl HasWindowHandle) -> Result<(), Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::content_stays(handle);
    }
    let _ = handle;
    Ok(())
}

/// What [`start_window_drag`] handed the system, for a debug line.
#[derive(Debug, Clone, PartialEq)]
pub struct DragStart {
    /// The event handed over.
    pub handed: DragEvent,
    /// The event AppKit was handling, by its type ("left mouse dragged"), or "none".
    pub current: String,
    /// Where the event handed over is, in the window's points from its bottom left.
    pub at: (f64, f64),
}

/// Which event [`start_window_drag`] handed AppKit's `performWindowDragWithEvent:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEvent {
    /// The press's own mouse-down, as AppKit asks for.
    Press,
    /// The left-button event AppKit was handling, where the press's was not kept.
    Current,
    /// A mouse-down built where the pointer is, where AppKit was handling another kind.
    Built,
}

impl std::fmt::Display for DragEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DragEvent::Press => "press's mouse-down",
            DragEvent::Current => "AppKit's event",
            DragEvent::Built => "built mouse-down",
        })
    }
}

/// Hands moving the window to the system, from a drag that began on the top bar's empty
/// areas (macOS): AppKit's `performWindowDragWithEvent:` with the press's own mouse-down,
/// which winit's view keeps once [`content_never_moves_window`] has run, else the
/// left-button event AppKit is handling, else a mouse-down built where the pointer is,
/// while winit's view lets the window move. Call it at the end of the app's frame, once
/// the widgets have drawn and before eframe paints, where egui's `StartDrag` would run;
/// it returns at once, the system then moving the window, which may keep the press's
/// release from the app. Returns what it handed over, or `None` where the platform has no
/// such call, for the app to send `StartDrag` instead.
pub fn start_window_drag(window: &impl HasWindowHandle) -> Result<Option<DragStart>, Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::drag(handle).map(Some);
    }
    let _ = handle;
    Ok(None)
}

/// Zooms the window, or returns it to its size before, as the Window menu's Zoom does,
/// from the run loop once the current event is handled (macOS). AppKit animates a zoom
/// within the call, and called from inside winit's event handling, as egui's
/// `Maximized` is, winit can deliver no resize nor redraw until it ends, so the window
/// shows its last frame stretched all the way. Returns false where the platform has no
/// such call, for the app to send `Maximized` instead.
pub fn zoom_window(window: &impl HasWindowHandle) -> Result<bool, Error> {
    let handle = window.window_handle()?;
    #[cfg(target_os = "macos")]
    if let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() {
        return macos::zoom(handle).map(|()| true);
    }
    let _ = handle;
    Ok(false)
}

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::{Cell, RefCell};
    use std::ffi::CString;
    use std::sync::OnceLock;
    use std::thread::LocalKey;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, NSObjectProtocol as _, Sel};
    use objc2::{Encode as _, MainThreadMarker, Message as _, sel};
    use objc2_app_kit::{
        NSApplication, NSButton, NSEvent, NSEventModifierFlags, NSEventType, NSView, NSWindow,
        NSWindowButton, NSWindowStyleMask,
    };
    use objc2_foundation::{
        NSObjectNSDelayedPerforming as _, NSPoint, NSRect, NSSize, NSString, NSUserDefaults,
    };
    use raw_window_handle::AppKitWindowHandle;

    use super::{DoubleClick, DragEvent, DragStart, WindowButtons};
    use crate::Error;

    thread_local! {
        /// Whether winit's view lets AppKit move the window: only while `drag` runs.
        static MOVES: Cell<bool> = const { Cell::new(false) };
        /// The last press's mouse-down in winit's view, which `drag` hands AppKit.
        static PRESS: RefCell<Option<Retained<NSEvent>>> = const { RefCell::new(None) };
        /// The window whose buttons `place` last placed, by address, and the top bar's
        /// height, which its frame's changes put them back for (`keep_placed`).
        static PLACED: Cell<Option<(usize, f64)>> = const { Cell::new(None) };
        /// Whether the buttons are being placed, so placing them, which lays out the
        /// title bar, never starts again within itself.
        static PLACING: Cell<bool> = const { Cell::new(false) };
    }

    /// `-[NSWindow setFrame:display:]` as winit's window class had it, and the title
    /// bar container's `layout`, before `keep_placed` replaced them.
    static SET_FRAME: OnceLock<Imp> = OnceLock::new();
    static LAYOUT: OnceLock<Imp> = OnceLock::new();
    /// winit's view's `mouseDown:`, before `content_stays` replaced it.
    static MOUSE_DOWN: OnceLock<Imp> = OnceLock::new();

    pub(super) fn double_click() -> DoubleClick {
        let defaults = NSUserDefaults::standardUserDefaults();
        let action = defaults.stringForKey(&NSString::from_str("AppleActionOnDoubleClick"));
        let minimise = defaults.boolForKey(&NSString::from_str("AppleMiniaturizeOnDoubleClick"));
        DoubleClick::from_defaults(action.map(|a| a.to_string()).as_deref(), minimise)
    }

    /// The view `handle` names, on the main thread, where AppKit may be used.
    fn view(handle: &AppKitWindowHandle) -> Result<&NSView, Error> {
        MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        // SAFETY: the handle comes from a live window, and this runs on the main thread.
        Ok(unsafe { handle.ns_view.cast().as_ref() })
    }

    /// The window of the view `handle` names.
    pub(super) fn window(handle: AppKitWindowHandle) -> Result<Retained<NSWindow>, Error> {
        view(&handle)?.window().ok_or(Error::NoWindow)
    }

    pub(super) fn zoom(handle: AppKitWindowHandle) -> Result<(), Error> {
        let window = window(handle)?;
        // SAFETY: `zoom:` is NSWindow's, and takes the sender, an object or nil.
        unsafe { window.performSelector_withObject_afterDelay(sel!(zoom:), None, 0.0) };
        Ok(())
    }

    /// Gives winit's view class a `mouseDownCanMoveWindow` that says yes only while
    /// `drag` runs, unless the class has one of its own; NSView's own stays as it is.
    pub(super) fn content_stays(handle: AppKitWindowHandle) -> Result<(), Error> {
        let view = view(&handle)?;
        // winit's class by name, not the view's own class, which a key-value observer
        // may have swapped for a subclass of its own.
        let class = AnyClass::get(c"WinitView")
            .filter(|class| view.isKindOfClass(class))
            .ok_or(Error::UnknownView)?;
        extern "C-unwind" fn moves_on_request(_: &AnyObject, _: Sel) -> Bool {
            Bool::new(MOVES.try_with(Cell::get).unwrap_or(false))
        }
        let moves_on_request: extern "C-unwind" fn(&AnyObject, Sel) -> Bool = moves_on_request;
        // The method's encoding: it returns a BOOL and takes self and the selector.
        let types = CString::new(format!("{}@:", Bool::ENCODING)).expect("no NUL in an encoding");
        // SAFETY: an IMP is any method's function pointer, called with the signature the
        // encoding gives, which is `moves_on_request`'s. The method goes on winit's own
        // class, which does not define it (`class_addMethod` adds nothing to a class that
        // does), so NSView and every other view keep theirs.
        let added = unsafe {
            let imp: Imp = std::mem::transmute(moves_on_request);
            let class = std::ptr::from_ref::<AnyClass>(class).cast_mut();
            objc2::ffi::class_addMethod(class, sel!(mouseDownCanMoveWindow), imp, types.as_ptr())
        };
        let pressed: extern "C-unwind" fn(&AnyObject, Sel, &NSEvent) = mouse_down;
        // SAFETY: `mouse_down` has `mouseDown:`'s signature and calls `MOUSE_DOWN`.
        unsafe {
            let imp: Imp = std::mem::transmute(pressed);
            replace(class, sel!(mouseDown:), imp, &MOUSE_DOWN);
        }
        if !added.as_bool() {
            return Err(Error::OwnMethod("mouseDownCanMoveWindow"));
        }
        if view.mouseDownCanMoveWindow() {
            return Err(Error::ContentMovesWindow);
        }
        Ok(())
    }

    /// winit's view's `mouseDown:`: the event kept for `drag`, then winit's.
    extern "C-unwind" fn mouse_down(view: &AnyObject, selector: Sel, event: &NSEvent) {
        let _ = PRESS.try_with(|press| press.replace(Some(event.retain())));
        let original = *MOUSE_DOWN
            .get()
            .expect("the original is kept before the method is replaced");
        // SAFETY: `MOUSE_DOWN` holds `mouseDown:`'s implementation as winit's view class
        // had it, whose signature this is.
        let original: extern "C-unwind" fn(&AnyObject, Sel, &NSEvent) =
            unsafe { std::mem::transmute(original) };
        original(view, selector, event);
    }

    pub(super) fn drag(handle: AppKitWindowHandle) -> Result<DragStart, Error> {
        let window = window(handle)?;
        let mtm = MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let current = NSApplication::sharedApplication(mtm).currentEvent();
        let kind = current.as_ref().map(|event| event.r#type());
        let left = matches!(
            kind,
            Some(NSEventType::LeftMouseDown | NSEventType::LeftMouseDragged)
        );
        // Taken, so a press is handed over once and an old one never.
        let press = PRESS
            .try_with(|press| press.take())
            .ok()
            .flatten()
            .filter(|press| press.windowNumber() == window.windowNumber());
        let (event, handed) = match (press, current) {
            (Some(press), _) => (press, DragEvent::Press),
            (None, Some(event)) if left => (event, DragEvent::Current),
            (None, other) => {
                let flags = other
                    .as_ref()
                    .map_or(NSEventModifierFlags::empty(), |e| e.modifierFlags());
                let time = other.as_ref().map_or(0.0, |e| e.timestamp());
                let built = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                    NSEventType::LeftMouseDown,
                    window.mouseLocationOutsideOfEventStream(),
                    flags,
                    time,
                    window.windowNumber(),
                    None,
                    0,
                    1,
                    1.0,
                )
                .ok_or(Error::NoEvent)?;
                (built, DragEvent::Built)
            }
        };
        let moves = Raised::new(&MOVES);
        window.performWindowDragWithEvent(&event);
        drop(moves);
        let at = event.locationInWindow();
        Ok(DragStart {
            handed,
            current: kind.map_or_else(|| "none".to_owned(), name),
            at: (at.x, at.y),
        })
    }

    fn name(kind: NSEventType) -> String {
        match kind {
            NSEventType::LeftMouseDown => "left mouse down".to_owned(),
            NSEventType::LeftMouseDragged => "left mouse dragged".to_owned(),
            NSEventType::LeftMouseUp => "left mouse up".to_owned(),
            NSEventType::MouseMoved => "mouse moved".to_owned(),
            other => format!("event type {}", other.0),
        }
    }

    pub(super) fn place(handle: AppKitWindowHandle, height: f64) -> Result<WindowButtons, Error> {
        let window = window(handle)?;
        let address = std::ptr::from_ref::<NSWindow>(&window) as usize;
        let _ = PLACED.try_with(|placed| placed.set(Some((address, height))));
        let placing = Raised::new(&PLACING);
        let placed = place_in(&window, height);
        drop(placing);
        if let WindowButtons::Placed(_) = placed
            && let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton)
            && let Some((_, container, _)) = title_bar_views(&window, &close)
        {
            keep_placed(&window, &container);
        }
        Ok(placed)
    }

    /// Makes AppKit's own layouts of the title bar put the buttons back where `place` put
    /// them, before anything is drawn. AppKit lays them out again in its own title bar's
    /// place whenever it tiles the title bar: on each step of a zoom's or a resize's
    /// animation, inside `-[NSWindow setFrame:display:]`, while egui draws no frame, and
    /// on leaving full screen, outside it. Placed once a frame, they would show out of
    /// place all the way through an animation, and for a frame after.
    ///
    /// The title bar's container is laid out after each tiling and before it is drawn,
    /// so its `layout` puts them back first. That class is a private one of AppKit's and
    /// could be renamed, so winit's window's `setFrame:display:` puts them back too, after
    /// AppKit's: a partial fallback, since with `display:YES` that step may already be
    /// drawn with them in AppKit's place. Both are replaced once per run, for every
    /// window, always run, and act for the window `place` placed only.
    fn keep_placed(window: &NSWindow, container: &NSView) {
        let laid_out: extern "C-unwind" fn(&NSView, Sel) = container_laid_out;
        let changed: extern "C-unwind" fn(&NSWindow, Sel, NSRect, Bool) = frame_changed;
        if let Some(class) =
            AnyClass::get(c"NSTitlebarContainerView").filter(|class| container.isKindOfClass(class))
        {
            // SAFETY: `container_laid_out` has `layout`'s signature and calls `LAYOUT`.
            unsafe {
                let imp: Imp = std::mem::transmute(laid_out);
                replace(class, sel!(layout), imp, &LAYOUT);
            }
        }
        if let Some(class) =
            AnyClass::get(c"WinitWindow").filter(|class| window.isKindOfClass(class))
        {
            // SAFETY: `frame_changed` has `setFrame:display:`'s signature and calls
            // `SET_FRAME`.
            unsafe {
                let imp: Imp = std::mem::transmute(changed);
                replace(class, sel!(setFrame:display:), imp, &SET_FRAME);
            }
        }
    }

    /// Puts `replacement` in the place of `class`'s method for `selector`, once, keeping
    /// what it replaces in `original` for the replacement to call: added to `class` where
    /// it inherits the method, so its superclasses keep theirs, or in place of its own.
    ///
    /// # Safety
    ///
    /// `replacement` takes and returns what the method does, and calls `original`.
    unsafe fn replace(class: &AnyClass, selector: Sel, replacement: Imp, original: &OnceLock<Imp>) {
        let Some(method) = class.instance_method(selector) else {
            return;
        };
        // Kept before the method is replaced, so the replacement always has it to call.
        if original.set(method.implementation()).is_err() {
            return;
        }
        // SAFETY: the replacement has the method's signature, as the caller promises, and
        // takes its type encoding. `class_addMethod` adds nothing to a class that has the
        // method itself, and then `instance_method` found that one, which is replaced.
        unsafe {
            let types = objc2::ffi::method_getTypeEncoding(method);
            let class = std::ptr::from_ref::<AnyClass>(class).cast_mut();
            if !objc2::ffi::class_addMethod(class, selector, replacement, types).as_bool() {
                objc2::ffi::method_setImplementation(method, replacement);
            }
        }
    }

    /// The title bar container's `layout`: AppKit's, then the buttons put back.
    extern "C-unwind" fn container_laid_out(container: &NSView, selector: Sel) {
        let original = *LAYOUT
            .get()
            .expect("the original is kept before the method is replaced");
        // SAFETY: `LAYOUT` holds `layout`'s implementation as the container's class had
        // it, whose signature this is.
        let original: extern "C-unwind" fn(&NSView, Sel) = unsafe { std::mem::transmute(original) };
        original(container, selector);
        if let Some(window) = container.window() {
            put_back(&window);
        }
    }

    /// `setFrame:display:` on winit's window class: AppKit's, then the buttons put back.
    extern "C-unwind" fn frame_changed(
        window: &NSWindow,
        selector: Sel,
        frame: NSRect,
        display: Bool,
    ) {
        let original = *SET_FRAME
            .get()
            .expect("the original is kept before the method is replaced");
        // SAFETY: `SET_FRAME` holds `setFrame:display:`'s implementation as winit's window
        // class had it, whose signature this is.
        let original: extern "C-unwind" fn(&NSWindow, Sel, NSRect, Bool) =
            unsafe { std::mem::transmute(original) };
        original(window, selector, frame, display);
        put_back(window);
    }

    /// Puts the buttons back if `window` is the one `place` placed them in, unless that
    /// is already under way.
    fn put_back(window: &NSWindow) {
        let placed = PLACED.try_with(Cell::get).ok().flatten();
        let address = std::ptr::from_ref::<NSWindow>(window) as usize;
        let Some((_, height)) = placed.filter(|(placed, _)| *placed == address) else {
            return;
        };
        let Some(_placing) = Raised::new(&PLACING) else {
            return;
        };
        place_in(window, height);
    }

    /// A thread's flag, raised until this is dropped, even when an Objective-C exception
    /// unwinds through the code that raised it.
    struct Raised(&'static LocalKey<Cell<bool>>);

    impl Raised {
        /// Raises `flag`, or `None` where it is raised already.
        fn new(flag: &'static LocalKey<Cell<bool>>) -> Option<Raised> {
            (flag.try_with(|raised| raised.replace(true)) == Ok(false)).then_some(Raised(flag))
        }
    }

    impl Drop for Raised {
        fn drop(&mut self) {
            let _ = self.0.try_with(|raised| raised.set(false));
        }
    }

    /// Places the buttons of `window` in a top bar `height` points tall.
    fn place_in(window: &NSWindow, height: f64) -> WindowButtons {
        // In full screen the buttons live in the system's own title bar, which slides
        // down over the window's top when the pointer reaches the screen's edge.
        if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
            return WindowButtons::None;
        }
        let buttons = [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ]
        .map(|button| window.standardWindowButton(button));
        // A window that hides its buttons still has the top bar for its title bar.
        let [Some(close), Some(minimise), Some(zoom)] = buttons else {
            return WindowButtons::Placed(0.0);
        };
        if close.isHidden() {
            return WindowButtons::Placed(0.0);
        }
        // The buttons sit in the title bar's view, in a container at the top of the
        // window's frame view as tall as the system's title bar. Made as tall as the app's
        // bar, it lets them sit at its centre and still take clicks. Nothing is touched
        // unless the views are as that says.
        let Some((title_bar, container, frame)) = title_bar_views(window, &close) else {
            return WindowButtons::Unplaced(room(&close, &zoom) as f32);
        };
        let height = height.round();
        let bounds = frame.bounds();
        let top = if frame.isFlipped() {
            bounds.origin.y
        } else {
            bounds.origin.y + bounds.size.height - height
        };
        let width = container.frame().size.width;
        let origin = NSPoint::new(container.frame().origin.x, top);
        set_frame(&container, NSRect::new(origin, NSSize::new(width, height)));
        set_frame(&title_bar, container.bounds());

        // The close button's centre is as far from the window's left edge as from its
        // top; the others keep their spacing after it.
        let left = container.frame().origin.x + title_bar.frame().origin.x;
        let first = close.frame();
        let x = (height / 2.0 - first.size.width / 2.0).round();
        let shift = x - left - first.origin.x;
        let along = title_bar.bounds();
        for button in [&close, &minimise, &zoom] {
            let at = button.frame();
            let y = along.origin.y + ((along.size.height - at.size.height) / 2.0).round();
            let origin = NSPoint::new(at.origin.x + shift, y);
            if origin != at.origin {
                button.setFrameOrigin(origin);
            }
        }
        WindowButtons::Placed(room(&close, &zoom) as f32)
    }

    /// The room the buttons take from the window's left edge, wherever they are: to past
    /// the zoom button by the close button's margin.
    fn room(close: &NSButton, zoom: &NSButton) -> f64 {
        let first = close.convertRect_toView(close.bounds(), None);
        let last = zoom.convertRect_toView(zoom.bounds(), None);
        last.origin.x + last.size.width + first.origin.x
    }

    /// The title bar's view holding `button`, its container and the window's frame view,
    /// if they are as AppKit has built them since OS X 10.10: the container straight in
    /// the frame view, the view that holds the window's content.
    fn title_bar_views(
        window: &NSWindow,
        button: &NSButton,
    ) -> Option<(Retained<NSView>, Retained<NSView>, Retained<NSView>)> {
        let content = window.contentView()?;
        // SAFETY: a view's superview is alive while it holds the view, and nothing changes
        // the hierarchy between these calls on the main thread; each is retained at once.
        let (frame, title_bar) = unsafe { (content.superview()?, button.superview()?) };
        let container = unsafe { title_bar.superview() }?;
        let parent = unsafe { container.superview() }?;
        let same = |a: &NSView, b: &NSView| std::ptr::eq(a, b);
        let expected = same(&parent, &frame)
            && !same(&container, &content)
            && !same(&title_bar, &content)
            && !same(&title_bar, &frame);
        expected.then_some((title_bar, container, frame))
    }

    fn set_frame(view: &NSView, frame: NSRect) {
        if view.frame() != frame {
            view.setFrame(frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DoubleClick;

    #[test]
    fn a_double_click_does_what_the_system_s_setting_says_and_zooms_otherwise() {
        for (action, minimise, expected) in [
            (Some("Maximize"), false, DoubleClick::Zoom),
            (Some("Fill"), false, DoubleClick::Zoom),
            (Some("Minimize"), false, DoubleClick::Minimise),
            (Some("None"), false, DoubleClick::Nothing),
            // The newer key wins over the older one.
            (Some("Maximize"), true, DoubleClick::Zoom),
            (Some("None"), true, DoubleClick::Nothing),
            // Unset: the setting before OS X 10.11, then zoom.
            (None, true, DoubleClick::Minimise),
            (None, false, DoubleClick::Zoom),
            // A value a later macOS may add zooms, as the system does by default.
            (Some("Tile"), false, DoubleClick::Zoom),
            (Some(""), false, DoubleClick::Zoom),
        ] {
            assert_eq!(
                DoubleClick::from_defaults(action, minimise),
                expected,
                "{action:?}, {minimise}"
            );
        }
    }
}
