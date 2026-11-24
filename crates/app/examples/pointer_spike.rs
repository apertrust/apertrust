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

//! The pointer spike (issue 555): a measuring instrument for custom pointers, on the
//! `spike/pointers` branch alone, never merged. Nothing here is a model for the real code.
//!
//! One window: a test pattern of single physical pixels, zones where the system's shapes
//! are set beside a zone where the custom image is, and a status line. Keys:
//!
//! - `1`, `2`, `3`: the crosshair with a picker's badge, the hands (closed while pressed),
//!   the 512-pixel circle;
//! - `S`: the box, 32 points, then 24, 32, 48, 64, 96, 128 and 256 physical pixels;
//! - `O`: the badge in the overlay (beside the system crosshair) or in the pointer;
//! - `A`: straight or premultiplied alpha in the bytes handed over;
//! - `M` (macOS): egui's `set_cursor_image` or the example's own `NSCursor`;
//! - `W` (Windows): `SetThreadCursorCreationScaling` with the window's DPI, or the default.
//!
//! It prints the platform's pointer values at start and whenever the window gains focus,
//! and once a second while the pointer moves, the main thread's time per frame.
//!
//! ```sh
//! cargo run --release -p apertrust --example pointer_spike
//! ```

#![allow(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::emath::GuiRounding as _;
use eframe::egui::{
    self, Color32, CursorIcon, CustomCursorImage, FontId, Key as Keyboard, Pos2, Rect, Sense,
    Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use egui_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use resvg::{tiny_skia, usvg};
use tiny_skia::{
    Color, GradientStop, LineCap, Paint, PathBuilder, Pixmap, Point, RadialGradient, SpreadMode,
    Transform,
};

/// The picker's badge: Lucide's pipette, as the app's `pick` icon.
const PIPETTE: &str = include_str!("../../ui/assets/icons/pick.svg");

/// The boxes `S` cycles through: `None` is 32 logical points at the display's scale, the
/// rest physical pixels. 96 makes the badged canvas 144 pixels, a size the Wayland row asks
/// for.
const BOXES: [Option<u32>; 8] = [
    None,
    Some(24),
    Some(32),
    Some(48),
    Some(64),
    Some(96),
    Some(128),
    Some(256),
];

/// Image 3's side in physical pixels.
const CIRCLE: u32 = 512;

/// The overlay badge's size and its offset below and right of the hotspot, in points, as
/// the app draws it (`apertrust_ui::overlay::Overlay::badge`).
const OVERLAY_BADGE: f32 = 12.0;
const OVERLAY_OFFSET: f32 = 12.0;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pointer spike")
            .with_inner_size([1280.0, 860.0]),
        ..Default::default()
    };
    eframe::run_native(
        "pointer-spike",
        options,
        Box::new(|_| Ok(Box::new(Spike::new()))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Image {
    Crosshair,
    Hands,
    Circle,
}

/// One rendered pointer: what it is, its size (the box for the crosshair and the hands, the
/// side for the circle) and the alpha of its bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Key {
    image: Image,
    closed: bool,
    badge: bool,
    size: u32,
    straight: bool,
}

/// An image rendered with tiny-skia, premultiplied, and its hotspot in pixels.
struct Art {
    pixmap: Pixmap,
    hotspot: [u32; 2],
}

/// The pipette's two drawings: the dark halo under the white line.
struct Badges {
    halo: usvg::Tree,
    line: usvg::Tree,
}

impl Badges {
    fn new() -> Self {
        let parse = |colour: &str, width: &str| {
            let svg = PIPETTE
                .replace("stroke=\"currentColor\"", &format!("stroke=\"{colour}\""))
                .replace("stroke-width=\"2\"", &format!("stroke-width=\"{width}\""));
            usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("the pipette parses")
        };
        Self {
            halo: parse("#202020", "5"),
            line: parse("#ffffff", "2"),
        }
    }

    /// Draws the badge `size` pixels square with its top-left corner at `at`.
    fn draw(&self, pixmap: &mut Pixmap, at: [f32; 2], size: f32) {
        let scale = size / 24.0;
        let transform = Transform::from_scale(scale, scale).post_translate(at[0], at[1]);
        resvg::render(&self.halo, transform, &mut pixmap.as_mut());
        resvg::render(&self.line, transform, &mut pixmap.as_mut());
    }
}

/// The main thread's frames over one second.
struct Stats {
    since: Instant,
    cpu: Option<Duration>,
    frames: u32,
    moving: u32,
    intervals: u32,
    interval_sum: f64,
    interval_max: f64,
    ui_sum: f64,
    last_frame: Option<Instant>,
    last_moving: bool,
    line: String,
}

impl Stats {
    fn new() -> Self {
        Self {
            since: Instant::now(),
            cpu: thread_cpu(),
            frames: 0,
            moving: 0,
            intervals: 0,
            interval_sum: 0.0,
            interval_max: 0.0,
            ui_sum: 0.0,
            last_frame: None,
            last_moving: false,
            line: "frame: move the pointer".to_owned(),
        }
    }

    /// Counts one frame that began at `start`, its `ui` taking `ui`. Returns the second's
    /// line once a second has passed and the pointer moved in it.
    fn frame(&mut self, start: Instant, moving: bool, ui: Duration) -> Option<String> {
        self.frames += 1;
        if moving {
            self.moving += 1;
            self.ui_sum += ui.as_secs_f64();
            if let (true, Some(last)) = (self.last_moving, self.last_frame) {
                let interval = (start - last).as_secs_f64();
                self.intervals += 1;
                self.interval_sum += interval;
                self.interval_max = self.interval_max.max(interval);
            }
        }
        self.last_frame = Some(start);
        self.last_moving = moving;
        let elapsed = self.since.elapsed();
        if elapsed < Duration::from_secs(1) {
            return None;
        }
        let cpu_now = thread_cpu();
        let cpu = match (self.cpu, cpu_now) {
            (Some(before), Some(now)) => {
                let spent = (now - before).as_secs_f64();
                format!(
                    "main thread CPU {:.3} ms a frame, {:.1} % of a core",
                    spent * 1e3 / f64::from(self.frames),
                    spent * 100.0 / elapsed.as_secs_f64()
                )
            }
            _ => "main thread CPU n/a".to_owned(),
        };
        let printed = (self.moving > 0).then(|| {
            let moving = f64::from(self.moving);
            self.line = format!(
                "frame: {} moving of {} frames, interval mean {:.2} ms, max {:.2} ms, ui {:.3} ms, {cpu}",
                self.moving,
                self.frames,
                self.interval_sum * 1e3 / f64::from(self.intervals.max(1)),
                self.interval_max * 1e3,
                self.ui_sum * 1e3 / moving,
            );
            self.line.clone()
        });
        self.since = Instant::now();
        self.cpu = cpu_now;
        self.frames = 0;
        self.moving = 0;
        self.intervals = 0;
        self.interval_sum = 0.0;
        self.interval_max = 0.0;
        self.ui_sum = 0.0;
        printed
    }
}

struct Spike {
    image: Image,
    box_index: usize,
    /// The badge inside the pointer (`O`); otherwise the overlay's, beside the system's
    /// crosshair.
    badge_in_pointer: bool,
    straight: bool,
    /// The example's own `NSCursor` rather than egui's `set_cursor_image` (`M`, macOS).
    ns_cursor: bool,
    /// While `W` is on, the creation scaling `SetThreadCursorCreationScaling` returned.
    creation_scaling: Option<u32>,
    badges: Badges,
    pointers: HashMap<Key, CustomCursorImage>,
    /// Images dropped from `pointers`, kept so a new one never reuses an old one's address,
    /// which egui-winit caches the system's pointer by.
    retired: Vec<CustomCursorImage>,
    pattern: Option<(TextureHandle, [usize; 2])>,
    overlay_badge: Option<(TextureHandle, u32)>,
    stats: Stats,
    focused: Option<bool>,
    #[cfg(target_os = "macos")]
    mac: mac::Route,
}

impl Spike {
    fn new() -> Self {
        Self {
            image: Image::Crosshair,
            box_index: 0,
            badge_in_pointer: true,
            straight: true,
            ns_cursor: false,
            creation_scaling: None,
            badges: Badges::new(),
            pointers: HashMap::new(),
            retired: Vec::new(),
            pattern: None,
            overlay_badge: None,
            stats: Stats::new(),
            focused: None,
            #[cfg(target_os = "macos")]
            mac: mac::Route::new(),
        }
    }

    /// The box in physical pixels at the display's scale `native`.
    fn box_px(&self, native: f32) -> u32 {
        BOXES[self.box_index].unwrap_or_else(|| (32.0 * native).round() as u32)
    }

    /// The pointer the custom zone shows, at `size` (the box, or the circle's side).
    fn key(&self, closed: bool, size: u32) -> Key {
        Key {
            image: self.image,
            closed: closed && self.image == Image::Hands,
            badge: self.image == Image::Crosshair && self.badge_in_pointer,
            size: if self.image == Image::Circle {
                CIRCLE
            } else {
                size
            },
            straight: self.straight,
        }
    }

    /// Whether the custom zone shows an image of the example's, not the system's crosshair
    /// with the overlay's badge.
    fn custom_image(&self) -> bool {
        self.image != Image::Crosshair || self.badge_in_pointer
    }

    fn pointer(&mut self, key: Key) -> CustomCursorImage {
        let badges = &self.badges;
        self.pointers
            .entry(key)
            .or_insert_with(|| {
                let art = art(key, badges);
                CustomCursorImage {
                    rgba: Arc::from(bytes(&art.pixmap, key.straight)),
                    size: [art.pixmap.width() as u16, art.pixmap.height() as u16],
                    hotspot: [art.hotspot[0] as u16, art.hotspot[1] as u16],
                }
            })
            .clone()
    }

    fn route(&self) -> &'static str {
        if !self.custom_image() {
            "system Crosshair, badge drawn by egui in the overlay"
        } else if self.ns_cursor {
            "the example's NSCursor (1x and 2x representations), set every frame"
        } else {
            "egui set_cursor_image"
        }
    }

    fn describe(&mut self, native: f32) -> String {
        let box_name = match BOXES[self.box_index] {
            None => format!("box 32 pt = {} px", self.box_px(native)),
            Some(px) => format!("box {px} px"),
        };
        let name = match (self.image, self.custom_image()) {
            (Image::Crosshair, true) => "1 crosshair with badge",
            (Image::Crosshair, false) => "1 crosshair, badge in the overlay",
            (Image::Hands, _) => "2 hands (closed while pressed)",
            (Image::Circle, _) => "3 circle",
        };
        let size = if self.custom_image() {
            let image = self.pointer(self.key(false, self.box_px(native)));
            format!(
                "{}×{} px, hotspot ({}, {})",
                image.size[0], image.size[1], image.hotspot[0], image.hotspot[1]
            )
        } else {
            "system size".to_owned()
        };
        let alpha = if self.straight {
            "straight"
        } else {
            "premultiplied"
        };
        format!(
            "image {name}, {size}, {box_name}, {alpha} alpha; route: {}",
            self.route()
        )
    }

    /// Prints the platform's values at start and whenever the window gains focus.
    fn print_on_focus(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let focused = ctx.input(|i| i.focused);
        let when = match self.focused {
            None => "start",
            Some(false) if focused => "focus",
            Some(_) => "",
        };
        self.focused = Some(focused);
        if when.is_empty() {
            return;
        }
        let (native, ppp) =
            ctx.input(|i| (i.viewport().native_pixels_per_point, i.pixels_per_point()));
        println!(
            "== {when}: native_pixels_per_point {native:?}, pixels_per_point {ppp}, window {}",
            backend(frame)
        );
        #[cfg(target_os = "linux")]
        linux::print_values();
        #[cfg(target_os = "macos")]
        mac::print_values();
        #[cfg(windows)]
        win::print_values(win::hwnd(frame));
    }

    fn keys(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let pressed = |key| ctx.input(|i| i.key_pressed(key));
        let mut changed = false;
        for (key, image) in [
            (Keyboard::Num1, Image::Crosshair),
            (Keyboard::Num2, Image::Hands),
            (Keyboard::Num3, Image::Circle),
        ] {
            if pressed(key) {
                self.image = image;
                changed = true;
            }
        }
        if pressed(Keyboard::S) {
            self.box_index = (self.box_index + 1) % BOXES.len();
            changed = true;
        }
        if pressed(Keyboard::O) {
            self.badge_in_pointer = !self.badge_in_pointer;
            changed = true;
        }
        if pressed(Keyboard::A) {
            self.straight = !self.straight;
            changed = true;
        }
        if cfg!(target_os = "macos") && pressed(Keyboard::M) {
            self.ns_cursor = !self.ns_cursor;
            #[cfg(target_os = "macos")]
            self.mac.reset();
            changed = true;
        }
        if cfg!(windows) && pressed(Keyboard::W) {
            self.toggle_creation_scaling(frame);
            changed = true;
        }
        if changed {
            let native = ctx.input(|i| i.viewport().native_pixels_per_point.unwrap_or(1.0));
            println!("pointer: {}", self.describe(native));
        }
    }

    /// `W`: sets the thread's cursor creation scaling to the window's DPI, or back to what
    /// it was, and drops the cached pointers so they are created again under it.
    fn toggle_creation_scaling(&mut self, frame: &eframe::Frame) {
        #[cfg(windows)]
        {
            let hwnd = win::hwnd(frame);
            self.creation_scaling = match self.creation_scaling.take() {
                None => {
                    let dpi = hwnd.map_or(96, win::dpi);
                    let previous = win::set_creation_scaling(dpi);
                    match previous {
                        Some(previous) => println!(
                            "W on: SetThreadCursorCreationScaling({dpi}) returned {previous}"
                        ),
                        None => {
                            println!("W: SetThreadCursorCreationScaling is not in this Windows")
                        }
                    }
                    previous
                }
                Some(previous) => {
                    let returned = win::set_creation_scaling(previous);
                    println!(
                        "W off: SetThreadCursorCreationScaling({previous}) returned {returned:?}"
                    );
                    None
                }
            };
        }
        #[cfg(not(windows))]
        let _ = frame;
        self.retired
            .extend(self.pointers.drain().map(|(_, image)| image));
    }

    /// The test pattern, one texel a physical pixel, regenerated when `size` changes.
    fn pattern(&mut self, ctx: &egui::Context, size: [usize; 2]) -> egui::TextureId {
        if self.pattern.as_ref().is_none_or(|(_, at)| *at != size) {
            let texture = ctx.load_texture("pattern", pattern(size), TextureOptions::NEAREST);
            self.pattern = Some((texture, size));
        }
        self.pattern.as_ref().expect("made above").0.id()
    }

    /// The overlay's badge, `px` pixels square.
    fn overlay_badge(&mut self, ctx: &egui::Context, px: u32) -> egui::TextureId {
        if self.overlay_badge.as_ref().is_none_or(|(_, at)| *at != px) {
            let mut pixmap = Pixmap::new(px, px).expect("a badge");
            self.badges.draw(&mut pixmap, [0.0, 0.0], px as f32);
            let image = egui::ColorImage::from_rgba_premultiplied(
                [px as usize, px as usize],
                pixmap.data(),
            );
            let texture = ctx.load_texture("badge", image, TextureOptions::NEAREST);
            self.overlay_badge = Some((texture, px));
        }
        self.overlay_badge.as_ref().expect("made above").0.id()
    }
}

