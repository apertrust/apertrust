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

//! The dialog host (decision 41; `docs/gui.md`, Dialogs and alerts): dialogs as native
//! windows of their own beside eframe's, on every platform, one at a time, and the jobs
//! window beside them.
//!
//! **A dialog** is a type implementing [`Dialog`], once: its id and translated title, the
//! form it asks for ([`Form`]: a sheet, a modal window or a modeless one), its look
//! ([`Look`]), its keys ([`Keys`]: what Escape, Cmd+W and Cmd+. do) and its content,
//! [`Dialog::show`], a function of `&mut Ui`. The host knows nothing else of it: it draws
//! any dialog the same way ([`draw`]), which the tests do in a harness of their own, and
//! each platform's part of each form is in one place, [`platform`]: which form a dialog
//! gets ([`platform::form`]: a sheet is AppKit's on the Mac and a modal window centred on
//! the main window elsewhere) and what its window is made with.
//!
//! eframe 0.37 hands the app no egui viewport's window and gives a viewport no AccessKit
//! adapter (egui pull request #8810 would add it), so the host makes each window itself, from `Gated`,
//! which runs eframe on the app's own event loop: a winit window, made hidden and kept out
//! of other windows' tabs, with an egui-winit `State` and its AccessKit adapter made before
//! it first shows, an egui `Context` of its own with the main window's theme, language and
//! interface scale, and an egui-wgpu painter on the engine's one device, from eframe's
//! render state. `Gated` routes the window's events, its AccessKit requests and its
//! repaints, asked under [`viewport`], to the host, never to eframe, all while it holds the
//! engine's submit gate, so every configure of the dialog's surface runs while the engine
//! cannot submit (decision 18), waiting, as every configure does, for what the engine has
//! submitted. The window takes its content's size at the interface's scale, measured as it
//! is drawn, but for one the person resizes, the log window's ([`Dialog::resize`]), whose
//! content takes the window's size; a modeless window opens where it was last left, which the app keeps in
//! settings, or centred on the main window, and a modal one centred on it.
//!
//! The jobs window (`docs/gui.md`, The jobs window) is a second, modeless window beside the
//! dialog's: it has a slot of its own ([`Slot`]), so it stays open while Settings or Export
//! open, closes alone and keeps its own place. The log window (`docs/gui.md`, The log
//! window) is a third, in a slot of its own the same way. On Linux the drawn alert is a
//! fourth, modal over the window it rises from. At most one dialog, one jobs window, one
//! log window and one alert are open at a time.
//!
//! A modeless window ([`Form::Modeless`]: Settings on the Mac, and the jobs window) leaves
//! both windows taking input, and a second request brings it to the front. A modal one
//! ([`Form::Modal`]: Settings on Windows and Linux, and a sheet there) keeps the main
//! window's input from it until it goes, as the app's `raw_input_hook` does, a click on the
//! main window bringing it forward ([`Dialogs::raise`]). A sheet ([`Form::Sheet`],
//! Export's) is the Mac's, through `apertrust-display`, where AppKit puts it, centred, the
//! window dimmed behind it, the main window taking no input until it goes. AppKit animates
//! a sheet in and out for about 200 ms in a run loop of its own, so `apertrust-display`
//! starts both from the run loop, outside the gated callbacks, and the host keeps the
//! sheet's window until AppKit's `endSheet:` has returned, reporting the dialog gone only
//! then: winit's callbacks run while AppKit animates, and Quit, which waits for the dialog
//! to go, must not close the main window with the sheet still on it. The app reaches the
//! host through [`Dialogs`], with [`AppDialogs`] over it.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Wake, Waker};
use std::time::Instant;

use apertrust_display::ColorSpace;
use apertrust_engine::session::ExportPreview;
use apertrust_ui::alert;
use apertrust_ui::components::{ActivityAction, Job, caption_button};
use apertrust_ui::icons::Icon;
use apertrust_ui::theme;
use eframe::egui::os::OperatingSystem;
use eframe::egui::{self, Key, Modifiers, UiBuilder, ViewportId, WidgetInfo, accesskit::Role};
use eframe::egui_wgpu::{self, RenderState, WgpuSetup, WgpuSetupExisting};
use egui_winit::winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use egui_winit::winit::event::WindowEvent;
use egui_winit::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use egui_winit::winit::window::{Window, WindowButtons, WindowId};

use crate::actions::Focus;
use crate::export;
use crate::jobs;
use crate::log_window;
use crate::settings::Position;
use crate::settings_dialog;

mod alert_dialog;
pub mod platform;

/// The viewport id the dialogs' repaints are asked under, which `Gated` routes to the
/// host. Each dialog's context has its own root viewport; this one names it to eframe's
/// loop, where eframe's own root would be taken for the main window.
pub fn viewport() -> ViewportId {
    ViewportId::from_hash_of("apertrust dialog host")
}

/// The viewport id the jobs window's repaints are asked under, apart from the dialog's, so
/// the host knows which of its windows to draw.
fn jobs_viewport() -> ViewportId {
    ViewportId::from_hash_of("apertrust dialog host jobs window")
}

/// The viewport id the log window's repaints are asked under, as the jobs window's are.
fn log_viewport() -> ViewportId {
    ViewportId::from_hash_of("apertrust dialog host log window")
}

/// The viewport id a drawn alert's repaints are asked under.
fn alert_viewport() -> ViewportId {
    ViewportId::from_hash_of("apertrust dialog host alert window")
}

/// Which of the host's windows: the dialog, a drawn alert, or the jobs or log window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Dialog,
    Jobs,
    Log,
    Alert,
}

/// How many windows the host may have open at once, one per [`Slot`].
const SLOTS: usize = 4;

impl Slot {
    const ALL: [Slot; SLOTS] = [Slot::Dialog, Slot::Jobs, Slot::Log, Slot::Alert];

    fn index(self) -> usize {
        self as usize
    }

    /// The viewport id its context's repaints are asked under.
    fn viewport(self) -> ViewportId {
        match self {
            Slot::Dialog => viewport(),
            Slot::Jobs => jobs_viewport(),
            Slot::Log => log_viewport(),
            Slot::Alert => alert_viewport(),
        }
    }

    /// The window a repaint asked under `id` is for, if it is the host's.
    fn of_viewport(id: ViewportId) -> Option<Slot> {
        Slot::ALL.into_iter().find(|slot| slot.viewport() == id)
    }
}

/// How a dialog shows (decision 41). A dialog asks for one ([`Dialog::form`]); the
/// platform gives it the one it can ([`platform::form`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Attached to the main window, which takes no input until it goes: AppKit's sheet on
    /// the Mac (Export's, and every dialog's to come but Settings'), a modal window
    /// elsewhere.
    Sheet,
    /// A window of its own, centred on the main window, which takes no input until it
    /// goes: Settings' on Windows and Linux.
    Modal,
    /// A window of its own beside the main window, both taking input, opening where it was
    /// last left: Settings' on the Mac, the jobs window's.
    Modeless,
}

impl Form {
    /// Whether it keeps the main window's input from it.
    pub fn modal(self) -> bool {
        self != Form::Modeless
    }

    /// Its name in the log.
    fn name(self) -> &'static str {
        match self {
            Form::Sheet => "sheet",
            Form::Modal => "modal window",
            Form::Modeless => "window",
        }
    }
}

/// How a dialog's window looks round its content. Every dialog's window takes its
/// content's size, measured as it is drawn, and the person never resizes it (decision
/// 41) but where the dialog says ([`Dialog::resize`]), so the look holds no size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// What its window is cleared to.
    pub background: egui::Color32,
    /// The margin round its content, in points: none for a content that draws its own.
    pub margin: f32,
}

/// What a dialog's keys do in its own window: each closes it as its Cancel. A key it
/// leaves alone may be a menu item's, as Close Window takes Cmd+W on the Mac for a
/// modeless window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keys {
    /// Escape, once no field or list of the dialog's has it: a field typed in as the last
    /// frame ended keeps it ([`typing`]), and so does a list the dialog says it holds
    /// ([`Asks::keeps_escape`]); an open popup takes it before the dialog sees it.
    pub escape: bool,
    /// Cmd+W, Ctrl+W elsewhere.
    pub close_window: bool,
    /// Cmd+., the Mac's cancel, Ctrl+. elsewhere.
    pub cancel: bool,
}

/// What one frame of a dialog's content asks of the host.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asks {
    /// What it asks of the app.
    pub output: Option<Output>,
    /// It asked to close: its Close button, or its work done.
    pub close: bool,
    /// A list of its own was open as the frame began, which Escape closes first.
    pub keeps_escape: bool,
}

/// A dialog the host shows (decision 41; `docs/gui.md`, Dialogs and alerts): every fact
/// the host needs of it, declared once by the dialog's own type, the same on every
/// platform but where a method is given the platform.
pub trait Dialog: Any + std::fmt::Debug {
    /// Its stable name, which tells one dialog from another and names it in the log, never
    /// translated.
    fn id(&self) -> &'static str;
    /// Its title, which its window takes and screen readers read, in `ctx`'s language.
    fn title(&self, ctx: &egui::Context) -> String;
    /// The form it asks for on `os`, which [`platform::form`] turns into the one the
    /// platform gives.
    fn form(&self, os: OperatingSystem) -> Form;
    /// Its look in the theme `t`.
    fn look(&self, t: &apertrust_ui::Theme) -> Look;
    /// The role of its window's node for screen readers, named by its title: a dialog, or
    /// an alert dialog for an alert the host draws.
    fn role(&self) -> Role {
        Role::Dialog
    }
    /// What its keys do on `os`.
    fn keys(&self, os: OperatingSystem) -> Keys;
    /// Draws one frame of its content in `ui` and says what it asked for.
    fn show(&mut self, ui: &mut egui::Ui) -> Asks;
    /// What the app is to keep of where its modeless window was left, `at`, as it closed,
    /// for it to open there next time; nothing for a dialog whose place is not kept.
    fn placed(&self, at: Position) -> Option<Output> {
        let _ = at;
        None
    }
    /// Takes `waker`, which asks for its window's next frame from any thread, as its window
    /// is made: for a dialog whose content changes off the UI thread, the log window's as
    /// lines come; nothing for any other.
    fn wake_with(&mut self, waker: Waker) {
        let _ = waker;
    }
    /// How the person resizes its window, if they do: the log window's. None for a window
    /// that takes its content's size, as every other dialog's does (decision 41).
    fn resize(&self) -> Option<Resize> {
        None
    }
    /// What the app is to keep of the size a window the person resized was left at, in
    /// points, as it closed; nothing for a dialog whose size is not kept.
    fn sized(&self, size: egui::Vec2) -> Option<Output> {
        let _ = size;
        None
    }
}

/// What a dialog asked of the app, which takes it before its next frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// A setting changed in Settings, for the shell to keep and the main window to show.
    Settings(settings_dialog::Change),
    /// Where Settings' own window was left as it closed, for it to open there next time;
    /// the one dialog whose place is kept.
    Placed(Position),
    /// The Export dialog's settings, changed, and whether Export was pressed.
    Export(export::Asked),
    /// What the user pressed in the jobs window, for the app to do.
    Jobs(ActivityAction),
    /// Where the jobs window was left as it closed, for it to open there next time.
    JobsPlaced(Position),
    /// What the log window asked of the app: Settings, or its lines saved.
    Log(log_window::Asked),
    /// Where the log window was left as it closed, for it to open there next time.
    LogPlaced(Position),
    /// The size the log window was left at as it closed, in points, which it opens at
    /// next time.
    LogSized(egui::Vec2),
    /// Key presses a dialog's window left alone, for the app's keymap to read, as Quit's.
    Keys(Unread),
}

/// The key presses a dialog's window left alone in a frame: none of Escape, Cmd+W and
/// Cmd+. where its keys take them, nor what its content consumed. Its window has no keymap,
/// so the app reads them with its own (`actions::read_from_dialog`), which leaves the
/// focused widget its keys and the availability rules to decide what rests.
#[derive(Clone, Debug, PartialEq)]
pub struct Unread {
    /// Each press, with the text it typed right after it, as egui-winit hands them over.
    pub events: Vec<egui::Event>,
    /// What had the focus in the dialog as the frame began.
    pub focus: Focus,
    /// The platform, whose spelling of the keys the keymap's matching follows.
    pub os: OperatingSystem,
}

