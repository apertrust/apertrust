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

//! The icons: one-colour SVG files in `assets/icons`, compiled in, most derived from
//! Lucide's (each names its source, `assets/icons/LICENSE-Lucide` holds Lucide's notices)
//! and the rest drawn for Apertrust in Lucide's style. Each is rendered with resvg at the
//! exact pixel size it is drawn at, its lines [`STROKE`] points wide whatever that size,
//! cached, and tinted with a theme colour, so it follows the theme and stays sharp at any
//! scale (`docs/gui.md`, Themes). The files' rules are in `assets/icons/README.md`;
//! `Icon::file` maps each icon to its file.
//!
//! A [`Picture`] is an SVG in its own colours, never tinted, rendered the same way: the
//! app's full logo in About.

use std::collections::HashMap;

use egui::emath::GuiRounding as _;
use egui::{
    Color32, ColorImage, Id, Painter, Rect, Response, Role, Sense, TextureHandle, TextureOptions,
    Ui, WidgetInfo, pos2, vec2,
};
use resvg::{tiny_skia, usvg};

/// Every icon the design uses so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    /// A section's open and closed state, pointing right; turned down when open.
    Chevron,
    /// A menu's ⌄.
    ChevronDown,
    /// ⌄ turned to point to the end, of the same size and weight: a closed collapsible
    /// group's chevron, which turns to ⌄ when it opens.
    ChevronEnd,
    /// A step back, pointing left: a dialog's page back to the one before.
    Back,
    /// A choice list's ⌃⌄.
    UpDown,
    /// Auto.
    Wand,
    Reset,
    Presets,
    /// A section's ⋯ options.
    Options,
    /// The tab bar's ⋮ options.
    OptionsVertical,
    /// » : more of something that doesn't fit, such as tabs.
    More,
    /// A quad's eyedropper: pick from the photo.
    Pick,
    /// A quad's line tool: draw on the photo.
    Line,
    Undo,
    Redo,
    /// Copy the edit: two sheets, as copying anything.
    Copy,
    /// Apply the copied edit: a clipboard with an arrow, as pasting anything.
    Apply,
    Import,
    Export,
    Settings,
    /// The central bar: every selected photo side by side.
    PhotosSelected,
    /// The central bar: the proof margin.
    ProofMargin,
    /// The central bar: before and after.
    BeforeAfter,
    /// The central bar's Guides: a grid over the photo, until it has an icon of its own.
    Grid,
    /// A new layer.
    Plus,
    ZoomIn,
    ZoomOut,
    /// The grid bar's Sort, showing the direction: lines narrow to wide under an arrow up,
    /// and wide to narrow over an arrow down.
    SortAscending,
    SortDescending,
    /// A small check: a menu item that is checked.
    Check,
    /// A triangle with a mark: something went wrong, as the jobs spinner and a job's row say
    /// of a failed job.
    Warning,
    /// A lock: a photo open for viewing only, in the caption under it.
    Lock,
    /// A pencil: a photo with an edit, on its cell in the grid panel.
    Edited,
    /// An eye: the grid bar's Labels, which shows the captions under the thumbnails.
    Labels,
    /// A star: a point of a photo's rating, under its thumbnail; filled for each point.
    Star,
    /// A crossed circle: a photo's reject mark, under its thumbnail.
    Reject,
    /// darktable's orientation buttons: a square's corner with an arrow turning
    /// anticlockwise and clockwise, so as not to be read as Reset's circular arrow, and two
    /// triangles mirrored across a vertical and a horizontal line.
    RotateLeft,
    RotateRight,
    FlipHorizontal,
    FlipVertical,
    /// The side panel's tabs: darktable's four module groups, base, color, correct and
    /// effect, in darktable's shapes and one colour (decision 38), and Apertrust's own.
    Base,
    Color,
    Correct,
    Effect,
    History,
    Library,
    Metadata,
    /// The scopes' graphs: the histogram, the waveform, the RGB parade, the vectorscope.
    Histogram,
    Waveform,
    Parade,
    Vectorscope,
    /// The histogram's logarithmic scale.
    LogScale,
    /// The scopes' clipping overlays: shadows at black, highlights at white.
    ClipShadows,
    ClipHighlights,
    /// The window's caption buttons, where the app draws them: minimise, maximise,
    /// restore a maximised window, close.
    Minimise,
    Maximise,
    Restore,
    Close,
    /// The top panel's ☰ on Windows and Linux, which opens the menus.
    Menu,
    /// The app's mark, the folded A with its cut, at the top panel's start: filled shapes,
    /// not lines, the one icon drawn so (`assets/icons/README.md`).
    Mark,
    /// Filled forms, drawn only to show a state: the chosen side panel tab, a bar's
    /// toggle that is on, a rating's points (`Icon::filled`).
    BaseFilled,
    ColorFilled,
    CorrectFilled,
    EffectFilled,
    HistoryFilled,
    LibraryFilled,
    MetadataFilled,
    PhotosSelectedFilled,
    ProofMarginFilled,
    BeforeAfterFilled,
    GridFilled,
    LabelsFilled,
    StarFilled,
}