impl eframe::App for Spike {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let start = Instant::now();
        let ctx = ui.ctx().clone();
        self.print_on_focus(&ctx, frame);
        self.keys(&ctx, frame);

        let ppp = ctx.pixels_per_point();
        let native = ctx.input(|i| i.viewport().native_pixels_per_point.unwrap_or(1.0));
        let moving = ctx.input(|i| i.pointer.is_moving());
        let painter = ui.painter().clone();
        let full = ui.max_rect();

        // The status lines.
        let status = Rect::from_min_size(full.min, vec2(full.width(), 66.0));
        painter.rect_filled(status, 0.0, Color32::from_gray(24));
        let description = self.describe(native);
        let mut lines = vec![
            description,
            format!(
                "{}; native_pixels_per_point {native}, pixels_per_point {ppp}",
                self.stats.line
            ),
            "keys: 1 2 3 image · S box · O badge in pointer or overlay · A alpha".to_owned(),
        ];
        if cfg!(target_os = "macos") {
            lines[2].push_str(" · M route");
            #[cfg(target_os = "macos")]
            lines[1].push_str(&format!("; {}", self.mac.summary()));
        }
        if cfg!(windows) {
            lines[2].push_str(&format!(
                " · W creation scaling ({})",
                if self.creation_scaling.is_some() {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        for (row, line) in lines.into_iter().enumerate() {
            painter.text(
                status.min + vec2(8.0, 6.0 + 19.0 * row as f32),
                egui::Align2::LEFT_TOP,
                line,
                FontId::monospace(13.0),
                Color32::from_gray(230),
            );
        }

        // The pattern, on whole physical pixels.
        let area = Rect::from_min_max(pos2(full.min.x, status.max.y), full.max);
        let min = pos2((area.min.x * ppp).ceil(), (area.min.y * ppp).ceil());
        let max = pos2((area.max.x * ppp).floor(), (area.max.y * ppp).floor());
        let size = [
            (max.x - min.x).max(1.0) as usize,
            (max.y - min.y).max(1.0) as usize,
        ];
        let shown = Rect::from_min_max(
            (min.to_vec2() / ppp).to_pos2(),
            (max.to_vec2() / ppp).to_pos2(),
        );
        let pattern = self.pattern(&ctx, size);
        painter.image(
            pattern,
            shown,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        // The zones: the system's shapes in a row, the custom pointer below.
        let inner = shown.shrink(12.0);
        let row = inner.height() * 0.38;
        let system = [
            ("Default", CursorIcon::Default),
            ("Crosshair", CursorIcon::Crosshair),
            ("Grab", CursorIcon::Grab),
            ("Grabbing", CursorIcon::Grabbing),
            ("PointingHand (IDC_HAND)", CursorIcon::PointingHand),
        ];
        let width = (inner.width() - 12.0 * (system.len() - 1) as f32) / system.len() as f32;
        let mut over_system = None;
        for (i, (name, icon)) in system.iter().enumerate() {
            let rect = Rect::from_min_size(
                inner.min + vec2(i as f32 * (width + 12.0), 0.0),
                vec2(width, row),
            );
            let response = ui.interact(rect, ui.scope_id().with(("system", i)), Sense::drag());
            if response.hovered() || response.is_pointer_button_down_on() {
                ctx.set_cursor_icon(*icon);
                over_system = Some(*name);
            }
            zone(&painter, rect, &format!("system: {name}"));
        }
        let custom = Rect::from_min_max(inner.min + vec2(0.0, row + 12.0), inner.max);
        let response = ui.interact(custom, ui.scope_id().with("custom"), Sense::drag());
        zone(&painter, custom, "custom: the selected image (keys above)");
        let over_custom = response.hovered() || response.is_pointer_button_down_on();
        let closed = response.is_pointer_button_down_on();

        #[cfg(target_os = "macos")]
        let mut ns_set = false;
        if over_custom {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
            let box_px = self.box_px(native);
            if !self.custom_image() {
                ctx.set_cursor_image(None);
                if let Some(at) = ctx.input(|i| i.pointer.latest_pos()) {
                    let px = (OVERLAY_BADGE * ppp).round() as u32;
                    let texture = self.overlay_badge(&ctx, px);
                    let min = (at + Vec2::splat(OVERLAY_OFFSET)).round_to_pixels(ppp);
                    let rect = Rect::from_min_size(min, Vec2::splat(px as f32 / ppp));
                    ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Foreground,
                        ui.scope_id().with("badge"),
                    ))
                    .image(
                        texture,
                        rect,
                        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            } else if self.ns_cursor {
                ctx.set_cursor_image(None);
                #[cfg(target_os = "macos")]
                {
                    let key = self.key(closed, box_px);
                    self.mac.set(key, native, &self.badges);
                    ns_set = true;
                    // AppKit's resets show between frames, so the route keeps drawing.
                    ctx.request_repaint();
                }
            } else {
                let image = self.pointer(self.key(closed, box_px));
                ctx.set_cursor_image(Some(image));
            }
        } else {
            ctx.set_cursor_image(None);
        }
        #[cfg(target_os = "macos")]
        if !ns_set {
            self.mac.idle();
        }

        let ui_time = start.elapsed();
        if let Some(line) = self.stats.frame(start, moving, ui_time) {
            let zone = if over_custom {
                "custom".to_owned()
            } else {
                over_system.map_or("none".to_owned(), |name| format!("system {name}"))
            };
            #[cfg(target_os = "macos")]
            let line = format!("{line}; {}", self.mac.summary());
            println!("{line}; zone {zone}; pointer: {}", self.describe(native));
        }
    }
}

/// A zone's outline and its label.
fn zone(painter: &egui::Painter, rect: Rect, label: &str) {
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, Color32::from_rgb(255, 160, 0)),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(label.to_owned(), FontId::monospace(12.0), Color32::WHITE);
    let chip = Rect::from_min_size(rect.min + vec2(4.0, 4.0), galley.size() + vec2(8.0, 4.0));
    painter.rect_filled(chip, 2.0, Color32::from_black_alpha(210));
    painter.galley(chip.min + vec2(4.0, 2.0), galley, Color32::WHITE);
}

/// The window's backend, from its handle.
fn backend(frame: &eframe::Frame) -> &'static str {
    match frame.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Wayland(_)) => "Wayland",
        Ok(RawWindowHandle::Xlib(_)) => "X11 (Xlib)",
        Ok(RawWindowHandle::Xcb(_)) => "X11 (xcb)",
        Ok(RawWindowHandle::AppKit(_)) => "AppKit",
        Ok(RawWindowHandle::Win32(_)) => "Win32",
        Ok(_) => "other",
        Err(_) => "unknown",
    }
}

