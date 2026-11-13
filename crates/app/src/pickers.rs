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

//! The active colour picker in the app (`docs/pickers.md`, decision 79): what the mirror
//! keeps of it, its marks over the photo and the commands the pointer and the keys send
//! there, the central bar's words, and the live region that reads a pick out.
//!
//! The session measures and applies every pick; the app sends `Command::Picker` and draws
//! what `Event::Picker` says. What the app asked since the last reading shows at once: a
//! picker turned on or off, and the region a drag or a key sent, until a reading includes
//! it, so nothing waits for the GPU.
//!
//! - **On the photo** ([`on_photo`]): the active point or area as the design system's
//!   [`PickerMarks`] draw it, and darktable's pointer (`darkroom.c:3815-4140`): a press
//!   with a point moves it; with an area, a press within 6 points of a corner takes that
//!   corner, the opposite one held, and one elsewhere starts a new area 2 % of the photo's
//!   width each way of it, as tall on screen, the press held; Ctrl-click (Cmd-click on the
//!   Mac) switches point and area, and a right-click resets an area. A drag's steps are
//!   `done: false` and its release `done: true`.
//! - **Keys,** while its node has the focus, which keeps them from the app's actions
//!   (`actions::Focus::keeps`): the arrows move the point or the area a frame pixel, Shift
//!   ten; Alt with them moves an area's right or bottom edge; Space switches point and
//!   area, Delete or Backspace resets, and Esc turns the picker off, the focus going back to
//!   its button. Each is a completed change.
//! - **To screen readers** a group named "Picker area, Exposure", its value the region in
//!   percent of the photo, Increment and Decrement moving it along x and the custom
//!   actions Move up and Move down along y; and a polite live region reading out what a
//!   pick changed, or the colour measured, once the picker has rested half a second
//!   ([`Said`]).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use apertrust_engine::picker::{DEFAULT_AREA, DEFAULT_POINT};
use apertrust_engine::session::{
    ControlValue, PickKeys, PickerCommand, PickerReading, PickerRef, Region, Values,
};
use apertrust_modules::{ControlKind, Definition, Module, PickerKind, Row, messages};
use apertrust_ui::Translator;
use apertrust_ui::components::{PickerAction, PickerMarks, PickerPlace};
use apertrust_ui::i18n::Args;
use apertrust_ui::overlay::Overlay;
use eframe::egui::{
    self, Color32, CursorIcon, Key, Modifiers, Pos2, Rect, Role, Sense, Vec2, WidgetInfo, pos2,
    vec2,
};
use egui::accesskit::{Action, ActionData, CustomAction, Live};

/// What the app knows of the active picker: the session's latest reading, and what the
/// app asked since that no reading shows yet.
#[derive(Debug, Default)]
pub struct Mirror {
    /// The session's latest reading; `None` while no picker is on.
    reading: Option<PickerReading>,
    /// The picker the app turned on, with whether it measures an area, or off (`None`), by
    /// its command's number, until a reading or an end through it says so or it is refused.
    asked: Option<(u64, Option<(PickerRef, bool)>)>,
    /// The region the app sent last, by its command's number, until a reading includes it.
    sent: Option<(u64, Region)>,
    /// Each picker's last point and area this session, as the session keeps them across
    /// photos: where `Switch` goes, shown before its reading arrives.
    places: BTreeMap<PickerRef, Places>,
}

/// A picker's last point and area, each with the command it is through.
#[derive(Debug, Default, Clone, Copy)]
struct Places {
    point: Option<(u64, Region)>,
    area: Option<(u64, Region)>,
}

impl Places {
    /// Keeps `region`, through the first `through` commands, unless its kind's place is
    /// through a later one already: a reading measured before a move never replaces it.
    fn keep(&mut self, through: u64, region: Region) {
        let place = match region {
            Region::Point(_) => &mut self.point,
            Region::Area(_) => &mut self.area,
        };
        if place.is_none_or(|(kept, _)| kept <= through) {
            *place = Some((through, region));
        }
    }
}

impl Mirror {
    /// The session's latest reading.
    pub fn reading(&self) -> Option<&PickerReading> {
        self.reading.as_ref()
    }

    /// The picker on, as the app shows it: the one it asked for, until the session answers,
    /// else the session's.
    pub fn active(&self) -> Option<PickerRef> {
        match self.asked {
            Some((_, asked)) => asked.map(|(picker, _)| picker),
            None => self.reading.as_ref().map(|reading| reading.picker),
        }
    }

    /// The active picker's region as shown: the one the app sent until a reading includes
    /// it, else the session's; `None` until the picker's first reading.
    pub fn region(&self) -> Option<Region> {
        let active = self.active()?;
        if let Some((_, region)) = self.sent {
            return Some(region);
        }
        let reading = self.reading.as_ref()?;
        (reading.picker == active).then_some(reading.region)
    }

    /// Whether the active picker measures an area: its region's kind, or before its first
    /// reading the kind it was turned on as.
    pub fn area(&self) -> Option<bool> {
        match (self.region(), self.asked) {
            (Some(region), _) => Some(matches!(region, Region::Area(_))),
            (None, Some((_, Some((_, area))))) => Some(area),
            _ => None,
        }
    }

    /// The command numbered `number` turned `picker` on, measuring an area where its flag
    /// says, or the picker off (`None`).
    pub fn asked(&mut self, number: u64, picker: Option<(PickerRef, bool)>) {
        self.asked = Some((number, picker));
        self.sent = None;
    }

    /// The command numbered `number` moved the active picker to `region`.
    pub fn sent(&mut self, number: u64, region: Region) {
        self.sent = Some((number, region));
        if let Some(picker) = self.active() {
            self.places.entry(picker).or_default().keep(number, region);
        }
    }

    /// Where `Switch` takes the active picker, at `region`: its other kind's last place
    /// this session, else darktable's default for it.
    pub fn switched(&self, region: Region) -> Region {
        let places = (self.active()).and_then(|picker| self.places.get(&picker).copied());
        let places = places.unwrap_or_default();
        match region {
            Region::Point(_) => places.area.map_or(DEFAULT_AREA, |(_, area)| area),
            Region::Area(_) => places.point.map_or(DEFAULT_POINT, |(_, point)| point),
        }
    }

    /// The session's `Event::Picker`, through the first `through` commands: what the app
    /// asked by one of them is answered, a picker turned on included where the session
    /// turned it off again before its first reading, as a change of its module does.
    pub fn apply(&mut self, through: u64, reading: Option<PickerReading>) {
        let answered = |sent: Option<u64>| sent.is_some_and(|n| n <= through);
        if answered(self.asked.map(|(n, _)| n)) {
            self.asked = None;
        }
        if answered(self.sent.map(|(n, _)| n)) || reading.is_none() && self.asked.is_none() {
            self.sent = None;
        }
        if let Some(reading) = &reading {
            (self.places.entry(reading.picker).or_default()).keep(through, reading.region);
        }
        self.reading = reading;
    }

    /// The command numbered `number` was refused: what it asked stops showing.
    pub fn refused(&mut self, number: u64) {
        if self.asked.is_some_and(|(n, _)| n == number) {
            self.asked = None;
        }
        if self.sent.is_some_and(|(n, _)| n == number) {
            self.sent = None;
        }
    }

    /// The photo is left: its picker with it, as the session turns it off; each picker's
    /// places stay, as the session keeps them.
    pub fn forget(&mut self) {
        *self = Mirror {
            places: std::mem::take(&mut self.places),
            ..Mirror::default()
        };
    }
}

/// The region a picker command moves the active picker to, if it moves it.
pub fn moved_to(command: &PickerCommand) -> Option<Region> {
    match *command {
        PickerCommand::Point { at, .. } => Some(Region::Point(at)),
        PickerCommand::Area { from, to, .. } => Some(Region::Area([
            from[0].min(to[0]),
            from[1].min(to[1]),
            from[0].max(to[0]),
            from[1].max(to[1]),
        ])),
        _ => None,
    }
}

/// What a press on `picker`'s button sends, `keys` held: `On`, with no tool settings and
/// no channel tab until a module reads them, or `Off` for the picker on, since `On` for it
/// would start it again (`docs/pickers.md`, Against darktable).
pub fn pressed(picker: PickerRef, action: PickerAction, keys: PickKeys) -> PickerCommand {
    match action {
        PickerAction::On { area } => PickerCommand::On {
            picker,
            area,
            keys,
            tools: Vec::new(),
            channel: None,
        },
        PickerAction::Off => PickerCommand::Off,
    }
}

/// What the app shows of a picker command at once, until the session answers: a picker
/// turned on, measuring the kind it asks, or off; a region moved; and, for a command that
/// applies a reading, a pick for the live region to wait for.
#[derive(Debug)]
pub struct Showing {
    asked: Option<Option<(PickerRef, bool)>>,
    moved: Option<Region>,
    /// `Switch` or `Reset`, which move the picker where the mirror knows.
    kind: Option<PickerCommand>,
    applies: bool,
}