/// The presses among `events`, each with the text that follows it.
fn presses(events: &[egui::Event]) -> Vec<egui::Event> {
    let mut kept = Vec::new();
    for (i, event) in events.iter().enumerate() {
        if let egui::Event::Key { pressed: true, .. } = event {
            kept.push(event.clone());
            if let Some(text @ egui::Event::Text(_)) = events.get(i + 1) {
                kept.push(text.clone());
            }
        }
    }
    kept
}

/// What a frame of a dialog's content asked for, and its size, in points.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub output: Option<Output>,
    /// It asked to close, itself or by its keys.
    pub close: bool,
    pub size: egui::Vec2,
    /// The key presses it left alone, if any.
    pub unread: Option<Unread>,
    /// What a press on the frame the host draws asks of its window, if one did.
    pub gesture: Option<Gesture>,
}

/// What a press on the frame the host draws (`platform::draws_frame`) asks of the
/// dialog's window: a drag on its empty strip moves it, through the compositor or the
/// window manager; a right-click there opens the system's window menu, at that point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    Move,
    Menu(egui::Pos2),
}

/// The height of the strip at a dialog window's top where the host draws its frame, in
/// points: its close button and the space a drag moves the window from.
const FRAME: f32 = 32.0;

/// The transparent margin round a shadowed dialog's card ([`platform::shadowed`]), in
/// points, on each side, where its shadow is drawn.
const MARGIN: f32 = 12.0;

/// Where a dialog's context keeps whether its window is shadowed.
fn shadow_id() -> egui::Id {
    egui::Id::unique("dialog host shadowed")
}

/// Whether the dialog drawn in `ctx` has the border and drop shadow the host draws
/// ([`platform::shadowed`]), as the host decided it from the main window as it made the
/// dialog's ([`set_shadow`]); none where nothing set it, as in a test's harness.
fn shadow(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(shadow_id())).unwrap_or(false)
}

/// Says whether the dialog drawn in `ctx` is shadowed ([`shadow`]).
fn set_shadow(ctx: &egui::Context, shadowed: bool) {
    ctx.data_mut(|d| d.insert_temp(shadow_id(), shadowed));
}

/// How far a dialog's card sits inside its window drawn in `ctx`, in points, on each side:
/// a shadowed one's margin and its border, a hairline of the theme's; none elsewhere.
fn card_inset(ctx: &egui::Context) -> f32 {
    if shadow(ctx) {
        MARGIN + theme(ctx).shape.hairline
    } else {
        0.0
    }
}

/// The room a dialog's window drawn in `ctx` takes beyond its content, in points: the frame
/// the host draws above it ([`platform::draws_frame`]), and on each side the card's inset
/// ([`card_inset`]). A window the person resizes holds its content's least and kept sizes
/// ([`Resize`]) with this room added.
fn chrome(ctx: &egui::Context) -> egui::Vec2 {
    let frame = if platform::draws_frame(ctx.os()) {
        FRAME
    } else {
        0.0
    };
    egui::vec2(0.0, frame) + egui::Vec2::splat(2.0 * card_inset(ctx))
}

/// The width of the bands a window the person resizes resizes from, in points, from the
/// window's edge: through the card's inset ([`card_inset`]) and the main window's band
/// inside the card's edge, so the person resizes from the card they see.
fn resize_band(ctx: &egui::Context) -> f32 {
    card_inset(ctx) + crate::RESIZE_BAND
}

/// egui's way to resize a window as winit's.
fn resize_direction(way: egui::ResizeDirection) -> egui_winit::winit::window::ResizeDirection {
    use egui::ResizeDirection as From;
    use egui_winit::winit::window::ResizeDirection as To;
    match way {
        From::North => To::North,
        From::South => To::South,
        From::East => To::East,
        From::West => To::West,
        From::NorthEast => To::NorthEast,
        From::SouthEast => To::SouthEast,
        From::NorthWest => To::NorthWest,
        From::SouthWest => To::SouthWest,
    }
}

/// How a window the person resizes is sized ([`Dialog::resize`]), in points, without the
/// frame the host draws: never smaller than `least`, and opened at `size`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resize {
    pub least: egui::Vec2,
    pub size: egui::Vec2,
}

/// Where [`draw`] keeps whether a field had the keyboard as the last frame ended.
fn typing_id() -> egui::Id {
    egui::Id::unique("dialog host typing")
}

/// Whether a field of the dialog drawn in `ctx` had the keyboard as its last frame ended:
/// Escape and Return are then the field's.
pub fn typing(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(typing_id())).unwrap_or(false)
}

/// Draws `dialog` in `ui`, its window's whole area, as the host does each frame: inside its
/// margin, under a node of its role ([`Dialog::role`]) named by its title, since egui's root node has none
/// (egui issue #8785), then its keys as `ui`'s platform has them ([`Dialog::keys`]).
/// Returns what it asked for, and the size it took with its margin, which its window is
/// given.
pub fn draw(ui: &mut egui::Ui, dialog: &mut dyn Dialog) -> Drawn {
    let ctx = ui.ctx().clone();
    let title = dialog.title(&ctx);
    let role = dialog.role();
    let t = theme(&ctx);
    let look = dialog.look(&t);
    let typing = typing(&ctx);
    let focus = Focus::of(&ctx);
    let mut asks = Asks::default();
    let framed = platform::draws_frame(ctx.os());
    let shadowed = shadow(&ctx);
    let margin = if shadowed { MARGIN } else { 0.0 };
    let mut card = egui::Frame::NONE;
    if shadowed {
        card = card
            .fill(look.background)
            .stroke(egui::Stroke::new(t.shape.hairline, t.colors.border))
            .shadow(egui::Shadow {
                offset: [0, 2],
                blur: 12,
                spread: 1,
                color: t.colors.scrim.gamma_multiply(0.6),
            });
    }
    let (mut gesture, mut closed) = (None, false);
    let max_rect = ui.available_rect_before_wrap().shrink(margin);
    let scope = ui.scope_builder(UiBuilder::new().max_rect(max_rect), |ui| {
        card.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let top = ui.cursor().min;
            if framed {
                ui.add_space(FRAME);
            }
            let content = egui::Frame::NONE
                .inner_margin(look.margin)
                .show(ui, |ui| asks = dialog.show(ui));
            if framed {
                // As wide as the content, but never past the window's edge, so its close is
                // always there to press, a window not at its size yet included.
                let width = content
                    .response
                    .rect
                    .width()
                    .min(ui.clip_rect().right() - top.x);
                let strip = egui::Rect::from_min_size(top, egui::vec2(width, FRAME));
                (gesture, closed) = frame(ui, strip);
            }
        });
    });
    scope
        .response
        .widget_info(|| WidgetInfo::labeled(role, true, &title));
    // After the content, so a field or a list of its own has seen the key first.
    let keys = dialog.keys(ctx.os());
    let pressed = |modifiers, key| ctx.input_mut(|i| i.consume_key(modifiers, key));
    let escape = keys.escape && pressed(Modifiers::NONE, Key::Escape);
    let mut close = closed || asks.close || escape && !typing && !asks.keeps_escape;
    close |= keys.cancel && pressed(Modifiers::COMMAND, Key::Period);
    close |= keys.close_window && pressed(Modifiers::COMMAND, Key::W);
    // A text field, not any control with the focus: a choice a list gave the focus back to
    // leaves Escape the dialog's.
    let now = ctx.text_edit_focused();
    ctx.data_mut(|d| d.insert_temp(typing_id(), now));
    let events = ctx.input(|i| presses(&i.events));
    Drawn {
        output: asks.output,
        close,
        size: scope.response.rect.size() + egui::Vec2::splat(2.0 * margin),
        unread: (!events.is_empty()).then(|| Unread {
            events,
            focus,
            os: ctx.os(),
        }),
        gesture,
    }
}

/// The frame the host draws at the top of a dialog's window (`platform::draws_frame`),
/// in `strip`: the close button at its end, in the main window's caption style, which
/// closes the dialog as its Cancel; a drag on the rest moves the window, and a right-click
/// opens the window's menu. The button comes after the content in the focus order.
fn frame(ui: &mut egui::Ui, strip: egui::Rect) -> (Option<Gesture>, bool) {
    let ctx = ui.ctx().clone();
    let t = theme(&ctx);
    // First, so the button over it takes its own presses.
    let empty = ui.interact(
        strip,
        ui.scope_id().with("dialog frame"),
        egui::Sense::click_and_drag(),
    );
    let mut end = ui.new_child(
        UiBuilder::new()
            .max_rect(strip)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    let name = apertrust_ui::tr(&ctx, "window-close");
    let closed = caption_button(&mut end, Icon::Close, &name, true, &t).clicked();
    let gesture = if empty.drag_started_by(egui::PointerButton::Primary) {
        Some(Gesture::Move)
    } else if empty.secondary_clicked() {
        empty.interact_pointer_pos().map(Gesture::Menu)
    } else {
        None
    };
    (gesture, closed)
}

/// Draws a dialog's buttons at the end of a row, in the order given, which is to be the
/// platform's: on the Mac the default button last, as Cancel, then Export. They are added
/// in that order, from the row's start, so screen readers and Tab meet them in the order
/// they are seen, where a row laid out from its end, as egui's right-to-left layout is,
/// would meet them backwards. Returns which was clicked, if one was. Their width is the last
/// pass's, and a pass that finds it changed is drawn again.
pub fn buttons(ui: &mut egui::Ui, words: &[&str]) -> Option<usize> {
    let id = ui.make_persistent_id("dialog buttons");
    let width: Option<f32> = ui.data(|d| d.get_temp(id));
    let mut clicked = None;
    let row = ui.horizontal(|ui| {
        ui.add_space((ui.available_width() - width.unwrap_or(0.0)).max(0.0));
        let start = ui.cursor().min.x;
        for (i, words) in words.iter().enumerate() {
            if ui.button(*words).clicked() {
                clicked = Some(i);
            }
        }
        ui.min_rect().max.x - start
    });
    if width != Some(row.inner) {
        ui.data_mut(|d| d.insert_temp(id, row.inner));
        ui.ctx().request_discard("a dialog's buttons measured");
    }
    clicked
}

/// What came of asking for an alert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// It is up.
    Shown,
    /// Another alert is up already: nothing was asked.
    Busy,
    /// The platform has no alert to show.
    NoAlerts,
    /// It could not be shown; the cause is logged.
    Failed,
}

/// How an alert ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The button at this index of the request was pressed, by a click or a key.
    Pressed(usize),
    /// The same, with the request's suppression box ticked.
    Ticked(usize),
    /// It ended without one: the system took it away.
    Dismissed,
    /// It was shown, but its window could not be made, as Linux's drawn alert's may not
    /// be: as an alert that could not be shown ([`Ask::Failed`]); the cause is logged.
    Failed,
}

impl Answer {
    /// How the platform's answer ended: the button pressed, if one, and whether the
    /// suppression box was ticked. An alert taken away is dismissed whatever the box held.
    fn of(pressed: Option<usize>, ticked: bool) -> Answer {
        match pressed {
            Some(i) if ticked => Answer::Ticked(i),
            Some(i) => Answer::Pressed(i),
            None => Answer::Dismissed,
        }
    }

    /// Whether the person said yes: the first button of a request, the default, which is
    /// the one that goes ahead.
    pub fn confirmed(self) -> bool {
        matches!(self, Answer::Pressed(0) | Answer::Ticked(0))
    }

    /// Whether the suppression box was ticked, whatever button was pressed.
    pub fn ticked(self) -> bool {
        matches!(self, Answer::Ticked(_))
    }
}

/// The dialog open, as the app sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown {
    pub form: Form,
    /// Whether its window has the focus.
    pub focused: bool,
}

impl Shown {
    /// Whether it keeps the main window's input from it.
    pub fn modal(&self) -> bool {
        self.form.modal()
    }
}

/// A dialog gone, with the control of the main window that had the focus when it was
/// asked for, which takes it back: after a modal dialog always, and after a modeless window
/// only where the main window's focus has not moved since, as the main window takes input
/// beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Closed {
    pub opener: Option<egui::Id>,
}