/// The test pattern, `size` physical pixels: bands 96 pixels wide of white, black and a
/// busy, photo-like texture, each with a 1-pixel grid every 16 pixels and an isolated
/// 1-pixel dot in each cell's middle.
fn pattern(size: [usize; 2]) -> egui::ColorImage {
    let [w, h] = size;
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let grid = x % 16 == 0 || y % 16 == 0;
            let dot = x % 16 == 8 && y % 16 == 8;
            let colour = match (x / 96) % 3 {
                0 if dot => [0, 0, 0],
                0 if grid => [150, 150, 150],
                0 => [255, 255, 255],
                1 if dot => [255, 255, 255],
                1 if grid => [100, 100, 100],
                1 => [0, 0, 0],
                _ if dot => [255, 255, 255],
                _ if grid => [0, 0, 0],
                _ => busy(x, y),
            };
            let at = (y * w + x) * 4;
            rgba[at..at + 4].copy_from_slice(&[colour[0], colour[1], colour[2], 255]);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied(size, &rgba)
}

/// A busy texture: colour waves with noise, as a crop of foliage might be.
fn busy(x: usize, y: usize) -> [u8; 3] {
    let (fx, fy) = (x as f32, y as f32);
    let noise = ((x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) % 97) as f32 - 48.0;
    let wave = |v: f32| (128.0 + 90.0 * v.sin() + noise).clamp(0.0, 255.0) as u8;
    [
        wave(fx * 0.031 + (fy * 0.017).sin() * 3.0),
        wave(fy * 0.029 + fx * 0.011),
        wave((fx + fy) * 0.023 + (fx * 0.05).sin()),
    ]
}

