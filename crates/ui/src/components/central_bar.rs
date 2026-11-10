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
//! values under the cursor. On the right: the zoom, one button showing the scale, which
//! opens darktable's zoom menu (`src/libs/navigation.c`): Small, Fit and Fill, the stops
//! from 50 to 1600%, then Zoom in and Zoom out.

use std::ops::RangeInclusive;

use egui::accesskit::HasPopup;
use egui::os::OperatingSystem;
use egui::{
    Align, FontId, Layout, Rect, Role, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::{IconButton, focus_ring, icon_colour, icon_lit, pull_down};
use crate::i18n::Args;
use crate::popup::{self, Entry, Item};
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

/// The scale the photo is shown at, as darktable's zoom menu names it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom {
    /// Half of Fit.
    Small,
    /// The whole photo in the view.
    Fit,
    /// The view filled, the photo cut where it is longer than the view.
    Fill,
    /// A percentage, 100 being one photo pixel per physical pixel.
    Percent(f64),
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
    /// A scale chosen in the zoom menu.
    Zoom(Zoom),
    /// The zoom menu's Zoom in: the next scale up its list.
    ZoomIn,
    /// The zoom menu's Zoom out: the next scale down its list.
    ZoomOut,
}

pub struct CentralBar<'a> {
    show_selected: Toggle,
    proof_margin: Toggle,
    before_after: Toggle,
    guides: Toggle,
    layer: &'a str,
    layers: bool,
    values: Option<[u8; 3]>,
    status: Option<&'a str>,
    zoom: Zoom,
    zoom_enabled: bool,
    zoomable: bool,
    shortcuts: Vec<(CentralBarAction, &'a str)>,
}

const BUTTON: f32 = 28.0;

/// The zoom menu, a separator where `None`: darktable's list, then the two steps.
const ZOOM_MENU: [Option<CentralBarAction>; 13] = [
    Some(CentralBarAction::Zoom(Zoom::Small)),
    Some(CentralBarAction::Zoom(Zoom::Fit)),
    Some(CentralBarAction::Zoom(Zoom::Fill)),
    None,
    Some(CentralBarAction::Zoom(Zoom::Percent(50.0))),
    Some(CentralBarAction::Zoom(Zoom::Percent(100.0))),
    Some(CentralBarAction::Zoom(Zoom::Percent(200.0))),
    Some(CentralBarAction::Zoom(Zoom::Percent(400.0))),
    Some(CentralBarAction::Zoom(Zoom::Percent(800.0))),
    Some(CentralBarAction::Zoom(Zoom::Percent(1600.0))),
    None,
    Some(CentralBarAction::ZoomIn),
    Some(CentralBarAction::ZoomOut),
];

/// The largest scale in the menu, past which Zoom in has nowhere to go.
const MOST: f64 = 1600.0;

/// The zoom's button, as the wireframe draws it: the scale and the chevron at its end, 8
/// points before the scale, 4 between them and 6 after the chevron, at least 64 wide.
const ZOOM_LEAST: f32 = 64.0;
const ZOOM_START: f32 = 8.0;
const ZOOM_GAP: f32 = 4.0;
const ZOOM_END: f32 = 6.0;
const CHEVRON: f32 = 10.0;

impl<'a> CentralBar<'a> {
    /// `layer` is the translated name of the layer being edited, `zoom` the scale shown.
    pub fn new(layer: &'a str, zoom: Zoom) -> Self {
        CentralBar {
            show_selected: Toggle::default(),
            proof_margin: Toggle::default(),
            before_after: Toggle::default(),
            guides: Toggle::default(),
            layer,
            layers: false,
            values: None,
            status: None,
            zoom,
            zoom_enabled: false,
            zoomable: false,
            shortcuts: Vec::new(),
        }
    }

    /// Whether the zoom's button can be used: while a photo is shown.
    pub fn zoom_enabled(mut self, enabled: bool) -> Self {
        self.zoom_enabled = enabled;
        self
    }