/// The dialog host as the app sees it: the host itself, or a test's.
pub trait Dialogs {
    /// Opens `dialog` over the main window, whose context is `main`, in the form its
    /// platform gives it ([`platform::form`]): a modeless window at `place`, where it was
    /// last left, if that is on a screen still, else centred on the main window, as a modal
    /// one always is. Where the same dialog is open as a modeless window already, its
    /// window comes to the front; while any other is open, nothing opens, one dialog at a
    /// time.
    fn open(&mut self, dialog: Box<dyn Dialog>, place: Option<Position>, main: &egui::Context);
    /// The dialog open, if one is: a sheet until AppKit has animated it away. An alert the
    /// host draws ([`platform::draws_alerts`]) comes first while it is open, a modal window
    /// over the dialog or the main window; the system's alerts are not the host's.
    fn shown(&self) -> Option<Shown>;
    /// Whether an alert the host draws is open or asked; never the system's.
    fn alert_shown(&self) -> bool;
    /// Brings the alert the host draws to the front with the focus, if one is open, else
    /// the dialog open if it is a modal window, as a click on the main window does under a
    /// system's modal window; AppKit keeps a sheet's itself, and a modeless window stays
    /// behind the window clicked.
    fn raise(&mut self);
    /// The last dialog gone, once.
    fn closed(&mut self) -> Option<Closed>;
    /// What the dialog open asked of the app since the last call, in order.
    fn outputs(&mut self) -> Vec<Output>;
    /// Closes the dialog open as its Cancel would, as Close Window does: a window at once, a
    /// sheet once AppKit has animated it away. While an alert the host draws is open or
    /// asked, that alert closes instead, answering dismissed, and the dialog under it stays.
    fn close(&mut self);
    /// Hands the Export dialog open the session's answer to the preview numbered `through`;
    /// any other dialog, or none, takes nothing.
    fn preview(&mut self, through: u64, preview: ExportPreview);
    /// Opens the jobs window, a window of its own beside any dialog, at `place`, where it
    /// was last left, if that is on a screen still, else centred on the main window, whose
    /// context is `main`. Where it is open already, its window comes to the front if `focus`
    /// asks, and nothing changes otherwise. It takes the focus as it opens only if `focus`.
    fn open_jobs(
        &mut self,
        window: jobs::Window,
        place: Option<Position>,
        main: &egui::Context,
        focus: bool,
    );
    /// The jobs window, if open.
    fn jobs_shown(&self) -> Option<Shown>;
    /// Gives the jobs window open the jobs it shows, in the order the app holds them.
    fn set_jobs(&mut self, jobs: Vec<Job>);
    /// Closes the jobs window open, which stops no job.
    fn close_jobs(&mut self);
    /// Opens the log window, a window of its own beside any dialog and the jobs window, at
    /// `place` as the jobs window opens, with the focus; where it is open already, its
    /// window comes to the front.
    fn open_log(
        &mut self,
        window: log_window::Window,
        place: Option<Position>,
        main: &egui::Context,
    );
    /// The log window, if open.
    fn log_shown(&self) -> Option<Shown>;
    /// Closes the log window open.
    fn close_log(&mut self);
    /// Asks `request` as the system's alert, on the dialog's window if a modal one is shown
    /// (a sheet or a modal window), else on the main window, whose context is `main`, and
    /// returns at
    /// once; the answer comes through [`Dialogs::answer`], and `main` is asked for a frame
    /// when it does. Says whether it was shown, the platform has no alert, or showing it
    /// failed; nothing was asked unless it was shown. One alert at a time: the caller asks
    /// again only after the answer.
    fn ask(&mut self, request: &alert::Request, main: &egui::Context) -> Ask;
    /// The answer to the alert asked, once, when it has gone.
    fn answer(&mut self) -> Option<Answer>;
}

/// The app's side of its dialogs, over the host behind [`Dialogs`]: the focus given back as
/// a dialog closes, and Quit held while one is open.
pub struct AppDialogs {
    host: Box<dyn Dialogs>,
    /// Whether Quit waits for the dialog it closed to go.
    quitting: bool,
}

impl AppDialogs {
    pub fn new(host: impl Dialogs + 'static) -> AppDialogs {
        AppDialogs {
            host: Box::new(host),
            quitting: false,
        }
    }

    /// Opens `dialog` over the main window, `main`'s, a modeless window at `place` where it
    /// is on a screen ([`Dialogs::open`]).
    pub fn open(&mut self, dialog: impl Dialog, place: Option<Position>, main: &egui::Context) {
        self.host.open(Box::new(dialog), place, main);
    }

    /// Brings a modal dialog's window forward, as a click on the main window under it asks
    /// ([`Dialogs::raise`]).
    pub fn raise(&mut self) {
        self.host.raise();
    }

    /// The windows open, as one: the dialog's form, which is what keeps the main window's
    /// input, and the focus if any window has it. The jobs and the log window are modeless.
    pub fn shown(&self) -> Option<Shown> {
        let windows = [
            self.host.shown(),
            self.host.jobs_shown(),
            self.host.log_shown(),
        ];
        let focused = windows.iter().flatten().any(|shown| shown.focused);
        let first = windows.into_iter().flatten().next()?;
        Some(Shown { focused, ..first })
    }

    /// What the windows open asked of the app since the last call.
    pub fn outputs(&mut self) -> Vec<Output> {
        self.host.outputs()
    }

    /// Opens the jobs window, or brings it forward ([`Dialogs::open_jobs`]).
    pub fn open_jobs(
        &mut self,
        window: jobs::Window,
        place: Option<Position>,
        main: &egui::Context,
        focus: bool,
    ) {
        self.host.open_jobs(window, place, main, focus);
    }

    /// Gives the jobs window open the jobs it shows.
    pub fn set_jobs(&mut self, jobs: Vec<Job>) {
        self.host.set_jobs(jobs);
    }

    /// Opens the log window, or brings it forward ([`Dialogs::open_log`]).
    pub fn open_log(
        &mut self,
        window: log_window::Window,
        place: Option<Position>,
        main: &egui::Context,
    ) {
        self.host.open_log(window, place, main);
    }

    /// Closes the window with the focus, as Close Window does: the jobs or the log window
    /// if it has it, else the dialog.
    pub fn close(&mut self) {
        let jobs = self.host.jobs_shown().is_some_and(|jobs| jobs.focused);
        let log = self.host.log_shown().is_some_and(|log| log.focused);
        let dialog = self.host.shown().is_some_and(|dialog| dialog.focused);
        if dialog || !(jobs || log) {
            self.host.close();
        } else if jobs {
            self.host.close_jobs();
        } else {
            self.host.close_log();
        }
    }

    /// Hands the Export dialog open the session's answer to the preview numbered
    /// `through`.
    pub fn preview(&mut self, through: u64, preview: ExportPreview) {
        self.host.preview(through, preview);
    }

    /// Asks `request` as the system's alert ([`Dialogs::ask`]).
    pub fn ask(&mut self, request: &alert::Request, main: &egui::Context) -> Ask {
        self.host.ask(request, main)
    }

    /// The answer to the alert asked, once, when it has gone.
    pub fn answer(&mut self) -> Option<Answer> {
        self.host.answer()
    }

    /// Whether Quit, or a close of the main window, may run now. Not while a dialog is
    /// open: the dialog closes as its Cancel, and Quit waits for it to go
    /// ([`AppDialogs::frame`]), since AppKit closes no window a sheet hangs on. While an
    /// alert the host draws asks, nothing happens, as under the Mac's `NSAlert`: the alert
    /// is the person's to answer, and the dialog under it stays.
    pub fn quit(&mut self) -> bool {
        if self.host.alert_shown() {
            return false;
        }
        if self.shown().is_none() {
            return true;
        }
        self.host.close();
        self.host.close_jobs();
        self.host.close_log();
        self.quitting = true;
        false
    }

    /// Before each of the main window's frames, `main`'s: gives the focus back to the
    /// control that had it when the dialog that closed was asked for, and says whether
    /// Quit, held for a dialog, may run now that it has gone.
    pub fn frame(&mut self, main: &egui::Context) -> bool {
        if let Some(Closed { opener: Some(id) }) = self.host.closed() {
            main.memory_mut(|m| m.request_focus(id));
        }
        let quit = self.quitting && self.shown().is_none();
        if quit {
            self.quitting = false;
        }
        quit
    }
}

/// The host, shared by the app, which asks for dialogs, and `Gated`, which runs them.
#[derive(Clone)]
pub struct Host(Rc<RefCell<Inner>>);

struct Inner {
    /// Sends the dialogs' repaints and AccessKit requests to the event loop.
    proxy: Arc<Mutex<EventLoopProxy<eframe::UserEvent>>>,
    /// The engine's device and eframe's window, once eframe has made them. The window is
    /// eframe's to drop: held here, it would never close, and eframe's loop would never
    /// end.
    render: Option<RenderState>,
    main: Weak<Window>,
    /// What each window is asked for, and the window open, by [`Slot`].
    asked: [Option<Asked>; SLOTS],
    open: [Option<Open>; SLOTS],
    closed: Option<Closed>,
    outputs: Vec<Output>,
    /// Whether each window open is to close as its Cancel would.
    close: [bool; SLOTS],
    /// How the alert asked ended, set from the run loop once its sheet has gone.
    answer: Rc<Cell<Option<Answer>>>,
}

/// A dialog asked for, made by the next callback.
struct Asked {
    dialog: Box<dyn Dialog>,
    /// Whether its window takes the focus as it shows.
    focus: bool,
    /// Where its window was last left.
    place: Option<Position>,
    main: egui::Context,
    at: Instant,
}

/// Where a dialog is in its life: a window's shows and goes at once, a sheet's comes and
/// goes in AppKit's animation, started from the run loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The sheet's presentation scheduled, AppKit's `beginSheet:` not returned yet; to end
    /// once it has, if a close was asked meanwhile.
    Beginning {
        close: bool,
    },
    Shown,
    /// The sheet's end scheduled: its window stays until AppKit's `endSheet:` returns.
    Ending,
}

/// What the host does next with a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// It stays, in this phase.
    Stay(Phase),
    /// Its sheet's end is to be scheduled.
    EndSheet,
    /// It goes, its window with it.
    Gone,
}

/// The next step for a dialog in `phase`, shown as `form`, as a close is asked (`close`)
/// and once AppKit's call for its sheet has returned (`returned`): a window goes at once; a
/// sheet slides in before it can slide away, a close asked meanwhile waiting for it, and
/// goes once it has slid away, a close asked then changing nothing.
fn step(phase: Phase, form: Form, close: bool, returned: bool) -> Step {
    match phase {
        Phase::Beginning { close: asked } if returned => {
            if asked || close {
                Step::EndSheet
            } else {
                Step::Stay(Phase::Shown)
            }
        }
        Phase::Beginning { close: asked } => Step::Stay(Phase::Beginning {
            close: asked || close,
        }),
        Phase::Shown if close && form == Form::Sheet => Step::EndSheet,
        Phase::Shown if close => Step::Gone,
        Phase::Shown => Step::Stay(Phase::Shown),
        Phase::Ending if returned => Step::Gone,
        Phase::Ending => Step::Stay(Phase::Ending),
    }
}

/// The dialog open, in its window.
struct Open {
    slot: Slot,
    dialog: Box<dyn Dialog>,
    /// The form the platform gave it, a modal window where a sheet could not be presented.
    form: Form,
    phase: Phase,
    /// Set as AppKit's call for the sheet returns, from the run loop; the host moves the
    /// phase on at its next callback.
    returned: Rc<Cell<bool>>,
    /// Dropped before the window, as the surface draws into it.
    painter: egui_wgpu::winit::Painter,
    state: egui_winit::State,
    ctx: egui::Context,
    window: Arc<Window>,
    info: egui::ViewportInfo,
    /// The main window's context, whose interface scale the dialog follows.
    main: egui::Context,
    opener: Option<egui::Id>,
    focused: bool,
    /// The title the window was given, in the dialog's language.
    title: String,
    /// The size the content last took, and the one its window was given, in points.
    size: egui::Vec2,
    fitted: Option<egui::Vec2>,
    /// Whether it has drawn a frame.
    painted: bool,
    /// Whether its last frame handed a move to the system, which may keep the press's
    /// release from it.
    moved: bool,
    /// When the dialog's context asked to be drawn again, if it has.
    repaint: Option<Instant>,
    /// The main window's theme as the window last followed it ([`Open::follow_look`]).
    theme: Option<Arc<apertrust_ui::Theme>>,
    /// Whether the person resizes it ([`Dialog::resize`]): then it never takes its
    /// content's size, which takes the window's.
    resizable: bool,
}

impl Host {
    /// The host, sending through `proxy`, the event loop's.
    pub fn new(proxy: EventLoopProxy<eframe::UserEvent>) -> Host {
        Host(Rc::new(RefCell::new(Inner {
            proxy: Arc::new(Mutex::new(proxy)),
            render: None,
            main: Weak::new(),
            asked: Default::default(),
            open: Default::default(),
            closed: None,
            outputs: Vec::new(),
            close: [false; SLOTS],
            answer: Rc::new(Cell::new(None)),
        })))
    }