/// The bytes handed over: tiny-skia's premultiplied pixels as they are, or straight.
fn bytes(pixmap: &Pixmap, straight: bool) -> Vec<u8> {
    if !straight {
        return pixmap.data().to_vec();
    }
    pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let c = pixel.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

fn art(key: Key, badges: &Badges) -> Art {
    match key.image {
        Image::Crosshair => crosshair(key.size, key.badge, badges),
        Image::Hands => hand(key.size, key.closed),
        Image::Circle => circle(key.size),
    }
}

fn solid(rgba: [u8; 4], anti_alias: bool) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
    paint.anti_alias = anti_alias;
    paint
}

/// Fills `[x0, x1) × [y0, y1)` in whole pixels.
fn fill(pixmap: &mut Pixmap, [x0, y0, x1, y1]: [i32; 4], rgba: [u8; 4]) {
    if let Some(rect) = tiny_skia::Rect::from_ltrb(x0 as f32, y0 as f32, x1 as f32, y1 as f32) {
        pixmap.fill_rect(rect, &solid(rgba, false), Transform::identity(), None);
    }
}

/// The crosshair in a box `b` pixels square, its centre pixel the hotspot and left open;
/// with `badge`, on a canvas 1.5 times the box with the pipette in its lower-right quadrant.
/// The halo is dark grey with soft edges, and the arms' tips half-transparent white, so the
/// alpha the bytes carry shows.
fn crosshair(b: u32, badge: bool, badges: &Badges) -> Art {
    let canvas = if badge { b * 3 / 2 } else { b };
    let mut pixmap = Pixmap::new(canvas.max(1), canvas.max(1)).expect("a canvas");
    let b = b as i32;
    let c = b / 2;
    let half = b / 48;
    let gap = (b / 8).max(2);
    let halo = 1 + b / 48;
    let (lo, hi) = (halo + 1, b - halo - 1);
    let arms = [
        [lo, c - half, c - gap, c + half + 1],
        [c + gap + 1, c - half, hi, c + half + 1],
        [c - half, lo, c + half + 1, c - gap],
        [c - half, c + gap + 1, c + half + 1, hi],
    ];
    for (grow, alpha) in [(halo + 1, 90), (halo, 220)] {
        for [x0, y0, x1, y1] in arms {
            fill(
                &mut pixmap,
                [x0 - grow, y0 - grow, x1 + grow, y1 + grow],
                [32, 32, 32, alpha],
            );
        }
    }
    for arm in arms {
        fill(&mut pixmap, arm, [255, 255, 255, 255]);
    }
    // Half-transparent white beyond each outer tip.
    let tips = [
        [lo - 1, c - half, lo, c + half + 1],
        [hi, c - half, hi + 1, c + half + 1],
        [c - half, lo - 1, c + half + 1, lo],
        [c - half, hi, c + half + 1, hi + 1],
    ];
    for tip in tips {
        fill(&mut pixmap, tip, [255, 255, 255, 128]);
    }
    if badge {
        let size = (b * 5 / 8) as f32;
        let at = canvas as f32 - size - 1.0;
        badges.draw(&mut pixmap, [at, at], size);
    }
    Art {
        pixmap,
        hotspot: [c as u32, c as u32],
    }
}