impl Icon {
    pub const ALL: [Icon; 73] = [
        Icon::Chevron,
        Icon::ChevronDown,
        Icon::ChevronEnd,
        Icon::Back,
        Icon::UpDown,
        Icon::Wand,
        Icon::Reset,
        Icon::Presets,
        Icon::Options,
        Icon::OptionsVertical,
        Icon::More,
        Icon::Pick,
        Icon::Line,
        Icon::Undo,
        Icon::Redo,
        Icon::Copy,
        Icon::Apply,
        Icon::Import,
        Icon::Export,
        Icon::Settings,
        Icon::PhotosSelected,
        Icon::ProofMargin,
        Icon::BeforeAfter,
        Icon::Grid,
        Icon::Plus,
        Icon::ZoomIn,
        Icon::ZoomOut,
        Icon::SortAscending,
        Icon::SortDescending,
        Icon::Check,
        Icon::Warning,
        Icon::Lock,
        Icon::Edited,
        Icon::Labels,
        Icon::Star,
        Icon::Reject,
        Icon::RotateLeft,
        Icon::RotateRight,
        Icon::FlipHorizontal,
        Icon::FlipVertical,
        Icon::Base,
        Icon::Color,
        Icon::Correct,
        Icon::Effect,
        Icon::History,
        Icon::Library,
        Icon::Metadata,
        Icon::Histogram,
        Icon::Waveform,
        Icon::Parade,
        Icon::Vectorscope,
        Icon::LogScale,
        Icon::ClipShadows,
        Icon::ClipHighlights,
        Icon::Minimise,
        Icon::Maximise,
        Icon::Restore,
        Icon::Close,
        Icon::Menu,
        Icon::Mark,
        Icon::BaseFilled,
        Icon::ColorFilled,
        Icon::CorrectFilled,
        Icon::EffectFilled,
        Icon::HistoryFilled,
        Icon::LibraryFilled,
        Icon::MetadataFilled,
        Icon::PhotosSelectedFilled,
        Icon::ProofMarginFilled,
        Icon::BeforeAfterFilled,
        Icon::GridFilled,
        Icon::LabelsFilled,
        Icon::StarFilled,
    ];

    /// The icon's filled form, which shows a state: the chosen tab, a toggle that is on or
    /// a rating's point. An icon without one is its own.
    pub fn filled(self) -> Icon {
        match self {
            Icon::Base => Icon::BaseFilled,
            Icon::Color => Icon::ColorFilled,
            Icon::Correct => Icon::CorrectFilled,
            Icon::Effect => Icon::EffectFilled,
            Icon::History => Icon::HistoryFilled,
            Icon::Library => Icon::LibraryFilled,
            Icon::Metadata => Icon::MetadataFilled,
            Icon::PhotosSelected => Icon::PhotosSelectedFilled,
            Icon::ProofMargin => Icon::ProofMarginFilled,
            Icon::BeforeAfter => Icon::BeforeAfterFilled,
            Icon::Grid => Icon::GridFilled,
            Icon::Labels => Icon::LabelsFilled,
            Icon::Star => Icon::StarFilled,
            other => other,
        }
    }

