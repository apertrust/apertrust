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

//! The histogram and the other scopes: the graphs of the Scopes section, one at a time,
//! chosen with buttons over the graph rather than tabs (`docs/gui.md`, The scopes).
//!
//! It draws what the caller measured and knows nothing of the engine: pixel counts in
//! small arrays, the histogram's bins for red, green, blue and luminance, a waveform's
//! columns of levels, a vectorscope's grid, and how much of the photo is clipped.
//!
//! - **The histogram:** the colour channels overlaid, mixing as light on a dark theme and
//!   as ink on a light one, luminance as a line over them, on a linear or a logarithmic
//!   scale; a mark at either end while shadows or highlights clip.
//! - **The waveform:** brightness across the photo, left to right as in the photo, dark at
//!   the bottom. **The RGB parade:** the red, green and blue waveforms side by side.
//!   **The vectorscope:** colour by hue, the angle, and saturation, the distance from the
//!   centre, its cells drawn as darktable draws them.
//!
//! Its controls show only while the pointer is over it or one of them has focus: the
//! graphs at the top left; the scale and the channels at the top right; the clipping
//! overlays' toggles in the bottom corners. They stay in the focus order and for screen
//! readers while hidden, so Tab reaches them and shows them. What the user chooses comes
//! back as a [`HistogramAction`], applied to the app's [`ScopeView`]; the overlays on the
//! photo are the app's.

use std::hash::{Hash, Hasher};

use apertrust_base::settings::Choice;
use egui::epaint::Mesh;
use egui::{
    Align2, AsIdSalt, Color32, ColorImage, FontId, Id, IdSalt, Painter, Rect, Response, Rgba, Role,
    Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, UiBuilder, WidgetInfo, pos2,
    vec2,
};

use super::{focus_ring, icon_colour, icon_lit};
use crate::i18n::Args;
use crate::{Icon, Theme, Translator, theme, translator};

/// The graph's height: the wireframe's.
const HEIGHT: f32 = 136.0;
/// How strongly each colour channel inks the histogram's area, from 0 to 1: the wireframe's
/// half. A waveform's cell inks as strongly as it is dense, and the vectorscope's as
/// darktable draws it ([`vectorscope_strength`]).
const INK: f32 = 0.5;
/// How far darktable lightens a vectorscope cell towards white, times the cell's strength:
/// white at 0.55 under `CAIRO_OPERATOR_HARD_LIGHT` (darktable,
/// `src/libs/scopes/vectorscope.c`, `_vec_draw`).
const LIGHTEN: f32 = 0.55;
/// The marks at the graph's ends that show clipping, and the luminance line.
const MARK: f32 = 3.0;
const LINE: f32 = 1.5;

/// The channels' colours: each its own primary at 0.80 in linear light and the other two at
/// 0.08. Being alike but for their order, all three together go neutral both ways they
/// mix, added as light (0.96 in each primary) and multiplied as ink at any strength, so
/// where they overlap a neutral photo reads grey. They are data, as a hue track's are
/// (`ui-conventions`), so no theme changes them; luminance takes the theme's text colour,
/// and the clipping marks its overlay colours.
const RED: Color32 = Color32::from_rgb(0xE7, 0x50, 0x50);
const GREEN: Color32 = Color32::from_rgb(0x50, 0xE7, 0x50);
const BLUE: Color32 = Color32::from_rgb(0x50, 0x50, 0xE7);

/// The graphs, in the order their buttons show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Scope {
    #[default]
    Histogram,
    Waveform,
    /// The red, green and blue waveforms side by side.
    Parade,
    Vectorscope,
}

impl Scope {
    pub const ALL: [Scope; 4] = [
        Scope::Histogram,
        Scope::Waveform,
        Scope::Parade,
        Scope::Vectorscope,
    ];

    /// The graph's name: its button's, and the graph's for screen readers.
    fn message(self) -> &'static str {
        match self {
            Scope::Histogram => "scope-histogram",
            Scope::Waveform => "scope-waveform",
            Scope::Parade => "scope-parade",
            Scope::Vectorscope => "scope-vectorscope",
        }
    }

    /// What the graph shows, in its tooltip.
    fn tip(self) -> &'static str {
        match self {
            Scope::Histogram => "scope-histogram-tip",
            Scope::Waveform => "scope-waveform-tip",
            Scope::Parade => "scope-parade-tip",
            Scope::Vectorscope => "scope-vectorscope-tip",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Scope::Histogram => Icon::Histogram,
            Scope::Waveform => Icon::Waveform,
            Scope::Parade => Icon::Parade,
            Scope::Vectorscope => Icon::Vectorscope,
        }
    }

    /// The channels it can show.
    fn channels(self) -> &'static [Channel] {
        match self {
            Scope::Histogram | Scope::Waveform => &Channel::ALL,
            Scope::Parade => &Channel::ALL[..3],
            Scope::Vectorscope => &[],
        }
    }
}

/// The scopes' graph as settings keep it (decision 43).
impl Choice for Scope {
    const OPTIONS: &'static [(Scope, &'static str)] = &[
        (Scope::Histogram, "histogram"),
        (Scope::Waveform, "waveform"),
        (Scope::Parade, "parade"),
        (Scope::Vectorscope, "vectorscope"),
    ];
}

/// The histogram's vertical scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Scale {
    #[default]
    Linear,
    /// The logarithm of each count, so a few pixels still show.
    Logarithmic,
}

/// The scopes' scale as settings keep it (decision 43).
impl Choice for Scale {
    const OPTIONS: &'static [(Scale, &'static str)] = &[
        (Scale::Linear, "linear"),
        (Scale::Logarithmic, "logarithmic"),
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    Red,
    Green,
    Blue,
    Luminance,
}

impl Channel {
    pub const ALL: [Channel; 4] = [
        Channel::Red,
        Channel::Green,
        Channel::Blue,
        Channel::Luminance,
    ];

    fn message(self) -> &'static str {
        match self {
            Channel::Red => "scope-red",
            Channel::Green => "scope-green",
            Channel::Blue => "scope-blue",
            Channel::Luminance => "scope-luminance",
        }
    }

    fn colour(self, t: &Theme) -> Color32 {
        match self {
            Channel::Red => RED,
            Channel::Green => GREEN,
            Channel::Blue => BLUE,
            Channel::Luminance => t.colors.text,
        }
    }
}