/// A rough open or closed hand in a box `b` pixels square, drawn on a 32-unit grid as thick
/// round-capped strokes: dark outlines first, then the white fill over them. The hotspot is
/// the palm's centre.
fn hand(b: u32, closed: bool) -> Art {
    let mut pixmap = Pixmap::new(b.max(1), b.max(1)).expect("a canvas");
    let u = b as f32 / 32.0;
    // (from, to, width) in units.
    let open: &[([f32; 2], [f32; 2], f32)] = &[
        ([12.0, 21.0], [20.0, 21.0], 11.0),
        ([11.5, 17.0], [11.5, 7.0], 3.2),
        ([15.0, 16.0], [15.0, 5.0], 3.2),
        ([18.5, 17.0], [18.5, 6.5], 3.2),
        ([22.0, 18.0], [22.0, 9.5], 3.0),
        ([9.0, 22.0], [5.0, 16.0], 3.4),
    ];
    let fist: &[([f32; 2], [f32; 2], f32)] = &[
        ([12.0, 20.0], [20.0, 20.0], 11.0),
        ([11.5, 15.0], [11.5, 12.5], 3.2),
        ([15.0, 15.0], [15.0, 12.0], 3.2),
        ([18.5, 15.0], [18.5, 12.5], 3.2),
        ([22.0, 16.0], [22.0, 14.0], 3.0),
        ([9.5, 21.0], [12.5, 18.0], 3.2),
    ];
    let strokes = if closed { fist } else { open };
    let outline = (1.2 * u).max(1.0);
    for (grow, rgba) in [
        (2.0 * outline, [24, 24, 24, 255]),
        (0.0, [255, 255, 255, 255]),
    ] {
        for (from, to, width) in strokes {
            let mut path = PathBuilder::new();
            path.move_to(from[0] * u, from[1] * u);
            path.line_to(to[0] * u, to[1] * u);
            let Some(path) = path.finish() else { continue };
            let stroke = tiny_skia::Stroke {
                width: width * u + grow,
                line_cap: LineCap::Round,
                ..Default::default()
            };
            pixmap.stroke_path(
                &path,
                &solid(rgba, true),
                &stroke,
                Transform::identity(),
                None,
            );
        }
    }
    let palm_y = if closed { 20.0 } else { 21.0 };
    Art {
        pixmap,
        hotspot: [(16.0 * u) as u32, (palm_y * u) as u32],
    }
}