impl Showing {
    /// What `command` shows, its picker among `modules`.
    pub fn of(command: &PickerCommand, modules: &[&dyn Module]) -> Showing {
        let asked = match *command {
            PickerCommand::On { picker, area, .. } => {
                let kind = declared(modules, picker).map(|(_, declared)| declared.kind);
                let area = kind.map_or(area, |kind| measures_area(kind, area));
                Some(Some((picker, area)))
            }
            PickerCommand::Off => Some(None),
            _ => None,
        };
        let kind = matches!(command, PickerCommand::Switch | PickerCommand::Reset);
        Showing {
            asked,
            moved: moved_to(command),
            kind: kind.then(|| command.clone()),
            applies: applies(command),
        }
    }

    /// Shows it, the command sent as `number`, in `mirror`, with `said` waiting for its
    /// pick against `values`; a picker turned on moves the focus to the photo.
    pub fn sent(
        self,
        number: u64,
        mirror: &mut Mirror,
        said: &mut Said,
        values: &Values,
        ctx: &egui::Context,
    ) {
        if let Some(asked) = self.asked {
            mirror.asked(number, asked);
            if asked.is_some() {
                focus_photo(ctx);
            }
        }
        let before = mirror.region();
        let moved = match (&self.kind, before) {
            (Some(command), Some(region)) => Some(follow(command, region, mirror.switched(region))),
            _ => self.moved,
        };
        if let Some(region) = moved {
            mirror.sent(number, region);
        }
        if self.applies
            && let Some(picker) = mirror.active()
        {
            // A move to where the picker is brings no reading: it rests the picker, its
            // reading still the one the last move awaits.
            if self.moved.is_some() && moved == before {
                said.rested(Instant::now());
            } else {
                said.moved(number, Instant::now(), picker, values);
            }
        }
    }
}

/// Where `command` moves a picker at `region`, `switched` where `Switch` takes it: a move
/// to its place, `Switch` to its other kind's, `Reset` to its kind's default.
pub fn follow(command: &PickerCommand, region: Region, switched: Region) -> Region {
    match command {
        PickerCommand::Switch => switched,
        PickerCommand::Reset => match region {
            Region::Point(_) => DEFAULT_POINT,
            Region::Area(_) => DEFAULT_AREA,
        },
        command => moved_to(command).unwrap_or(region),
    }
}

/// Whether the session applies a module picker's reading after `command`
/// (`docs/pickers.md`, What a pick writes): what the live region waits for.
pub fn applies(command: &PickerCommand) -> bool {
    matches!(
        command,
        PickerCommand::On { .. }
            | PickerCommand::Point { .. }
            | PickerCommand::Area { .. }
            | PickerCommand::Switch
            | PickerCommand::Reset
    )
}

/// Whether `command` completes a change where its pick writes, as the session saves it:
/// `On`, `Switch`, `Reset`, and a move's release (`docs/pickers.md`, What a pick writes).
pub fn completes(command: &PickerCommand) -> bool {
    matches!(
        command,
        PickerCommand::On { .. }
            | PickerCommand::Switch
            | PickerCommand::Reset
            | PickerCommand::Point { done: true, .. }
            | PickerCommand::Area { done: true, .. }
    )
}

/// Whether a picker of `kind` turned on with `area` measures an area: one kind, that
/// kind; both, an area where asked.
pub fn measures_area(kind: PickerKind, area: bool) -> bool {
    match kind {
        PickerKind::Area => true,
        PickerKind::Point => false,
        PickerKind::PointArea => area,
    }
}

/// The keys held as a pick reads them: Ctrl, or Cmd on the Mac, and Shift.
pub fn pick_keys(modifiers: Modifiers) -> PickKeys {
    PickKeys {
        command: modifiers.command || modifiers.ctrl || modifiers.mac_cmd,
        shift: modifiers.shift,
    }
}

/// Whether a press with `modifiers` is darktable's Ctrl-click on the photo, which compares
/// the whole mask: Ctrl alone, or on the Mac (`mac`) Cmd alone, Control-click being the
/// Mac's right-click ([`mac_right_click`]).
fn command_alone(modifiers: Modifiers, mac: bool) -> bool {
    let command = if mac {
        modifiers.mac_cmd && !modifiers.ctrl
    } else {
        modifiers.command || modifiers.ctrl
    };
    command && !modifiers.shift && !modifiers.alt
}

/// Whether a primary press with `modifiers` is the Mac's right-click, Control-click with
/// no Cmd, as everywhere in the app (`docs/gui.md`, Popups).
fn mac_right_click(modifiers: Modifiers, mac: bool) -> bool {
    mac && modifiers.ctrl && !modifiers.mac_cmd
}

/// A module's picker and its definition, among `modules`.
fn declared<'m>(
    modules: &'m [&'m dyn Module],
    picker: PickerRef,
) -> Option<(&'m Definition, &'static apertrust_modules::Picker)> {
    let PickerRef::Module { module, picker } = picker else {
        return None;
    };
    let found = modules.iter().find(|m| m.definition().id == module)?;
    let definition = found.definition();
    Some((definition, definition.picker(picker)?))
}

/// A picker's name (`docs/pickers.md`, On the photo): its control's label where it sits in
/// a slider's quad, else its own words.
pub fn name(words: &Translator, definition: &Definition, picker: &str) -> String {
    let module = definition.id;
    let quad = definition.rows().find_map(|row| match row {
        Row::Slider(slider) if row.pickers() == [picker] => Some(slider),
        _ => None,
    });
    let id = match quad {
        Some(slider) => match slider.label {
            Some(own) => messages::row_words(module, slider.control, own)[0].clone(),
            None => messages::label(module, slider.control),
        },
        None => messages::picker(module, picker)[0].clone(),
    };
    words.text(&id)
}

/// The central bar's words while `picker` is on, measuring an area where `area` says
/// (`docs/pickers.md`, On the photo): which picker it is, the module's title and the
/// picker's name, and how to use it, its modifier as `mac` spells it. `None` for the Colour
/// picker's, whose words come with its module.
pub fn bar_words(
    words: &Translator,
    modules: &[&dyn Module],
    picker: PickerRef,
    area: bool,
    mac: bool,
) -> Option<(String, String)> {
    let (definition, declared) = declared(modules, picker)?;
    let mut args = Args::new();
    args.set("module", words.text(&messages::title(definition.id)));
    args.set("picker", name(words, definition, declared.id));
    let name = words.text_with("picker-bar-name", Some(&args));
    let mut args = Args::new();
    args.set("command", if mac { "Cmd" } else { "Ctrl" });
    let how = if area {
        words.text_with("picker-bar-area", Some(&args))
    } else {
        // A point picker's own sentence; an area picker switched to a point has none.
        let what = if declared.kind == PickerKind::Area {
            words.text("picker-bar-point-what")
        } else {
            words.text(&messages::picker(definition.id, declared.id)[3])
        };
        args.set("what", what);
        words.text_with("picker-bar-point", Some(&args))
    };
    Some((name, how))
}

/// The id of the picker's node on the photo, whatever draws it.
fn photo_id() -> egui::Id {
    egui::Id::unique("picker on the photo")
}

/// Asks the picker's node on the photo to take the keyboard's focus as it next shows: a
/// picker turned on moves the focus to the photo, as darktable gives the centre view the
/// focus (`lib.c:1590`).
pub fn focus_photo(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.insert_temp(photo_id().with("focus"), true);
        d.remove::<(PickerRef, u64)>(photo_id().with("give back"));
    });
}

/// How many egui passes after Esc the picker's button may take the focus back, counted by
/// `Context::cumulative_pass_nr`, not frames shown: a frame may run more than one pass,
/// as when egui asks to discard one. The next pass, or the one after where the first
/// drew the button before the picker turned off; later the focus never jumps to it.
const GIVE_BACK: u64 = 2;

/// Asks `picker`'s button to take the focus back as it is next drawn, within [`GIVE_BACK`]
/// frames: Esc on the photo.
fn give_back(ctx: &egui::Context, picker: PickerRef) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(photo_id().with("give back"), (picker, pass)));
}

/// Whether `picker`'s button takes the focus back now, which it then has.
pub fn takes_back(ctx: &egui::Context, picker: PickerRef) -> bool {
    let id = photo_id().with("give back");
    let Some((asked, at)) = ctx.data(|d| d.get_temp::<(PickerRef, u64)>(id)) else {
        return false;
    };
    let late = ctx.cumulative_pass_nr() > at + GIVE_BACK;
    if late || asked == picker {
        ctx.data_mut(|d| d.remove::<(PickerRef, u64)>(id));
    }
    asked == picker && !late
}