    /// The icon's name in code, its own file's name without `.svg`; never shown to users.
    pub fn name(self) -> &'static str {
        self.file().name
    }

    /// The side of the square it is drawn in by default, in points. Its file's design box
    /// is Lucide's 24 units, but for the mark's and the caption buttons', which are their
    /// size.
    pub fn size(self) -> f32 {
        self.file().size
    }

    fn source(self) -> &'static str {
        self.file().source
    }

    /// Whether the icon keeps a design box and stroke of its own rather than Lucide's: the
    /// mark and the window's caption buttons, drawn as the systems draw theirs.
    fn keeps_its_box(self) -> bool {
        matches!(
            self,
            Icon::Mark | Icon::Minimise | Icon::Maximise | Icon::Restore | Icon::Close
        )
    }

    /// The Lucide icon the icon's file is derived from; None for one drawn for Apertrust.
    pub fn lucide(self) -> Option<&'static str> {
        self.file().lucide
    }

    /// The one place each icon is mapped to its file, `assets/icons/<name>.svg`: its
    /// name, the size it is drawn at, the file's text and the Lucide icon it is derived
    /// from, which the file's first line names (`assets/icons/README.md`).
    fn file(self) -> File {
        macro_rules! derived {
            ($name:literal, $lucide:literal, $size:literal) => {
                File {
                    name: $name,
                    size: $size,
                    source: include_str!(concat!("../assets/icons/", $name, ".svg")),
                    lucide: Some($lucide),
                }
            };
        }
        macro_rules! own {
            ($name:literal, $size:literal) => {
                File {
                    name: $name,
                    size: $size,
                    source: include_str!(concat!("../assets/icons/", $name, ".svg")),
                    lucide: None,
                }
            };
        }
        match self {
            Icon::Chevron => derived!("chevron", "chevron-right", 12.0),
            Icon::ChevronDown => derived!("chevron-down", "chevron-down", 10.0),
            Icon::ChevronEnd => derived!("chevron-end", "chevron-right", 10.0),
            Icon::Back => derived!("back", "chevron-left", 12.0),
            Icon::UpDown => derived!("up-down", "chevrons-up-down", 12.0),
            Icon::Wand => derived!("wand", "wand", 14.0),
            Icon::Reset => derived!("reset", "rotate-ccw", 14.0),
            Icon::Presets => derived!("presets", "menu", 14.0),
            Icon::Options => derived!("options", "ellipsis", 14.0),
            Icon::OptionsVertical => derived!("options-vertical", "ellipsis-vertical", 14.0),
            Icon::More => derived!("more", "chevrons-right", 14.0),
            Icon::Pick => derived!("pick", "pipette", 14.0),
            Icon::Line => own!("line", 14.0),
            Icon::Undo => derived!("undo", "undo-2", 16.0),
            Icon::Redo => derived!("redo", "redo-2", 16.0),
            Icon::Copy => own!("copy", 16.0),
            Icon::Apply => own!("apply", 16.0),
            Icon::Import => derived!("import", "download", 16.0),
            Icon::Export => derived!("export", "upload", 16.0),
            Icon::Settings => derived!("settings", "settings", 16.0),
            Icon::PhotosSelected => derived!("photos-selected", "layout-grid", 16.0),
            Icon::ProofMargin => own!("proof-margin", 16.0),
            Icon::BeforeAfter => derived!("before-after", "square-split-horizontal", 16.0),
            Icon::Grid => derived!("grid", "grid-3x3", 16.0),
            Icon::Plus => derived!("plus", "plus", 14.0),
            // Their files mirrored, the lens at the top right and the handle at the bottom left.
            Icon::ZoomIn => derived!("zoom-in", "zoom-in", 16.0),
            Icon::ZoomOut => derived!("zoom-out", "zoom-out", 16.0),
            Icon::SortAscending => derived!("sort-ascending", "arrow-up-narrow-wide", 16.0),
            Icon::SortDescending => derived!("sort-descending", "arrow-down-wide-narrow", 16.0),
            Icon::Check => derived!("check", "check", 10.0),
            Icon::Warning => derived!("warning", "triangle-exclamation-point", 12.0),
            Icon::Lock => derived!("lock", "lock", 12.0),
            Icon::Edited => derived!("edited", "pencil", 12.0),
            Icon::Labels => derived!("labels", "eye", 16.0),
            Icon::Star => derived!("star", "star", 12.0),
            Icon::Reject => derived!("reject", "circle-x", 12.0),
            Icon::RotateLeft => derived!("rotate-left", "rotate-ccw-square", 14.0),
            Icon::RotateRight => derived!("rotate-right", "rotate-cw-square", 14.0),
            Icon::FlipHorizontal => derived!(
                "flip-horizontal",
                "triangles-centerline-dashed-vertical",
                14.0
            ),
            Icon::FlipVertical => derived!(
                "flip-vertical",
                "triangles-centerline-dashed-horizontal",
                14.0
            ),
            Icon::Base => own!("base", 22.0),
            Icon::Color => own!("color", 22.0),
            Icon::Correct => own!("correct", 22.0),
            Icon::Effect => own!("effect", 22.0),
            Icon::History => derived!("history", "rotate-ccw-clock", 22.0),
            Icon::Library => derived!("library", "folder", 22.0),
            Icon::Metadata => derived!("metadata", "tag", 22.0),
            Icon::Histogram => own!("histogram", 14.0),
            Icon::Waveform => own!("waveform", 14.0),
            Icon::Parade => own!("parade", 14.0),
            Icon::Vectorscope => own!("vectorscope", 14.0),
            Icon::LogScale => own!("log-scale", 14.0),
            Icon::ClipShadows => own!("clip-shadows", 12.0),
            Icon::ClipHighlights => own!("clip-highlights", 12.0),
            Icon::Minimise => own!("minimise", 10.0),
            Icon::Maximise => own!("maximise", 10.0),
            Icon::Restore => own!("restore", 10.0),
            Icon::Close => own!("close", 10.0),
            Icon::Menu => derived!("menu", "menu", 16.0),
            Icon::Mark => own!("mark", 20.0),
            Icon::BaseFilled => own!("base-filled", 22.0),
            Icon::ColorFilled => own!("color-filled", 22.0),
            Icon::CorrectFilled => own!("correct-filled", 22.0),
            Icon::EffectFilled => own!("effect-filled", 22.0),
            Icon::HistoryFilled => derived!("history-filled", "rotate-ccw-clock", 22.0),
            Icon::LibraryFilled => derived!("library-filled", "folder", 22.0),
            Icon::MetadataFilled => derived!("metadata-filled", "tag", 22.0),
            Icon::PhotosSelectedFilled => derived!("photos-selected-filled", "layout-grid", 16.0),
            Icon::ProofMarginFilled => own!("proof-margin-filled", 16.0),
            Icon::BeforeAfterFilled => {
                derived!("before-after-filled", "square-split-horizontal", 16.0)
            }
            Icon::GridFilled => derived!("grid-filled", "grid-3x3", 16.0),
            Icon::LabelsFilled => derived!("labels-filled", "eye", 16.0),
            Icon::StarFilled => derived!("star-filled", "star", 12.0),
        }
    }

    /// Draws the icon centred in `rect`, as large as fits, in `color`. Its corner sits on
    /// the pixel grid and it is rendered at its size in physical pixels, so nothing is
    /// resampled; its lines are [`STROKE`] points wide whatever its size.
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        let ppp = painter.pixels_per_point();
        let pixels = (rect.width().min(rect.height()) * ppp).round().max(1.0) as u32;
        let side = pixels as f32 / ppp;
        let Some(texture) = texture(painter.ctx(), self, pixels, side) else {
            return;
        };
        let min = (rect.center() - vec2(side, side) / 2.0).round_to_pixels(ppp);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(
            texture.id(),
            Rect::from_min_size(min, vec2(side, side)),
            uv,
            color,
        );
    }
}