    /// Gives the host the engine's device, from eframe's `render`, and eframe's window,
    /// `main`, once eframe has made them.
    pub fn attach(&self, render: RenderState, main: Option<Arc<Window>>) {
        let mut inner = self.0.borrow_mut();
        inner.render = Some(render);
        inner.main = main.as_ref().map_or_else(Weak::new, Arc::downgrade);
    }

    /// Whether `window` is one of the host's.
    pub fn owns(&self, window: WindowId) -> bool {
        self.0.borrow().slot_of(window).is_some()
    }

    /// Handles one of the host's windows' events.
    pub fn window_event(
        &self,
        event_loop: &ActiveEventLoop,
        window: WindowId,
        event: &WindowEvent,
    ) {
        let mut inner = self.0.borrow_mut();
        let Some(slot) = inner.slot_of(window) else {
            return;
        };
        // The window an alert rose from takes no input while it asks, and a press on it
        // brings the alert forward, as the main window does under a modal one.
        if inner.held_by_alert() == Some(slot) {
            let under = under_alert(event);
            if under.raises {
                inner.raise_alert();
            }
            if !under.handled {
                return;
            }
        }
        let Some(open) = inner.open[slot.index()].as_mut() else {
            return;
        };
        match event {
            WindowEvent::RedrawRequested => {
                // A sheet on its way out is not drawn again, nor asks for anything.
                if open.phase == Phase::Ending {
                    return;
                }
                let drawn = open.paint(event_loop);
                if open.fit() {
                    open.window.request_redraw();
                }
                if let Some(output) = drawn.output {
                    inner.output(slot, output);
                }
                if let Some(unread) = drawn.unread {
                    inner.output(slot, Output::Keys(unread));
                }
                if drawn.close {
                    inner.advance(slot, true);
                }
                return;
            }
            WindowEvent::CloseRequested => return inner.advance(slot, true),
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    open.painter
                        .on_window_resized(ViewportId::ROOT, width, height);
                }
                // Not the size it was given: on Wayland a size asked for before the
                // compositor's first configure is lost, the window kept at its first
                // guess. Asked again on the next frame, it holds.
                if !open.fits(*size) {
                    open.fitted = None;
                    open.window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                open.focused = *focused;
                // The menus and the actions follow the focused window.
                open.main.request_repaint();
            }
            _ => {}
        }
        if open.state.on_window_event(&open.window, event).repaint {
            open.window.request_redraw();
        }
    }

    /// Takes a user event of the host's, a window's repaint or AccessKit request, and
    /// gives back any other, eframe's.
    pub fn user_event(&self, event: eframe::UserEvent) -> Option<eframe::UserEvent> {
        let mut inner = self.0.borrow_mut();
        match event {
            eframe::UserEvent::RequestRepaint {
                viewport_id, when, ..
            } if Slot::of_viewport(viewport_id).is_some() => {
                let slot = Slot::of_viewport(viewport_id)?;
                if let Some(open) = inner.open[slot.index()].as_mut() {
                    open.repaint = Some(open.repaint.map_or(when, |at| at.min(when)));
                }
                None
            }
            eframe::UserEvent::AccessKitActionRequest(request) => {
                let Some(slot) = inner.slot_of(request.window_id) else {
                    return Some(eframe::UserEvent::AccessKitActionRequest(request));
                };
                // A screen reader's actions are input too, which the window an alert rose
                // from takes none of.
                let held = inner.held_by_alert() == Some(slot);
                let open = inner.open[slot.index()].as_mut()?;
                use egui_winit::accesskit_winit::WindowEvent as Access;
                match request.window_event {
                    Access::InitialTreeRequested => open.ctx.enable_accesskit(),
                    Access::ActionRequested(_) if held => {}
                    Access::ActionRequested(request) => {
                        open.state.on_accesskit_action_request(request);
                    }
                    Access::AccessibilityDeactivated => open.ctx.disable_accesskit(),
                }
                open.window.request_redraw();
                None
            }
            event => Some(event),
        }
    }

    /// After each of eframe's callbacks: moves a sheet on once AppKit's call for it has
    /// returned, closes the windows asked to close, makes the ones asked for, follows the
    /// interface's scale, and asks for each window's frame once it is due.
    pub fn run(&self, event_loop: &ActiveEventLoop) {
        let mut inner = self.0.borrow_mut();
        let close = std::mem::take(&mut inner.close);
        for slot in Slot::ALL {
            inner.advance(slot, close[slot.index()]);
        }
        for slot in Slot::ALL {
            if let Some(asked) = inner.asked[slot.index()].take() {
                inner.make(event_loop, slot, asked);
            }
        }
        for open in inner.open.iter_mut().flatten() {
            open.follow_scale();
            open.follow_look();
            if open.repaint.is_some_and(|at| at <= Instant::now()) {
                open.repaint = None;
                open.window.request_redraw();
            }
        }
    }

    /// Before the loop waits: wakes it when a window's next frame is due, or eframe's,
    /// whichever comes first.
    pub fn wait(&self, event_loop: &ActiveEventLoop) {
        let inner = self.0.borrow();
        let due = inner.open.iter().flatten().filter_map(|open| open.repaint);
        let Some(at) = due.min() else {
            return;
        };
        match event_loop.control_flow() {
            ControlFlow::Poll => {}
            ControlFlow::Wait => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            ControlFlow::WaitUntil(eframe) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(eframe.min(at)));
            }
        }
    }

    /// As the loop ends: lets the windows open go with it, with no animation, since the run
    /// loop runs no more.
    pub fn exiting(&self) {
        self.0.borrow_mut().open = Default::default();
    }
}

impl Dialogs for Host {
    fn open(&mut self, dialog: Box<dyn Dialog>, place: Option<Position>, main: &egui::Context) {
        let mut inner = self.0.borrow_mut();
        let slot = Slot::Dialog.index();
        match &inner.open[slot] {
            Some(open) if open.form == Form::Modeless && open.dialog.id() == dialog.id() => {
                platform::activate(&open.window);
            }
            Some(_) => {}
            None => {
                inner.asked[slot] = Some(Asked {
                    dialog,
                    focus: true,
                    place,
                    main: main.clone(),
                    at: Instant::now(),
                });
            }
        }
    }

    fn shown(&self) -> Option<Shown> {
        let inner = self.0.borrow();
        inner.open[Slot::Alert.index()]
            .as_ref()
            .or(inner.open[Slot::Dialog.index()].as_ref())
            .map(|open| Shown {
                form: open.form,
                focused: open.focused,
            })
    }

    fn alert_shown(&self) -> bool {
        let inner = self.0.borrow();
        let alert = Slot::Alert.index();
        inner.open[alert].is_some() || inner.asked[alert].is_some()
    }

    fn raise(&mut self) {
        let inner = self.0.borrow();
        let open = inner.open[Slot::Alert.index()]
            .as_ref()
            .or(inner.open[Slot::Dialog.index()].as_ref());
        if let Some(open) = open
            && open.form == Form::Modal
            && !open.focused
        {
            platform::activate(&open.window);
        }
    }

    fn closed(&mut self) -> Option<Closed> {
        self.0.borrow_mut().closed.take()
    }

    fn outputs(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.0.borrow_mut().outputs)
    }

    fn close(&mut self) {
        let mut inner = self.0.borrow_mut();
        let alert = Slot::Alert.index();
        if inner.open[alert].is_some() || inner.asked[alert].is_some() {
            inner.asked[alert] = None;
            inner.close[alert] = true;
        } else {
            inner.close[Slot::Dialog.index()] = true;
        }
    }

    fn preview(&mut self, through: u64, preview: ExportPreview) {
        let mut inner = self.0.borrow_mut();
        if let Some(open) = inner.open[Slot::Dialog.index()].as_mut()
            && let Some(export) = dialog_mut::<export::Export>(&mut open.dialog)
        {
            export.receive(through, preview);
            open.window.request_redraw();
        }
    }

    fn open_jobs(
        &mut self,
        window: jobs::Window,
        place: Option<Position>,
        main: &egui::Context,
        focus: bool,
    ) {
        let mut inner = self.0.borrow_mut();
        let slot = Slot::Jobs.index();
        match &inner.open[slot] {
            Some(open) => {
                if focus {
                    platform::activate(&open.window);
                }
            }
            None => {
                // Asked twice before it is made, the stronger ask stays: a click's focus
                // is never lost to an automatic open after it.
                let focus = focus || inner.asked[slot].as_ref().is_some_and(|a| a.focus);
                inner.asked[slot] = Some(Asked {
                    dialog: Box::new(window),
                    focus,
                    place,
                    main: main.clone(),
                    at: Instant::now(),
                });
            }
        }
    }

    fn jobs_shown(&self) -> Option<Shown> {
        let inner = self.0.borrow();
        inner.open[Slot::Jobs.index()].as_ref().map(|open| Shown {
            form: open.form,
            focused: open.focused,
        })
    }

    fn set_jobs(&mut self, jobs: Vec<Job>) {
        let mut inner = self.0.borrow_mut();
        let slot = Slot::Jobs.index();
        if let Some(open) = inner.open[slot].as_mut() {
            if dialog_mut::<jobs::Window>(&mut open.dialog).is_some_and(|window| window.set(jobs)) {
                open.window.request_redraw();
            }
        } else if let Some(asked) = inner.asked[slot].as_mut()
            && let Some(window) = dialog_mut::<jobs::Window>(&mut asked.dialog)
        {
            window.set(jobs);
        }
    }

    fn close_jobs(&mut self) {
        self.0.borrow_mut().close[Slot::Jobs.index()] = true;
    }

    fn open_log(
        &mut self,
        window: log_window::Window,
        place: Option<Position>,
        main: &egui::Context,
    ) {
        let mut inner = self.0.borrow_mut();
        let slot = Slot::Log.index();
        match &inner.open[slot] {
            Some(open) => platform::activate(&open.window),
            None => {
                inner.asked[slot] = Some(Asked {
                    dialog: Box::new(window),
                    focus: true,
                    place,
                    main: main.clone(),
                    at: Instant::now(),
                });
            }
        }
    }

    fn log_shown(&self) -> Option<Shown> {
        let inner = self.0.borrow();
        inner.open[Slot::Log.index()].as_ref().map(|open| Shown {
            form: open.form,
            focused: open.focused,
        })
    }

    fn close_log(&mut self) {
        self.0.borrow_mut().close[Slot::Log.index()] = true;
    }

    fn ask(&mut self, request: &alert::Request, main: &egui::Context) -> Ask {
        let mut inner = self.0.borrow_mut();
        if platform::draws_alerts(main.os()) {
            let Inner {
                asked,
                open,
                answer,
                ..
            } = &mut *inner;
            let slot = Slot::Alert.index();
            return ask_drawn(
                &mut asked[slot],
                open[slot].is_some(),
                request,
                answer,
                main,
            );
        }
        // The modal dialog's window if one is up, else the main window: a window with a
        // sheet already would queue the alert behind it, and the main window takes no
        // input under a modal one.
        let sheet = inner.open[Slot::Dialog.index()]
            .as_ref()
            .filter(|open| open.form.modal() && open.phase == Phase::Shown)
            .map(|open| open.window.clone());
        let Some(parent) = sheet.or_else(|| inner.main.upgrade()) else {
            tracing::warn!(target: "gpu", "alert not shown: the main window is not up yet");
            return Ask::Failed;
        };
        let buttons: Vec<apertrust_display::AlertButton<'_>> = request
            .buttons
            .iter()
            .enumerate()
            .map(|(i, button)| apertrust_display::AlertButton {
                default: i == request.default,
                title: &button.label,
                role: match button.role {
                    alert::Role::Default => apertrust_display::ButtonRole::Default,
                    alert::Role::Cancel => apertrust_display::ButtonRole::Cancel,
                    alert::Role::Destructive => apertrust_display::ButtonRole::Destructive,
                },
            })
            .collect();
        let (answer, wake) = (Rc::clone(&inner.answer), main.clone());
        let answered = move |pressed: Option<usize>, ticked: bool| {
            answer.set(Some(Answer::of(pressed, ticked)));
            wake.request_repaint();
        };
        match apertrust_display::begin_alert(
            &*parent,
            &request.title,
            &request.message,
            &buttons,
            request.suppression.as_deref(),
            answered,
        ) {
            Ok(true) => Ask::Shown,
            Ok(false) => Ask::NoAlerts,
            Err(e) => {
                tracing::warn!(target: "gpu", "alert not shown: {e}");
                Ask::Failed
            }
        }
    }

    fn answer(&mut self) -> Option<Answer> {
        self.0.borrow().answer.take()
    }
}