/// The active picker over the photo this frame.
pub struct OnPhoto<'a> {
    pub picker: PickerRef,
    /// The photo's rectangle on the screen, the frame drawn in it pixel for pixel.
    pub photo: Rect,
    /// The frame's size in its pixels.
    pub pixels: [u32; 2],
    /// Where the picker is, as shown ([`Mirror::region`]).
    pub region: Region,
    /// Where `Switch` takes it ([`Mirror::switched`]).
    pub switched: Region,
    /// What its point sampled, as the screen shows it.
    pub colour: Color32,
    /// Its name, as the central bar gives it ([`name`]).
    pub name: &'a str,
}

/// A drag of the picker on the photo, between its press and its release.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// The point follows the pointer; `last` is where it was sent last.
    Point { last: [f32; 2] },
    /// The area runs from `held`, the corner or the press that stays, to the pointer;
    /// `last` is what was sent last, which the release sends again as done.
    Area {
        held: [f32; 2],
        last: Option<([f32; 2], [f32; 2])>,
    },
}

/// How near an area's edges, in points, a press takes their corner, strictly within it.
const CORNER: f32 = 6.0;
/// A new area's half side across, as a share of the photo's width.
const NEW_AREA: f32 = 0.02;

/// Draws the picker's marks over the photo and returns the commands the pointer and the
/// keys sent this frame, in order. Its node takes the focus where asked
/// ([`focus_photo`]); Esc's `Off` gives it back to the picker's button.
pub fn on_photo(ui: &mut egui::Ui, on: &OnPhoto) -> Vec<PickerCommand> {
    let ctx = ui.ctx().clone();
    let id = photo_id();
    let response = ui.interact(on.photo, id, Sense::click_and_drag());
    if ctx.data_mut(|d| d.remove_temp::<bool>(id.with("focus"))) == Some(true) {
        response.request_focus();
    }
    // The arrows and Esc are the picker's, never egui's moves of the focus.
    let keeps = egui::EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        escape: true,
        ..Default::default()
    };
    ctx.memory_mut(|m| m.set_focus_lock_filter(id, keeps));
    let size = on.photo.size();
    let pixel = [
        1.0 / on.pixels[0].max(1) as f32,
        1.0 / on.pixels[1].max(1) as f32,
    ];
    let at = |pos: Pos2| {
        let p = (pos - on.photo.min) / size;
        [p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0)]
    };
    let mut commands = Vec::new();

    // The pointer.
    let drag_id = id.with("drag");
    let mut drag: Option<Drag> = ctx.data(|d| d.get_temp(drag_id));
    let (pressed, released, down, moved, pos, origin, modifiers) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.primary_down(),
            i.pointer.delta() != Vec2::ZERO,
            i.pointer.latest_pos(),
            i.pointer.press_origin(),
            i.modifiers,
        )
    });
    // A drag lasts while the button is held: one whose release this never saw, as the
    // picker turned off under it, is over.
    if !(down || released) {
        drag = None;
    }
    let keys = pick_keys(modifiers);
    // The press where it was, a move after it in the same frame then followed.
    let pressed_at = origin
        .or(pos)
        .filter(|&at| pressed && on.photo.contains(at) && response.contains_pointer());
    if let Some(origin) = pressed_at {
        response.request_focus();
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        drag = pressed_on(&mut commands, on, at(origin), origin, modifiers, mac);
    }
    let moved = moved && pressed_at.is_none_or(|origin| pos != Some(origin));
    if let (Some(now), Some(pos), true) = (drag, pos, moved) {
        let to = at(pos);
        drag = Some(match now {
            Drag::Point { .. } => {
                commands.push(PickerCommand::Point {
                    at: to,
                    done: false,
                    keys,
                });
                Drag::Point { last: to }
            }
            Drag::Area { held, .. } => {
                let to = at_least_a_pixel(held, to, pixel);
                commands.push(PickerCommand::Area {
                    from: held,
                    to,
                    done: false,
                    keys,
                });
                Drag::Area {
                    held,
                    last: Some((held, to)),
                }
            }
        });
    }
    if released && let Some(ended) = drag.take() {
        match ended {
            Drag::Point { last } => commands.push(PickerCommand::Point {
                at: last,
                done: true,
                keys,
            }),
            Drag::Area {
                last: Some((from, to)),
                ..
            } => commands.push(PickerCommand::Area {
                from,
                to,
                done: true,
                keys,
            }),
            // A corner taken and let go where it was moves nothing.
            Drag::Area { last: None, .. } => {}
        }
    }
    if response.secondary_clicked() && matches!(on.region, Region::Area(_)) {
        commands.push(PickerCommand::Reset);
    }
    if matches!(drag, Some(Drag::Area { .. })) {
        ctx.set_cursor_icon(CursorIcon::Move);
    }
    ctx.data_mut(|d| match drag {
        Some(drag) => {
            d.insert_temp(drag_id, drag);
        }
        None => d.remove::<Drag>(drag_id),
    });

    // The keys and screen readers' actions, while the node has the focus.
    let (mut region, mut switched) = (on.region, on.switched);
    for command in &commands {
        (region, switched) = followed(command, region, switched);
    }
    if response.has_focus() {
        let pressed = ui.input_mut(|i| {
            let mut taken = Vec::new();
            i.events.retain(|event| match *event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if picker_key(key, modifiers).is_some() => {
                    taken.extend(picker_key(key, modifiers));
                    false
                }
                _ => true,
            });
            taken
        });
        for key in pressed {
            let Some(command) = key_command(key, region, pixel) else {
                continue;
            };
            if command == PickerCommand::Off {
                give_back(&ctx, on.picker);
            }
            (region, switched) = followed(&command, region, switched);
            commands.push(command);
        }
    }
    let requests = ui.input(|i| {
        let along = |action| i.num_accesskit_action_requests(id, action);
        let custom = |which: i32| {
            i.accesskit_action_requests(id, Action::CustomAction)
                .filter(|r| r.data == Some(ActionData::CustomAction(which)))
                .count()
        };
        [
            (Step::Right, along(Action::Increment)),
            (Step::Left, along(Action::Decrement)),
            (Step::Up, custom(MOVE_UP)),
            (Step::Down, custom(MOVE_DOWN)),
        ]
    });
    for (step, count) in requests {
        for _ in 0..count {
            let command = key_command(PickerKey::Move(step, 1.0), region, pixel);
            if let Some(command) = command {
                (region, switched) = followed(&command, region, switched);
                commands.push(command);
            }
        }
    }

    // The marks, at what was sent this frame.
    let place = match region {
        Region::Point(at) => PickerPlace::Point {
            at: pos2(at[0], at[1]),
            colour: on.colour,
        },
        Region::Area(area) => PickerPlace::Area(Rect::from_min_max(
            pos2(area[0], area[1]),
            pos2(area[2], area[3]),
        )),
    };
    let zoom = on.photo.width() / on.pixels[0].max(1) as f32;
    let painter = ui.painter_at(on.photo);
    PickerMarks::new(place)
        .focused(response.has_focus())
        .paint(&Overlay::new(&painter, on.photo, zoom));

    // To screen readers.
    let words = apertrust_ui::translator(&ctx);
    let mut args = Args::new();
    args.set("picker", on.name);
    let (kind, value) = match region {
        Region::Point([x, y]) => {
            args.set("x", percent(&words, x));
            args.set("y", percent(&words, y));
            ("picker-photo-point", "picker-photo-at-point")
        }
        Region::Area([left, top, right, bottom]) => {
            args.set("left", percent(&words, left));
            args.set("top", percent(&words, top));
            args.set("right", percent(&words, right));
            args.set("bottom", percent(&words, bottom));
            ("picker-photo-area", "picker-photo-at-area")
        }
    };
    let label = words.text_with(kind, Some(&args));
    let value = words.text_with(value, Some(&args));
    let [up, down] = ["picker-photo-up", "picker-photo-down"].map(|id| words.text(id));
    response.widget_info(|| WidgetInfo::labeled(Role::Group, true, &label));
    ctx.accesskit_node_builder(id, |node| {
        node.set_value(value);
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);
        node.set_custom_actions([
            CustomAction {
                id: MOVE_UP,
                description: up,
            },
            CustomAction {
                id: MOVE_DOWN,
                description: down,
            },
        ]);
    });
    commands
}