/// The width of every icon's lines, in points, whatever size it is drawn at: Lucide's
/// `absoluteStrokeWidth`, set as each icon is rendered, so a 10-point chevron is as heavy
/// as a 22-point tab. The mark and the caption buttons keep their own.
pub const STROKE: f32 = 1.25;

/// An icon's file and how it is drawn (`Icon::file`).
struct File {
    name: &'static str,
    size: f32,
    source: &'static str,
    lucide: Option<&'static str>,
}

/// Rendered icons, by icon, size in pixels and size in points, kept for the session.
#[derive(Clone, Default)]
struct Cache(HashMap<(Icon, u32, u32), TextureHandle>);

fn texture(ctx: &egui::Context, icon: Icon, pixels: u32, points: f32) -> Option<TextureHandle> {
    let id = Id::unique("apertrust-ui icons");
    let key = (icon, pixels, points.to_bits());
    if let Some(texture) =
        ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(id).0.get(&key).cloned())
    {
        return Some(texture);
    }
    let image = render(icon, pixels, points)?;
    let texture = ctx.load_texture(
        format!("icon {} {pixels}px", icon.name()),
        image,
        TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(id)
            .0
            .insert(key, texture.clone())
    });
    Some(texture)
}

/// Renders the icon in white at `pixels` × `pixels`, to be drawn `points` wide, its lines
/// [`STROKE`] points wide; drawing it tints the white.
fn render(icon: Icon, pixels: u32, points: f32) -> Option<ColorImage> {
    let mut source = icon.source().replace("currentColor", "#FFFFFF");
    if !icon.keeps_its_box() {
        // Every file of Lucide's grid sets its 2-unit stroke once, on the <svg> element.
        let units = STROKE * 24.0 / points;
        source = source.replace(r#"stroke-width="2""#, &format!(r#"stroke-width="{units}""#));
    }
    let tree = usvg::Tree::from_str(&source, &usvg::Options::default()).ok()?;
    rasterise(&tree, pixels, pixels)
}

/// Renders `tree` into `width` × `height` pixels, as large as fits in its proportions.
fn rasterise(tree: &usvg::Tree, width: u32, height: u32) -> Option<ColorImage> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let size = tree.size();
    let scale = (width as f32 / size.width()).min(height as f32 / size.height());
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(ColorImage::from_rgba_premultiplied(
        [width as usize, height as usize],
        pixmap.data(),
    ))
}