impl Inner {
    /// The window of the host's with the id `window`, if it is one's.
    fn slot_of(&self, window: WindowId) -> Option<Slot> {
        Slot::ALL.into_iter().find(|slot| {
            self.open[slot.index()]
                .as_ref()
                .is_some_and(|open| open.window.id() == window)
        })
    }

    /// The window of the host's an alert rises from, the modal dialog's while it is shown;
    /// none, for the main window.
    fn alert_parent(&self) -> Option<Arc<Window>> {
        self.open[Slot::Dialog.index()]
            .as_ref()
            .filter(|open| open.form.modal() && open.phase == Phase::Shown)
            .map(|open| open.window.clone())
    }

    /// The host's window an alert open or asked rose from ([`Inner::alert_parent`]), which
    /// takes no input while it asks; none where it rose from the main window, which the
    /// app's `raw_input_hook` holds, or where no alert asks.
    fn held_by_alert(&self) -> Option<Slot> {
        let alert = Slot::Alert.index();
        let asks = self.open[alert].is_some() || self.asked[alert].is_some();
        (asks && self.alert_parent().is_some()).then_some(Slot::Dialog)
    }

    /// Brings the alert open forward, as a press on the window it rose from asks.
    fn raise_alert(&self) {
        if let Some(open) = &self.open[Slot::Alert.index()] {
            platform::activate(&open.window);
        }
    }

    /// Makes the window `asked` for in `slot` ([`Inner::make_window`]). An alert whose window
    /// cannot be made answers [`Answer::Failed`], as the system's alert that cannot be shown
    /// does, rather than seem dismissed by the person.
    fn make(&mut self, event_loop: &ActiveEventLoop, slot: Slot, asked: Asked) {
        let main = asked.main.clone();
        if !self.make_window(event_loop, slot, asked) && slot == Slot::Alert {
            // Whenever the alert's dialog was dropped, before or after this: its drop
            // answers dismissed only where no answer is set, and this one is set over it.
            self.answer.set(Some(Answer::Failed));
            main.request_repaint();
        }
    }

    /// Makes the window `asked` for in `slot` and shows it once its first frame is drawn at
    /// its content's size, logging under `perf` how long that took. Returns whether it was
    /// made; why not is logged.
    fn make_window(&mut self, event_loop: &ActiveEventLoop, slot: Slot, asked: Asked) -> bool {
        let (Some(render), Some(main)) = (self.render.clone(), self.main.upgrade()) else {
            tracing::warn!(target: "gpu", "dialog not shown: the main window is not up yet");
            return false;
        };
        let Asked {
            dialog,
            focus,
            place,
            main: from,
            at,
        } = asked;
        let zoom = from.zoom_factor();
        let ctx = context(&from, &self.proxy, slot.viewport());
        let title = dialog.title(&from);
        let os = from.os();
        // Decided once, from the main window: X11 or Wayland is known only at run time.
        let shadowed = platform::shadowed(os, platform::wayland(&main));
        set_shadow(&ctx, shadowed);
        let form = platform::form(dialog.form(os), os);
        let parent = match slot {
            Slot::Alert => self.alert_parent().unwrap_or_else(|| main.clone()),
            _ => main.clone(),
        };
        // A first guess, which the first frame corrects before the window shows.
        let guess = egui::vec2(400.0, 300.0) * zoom;
        let logical = |size: egui::Vec2| LogicalSize::new(f64::from(size.x), f64::from(size.y));
        let attributes = Window::default_attributes()
            .with_title(&title)
            .with_inner_size(logical(guess))
            .with_resizable(false)
            .with_enabled_buttons(WindowButtons::CLOSE)
            .with_visible(false)
            .with_transparent(shadowed);
        // One the person resizes opens at its size, never smaller than its least, the
        // frame the host draws on top of both.
        let resize = dialog.resize();
        let attributes = match resize {
            Some(resize) => {
                let frame = chrome(&ctx);
                attributes
                    .with_resizable(true)
                    .with_inner_size(logical((resize.size + frame) * zoom))
                    .with_min_inner_size(logical((resize.least + frame) * zoom))
            }
            None => attributes,
        };
        let attributes = platform::attributes(attributes, form, &parent);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                tracing::error!(target: "gpu", "dialog not shown: {e}");
                return false;
            }
        };
        if let Err(e) = apertrust_display::keep_out_of_tabs(&*window) {
            tracing::warn!(target: "gpu", "dialog may open as a tab: {e}");
        }
        platform::made(&window, form, &parent);
        let mut state = egui_winit::State::new(
            ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(render.device.limits().max_texture_dimension_2d as usize),
        );
        // Before the window first shows, as accesskit_winit asks.
        let proxy = self
            .proxy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        // A proxy of the waker's own, so no lock another holds keeps a line waiting.
        let mut dialog = dialog;
        dialog.wake_with(Waker::from(Arc::new(Wakes {
            proxy: Mutex::new(proxy.clone()),
            viewport: slot.viewport(),
        })));
        state.init_accesskit(event_loop, &window, proxy);
        let configuration = egui_wgpu::WgpuConfiguration {
            wgpu_setup: WgpuSetup::Existing(WgpuSetupExisting {
                instance: render.instance.clone(),
                adapter: render.adapter.clone(),
                device: render.device.clone(),
                queue: render.queue.clone(),
            }),
            ..Default::default()
        };
        let options = egui_wgpu::RendererOptions::default();
        let painter = ready(egui_wgpu::winit::Painter::new(
            ctx.clone(),
            configuration,
            shadowed,
            options,
        ));
        let Some(mut painter) = painter else {
            tracing::error!(target: "gpu", "dialog not shown: its painter waited");
            return false;
        };
        // The surface, configured on the engine's device: this waits for what the engine
        // has submitted to finish.
        match ready(painter.set_window(ViewportId::ROOT, Some(window.clone()))) {
            Some(Ok(())) => {}
            Some(Err(e)) => {
                let e = apertrust_base::log::one_line(&e.to_string());
                tracing::error!(target: "gpu", "dialog not shown: {e}");
                return false;
            }
            None => {
                tracing::error!(target: "gpu", "dialog not shown: its surface waited");
                return false;
            }
        }
        let opener = from.memory(|m| m.focused());
        let mut open = Open {
            slot,
            dialog,
            form,
            phase: Phase::Shown,
            returned: Rc::new(Cell::new(false)),
            painter,
            state,
            ctx,
            window,
            info: egui::ViewportInfo::default(),
            main: from,
            opener,
            focused: false,
            title,
            size: egui::Vec2::ZERO,
            fitted: None,
            painted: false,
            moved: false,
            repaint: None,
            theme: None,
            resizable: resize.is_some(),
        };
        // Drawn before it shows, at its content's size, so it never shows empty or
        // stretched: AppKit draws no hidden window, so these frames are drawn here rather
        // than on a redraw.
        let mut drawn = open.paint(event_loop);
        if open.fit() {
            drawn = open.paint(event_loop);
        }
        open.show(&parent, place, &self.proxy, focus);
        let ms = at.elapsed().as_secs_f64() * 1000.0;
        let (name, form) = (open.dialog.id(), open.form.name());
        tracing::debug!(target: "perf", "dialog {name}: {form}, first frame in {ms:.1} ms");
        self.open[slot.index()] = Some(open);
        if let Some(output) = drawn.output {
            self.output(slot, output);
        }
        if drawn.close {
            self.advance(slot, true);
        }
        true
    }

    /// Keeps what the dialog's content asked of the app, and wakes the main window to
    /// take it.
    fn output(&mut self, slot: Slot, output: Output) {
        self.outputs.push(output);
        if let Some(open) = &self.open[slot.index()] {
            open.main.request_repaint();
        }
    }

    /// Moves the window open in `slot` on ([`step`]), as a close is asked (`close`) and once
    /// AppKit's call for its sheet has returned: a window goes at once; a sheet slides away
    /// from the run loop once it has slid in, its window kept until it has gone.
    fn advance(&mut self, slot: Slot, close: bool) {
        let Some(open) = self.open[slot.index()].as_mut() else {
            return;
        };
        let returned = open.returned.replace(false);
        match step(open.phase, open.form, close, returned) {
            Step::Stay(phase) => open.phase = phase,
            Step::EndSheet => {
                let ending = self.main.upgrade().map(|parent| {
                    let done = open.returned_then_wake(&self.proxy);
                    apertrust_display::end_sheet(&*parent, &*open.window, done)
                });
                match ending {
                    Some(Ok(true)) => open.phase = Phase::Ending,
                    Some(Err(e)) => {
                        tracing::warn!(target: "gpu", "dialog sheet not ended: {e}");
                        self.gone(slot);
                    }
                    Some(Ok(false)) | None => self.gone(slot),
                }
            }
            Step::Gone => self.gone(slot),
        }
    }

    /// The window open in `slot` goes, with its window, the place of a modeless one kept
    /// for the next time where its dialog keeps it ([`Dialog::placed`]), and the main window
    /// is asked for a frame, for the app to give the focus back where it should go back
    /// ([`Closed`]).
    fn gone(&mut self, slot: Slot) {
        let Some(open) = self.open[slot.index()].take() else {
            return;
        };
        let Open {
            dialog,
            form,
            painter,
            state,
            window,
            main,
            opener,
            ctx,
            resizable,
            ..
        } = open;
        if form == Form::Modeless
            && let Ok(at) = window.outer_position()
        {
            let place = Position {
                pixels: [at.x as f32, at.y as f32],
                scale_factor: window.scale_factor() as f32,
            };
            self.outputs.extend(dialog.placed(place));
        }
        // A window the person resized: its size in points, the host's frame left out.
        if resizable {
            let logical = window.inner_size().to_logical::<f32>(window.scale_factor());
            let points = egui::vec2(logical.width, logical.height) / ctx.zoom_factor();
            self.outputs.extend(dialog.sized(points - chrome(&ctx)));
        }
        drop(painter);
        drop(state);
        drop(window);
        let opener = given_back(form, opener, main.memory(|m| m.focused()));
        // A modal window's parent takes the focus back where the system may not give it:
        // for an alert, that is the dialog it rose from, else the main window.
        let parent = match slot {
            Slot::Alert => self.alert_parent().or_else(|| self.main.upgrade()),
            _ => self.main.upgrade(),
        };
        if form == Form::Modal
            && !cfg!(target_os = "windows")
            && let Some(window) = parent
        {
            window.focus_window();
        }
        let (name, form) = (dialog.id(), form.name());
        tracing::debug!(target: "gpu", "dialog {name}: {form} closed");
        if slot != Slot::Alert {
            self.closed = Some(Closed { opener });
        }
        main.request_repaint();
    }
}

impl Open {
    /// Draws one frame of the dialog and returns what its content asked for, keeping its
    /// size; the window's title follows its language.
    fn paint(&mut self, event_loop: &ActiveEventLoop) -> Drawn {
        // wgpu leaves the surface untagged, and macOS then shows it without colour
        // management; tagged again each frame, as it clears the tag as it configures.
        if let Err(e) = apertrust_display::tag_window(&*self.window, ColorSpace::Srgb) {
            tracing::warn!(target: "gpu", "dialog colour space untagged: {e}");
        }
        let first = !std::mem::replace(&mut self.painted, true);
        egui_winit::update_viewport_info(&mut self.info, &self.ctx, &self.window, first);
        let mut input = self.state.take_egui_input(&self.window);
        if std::mem::take(&mut self.moved) {
            crate::release_after_drag(&self.ctx, &mut input);
        }
        input.viewports = std::iter::once((ViewportId::ROOT, self.info.clone())).collect();
        // Where the host draws the frame, a window the person resizes resizes from the
        // bands inside its edges, as the main window does there.
        let banded = self.resizable && platform::draws_frame(self.ctx.os());
        let band = resize_band(&self.ctx);
        if banded && let Some(way) = crate::resize_press(&self.ctx, &mut input, band) {
            match self.window.drag_resize_window(resize_direction(way)) {
                Ok(()) => self.moved = true,
                Err(e) => tracing::warn!(target: "gpu", "dialog not resized: {e}"),
            }
        }
        let mut drawn = None;
        let dialog = self.dialog.as_mut();
        let output = self.ctx.run_ui(input, |ui| {
            drawn = Some(draw(ui, dialog));
            if banded {
                crate::resize_hover(ui.ctx(), band);
            }
        });
        crate::popups::show(&self.ctx, &*self.window);
        self.info.events.clear();
        self.state.handle_platform_output_with_event_loop(
            &self.window,
            event_loop,
            output.platform_output,
        );
        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let t = theme(&self.ctx);
        let background = if shadow(&self.ctx) {
            egui::Color32::TRANSPARENT
        } else {
            self.dialog.look(&t).background
        };
        let clear = background.to_normalized_gamma_f32();
        let mut textures = output.textures_delta;
        self.painter.paint_and_update_textures(
            ViewportId::ROOT,
            output.pixels_per_point,
            clear,
            &primitives,
            &mut textures,
            Vec::new(),
            &self.window,
        );
        let title = self.dialog.title(&self.ctx);
        if title != self.title {
            self.window.set_title(&title);
            self.title = title;
        }
        let drawn = drawn.unwrap_or(Drawn {
            output: None,
            close: false,
            size: self.size,
            unread: None,
            gesture: None,
        });
        self.size = drawn.size;
        self.gesture(drawn.gesture, output.pixels_per_point);
        drawn
    }