    /// Whether the scale can change. A viewport that only fits cannot zoom, so its menu
    /// offers Fit alone, the rest drawn disabled.
    pub fn zoomable(mut self, zoomable: bool) -> Self {
        self.zoomable = zoomable;
        self
    }

    /// The keys that run one of the zoom menu's entries, in the platform's spelling
    /// (`Ctrl+0`), shown at its end; the app's keymap handles them.
    pub fn shortcut(mut self, entry: CentralBarAction, keys: &'a str) -> Self {
        self.shortcuts.push((entry, keys));
        self
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

    /// A quiet word on what the photo is waiting for, after the layer's buttons, as
    /// Rendering… while a photo opening is slow to go live (decision 77); `None` for none.
    pub fn status(mut self, status: Option<&'a str>) -> Self {
        self.status = status;
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

        // The zoom, on the right.
        let right_start = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    if let Some(chosen) = self.zoom_button(ui) {
                        action = Some(chosen);
                    }
                    ui.min_rect().left()
                },
            )
            .inner;

        // The status, quiet, at the gap's start, never over the photo.
        let mut left_end = left_end;
        if let Some(status) = self.status
            && right_start > left_end
        {
            left_end = status_label(ui, left_end..=right_start, inner, status);
        }
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

    /// The zoom's button: the scale and ⌄, opening the zoom menu under it through the
    /// popup service. Returns the entry chosen.
    fn zoom_button(&self, ui: &mut Ui) -> Option<CentralBarAction> {
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let enabled = self.zoom_enabled;
        let scale = zoom_words(&ctx, CentralBarAction::Zoom(self.zoom));
        let colour = if enabled {
            t.colors.text
        } else {
            t.colors.text_disabled
        };
        let galley =
            ui.painter()
                .layout_no_wrap(scale.clone(), FontId::proportional(t.text.body), colour);
        let width = ZOOM_START + galley.size().x + ZOOM_GAP + CHEVRON + ZOOM_END;
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width.max(ZOOM_LEAST), BUTTON), sense);
        let open = popup::is_open(&ctx, popup::button_menu(response.id));
        if enabled && (response.hovered() || open) {
            ui.painter()
                .rect_filled(rect, t.shape.radius, t.colors.hover);
        }
        // The scale and the chevron at the button's end, so a longer scale grows it
        // towards the bar's middle.
        let chevron = Rect::from_center_size(
            pos2(rect.right() - ZOOM_END - CHEVRON / 2.0, rect.center().y),
            vec2(CHEVRON, CHEVRON),
        );
        let text_at = pos2(
            chevron.left() - ZOOM_GAP - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_at, galley, colour);
        Icon::ChevronDown.paint(
            ui.painter(),
            chevron,
            icon_colour(&t, enabled, icon_lit(&ctx, &response)),
        );
        if response.has_focus() {
            focus_ring(ui.painter(), rect, &t);
        }
        let name = tr(&ctx, "photo-zoom");
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &name));
        ctx.accesskit_node_builder(response.id, |node| {
            node.set_value(scale.clone());
            node.set_has_popup(HasPopup::Menu);
        });
        let later = tr(&ctx, "photo-zoom-later");
        let tip = if self.zoomable {
            let mut args = Args::new();
            // The pointer's modifier as the app spells its shortcuts.
            let mac = ctx.os() == OperatingSystem::Mac;
            args.set("command", if mac { "Cmd" } else { "Ctrl" });
            let exact = self.zoom == Zoom::Percent(100.0);
            let id = if exact {
                "photo-zoom-tip-exact"
            } else {
                "photo-zoom-tip"
            };
            translator(&ctx).text_with(id, Some(&args))
        } else {
            later.clone()
        };
        let response = response.on_hover_text(tip);
        let picked = pull_down(ui, &response, &name, || {
            ZOOM_MENU
                .iter()
                .map(|entry| match entry {
                    None => Entry::Separator,
                    Some(entry) => {
                        let can = self.can_choose(*entry);
                        let mut item =
                            Item::new(zoom_id(*entry), zoom_words(&ctx, *entry)).enabled(can);
                        // The scales are a set, the one shown checked; an in-between
                        // scale checks none. Zoom in and out are commands.
                        if let CentralBarAction::Zoom(zoom) = entry {
                            item = item.checked(*zoom == self.zoom);
                        }
                        if let Some((_, keys)) = self.shortcuts.iter().find(|(e, _)| e == entry) {
                            item = item.shortcut(*keys);
                        }
                        if !can && !self.zoomable {
                            item = item.tip(later.clone());
                        }
                        item.into()
                    }
                })
                .collect()
        });
        picked.and_then(|id| {
            ZOOM_MENU
                .into_iter()
                .flatten()
                .find(|entry| zoom_id(*entry) == id)
        })
    }

    /// Whether the zoom menu's `entry` can be chosen: Fit always, the rest only while the
    /// scale can change, Zoom in short of the largest scale and Zoom out above Small.
    fn can_choose(&self, entry: CentralBarAction) -> bool {
        match entry {
            CentralBarAction::Zoom(Zoom::Fit) => true,
            _ if !self.zoomable => false,
            CentralBarAction::ZoomIn => !matches!(self.zoom, Zoom::Percent(p) if p >= MOST),
            CentralBarAction::ZoomOut => self.zoom != Zoom::Small,
            _ => true,
        }
    }
}

