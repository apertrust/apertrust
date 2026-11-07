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

//! Edit's thin bar over the photo, and the caption under it (`docs/gui.md`, The shell).
//!
//! On the left: the view toggles (every selected photo, the proof margin, before and
//! after, the guides), then the layer being edited and + for a new one. In the middle: the
//! values under the cursor. On the right: the scale, Fit or a percentage, with − a slider
//! and +. The slider is logarithmic, from Fit to 1600%, and the buttons step through 50,
//! 100, 200, 400, 800 and 1600%.

use egui::accesskit::{Action, ActionData};
use egui::{
    Align, Color32, FontId, Layout, Rect, Role, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo,
    pos2, vec2,
};

use super::{IconButton, focus_ring, icon_colour, icon_lit};
use crate::i18n::Args;
use crate::{Icon, theme, tr, translator};

/// A view toggle's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Toggle {
    pub on: bool,
    pub enabled: bool,
}

impl Toggle {
    pub fn new(on: bool, enabled: bool) -> Self {
        Toggle { on, enabled }
    }
}

/// The scale the photo is shown at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    /// The percentage that fits the photo in the view.
    pub fit: f64,
    /// `None` at Fit; otherwise the percentage, 100 being one pixel per physical pixel.
    pub scale: Option<f64>,
    /// Whether the scale can change: a viewport that only fits cannot zoom.
    pub enabled: bool,
}

/// What the user did to the central bar this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CentralBarAction {
    ShowSelected,
    ProofMargin,
    BeforeAfter,
    Guides,
    /// Open the layer menu.
    Layers,
    NewLayer,
    /// A new scale: `None` is Fit.
    Zoom(Option<f64>),
}

pub struct CentralBar<'a> {
    show_selected: Toggle,
    proof_margin: Toggle,
    before_after: Toggle,
    guides: Toggle,
    layer: &'a str,
    layers: bool,
    values: Option<[u8; 3]>,
    zoom: Zoom,
}

const STOPS: [f64; 6] = [50.0, 100.0, 200.0, 400.0, 800.0, 1600.0];
const MOST: f64 = 1600.0;
const BUTTON: f32 = 28.0;

impl<'a> CentralBar<'a> {
    /// `layer` is the translated name of the layer being edited.
    pub fn new(layer: &'a str, zoom: Zoom) -> Self {
        CentralBar {
            show_selected: Toggle::default(),
            proof_margin: Toggle::default(),
            before_after: Toggle::default(),
            guides: Toggle::default(),
            layer,
            layers: false,
            values: None,
            zoom,
        }
    }

    pub fn toggles(
        mut self,
        show_selected: Toggle,
        proof_margin: Toggle,
        before_after: Toggle,
        guides: Toggle,
    ) -> Self {
        self.show_selected = show_selected;
        self.proof_margin = proof_margin;
        self.before_after = before_after;
        self.guides = guides;
        self
    }

    /// Whether the layer menu and + can be used.
    pub fn layers(mut self, enabled: bool) -> Self {
        self.layers = enabled;
        self
    }

    /// The sRGB values under the cursor, while it is over the photo.
    pub fn values(mut self, rgb: Option<[u8; 3]>) -> Self {
        self.values = rgb;
        self
    }

    /// The height [`Self::show`] takes, for laying out what goes under the bar before it.
    pub fn height(ctx: &egui::Context) -> f32 {
        BUTTON + 2.0 * theme(ctx).spacing.xs
    }