    /// Does what a press on the frame the host draws asked of the window: a move through
    /// the system, or its window menu at the press, `pixels_per_point` turning points into
    /// the window's pixels.
    fn gesture(&mut self, gesture: Option<Gesture>, pixels_per_point: f32) {
        match gesture {
            Some(Gesture::Move) => match self.window.drag_window() {
                Ok(()) => self.moved = true,
                Err(e) => tracing::warn!(target: "gpu", "dialog not moved: {e}"),
            },
            Some(Gesture::Menu(at)) => {
                let at = at * pixels_per_point;
                self.window
                    .show_window_menu(PhysicalPosition::new(f64::from(at.x), f64::from(at.y)));
            }
            None => {}
        }
    }

    /// Whether a window of `size` has the size the host last gave it, to the pixel, or was
    /// given none yet.
    fn fits(&self, size: PhysicalSize<u32>) -> bool {
        let Some(fitted) = self.fitted else {
            return true;
        };
        let logical = fitted * self.ctx.zoom_factor();
        let wanted: PhysicalSize<u32> =
            LogicalSize::new(f64::from(logical.x), f64::from(logical.y))
                .to_physical(self.window.scale_factor());
        wanted.width.abs_diff(size.width) <= 1 && wanted.height.abs_diff(size.height) <= 1
    }

    /// Gives the window its content's size at the interface's scale, in whole points, as
    /// its last frame measured it; a window the person resizes keeps its own. Returns
    /// whether it changed, for the frame to be drawn
    /// again.
    fn fit(&mut self) -> bool {
        let size = self.size.ceil();
        if self.resizable || self.fitted == Some(size) || size.min_elem() <= 0.0 {
            return false;
        }
        self.fitted = Some(size);
        let logical = size * self.ctx.zoom_factor();
        let logical = LogicalSize::new(f64::from(logical.x), f64::from(logical.y));
        // AppKit resizes it at once, though winit says nothing until its resize event: the
        // surface follows now, so a hidden window is drawn again at its size before it
        // shows. Where the platform resizes it later, its resize event brings the surface.
        let before = self.window.inner_size();
        // A window the person cannot resize is held at its size by its minimum and its
        // maximum, which Wayland's winit sets to the first guess as the window is made: they
        // move with it, or the compositor keeps the guess.
        self.window.set_min_inner_size(None::<LogicalSize<f64>>);
        self.window.set_max_inner_size(None::<LogicalSize<f64>>);
        let _ = self.window.request_inner_size(logical);
        self.window.set_min_inner_size(Some(logical));
        self.window.set_max_inner_size(Some(logical));
        let now = self.window.inner_size();
        if now != before
            && let (Some(width), Some(height)) =
                (NonZeroU32::new(now.width), NonZeroU32::new(now.height))
        {
            self.painter
                .on_window_resized(ViewportId::ROOT, width, height);
        }
        true
    }

    /// Shows the window, its first frame drawn: as a sheet on `main`, presented from the
    /// run loop; or, modeless, at `place`, where it was last left, if that is on a screen
    /// still; else centred on `main`, as a modal window always is and a sheet becomes where
    /// it cannot be presented ([`platform::unsheeted`]).
    fn show(
        &mut self,
        main: &Window,
        place: Option<Position>,
        proxy: &Arc<Mutex<EventLoopProxy<eframe::UserEvent>>>,
        focus: bool,
    ) {
        if self.form == Form::Sheet {
            let begun = self.returned_then_wake(proxy);
            match apertrust_display::begin_sheet(main, &*self.window, begun) {
                Ok(true) => {
                    self.phase = Phase::Beginning { close: false };
                    return;
                }
                Ok(false) => {
                    tracing::warn!(target: "gpu", "dialog sheet: no sheets here, a modal window");
                }
                Err(e) => {
                    tracing::warn!(target: "gpu", "dialog sheet: a modal window instead: {e}");
                }
            }
            // Still modal, as the app is told: the main window takes no input meanwhile.
            self.form = platform::unsheeted(self.form);
        }
        let size = self
            .window
            .inner_size()
            .to_logical::<f32>(self.window.scale_factor());
        let monitors = crate::monitors(&self.window);
        let mac = cfg!(target_os = "macos");
        let place = place.filter(|_| self.form == Form::Modeless);
        let kept =
            place.and_then(|at| crate::on_screen(&at, [size.width, size.height], &monitors, mac));
        if let Some(spot) = kept {
            crate::move_to(&self.window, spot);
        } else if let Ok(at) = main.outer_position() {
            let (main_size, size) = (main.outer_size(), self.window.outer_size());
            let x = f64::from(at.x) + (f64::from(main_size.width) - f64::from(size.width)) / 2.0;
            let y = f64::from(at.y) + (f64::from(main_size.height) - f64::from(size.height)) / 2.0;
            self.window
                .set_outer_position(PhysicalPosition::new(x.round(), y.round()));
        }
        // A window the person did not ask for is put in front without being made the key
        // window, which winit's `set_visible` does on macOS; elsewhere it shows as usual.
        let quietly = !focus
            && matches!(
                apertrust_display::show_without_focus(&*self.window),
                Ok(true)
            );
        if !quietly {
            self.window.set_visible(true);
        }
        if focus {
            self.window.focus_window();
        }
    }

    /// What AppKit's call for the sheet runs once it has returned: marks it returned, and
    /// wakes the loop, which may be waiting, so that the host's next callback moves on.
    fn returned_then_wake(
        &self,
        proxy: &Arc<Mutex<EventLoopProxy<eframe::UserEvent>>>,
    ) -> impl FnOnce() + 'static {
        let returned = Rc::clone(&self.returned);
        let proxy = Arc::clone(proxy);
        let viewport = self.slot.viewport();
        move || {
            returned.set(true);
            wake(&proxy, viewport);
        }
    }

    /// Follows the main window's interface scale, which the dialog's points are scaled by:
    /// its context's zoom, and its window's size from the next frame.
    fn follow_scale(&mut self) {
        let zoom = self.main.zoom_factor();
        if zoom == self.ctx.zoom_factor() {
            return;
        }
        self.ctx.options_mut(|o| o.zoom_factor = zoom);
        self.fitted = None;
        self.window.request_redraw();
    }
}

/// What the window an alert rose from does with an event while the alert asks
/// ([`under_alert`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UnderAlert {
    /// It handles it as usual.
    handled: bool,
    /// It brings the alert forward.
    raises: bool,
}

/// What the window an alert rose from does with `event` while the alert asks: it is drawn,
/// sized, scaled and themed, and keeps its focus, the modifiers held and the pointer's
/// leaving as usual, but takes no pointer or keyboard input, nor files dropped on it. A
/// press on it, its taking the focus or a close asked of it, which it ignores, brings the
/// alert forward.
fn under_alert(event: &WindowEvent) -> UnderAlert {
    use egui_winit::winit::event::{ElementState, TouchPhase};
    let handled = matches!(
        event,
        WindowEvent::RedrawRequested
            | WindowEvent::Resized(_)
            | WindowEvent::Moved(_)
            | WindowEvent::ScaleFactorChanged { .. }
            | WindowEvent::ThemeChanged(_)
            | WindowEvent::Occluded(_)
            | WindowEvent::Destroyed
            | WindowEvent::Focused(_)
            | WindowEvent::ModifiersChanged(_)
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::ActivationTokenDone { .. }
    );
    let raises = match event {
        WindowEvent::Focused(true) | WindowEvent::CloseRequested => true,
        WindowEvent::MouseInput { state, .. } => *state == ElementState::Pressed,
        WindowEvent::Touch(touch) => touch.phase == TouchPhase::Started,
        _ => false,
    };
    UnderAlert { handled, raises }
}

/// Asks for `request` as an alert the host draws ([`platform::draws_alerts`]), made by the
/// next callback in the alert's slot, `asked`, its answer kept in `answer` and `main` asked
/// for a frame as it goes. One at a time: while one is asked or `open`, nothing is asked.
fn ask_drawn(
    asked: &mut Option<Asked>,
    open: bool,
    request: &alert::Request,
    answer: &Rc<Cell<Option<Answer>>>,
    main: &egui::Context,
) -> Ask {
    if open || asked.is_some() {
        return Ask::Busy;
    }
    *asked = Some(Asked {
        dialog: Box::new(alert_dialog::AlertDialog::new(
            request,
            Rc::clone(answer),
            main,
        )),
        focus: true,
        place: None,
        main: main.clone(),
        at: Instant::now(),
    });
    Ask::Shown
}

impl Open {
    /// Follows the main window's theme and language, for the windows that stay open while
    /// Settings changes them (the jobs and the log window); a dialog's own context is
    /// changed by the dialog itself, which the main window then follows, so it is left
    /// alone.
    fn follow_look(&mut self) {
        if self.slot == Slot::Dialog {
            return;
        }
        let theme = apertrust_ui::theme(&self.main);
        if !self
            .theme
            .as_ref()
            .is_some_and(|seen| Arc::ptr_eq(seen, &theme))
        {
            if *theme != *apertrust_ui::theme(&self.ctx) {
                apertrust_ui::set_theme(&self.ctx, (*theme).clone());
                self.window.request_redraw();
            }
            self.theme = Some(theme);
        }
        let locale = apertrust_ui::translator(&self.main).locale();
        if locale != apertrust_ui::translator(&self.ctx).locale() {
            apertrust_ui::set_locale(&self.ctx, locale);
            self.window.request_redraw();
        }
    }
}

/// The control of the main window the focus goes back to as a dialog shown as `form` goes,
/// where `opener` had the focus as the dialog was asked for and `now` has it: after a
/// modal dialog, the opener; after a modeless window, beside which the main window takes
/// input, the opener only where the focus has not moved since, so a control the user chose
/// keeps it.
fn given_back(form: Form, opener: Option<egui::Id>, now: Option<egui::Id>) -> Option<egui::Id> {
    if form.modal() {
        opener
    } else {
        opener.filter(|_| now == opener)
    }
}

/// `dialog` as the dialog type `T` it is, if it is: for the messages the app hands one
/// dialog, Export's preview and the jobs window's rows.
fn dialog_mut<T: Dialog>(dialog: &mut Box<dyn Dialog>) -> Option<&mut T> {
    (dialog.as_mut() as &mut dyn Any).downcast_mut()
}

/// The waker a dialog is handed ([`Dialog::wake_with`]): it asks for its window's frame
/// from any thread, as its context's repaint does, through a proxy of its own.
struct Wakes {
    proxy: Mutex<EventLoopProxy<eframe::UserEvent>>,
    viewport: ViewportId,
}

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        wake(&self.proxy, self.viewport);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        wake(&self.proxy, self.viewport);
    }
}

/// Wakes the event loop for the host's next callback, as a window's repaint does.
fn wake(proxy: &Mutex<EventLoopProxy<eframe::UserEvent>>, viewport: ViewportId) {
    let repaint = eframe::UserEvent::RequestRepaint {
        viewport_id: viewport,
        when: Instant::now(),
        cumulative_pass_nr: 0,
    };
    let proxy = proxy
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ = proxy.send_event(repaint);
}