/// An SVG drawn in its own colours, never tinted: the app's full logo, whose files the app
/// compiles in. Rendered with resvg at the exact pixel size it is drawn at and cached, as
/// the icons are, so it stays sharp at any scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Picture {
    /// Its name in code, for the cache; never shown to users.
    name: &'static str,
    /// The SVG file's text.
    source: &'static str,
}

impl Picture {
    pub const fn new(name: &'static str, source: &'static str) -> Self {
        Picture { name, source }
    }

    /// Draws it `width` points wide, as tall as its proportions make it, its corner on the
    /// pixel grid; screen readers hear it as an image named `label`, which is translated.
    pub fn show(self, ui: &mut Ui, width: f32, label: &str) -> Response {
        let ppp = ui.ctx().pixels_per_point();
        let pixels = (width * ppp).round().max(1.0) as u32;
        let texture = self.texture(ui.ctx(), pixels);
        let [w, h] = texture.as_ref().map_or([pixels as usize, 0], |t| t.size());
        let (rect, response) =
            ui.allocate_exact_size(vec2(w as f32, h as f32) / ppp, Sense::hover());
        if let Some(texture) = texture
            && ui.is_rect_visible(rect)
        {
            let rect = Rect::from_min_size(rect.min.round_to_pixels(ppp), rect.size());
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Image, true, label));
        response
    }

    /// The picture rendered `pixels` wide, from the cache or rendered now; None when its
    /// file can't be read.
    fn texture(self, ctx: &egui::Context, pixels: u32) -> Option<TextureHandle> {
        let id = Id::unique("apertrust-ui pictures");
        let key = (self.name, pixels);
        let cached = ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<PictureCache>(id)
                .0
                .get(&key)
                .cloned()
        });
        if cached.is_some() {
            return cached;
        }
        let tree = usvg::Tree::from_str(self.source, &usvg::Options::default()).ok()?;
        let size = tree.size();
        let height = (pixels as f32 * size.height() / size.width())
            .round()
            .max(1.0) as u32;
        let image = rasterise(&tree, pixels, height)?;
        let texture = ctx.load_texture(
            format!("picture {} {pixels}px", self.name),
            image,
            TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<PictureCache>(id)
                .0
                .insert(key, texture.clone())
        });
        Some(texture)
    }
}