/// The scopes' channels as settings keep them (decision 43).
impl Choice for Channel {
    const OPTIONS: &'static [(Channel, &'static str)] = &[
        (Channel::Red, "red"),
        (Channel::Green, "green"),
        (Channel::Blue, "blue"),
        (Channel::Luminance, "luminance"),
    ];
}

/// What the scopes show and which clipping overlays the photo shows: the app's state,
/// which persists in its settings, never the engine's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeView {
    pub scope: Scope,
    pub scale: Scale,
    /// Whether each channel is shown, in [`Channel::ALL`]'s order.
    pub channels: [bool; 4],
    pub shadow_clipping: bool,
    pub highlight_clipping: bool,
}

impl Default for ScopeView {
    /// The histogram, linear, red, green and blue; no overlays.
    fn default() -> Self {
        ScopeView {
            scope: Scope::Histogram,
            scale: Scale::Linear,
            channels: [true, true, true, false],
            shadow_clipping: false,
            highlight_clipping: false,
        }
    }
}

impl ScopeView {
    pub fn shows(&self, channel: Channel) -> bool {
        self.channels[channel as usize]
    }

    /// Applies what the scopes reported.
    pub fn apply(&mut self, action: HistogramAction) {
        match action {
            HistogramAction::Scope(scope) => self.scope = scope,
            HistogramAction::Scale(scale) => self.scale = scale,
            HistogramAction::Channel(channel, on) => self.channels[channel as usize] = on,
            HistogramAction::ShadowClipping(on) => self.shadow_clipping = on,
            HistogramAction::HighlightClipping(on) => self.highlight_clipping = on,
        }
    }
}

/// What the user did to the scopes this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistogramAction {
    Scope(Scope),
    Scale(Scale),
    /// A channel shown, or hidden.
    Channel(Channel, bool),
    /// The photo's overlay of clipped shadows on or off; the overlay is the app's.
    ShadowClipping(bool),
    HighlightClipping(bool),
}

/// The histogram: pixel counts in bins evenly spaced over the shown values, black first, for
/// each channel. A channel the caller has none for is an empty slice.
#[derive(Clone, Copy, Debug, Default)]
pub struct HistogramBins<'a> {
    pub red: &'a [f32],
    pub green: &'a [f32],
    pub blue: &'a [f32],
    pub luminance: &'a [f32],
}

impl HistogramBins<'_> {
    fn channel(&self, channel: Channel) -> &[f32] {
        match channel {
            Channel::Red => self.red,
            Channel::Green => self.green,
            Channel::Blue => self.blue,
            Channel::Luminance => self.luminance,
        }
    }
}

/// The waveform: pixel counts by column of the photo and by level, `levels` counts for each
/// of `columns`, column after column from the photo's left, the darkest level first. A
/// channel the caller has none for is an empty slice.
#[derive(Clone, Copy, Debug, Default)]
pub struct WaveformBins<'a> {
    pub columns: usize,
    pub levels: usize,
    pub red: &'a [f32],
    pub green: &'a [f32],
    pub blue: &'a [f32],
    pub luminance: &'a [f32],
}

impl WaveformBins<'_> {
    /// The channel's counts, if it has `columns` × `levels` of them.
    fn channel(&self, channel: Channel) -> Option<&[f32]> {
        let counts = match channel {
            Channel::Red => self.red,
            Channel::Green => self.green,
            Channel::Blue => self.blue,
            Channel::Luminance => self.luminance,
        };
        (self.columns > 0 && self.levels > 0 && counts.len() == self.columns * self.levels)
            .then_some(counts)
    }
}

/// The vectorscope: counts on a `size` × `size` grid over a chroma plane, row after row
/// from the top, each sample the mean of 2 × 2 pixels, as darktable samples. Its centre is
/// neutral; the angle is hue and the distance saturation, out to the most saturated colour
/// the caller measures at the grid's edge. `colours`, when it has a colour for each cell,
/// colours the counts, as darktable's plane does; without it they take the text's. Each
/// cell is drawn as darktable draws it: lightened towards white on a dark ground, and on a
/// light one the mirror, darkened towards black (`docs/gui.md`, The scopes).
#[derive(Clone, Copy, Debug, Default)]
pub struct VectorscopeBins<'a> {
    pub size: usize,
    pub counts: &'a [f32],
    pub colours: &'a [Color32],
}

/// How much of the photo clips, each a share of its pixels from 0 to 1: at black, and at
/// white.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Clipped {
    pub shadows: f32,
    pub highlights: f32,
}

pub struct Histogram<'a> {
    view: ScopeView,
    histogram: HistogramBins<'a>,
    waveform: WaveformBins<'a>,
    vectorscope: VectorscopeBins<'a>,
    clipped: Option<Clipped>,
    /// Which graphs the app can measure, in [`Scope::ALL`]'s order.
    available: [bool; 4],
    overlays: bool,
    always: bool,
    height: f32,
    salt: IdSalt,
}

impl<'a> Histogram<'a> {
    /// The scopes as `view` has them. The app measures only the graph shown, so a graph
    /// with no data yet shows that nothing is measured, and its button stays live unless
    /// the app cannot measure it ([`Histogram::available`]).
    pub fn new(view: ScopeView) -> Self {
        Histogram {
            view,
            histogram: HistogramBins::default(),
            waveform: WaveformBins::default(),
            vectorscope: VectorscopeBins::default(),
            clipped: None,
            available: [true; 4],
            overlays: false,
            always: false,
            height: HEIGHT,
            salt: IdSalt::new("histogram"),
        }
    }

    /// The graphs the app can measure: the others' buttons are drawn disabled. All four
    /// unless set.
    pub fn available(mut self, scopes: &[Scope]) -> Self {
        self.available = Scope::ALL.map(|scope| scopes.contains(&scope));
        self
    }

    /// Whether the app draws the clipping overlays on the photo: until it does, their
    /// toggles are drawn disabled, as an action with nothing behind it is. Off unless set.
    pub fn overlays(mut self, enabled: bool) -> Self {
        self.overlays = enabled;
        self
    }

    pub fn histogram(mut self, bins: HistogramBins<'a>) -> Self {
        self.histogram = bins;
        self
    }

    /// The waveform, which the RGB parade draws too.
    pub fn waveform(mut self, bins: WaveformBins<'a>) -> Self {
        self.waveform = bins;
        self
    }

    pub fn vectorscope(mut self, bins: VectorscopeBins<'a>) -> Self {
        self.vectorscope = bins;
        self
    }

    /// How much clips; until it is given, nothing is measured.
    pub fn clipped(mut self, clipped: Clipped) -> Self {
        self.clipped = Some(clipped);
        self
    }