/// A dialog's egui context, as the main window's, `from`: its fonts, theme, language and
/// interface scale, the registry's keys left alone as the main window leaves them, its
/// popups the Mac's menus over its own window, and its repaints asked under `viewport`
/// through `proxy`.
fn context(
    from: &egui::Context,
    proxy: &Arc<Mutex<EventLoopProxy<eframe::UserEvent>>>,
    viewport: ViewportId,
) -> egui::Context {
    let ctx = egui::Context::default();
    apertrust_ui::install(&ctx, (*apertrust_ui::theme(from)).clone());
    apertrust_ui::components::set_marker(&ctx, apertrust_ui::components::marker(from));
    apertrust_ui::set_locale(&ctx, apertrust_ui::translator(from).locale());
    ctx.options_mut(|o| {
        o.zoom_factor = from.zoom_factor();
        o.zoom_with_keyboard = false;
        o.quit_shortcuts.clear();
    });
    crate::popups::install(&ctx);
    let proxy = Arc::clone(proxy);
    ctx.set_request_repaint_callback(move |info| {
        let repaint = eframe::UserEvent::RequestRepaint {
            viewport_id: viewport,
            when: Instant::now() + info.delay,
            cumulative_pass_nr: info.current_cumulative_pass_nr,
        };
        let proxy = proxy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = proxy.send_event(repaint);
    });
    ctx
}