/// A circle `d` pixels across: a soft white glow, a dark ring, a grey disc and a black
/// centre pixel, the hotspot.
fn circle(d: u32) -> Art {
    let mut pixmap = Pixmap::new(d.max(1), d.max(1)).expect("a canvas");
    let r = d as f32 / 2.0;
    let centre = Point::from_xy(r, r);
    if let Some(shader) = RadialGradient::new(
        centre,
        0.76 * r,
        centre,
        0.98 * r,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 230)),
            GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    ) && let Some(path) = PathBuilder::from_circle(r, r, 0.98 * r)
    {
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Default::default()
        };
        pixmap.fill_path(
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    for (radius, rgba) in [
        (0.76 * r, [32, 32, 32, 255]),
        (0.71 * r, [128, 128, 128, 255]),
    ] {
        if let Some(path) = PathBuilder::from_circle(r, r, radius) {
            pixmap.fill_path(
                &path,
                &solid(rgba, true),
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
    let c = (d / 2) as i32;
    fill(&mut pixmap, [c, c, c + 1, c + 1], [0, 0, 0, 255]);
    Art {
        pixmap,
        hotspot: [c as u32, c as u32],
    }
}

/// The calling thread's CPU time so far, where the platform gives it.
fn thread_cpu() -> Option<Duration> {
    #[cfg(target_os = "linux")]
    return linux::thread_cpu();
    #[cfg(windows)]
    return win::thread_cpu();
    #[cfg(not(any(target_os = "linux", windows)))]
    None
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::PathBuf;
    use std::time::Duration;

    /// The main thread's time on a CPU, from the scheduler's statistics, in nanoseconds.
    pub fn thread_cpu() -> Option<Duration> {
        let text = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
        let nanos = text.split_whitespace().next()?.parse().ok()?;
        Some(Duration::from_nanos(nanos))
    }

    /// Prints the pointer's size and theme from every source, on a thread of its own so a
    /// slow portal never holds a frame.
    pub fn print_values() {
        std::thread::spawn(|| {
            let env = |name: &str| std::env::var(name).unwrap_or_else(|_| "unset".to_owned());
            let mut lines = vec![format!(
                "session: XDG_SESSION_TYPE {}, XDG_CURRENT_DESKTOP {}, WAYLAND_DISPLAY {}, DISPLAY {}",
                env("XDG_SESSION_TYPE"),
                env("XDG_CURRENT_DESKTOP"),
                env("WAYLAND_DISPLAY"),
                env("DISPLAY")
            )];
            lines.push(format!("XCURSOR_SIZE {}", env("XCURSOR_SIZE")));
            lines.push(format!("XCURSOR_THEME {}", env("XCURSOR_THEME")));
            lines.extend(portal());
            lines.extend(kde());
            lines.extend(x11());
            println!("{}", lines.join("\n"));
        });
    }

    /// `org.gnome.desktop.interface`'s `cursor-size` and `cursor-theme` through the settings
    /// portal: `ReadOne`, or `Read` where the portal is older.
    fn portal() -> Vec<String> {
        let connection = match zbus::blocking::Connection::session() {
            Ok(connection) => connection,
            Err(error) => return vec![format!("settings portal: no session bus ({error})")],
        };
        ["cursor-size", "cursor-theme"]
            .into_iter()
            .map(|key| {
                let read = |method: &str| -> zbus::Result<zbus::zvariant::OwnedValue> {
                    let reply = connection.call_method(
                        Some("org.freedesktop.portal.Desktop"),
                        "/org/freedesktop/portal/desktop",
                        Some("org.freedesktop.portal.Settings"),
                        method,
                        &("org.gnome.desktop.interface", key),
                    )?;
                    reply.body().deserialize()
                };
                match read("ReadOne").or_else(|_| read("Read")) {
                    Ok(value) => format!("portal org.gnome.desktop.interface {key}: {value:?}"),
                    Err(error) => {
                        format!("portal org.gnome.desktop.interface {key}: unread ({error})")
                    }
                }
            })
            .collect()
    }

    /// `kcminputrc`'s `[Mouse]` `cursorSize` and `cursorTheme`.
    fn kde() -> Vec<String> {
        let dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
        let Some(path) = dir.map(|dir| dir.join("kcminputrc")) else {
            return vec!["kcminputrc: no config folder".to_owned()];
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return vec![format!("kcminputrc: none at {}", path.display())];
        };
        let mut lines = Vec::new();
        let mut mouse = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                mouse = line == "[Mouse]";
            } else if mouse
                && let Some((key, value)) = line.split_once('=')
                && matches!(key.trim(), "cursorSize" | "cursorTheme")
            {
                lines.push(format!(
                    "kcminputrc [Mouse] {}: {}",
                    key.trim(),
                    value.trim()
                ));
            }
        }
        if lines.is_empty() {
            lines.push(format!(
                "kcminputrc [Mouse] cursorSize: not set ({})",
                path.display()
            ));
        }
        lines
    }

    /// The X resources `Xcursor.size`, `Xcursor.theme` and `Xft.dpi`, where there is an X
    /// server (XWayland's under Wayland).
    fn x11() -> Vec<String> {
        if std::env::var_os("DISPLAY").is_none() {
            return vec!["X resources: no DISPLAY".to_owned()];
        }
        let connection = match x11rb::connect(None) {
            Ok((connection, _)) => connection,
            Err(error) => return vec![format!("X resources: no connection ({error})")],
        };
        match x11rb::resource_manager::new_from_default(&connection) {
            Ok(database) => ["Xcursor.size", "Xcursor.theme", "Xft.dpi"]
                .into_iter()
                .map(|name| {
                    format!(
                        "X resource {name}: {}",
                        database.get_string(name, "").unwrap_or("unset")
                    )
                })
                .collect(),
            Err(error) => vec![format!("X resources: unread ({error})")],
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::collections::HashMap;

    use objc2::AnyThread;
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSBitmapFormat, NSBitmapImageRep, NSCursor, NSDeviceRGBColorSpace, NSImage,
    };
    use objc2_foundation::{NSPoint, NSSize};

    use super::{Badges, Key, art, bytes};

    pub fn print_values() {
        let cursor = NSCursor::crosshairCursor();
        let image = cursor.image();
        let size = image.size();
        let hotspot = cursor.hotSpot();
        let representations = image.representations();
        let pixels: Vec<String> = (0..representations.count())
            .map(|i| {
                let representation = representations.objectAtIndex(i);
                format!(
                    "{}×{}",
                    representation.pixelsWide(),
                    representation.pixelsHigh()
                )
            })
            .collect();
        println!(
            "NSCursor.crosshairCursor.image.size {} × {} pt, .hotSpot ({}, {}), representations {}",
            size.width,
            size.height,
            hotspot.x,
            hotspot.y,
            pixels.join(", ")
        );
    }

    /// The example's own `NSCursor`s and the count of frames AppKit had reset them in.
    pub struct Route {
        cursors: HashMap<(Key, u32), Retained<NSCursor>>,
        set_last_frame: Option<*const NSCursor>,
        frames: u64,
        resets: u64,
        since: std::time::Instant,
    }

    impl Route {
        pub fn new() -> Self {
            Self {
                cursors: HashMap::new(),
                set_last_frame: None,
                frames: 0,
                resets: 0,
                since: std::time::Instant::now(),
            }
        }

        pub fn reset(&mut self) {
            self.set_last_frame = None;
            self.frames = 0;
            self.resets = 0;
            self.since = std::time::Instant::now();
        }

        /// A frame off the route: the next one on it starts a new run.
        pub fn idle(&mut self) {
            self.set_last_frame = None;
        }

        pub fn summary(&self) -> String {
            format!(
                "NSCursor: {} of {} frames found reset, over {:.0} s",
                self.resets,
                self.frames,
                self.since.elapsed().as_secs_f64()
            )
        }

        /// Sets the cursor for `key`, counting the frame as reset when the current cursor
        /// is no longer the one set the frame before.
        pub fn set(&mut self, key: Key, native: f32, badges: &Badges) {
            let scale = (native * 100.0).round() as u32;
            let cursor = self
                .cursors
                .entry((key, scale))
                .or_insert_with(|| cursor(key, native, badges))
                .clone();
            let current = NSCursor::currentCursor();
            if let Some(last) = self.set_last_frame {
                self.frames += 1;
                if Retained::as_ptr(&current) != last {
                    self.resets += 1;
                    println!("NSCursor reset found: {}", self.summary());
                }
            }
            cursor.set();
            self.set_last_frame = Some(Retained::as_ptr(&cursor));
        }
    }

    /// An `NSImage` sized in points, `key`'s size divided by the display's scale, with a 1x
    /// and a 2x `NSBitmapImageRep`, each rendered at its own size, the bytes declared
    /// straight as egui's are.
    fn cursor(key: Key, native: f32, badges: &Badges) -> Retained<NSCursor> {
        let points = key.size as f32 / native;
        let one = art(
            Key {
                size: points.round() as u32,
                ..key
            },
            badges,
        );
        let two = art(
            Key {
                size: (2.0 * points).round() as u32,
                ..key
            },
            badges,
        );
        let size = NSSize::new(
            f64::from(two.pixmap.width()) / 2.0,
            f64::from(two.pixmap.height()) / 2.0,
        );
        let image = NSImage::initWithSize(NSImage::alloc(), size);
        for art in [&one, &two] {
            let data = bytes(&art.pixmap, key.straight);
            let (w, h) = (art.pixmap.width() as isize, art.pixmap.height() as isize);
            // SAFETY: a null plane makes AppKit allocate the bitmap, `w * 4` bytes a row,
            // which `data` fills exactly.
            let representation = unsafe {
                NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
                    NSBitmapImageRep::alloc(),
                    std::ptr::null_mut(),
                    w,
                    h,
                    8,
                    4,
                    true,
                    false,
                    NSDeviceRGBColorSpace,
                    NSBitmapFormat::AlphaNonpremultiplied,
                    w * 4,
                    32,
                )
            }
            .expect("a bitmap representation");
            // SAFETY: the representation's buffer holds `w * h * 4` bytes, as `data` does.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    representation.bitmapData(),
                    data.len(),
                );
            }
            representation.setSize(size);
            image.addRepresentation(&representation);
        }
        let hotspot = NSPoint::new(
            f64::from(two.hotspot[0]) / 2.0,
            f64::from(two.hotspot[1]) / 2.0,
        );
        NSCursor::initWithImage_hotSpot(NSCursor::alloc(), &image, hotspot)
    }
}