    /// Shows the controls at rest too, as the gallery draws the scopes in use.
    pub fn show_controls(mut self, always: bool) -> Self {
        self.always = always;
        self
    }

    /// The graph's height, in points; the wireframe's 136 unless set.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Tells apart several scopes in one place.
    pub fn id_salt(mut self, salt: impl AsIdSalt) -> Self {
        self.salt = IdSalt::new(salt);
        self
    }

    /// Whether the caller gave `scope` anything to draw.
    fn has(&self, scope: Scope) -> bool {
        match scope {
            Scope::Histogram => Channel::ALL
                .iter()
                .any(|&c| !self.histogram.channel(c).is_empty()),
            Scope::Waveform | Scope::Parade => scope
                .channels()
                .iter()
                .any(|&c| self.waveform.channel(c).is_some()),
            Scope::Vectorscope => {
                let v = &self.vectorscope;
                v.size > 0 && v.counts.len() == v.size * v.size
            }
        }
    }

    /// Whether `channel` can show in `scope`: always while nothing is measured, since what
    /// comes may hold it, and then only if it does.
    fn has_channel(&self, scope: Scope, channel: Channel) -> bool {
        if !self.has(scope) {
            return true;
        }
        match scope {
            Scope::Histogram => !self.histogram.channel(channel).is_empty(),
            Scope::Waveform | Scope::Parade => self.waveform.channel(channel).is_some(),
            Scope::Vectorscope => false,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Option<HistogramAction> {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let id = ui.make_persistent_id(self.salt);
        let view = self.view;
        let (frame, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());

        // The graph, for screen readers an image: its name, and what clips.
        let graph = ui.interact(frame, id, Sense::hover());
        let name = tr.text(view.scope.message());
        graph.widget_info(|| WidgetInfo::labeled(Role::Image, true, &name));
        let measured = self.has(view.scope);
        let description = if measured {
            self.clipped.map(|c| clipping(&tr, c))
        } else {
            Some(tr.text("scope-empty"))
        };
        if let Some(description) = description.clone() {
            ui.ctx()
                .accesskit_node_builder(id, |node| node.set_description(description));
        }

        // The controls, before anything is drawn, so a focused one shows them all.
        let mut controls = Vec::new();
        let side = t.sizes.control;
        let gap = t.spacing.xxs;
        let inset = t.spacing.xs;
        let row = |n: usize| n as f32 * (side + gap) - gap;
        let graphs = Rect::from_min_size(frame.min + vec2(inset, inset), vec2(row(4), side));
        let group = ui.interact(graphs, id.with("graphs"), Sense::hover());
        let group_name = tr.text("scope-graphs");
        group.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, true, &group_name));
        let graphs_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(graphs)
                .accessibility_parent(group.id),
        );
        for (i, scope) in Scope::ALL.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                graphs.min + vec2(i as f32 * (side + gap), 0.0),
                vec2(side, side),
            );
            let on = scope == view.scope;
            controls.push(Control::new(
                &graphs_ui,
                rect,
                id.with(("graph", i)),
                Role::RadioButton,
                tr.text(scope.message()),
                Look::Icon(scope.icon()),
                (on, self.available[i]),
                (!on).then_some(HistogramAction::Scope(scope)),
            ));
        }

        // The scale and the channels, at the top right.
        let log = view.scale == Scale::Logarithmic;
        let mut options: Vec<(Id, Look, String, bool, bool, HistogramAction)> = Vec::new();
        if view.scope == Scope::Histogram {
            options.push((
                id.with("scale"),
                Look::Icon(Icon::LogScale),
                tr.text("scope-log-scale"),
                log,
                true,
                HistogramAction::Scale(if log {
                    Scale::Linear
                } else {
                    Scale::Logarithmic
                }),
            ));
        }
        for &channel in view.scope.channels() {
            let on = view.shows(channel);
            options.push((
                id.with(channel.message()),
                Look::Swatch(channel.colour(&t)),
                tr.text(channel.message()),
                on,
                self.has_channel(view.scope, channel),
                HistogramAction::Channel(channel, !on),
            ));
        }
        let mut x = frame.right() - inset - row(options.len());
        for (option, look, name, on, enabled, action) in options {
            let rect = Rect::from_min_size(pos2(x, frame.top() + inset), vec2(side, side));
            x += side + gap;
            controls.push(Control::new(
                ui,
                rect,
                option,
                Role::Button,
                name,
                look,
                (on, enabled),
                Some(action),
            ));
        }

        // The clipping overlays' toggles, in the bottom corners.
        let small = side - 2.0 * gap;
        for (corner, icon, message, on, action) in [
            (
                frame.left_bottom() + vec2(inset, -inset - small),
                Icon::ClipShadows,
                "scope-show-shadow-clipping",
                view.shadow_clipping,
                HistogramAction::ShadowClipping(!view.shadow_clipping),
            ),
            (
                frame.right_bottom() + vec2(-inset - small, -inset - small),
                Icon::ClipHighlights,
                "scope-show-highlight-clipping",
                view.highlight_clipping,
                HistogramAction::HighlightClipping(!view.highlight_clipping),
            ),
        ] {
            controls.push(Control::new(
                ui,
                Rect::from_min_size(corner, vec2(small, small)),
                id.with(message),
                Role::Button,
                tr.text(message),
                Look::Icon(icon),
                (on, self.overlays),
                Some(action),
            ));
        }

        let shown = self.always
            || ui.rect_contains_pointer(frame)
            || controls.iter().any(|c| c.response.has_focus());

        // Drawing.
        if ui.is_rect_visible(frame) {
            let painter = ui.painter();
            painter.rect(
                frame,
                t.shape.radius_small,
                t.colors.sunken,
                Stroke::new(t.shape.hairline, t.colors.divider),
                StrokeKind::Inside,
            );
            let inside = painter.with_clip_rect(frame.intersect(painter.clip_rect()));
            let plot = Rect::from_min_max(
                frame.min + vec2(t.shape.hairline, t.spacing.s),
                frame.max - vec2(t.shape.hairline, t.shape.hairline),
            );
            match view.scope {
                Scope::Histogram => self.paint_histogram(&inside, plot, &t),
                Scope::Waveform | Scope::Parade => self.paint_waveform(&inside, plot, &t, id),
                Scope::Vectorscope => self.paint_vectorscope(&inside, plot, &t, id),
            }
            self.paint_grid(&inside, plot, &t);
            self.paint_clipping(&inside, plot, &t);
            if !measured && let Some(empty) = description {
                inside.text(
                    plot.center(),
                    Align2::CENTER_CENTER,
                    empty,
                    FontId::proportional(t.text.small),
                    t.colors.text_secondary,
                );
            }
            if shown {
                for control in &controls {
                    control.paint(painter, &t);
                }
            }
        }

        let over_control = controls.iter().any(|c| c.response.hovered());
        if !over_control {
            graph.on_hover_text(tr.text(view.scope.tip()));
        }
        let mut action = None;
        for control in controls {
            if control.enabled && control.response.clicked() && action.is_none() {
                action = control.action;
            }
            control.response.on_hover_text(control.name);
        }
        action
    }

    /// The colour channels overlaid, column by column, and luminance as a line over them.
    fn paint_histogram(&self, painter: &Painter, plot: Rect, t: &Theme) {
        let view = self.view;
        let shown = |c: Channel| view.shows(c) && !self.histogram.channel(c).is_empty();
        let colours: Vec<(Color32, &[f32])> = Channel::ALL[..3]
            .iter()
            .filter(|&&c| shown(c))
            .map(|&c| (c.colour(t), self.histogram.channel(c)))
            .collect();
        let luminance = shown(Channel::Luminance).then_some(self.histogram.luminance);
        // Luminance is filled too when it is alone.
        let fills: Vec<(Color32, &[f32])> = if colours.is_empty() {
            luminance.map(|l| (t.colors.text, l)).into_iter().collect()
        } else {
            colours
        };
        let peak = fills
            .iter()
            .map(|(_, bins)| *bins)
            .chain(luminance)
            .map(peak)
            .fold(0.0, f32::max);
        if peak <= 0.0 {
            return;
        }
        let height = |count: f32| {
            let share = match view.scale {
                Scale::Linear => count / peak,
                Scale::Logarithmic => count.ln_1p() / peak.ln_1p(),
            };
            share.clamp(0.0, 1.0) * plot.height()
        };
        let columns = plot.width().round().max(1.0) as usize;
        let width = plot.width() / columns as f32;
        let mut mesh = Mesh::default();
        let mut line = Vec::with_capacity(columns);
        let mut heights: Vec<(f32, Color32)> = Vec::with_capacity(3);
        for k in 0..columns {
            let (f0, f1) = (k as f32 / columns as f32, (k + 1) as f32 / columns as f32);
            let x0 = plot.left() + k as f32 * width;
            heights.clear();
            heights.extend(
                fills
                    .iter()
                    .map(|(colour, bins)| (height(sample(bins, f0, f1)), *colour)),
            );
            heights.sort_by(|a, b| a.0.total_cmp(&b.0));
            // From the foot up: each band takes the ink of every channel that reaches past it.
            let mut below = 0.0;
            for i in 0..heights.len() {
                let top = heights[i].0;
                if top > below {
                    let inks = heights[i..].iter().map(|&(_, colour)| (colour, INK));
                    mesh.add_colored_rect(
                        Rect::from_min_max(
                            pos2(x0, plot.bottom() - top),
                            pos2(x0 + width, plot.bottom() - below),
                        ),
                        mix(t.colors.sunken, inks),
                    );
                    below = top;
                }
            }
            if let Some(bins) = luminance {
                line.push(pos2(
                    x0 + width / 2.0,
                    plot.bottom() - height(sample(bins, f0, f1)),
                ));
            }
        }
        painter.add(mesh);
        if line.len() > 1 {
            painter.line(line, Stroke::new(LINE, t.colors.text));
        }
    }

    /// The waveform, the channels overlaid, or the parade, each in its own third.
    fn paint_waveform(&self, painter: &Painter, plot: Rect, t: &Theme, id: Id) {
        let w = &self.waveform;
        let view = self.view;
        let parade = view.scope == Scope::Parade;
        let channels: Vec<(Color32, &[f32])> = view
            .scope
            .channels()
            .iter()
            .filter(|&&c| view.shows(c))
            .filter_map(|&c| w.channel(c).map(|counts| (c.colour(t), counts)))
            .collect();
        if channels.is_empty() {
            return;
        }
        let ground = t.colors.sunken;
        let key = fingerprint(
            channels.iter().map(|(_, counts)| *counts),
            (
                view.scope,
                w.columns,
                w.levels,
                ground,
                &channels_colours(&channels),
            ),
        );
        let images = || {
            if parade {
                channels
                    .iter()
                    .map(|&(colour, counts)| waveform_image(w, &[(colour, counts)], ground))
                    .collect()
            } else {
                vec![waveform_image(w, &channels, ground)]
            }
        };
        let textures = textures(painter.ctx(), id.with("waveform"), key, images);
        let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        if parade {
            // Each channel in the third that is its own, whichever are hidden.
            let thirds = thirds(plot, t);
            let mut drawn = textures.iter();
            for (k, &channel) in Channel::ALL[..3].iter().enumerate() {
                if view.shows(channel)
                    && w.channel(channel).is_some()
                    && let Some(texture) = drawn.next()
                {
                    painter.image(texture.id(), thirds[k], full, Color32::WHITE);
                }
            }
        } else if let Some(texture) = textures.first() {
            painter.image(texture.id(), plot, full, Color32::WHITE);
        }
    }

    /// The vectorscope in a circle as tall as the plot.
    fn paint_vectorscope(&self, painter: &Painter, plot: Rect, t: &Theme, id: Id) {
        if !self.has(Scope::Vectorscope) {
            return;
        }
        let v = self.vectorscope;
        let ground = t.colors.sunken;
        let coloured = v.colours.len() == v.counts.len();
        let colours = if coloured { v.colours } else { &[] };
        let key = fingerprint(
            std::iter::once(v.counts),
            (v.size, ground, t.colors.text, colours),
        );
        let textures = textures(painter.ctx(), id.with("vectorscope"), key, || {
            let samples: f32 = v.counts.iter().map(|&c| count(c)).sum();
            let pixels = v
                .counts
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let strength = vectorscope_strength(count(c), samples, v.size);
                    if strength <= 0.0 {
                        return Color32::TRANSPARENT;
                    }
                    let colour = if coloured {
                        v.colours[i]
                    } else {
                        t.colors.text
                    };
                    vectorscope_ink(ground, colour, strength)
                })
                .collect();
            vec![ColorImage::new([v.size, v.size], pixels)]
        });
        if let Some(texture) = textures.first() {
            let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            painter.image(texture.id(), vectorscope_square(plot), full, Color32::WHITE);
        }
    }

    /// The grid, over the data so fills never hide it: quarters of the values across the
    /// histogram and up a waveform or each third of the parade; the vectorscope's rings at
    /// quarters of its radius and its axes.
    fn paint_grid(&self, painter: &Painter, plot: Rect, t: &Theme) {
        let grid = Stroke::new(t.shape.hairline, t.colors.divider);
        let quarters = (1..4).map(|k| k as f32 / 4.0);
        match self.view.scope {
            Scope::Histogram => {
                for f in quarters {
                    painter.vline(plot.left() + f * plot.width(), plot.y_range(), grid);
                }
            }
            Scope::Waveform => {
                for f in quarters {
                    painter.hline(plot.x_range(), plot.bottom() - f * plot.height(), grid);
                }
            }
            Scope::Parade => {
                for third in thirds(plot, t) {
                    for f in quarters.clone() {
                        painter.hline(third.x_range(), plot.bottom() - f * plot.height(), grid);
                    }
                }
            }
            Scope::Vectorscope => {
                let square = vectorscope_square(plot);
                let (centre, radius) = (square.center(), square.width() / 2.0);
                for f in quarters {
                    painter.circle_stroke(centre, radius * f, grid);
                }
                painter.circle_stroke(
                    centre,
                    radius,
                    Stroke::new(t.shape.hairline, t.colors.border),
                );
                painter.hline(square.x_range(), centre.y, grid);
                painter.vline(centre.x, square.y_range(), grid);
            }
        }
    }

    /// A mark along the edge where shadows clip, and one where highlights do, inside the
    /// plot: the ends of the histogram, the foot and the top of a waveform. The vectorscope
    /// has neither.
    fn paint_clipping(&self, painter: &Painter, plot: Rect, t: &Theme) {
        let Some(clipped) = self.clipped else {
            return;
        };
        let marks = match self.view.scope {
            Scope::Histogram => [
                Rect::from_x_y_ranges(plot.left()..=plot.left() + MARK, plot.y_range()),
                Rect::from_x_y_ranges(plot.right() - MARK..=plot.right(), plot.y_range()),
            ],
            Scope::Waveform | Scope::Parade => [
                Rect::from_x_y_ranges(plot.x_range(), plot.bottom() - MARK..=plot.bottom()),
                Rect::from_x_y_ranges(plot.x_range(), plot.top()..=plot.top() + MARK),
            ],
            Scope::Vectorscope => return,
        };
        if clipped.shadows > 0.0 {
            painter.rect_filled(marks[0], 0.0, t.colors.clip_shadows);
        }
        if clipped.highlights > 0.0 {
            painter.rect_filled(marks[1], 0.0, t.colors.clip_highlights);
        }
    }
}