/// What `future` gives, if it is ready at once, as egui-wgpu's painter is on a device it
/// is handed: it waits for nothing, so the UI thread never blocks on it.
fn ready<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    match future.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(value) => Some(value),
        std::task::Poll::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{self, Keeper};
    use apertrust_ui::{Builtin, Theme};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    #[test]
    fn an_alert_s_answer_is_the_button_and_whether_the_box_was_ticked() {
        assert_eq!(Answer::of(Some(0), false), Answer::Pressed(0));
        assert_eq!(Answer::of(Some(0), true), Answer::Ticked(0));
        assert_eq!(Answer::of(Some(1), true), Answer::Ticked(1));
        // Taken away, there is no answer to tick.
        assert_eq!(Answer::of(None, true), Answer::Dismissed);
        assert!(Answer::Ticked(0).confirmed() && Answer::Ticked(0).ticked());
        assert!(Answer::Pressed(0).confirmed() && !Answer::Pressed(0).ticked());
        assert!(!Answer::Ticked(1).confirmed() && Answer::Ticked(1).ticked());
        assert!(!Answer::Dismissed.confirmed() && !Answer::Dismissed.ticked());
    }

    #[test]
    fn a_drawn_alert_taken_away_answers_dismissed_and_wakes_the_app() {
        let answer = Rc::new(Cell::new(None));
        let request = alert::Request::new("Title", "Message").button("OK", alert::Role::Default);
        let wake = egui::Context::default();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let called = Arc::clone(&calls);
        wake.set_request_repaint_callback(move |_| {
            called.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        let dialog = alert_dialog::AlertDialog::new(&request, Rc::clone(&answer), &wake);
        assert_eq!(dialog.form(OperatingSystem::Nix), Form::Modal);
        drop(dialog);
        assert_eq!(answer.get(), Some(Answer::Dismissed));
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn a_drawn_alert_s_window_is_one_alert_dialog_named_once_by_its_title() {
        let answer = Rc::new(Cell::new(None));
        let request = alert::Request::new("Overwrite?", "The old files go.")
            .button("Overwrite", alert::Role::Destructive)
            .button("Cancel", alert::Role::Cancel);
        let wake = egui::Context::default();
        let dialog = alert_dialog::AlertDialog::new(&request, Rc::clone(&answer), &wake);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(480.0, 240.0))
            .with_os(OperatingSystem::Nix)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, dialog: &mut alert_dialog::AlertDialog| {
                    draw(ui, dialog);
                },
                dialog,
            );
        harness.run();
        let alerts: Vec<String> = harness
            .query_all_by_role(Role::AlertDialog)
            .map(|node| node.accesskit_node().label().unwrap_or_default())
            .collect();
        assert_eq!(alerts, ["Overwrite?"]);
        assert_eq!(harness.query_all_by_role(Role::Dialog).count(), 0);
    }

    /// A dialog's context on `os`, shadowed as the host makes it for a main window on
    /// Wayland or not (`wayland`).
    fn hosted(os: OperatingSystem, wayland: bool) -> egui::Context {
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        ctx.set_os(os);
        set_shadow(&ctx, platform::shadowed(os, wayland));
        ctx
    }

    #[test]
    fn a_window_s_content_s_least_and_kept_sizes_leave_out_its_frame_and_margins() {
        let on = |os, wayland| hosted(os, wayland);
        assert_eq!(chrome(&on(OperatingSystem::Mac, false)), egui::Vec2::ZERO);
        assert_eq!(
            chrome(&on(OperatingSystem::Windows, false)),
            egui::Vec2::ZERO
        );
        // X11: the strip above alone.
        let x11 = chrome(&on(OperatingSystem::Nix, false));
        assert_eq!(x11, egui::vec2(0.0, FRAME));
        // Wayland: the strip, and the shadow's margin and the border on each side.
        let side = 2.0 * (MARGIN + Theme::default().shape.hairline);
        let wayland = chrome(&on(OperatingSystem::Nix, true));
        assert_eq!(wayland, egui::vec2(side, FRAME + side));
        // As `draw` measures it: the content's size and this room are the window's.
        #[derive(Debug)]
        struct Fixed;
        impl Dialog for Fixed {
            fn id(&self) -> &'static str {
                "fixed"
            }
            fn title(&self, _: &egui::Context) -> String {
                "Fixed".to_owned()
            }
            fn form(&self, _: OperatingSystem) -> Form {
                Form::Modeless
            }
            fn look(&self, t: &apertrust_ui::Theme) -> Look {
                Look {
                    background: t.colors.panel,
                    margin: 0.0,
                }
            }
            fn keys(&self, _: OperatingSystem) -> Keys {
                Keys::default()
            }
            fn show(&mut self, ui: &mut egui::Ui) -> Asks {
                ui.allocate_exact_size(egui::vec2(300.0, 200.0), egui::Sense::hover());
                Asks::default()
            }
        }
        for (os, wayland) in [
            (OperatingSystem::Mac, false),
            (OperatingSystem::Windows, false),
            (OperatingSystem::Nix, false),
            (OperatingSystem::Nix, true),
        ] {
            let mut harness = Harness::builder()
                .with_size(egui::vec2(600.0, 400.0))
                .with_os(os)
                .with_context_setup(move |ctx| {
                    apertrust_ui::install(ctx, Theme::default());
                    set_shadow(ctx, platform::shadowed(os, wayland));
                })
                .build_ui_state(
                    |ui, sizes: &mut Vec<egui::Vec2>| sizes.push(draw(ui, &mut Fixed).size),
                    Vec::new(),
                );
            harness.run();
            let size = *harness.state().last().expect("drawn");
            let chrome = chrome(&harness.ctx);
            assert_eq!(size, egui::vec2(300.0, 200.0) + chrome, "{os:?} {wayland}");
        }
    }

    #[test]
    fn a_resizable_window_resizes_from_its_edge_to_just_inside_its_card() {
        use egui::ResizeDirection as Way;
        let on = |os, wayland| hosted(os, wayland);
        // Where no card is inset, X11's among them, the main window's band.
        for (os, wayland) in [
            (OperatingSystem::Mac, false),
            (OperatingSystem::Windows, false),
            (OperatingSystem::Nix, false),
        ] {
            assert_eq!(resize_band(&on(os, wayland)), crate::RESIZE_BAND, "{os:?}");
        }
        let linux = on(OperatingSystem::Nix, true);
        let card = card_inset(&linux);
        assert_eq!(card, MARGIN + Theme::default().shape.hairline);
        let band = resize_band(&linux);
        assert_eq!(band, card + crate::RESIZE_BAND);
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let at = |x, y| crate::resize_direction(window, egui::pos2(x, y), band);
        // The transparent margin, the card's border and just inside it resize.
        assert_eq!(at(1.0, 200.0), Some(Way::West));
        assert_eq!(at(card + 2.0, 200.0), Some(Way::West));
        assert_eq!(at(300.0, 400.0 - card - 2.0), Some(Way::South));
        assert_eq!(at(card + 2.0, card + 2.0), Some(Way::NorthWest));
        // Further in is the content's.
        assert_eq!(at(band + 1.0, 200.0), None);
    }

    #[test]
    fn a_drawn_alert_whose_window_failed_answers_failed_whenever_its_dialog_drops() {
        let request = alert::Request::new("Title", "Message").button("OK", alert::Role::Default);
        let wake = egui::Context::default();
        // Dropped first, as `make_window` drops it, then set by `make`.
        let answer = Rc::new(Cell::new(None));
        drop(alert_dialog::AlertDialog::new(
            &request,
            Rc::clone(&answer),
            &wake,
        ));
        answer.set(Some(Answer::Failed));
        assert_eq!(answer.get(), Some(Answer::Failed));
        // Set first: the drop leaves it.
        let answer = Rc::new(Cell::new(None));
        let dialog = alert_dialog::AlertDialog::new(&request, Rc::clone(&answer), &wake);
        answer.set(Some(Answer::Failed));
        drop(dialog);
        assert_eq!(answer.get(), Some(Answer::Failed));
    }

    #[test]
    fn a_drawn_alert_is_asked_once_and_a_second_ask_meanwhile_is_busy() {
        let answer = Rc::new(Cell::new(None));
        let request = alert::Request::new("Title", "Message").button("OK", alert::Role::Default);
        let main = egui::Context::default();
        let mut asked = None;
        assert_eq!(
            ask_drawn(&mut asked, false, &request, &answer, &main),
            Ask::Shown
        );
        let dialog = &asked.as_ref().expect("asked").dialog;
        assert_eq!(dialog.id(), "alert");
        assert_eq!(dialog.form(OperatingSystem::Nix), Form::Modal);
        // Asked, then open: a second ask asks nothing, and the first is left to answer.
        assert_eq!(
            ask_drawn(&mut asked, false, &request, &answer, &main),
            Ask::Busy
        );
        let mut none = None;
        assert_eq!(
            ask_drawn(&mut none, true, &request, &answer, &main),
            Ask::Busy
        );
        assert!(none.is_none());
        assert_eq!(answer.get(), None);
        // Taken away before it was made, it answers dismissed.
        drop(asked);
        assert_eq!(answer.get(), Some(Answer::Dismissed));
    }

    #[test]
    fn the_window_an_alert_rose_from_is_drawn_but_takes_no_input_and_a_press_raises_it() {
        use egui_winit::winit::dpi::PhysicalPosition;
        use egui_winit::winit::event::{DeviceId, ElementState, MouseButton};
        let device_id = DeviceId::dummy();
        let under = |handled, raises| UnderAlert { handled, raises };
        // Drawn, sized and focused as usual; its taking the focus raises the alert.
        for event in [
            WindowEvent::RedrawRequested,
            WindowEvent::Resized(PhysicalSize::new(640, 480)),
            WindowEvent::Moved(PhysicalPosition::new(10, 10)),
            WindowEvent::Focused(false),
            WindowEvent::CursorLeft { device_id },
        ] {
            assert_eq!(under_alert(&event), under(true, false), "{event:?}");
        }
        assert_eq!(under_alert(&WindowEvent::Focused(true)), under(true, true));
        // A press, or its close, raises the alert and does nothing else.
        let press = |state| WindowEvent::MouseInput {
            device_id,
            state,
            button: MouseButton::Left,
        };
        for event in [press(ElementState::Pressed), WindowEvent::CloseRequested] {
            assert_eq!(under_alert(&event), under(false, true), "{event:?}");
        }
        // The rest of its input goes nowhere.
        for event in [
            press(ElementState::Released),
            WindowEvent::CursorMoved {
                device_id,
                position: PhysicalPosition::new(20.0, 20.0),
            },
            WindowEvent::CursorEntered { device_id },
            WindowEvent::Ime(egui_winit::winit::event::Ime::Commit("é".into())),
            WindowEvent::DroppedFile("photo.raw".into()),
        ] {
            assert_eq!(under_alert(&event), under(false, false), "{event:?}");
        }
    }

    #[test]
    fn a_dialog_s_buttons_sit_at_its_end_in_the_order_given_for_readers_and_tab() {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 80.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, clicked: &mut Vec<usize>| {
                    clicked.extend(buttons(ui, &["Cancel", "OK"]));
                },
                Vec::new(),
            );
        harness.run();
        // In the tree as they are seen: Cancel, then the default button last, as the Mac
        // has them, both at the row's end.
        let names: Vec<String> = harness
            .get_all_by_role(Role::Button)
            .map(|node| node.accesskit_node().label().unwrap_or_default())
            .collect();
        assert_eq!(names, ["Cancel", "OK"]);
        let cancel = harness.get_by_label("Cancel").rect();
        let ok = harness.get_by_label("OK").rect();
        assert!(cancel.right() < ok.left(), "{cancel:?} {ok:?}");
        assert!(ok.right() > 390.0, "at the row's end: {ok:?}");
        // Tab meets them in the same order.
        harness.key_press(egui::Key::Tab);
        harness.run();
        assert!(harness.get_by_label("Cancel").is_focused());
        harness.key_press(egui::Key::Tab);
        harness.run();
        assert!(harness.get_by_label("OK").is_focused());
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state(), &[1]);
    }

    /// Settings as the host draws it in its window, what each frame asked kept.
    struct Bench {
        content: settings_dialog::Settings,
        asked: Vec<Drawn>,
    }

    impl Bench {
        fn outputs(&self) -> Vec<Output> {
            self.asked.iter().filter_map(|d| d.output.clone()).collect()
        }

        fn closed(&self) -> bool {
            self.asked.iter().any(|d| d.close)
        }
    }

    /// Settings in its window on the Mac, as the shell opens it there.
    fn settings_window() -> Harness<'static, Bench> {
        settings_on(OperatingSystem::Mac, &[])
    }

    /// Settings in its window on `os`, with `conf`'s `--conf` for the run, as the shell
    /// opens it there: with a Close button but on the Mac.
    fn settings_on(os: OperatingSystem, conf: &[&str]) -> Harness<'static, Bench> {
        let conf: Vec<String> = conf.iter().map(|c| (*c).to_owned()).collect();
        let overrides = settings::overrides(&conf).expect("read");
        let (keeper, _) = Keeper::open_in(None, &overrides);
        let settings = settings_dialog::Settings::new(&keeper, os != OperatingSystem::Mac);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .with_os(os)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
            })
            .build_ui_state(
                |ui, bench: &mut Bench| {
                    let drawn = draw(ui, &mut bench.content);
                    bench.asked.push(drawn);
                },
                Bench {
                    content: settings,
                    asked: Vec::new(),
                },
            );
        harness.run();
        harness
    }

    #[test]
    fn the_host_draws_a_close_and_a_strip_to_move_by_on_linux_alone() {
        for os in [
            OperatingSystem::Mac,
            OperatingSystem::Windows,
            OperatingSystem::Nix,
        ] {
            let mut harness = settings_on(os, &[]);
            let closes = harness
                .query_all_by_role_and_label(Role::Button, "Close")
                .count();
            // Settings' own Close on Windows and Linux, and on Linux the frame's after it.
            let expected = match os {
                OperatingSystem::Mac => 0,
                OperatingSystem::Windows => 1,
                _ => 2,
            };
            assert_eq!(closes, expected, "{os:?}");
            if os != OperatingSystem::Nix {
                continue;
            }
            // The frame's close closes it as its Cancel.
            harness
                .query_all_by_role_and_label(Role::Button, "Close")
                .last()
                .expect("the frame's close")
                .click();
            harness.run();
            assert!(harness.state().closed());
            // A drag on the strip's empty part asks for a move, a right-click for the menu.
            let mut harness = settings_on(os, &[]);
            let close = harness
                .query_all_by_role_and_label(Role::Button, "Close")
                .last()
                .expect("the frame's close")
                .rect();
            let at = egui::pos2(close.left() - 24.0, close.center().y);
            harness.drag_at(at);
            harness.hover_at(at + egui::vec2(30.0, 0.0));
            harness.run();
            harness.drop_at(at + egui::vec2(30.0, 0.0));
            harness.run();
            assert!(
                harness
                    .state()
                    .asked
                    .iter()
                    .any(|d| d.gesture == Some(Gesture::Move)),
                "moved"
            );
            harness.state_mut().asked.clear();
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            });
            harness.run();
            assert!(
                harness
                    .state()
                    .asked
                    .iter()
                    .any(|d| matches!(d.gesture, Some(Gesture::Menu(_)))),
                "menu"
            );
        }
    }

    #[test]
    fn settings_in_its_modal_window_on_windows_and_linux_changes_at_once_and_closes() {
        for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
            let mut harness = settings_on(os, &[]);
            harness.get_by_role_and_label(Role::Dialog, "Settings");
            harness
                .get_by_role_and_label(Role::ComboBox, "Theme")
                .click();
            harness.run();
            harness.get_by_label("Light").click();
            harness.run();
            assert_eq!(
                theme(&harness.ctx).colors,
                Theme::builtin(Builtin::Light).colors
            );
            harness
                .get_by_role_and_label(Role::ComboBox, "Language")
                .click();
            harness.run();
            harness.get_by_label("Українська").click();
            harness.run();
            assert_eq!(
                harness.state().outputs(),
                [
                    Output::Settings(settings_dialog::Change::Set("theme", "light".into())),
                    Output::Settings(settings_dialog::Change::Set("locale", "uk".into())),
                ],
                "{os:?}"
            );
            // It speaks the new language on the next frame, and its Close closes it: the
            // first so named, since on Linux the frame's close comes after the content.
            assert!(!harness.state().closed());
            harness
                .query_all_by_label(&apertrust_ui::tr(&harness.ctx, "settings-close"))
                .next()
                .expect("Settings' Close")
                .click();
            harness.run();
            assert!(harness.state().closed(), "{os:?}");
            // Escape closes it too, as every modal dialog, once no list of its has it.
            let mut harness = settings_on(os, &[]);
            harness
                .get_by_role_and_label(Role::ComboBox, "Language")
                .click();
            harness.run();
            harness.key_press(egui::Key::Escape);
            harness.run();
            assert!(!harness.state().closed(), "the list closed first: {os:?}");
            harness.key_press(egui::Key::Escape);
            harness.run();
            assert!(harness.state().closed(), "{os:?}");
        }
    }

    #[test]
    fn the_keys_a_dialog_leaves_alone_go_up_for_the_app_and_its_own_never_do() {
        let mut harness = settings_on(OperatingSystem::Windows, &[]);
        let unread = |harness: &Harness<'_, Bench>| -> Vec<Unread> {
            harness
                .state()
                .asked
                .iter()
                .filter_map(|d| d.unread.clone())
                .collect()
        };
        harness.key_press_modifiers(Modifiers::COMMAND, Key::Q);
        harness.run();
        let up = unread(&harness);
        assert_eq!(up.len(), 1, "{up:?}");
        assert!(matches!(
            up[0].events[..],
            [egui::Event::Key {
                key: Key::Q,
                pressed: true,
                ..
            }]
        ));
        assert_eq!(up[0].os, OperatingSystem::Windows);
        // Escape is the dialog's own: it closes it, and goes nowhere else.
        harness.state_mut().asked.clear();
        harness.key_press(Key::Escape);
        harness.run();
        assert!(harness.state().closed());
        assert!(unread(&harness).is_empty());
    }

    /// A row `--conf` sets for this run is disabled, saying why under it, and choosing in
    /// it changes nothing (decision 43).
    #[test]
    fn settings_set_by_conf_are_drawn_disabled_saying_why() {
        let mut harness = settings_on(OperatingSystem::Windows, &["theme=light", "locale=pl"]);
        let why = "Set by --conf for this run";
        assert_eq!(harness.query_all_by_label(why).count(), 2);
        for name in ["Theme", "Language"] {
            let row = harness.get_by_role_and_label(Role::ComboBox, name);
            assert!(row.accesskit_node().is_disabled(), "{name}");
            row.click();
            harness.run();
        }
        assert!(harness.query_by_label("Українська").is_none());
        assert!(harness.state().outputs().is_empty());
    }

    #[test]
    fn settings_in_its_window_is_a_dialog_named_settings_and_measures_itself() {
        let harness = settings_window();
        harness.get_by_role_and_label(Role::Dialog, "Settings");
        harness.get_by_role_and_label(Role::ComboBox, "Theme");
        harness.get_by_role_and_label(Role::ComboBox, "Language");
        let size = harness.state().asked.last().expect("drawn").size;
        // Its width, with no margin, and a height of its own.
        assert_eq!(size.x, settings_dialog::WIDTH);
        assert!(size.y > 600.0 && size.y < 760.0, "{size:?}");
    }

    #[test]
    fn settings_in_its_window_applies_a_change_at_once_and_escape_leaves_it_open() {
        let mut harness = settings_window();
        harness
            .get_by_role_and_label(Role::ComboBox, "Theme")
            .click();
        harness.run();
        harness.get_by_label("Light").click();
        harness.run();
        let asked: Vec<Output> = harness
            .state()
            .asked
            .iter()
            .filter_map(|d| d.output.clone())
            .collect();
        assert_eq!(
            asked,
            [Output::Settings(settings_dialog::Change::Set(
                "theme",
                "light".into()
            ))]
        );
        // Its own context shows it at once.
        assert_eq!(
            theme(&harness.ctx).colors,
            Theme::builtin(Builtin::Light).colors
        );
        // Escape does nothing, as in the Mac's Settings windows, and there is no Close
        // button: the window's own and Cmd+W close it.
        harness.key_press(egui::Key::Escape);
        harness.run();
        assert!(harness.state().asked.iter().all(|d| !d.close));
        assert!(harness.query_by_label("Close").is_none());
    }

    #[test]
    fn the_focus_goes_back_after_a_window_only_where_it_has_not_moved() {
        let (opener, other) = (egui::Id::unique("opener"), egui::Id::unique("other"));
        // A sheet or a modal window kept the main window's input from it: back to the
        // opener.
        assert_eq!(given_back(Form::Sheet, Some(opener), None), Some(opener));
        assert_eq!(
            given_back(Form::Modal, Some(opener), Some(other)),
            Some(opener)
        );
        // Beside Settings' window the user may have chosen another control, or none.
        assert_eq!(
            given_back(Form::Modeless, Some(opener), Some(opener)),
            Some(opener)
        );
        assert_eq!(given_back(Form::Modeless, Some(opener), Some(other)), None);
        assert_eq!(given_back(Form::Modeless, Some(opener), None), None);
        assert_eq!(given_back(Form::Modeless, None, Some(other)), None);
    }

    #[test]
    fn a_window_goes_at_once_when_asked_to_close() {
        for window in [Form::Modeless, Form::Modal] {
            assert_eq!(step(Phase::Shown, window, true, false), Step::Gone);
            assert_eq!(
                step(Phase::Shown, window, false, false),
                Step::Stay(Phase::Shown)
            );
        }
    }

    #[test]
    fn a_sheet_or_a_modal_window_keeps_the_main_window_s_input_and_a_modeless_one_not() {
        assert!(Form::Sheet.modal() && Form::Modal.modal() && !Form::Modeless.modal());
        let shown = |form| Shown {
            form,
            focused: false,
        };
        assert!(shown(Form::Modal).modal() && !shown(Form::Modeless).modal());
    }

    #[test]
    fn a_sheet_slides_in_then_away_and_goes_only_once_appkit_has_returned() {
        let sheet = Form::Sheet;
        let beginning = Phase::Beginning { close: false };
        // Sliding in, AppKit's `beginSheet:` not returned: it stays, then shows.
        assert_eq!(step(beginning, sheet, false, false), Step::Stay(beginning));
        assert_eq!(
            step(beginning, sheet, false, true),
            Step::Stay(Phase::Shown)
        );
        // Shown, a close ends the sheet, and it stays until `endSheet:` has returned.
        assert_eq!(step(Phase::Shown, sheet, true, false), Step::EndSheet);
        assert_eq!(
            step(Phase::Ending, sheet, false, false),
            Step::Stay(Phase::Ending)
        );
        assert_eq!(step(Phase::Ending, sheet, false, true), Step::Gone);
        // A close asked on its way out changes nothing: it is ended once.
        assert_eq!(
            step(Phase::Ending, sheet, true, false),
            Step::Stay(Phase::Ending)
        );
    }

    #[test]
    fn a_close_asked_while_a_sheet_slides_in_waits_for_it_to_arrive() {
        let sheet = Form::Sheet;
        let beginning = Phase::Beginning { close: false };
        let held = Phase::Beginning { close: true };
        // Asked as it slides in: held, never ended before `beginSheet:` returns.
        assert_eq!(step(beginning, sheet, true, false), Step::Stay(held));
        assert_eq!(step(held, sheet, false, false), Step::Stay(held));
        // Then ended as soon as it has arrived, with no frame shown between.
        assert_eq!(step(held, sheet, false, true), Step::EndSheet);
        // Asked in the same callback that sees it arrive: ended at once too.
        assert_eq!(step(beginning, sheet, true, true), Step::EndSheet);
    }
}