/// What a press at `at`, in the photo's coordinates, `pos` on the screen, with `modifiers`
/// held, on the Mac where `mac`, sends into `commands`, and the drag it starts: Ctrl alone
/// (Cmd on the Mac) switches point and area, and the Mac's Control-click is a right-click,
/// resetting an area; with a point, the point moves there; with an area, a press within
/// [`CORNER`] points of its nearest edge on each axis takes that corner, the opposite one
/// held, as darktable's (`darkroom.c:4050-4062`), and one elsewhere starts a new area round
/// it.
fn pressed_on(
    commands: &mut Vec<PickerCommand>,
    on: &OnPhoto,
    at: [f32; 2],
    pos: Pos2,
    modifiers: Modifiers,
    mac: bool,
) -> Option<Drag> {
    let keys = pick_keys(modifiers);
    if mac_right_click(modifiers, mac) {
        if matches!(on.region, Region::Area(_)) {
            commands.push(PickerCommand::Reset);
        }
        return None;
    }
    if command_alone(modifiers, mac) {
        commands.push(PickerCommand::Switch);
        return None;
    }
    let area = match on.region {
        Region::Point(_) => {
            commands.push(PickerCommand::Point {
                at,
                done: false,
                keys,
            });
            return Some(Drag::Point { last: at });
        }
        Region::Area(area) => area,
    };
    let rect = to_screen(on.photo, area);
    // The nearest edge on each axis, darktable's `MIN(dx0, dx1) < hx`; the corner held is
    // the opposite one.
    let (left, right) = ((pos.x - rect.left()).abs(), (pos.x - rect.right()).abs());
    let (top, bottom) = ((pos.y - rect.top()).abs(), (pos.y - rect.bottom()).abs());
    if left.min(right) < CORNER && top.min(bottom) < CORNER {
        let held = [
            if left < right { area[2] } else { area[0] },
            if top < bottom { area[3] } else { area[1] },
        ];
        return Some(Drag::Area { held, last: None });
    }
    // As tall on screen as it is wide.
    let size = on.photo.size();
    let (across, down) = (NEW_AREA, NEW_AREA * size.x / size.y.max(1.0));
    let from = [(at[0] - across).max(0.0), (at[1] - down).max(0.0)];
    let to = [(at[0] + across).min(1.0), (at[1] + down).min(1.0)];
    commands.push(PickerCommand::Area {
        from,
        to,
        done: false,
        keys,
    });
    Some(Drag::Area {
        held: at,
        last: Some((from, to)),
    })
}

/// The picker's place after `command`, at `region` with `switched` its other kind's, and
/// its other kind's then: a switch trades them.
fn followed(command: &PickerCommand, region: Region, switched: Region) -> (Region, Region) {
    let next = follow(command, region, switched);
    if *command == PickerCommand::Switch {
        (next, region)
    } else {
        (next, switched)
    }
}

/// The custom actions' ids: Move up and Move down.
const MOVE_UP: i32 = 0;
const MOVE_DOWN: i32 = 1;

/// A direction a key moves the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Left,
    Right,
    Up,
    Down,
}

/// What a key does to the picker on the photo.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PickerKey {
    /// The point or the whole area, by this many frame pixels.
    Move(Step, f32),
    /// An area's right or bottom edge, by this many frame pixels.
    Resize(Step, f32),
    Switch,
    Reset,
    Off,
}

/// The picker's key `key` with `modifiers` is, if any: the arrows alone or with Shift,
/// with Alt and maybe Shift, Space, Delete, Backspace and Esc alone.
fn picker_key(key: Key, modifiers: Modifiers) -> Option<PickerKey> {
    let command = modifiers.command || modifiers.ctrl || modifiers.mac_cmd;
    if command {
        return None;
    }
    let step = match key {
        Key::ArrowLeft => Some(Step::Left),
        Key::ArrowRight => Some(Step::Right),
        Key::ArrowUp => Some(Step::Up),
        Key::ArrowDown => Some(Step::Down),
        _ => None,
    };
    let by = if modifiers.shift { 10.0 } else { 1.0 };
    if let Some(step) = step {
        return Some(if modifiers.alt {
            PickerKey::Resize(step, by)
        } else {
            PickerKey::Move(step, by)
        });
    }
    if modifiers.shift || modifiers.alt {
        return None;
    }
    match key {
        Key::Space => Some(PickerKey::Switch),
        Key::Delete | Key::Backspace => Some(PickerKey::Reset),
        Key::Escape => Some(PickerKey::Off),
        _ => None,
    }
}

/// The command `key` sends with the picker at `region`, a frame pixel `pixel` of the photo
/// on each axis: a move or a resize held inside the photo, an area at least a pixel each
/// way. A resize of a point sends nothing.
fn key_command(key: PickerKey, region: Region, pixel: [f32; 2]) -> Option<PickerCommand> {
    let keys = PickKeys::default();
    let delta = |step: Step, by: f32| match step {
        Step::Left => [-by * pixel[0], 0.0],
        Step::Right => [by * pixel[0], 0.0],
        Step::Up => [0.0, -by * pixel[1]],
        Step::Down => [0.0, by * pixel[1]],
    };
    Some(match (key, region) {
        (PickerKey::Move(step, by), Region::Point(at)) => {
            let [dx, dy] = delta(step, by);
            PickerCommand::Point {
                at: [(at[0] + dx).clamp(0.0, 1.0), (at[1] + dy).clamp(0.0, 1.0)],
                done: true,
                keys,
            }
        }
        (PickerKey::Move(step, by), Region::Area([left, top, right, bottom])) => {
            let [dx, dy] = delta(step, by);
            // The whole area, held inside the photo at its size.
            let dx = dx.clamp(-left, 1.0 - right);
            let dy = dy.clamp(-top, 1.0 - bottom);
            PickerCommand::Area {
                from: [left + dx, top + dy],
                to: [right + dx, bottom + dy],
                done: true,
                keys,
            }
        }
        (PickerKey::Resize(step, by), Region::Area([left, top, right, bottom])) => {
            let [dx, dy] = delta(step, by);
            let right = (right + dx).clamp((left + pixel[0]).min(1.0), 1.0);
            let bottom = (bottom + dy).clamp((top + pixel[1]).min(1.0), 1.0);
            PickerCommand::Area {
                from: [left, top],
                to: [right, bottom],
                done: true,
                keys,
            }
        }
        (PickerKey::Resize(..), Region::Point(_)) => return None,
        (PickerKey::Switch, _) => PickerCommand::Switch,
        (PickerKey::Reset, _) => PickerCommand::Reset,
        (PickerKey::Off, _) => PickerCommand::Off,
    })
}

/// `to`, moved where needed to be at least a frame pixel from `from` on each axis, inside
/// the photo.
fn at_least_a_pixel(from: [f32; 2], to: [f32; 2], pixel: [f32; 2]) -> [f32; 2] {
    let apart = |a: f32, b: f32, least: f32| {
        if (b - a).abs() >= least {
            b
        } else if b >= a && a + least <= 1.0 || a - least < 0.0 {
            a + least
        } else {
            a - least
        }
    };
    [
        apart(from[0], to[0], pixel[0]),
        apart(from[1], to[1], pixel[1]),
    ]
}

/// An area in the photo's coordinates, left, top, right and bottom, on the screen.
fn to_screen(photo: Rect, [left, top, right, bottom]: [f32; 4]) -> Rect {
    let at = |x: f32, y: f32| photo.min + vec2(x * photo.width(), y * photo.height());
    Rect::from_min_max(at(left, top), at(right, bottom))
}

/// A share of the photo, 0 to 1, as a whole percentage in the language's numbers.
fn percent(words: &Translator, share: f32) -> String {
    words.format_number(f64::from(share) * 100.0, 0, false)
}