/// The parade's thirds of the plot, with a gap between them.
fn thirds(plot: Rect, t: &Theme) -> [Rect; 3] {
    let gap = t.spacing.xs;
    let third = (plot.width() - 2.0 * gap) / 3.0;
    std::array::from_fn(|k| {
        let left = plot.left() + k as f32 * (third + gap);
        Rect::from_x_y_ranges(left..=left + third, plot.y_range())
    })
}

/// The vectorscope's square, as tall as the plot and centred in it.
fn vectorscope_square(plot: Rect) -> Rect {
    Rect::from_center_size(plot.center(), vec2(plot.height(), plot.height()))
}

/// How a control looks: an icon, or a channel's swatch, filled while the channel shows.
#[derive(Clone, Copy)]
enum Look {
    Icon(Icon),
    Swatch(Color32),
}

/// One of the controls over the graph.
struct Control {
    response: Response,
    look: Look,
    on: bool,
    enabled: bool,
    name: String,
    /// What a click reports.
    action: Option<HistogramAction>,
}

impl Control {
    /// A control at `rect`, its `role` and translated `name` for screen readers, with whether
    /// it is on and whether it is enabled.
    #[expect(clippy::too_many_arguments, reason = "one call per kind of control")]
    fn new(
        ui: &Ui,
        rect: Rect,
        id: Id,
        role: Role,
        name: String,
        look: Look,
        (on, enabled): (bool, bool),
        action: Option<HistogramAction>,
    ) -> Control {
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, id, sense);
        response.widget_info(|| WidgetInfo::selected(role, enabled, on, &name));
        Control {
            response,
            look,
            on,
            enabled,
            name,
            action,
        }
    }

    /// Drawn over the graph, on the raised colour so it reads over the data.
    fn paint(&self, painter: &Painter, t: &Theme) {
        let rect = self.response.rect;
        let pressed = self.on && matches!(self.look, Look::Icon(_));
        let fill = if pressed {
            t.colors.sunken
        } else if self.enabled && self.response.hovered() {
            t.colors.hover
        } else {
            t.colors.raised
        };
        painter.rect(
            rect,
            t.shape.radius,
            fill,
            Stroke::new(t.shape.hairline, t.colors.border),
            StrokeKind::Inside,
        );
        match self.look {
            Look::Icon(icon) => {
                // A scope's or a toggle's icon: secondary text at rest, text while lit or on.
                let lit = pressed || icon_lit(painter.ctx(), &self.response);
                let colour = icon_colour(t, self.enabled, lit);
                let size = icon.size();
                icon.paint(
                    painter,
                    Rect::from_center_size(rect.center(), vec2(size, size)),
                    colour,
                );
            }
            Look::Swatch(colour) => {
                let colour = if self.enabled {
                    colour
                } else {
                    t.colors.text_disabled
                };
                let swatch = rect.shrink(t.spacing.s);
                if self.on {
                    painter.rect_filled(swatch, t.shape.radius_small, colour);
                } else {
                    painter.rect_stroke(
                        swatch,
                        t.shape.radius_small,
                        Stroke::new(LINE, colour),
                        StrokeKind::Inside,
                    );
                }
            }
        }
        if self.response.has_focus() {
            focus_ring(painter, rect, t);
        }
    }
}