/// A zoom menu entry's id in the popup's request; the bar's other actions have none.
fn zoom_id(entry: CentralBarAction) -> String {
    match entry {
        CentralBarAction::Zoom(Zoom::Small) => "zoom-small".to_owned(),
        CentralBarAction::Zoom(Zoom::Fit) => "zoom-fit".to_owned(),
        CentralBarAction::Zoom(Zoom::Fill) => "zoom-fill".to_owned(),
        CentralBarAction::Zoom(Zoom::Percent(p)) => format!("zoom-{p}"),
        CentralBarAction::ZoomIn => "zoom-in".to_owned(),
        CentralBarAction::ZoomOut => "zoom-out".to_owned(),
        _ => String::new(),
    }
}

/// A zoom menu entry's words, translated: a scale as the button shows it, a percentage
/// rounded to a whole number. The bar's other actions have none.
fn zoom_words(ctx: &egui::Context, entry: CentralBarAction) -> String {
    let id = match entry {
        CentralBarAction::Zoom(Zoom::Percent(p)) => {
            return format!("{}%", translator(ctx).format_number(p, 0, false));
        }
        CentralBarAction::Zoom(Zoom::Small) => "photo-zoom-small",
        CentralBarAction::Zoom(Zoom::Fit) => "photo-zoom-fit",
        CentralBarAction::Zoom(Zoom::Fill) => "photo-zoom-fill",
        CentralBarAction::ZoomIn => "photo-zoom-in",
        CentralBarAction::ZoomOut => "photo-zoom-out",
        _ => return String::new(),
    };
    tr(ctx, id)
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
/// Draws `status` quietly from `span`'s start, in `inner`'s height, when it fits; where it
/// ends, or `span`'s start where it does not fit.
fn status_label(ui: &mut Ui, span: RangeInclusive<f32>, inner: Rect, status: &str) -> f32 {
    let t = theme(ui.ctx());
    let font = FontId::proportional(t.text.small);
    let galley = (ui.painter()).layout_no_wrap(status.to_owned(), font, t.colors.text_disabled);
    let (start, end) = (*span.start() + t.spacing.l, *span.end() - t.spacing.l);
    if start + galley.size().x > end {
        return *span.start();
    }
    let at = pos2(start, inner.center().y - galley.size().y / 2.0);
    let rect = Rect::from_min_size(at, galley.size());
    ui.painter().galley(at, galley, t.colors.text_disabled);
    let response = ui.interact(rect, ui.scope_id().with("status"), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, status));
    rect.right()
}

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