/// Rendered pictures, by name and width in pixels, kept for the session.
#[derive(Clone, Default)]
struct PictureCache(HashMap<(&'static str, u32), TextureHandle>);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_file_follows_the_icon_rules() {
        for icon in Icon::ALL {
            let name = icon.name();
            let source = icon.source();
            let tree = usvg::Tree::from_str(source, &usvg::Options::default())
                .unwrap_or_else(|e| panic!("{name}.svg: {e}"));
            // Lucide's 24-unit box, but for the mark's and the caption buttons', which are
            // their size.
            let side = if icon.keeps_its_box() {
                icon.size()
            } else {
                24.0
            };
            let size = tree.size();
            assert_eq!(
                (size.width(), size.height()),
                (side, side),
                "{name}.svg's box is not {side} units"
            );
            // Lucide's weight, set once on the <svg>, where `render` replaces it: a 2-unit
            // stroke, round caps and joins.
            if !icon.keeps_its_box() {
                let start = source.find("<svg").unwrap();
                let root = &source[start..start + source[start..].find('>').unwrap()];
                for attribute in [
                    r#"stroke-width="2""#,
                    r#"stroke-linecap="round""#,
                    r#"stroke-linejoin="round""#,
                ] {
                    assert!(root.contains(attribute), "{name}.svg lacks {attribute}");
                }
                assert_eq!(
                    source.matches("stroke-width").count(),
                    1,
                    "{name}.svg sets a stroke width other than its <svg>'s"
                );
            }
            // One colour: no colour but currentColor is named. Only a filled form, and the
            // mark's cut, may cut lines out, with the one mask their rules allow.
            let filled = Icon::ALL.iter().any(|&i| i != icon && i.filled() == icon);
            let masked = filled || icon == Icon::Mark;
            let source = if masked {
                source.replace("url(#cut)", "")
            } else {
                source.to_owned()
            };
            if source.contains("<mask") {
                assert!(masked, "{name}.svg uses a mask but is not a filled form");
                assert_eq!(source.matches("<mask").count(), 1, "{name}.svg");
                assert!(source.contains(r#"<mask id="cut""#), "{name}.svg");
            }
            for forbidden in ["#", "rgb(", "<text", "<image", "Gradient", "<filter"] {
                assert!(!source.contains(forbidden), "{name}.svg uses {forbidden}");
            }
        }
    }

    #[test]
    fn every_icon_renders_at_any_size() {
        for icon in Icon::ALL {
            for pixels in [10, 14, 28, 44] {
                let image = render(icon, pixels, pixels as f32)
                    .unwrap_or_else(|| panic!("{}", icon.name()));
                assert_eq!(image.size, [pixels as usize; 2]);
                let inked = image.pixels.iter().filter(|p| p.a() > 0).count();
                assert!(inked > 0, "{} draws nothing at {pixels}px", icon.name());
                // White, so a tint gives the theme's colour exactly.
                assert!(
                    image
                        .pixels
                        .iter()
                        .all(|p| p.r() == p.a() && p.g() == p.a() && p.b() == p.a()),
                    "{} is not white",
                    icon.name()
                );
            }
        }
    }

    #[test]
    fn a_filled_form_matches_its_outline() {
        for icon in Icon::ALL {
            let filled = icon.filled();
            if filled != icon {
                assert_eq!(filled.size(), icon.size(), "{}", icon.name());
                assert_eq!(filled.name(), format!("{}-filled", icon.name()));
                assert_eq!(filled.filled(), filled, "{}", filled.name());
            }
        }
    }

    #[test]
    fn a_picture_keeps_its_colours_and_its_proportions() {
        // Two squares side by side, red and blue, in a box offset as the logo's is.
        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10" viewBox="5 5 20 10"><rect x="5" y="5" width="10" height="10" fill="#FF0000"/><rect x="15" y="5" width="10" height="10" fill="#0000FF"/></svg>"##;
        let ctx = egui::Context::default();
        let texture = Picture::new("test", source).texture(&ctx, 40).unwrap();
        assert_eq!(texture.size(), [40, 20]);
        let tree = usvg::Tree::from_str(source, &usvg::Options::default()).unwrap();
        let image = rasterise(&tree, 40, 20).unwrap();
        assert_eq!(image.pixels[40 * 10 + 5], Color32::from_rgb(255, 0, 0));
        assert_eq!(image.pixels[40 * 10 + 35], Color32::from_rgb(0, 0, 255));
    }

    /// The ink down a column of `image`, in pixels of full coverage.
    fn ink_down(image: &ColorImage, x: usize) -> f32 {
        let [w, h] = image.size;
        (0..h)
            .map(|y| image.pixels[y * w + x].a() as f32 / 255.0)
            .sum()
    }

    #[test]
    fn lines_are_as_wide_at_every_size() {
        // The menu's three level lines, crossed by a column through their middle.
        for (pixels, ppp) in [
            (10, 1.0),
            (14, 1.0),
            (16, 1.0),
            (22, 1.0),
            (28, 2.0),
            (44, 2.0),
        ] {
            let points = pixels as f32 / ppp;
            let image = render(Icon::Menu, pixels, points).unwrap();
            let width = ink_down(&image, pixels as usize / 2) / 3.0 / ppp;
            assert!(
                (width - STROKE).abs() < 0.1,
                "{width} points wide at {pixels} pixels, {ppp} a point"
            );
        }
    }

    #[test]
    fn the_magnifiers_hold_their_lens_at_the_top_right() {
        for icon in [Icon::ZoomIn, Icon::ZoomOut] {
            let image = render(icon, 48, 48.0).unwrap();
            let [left, right] = [6, 42].map(|x| ink_down(&image, x));
            // Down the left edge the handle, down the right the lens, which is taller.
            assert!(right > left, "{}: {left} left, {right} right", icon.name());
            let handle = |x: usize, y: usize| image.pixels[y * 48 + x].a();
            assert!(handle(6, 42) > 0 && handle(42, 42) == 0, "{}", icon.name());
        }
    }

    #[test]
    fn every_icon_derived_from_lucide_names_its_source_and_licence() {
        let licence = include_str!("../assets/icons/LICENSE-Lucide");
        assert!(licence.starts_with("ISC License"));
        assert!(licence.contains("Copyright (c) 2026 Lucide Icons and Contributors"));
        assert!(licence.contains("The MIT License (MIT)"));
        assert!(licence.contains("Copyright (c) 2013-present Cole Bemis"));
        // The icons Lucide's licence lists as derived from Feather, under the MIT licence.
        let feather: Vec<&str> = licence
            .split("derived from the Feather project:")
            .nth(1)
            .and_then(|rest| rest.trim().lines().next())
            .unwrap()
            .split(", ")
            .collect();
        for icon in Icon::ALL {
            let first = icon.source().lines().next().unwrap();
            match icon.lucide() {
                Some(lucide) => {
                    let licence = if feather.contains(&lucide) {
                        "itself derived from Feather, MIT licence"
                    } else {
                        "ISC licence"
                    };
                    let notice = format!(
                        r#"<!-- Derived from Lucide's "{lucide}" (https://lucide.dev), {licence}"#
                    );
                    assert!(first.starts_with(&notice), "{}.svg: {first}", icon.name());
                }
                None => assert!(!icon.source().contains("Lucide"), "{}", icon.name()),
            }
        }
    }

    /// The icons Lucide's licence lists as derived from Feather, under the MIT licence.
    fn from_feather() -> Vec<&'static str> {
        include_str!("../assets/icons/LICENSE-Lucide")
            .split("derived from the Feather project:")
            .nth(1)
            .and_then(|rest| rest.trim().lines().next())
            .unwrap()
            .split(", ")
            .collect()
    }

    /// A list as the credits write one, `a, b and c`, its words split apart.
    fn listed(text: &str) -> BTreeSet<String> {
        text.trim_end_matches('.')
            .replace(" and ", ", ")
            .split(", ")
            .map(str::to_owned)
            .collect()
    }

    /// The icons' README names each icon once with its source, as `Icon::file` maps it:
    /// drawn for Apertrust, or the Lucide icon its file says, from Feather where it is.
    #[test]
    fn the_icons_readme_names_each_icon_s_source_as_its_file_does() {
        let readme = include_str!("../assets/icons/README.md");
        let feather = from_feather();
        let quoted = |text: &str| -> Vec<String> {
            text.split('`')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect()
        };
        let mut named = Vec::new();
        for row in readme.lines().filter(|l| l.starts_with("| `")) {
            let cells: Vec<&str> = row.split('|').map(str::trim).collect();
            let (icons, source) = (cells[1], cells[2]);
            let mut names = quoted(icons);
            if icons.contains("their filled forms") {
                let filled: Vec<String> = names.iter().map(|n| format!("{n}-filled")).collect();
                names.extend(filled);
            }
            let sources = quoted(source);
            for (i, name) in names.iter().enumerate() {
                let icon = Icon::ALL
                    .into_iter()
                    .find(|icon| icon.name() == name)
                    .unwrap_or_else(|| panic!("README names no icon {name}"));
                if source.starts_with("Drawn for Apertrust") {
                    assert_eq!(icon.lucide(), None, "{name}");
                    continue;
                }
                let lucide = if sources.len() == 1 {
                    &sources[0]
                } else {
                    &sources[i % sources.len()]
                };
                assert_eq!(icon.lucide(), Some(lucide.as_str()), "{name}");
                let mit = feather.contains(&lucide.as_str());
                assert_eq!(source.contains("from Feather"), mit, "{name}");
            }
            named.extend(names);
        }
        named.sort();
        let mut all: Vec<String> = Icon::ALL.iter().map(|i| i.name().to_owned()).collect();
        all.sort();
        assert_eq!(named, all, "each icon once");
    }

    /// CREDITS.md, which About shows, lists every Lucide icon Apertrust's are derived from
    /// under its licence, and every icon drawn for Apertrust.
    #[test]
    fn the_credits_list_the_icons_as_their_files_name_their_sources() {
        let credits = include_str!("../../../CREDITS.md");
        let line = |start: &str| {
            let line = credits
                .lines()
                .find(|l| l.starts_with(start))
                .unwrap_or_else(|| panic!("CREDITS.md has no line {start}"));
            line[start.len()..]
                .trim_start_matches("Lucide's ")
                .to_owned()
        };
        let feather = from_feather();
        let lucide: BTreeSet<&str> = Icon::ALL.iter().filter_map(|i| i.lucide()).collect();
        let (mit, isc): (BTreeSet<&str>, BTreeSet<&str>) =
            lucide.into_iter().partition(|name| feather.contains(name));
        let owned = |set: BTreeSet<&str>| set.into_iter().map(str::to_owned).collect();
        assert_eq!(
            listed(&line("- Derived from Lucide, under the ISC licence: ")),
            owned(isc)
        );
        assert_eq!(
            listed(&line(
                "- Derived from Lucide's icons from Feather, under the MIT licence: "
            )),
            owned(mit)
        );
        let drawn = line("- Drawn for Apertrust: ");
        let (plain, filled) = drawn
            .split_once(", with the filled forms of ")
            .unwrap_or((drawn.as_str(), ""));
        let mut listed_drawn = listed(plain);
        if !filled.is_empty() {
            listed_drawn.extend(listed(filled).into_iter().map(|n| format!("{n}-filled")));
        }
        let ours: BTreeSet<String> = Icon::ALL
            .iter()
            .filter(|i| i.lucide().is_none())
            .map(|i| i.name().to_owned())
            .collect();
        assert_eq!(listed_drawn, ours);
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<_> = Icon::ALL.iter().map(|i| i.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Icon::ALL.len());
    }
}