/// What clips, in words: "Highlights clipped: 0.4%", or "No clipping".
fn clipping(tr: &Translator, clipped: Clipped) -> String {
    let mut args = Args::new();
    args.set("shadows", percent(clipped.shadows));
    args.set("highlights", percent(clipped.highlights));
    let id = match (clipped.shadows > 0.0, clipped.highlights > 0.0) {
        (false, false) => "scope-no-clipping",
        (true, false) => "scope-clipped-shadows",
        (false, true) => "scope-clipped-highlights",
        (true, true) => "scope-clipped-both",
    };
    tr.text_with(id, Some(&args))
}

/// A share of the pixels as a percentage to read: one decimal from 1%, rounded down so
/// almost every pixel never reads as all of them, and two below 1%, rounded up so a few
/// never read as none.
fn percent(share: f32) -> f64 {
    let share = count(share);
    if share == 0.0 {
        return 0.0;
    }
    // In hundredths of a percent, with the noise past f32's digits rounded off first, so
    // 0.004 reads 0.4 and not 0.41.
    let hundredths = (f64::from(share.min(1.0)) * 1e7).round() / 1e3;
    let shown = if hundredths >= 100.0 {
        (hundredths / 10.0).floor() / 10.0
    } else {
        hundredths.ceil() / 100.0
    };
    shown.max(0.01)
}