#[cfg(windows)]
mod win {
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::time::Duration;

    use egui_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::{FILETIME, HWND};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
        GetDIBits, GetObjectW, HBITMAP, ReleaseDC,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows_sys::Win32::System::Threading::{GetCurrentThread, GetThreadTimes};
    use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DestroyCursor, GetIconInfo, GetSystemMetrics, ICONINFO, IDC_ARROW, IMAGE_CURSOR,
        LR_DEFAULTSIZE, LoadImageW, SM_CXCURSOR, SM_CYCURSOR, SPI_GETCURSORSHADOW,
        SystemParametersInfoW,
    };

    /// `SetThreadCursorCreationScaling`, Windows 11 24H2's, which windows-sys 0.61.2 lacks:
    /// looked up in user32 at run time, so the example still starts on an older Windows.
    type SetThreadCursorCreationScaling = unsafe extern "system" fn(cursor_dpi: u32) -> u32;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    pub fn hwnd(frame: &eframe::Frame) -> Option<HWND> {
        match frame.window_handle().map(|handle| handle.as_raw()) {
            Ok(RawWindowHandle::Win32(handle)) => Some(handle.hwnd.get() as HWND),
            _ => None,
        }
    }

    pub fn dpi(hwnd: HWND) -> u32 {
        // SAFETY: the window's own handle.
        unsafe { GetDpiForWindow(hwnd) }
    }

    /// Calls `SetThreadCursorCreationScaling(dpi)`, returning what it returned, or `None`
    /// where this Windows has no such function.
    pub fn set_creation_scaling(dpi: u32) -> Option<u32> {
        let user32 = wide("user32.dll");
        // SAFETY: user32 is loaded in every windowed process; the function's type is its
        // documented one.
        unsafe {
            let module = GetModuleHandleW(user32.as_ptr());
            if module.is_null() {
                return None;
            }
            let function =
                GetProcAddress(module, c"SetThreadCursorCreationScaling".as_ptr().cast())?;
            let function = std::mem::transmute::<
                unsafe extern "system" fn() -> isize,
                SetThreadCursorCreationScaling,
            >(function);
            Some(function(dpi))
        }
    }

    pub fn thread_cpu() -> Option<Duration> {
        let mut times = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 4];
        let [created, exited, kernel, user] = &mut times;
        // SAFETY: four writable FILETIMEs and the thread's own pseudo-handle.
        if unsafe { GetThreadTimes(GetCurrentThread(), created, exited, kernel, user) } == 0 {
            return None;
        }
        let ticks = |t: &FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        Some(Duration::from_nanos((ticks(kernel) + ticks(user)) * 100))
    }

    fn registry_dword(subkey: &str, value: &str) -> String {
        let (subkey, value) = (wide(subkey), wide(value));
        let mut data = 0u32;
        let mut size = 4u32;
        // SAFETY: NUL-terminated names and a DWORD's room.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&raw mut data).cast(),
                &mut size,
            )
        };
        if status == 0 {
            data.to_string()
        } else {
            format!("unread (error {status})")
        }
    }

    fn bitmap(handle: HBITMAP) -> BITMAP {
        // SAFETY: an all-zero BITMAP is valid, and GetObjectW writes at most its size.
        unsafe {
            let mut bitmap: BITMAP = std::mem::zeroed();
            GetObjectW(
                handle,
                size_of::<BITMAP>() as i32,
                (&raw mut bitmap).cast::<c_void>(),
            );
            bitmap
        }
    }

    /// The colour bitmap's pixels: how many colours, whether alpha is used, the commonest.
    fn colours(handle: HBITMAP) -> String {
        let BITMAP {
            bmWidth: w,
            bmHeight: h,
            ..
        } = bitmap(handle);
        let mut pixels = vec![0u8; (w * h * 4).max(0) as usize];
        // SAFETY: an all-zero BITMAPINFO is valid; the header asks for top-down 32-bit
        // rows, `w * h * 4` bytes, which `pixels` holds.
        let rows = unsafe {
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let dc = GetDC(std::ptr::null_mut());
            let rows = GetDIBits(
                dc,
                handle,
                0,
                h as u32,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            );
            ReleaseDC(std::ptr::null_mut(), dc);
            rows
        };
        if rows == 0 {
            return "arrow's colour pixels: unread".to_owned();
        }
        let mut counts: HashMap<[u8; 4], u32> = HashMap::new();
        for &[b, g, r, a] in pixels.as_chunks::<4>().0 {
            *counts.entry([r, g, b, a]).or_default() += 1;
        }
        let alpha = counts.keys().any(|c| c[3] != 0);
        let mut common: Vec<_> = counts.iter().collect();
        common.sort_by(|a, b| b.1.cmp(a.1));
        let common: Vec<String> = common
            .iter()
            .take(8)
            .map(|(c, n)| format!("#{:02x}{:02x}{:02x}{:02x}×{n}", c[0], c[1], c[2], c[3]))
            .collect();
        format!(
            "arrow's colour pixels: {} colours, alpha {}, commonest {}",
            counts.len(),
            if alpha { "used" } else { "all zero" },
            common.join(" ")
        )
    }

    pub fn print_values(hwnd: Option<HWND>) {
        let mut lines = Vec::new();
        // SAFETY: the system's arrow, loaded as a copy and destroyed after; its bitmaps
        // are the caller's to delete.
        unsafe {
            let arrow = LoadImageW(
                std::ptr::null_mut(),
                IDC_ARROW,
                IMAGE_CURSOR,
                0,
                0,
                LR_DEFAULTSIZE,
            );
            if arrow.is_null() {
                lines.push(
                    "LoadImageW(IDC_ARROW, IMAGE_CURSOR, 0, 0, LR_DEFAULTSIZE): failed".to_owned(),
                );
            } else {
                let mut info: ICONINFO = std::mem::zeroed();
                if GetIconInfo(arrow, &mut info) != 0 {
                    let mask = bitmap(info.hbmMask);
                    lines.push(format!(
                        "arrow (LoadImageW IDC_ARROW, LR_DEFAULTSIZE): hotspot ({}, {}), mask {}×{} at {} bpp",
                        info.xHotspot, info.yHotspot, mask.bmWidth, mask.bmHeight, mask.bmBitsPixel
                    ));
                    if info.hbmColor.is_null() {
                        lines.push(
                            "arrow hbmColor: none (monochrome: the mask is AND over XOR, twice the height)"
                                .to_owned(),
                        );
                    } else {
                        let colour = bitmap(info.hbmColor);
                        lines.push(format!(
                            "arrow hbmColor: {}×{} at {} bpp",
                            colour.bmWidth, colour.bmHeight, colour.bmBitsPixel
                        ));
                        lines.push(colours(info.hbmColor));
                        DeleteObject(info.hbmColor);
                    }
                    DeleteObject(info.hbmMask);
                } else {
                    lines.push("GetIconInfo(arrow): failed".to_owned());
                }
                DestroyCursor(arrow);
            }
        }
        lines.push(format!(
            "HKCU\\Control Panel\\Cursors\\CursorBaseSize: {}",
            registry_dword("Control Panel\\Cursors", "CursorBaseSize")
        ));
        lines.push(format!(
            "HKCU\\Software\\Microsoft\\Accessibility\\CursorSize: {}",
            registry_dword("Software\\Microsoft\\Accessibility", "CursorSize")
        ));
        let dpi = hwnd.map(dpi);
        lines.push(format!("GetDpiForWindow: {dpi:?}"));
        // SAFETY: plain queries.
        unsafe {
            if let Some(dpi) = dpi {
                lines.push(format!(
                    "GetSystemMetricsForDpi(SM_CXCURSOR, {dpi}): {}, SM_CYCURSOR {}",
                    GetSystemMetricsForDpi(SM_CXCURSOR, dpi),
                    GetSystemMetricsForDpi(SM_CYCURSOR, dpi)
                ));
            }
            lines.push(format!(
                "GetSystemMetrics(SM_CXCURSOR): {}, SM_CYCURSOR {}",
                GetSystemMetrics(SM_CXCURSOR),
                GetSystemMetrics(SM_CYCURSOR)
            ));
            let mut shadow = 0i32;
            let read = SystemParametersInfoW(SPI_GETCURSORSHADOW, 0, (&raw mut shadow).cast(), 0);
            lines.push(if read == 0 {
                "SPI_GETCURSORSHADOW: unread".to_owned()
            } else {
                format!(
                    "SPI_GETCURSORSHADOW: {}",
                    if shadow != 0 { "on" } else { "off" }
                )
            });
        }
        println!("{}", lines.join("\n"));
    }
}