/// The colour a reading's frame part measured, its mean as the screen shows it; clear
/// where it measured none.
pub fn sampled(reading: Option<&PickerReading>) -> Color32 {
    let Some([r, g, b]) = reading.and_then(|r| r.frame).map(|frame| frame.rgb[0]) else {
        return Color32::TRANSPARENT;
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(byte(r), byte(g), byte(b))
}

/// How long the pointer or the keys rest before a pick is read out.
pub const RESTS: Duration = Duration::from_millis(500);

/// What the live region reads out of a pick (`docs/pickers.md`, Accessibility): once a
/// reading includes the last command that moved or pressed the picker and the picker has
/// rested [`RESTS`], each control the pick changed with its value, from the values that
/// follow, or where it changed none the colour the frame measured.
#[derive(Debug, Default)]
pub struct Said {
    /// The picker its pick and its words are about.
    picker: Option<PickerRef>,
    rest: Option<Rest>,
    text: String,
}

/// A pick waiting to be read out.
#[derive(Debug)]
struct Rest {
    /// When the picker last moved, and by which command.
    at: Instant,
    number: u64,
    /// The picker's module's values when its gesture began.
    before: Vec<(&'static str, ControlValue)>,
}

impl Said {
    /// What the live region says now.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The picker on is `active`, as the mirror shows it: another, or none, as it turns off
    /// or the photo is left, forgets the pick waiting and what was said.
    pub fn follow(&mut self, active: Option<PickerRef>) {
        if active != self.picker {
            *self = Said {
                picker: active,
                ..Said::default()
            };
        }
    }

    /// The command numbered `number`, which applies a reading, was sent at `now` for
    /// `picker`, the values shown being `values`: the moves of one gesture are read out
    /// once, against the values before its first.
    pub fn moved(&mut self, number: u64, now: Instant, picker: PickerRef, values: &Values) {
        self.follow(Some(picker));
        let before = match self.rest.take() {
            Some(rest) => rest.before,
            None => module_values(picker, values),
        };
        self.rest = Some(Rest {
            at: now,
            number,
            before,
        });
    }

    /// The picker rested again at `now` with nothing new to measure: the pick waiting, if
    /// any, waits from then, for the reading it already awaits.
    pub fn rested(&mut self, now: Instant) {
        if let Some(rest) = &mut self.rest {
            rest.at = now;
        }
    }

    /// Reads the pick out where it is due at `now`, from `reading` and the values shown;
    /// returns how long until it is due, for a frame then.
    pub fn update(
        &mut self,
        now: Instant,
        reading: Option<&PickerReading>,
        values: &Values,
        modules: &[&dyn Module],
        words: &Translator,
    ) -> Option<Duration> {
        let rest = self.rest.as_ref()?;
        let due = rest.at + RESTS;
        if now < due {
            return Some(due - now);
        }
        let reading = reading.filter(|r| r.through >= rest.number)?;
        let rest = self.rest.take()?;
        self.text = said(reading, &rest.before, values, modules, words);
        None
    }
}

/// The controls of `picker`'s module, as `values` show them.
fn module_values(picker: PickerRef, values: &Values) -> Vec<(&'static str, ControlValue)> {
    let PickerRef::Module { module, .. } = picker else {
        return Vec::new();
    };
    values
        .modules
        .iter()
        .find(|state| state.module == module)
        .map(|state| {
            state
                .controls
                .iter()
                .map(|c| (c.control, c.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// What `reading` read out says: each control its pick wrote that differs from `before`,
/// with its value, or the frame's mean colour.
fn said(
    reading: &PickerReading,
    before: &[(&'static str, ControlValue)],
    values: &Values,
    modules: &[&dyn Module],
    words: &Translator,
) -> String {
    let now = module_values(reading.picker, values);
    let changed: Vec<String> = declared(modules, reading.picker)
        .map(|(definition, declared)| {
            declared
                .writes
                .iter()
                .filter_map(|&control| {
                    let value = now.iter().find(|(c, _)| *c == control)?.1.clone();
                    let was = before.iter().find(|(c, _)| *c == control).map(|(_, v)| v);
                    if was == Some(&value) {
                        return None;
                    }
                    let mut args = Args::new();
                    args.set(
                        "control",
                        words.text(&messages::label(definition.id, control)),
                    );
                    args.set("value", value_said(words, definition, control, &value)?);
                    Some(words.text_with("picker-said-control", Some(&args)))
                })
                .collect()
        })
        .unwrap_or_default();
    if !changed.is_empty() {
        return changed.join(", ");
    }
    let Some(frame) = reading.frame else {
        return String::new();
    };
    let mut args = Args::new();
    for (name, v) in ["red", "green", "blue"].into_iter().zip(frame.rgb[0]) {
        let byte = (f64::from(v).clamp(0.0, 1.0) * 255.0).round();
        args.set(name, words.format_number(byte, 0, false));
    }
    words.text_with("picker-said-rgb", Some(&args))
}

/// A control's value as its row shows it: a number with its row's decimals, signed where
/// its range spans zero, and its unit; an option by its words.
fn value_said(
    words: &Translator,
    definition: &Definition,
    control: &str,
    value: &ControlValue,
) -> Option<String> {
    let module = definition.id;
    let found = definition.control(control)?;
    match (found.kind, value) {
        (ControlKind::Number { min, max, .. }, ControlValue::Number(value)) => {
            let decimals = definition
                .rows()
                .find_map(|row| match row {
                    Row::Slider(slider) if slider.control == control => Some(slider.decimals),
                    _ => None,
                })
                .unwrap_or(2);
            let signed = min < 0.0 && max > 0.0;
            let number = words.format_number(f64::from(*value), usize::from(decimals), signed);
            Some(match found.unit {
                Some(unit) => {
                    let mut args = Args::new();
                    args.set("value", number);
                    words.text_with(&messages::unit_value(unit), Some(&args))
                }
                None => number,
            })
        }
        (ControlKind::Choice { options, .. }, ControlValue::Choice(index)) => {
            let option = options.get(*index as usize)?;
            Some(words.text(&messages::option(module, control, option)))
        }
        _ => None,
    }
}

/// The polite live region at `at`, saying `text`: a status that takes no room.
pub fn live_region(ui: &mut egui::Ui, at: Pos2, text: &str) {
    let response = ui.interact(
        Rect::from_min_size(at, Vec2::ZERO),
        photo_id().with("said"),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Status, true, text));
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_live(Live::Polite));
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use apertrust_engine::session::{
        ControlState, FrameReading, ModuleState, PickKeys, PickerCommand, PickerReading, PickerRef,
        Region, Values,
    };
    use apertrust_modules::testing::Sample;
    use apertrust_modules::{ControlValue, Module};
    use apertrust_ui::{Builtin, Locale, Theme};
    use egui::accesskit::{Action, ActionData, ActionRequest, Role, Toggled, TreeId};
    use egui::os::OperatingSystem;
    use egui::{Color32, Key, Modifiers, PointerButton, Rect, UiBuilder, pos2, vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use super::*;
    use crate::modules::{self, Change, Panels};

    static MODULES: &[&dyn Module] = &[&Sample];

    /// `testing::Sample`'s pickers: brightness's quad, an area; and the row's spread and
    /// show, each a point or an area.
    const BRIGHTNESS: PickerRef = PickerRef::Module {
        module: "sample",
        picker: "brightness",
    };
    const SPREAD: PickerRef = PickerRef::Module {
        module: "sample",
        picker: "spread",
    };
    const SHOW: PickerRef = PickerRef::Module {
        module: "sample",
        picker: "show",
    };

    /// The pickers' buttons, by their full names: Sample has no messages, so its words are
    /// its message ids.
    const QUAD: &str = "module-sample-picker-brightness-full";
    const SPREAD_BUTTON: &str = "module-sample-picker-spread-full";
    const SHOW_BUTTON: &str = "module-sample-picker-show-full";

    /// Where the panel is, and the photo, its frame a pixel a point.
    const PANEL: Rect = Rect {
        min: pos2(0.0, 0.0),
        max: pos2(320.0, 400.0),
    };
    const PHOTO: Rect = Rect {
        min: pos2(340.0, 40.0),
        max: pos2(740.0, 340.0),
    };
    const PIXELS: [u32; 2] = [400, 300];

    const DEFAULT_AREA: Region = Region::Area([0.02, 0.02, 0.98, 0.98]);

    /// Sample's section and the photo, wired as the app wires them: a press on a button
    /// sends its command, the photo shows the picker on and sends what the pointer and the
    /// keys do, every command numbered as the session numbers it. The session's answers
    /// are the test's to give ([`answer`]).
    struct Scene {
        mirror: Mirror,
        said: Said,
        values: Values,
        number: u64,
        sent: Vec<PickerCommand>,
    }

    impl Scene {
        fn send(&mut self, command: PickerCommand, ctx: &egui::Context) {
            let showing = Showing::of(&command, MODULES);
            self.number += 1;
            showing.sent(
                self.number,
                &mut self.mirror,
                &mut self.said,
                &self.values,
                ctx,
            );
            self.sent.push(command);
        }
    }

    /// Sample's values at its definition's defaults, every control available.
    fn values() -> Values {
        let definition = Sample.definition();
        let controls = definition
            .controls
            .iter()
            .map(|control| ControlState {
                control: control.id,
                value: control.default(),
                default: control.default(),
                mark: None,
                available: true,
                options: Vec::new(),
                range: None,
            })
            .collect();
        Values {
            modules: vec![ModuleState {
                module: definition.id,
                enabled: true,
                edited: false,
                controls,
                presets: Vec::new(),
                views: Vec::new(),
            }],
            settings: Vec::new(),
            edited: false,
            through: 0,
            extent: [6000, 4000],
            live: true,
        }
    }

    /// The scene on `os`.
    fn scene_on(os: OperatingSystem) -> Harness<'static, Scene> {
        let mut harness = Harness::builder()
            .with_size(vec2(760.0, 400.0))
            .with_os(os)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
            .build_ui_state(
                |ui, scene: &mut Scene| {
                    let ctx = ui.ctx().clone();
                    let t = apertrust_ui::theme(&ctx);
                    let colours = &t.colors;
                    (ui.painter()).rect_filled(ctx.content_rect(), 0.0, colours.photo_surround);
                    ui.painter().rect_filled(PANEL, 0.0, colours.panel);
                    let pickers = modules::Pickers {
                        drawn: &["sample"],
                        on: scene.mirror.active(),
                        enabled: true,
                        why: None,
                    };
                    let state = &scene.values.modules[0];
                    let changes = ui
                        .scope_builder(UiBuilder::new().max_rect(PANEL), |ui| {
                            modules::section(
                                ui,
                                &Sample,
                                state,
                                true,
                                &Panels::default(),
                                &|_: &str| false,
                                false,
                                true,
                                None,
                                &pickers,
                            )
                        })
                        .inner;
                    for change in changes {
                        if let Change::Picker {
                            module,
                            picker,
                            action,
                            keys,
                        } = change
                        {
                            let picker = PickerRef::Module { module, picker };
                            scene.send(pressed(picker, action, keys), &ctx);
                        }
                    }
                    // A sky over a pale wall over earth, as the wireframe's photo.
                    let bands = [(141, 162, 181), (217, 210, 196), (110, 91, 69)];
                    for (i, (r, g, b)) in bands.into_iter().enumerate() {
                        let third = PHOTO.height() / 3.0;
                        let band = Rect::from_min_size(
                            PHOTO.min + vec2(0.0, i as f32 * third),
                            vec2(PHOTO.width(), third),
                        );
                        (ui.painter()).rect_filled(band, 0.0, Color32::from_rgb(r, g, b));
                    }
                    let shown = (scene.mirror.active()).zip(scene.mirror.region());
                    let Some((picker, region)) = shown else {
                        return;
                    };
                    let PickerRef::Module { picker: id, .. } = picker else {
                        return;
                    };
                    let words = apertrust_ui::translator(&ctx);
                    let name = name(&words, Sample.definition(), id);
                    let on = OnPhoto {
                        picker,
                        photo: PHOTO,
                        pixels: PIXELS,
                        region,
                        switched: scene.mirror.switched(region),
                        colour: sampled(scene.mirror.reading()),
                        name: &name,
                    };
                    for command in on_photo(ui, &on) {
                        scene.send(command, &ctx);
                    }
                    live_region(ui, PHOTO.left_bottom(), scene.said.text());
                },
                Scene {
                    mirror: Mirror::default(),
                    said: Said::default(),
                    values: values(),
                    number: 0,
                    sent: Vec::new(),
                },
            );
        harness.run();
        harness
    }

    fn scene() -> Harness<'static, Scene> {
        scene_on(OperatingSystem::Nix)
    }

    /// The session's reading of `picker` at `region`, through `through`, its frame's mean
    /// a warm grey.
    fn reading(picker: PickerRef, region: Region, through: u64) -> PickerReading {
        PickerReading {
            through,
            picker,
            region,
            module: None,
            frame: Some(FrameReading {
                rgb: [[0.6, 0.5, 0.4]; 3],
                lab: [[0.0; 3]; 3],
            }),
        }
    }

    /// The session answers every command sent: the active picker at `region`.
    fn answer(harness: &mut Harness<'_, Scene>, region: Region) {
        let scene = harness.state_mut();
        let picker = scene.mirror.active().expect("a picker on");
        let through = scene.number;
        scene
            .mirror
            .apply(through, Some(reading(picker, region, through)));
        harness.run();
    }

    fn sent(harness: &mut Harness<'_, Scene>) -> Vec<PickerCommand> {
        std::mem::take(&mut harness.state_mut().sent)
    }

    fn toggled(harness: &Harness<'_, Scene>, name: &str) -> Option<Toggled> {
        harness
            .get_by_role_and_label(Role::Button, name)
            .accesskit_node()
            .toggled()
    }

    fn on(picker: PickerRef, area: bool, keys: PickKeys) -> PickerCommand {
        PickerCommand::On {
            picker,
            area,
            keys,
            tools: Vec::new(),
            channel: None,
        }
    }

    /// A point on the photo, in the photo's coordinates, on the screen.
    fn at(x: f32, y: f32) -> egui::Pos2 {
        PHOTO.min + vec2(x * PHOTO.width(), y * PHOTO.height())
    }

    /// The picker on the photo, as screen readers find it: a group with a value.
    fn on_the_photo<'h, S>(harness: &'h Harness<'_, S>) -> egui_kittest::Node<'h> {
        harness.get_by(|node| node.role() == Role::Group && node.value().is_some())
    }

    fn shown_on_the_photo<S>(harness: &Harness<'_, S>) -> bool {
        (harness.query_by(|node| node.role() == Role::Group && node.value().is_some())).is_some()
    }

    /// The quad and the row's buttons show their picker on, at once as pressed, and a
    /// press on the one on turns it off; a right-click on a row's button and Ctrl-click
    /// on either turn it on as an area, the keys held riding with it. Turning one on moves
    /// the focus to the photo, where its marks show once the session's reading arrives.
    #[test]
    fn the_panel_s_buttons_show_the_picker_on_and_turn_it_on_and_off() {
        let mut harness = scene();
        for button in [QUAD, SPREAD_BUTTON, SHOW_BUTTON] {
            assert_eq!(toggled(&harness, button), Some(Toggled::False), "{button}");
        }
        // The row's label, at its start.
        harness.get_by_label("module-sample-group-measure");

        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        assert_eq!(
            sent(&mut harness),
            [on(BRIGHTNESS, false, PickKeys::default())]
        );
        assert_eq!(toggled(&harness, QUAD), Some(Toggled::True));
        assert!(!shown_on_the_photo(&harness));
        answer(&mut harness, DEFAULT_AREA);
        let photo = on_the_photo(&harness);
        assert!(photo.is_focused());
        assert_eq!(
            photo.accesskit_node().label().as_deref(),
            Some("Picker area, module-sample-control-brightness-label")
        );
        assert_eq!(
            photo.value().as_deref(),
            Some("area from 2%, 2% to 98%, 98%")
        );

        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        assert_eq!(sent(&mut harness), [PickerCommand::Off]);
        assert_eq!(toggled(&harness, QUAD), Some(Toggled::False));
        assert!(!shown_on_the_photo(&harness));

        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click_secondary();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, SHOW_BUTTON)
            .click_modifiers(Modifiers::COMMAND);
        harness.run();
        let command = PickKeys {
            command: true,
            shift: false,
        };
        assert_eq!(
            sent(&mut harness),
            [
                on(SPREAD, true, PickKeys::default()),
                on(SHOW, true, command)
            ]
        );
        assert_eq!(toggled(&harness, SHOW_BUTTON), Some(Toggled::True));
        assert_eq!(toggled(&harness, SPREAD_BUTTON), Some(Toggled::False));
    }

    /// Presses at `from` and releases at `to` on the photo, moving between them.
    fn drag(harness: &mut Harness<'_, Scene>, from: egui::Pos2, to: egui::Pos2) {
        harness.hover_at(from);
        harness.run();
        harness.drag_at(from);
        harness.run();
        if to != from {
            harness.hover_at(to);
            harness.run();
        }
        harness.event(egui::Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
    }

    /// A point follows a drag at once, each step `done: false` and the release `done:
    /// true`, before any reading includes it.
    #[test]
    fn a_dragged_point_sends_its_steps_then_its_release() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        drag(&mut harness, at(0.25, 0.5), at(0.3, 0.75));
        let point = |at, done| PickerCommand::Point {
            at,
            done,
            keys: PickKeys::default(),
        };
        assert_eq!(
            sent(&mut harness),
            [
                point([0.25, 0.5], false),
                point([0.3, 0.75], false),
                point([0.3, 0.75], true)
            ]
        );
        assert_eq!(
            harness.state().mirror.region(),
            Some(Region::Point([0.3, 0.75]))
        );
        assert_eq!(
            on_the_photo(&harness).value().as_deref(),
            Some("point at 30%, 75%")
        );
    }

    /// A press within 6 points of an area's corner takes it, the opposite corner held; one
    /// elsewhere draws a new area round it, 2 % of the photo's width each way and as tall
    /// on screen; a corner taken and let go where it was sends nothing.
    #[test]
    fn an_area_is_dragged_by_its_corner_or_drawn_anew() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let area = |from, to, done| PickerCommand::Area {
            from,
            to,
            done,
            keys: PickKeys::default(),
        };

        let corner = at(0.98, 0.98) + vec2(4.0, -3.0);
        drag(&mut harness, corner, at(0.5, 0.5));
        assert_eq!(
            sent(&mut harness),
            [
                area([0.02, 0.02], [0.5, 0.5], false),
                area([0.02, 0.02], [0.5, 0.5], true)
            ]
        );

        let down = 0.02 * PHOTO.width() / PHOTO.height();
        let new = ([0.75 - 0.02, 0.75 - down], [0.75 + 0.02, 0.75 + down]);
        drag(&mut harness, at(0.75, 0.75), at(0.75, 0.75));
        assert_eq!(
            sent(&mut harness),
            [area(new.0, new.1, false), area(new.0, new.1, true)]
        );
        // The new area's top left: taken and let go where it was.
        let shown = harness.state().mirror.region();
        drag(&mut harness, at(new.0[0], new.0[1]), at(new.0[0], new.0[1]));
        assert_eq!(sent(&mut harness), []);
        assert_eq!(harness.state().mirror.region(), shown);

        // Exactly 6 points from a corner is outside it, as darktable's `<`: a new area.
        answer(&mut harness, DEFAULT_AREA);
        let outside = at(0.02, 0.02) + vec2(-6.0, 2.0);
        drag(&mut harness, outside, outside);
        assert!(matches!(
            sent(&mut harness)[..],
            [PickerCommand::Area { from, .. }, _] if from != [0.98, 0.98]
        ));
    }

    /// Clicks `button` in the photo's middle with `modifiers` held.
    fn click(harness: &mut Harness<'_, Scene>, button: PointerButton, modifiers: Modifiers) {
        harness.hover_at(at(0.5, 0.5));
        harness.run();
        for pressed in [true, false] {
            let event = egui::Event::PointerButton {
                pos: at(0.5, 0.5),
                button,
                pressed,
                modifiers,
            };
            harness.event_modifiers(event, modifiers);
        }
        harness.run();
    }

    /// Ctrl-click on the photo switches point and area, and a right-click resets an area;
    /// on a point a right-click does nothing, as darktable's.
    #[test]
    fn ctrl_click_switches_and_a_right_click_resets_an_area() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        click(&mut harness, PointerButton::Secondary, Modifiers::NONE);
        click(&mut harness, PointerButton::Primary, Modifiers::COMMAND);
        assert_eq!(
            sent(&mut harness),
            [PickerCommand::Reset, PickerCommand::Switch]
        );
        answer(&mut harness, Region::Point([0.5, 0.5]));
        click(&mut harness, PointerButton::Secondary, Modifiers::NONE);
        assert_eq!(sent(&mut harness), []);

        // On the Mac, Control-click is the right-click, and Cmd-click switches.
        let mut harness = scene_on(OperatingSystem::Mac);
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        click(&mut harness, PointerButton::Primary, Modifiers::CTRL);
        click(&mut harness, PointerButton::Primary, Modifiers::MAC_CMD);
        assert_eq!(
            sent(&mut harness),
            [PickerCommand::Reset, PickerCommand::Switch]
        );
        answer(&mut harness, Region::Point([0.5, 0.5]));
        click(&mut harness, PointerButton::Primary, Modifiers::CTRL);
        assert_eq!(sent(&mut harness), []);
    }

    /// With the picker on the photo focused: the arrows move it a frame pixel, Shift ten,
    /// Alt with them an area's right or bottom edge, Space switches, Delete and Backspace
    /// reset, and Esc ends it, the focus back on its button. Each is a completed change.
    #[test]
    fn the_keys_move_resize_switch_reset_and_end_it() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, Region::Area([0.25, 0.25, 0.5, 0.5]));
        sent(&mut harness);
        assert!(on_the_photo(&harness).is_focused());
        let (x, y) = (1.0 / 400.0_f32, 1.0 / 300.0_f32);
        let area = |from, to| PickerCommand::Area {
            from,
            to,
            done: true,
            keys: PickKeys::default(),
        };
        harness.key_press(Key::ArrowRight);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
        harness.run();
        harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
        harness.run();
        harness.key_press_modifiers(Modifiers::ALT | Modifiers::SHIFT, Key::ArrowUp);
        harness.run();
        let (left, right) = (0.25 + x, 0.5 + x);
        let (top, bottom) = (0.25 + 10.0 * y, 0.5 + 10.0 * y);
        assert_eq!(
            sent(&mut harness),
            [
                area([left, 0.25], [right, 0.5]),
                area([left, top], [right, bottom]),
                area([left, top], [right + x, bottom]),
                area([left, top], [right + x, bottom - 10.0 * y]),
            ]
        );
        for key in [Key::Space, Key::Delete, Key::Backspace] {
            harness.key_press(key);
            harness.run();
        }
        assert_eq!(
            sent(&mut harness),
            [
                PickerCommand::Switch,
                PickerCommand::Reset,
                PickerCommand::Reset
            ]
        );
        harness.key_press(Key::Escape);
        harness.run();
        harness.run();
        assert_eq!(sent(&mut harness), [PickerCommand::Off]);
        assert!(!shown_on_the_photo(&harness));
        assert!(
            harness
                .get_by_role_and_label(Role::Button, QUAD)
                .is_focused()
        );
    }

    /// Screen readers move the picker along x with Increment and Decrement, and along y
    /// with its custom actions, Move up and Move down.
    #[test]
    fn screen_readers_move_the_picker_on_the_photo() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        on_the_photo(&harness).increment_accesskit();
        harness.run();
        harness.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: Action::CustomAction,
            target_tree: TreeId::ROOT,
            target_node: photo_id().accesskit_id(),
            data: Some(ActionData::CustomAction(MOVE_DOWN)),
        }));
        harness.run();
        let (x, y) = (1.0 / 400.0_f32, 1.0 / 300.0_f32);
        let point = |at| PickerCommand::Point {
            at,
            done: true,
            keys: PickKeys::default(),
        };
        assert_eq!(
            sent(&mut harness),
            [point([0.5 + x, 0.5]), point([0.5 + x, 0.5 + y])]
        );
    }

    /// The live region is a polite status on the photo.
    fn said(harness: &Harness<'_, Scene>) -> String {
        let live = harness
            .query_all_by_role(Role::Status)
            .find(|node| node.accesskit_node().data().live() == Some(Live::Polite))
            .expect("a live region");
        live.accesskit_node()
            .label()
            .unwrap_or_default()
            .to_string()
    }

    /// A pick is read out once a reading includes the last move and the picker has rested
    /// half a second: each control it changed with its value, else the frame's colour.
    #[test]
    fn a_pick_is_read_out_once_it_has_rested() {
        let words = Translator::new(Locale::EnGb);
        let mut values = values();
        let mut said = Said::default();
        let now = Instant::now();
        said.moved(3, now, BRIGHTNESS, &values);
        // A later step of the same drag keeps what the values were as it began.
        said.moved(4, now + Duration::from_millis(100), BRIGHTNESS, &values);
        let brightness = &mut values.modules[0].controls[0];
        assert_eq!(brightness.control, "brightness");
        brightness.value = ControlValue::Number(0.5);
        let at = |ms| now + Duration::from_millis(ms);
        let read = reading(BRIGHTNESS, DEFAULT_AREA, 4);
        let early = reading(BRIGHTNESS, DEFAULT_AREA, 3);
        let update =
            |said: &mut Said, ms, read| said.update(at(ms), Some(read), &values, MODULES, &words);
        assert_eq!(
            update(&mut said, 400, &read),
            Some(Duration::from_millis(200))
        );
        assert_eq!(said.text(), "");
        // Rested, but no reading includes the last move yet.
        assert_eq!(update(&mut said, 700, &early), None);
        assert_eq!(said.text(), "");
        assert_eq!(update(&mut said, 700, &read), None);
        assert_eq!(
            said.text(),
            "module-sample-control-brightness-label +0.50 EV"
        );

        // The picker turning off, or another on, forgets what was said and what waits.
        said.follow(Some(BRIGHTNESS));
        assert_ne!(said.text(), "");
        said.follow(None);
        assert_eq!(said.text(), "");
        said.moved(5, at(750), BRIGHTNESS, &values);
        said.follow(Some(SPREAD));
        assert_eq!(update(&mut said, 1400, &read), None);
        assert_eq!(said.text(), "");

        // A pick that changed nothing reads out the colour measured.
        said.moved(5, at(800), SHOW, &values);
        let read = reading(SHOW, Region::Point([0.5, 0.5]), 5);
        assert_eq!(update(&mut said, 1400, &read), None);
        assert_eq!(said.text(), "mean RGB 153, 128, 102");
    }

    /// The scene's live region says it, after the picker has rested.
    #[test]
    fn the_live_region_reads_the_pick_out() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SHOW_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        assert_eq!(said(&harness), "");
        // As the app's frame does: a frame once the rest is over.
        std::thread::sleep(RESTS);
        let scene = harness.state_mut();
        let words = Translator::new(Locale::EnGb);
        let reading = scene.mirror.reading().cloned();
        scene.said.update(
            Instant::now(),
            reading.as_ref(),
            &scene.values,
            MODULES,
            &words,
        );
        harness.run();
        assert_eq!(said(&harness), "mean RGB 153, 128, 102");
    }

    /// Esc's focus given back to a picker's button is taken by that button within two
    /// frames, never later, nor once a picker turns on again.
    #[test]
    fn the_focus_given_back_lasts_two_frames() {
        let ctx = egui::Context::default();
        let frame = |ctx: &egui::Context| {
            let mut output = ctx.run_ui(Default::default(), |_| {});
            output.textures_delta.clear();
        };
        give_back(&ctx, SPREAD);
        frame(&ctx);
        assert!(!takes_back(&ctx, SHOW));
        assert!(takes_back(&ctx, SPREAD));
        assert!(!takes_back(&ctx, SPREAD));

        give_back(&ctx, SPREAD);
        for _ in 0..3 {
            frame(&ctx);
        }
        assert!(!takes_back(&ctx, SPREAD));

        give_back(&ctx, SPREAD);
        focus_photo(&ctx);
        assert!(!takes_back(&ctx, SPREAD));
    }

    /// A release to where the last move left the picker brings no reading of its own: the
    /// live region reads the pick out from the move's reading once the picker has rested.
    #[test]
    fn a_release_where_the_move_left_it_waits_for_no_reading_of_its_own() {
        let words = Translator::new(Locale::EnGb);
        let ctx = egui::Context::default();
        let values = values();
        let (mut mirror, mut said) = (Mirror::default(), Said::default());
        let send = |number, command: PickerCommand, mirror: &mut Mirror, said: &mut Said| {
            Showing::of(&command, MODULES).sent(number, mirror, said, &values, &ctx);
        };
        send(
            1,
            on(SHOW, false, PickKeys::default()),
            &mut mirror,
            &mut said,
        );
        mirror.apply(1, Some(reading(SHOW, Region::Point([0.5, 0.5]), 1)));
        let point = |done| PickerCommand::Point {
            at: [0.3, 0.3],
            done,
            keys: PickKeys::default(),
        };
        send(2, point(false), &mut mirror, &mut said);
        send(3, point(true), &mut mirror, &mut said);
        // The session measures the move; the release, unchanged, brings nothing.
        mirror.apply(2, Some(reading(SHOW, Region::Point([0.3, 0.3]), 2)));
        let later = Instant::now() + RESTS + Duration::from_millis(100);
        assert_eq!(
            said.update(later, mirror.reading(), &values, MODULES, &words),
            None
        );
        assert_eq!(said.text(), "mean RGB 153, 128, 102");
    }

    /// `Switch` and `Reset` show at once where they take the picker, so a move after them,
    /// in the same frame or before their reading, moves that kind: Space then → on a
    /// point moves the area `Switch` went to, and a reset area is darktable's default.
    #[test]
    fn a_switch_or_a_reset_shows_at_once_and_the_keys_after_it_follow() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        harness.key_press(Key::Space);
        harness.key_press(Key::ArrowRight);
        harness.run();
        let x = 1.0 / 400.0_f32;
        assert_eq!(
            sent(&mut harness),
            [
                PickerCommand::Switch,
                PickerCommand::Area {
                    from: [0.02 + x, 0.02],
                    to: [0.98 + x, 0.98],
                    done: true,
                    keys: PickKeys::default(),
                }
            ]
        );
        // Switched back before any reading: the point it left.
        harness.key_press(Key::Space);
        harness.run();
        assert_eq!(sent(&mut harness), [PickerCommand::Switch]);
        assert_eq!(
            harness.state().mirror.region(),
            Some(Region::Point([0.5, 0.5]))
        );
        harness.key_press(Key::Space);
        harness.key_press(Key::Delete);
        harness.run();
        sent(&mut harness);
        assert_eq!(harness.state().mirror.region(), Some(DEFAULT_AREA));
    }

    /// A press on an area's corner and a move in the same frame take the corner, from
    /// where the press was, and follow the move.
    #[test]
    fn a_press_and_a_move_in_one_frame_take_the_corner_pressed() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let corner = at(0.98, 0.98);
        harness.hover_at(corner);
        harness.run();
        harness.event(egui::Event::PointerButton {
            pos: corner,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.event(egui::Event::PointerMoved(at(0.5, 0.5)));
        harness.run();
        assert_eq!(
            sent(&mut harness),
            [PickerCommand::Area {
                from: [0.02, 0.02],
                to: [0.5, 0.5],
                done: false,
                keys: PickKeys::default(),
            }]
        );
    }

    /// A reading measured before a move never replaces the place the move sent in the
    /// mirror's memory of places: switching away and back before the move's reading comes
    /// returns to the moved place.
    #[test]
    fn an_older_reading_keeps_the_place_a_later_move_sent() {
        let mut mirror = Mirror::default();
        mirror.asked(1, Some((BRIGHTNESS, true)));
        let (old, new) = ([0.1, 0.1, 0.4, 0.4], [0.2, 0.2, 0.6, 0.6]);
        mirror.apply(1, Some(reading(BRIGHTNESS, Region::Area(old), 1)));
        mirror.sent(2, Region::Area(new));
        // The frame before the move, measured late.
        mirror.apply(1, Some(reading(BRIGHTNESS, Region::Area(old), 1)));
        assert_eq!(
            mirror.switched(Region::Point([0.5, 0.5])),
            Region::Area(new)
        );
    }

    /// What the app asked shows until the session answers: a picker on or off, and the
    /// region sent, until a reading through its command; a refused command stops showing.
    #[test]
    fn what_the_app_asked_shows_until_the_session_answers() {
        let mut mirror = Mirror::default();
        mirror.asked(1, Some((SPREAD, false)));
        assert_eq!(mirror.active(), Some(SPREAD));
        assert_eq!((mirror.region(), mirror.area()), (None, Some(false)));
        mirror.apply(1, Some(reading(SPREAD, Region::Point([0.5, 0.5]), 1)));
        mirror.sent(2, Region::Point([0.2, 0.3]));
        assert_eq!(mirror.region(), Some(Region::Point([0.2, 0.3])));
        // A reading of the frame before the move leaves the move shown.
        mirror.apply(1, Some(reading(SPREAD, Region::Point([0.5, 0.5]), 1)));
        assert_eq!(mirror.region(), Some(Region::Point([0.2, 0.3])));
        mirror.apply(2, Some(reading(SPREAD, Region::Point([0.21, 0.3]), 2)));
        assert_eq!(mirror.region(), Some(Region::Point([0.21, 0.3])));

        // Another picker shows on at once, with no region until its reading.
        mirror.asked(3, Some((BRIGHTNESS, true)));
        assert_eq!((mirror.active(), mirror.region()), (Some(BRIGHTNESS), None));
        mirror.refused(3);
        assert_eq!(mirror.active(), Some(SPREAD));

        // A picker turned on that the session turns off again before its first reading,
        // as a change of its module does, by command 5: the end answers the ask.
        mirror.asked(4, Some((BRIGHTNESS, true)));
        mirror.apply(5, None);
        assert_eq!((mirror.active(), mirror.region()), (None, None));
        mirror.apply(5, Some(reading(SPREAD, Region::Point([0.21, 0.3]), 5)));

        // Off shows at once; a reading from before it changes nothing, and the end
        // confirms it.
        mirror.asked(6, None);
        assert_eq!(mirror.active(), None);
        mirror.apply(3, Some(reading(SPREAD, Region::Point([0.21, 0.3]), 3)));
        assert_eq!(mirror.active(), None);
        mirror.apply(6, None);
        assert_eq!((mirror.active(), mirror.region()), (None, None));
    }

    /// The central bar names the module and the picker, its control's label on a quad, and
    /// says how to use it: an area's sentence, or a point picker's own; an area picker
    /// switched to a point has none of its own. The Mac says Cmd.
    #[test]
    fn the_central_bar_names_the_picker_and_says_how_to_use_it() {
        let words = Translator::new(Locale::EnGb);
        let bar = |picker, area, mac| bar_words(&words, MODULES, picker, area, mac).unwrap();
        assert_eq!(
            bar(BRIGHTNESS, true, false),
            (
                "module-sample-title · module-sample-control-brightness-label".to_owned(),
                "Drag an area on the photo. Ctrl-click switches to a point. Esc ends.".to_owned()
            )
        );
        assert_eq!(
            bar(BRIGHTNESS, false, true).1,
            "Click or drag the point on the photo. Cmd-click switches to an area. Esc ends."
        );
        assert_eq!(
            bar(SPREAD, false, false),
            (
                "module-sample-title · module-sample-picker-spread-label".to_owned(),
                "module-sample-picker-spread-what Ctrl-click switches to an area. Esc ends."
                    .to_owned()
            )
        );
        assert_eq!(
            bar_words(&words, MODULES, PickerRef::ColourPicker, true, false),
            None
        );
    }
}