    pub fn show(self, ui: &mut Ui) -> Option<CentralBarAction> {
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let (bar, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), Self::height(&ctx)),
            Sense::hover(),
        );
        let bar_name = tr(&ctx, "central-bar");
        response.widget_info(|| WidgetInfo::labeled(Role::Toolbar, true, &bar_name));
        ui.painter().rect_filled(bar, 0.0, t.colors.surface);
        ui.painter().hline(
            bar.x_range(),
            bar.bottom() - t.shape.separator_width / 2.0,
            Stroke::new(t.shape.separator_width, t.colors.separator),
        );
        let inner = bar.shrink2(vec2(t.spacing.m, 0.0));
        let mut action = None;

        // The toggles, the layer and +.
        let left_end = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = t.spacing.xxs;
                    for (icon, id, toggle, done) in [
                        (
                            Icon::PhotosSelected,
                            "photo-show-selected",
                            self.show_selected,
                            CentralBarAction::ShowSelected,
                        ),
                        (
                            Icon::ProofMargin,
                            "photo-proof-margin",
                            self.proof_margin,
                            CentralBarAction::ProofMargin,
                        ),
                        (
                            Icon::BeforeAfter,
                            "photo-before-after",
                            self.before_after,
                            CentralBarAction::BeforeAfter,
                        ),
                        (
                            Icon::Grid,
                            "photo-guides",
                            self.guides,
                            CentralBarAction::Guides,
                        ),
                    ] {
                        if IconButton::new(icon, &tr(&ctx, id))
                            .toggled(toggle.on)
                            .enabled(toggle.enabled)
                            .size(BUTTON)
                            .show(ui)
                            .clicked()
                        {
                            action = Some(done);
                        }
                    }
                    ui.add_space(t.spacing.m);
                    let mut args = Args::new();
                    args.set("layer", self.layer);
                    let layer_name = translator(&ctx).text_with("photo-layer", Some(&args));
                    if layer_button(ui, self.layer, &layer_name, self.layers) {
                        action = Some(CentralBarAction::Layers);
                    }
                    if IconButton::new(Icon::Plus, &tr(&ctx, "photo-new-layer"))
                        .enabled(self.layers)
                        .size(BUTTON)
                        .show(ui)
                        .clicked()
                    {
                        action = Some(CentralBarAction::NewLayer);
                    }
                    ui.min_rect().right()
                },
            )
            .inner;

        // The scale, on the right.
        let right_start = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = t.spacing.xxs;
                    let z = self.zoom;
                    let current = z.scale.unwrap_or(z.fit);
                    if IconButton::new(Icon::ZoomIn, &tr(&ctx, "photo-zoom-in"))
                        .enabled(z.enabled && current < MOST)
                        .size(BUTTON)
                        .show(ui)
                        .clicked()
                    {
                        let next = STOPS
                            .iter()
                            .copied()
                            .find(|&s| s > current + 1e-6 && s > z.fit);
                        action = Some(CentralBarAction::Zoom(next.or(Some(MOST))));
                    }
                    if let Some(scale) = zoom_slider(ui, z) {
                        action = Some(CentralBarAction::Zoom(scale));
                    }
                    if IconButton::new(Icon::ZoomOut, &tr(&ctx, "photo-zoom-out"))
                        .enabled(z.enabled && z.scale.is_some())
                        .size(BUTTON)
                        .show(ui)
                        .clicked()
                    {
                        let previous = STOPS
                            .iter()
                            .rev()
                            .copied()
                            .find(|&s| s < current - 1e-6 && s > z.fit);
                        action = Some(CentralBarAction::Zoom(previous));
                    }
                    let text = match z.scale {
                        None => tr(&ctx, "photo-zoom-fit"),
                        Some(s) => format!("{}%", translator(&ctx).format_number(s, 0, false)),
                    };
                    let galley = ui.painter().layout_no_wrap(
                        text,
                        FontId::proportional(t.text.body),
                        t.colors.text,
                    );
                    let (r, _) = ui.allocate_exact_size(
                        vec2(galley.size().x.max(52.0), BUTTON),
                        Sense::hover(),
                    );
                    ui.painter().galley(
                        pos2(
                            r.right() - galley.size().x,
                            r.center().y - galley.size().y / 2.0,
                        ),
                        galley,
                        t.colors.text,
                    );
                    ui.min_rect().left()
                },
            )
            .inner;

        // The values under the cursor, in the gap between the two ends when they fit.
        if let Some(rgb) = self.values
            && right_start > left_end
        {
            values(
                ui,
                Rect::from_x_y_ranges(left_end..=right_start, inner.y_range()),
                rgb,
            );
        }
        action
    }
}

/// The layer menu's button: the layer's name and ⌄.
fn layer_button(ui: &mut Ui, layer: &str, name: &str, enabled: bool) -> bool {
    let t = theme(ui.ctx());
    let colour = if enabled {
        t.colors.text
    } else {
        t.colors.text_disabled
    };
    let galley =
        ui.painter()
            .layout_no_wrap(layer.to_owned(), FontId::proportional(t.text.body), colour);
    let size = vec2(galley.size().x + 10.0 + 3.0 * t.spacing.s, BUTTON);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if enabled && response.hovered() {
        ui.painter()
            .rect_filled(rect, t.shape.radius, t.colors.hover);
    }
    let text_y = rect.center().y - galley.size().y / 2.0;
    let text_w = galley.size().x;
    ui.painter()
        .galley(pos2(rect.left() + t.spacing.s, text_y), galley, colour);
    Icon::ChevronDown.paint(
        ui.painter(),
        Rect::from_center_size(
            pos2(
                rect.left() + 2.0 * t.spacing.s + text_w + 5.0,
                rect.center().y,
            ),
            vec2(10.0, 10.0),
        ),
        // The chevron is a step quieter than the layer's name.
        icon_colour(&t, enabled, icon_lit(ui.ctx(), &response)),
    );
    if response.has_focus() {
        focus_ring(ui.painter(), rect, &t);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, name));
    response.on_hover_text(name).clicked()
}