/// A count as the graphs take it: one that is not a number, or below zero, is none.
fn count(v: f32) -> f32 {
    if v.is_finite() && v > 0.0 { v } else { 0.0 }
}

/// The tallest bin but the two ends, where clipped pixels pile up: those may reach past it
/// and are cut at the top. When only the ends have pixels, as in a frame that is all black
/// or all clipped, the taller end.
fn peak(bins: &[f32]) -> f32 {
    let tallest = |bins: &[f32]| bins.iter().map(|&v| count(v)).fold(0.0, f32::max);
    let inner = if bins.len() > 2 {
        tallest(&bins[1..bins.len() - 1])
    } else {
        0.0
    };
    if inner > 0.0 { inner } else { tallest(bins) }
}

/// The bins, evenly spaced from 0 to 1, over the span from `f0` to `f1`: the largest bin it
/// covers when it covers several, else the value between the two nearest bins' centres.
fn sample(bins: &[f32], f0: f32, f1: f32) -> f32 {
    let n = bins.len();
    if n == 0 {
        return 0.0;
    }
    if (f1 - f0) * n as f32 > 1.0 {
        let first = ((f0 * n as f32).floor() as usize).min(n - 1);
        let last = ((f1 * n as f32).ceil() as usize).clamp(first + 1, n);
        return bins[first..last]
            .iter()
            .map(|&v| count(v))
            .fold(0.0, f32::max);
    }
    let at = ((f0 + f1) / 2.0 * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
    let i = at.floor() as usize;
    let j = (i + 1).min(n - 1);
    let w = at - i as f32;
    count(bins[i]) * (1.0 - w) + count(bins[j]) * w
}

/// How dense a cell of a waveform looks, from 0 to 1: the logarithm of its count against the
/// largest, so sparse values still show.
fn density(count: f32, most: f32) -> f32 {
    (count.ln_1p() / most.ln_1p()).clamp(0.0, 1.0)
}

/// Inks, each a colour and a strength from 0 to 1, over `ground`: added as light on a dark
/// ground and multiplied as ink on a light one, so where channels overlap they mix as the
/// eye expects of each, and all three together go neutral. In linear light; opaque.
fn mix(ground: Color32, inks: impl Iterator<Item = (Color32, f32)>) -> Color32 {
    let ground = Rgba::from(ground);
    let light = ground.intensity() < 0.5;
    let mut out = [ground.r(), ground.g(), ground.b()];
    for (colour, strength) in inks {
        let colour = Rgba::from(colour);
        for (o, c) in out.iter_mut().zip([colour.r(), colour.g(), colour.b()]) {
            if light {
                *o += strength * c;
            } else {
                *o *= 1.0 - strength * (1.0 - c);
            }
        }
    }
    let [r, g, b] = out.map(|v| v.clamp(0.0, 1.0));
    Color32::from(Rgba::from_rgb(r, g, b))
}

/// How strongly a vectorscope cell holding `count` of the grid's `samples` draws, from 0 to
/// 1, as darktable's (`_vec_process`): the count scaled as darktable scales it, by 1/30 of
/// the grid's cells over the frame's pixels, four to a sample, so a cell with 120 times an
/// even spread's share draws at full strength; then through darktable's shortcut to display
/// gamma, the HLG curve ([`hlg`]), and to 8 bits, rounded down, as darktable's mask is. The
/// samples are those on the grid, where darktable's frame also holds the few off it.
fn vectorscope_strength(count: f32, samples: f32, size: usize) -> f32 {
    if count <= 0.0 || samples <= 0.0 {
        return 0.0;
    }
    let scaled = count * (size * size) as f32 / (120.0 * samples);
    (hlg(scaled.min(1.0)) * 255.0).floor() / 255.0
}

/// darktable's HLG Rec2020 profile's output curve, from linear to encoded, which darktable
/// reads as its shortcut to display gamma: BT.2100's HLG over darktable's black lift of
/// 0.04 (`src/common/colorspaces.c`, `_HLG_fct`), the inverse lcms tabulates. 0 below the
/// lift's black, 1 at 1.
fn hlg(linear: f32) -> f32 {
    const A: f32 = 0.178_832_77;
    const B: f32 = 0.284_668_92;
    const C: f32 = 0.559_910_7;
    const LIFT: f32 = 0.04;
    let linear = linear.clamp(0.0, 1.0);
    let encoded = if linear <= 1.0 / 12.0 {
        (3.0 * linear).sqrt()
    } else {
        A * (12.0 * linear - B).ln() + C
    };
    ((encoded - LIFT) / (1.0 - LIFT)).clamp(0.0, 1.0)
}

/// A vectorscope cell drawn over `ground` in `colour` at `strength`, from 0 to 1, as
/// darktable draws its counts (`_vec_draw`): on a dark ground the colour added at that
/// strength (`CAIRO_OPERATOR_ADD`), then lightened towards white by [`LIGHTEN`] times it
/// (`CAIRO_OPERATOR_HARD_LIGHT` with white at 0.55); on encoded values, as cairo composes.
/// On a light ground, where that would vanish into white, its mirror: the colour multiplied
/// as ink, then darkened as far towards black. Opaque.
fn vectorscope_ink(ground: Color32, colour: Color32, strength: f32) -> Color32 {
    let light = Rgba::from(ground).intensity() < 0.5;
    let channel = |ground: u8, colour: u8| {
        let (d, c) = (f32::from(ground) / 255.0, f32::from(colour) / 255.0);
        let drawn = if light {
            let added = (d + strength * c).min(1.0);
            added + LIGHTEN * strength * (1.0 - added)
        } else {
            let inked = d * (1.0 - strength * (1.0 - c));
            inked * (1.0 - LIGHTEN * strength)
        };
        (drawn.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color32::from_rgb(
        channel(ground.r(), colour.r()),
        channel(ground.g(), colour.g()),
        channel(ground.b(), colour.b()),
    )
}

/// A waveform's cells as an image, a pixel each, the brightest level at the top: each
/// channel's density inked over `ground`, and nothing where no channel has a pixel.
fn waveform_image(w: &WaveformBins, inks: &[(Color32, &[f32])], ground: Color32) -> ColorImage {
    let (columns, levels) = (w.columns, w.levels);
    let most: Vec<f32> = inks
        .iter()
        .map(|(_, counts)| counts.iter().map(|&v| count(v)).fold(0.0, f32::max))
        .collect();
    let mut pixels = vec![Color32::TRANSPARENT; columns * levels];
    for column in 0..columns {
        for level in 0..levels {
            let i = column * levels + level;
            let mut strengths = inks
                .iter()
                .zip(&most)
                .map(|((colour, counts), &most)| (*colour, count(counts[i]), most))
                .filter(|&(_, c, most)| c > 0.0 && most > 0.0)
                .map(|(colour, c, most)| (colour, density(c, most)))
                .peekable();
            if strengths.peek().is_some() {
                pixels[(levels - 1 - level) * columns + column] = mix(ground, strengths);
            }
        }
    }
    ColorImage::new([columns, levels], pixels)
}

fn channels_colours(channels: &[(Color32, &[f32])]) -> Vec<Color32> {
    channels.iter().map(|(colour, _)| *colour).collect()
}

/// The textures a graph is drawn from, made again only when what they show changes, as
/// `key` tells. New counts of the same size go into the textures already allocated.
fn textures(
    ctx: &egui::Context,
    id: Id,
    key: u64,
    make: impl FnOnce() -> Vec<ColorImage>,
) -> Vec<TextureHandle> {
    let cached = ctx.data(|d| d.get_temp::<Textures>(id));
    if let Some(cached) = &cached
        && cached.key == key
    {
        return cached.handles.clone();
    }
    let images = make();
    let reusable = cached.filter(|cached| {
        cached.handles.len() == images.len()
            && cached
                .handles
                .iter()
                .zip(&images)
                .all(|(handle, image)| handle.size() == image.size)
    });
    let handles: Vec<TextureHandle> = match reusable {
        Some(cached) => {
            for (handle, image) in cached.handles.iter().zip(images) {
                let mut handle = handle.clone();
                handle.set(image, TextureOptions::LINEAR);
            }
            cached.handles
        }
        None => images
            .into_iter()
            .enumerate()
            .map(|(i, image)| ctx.load_texture(format!("scope {i}"), image, TextureOptions::LINEAR))
            .collect(),
    };
    ctx.data_mut(|d| {
        d.insert_temp(
            id,
            Textures {
                key,
                handles: handles.clone(),
            },
        )
    });
    handles
}

/// A graph's textures and what they were made from, in egui's memory.
#[derive(Clone)]
struct Textures {
    key: u64,
    handles: Vec<TextureHandle>,
}

/// A fingerprint of the counts a texture shows and how it shows them: FNV-1a over the
/// counts' bits, cheap enough for every frame.
fn fingerprint<'a>(counts: impl Iterator<Item = &'a [f32]>, how: impl Hash) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    how.hash(&mut hasher);
    let mut h = hasher.finish();
    for slice in counts {
        for v in slice {
            h = (h ^ u64::from(v.to_bits())).wrapping_mul(0x0100_0000_01b3);
        }
        h = (h ^ slice.len() as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_graph_scale_and_channel_is_an_option() {
        let graphs: Vec<Scope> = Scope::OPTIONS.iter().map(|(s, _)| *s).collect();
        assert_eq!(graphs, Scope::ALL);
        let channels: Vec<Channel> = Channel::OPTIONS.iter().map(|(c, _)| *c).collect();
        assert_eq!(channels, Channel::ALL);
        assert_eq!(Scale::OPTIONS.len(), 2);
    }

    #[test]
    fn a_column_over_several_bins_takes_the_largest_and_between_bins_interpolates() {
        let bins = [0.0, 4.0, 2.0, 8.0];
        // Half the range covers the first two bins.
        assert_eq!(sample(&bins, 0.0, 0.5), 4.0);
        // Halfway between the centres of the second and third bins.
        assert!((sample(&bins, 0.49, 0.51) - 3.0).abs() < 1e-5);
        // Past the outer centres, the end bins.
        assert_eq!(sample(&bins, 0.0, 0.01), 0.0);
        assert_eq!(sample(&bins, 0.99, 1.0), 8.0);
        assert_eq!(sample(&[], 0.0, 1.0), 0.0);
    }

    #[test]
    fn counts_that_are_not_numbers_count_as_none() {
        let bins = [f32::NAN, 4.0, f32::INFINITY, -2.0];
        assert_eq!(sample(&bins, 0.0, 1.0), 4.0);
        assert_eq!(sample(&bins, 0.0, 0.01), 0.0);
        assert_eq!(peak(&bins), 4.0);
        let w = WaveformBins {
            columns: 1,
            levels: 2,
            red: &[f32::NAN, 3.0],
            ..WaveformBins::default()
        };
        let image = waveform_image(&w, &[(RED, w.red)], Color32::from_gray(0x1C));
        // The brightest level at the top: the NaN level stays empty.
        assert_ne!(image.pixels[0], Color32::TRANSPARENT);
        assert_eq!(image.pixels[1], Color32::TRANSPARENT);
        assert_eq!(percent(f32::NAN), 0.0);
    }

    #[test]
    fn the_peak_leaves_out_the_ends_where_clipping_piles_up() {
        assert_eq!(peak(&[900.0, 3.0, 5.0, 2.0, 700.0]), 5.0);
        assert_eq!(peak(&[1.0, 2.0]), 2.0);
        // A lens cap, or everything clipped: only the ends hold pixels.
        assert_eq!(peak(&[900.0, 0.0, 0.0, 0.0, 0.0]), 900.0);
        assert_eq!(peak(&[0.0, 0.0, 0.0, 70.0]), 70.0);
    }

    #[test]
    fn clipping_reads_in_percent_never_as_none_and_never_as_all() {
        assert_eq!(percent(0.004), 0.4);
        assert_eq!(percent(0.0000001), 0.01);
        // One pixel of a 6016 × 3384 frame.
        assert_eq!(percent(1.0 / (6016.0 * 3384.0)), 0.01);
        assert_eq!(percent(0.1234), 12.3);
        assert_eq!(percent(0.9996), 99.9);
        assert_eq!(percent(1.0), 100.0);
        assert_eq!(percent(0.0), 0.0);
        let tr = Translator::new(crate::Locale::EnGb);
        let text = |shadows, highlights| {
            clipping(
                &tr,
                Clipped {
                    shadows,
                    highlights,
                },
            )
        };
        assert_eq!(text(0.0, 0.0), "No clipping");
        assert_eq!(text(0.0, 0.004), "Highlights clipped: 0.4%");
        assert_eq!(
            text(0.0012, 0.004),
            "Shadows clipped: 0.12%, highlights clipped: 0.4%"
        );
        let uk = Translator::new(crate::Locale::Uk);
        assert!(
            clipping(
                &uk,
                Clipped {
                    shadows: 0.0,
                    highlights: 0.004
                }
            )
            .contains("0,4")
        );
    }

    /// Whether a colour is grey within a step of 8 bits.
    fn neutral(c: Color32) -> bool {
        let [r, g, b, _] = c.to_array();
        r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1 && r.abs_diff(b) <= 1
    }

    #[test]
    fn all_three_channels_go_neutral_on_either_ground_at_any_strength() {
        let dark = Color32::from_gray(0x1C);
        let light = Color32::from_gray(0xF2);
        for ground in [dark, light] {
            for strength in [INK, 0.25, 1.0] {
                let all = mix(
                    ground,
                    [RED, GREEN, BLUE].map(|c| (c, strength)).into_iter(),
                );
                assert!(neutral(all), "{all:?} on {ground:?} at {strength}");
            }
        }
        // Away from the ground: lighter on dark, darker on light.
        let full = |ground| mix(ground, [RED, GREEN, BLUE].map(|c| (c, 1.0)).into_iter());
        assert!(full(dark).r() > 0xC0);
        assert!(full(light).r() < 0x80);
    }

    #[test]
    fn one_or_two_channels_keep_their_hue_on_either_ground() {
        for ground in [Color32::from_gray(0x1C), Color32::from_gray(0xF2)] {
            let red = mix(ground, [(RED, INK)].into_iter());
            assert!(red.r() > red.g() && red.g() == red.b(), "{red:?}");
            // Red and green make yellow.
            let yellow = mix(ground, [(RED, 1.0), (GREEN, 1.0)].into_iter());
            assert!(
                yellow.r() == yellow.g() && yellow.g() > yellow.b(),
                "{yellow:?}"
            );
        }
        // No ink, no change.
        let dark = Color32::from_gray(0x1C);
        assert_eq!(mix(dark, std::iter::empty()), dark);
    }

    #[test]
    fn a_vectorscope_cell_draws_as_strongly_as_darktable_s() {
        // darktable's HLG output curve: black up to its lift, and white at 1.
        assert_eq!(hlg(0.0), 0.0);
        assert_eq!(hlg(0.04 * 0.04 / 3.0), 0.0);
        assert!((hlg(1.0 / 12.0) - 0.46 / 0.96).abs() < 1e-6);
        assert!((hlg(1.0) - 1.0).abs() < 1e-6);
        // An even spread over the grid draws at 31/255, HLG of 1/120.
        let (size, samples) = (128, 960_000.0);
        let even = samples / (size * size) as f32;
        assert_eq!(vectorscope_strength(even, samples, size), 31.0 / 255.0);
        // The same spread on a coarser grid draws as strongly.
        assert_eq!(
            vectorscope_strength(4.0 * even, samples, size / 2),
            31.0 / 255.0
        );
        // From 120 times its share, full strength; none counted, none.
        assert_eq!(vectorscope_strength(120.0 * even, samples, size), 1.0);
        assert_eq!(vectorscope_strength(1e4 * even, samples, size), 1.0);
        assert_eq!(vectorscope_strength(0.0, samples, size), 0.0);
        assert_eq!(vectorscope_strength(1.0, 0.0, size), 0.0);
    }

    #[test]
    fn the_vectorscope_lightens_its_colours_on_a_dark_ground_and_darkens_them_on_a_light_one() {
        let dark = Color32::from_gray(0x23);
        let white = Color32::WHITE;
        let red = Color32::from_rgb(255, 0, 0);
        // darktable's: red added to the ground, then 55% of the way to white.
        assert_eq!(
            vectorscope_ink(dark, red, 1.0),
            Color32::from_rgb(255, 156, 156)
        );
        // On white the mirror: inked, then 55% of the way to black. The azure darktable
        // paints at neutral.
        let azure = Color32::from_rgb(0, 136, 255);
        assert_eq!(
            vectorscope_ink(white, azure, 1.0),
            Color32::from_rgb(0, 61, 115)
        );
        for ground in [dark, white, Color32::BLACK] {
            assert_eq!(vectorscope_ink(ground, red, 0.0), ground);
        }
        // Every hue reads against either ground at full strength: far lighter than the dark
        // one, far darker than white.
        for hue in [
            red,
            Color32::from_rgb(255, 255, 0),
            Color32::from_rgb(0, 255, 0),
            azure,
            Color32::from_rgb(0, 0, 255),
            Color32::from_rgb(255, 0, 255),
        ] {
            let lit = Rgba::from(vectorscope_ink(dark, hue, 1.0)).intensity();
            assert!(lit > 0.3, "{hue:?} on the dark ground: {lit}");
            let inked = Rgba::from(vectorscope_ink(white, hue, 1.0)).intensity();
            assert!(inked < 0.2, "{hue:?} on white: {inked}");
        }
    }

    #[test]
    fn new_counts_of_the_same_size_reuse_the_textures() {
        let ctx = egui::Context::default();
        let id = Id::unique("scope textures");
        let image = |v: u8| vec![ColorImage::filled([4, 2], Color32::from_gray(v))];
        let first = textures(&ctx, id, 1, || image(10));
        let again = textures(&ctx, id, 1, || unreachable!("the key has not changed"));
        assert_eq!(first[0].id(), again[0].id());
        let changed = textures(&ctx, id, 2, || image(20));
        assert_eq!(first[0].id(), changed[0].id());
        let resized = textures(&ctx, id, 3, || {
            vec![ColorImage::filled([8, 2], Color32::from_gray(30))]
        });
        assert_ne!(first[0].id(), resized[0].id());
    }
}