/// Red, green, blue and grey under the cursor, 0 to 255, centred in `bar`; each value in
/// its channel's colour, with no letters and no swatch; nothing when they don't fit.
fn values(ui: &mut Ui, bar: Rect, [r, g, b]: [u8; 3]) {
    let t = theme(ui.ctx());
    let tr_ = translator(ui.ctx());
    // Rec. 709 luminance of the 8-bit values, as the scopes' luminance line.
    let grey = (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)).round();
    let c = &t.colors;
    let parts = [
        (f64::from(r), c.channel_red),
        (f64::from(g), c.channel_green),
        (f64::from(b), c.channel_blue),
        (grey, c.text),
    ];
    let font = FontId::proportional(t.text.small);
    let painter = ui.painter();
    let galleys: Vec<_> = parts
        .iter()
        .map(|&(value, colour)| {
            let text = tr_.format_number(value, 0, false);
            (painter.layout_no_wrap(text, font.clone(), colour), colour)
        })
        .collect();
    let gap = t.spacing.l;
    let width =
        galleys.iter().map(|(v, _)| v.size().x).sum::<f32>() + gap * (galleys.len() as f32 - 1.0);
    if width > bar.width() - 2.0 * gap {
        return;
    }
    let mut x = bar.center().x - width / 2.0;
    let y = bar.center().y;
    for (v, colour) in galleys {
        let w = v.size().x;
        painter.galley(pos2(x, y - v.size().y / 2.0), v, colour);
        x += w + gap;
    }
    let label = tr(ui.ctx(), "photo-values");
    let mut args = crate::i18n::Args::new();
    args.set("red", tr_.format_number(f64::from(r), 0, false));
    args.set("green", tr_.format_number(f64::from(g), 0, false));
    args.set("blue", tr_.format_number(f64::from(b), 0, false));
    args.set("grey", tr_.format_number(grey, 0, false));
    let value = tr_.text_with("photo-values-read", Some(&args));
    let (_, response) = ui.allocate_exact_size(vec2(0.0, 0.0), Sense::hover());
    // A status, not a label: egui makes a label's name its value, which would lose it.
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::Status, true, &label);
        info.current_text_value = Some(value.clone());
        info
    });
}

/// The zoom slider's size, its track's height and its handle's, as the wireframe draws
/// them: a rounded track with a pill for a handle, not a module slider's triangle.
const ZOOM_WIDTH: f32 = 130.0;
const ZOOM_TRACK: f32 = 8.0;
const ZOOM_HANDLE: Vec2 = vec2(24.0, 10.0);

/// A small slider for the scale, logarithmic from Fit to 1600%.
fn zoom_slider(ui: &mut Ui, z: Zoom) -> Option<Option<f64>> {
    let t = theme(ui.ctx());
    let span = (MOST / z.fit.max(1.0)).ln().max(f64::EPSILON);
    let fraction =
        |scale: Option<f64>| scale.map_or(0.0, |s| ((s / z.fit).ln() / span).clamp(0.0, 1.0));
    let scale_at = |f: f64| {
        let s = z.fit * (f * span).exp();
        // Near Fit snaps to Fit.
        (f > 0.02).then_some(s.round())
    };
    let sense = if z.enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(ZOOM_WIDTH, BUTTON), sense);
    // Where the handle's centre runs, so the pill stays over the track at both ends.
    let track = rect.shrink2(vec2(ZOOM_HANDLE.x / 2.0, 0.0));
    let mut chosen = None;
    if z.enabled
        && let Some(p) = response.interact_pointer_pos()
        && response.is_pointer_button_down_on()
    {
        let f = f64::from(((p.x - track.left()) / track.width()).clamp(0.0, 1.0));
        let scale = scale_at(f);
        if scale != z.scale {
            chosen = Some(scale);
        }
    }
    let shown = chosen.unwrap_or(z.scale);
    let id = response.id;
    let requested = ui.input(|i| {
        i.accesskit_action_requests(id, Action::SetValue)
            .filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) => Some(v),
                _ => None,
            })
            .last()
    });
    if z.enabled
        && let Some(v) = requested
    {
        chosen = Some((v > z.fit + 0.5).then_some(v.min(MOST)));
    }
    let painter = ui.painter();
    let fade = |c: Color32| if z.enabled { c } else { c.gamma_multiply(0.5) };
    let line = Rect::from_center_size(rect.center(), vec2(rect.width(), ZOOM_TRACK));
    painter.rect_filled(line, ZOOM_TRACK / 2.0, fade(t.colors.slider_track));
    let x = track.left() + fraction(shown) as f32 * track.width();
    let pill = Rect::from_center_size(pos2(x, line.center().y), ZOOM_HANDLE);
    let lit = z.enabled && (response.hovered() || response.dragged());
    let fill = if lit {
        t.colors.slider_handle_hover
    } else {
        t.colors.slider_handle
    };
    painter.rect_filled(pill, ZOOM_HANDLE.y / 2.0, fade(fill));
    if response.has_focus() {
        focus_ring(painter, line.expand2(vec2(3.0, 7.0)), &t);
    }
    let name = tr(ui.ctx(), "photo-zoom");
    let value = shown.unwrap_or(z.fit);
    response.widget_info(|| WidgetInfo::slider(z.enabled, value, &name));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_min_numeric_value(z.fit);
        node.set_max_numeric_value(MOST);
        node.add_action(Action::SetValue);
    });
    chosen
}
