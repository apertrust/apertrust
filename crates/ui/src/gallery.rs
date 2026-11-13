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

//! The gallery: every token, icon and, as they arrive, every component, in every theme
//! (`docs/gui.md`, Components). A component is built here before any panel uses it, and
//! the snapshot tests photograph the gallery. It is a tool for developers, so its own
//! labels are not translated.

use std::ops::RangeInclusive;

use egui::emath::GuiRounding as _;
use egui::{Align2, Color32, FontFamily, FontId, Frame, RichText, Sense, Stroke, Ui, pos2, vec2};

use crate::components::{
    ActionRow, Available, Axis, Button, ButtonState, CellPicture, CellRating, CellStatus,
    CentralBar, CentralBarAction, Checkbox, Choice, ChoiceAction, ChoiceStyle, Clipped,
    CollapsibleGroup, ColourLabel, ColourLabels, Curve, CurveAction, Equalizer, EqualizerAction,
    FileManager, GridCell, GridPanel, GridPanelAction, Histogram, HistogramBins, IconButton, Job,
    JobFailure, JobMessage, JobPhase, Jobs, Margin, Mark, Marker, MaskHistogram, PanelBody,
    PanelEdge, PanelEdgeAction, PanelTab, Patch, Patches, PhotoCaption, PhotoFrame, PickerButton,
    PickerMarks, PickerPlace, Quad, Rail, RailLine, RailNode, SaveState, Scale, Scope, ScopeView,
    Section, SectionAction, Segmented, SliderAction, SliderRow, Swatch, SwatchFill, Swatches, TabBar,
    TabBarAction, Thumb, Toggle, ToggleRow, ToolButton, TopPanel, TopPanelAction, Track,
    VectorscopeBins, WaveformBins, Zoom, group_label,
};
use crate::fonts::{MEDIUM, SEMIBOLD};
use crate::i18n::{Args, Locale, Translator};
use crate::overlay::Overlay;
use crate::popup::{self, Answer, Entry, Item, Request, Submenu};
use crate::theme::{self, Builtin, Theme, contrast};
use crate::{Icon, set_locale, set_theme};

/// Text that exercises the typeface: English, Ukrainian and Polish, a signed number with
/// the minus sign (U+2212) and one with the decimal comma.
const SAMPLE: &str = "Exposure · Експозиція · Ekspozycja  −1.25 EV  +0,50";

/// What the gallery shows: the theme and the language the developer picked.
pub struct Gallery {
    pub theme: Builtin,
    pub locale: Locale,
    /// One translator for each locale, for the translation table.
    translators: Vec<Translator>,
    /// The controls' values: the gallery stands in for the session, applying what the
    /// components report.
    demo: Demo,
}

impl Gallery {
    pub fn new(theme: Builtin) -> Self {
        Gallery {
            theme,
            locale: Locale::default(),
            translators: Locale::ALL.into_iter().map(Translator::new).collect(),
            demo: Demo::default(),
        }
    }

    /// Opens the ☰'s menu on the first frame, with View expanded, as the snapshot tests
    /// photograph it. The gallery run by hand opens it only when the ☰ is pressed.
    pub fn with_menu_open(mut self) -> Self {
        self.demo.open_menu = true;
        self
    }

    /// Draws the gallery's side panel alone, `width` points wide, as the snapshot tests
    /// photograph it at its narrowest.
    pub fn show_side_panel(&mut self, ui: &mut Ui, width: f32) {
        let t = theme::theme(ui.ctx());
        self.demo.panel_width = Some(width);
        side_panel(ui, &t, &mut self.demo);
    }

    /// Draws the Slider markers section alone, as the snapshot tests photograph each shape:
    /// Exposure at its minimum, its default and its maximum, and a row they drag.
    pub fn show_markers(&mut self, ui: &mut Ui) {
        slider_markers(ui, &mut self.demo);
    }

    /// Draws the On-photo overlays section alone, as the snapshot tests photograph it.
    pub fn show_overlays(&mut self, ui: &mut Ui) {
        let t = theme::theme(ui.ctx());
        overlays(ui, &t);
    }

    /// Draws the Pickers section alone, as the snapshot tests photograph it.
    pub fn show_pickers(&mut self, ui: &mut Ui) {
        let t = theme::theme(ui.ctx());
        pickers(ui, &t);
    }

    /// Draws the gallery on the panel's colour, as a page of its own.
    pub fn show(&mut self, ui: &mut Ui) {
        let t = theme::theme(ui.ctx());
        egui::Frame::new()
            .fill(t.colors.panel)
            .inner_margin(t.spacing.l)
            .show(ui, |ui| self.page(ui));
    }

    fn page(&mut self, ui: &mut Ui) {
        let t = theme::theme(ui.ctx());
        ui.horizontal(|ui| {
            ui.label("Theme");
            for which in Builtin::ALL {
                if ui
                    .selectable_label(self.theme == which, which.id())
                    .clicked()
                {
                    self.theme = which;
                    set_theme(ui.ctx(), Theme::builtin(which));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Language");
            for locale in Locale::ALL {
                if ui
                    .selectable_label(self.locale == locale, locale.native_name())
                    .clicked()
                {
                    self.locale = locale;
                    set_locale(ui.ctx(), locale);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Marker");
            let current = crate::components::marker(ui.ctx());
            for (which, id) in <Marker as apertrust_base::settings::Choice>::OPTIONS {
                if ui.selectable_label(current == *which, *id).clicked() {
                    crate::components::set_marker(ui.ctx(), *which);
                }
            }
        });
        checks(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Checkboxes, text buttons and icon buttons");
        button_states(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Controls");
        controls(ui, &t, &mut self.demo);
        ui.add_space(t.spacing.l);
        heading(ui, "Slider markers");
        slider_markers(ui, &mut self.demo);
        ui.add_space(t.spacing.l);
        heading(ui, "Side panel");
        side_panel(ui, &t, &mut self.demo);
        ui.add_space(t.spacing.l);
        scopes_in_use(ui, &t, &mut self.demo);
        ui.add_space(t.spacing.l);
        heading(ui, "Shell");
        shell(ui, &t, &mut self.demo);
        ui.add_space(t.spacing.l);
        heading(ui, "Grid cells");
        grid_cells(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "On-photo overlays");
        overlays(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Pickers");
        pickers(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Timeline rail");
        rails(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Jobs");
        jobs_section(ui, &t, &mut self.demo);
        ui.add_space(t.spacing.l);
        heading(ui, "Colours");
        colours(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Type");
        typography(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Icons");
        icons(ui, &t);
        ui.add_space(t.spacing.l);
        heading(ui, "Translation");
        translation(ui, &t, &self.translators);
        ui.add_space(t.spacing.l);
        heading(ui, "Spacing");
        spacing(ui, &t);
    }
}

/// Each state of the three pressable controls side by side, pinned with their `state`
/// (`docs/gui.md`, Checkboxes, text buttons and icon buttons): a column a state, a row a
/// control, then an icon toggle's own looks. The live ones are under Controls. Short words,
/// so the five states fit the snapshots' width.
fn button_states(ui: &mut Ui, t: &Theme) {
    const COLUMN: f32 = 84.0;
    let label = |ui: &mut Ui, text: &str| {
        ui.allocate_ui(vec2(COLUMN, t.sizes.control), |ui| {
            ui.set_width(COLUMN);
            ui.label(RichText::new(text).color(t.colors.text_secondary));
        });
    };
    let row = |ui: &mut Ui, name: &str, cell: &mut dyn FnMut(&mut Ui, ButtonState)| {
        ui.push_id(name, |ui| {
            ui.horizontal(|ui| {
                label(ui, name);
                for state in ButtonState::ALL {
                    ui.allocate_ui(vec2(COLUMN, t.sizes.top_bar - 14.0), |ui| {
                        ui.set_width(COLUMN);
                        ui.push_id(state, |ui| cell(ui, state));
                    });
                }
            });
        });
    };
    ui.horizontal(|ui| {
        label(ui, "");
        for state in ButtonState::ALL {
            label(ui, &format!("{state:?}"));
        }
    });
    for ticked in [false, true] {
        let name = if ticked { "Ticked" } else { "Unticked" };
        row(ui, name, &mut |ui, state| {
            let mut on = ticked;
            Checkbox::new(&mut on, "Guides").state(state).show(ui);
        });
    }
    row(ui, "Button", &mut |ui, state| {
        Button::new("Close").state(state).show(ui);
    });
    row(ui, "Primary", &mut |ui, state| {
        Button::new("Export").primary(true).state(state).show(ui);
    });
    row(ui, "Quiet", &mut |ui, state| {
        ToolButton::new("Reset")
            .icon(Icon::Reset)
            .state(state)
            .show(ui);
    });
    row(ui, "Icon button", &mut |ui, state| {
        IconButton::new(Icon::Reset, "Reset Exposure")
            .state(state)
            .show(ui);
    });
    // A toggle: off, on with its filled form, and on under the pointer; a picker on, its
    // filled pipette, as a button whose menu is open is drawn, never on a plate
    // (decision 81).
    ui.horizontal(|ui| {
        label(ui, "Icon toggle");
        let toggles = [
            ("Guides, off", Icon::Grid, false, ButtonState::Rest),
            ("Guides, on", Icon::Grid, true, ButtonState::Rest),
            ("Guides, on, hover", Icon::Grid, true, ButtonState::Hover),
            ("Picker, on", Icon::Pick, true, ButtonState::Rest),
        ];
        for (name, icon, on, state) in toggles {
            ui.allocate_ui(vec2(COLUMN, t.sizes.top_bar - 14.0), |ui| {
                ui.set_width(COLUMN);
                IconButton::new(icon, name)
                    .size(28.0)
                    .toggled(on)
                    .state(state)
                    .show(ui);
            });
        }
    });
    ui.label(
        RichText::new(
            "Icon toggle: off; on, its filled form; on under the pointer; a picker on, its \
             filled form too. No plate in any state.",
        )
        .color(t.colors.text_secondary),
    );
}

/// Exposure, −4 to +4 EV, at its minimum, its default and its maximum, the marker in use
/// whole and flush at the track's ends, and once more for the tests to drag.
fn slider_markers(ui: &mut Ui, demo: &mut Demo) {
    let names = [
        "Exposure, at its minimum",
        "Exposure, at its default",
        "Exposure, at its maximum",
        "Exposure, dragged",
    ];
    // Ids of their own, apart from the controls' Exposure.
    ui.push_id("slider markers", |ui| {
        for (value, name) in demo.markers.iter_mut().zip(names) {
            let action = SliderRow::new("Exposure", *value, -4.0..=4.0)
                .name(name)
                .step(0.01)
                .decimals(2)
                .unit(EV)
                .show(ui);
            if let Some(SliderAction::Set { value: set, .. }) = action {
                *value = set;
            }
        }
    });
}

/// The grid cell's states, as it draws them at rest, with a sample picture or with none:
/// plain, selected, active, edited, with a status mark and with a long name, each with a
/// sample rating and labels, so the whole caption shows; then with labels off and with
/// square thumbnails.
fn grid_cells(ui: &mut Ui, t: &Theme) {
    use ColourLabel::*;
    let ctx = ui.ctx().clone();
    let (wide, tall) = (sample_picture(&ctx, true), sample_picture(&ctx, false));
    let rating = |stars, rejected, labels: &[ColourLabel]| {
        Some(CellRating {
            stars,
            rejected,
            labels: ColourLabels::of(labels.iter().copied()),
        })
    };
    ui.spacing_mut().item_spacing = vec2(t.spacing.m, t.spacing.m);
    // Rows of cells from the top, so cells with and without captions line up.
    let rows = egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true);
    ui.with_layout(rows, |ui| {
        let cell = |name| GridCell::new(name);
        cell("IMG_2041.ARW")
            .picture(Some(wide))
            .rating(rating(3, false, &[Red, Yellow, Green]))
            .show(ui);
        cell("IMG_2042.ARW")
            .picture(Some(tall))
            .selected(true)
            .rating(rating(1, false, &[Red]))
            .show(ui);
        cell("IMG_2043.ARW")
            .picture(Some(tall))
            .active(true)
            .selected(true)
            .edited(true)
            .rating(rating(2, false, &[Blue, Purple]))
            .show(ui);
        cell("IMG_2044.CR3")
            .picture(Some(wide))
            .status(Some(CellStatus::ViewOnly))
            .rating(rating(4, true, &[]))
            .show(ui);
        cell("IMG_2045.NEF")
            .status(Some(CellStatus::Unreadable("not an Apertrust edit")))
            .rating(rating(0, false, &[]))
            .show(ui);
        cell("Wedding_2026-10-31_ceremony_0042.DNG")
            .picture(Some(wide))
            .rating(rating(5, false, &[Green]))
            .show(ui);
    });
    ui.with_layout(rows, |ui| {
        let cell = |name| GridCell::new(name);
        // Without a rating, as the library has none yet: the name alone.
        cell("IMG_2046.ARW").picture(Some(wide)).show(ui);
        // Labels off: the picture alone, the corner marks kept.
        cell("IMG_2047.ARW")
            .picture(Some(tall))
            .edited(true)
            .labels(false)
            .active(true)
            .show(ui);
        // Square thumbnails: each fills its square, the ring round the square.
        cell("IMG_2048.ARW")
            .picture(Some(wide))
            .square(true)
            .selected(true)
            .show(ui);
        cell("IMG_2049.ARW")
            .picture(Some(tall))
            .square(true)
            .edited(true)
            .show(ui);
        // Small: the name alone, cut in its middle.
        cell("P1044066.RW2")
            .side(84.0)
            .picture(Some(wide))
            .rating(rating(3, false, &[Red]))
            .show(ui);
    });
}

/// A sample thumbnail for the gallery's cells, 3:2 or 2:3: a sky over a field, made once.
fn sample_picture(ctx: &egui::Context, landscape: bool) -> CellPicture {
    let id = egui::Id::unique(if landscape {
        "gallery wide picture"
    } else {
        "gallery tall picture"
    });
    let handle = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id));
    let handle = handle.unwrap_or_else(|| {
        let [w, h] = if landscape { [30, 20] } else { [20, 30] };
        let pixels = (0..w * h)
            .map(|i| {
                if i / w < h * 3 / 5 {
                    Color32::from_rgb(122, 147, 168)
                } else {
                    Color32::from_rgb(90, 107, 78)
                }
            })
            .collect();
        let image = egui::ColorImage::new([w, h], pixels);
        let handle = ctx.load_texture("gallery sample", image, egui::TextureOptions::NEAREST);
        ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
        handle
    });
    CellPicture {
        texture: handle.id(),
        size: handle.size_vec2(),
    }
}

/// The on-photo overlay's primitives (`docs/gui.md`, On-photo overlays), each row over a
/// light, a dark and a busy photo made here: the line's three weights, the handles, the
/// point marks, a point's centre once a frame pixel spans 10 points, the secondary weight
/// with labels, and the focus ring.
fn overlays(ui: &mut Ui, t: &Theme) {
    use crate::overlay::{Mark, Point, Weight};

    let rows: [(&str, bool, OverPhoto); 6] = [
        ("The line: normal, selected, secondary", false, |o, _, _| {
            for (weight, y) in Weight::ALL.into_iter().zip([0.25, 0.5, 0.75]) {
                o.stroke(&Mark::new(weight).line([pos2(0.08, y), pos2(0.92, y)]));
            }
        }),
        (
            "Handles: an area's squares, circles; normal and selected",
            false,
            |o, _, t| {
                let area = egui::Rect::from_min_max(pos2(0.1, 0.2), pos2(0.55, 0.8));
                o.stroke(&Mark::new(Weight::Normal).rect(area).corners(area));
                let radius = t.overlay.handle / 2.0;
                for (weight, x) in [(Weight::Normal, 0.72), (Weight::Selected, 0.88)] {
                    o.stroke(&Mark::new(weight).circle(pos2(x, 0.3), radius));
                    o.stroke(&Mark::new(weight).square(pos2(x, 0.72)));
                }
            },
        ),
        (
            "Points: the active picker, a stored sample, the one selected",
            false,
            |o, photo, _| {
                let marks = [
                    (0.12, Point::Active, Weight::Normal),
                    (0.34, Point::Stored, Weight::Normal),
                    (0.76, Point::Stored, Weight::Selected),
                ];
                for (x, point, weight) in marks {
                    let at = pos2(x, 0.5);
                    o.point(at, point, photo.sample(at), weight);
                }
            },
        ),
        (
            "Zoomed in, a frame pixel 10 points across: the pixel sampled",
            true,
            |o, photo, _| {
                let at = pos2(6.5 / 18.0, 5.5 / 11.0);
                o.point(at, Point::Active, photo.sample(at), Weight::Normal);
                let area = egui::Rect::from_min_max(
                    pos2(11.0 / 18.0, 2.0 / 11.0),
                    pos2(16.0 / 18.0, 8.0 / 11.0),
                );
                o.stroke(&Mark::new(Weight::Secondary).rect(area));
            },
        ),
        ("Labels over secondary marks", false, |o, photo, t| {
            let area = egui::Rect::from_min_max(pos2(0.08, 0.3), pos2(0.42, 0.78));
            o.stroke(&Mark::new(Weight::Secondary).rect(area));
            let above = vec2(0.0, -t.spacing.xs);
            o.label(area.left_top(), above, Align2::LEFT_BOTTOM, "1");
            let at = pos2(0.7, 0.5);
            o.point(at, Point::Stored, photo.sample(at), Weight::Secondary);
            let beside = vec2(
                o.reach(Point::Stored, Weight::Secondary) + t.spacing.xs,
                0.0,
            );
            o.label(at, beside, Align2::LEFT_BOTTOM, "2");
        }),
        (
            "The focus ring: round a focused area, round a focused point",
            false,
            |o, photo, _| {
                let area = egui::Rect::from_min_max(pos2(0.1, 0.25), pos2(0.5, 0.75));
                o.stroke(&Mark::new(Weight::Normal).rect(area).corners(area));
                o.focus_rect(area, Weight::Normal);
                let at = pos2(0.76, 0.5);
                o.point(at, Point::Active, photo.sample(at), Weight::Normal);
                o.focus_circle(at, o.reach(Point::Active, Weight::Normal), Weight::Normal);
            },
        ),
    ];
    over_photos(ui, t, &rows);
}

/// How a row of the overlay sections draws over each photo.
type OverPhoto = fn(&Overlay, &OverlayPhoto, &Theme);

/// `rows` of marks, each over a light, a dark and a busy photo, under the photos' names: a
/// row's title, whether its photos are zoomed in, and what it draws.
fn over_photos(ui: &mut Ui, t: &Theme, rows: &[(&str, bool, OverPhoto)]) {
    let ctx = ui.ctx().clone();
    ui.horizontal(|ui| {
        for ground in ["Light", "Dark", "Busy"] {
            let size = vec2(OVERLAY_PHOTO.x, t.sizes.row);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter().text(
                rect.left_center(),
                Align2::LEFT_CENTER,
                ground,
                FontId::proportional(t.text.body),
                t.colors.text_secondary,
            );
        }
    });
    for &(title, zoomed, draw) in rows {
        ui.label(title);
        ui.horizontal(|ui| {
            for ground in Ground::ALL {
                let photo = overlay_photo(&ctx, ground, zoomed);
                let (rect, _) = ui.allocate_exact_size(OVERLAY_PHOTO, Sense::hover());
                let rect = rect.round_to_pixels(ui.pixels_per_point());
                let uv = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                ui.painter()
                    .image(photo.texture.id(), rect, uv, Color32::WHITE);
                let zoom = OVERLAY_PHOTO.x / photo.size[0] as f32;
                draw(&Overlay::new(ui.painter(), rect, zoom), &photo, t);
            }
        });
        ui.add_space(t.spacing.xs);
    }
}

/// The picker button's states (`docs/pickers.md`, The picker button), pinned as the
/// buttons' strip pins them, then the active picker's marks over a light, a dark and a busy
/// photo: an area with its handles, a point with its sampled colour, and each focused.
fn pickers(ui: &mut Ui, t: &Theme) {
    const COLUMN: f32 = 84.0;
    let opening = {
        let mut args = Args::new();
        args.set("name", "IMG_2041");
        crate::translator(ui.ctx()).text_with("controls-opening", Some(&args))
    };
    let states: [(&str, bool, ButtonState); 6] = [
        ("Off", false, ButtonState::Rest),
        ("Hover", false, ButtonState::Hover),
        ("Held", false, ButtonState::Pressed),
        ("On", true, ButtonState::Rest),
        ("Focus", false, ButtonState::Focus),
        ("Disabled", false, ButtonState::Disabled),
    ];
    ui.push_id("pickers", |ui| {
        ui.horizontal(|ui| {
            for (name, on, state) in states {
                ui.allocate_ui(vec2(COLUMN, t.sizes.top_bar - 14.0), |ui| {
                    ui.set_width(COLUMN);
                    ui.horizontal(|ui| {
                        let full = format!("Exposure: pick from the photo, {name}");
                        let mut button = PickerButton::new(&full, on).state(state);
                        if state == ButtonState::Disabled {
                            button = button.disabled(&opening);
                        }
                        button.show(ui);
                        ui.label(RichText::new(name).color(t.colors.text_secondary));
                    });
                });
            }
        });
    });
    ui.label(
        RichText::new(
            "18 by 18, as in a slider's quad, with no plate: the filled pipette while on, \
             dimmed while disabled with its tooltip saying why.",
        )
        .color(t.colors.text_secondary),
    );
    // Exposure's area exposure mapping before a pick, and after one at L 45.2 %.
    ui.push_id("patches", |ui| {
        let target = Patch {
            heading: "Target",
            colour: Some(Color32::from_gray(119)),
            tip: "The desired target exposure after mapping.",
            readout: None,
        };
        let lch = "These LCh coordinates are computed from CIE Lab 1976 coordinates.";
        for (colour, readout) in [
            (None, "L: N/A"),
            (Some(Color32::from_gray(108)), "L: 45.2%"),
        ] {
            Patches::new(
                Patch {
                    heading: "Input",
                    colour,
                    tip: "The input colour that should be mapped to the target.",
                    readout: Some((readout, lch)),
                },
                target,
            )
            .show(ui);
        }
    });
    ui.label(
        RichText::new(
            "Patches: the input before a pick, outlined, and after one, beside the target.",
        )
        .color(t.colors.text_secondary),
    );
    ui.add_space(t.spacing.xs);
    let marks: [(&str, bool, OverPhoto); 4] = [
        (
            "The active area, a square handle on each corner",
            false,
            |o, _, _| {
                PickerMarks::new(PickerPlace::Area(PICKER_AREA)).paint(o);
            },
        ),
        (
            "The active point, its sampled colour at its centre",
            false,
            |o, photo, _| {
                let at = pos2(0.5, 0.5);
                let colour = photo.sample(at);
                PickerMarks::new(PickerPlace::Point { at, colour }).paint(o);
            },
        ),
        ("The area focused, after a key", false, |o, _, _| {
            PickerMarks::new(PickerPlace::Area(PICKER_AREA))
                .focused(true)
                .paint(o);
        }),
        ("The point focused, after a key", false, |o, photo, _| {
            let at = pos2(0.5, 0.5);
            let colour = photo.sample(at);
            PickerMarks::new(PickerPlace::Point { at, colour })
                .focused(true)
                .paint(o);
        }),
    ];
    over_photos(ui, t, &marks);
}

/// The gallery's active area, in the photo's coordinates.
const PICKER_AREA: egui::Rect = egui::Rect {
    min: pos2(0.2, 0.22),
    max: pos2(0.8, 0.78),
};

/// The overlay section's photos, in points.
const OVERLAY_PHOTO: egui::Vec2 = vec2(180.0, 110.0);

/// What a synthetic photo under the overlays shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Ground {
    /// Bright and smooth: a pale sky to a sunlit wall.
    Light,
    /// Deep shadow, near black.
    Dark,
    /// Noise over stripes of light and dark, with some colour: no part of it plain.
    Busy,
}

impl Ground {
    const ALL: [Ground; 3] = [Ground::Light, Ground::Dark, Ground::Busy];
}

/// A synthetic photo for the overlay section, made in code and kept in the context.
#[derive(Clone)]
struct OverlayPhoto {
    texture: egui::TextureHandle,
    size: [usize; 2],
    pixels: std::sync::Arc<Vec<Color32>>,
}

impl OverlayPhoto {
    /// The colour of the pixel at `at`, in the photo's coordinates: what a picker there
    /// would sample.
    fn sample(&self, at: egui::Pos2) -> Color32 {
        let [w, h] = self.size;
        let x = ((at.x * w as f32) as usize).min(w - 1);
        let y = ((at.y * h as f32) as usize).min(h - 1);
        self.pixels[y * w + x]
    }
}

/// `ground` as a photo of a pixel a point at the section's size, or `zoomed`, of a pixel
/// 10 points across. The same every run: the busy one's noise has a fixed seed.
fn overlay_photo(ctx: &egui::Context, ground: Ground, zoomed: bool) -> OverlayPhoto {
    let id = egui::Id::unique(("gallery overlay photo", ground, zoomed));
    if let Some(photo) = ctx.data(|d| d.get_temp::<OverlayPhoto>(id)) {
        return photo;
    }
    let scale = if zoomed { 10.0 } else { 1.0 };
    let [w, h] = [
        (OVERLAY_PHOTO.x / scale) as usize,
        (OVERLAY_PHOTO.y / scale) as usize,
    ];
    // Grain two points across at the section's size, in four steps of ±42: busy, the same
    // on every machine, and small in the snapshots.
    let block = if zoomed { 1 } else { 2 };
    let grain = |x: usize, y: usize| {
        let mut h = ((x / block) as u32).wrapping_mul(0x9E37_79B9)
            ^ ((y / block) as u32).wrapping_mul(0x85EB_CA6B);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 12;
        ((h >> 30) as f32 - 1.5) * 28.0
    };
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            // Where the pixel is, 0 to 1 across and down, the same at either scale.
            let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            let mix = |a: [f32; 3], b: [f32; 3], t: f32| {
                Color32::from_rgb(
                    (a[0] + (b[0] - a[0]) * t) as u8,
                    (a[1] + (b[1] - a[1]) * t) as u8,
                    (a[2] + (b[2] - a[2]) * t) as u8,
                )
            };
            let pixel = match ground {
                Ground::Light => mix([214.0, 228.0, 240.0], [248.0, 242.0, 230.0], v),
                Ground::Dark => mix([10.0, 12.0, 16.0], [42.0, 34.0, 28.0], u * 0.6 + v * 0.4),
                Ground::Busy => {
                    // Diagonal stripes 14 points wide, light and dark, a warm and a cool.
                    let stripe = ((u * OVERLAY_PHOTO.x + v * OVERLAY_PHOTO.y) / 14.0) as u32 % 4;
                    let base: [f32; 3] = match stripe {
                        0 => [236.0, 232.0, 220.0],
                        1 => [40.0, 44.0, 52.0],
                        2 => [196.0, 120.0, 64.0],
                        _ => [70.0, 110.0, 150.0],
                    };
                    let n = grain(x, y);
                    let at = |c: f32| (c + n).clamp(0.0, 255.0);
                    Color32::from_rgb(at(base[0]) as u8, at(base[1]) as u8, at(base[2]) as u8)
                }
            };
            pixels.push(pixel);
        }
    }
    let image = egui::ColorImage::new([w, h], pixels.clone());
    let texture = ctx.load_texture(
        "gallery overlay photo",
        image,
        egui::TextureOptions::NEAREST,
    );
    let photo = OverlayPhoto {
        texture,
        size: [w, h],
        pixels: std::sync::Arc::new(pixels),
    };
    ctx.data_mut(|d| d.insert_temp(id, photo.clone()));
    photo
}

/// The rail History draws its steps on (`Rail`), as a list of rows with no space between
/// them, so the rail runs through: an undone group, hollow, and an undone step on a
/// dashed rail; the end, the larger filled dot, on the chosen band; a step; Reset's glyph,
/// then Reset at the end, ringed; a group at the end and a group, squares; and a step
/// drawn disabled, where the rail ends.
fn rails(ui: &mut Ui, t: &Theme) {
    use RailLine::{Dashed, Solid};
    let group = |end, undone| RailNode::Group { end, undone };
    let rows: [(&str, RailNode, RailLine, RailLine, bool); 9] = [
        (
            "Group, undone",
            group(false, true),
            RailLine::None,
            Dashed,
            true,
        ),
        ("Undone", RailNode::Undone, Dashed, Dashed, true),
        ("End", RailNode::End, Dashed, Solid, true),
        ("Step", RailNode::Step, Solid, Solid, true),
        ("Reset", RailNode::Reset { end: false }, Solid, Solid, true),
        (
            "Reset, the end",
            RailNode::Reset { end: true },
            Solid,
            Solid,
            true,
        ),
        ("Group, the end", group(true, false), Solid, Solid, true),
        ("Group", group(false, false), Solid, Solid, true),
        ("Disabled", RailNode::End, Solid, RailLine::None, false),
    ];
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for (name, node, above, below, enabled) in rows {
            let (rect, _) = ui.allocate_exact_size(vec2(220.0, t.sizes.panel_line), Sense::hover());
            let painter = ui.painter();
            let end = enabled
                && matches!(
                    node,
                    RailNode::End
                        | RailNode::Reset { end: true }
                        | RailNode::Group { end: true, .. }
                );
            let ground = if end {
                t.colors.list_chosen
            } else {
                t.colors.panel
            };
            painter.rect_filled(rect, if end { t.shape.radius } else { 0.0 }, ground);
            let rail = Rail { node, above, below };
            rail.paint(painter, rect, rect.left() + 12.0, ground, enabled, t);
            let ink = match (end, enabled) {
                (_, false) => t.colors.text_disabled,
                (true, true) => t.colors.text_strong,
                (false, true) => t.colors.text,
            };
            painter.text(
                pos2(rect.left() + 28.0, rect.center().y),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(t.text.body),
                ink,
            );
        }
    });
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).heading());
}

fn checks(ui: &mut Ui, t: &Theme) {
    let issues = t.check();
    if issues.is_empty() {
        ui.label(
            RichText::new("Checks pass: contrast, the menu band (AAA in high contrast) and a neutral surround.").color(t.colors.success),
        );
    }
    for issue in issues {
        ui.label(RichText::new(issue.to_string()).color(t.colors.error));
    }
}

fn colours(ui: &mut Ui, t: &Theme) {
    egui::Grid::new("gallery colours")
        .num_columns(4)
        .spacing(vec2(t.spacing.l, t.spacing.xs))
        .show(ui, |ui| {
            for (name, colour) in t.named_colors() {
                let (rect, _) = ui.allocate_exact_size(vec2(36.0, 18.0), Sense::hover());
                ui.painter().rect(
                    rect,
                    t.shape.radius_small,
                    colour,
                    Stroke::new(t.shape.hairline, t.colors.border),
                    egui::StrokeKind::Inside,
                );
                ui.label(name);
                let [r, g, b, a] = colour.to_srgba_unmultiplied();
                let hex = if a == 255 {
                    format!("#{r:02X}{g:02X}{b:02X}")
                } else {
                    format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
                };
                ui.label(RichText::new(hex).color(t.colors.text_secondary));
                let ratio = if a == 255 {
                    format!("{:.1}:1 on panel", contrast(colour, t.colors.panel))
                } else {
                    String::new()
                };
                ui.label(RichText::new(ratio).color(t.colors.text_secondary));
                ui.end_row();
            }
        });
}

fn typography(ui: &mut Ui, t: &Theme) {
    let families = [
        ("Regular", FontFamily::Proportional),
        ("Medium", FontFamily::Name(MEDIUM.into())),
        ("SemiBold", FontFamily::Name(SEMIBOLD.into())),
    ];
    for (name, size) in [
        ("heading", t.text.heading),
        ("title", t.text.title),
        ("section title", t.text.section_title),
        ("body", t.text.body),
        ("small", t.text.small),
        ("caption", t.text.caption),
    ] {
        for (weight, family) in &families {
            ui.horizontal(|ui| {
                ui.add_sized(
                    vec2(150.0, t.sizes.row),
                    egui::Label::new(
                        RichText::new(format!("{name} {size} {weight}"))
                            .color(t.colors.text_secondary),
                    ),
                );
                ui.label(RichText::new(SAMPLE).font(FontId::new(size, family.clone())));
            });
        }
    }
}

fn icons(ui: &mut Ui, t: &Theme) {
    egui::Grid::new("gallery icons")
        .num_columns(6)
        .spacing(vec2(t.spacing.l, t.spacing.s))
        .show(ui, |ui| {
            for (i, icon) in Icon::ALL.into_iter().enumerate() {
                ui.horizontal(|ui| {
                    for (scale, colour) in [(1.0, t.colors.text), (2.0, t.colors.accent_text)] {
                        let side = icon.size() * scale;
                        let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
                        icon.paint(ui.painter(), rect, colour);
                    }
                    ui.label(RichText::new(icon.name()).color(t.colors.text_secondary));
                });
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });
}

fn spacing(ui: &mut Ui, t: &Theme) {
    let s = &t.spacing;
    for (name, value) in [
        ("xxs", s.xxs),
        ("xs", s.xs),
        ("s", s.s),
        ("m", s.m),
        ("l", s.l),
        ("xl", s.xl),
        ("xxl", s.xxl),
    ] {
        ui.horizontal(|ui| {
            ui.add_sized(
                vec2(60.0, t.sizes.control),
                egui::Label::new(
                    RichText::new(format!("{name} {value}")).color(t.colors.text_secondary),
                ),
            );
            let (rect, _) = ui.allocate_exact_size(vec2(value, 12.0), Sense::hover());
            ui.painter().rect_filled(rect, 0.0, t.colors.accent_text);
        });
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("photo surround").color(t.colors.text_secondary));
        let (rect, _) = ui.allocate_exact_size(vec2(120.0, 40.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, t.shape.radius, t.colors.photo_surround);
        ui.painter().rect_filled(
            rect.shrink2(vec2(30.0, 8.0)),
            0.0,
            Color32::from_rgb(0x9A, 0x84, 0x6B),
        );
    });
}

/// A row of the translation table: its name, and how to make its text in a locale.
type Sample<'a> = (&'a str, &'a dyn Fn(&Translator) -> String);

/// Sample messages and numbers in every locale, side by side.
fn translation(ui: &mut Ui, t: &Theme, translators: &[Translator]) {
    let count = |translator: &Translator, n: f64| {
        let mut args = Args::new();
        args.set("count", n);
        translator.text_with("photo-count", Some(&args))
    };
    // A value with its unit, as a slider writes it at rest.
    let with_unit = |tr: &Translator, unit: &str, value: f64, decimals: usize, signed: bool| {
        let mut args = Args::new();
        args.set("value", tr.format_number(value, decimals, signed));
        tr.text_with(unit, Some(&args))
    };
    let samples: [Sample; 12] = [
        ("tab-base", &|tr| tr.text("tab-base")),
        ("action-copy-edit", &|tr| tr.text("action-copy-edit")),
        ("theme-high-contrast", &|tr| tr.text("theme-high-contrast")),
        ("photo-count 1", &|tr| count(tr, 1.0)),
        ("photo-count 3", &|tr| count(tr, 3.0)),
        ("photo-count 5", &|tr| count(tr, 5.0)),
        ("photo-count 1.5", &|tr| count(tr, 1.5)),
        ("photo-count 12345", &|tr| count(tr, 12345.0)),
        ("value −1.25", &|tr| tr.format_number(-1.25, 2, true)),
        ("value 5600 K", &|tr| tr.format_number(5600.0, 0, false)),
        ("unit-ev-value 0, signed", &|tr| {
            with_unit(tr, EV, 0.0, 2, true)
        }),
        ("unit-percent-value 25", &|tr| {
            with_unit(tr, PERCENT, 25.0, 2, false)
        }),
    ];
    let shown: Vec<&Translator> = translators
        .iter()
        .filter(|tr| tr.locale() != Locale::Pseudo)
        .collect();
    egui::Grid::new("gallery translation")
        .num_columns(1 + shown.len())
        .spacing(vec2(t.spacing.l, t.spacing.xs))
        .show(ui, |ui| {
            ui.label("");
            for tr in &shown {
                ui.label(RichText::new(tr.locale().id()).color(t.colors.text_secondary));
            }
            ui.end_row();
            for (name, sample) in samples {
                ui.label(RichText::new(name).color(t.colors.text_secondary));
                for tr in &shown {
                    ui.label(sample(tr));
                }
                ui.end_row();
            }
        });
    if let Some(pseudo) = translators.iter().find(|tr| tr.locale() == Locale::Pseudo) {
        ui.add_space(t.spacing.s);
        ui.label(RichText::new("Pseudo-locale").color(t.colors.text_secondary));
        for text in [
            pseudo.text("action-copy-edit"),
            pseudo.text("slider-keep"),
            count(pseudo, 12.0),
        ] {
            ui.label(text);
        }
    }
}

/// A slider's sample state.
#[derive(Clone, Debug)]
struct DemoSlider {
    value: f64,
    default: f64,
    mark: Mark,
}

impl DemoSlider {
    fn new(value: f64, default: f64, mark: Mark) -> Self {
        DemoSlider {
            value,
            default,
            mark,
        }
    }

    fn apply(&mut self, action: SliderAction) {
        match action {
            SliderAction::Set { value, .. } => {
                self.value = value;
                self.mark = Mark::Kept;
            }
            SliderAction::Reset => {
                self.value = self.default;
                self.mark = Mark::None;
            }
            SliderAction::ToggleKeep => {
                self.mark = if self.mark == Mark::Kept {
                    Mark::None
                } else {
                    Mark::Kept
                };
            }
            SliderAction::Quad | SliderAction::Picker(_) => {}
        }
    }
}

/// The controls' sample values, one of each state the wireframe draws.
#[derive(Clone, Debug)]
struct Demo {
    sliders: Vec<DemoSlider>,
    picking: bool,
    scale: usize,
    adaptation: usize,
    /// The Blended Adaptation's kept marker, as a module's choice has one.
    adaptation_mark: Mark,
    /// White balance's Mode, a preset row: none of its options while the light is the
    /// user's own, the box then showing Custom.
    mode: Option<usize>,
    tool: usize,
    camera_bias: bool,
    /// The collapsible groups ([`GROUPS`]): which are open, and their sliders' values,
    /// two to a group.
    groups: [bool; 3],
    group_sliders: Vec<DemoSlider>,
    range: usize,
    last: String,
    /// The curves: Tone curve's luminance curve, the points between its ends, and a
    /// histogram; its red curve, on a logarithmic axis, with levels.
    curve: Vec<(f32, f32)>,
    levels: (f32, f32),
    histogram: Vec<f32>,
    red_curve: Vec<(f32, f32)>,
    red_levels: (f32, f32),
    /// The tone equalizer's bands, −8 EV to +0 EV, and its mask's histograms: one that
    /// fits the bands and one that overflows them at both ends.
    bands: Vec<f32>,
    mask: (Vec<f32>, [f32; 2]),
    overflowing: (Vec<f32>, [f32; 2]),
    /// The side panel: its width (None for the theme's), its tab, whether tabs show names,
    /// which sections are open, Exposure's tab, and which section is peeking.
    panel_width: Option<f32>,
    panel_tab: usize,
    tab_names: bool,
    open: [bool; 7],
    exposure_tab: usize,
    peeking: Option<&'static str>,
    /// The scopes: a gradient measured, and the same half as bright again from a lower black,
    /// clipping at both ends; the pinned section's view, and the five in use.
    gradient: Measured,
    clipping: Measured,
    scopes: ScopeView,
    in_use: [ScopeView; 5],
    /// The shell: whether the ☰'s menu is to open by itself ([`Gallery::with_menu_open`]),
    /// the central bar's toggles and scale, and the grid panel.
    open_menu: bool,
    guides: bool,
    proof: bool,
    zoom: Zoom,
    active: usize,
    selected: Vec<usize>,
    /// The slider markers' rows: Exposure at its minimum, its default, its maximum, and
    /// one for the snapshot tests to drag.
    markers: [f64; 4],
}

impl Default for Demo {
    fn default() -> Self {
        Demo {
            sliders: vec![
                DemoSlider::new(0.42, 0.0, Mark::Auto),
                DemoSlider::new(12.0, 0.0, Mark::Kept),
                DemoSlider::new(0.0, 0.0, Mark::None),
                DemoSlider::new(5600.0, 5600.0, Mark::None),
                DemoSlider::new(215.0, 0.0, Mark::Kept),
                DemoSlider::new(-100.0, 0.0, Mark::None),
                DemoSlider::new(100.0, 0.0, Mark::None),
                DemoSlider::new(18.0, 18.0, Mark::None),
                DemoSlider::new(1600.0, 1600.0, Mark::None),
            ],
            picking: false,
            scale: 0,
            adaptation: 0,
            adaptation_mark: Mark::Kept,
            mode: None,
            tool: 0,
            camera_bias: true,
            groups: [false, false, true],
            group_sliders: vec![
                DemoSlider::new(0.0, 0.0, Mark::None),
                DemoSlider::new(100.0, 100.0, Mark::None),
                DemoSlider::new(1.0, 1.0, Mark::None),
                DemoSlider::new(-0.05, 0.0, Mark::Kept),
                DemoSlider::new(20.0, 0.0, Mark::Kept),
                DemoSlider::new(50.0, 50.0, Mark::None),
            ],
            range: 0,
            last: "nothing yet".to_owned(),
            curve: vec![(0.25, 0.19), (0.72, 0.83)],
            levels: (0.03, 0.97),
            // Two humps, as a photo's luminance might have.
            histogram: (0..96)
                .map(|i| {
                    let x = (i as f32 + 0.5) / 96.0;
                    let hump = |at: f32, width: f32| (-((x - at) / width).powi(2)).exp();
                    hump(0.28, 0.11) + 0.55 * hump(0.62, 0.16) + 0.04
                })
                .collect(),
            red_curve: vec![(0.0, 0.0), (0.08, 0.14), (0.4, 0.52), (1.0, 1.0)],
            red_levels: (0.0, 1.0),
            bands: vec![0.0, 0.15, 0.4, 0.55, 0.45, 0.1, -0.35, -0.7, -0.9],
            mask: mask_histogram(false),
            overflowing: mask_histogram(true),
            panel_width: None,
            panel_tab: 1,
            tab_names: true,
            open: [true, false, false, true, false, false, false],
            exposure_tab: 0,
            peeking: None,
            gradient: Measured::of(gradient),
            clipping: Measured::of(|x, y| gradient(x, y).map(|c| 1.5 * c - 0.05)),
            scopes: ScopeView::default(),
            in_use: [
                ScopeView {
                    scale: Scale::Logarithmic,
                    channels: [true, true, true, true],
                    highlight_clipping: true,
                    ..ScopeView::default()
                },
                ScopeView {
                    scope: Scope::Waveform,
                    ..ScopeView::default()
                },
                ScopeView {
                    scope: Scope::Parade,
                    ..ScopeView::default()
                },
                ScopeView {
                    scope: Scope::Vectorscope,
                    ..ScopeView::default()
                },
                ScopeView {
                    scope: Scope::Waveform,
                    ..ScopeView::default()
                },
            ],
            open_menu: false,
            guides: false,
            proof: false,
            zoom: Zoom::Percent(400.0),
            active: 5,
            selected: vec![5, 6],
            markers: [-4.0, 0.0, 4.0, 1.28],
        }
    }
}

/// A sample slider: its label, full name, range, step, decimal places and unit, the
/// message that writes a value with it. Its sign follows from its range.
type DemoRow = (
    &'static str,
    &'static str,
    RangeInclusive<f64>,
    f64,
    usize,
    Option<&'static str>,
);

/// The units the samples are written with, as darktable writes them.
const EV: &str = "unit-ev-value";
const PERCENT: &str = "unit-percent-value";
const DEGREES: &str = "unit-degrees-value";
const KELVIN: &str = "unit-kelvin-value";
const ISO: &str = "unit-iso-value";

/// White balance's modes, and colour calibration's illuminants as darktable names them.
/// Custom is no mode: what Mode's box shows while the light matches none (decision 2).
const WHITE_BALANCE_MODES: [&str; 3] = ["As shot", "Camera reference", "Daylight"];
const ILLUMINANTS: [&str; 4] = [
    "Daylight",
    "Black body",
    "Fluorescent F11 (narrow tri-band)",
    "Custom",
];

fn controls(ui: &mut Ui, t: &Theme, demo: &mut Demo) {
    let rows: [DemoRow; 9] = [
        ("Exposure", "Exposure", -4.0..=4.0, 0.01, 2, Some(EV)),
        (
            "Contrast",
            "Contrast",
            -100.0..=100.0,
            1.0,
            0,
            Some(PERCENT),
        ),
        (
            "Brightness",
            "Brightness",
            -100.0..=100.0,
            1.0,
            0,
            Some(PERCENT),
        ),
        (
            "Temperature",
            "Temperature",
            2000.0..=12000.0,
            50.0,
            0,
            Some(KELVIN),
        ),
        ("Hue", "Shadows hue", 0.0..=360.0, 1.0, 0, Some(DEGREES)),
        (
            "Saturation",
            "Saturation, at its minimum",
            -100.0..=100.0,
            1.0,
            0,
            Some(PERCENT),
        ),
        (
            "Highlights",
            "Highlights, at its maximum",
            -100.0..=100.0,
            1.0,
            0,
            Some(PERCENT),
        ),
        (
            "Pivot",
            "Contrast pivot, disabled",
            1.0..=50.0,
            0.1,
            1,
            Some(PERCENT),
        ),
        (
            "Coarseness",
            "Grain coarseness",
            20.0..=6400.0,
            1.0,
            0,
            Some(ISO),
        ),
    ];
    for (i, (label, name, range, step, decimals, unit)) in rows.into_iter().enumerate() {
        let sample = &demo.sliders[i];
        let mut row = SliderRow::new(label, sample.value, range)
            .name(name)
            .promise("The control's promise: what it changes and what it keeps. 0 = untouched.")
            .default_value(sample.default)
            .step(step)
            .decimals(decimals)
            .mark(sample.mark);
        if let Some(unit) = unit {
            row = row.unit(unit);
        }
        row = match i {
            0 => row.before_match(0.0),
            3 => row.track(Track::temperature()).no_fill().quad(Quad {
                icon: Icon::Pick,
                name: "Pick a neutral on the photo",
                active: demo.picking,
            }),
            4 => row.track(Track::hue()).no_fill(),
            7 => row.no_fill().enabled(false),
            8 => row.no_fill(),
            _ => row,
        };
        if let Some(action) = row.show(ui) {
            if action == SliderAction::Quad {
                demo.picking = !demo.picking;
            }
            demo.sliders[i].apply(action);
            demo.last = format!("{name}: {action:?}");
        }
    }
    ui.label(RichText::new(format!("Last action: {}", demo.last)).color(t.colors.text_secondary));
    ui.add_space(t.spacing.s);
    ui.horizontal(|ui| {
        IconButton::new(Icon::Wand, "Auto").show(ui);
        IconButton::new(Icon::Reset, "Reset Exposure")
            .shortcut("Ctrl+R")
            .show(ui);
        IconButton::new(Icon::Presets, "Exposure presets").show(ui);
        IconButton::new(Icon::Options, "Scopes options").show(ui);
        IconButton::new(Icon::Pick, "Picking")
            .toggled(true)
            .show(ui);
        IconButton::new(Icon::Undo, "Undo, nothing to undo")
            .enabled(false)
            .show(ui);
        ui.add_space(t.spacing.l);
        if let Some(i) = Segmented::new("Scale", &["Linear", "Log"], demo.scale).show(ui) {
            demo.scale = i;
        }
    });
    ui.add_space(t.spacing.s);
    // Dropdown rows, Boxed as everywhere but module panels; then the same rows Blended, as
    // module panels draw them, at the side panel's width under a slider row whose value
    // their options end with, stacked as a section stacks rows.
    ui.push_id("boxed", |ui| choices(ui, t, demo, ChoiceStyle::Boxed));
    ui.add_space(t.spacing.s);
    let column = t.sizes.side_panel - 2.0 * t.spacing.l;
    ui.push_id("blended", |ui| {
        ui.allocate_ui(vec2(column, t.sizes.row), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            SliderRow::new("Exposure", 0.42, -4.0..=4.0)
                .name("Exposure, over the blended rows")
                .decimals(2)
                .unit(EV)
                .show(ui);
            choices(ui, t, demo, ChoiceStyle::Blended);
        });
    });
    ui.add_space(t.spacing.s);
    // A segmented row: its label on a line of its own, the options across the width.
    if let Some(i) = Segmented::new("Retouch tool", &["Heal", "Clone", "AI erase"], demo.tool)
        .label("Tool")
        .show(ui)
    {
        demo.tool = i;
    }
    // A checkbox row, live and disabled, as on a photo that records no camera bias.
    if let Some(on) = ToggleRow::new("Use camera bias", demo.camera_bias)
        .name("Use the camera's exposure bias")
        .promise("Undoes the exposure compensation set on the camera.")
        .show(ui)
    {
        demo.camera_bias = on;
    }
    ToggleRow::new("Use camera bias", true)
        .name("Use the camera's exposure bias, no bias recorded")
        .enabled(false)
        .show(ui);
    // An action row, live, and disabled as an action is until its tool comes.
    if ActionRow::new("Set from above")
        .name("Set the primaries after tone mapping from above")
        .promise("Set parameters to completely reverse primaries modifications.")
        .show(ui)
    {
        demo.last = "Set from above: run".to_owned();
    }
    ActionRow::new("Auto tune levels")
        .promise("Set black and white relative exposure using the selected area.")
        .enabled(false)
        .show(ui);
    groups(ui, demo);
    let colour = |rgb: u32| {
        let [_, r, g, b] = rgb.to_be_bytes();
        SwatchFill::Color(Color32::from_rgb(r, g, b))
    };
    let swatches = [
        Swatch {
            name: "Red",
            fill: colour(0xD9534F),
        },
        Swatch {
            name: "Orange",
            fill: colour(0xD98A4F),
        },
        Swatch {
            name: "Yellow",
            fill: colour(0xD9B44F),
        },
        Swatch {
            name: "Green",
            fill: colour(0x8CC152),
        },
        Swatch {
            name: "Aqua",
            fill: colour(0x4FC1C1),
        },
        Swatch {
            name: "Blue",
            fill: colour(0x4F7BD9),
        },
        Swatch {
            name: "Purple",
            fill: colour(0x8A5FD1),
        },
        Swatch {
            name: "Magenta",
            fill: colour(0xC25FA8),
        },
        Swatch {
            name: "Custom",
            fill: SwatchFill::Spectrum,
        },
    ];
    if let Some(i) = Swatches::new("Colour range", &swatches, demo.range).show(ui) {
        demo.range = i;
    }
    ui.add_space(t.spacing.s);
    curves(ui, t, demo);
    ui.add_space(t.spacing.l);
    equalizers(ui, t, demo);
}

/// A slider in a collapsible group: its label, full name, range, decimal places and
/// unit, the message that writes a value with it.
type GroupRow = (
    &'static str,
    &'static str,
    RangeInclusive<f64>,
    usize,
    Option<&'static str>,
);

/// Collapsible groups as darktable's modules have them, two sliders each: AgX's Advanced
/// curve parameters and Look, and Lens correction's Manual vignette correction.
/// Dropdown rows in `style`: one listing an option it cannot choose, as darktable's options
/// that are not ported, a preset row with a picker before its option, showing Custom while
/// none of its options is chosen, one disabled, and in narrow columns one whose widest
/// illuminant does not fit beside its label, so the label and the option share the line,
/// each cut at its end, and narrower still share less. Blended, as in a module panel, the
/// controls have the kept marker, the first kept, and the preset row, which stores
/// nothing, has none.
fn choices(ui: &mut Ui, t: &Theme, demo: &mut Demo, style: ChoiceStyle) {
    let blended = style == ChoiceStyle::Blended;
    let marked = |row: Choice<'static>, mark: Mark| {
        if blended { row.mark(mark) } else { row }
    };
    let adaptation = Choice::new(
        "Adaptation",
        &["CAT16", "Bradford", "XYZ scaling", "None"],
        demo.adaptation,
    )
    .name("Chromatic adaptation")
    .promise("How the photo's white is carried to the display's.")
    .disabled_options(&[3], "Coming in a later version")
    .style(style);
    match marked(adaptation, demo.adaptation_mark).show(ui) {
        Some(ChoiceAction::Set(i)) => demo.adaptation = i,
        Some(ChoiceAction::ToggleKeep) => {
            demo.adaptation_mark = if demo.adaptation_mark == Mark::Kept {
                Mark::None
            } else {
                Mark::Kept
            };
        }
        Some(ChoiceAction::Quad) | None => {}
    }
    match Choice::new("Mode", &WHITE_BALANCE_MODES, demo.mode)
        .name("White balance mode")
        .none_chosen("Custom")
        .quad(Quad {
            icon: Icon::Pick,
            name: "Pick a neutral on the photo",
            active: demo.picking,
        })
        .style(style)
        .show(ui)
    {
        Some(ChoiceAction::Set(i)) => demo.mode = Some(i),
        Some(ChoiceAction::Quad) => demo.picking = !demo.picking,
        Some(ChoiceAction::ToggleKeep) | None => {}
    }
    let disabled = Choice::new("Adaptation", &["CAT16", "Bradford", "XYZ scaling"], 1)
        .name("Chromatic adaptation, disabled")
        .enabled(false)
        .style(style);
    marked(disabled, Mark::None).show(ui);
    for (width, name) in [(240.0, "Illuminant"), (160.0, "Illuminant, cut")] {
        ui.allocate_ui(vec2(width, t.sizes.row), |ui| {
            let row = Choice::new("Illuminant", &ILLUMINANTS, 2)
                .name(name)
                .style(style);
            marked(row, Mark::None).show(ui)
        });
    }
}

const GROUPS: [(&str, [GroupRow; 2]); 3] = [
    (
        "Advanced curve parameters",
        [
            (
                "Shoulder start",
                "Shoulder start",
                0.0..=100.0,
                2,
                Some(PERCENT),
            ),
            (
                "Target white",
                "Target white",
                50.0..=100.0,
                2,
                Some(PERCENT),
            ),
        ],
    ),
    (
        "Look",
        [
            ("Slope", "Look slope", 0.0..=2.0, 2, None),
            ("Lift", "Look lift", -0.5..=0.5, 2, None),
        ],
    ),
    (
        "Manual vignette correction",
        [
            (
                "Strength",
                "Manual vignette strength",
                0.0..=100.0,
                0,
                Some(PERCENT),
            ),
            (
                "Radius",
                "Manual vignette radius",
                0.0..=100.0,
                0,
                Some(PERCENT),
            ),
        ],
    ),
];

/// A static group label over its row, then collapsible groups: one closed, one closed with
/// a control in it set, which shows the dot, one open, and one disabled, with nothing
/// behind it yet. A group counts as set the
/// sliders in it off their defaults, as the app reads them from the session's mirror.
fn groups(ui: &mut Ui, demo: &mut Demo) {
    group_label(ui, "Basic curve parameters");
    SliderRow::new("Contrast", 3.0, 0.1..=5.0)
        .name("AgX contrast")
        .default_value(3.0)
        .decimals(2)
        .no_fill()
        .show(ui);
    for (g, (title, rows)) in GROUPS.iter().enumerate() {
        let sliders = &mut demo.group_sliders[2 * g..2 * g + 2];
        let set = sliders.iter().filter(|s| s.value != s.default).count();
        let mut changed = None;
        let toggled = CollapsibleGroup::new(title, demo.groups[g])
            .set(set)
            .show(ui, |ui| {
                for (i, ((label, name, range, decimals, unit), sample)) in
                    rows.iter().zip(sliders.iter()).enumerate()
                {
                    let mut row = SliderRow::new(label, sample.value, range.clone())
                        .name(name)
                        .default_value(sample.default)
                        .decimals(*decimals)
                        .no_fill()
                        .mark(sample.mark);
                    if let Some(unit) = unit {
                        row = row.unit(unit);
                    }
                    let action = row.show(ui);
                    if let Some(action) = action {
                        changed = Some((i, action));
                    }
                }
            });
        if let Some((i, action)) = changed {
            sliders[i].apply(action);
            demo.last = format!("{}: {action:?}", rows[i].1);
        }
        if toggled {
            demo.groups[g] = !demo.groups[g];
            demo.last = format!("{title}: toggled");
        }
    }
    // A group with nothing behind it yet, drawn disabled, as AgX's Show curve is.
    CollapsibleGroup::new("Show curve", false)
        .disabled("Coming in a later version")
        .show(ui, |_| {});
}

/// Tone curve's luminance curve, its handles the curve's ends, over a histogram; its red
/// curve on a logarithmic axis, with levels; and its green curve, disabled.
fn curves(ui: &mut Ui, t: &Theme, demo: &mut Demo) {
    let width = (ui.available_width() - 2.0 * t.spacing.l) / 3.0;
    let green = [(0.0, 0.0), (0.5, 0.42), (1.0, 1.0)];
    let (black, white) = demo.levels;
    let nodes: Vec<(f32, f32)> = std::iter::once((black, 0.0))
        .chain(demo.curve.iter().copied())
        .chain(std::iter::once((white, 1.0)))
        .collect();
    let luma = Smooth::through(&nodes);
    let red = Smooth::through(&demo.red_curve);
    let (luma_action, red_action) = ui
        .horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = t.spacing.l;
            let luma_action = ui
                .allocate_ui(vec2(width, width + t.sizes.row), |ui| {
                    Curve::new("Tone curve, Luma", &demo.curve)
                        .function(&|x| luma.at(x))
                        .ends(black, white)
                        .histogram(&demo.histogram)
                        .max_points(18)
                        .show(ui)
                })
                .inner;
            let red_action = ui
                .allocate_ui(vec2(width, width + t.sizes.row), |ui| {
                    Curve::new("Tone curve, Red", &demo.red_curve)
                        .function(&|x| red.at(x))
                        .levels(demo.red_levels.0, demo.red_levels.1)
                        .axis(Axis::Log(32.0))
                        .max_points(20)
                        .show(ui)
                })
                .inner;
            ui.allocate_ui(vec2(width, width + t.sizes.row), |ui| {
                Curve::new("Tone curve, Green", &green)
                    .levels(0.0, 1.0)
                    .enabled(false)
                    .show(ui)
            });
            (luma_action, red_action)
        })
        .inner;
    for (action, points, levels, name, reset) in [
        (
            luma_action,
            &mut demo.curve,
            &mut demo.levels,
            "Luma",
            vec![],
        ),
        (
            red_action,
            &mut demo.red_curve,
            &mut demo.red_levels,
            "Red",
            vec![(0.0, 0.0), (1.0, 1.0)],
        ),
    ] {
        let Some(action) = action else { continue };
        match action {
            CurveAction::Move { index, x, y, .. } => points[index] = (x, y),
            CurveAction::Add { x, y } => {
                let at = points.partition_point(|p| p.0 < x);
                points.insert(at, (x, y));
            }
            CurveAction::Remove(index) => {
                points.remove(index);
            }
            CurveAction::BlackPoint { value, .. } => levels.0 = value,
            CurveAction::WhitePoint { value, .. } => levels.1 = value,
            CurveAction::Reset => {
                *points = reset;
                *levels = (0.0, 1.0);
            }
        }
        demo.last = format!("Tone curve, {name}: {action:?}");
    }
}

/// A smooth curve through points, monotone between them (Fritsch and Carlson's cubic
/// Hermite): a stand-in for the engine's interpolation, which the curve leaves to its
/// caller.
struct Smooth<'a> {
    points: &'a [(f32, f32)],
    slopes: Vec<f32>,
}

impl<'a> Smooth<'a> {
    fn through(points: &'a [(f32, f32)]) -> Self {
        let secants: Vec<f32> = points
            .windows(2)
            .map(|w| (w[1].1 - w[0].1) / (w[1].0 - w[0].0).max(f32::EPSILON))
            .collect();
        let n = points.len();
        let mut slopes: Vec<f32> = (0..n)
            .map(
                |i| match (i.checked_sub(1).map(|k| secants[k]), secants.get(i)) {
                    (Some(a), Some(&b)) if a * b > 0.0 => (a + b) / 2.0,
                    (Some(_), Some(_)) => 0.0,
                    (Some(a), None) => a,
                    (None, Some(&b)) => b,
                    (None, None) => 0.0,
                },
            )
            .collect();
        for (i, &secant) in secants.iter().enumerate() {
            if secant == 0.0 {
                slopes[i] = 0.0;
                slopes[i + 1] = 0.0;
                continue;
            }
            let (a, b) = (slopes[i] / secant, slopes[i + 1] / secant);
            let length = a * a + b * b;
            if length > 9.0 {
                let scale = 3.0 / length.sqrt();
                slopes[i] = scale * a * secant;
                slopes[i + 1] = scale * b * secant;
            }
        }
        Smooth { points, slopes }
    }

    fn at(&self, x: f32) -> f32 {
        let p = self.points;
        let after = p.partition_point(|q| q.0 <= x);
        if after == 0 || after == p.len() {
            return p
                .get(after.min(p.len().saturating_sub(1)))
                .map_or(x, |q| q.1);
        }
        let ((x0, y0), (x1, y1)) = (p[after - 1], p[after]);
        let h = x1 - x0;
        let s = (x - x0) / h;
        let (s2, s3) = (s * s, s * s * s);
        (2.0 * s3 - 3.0 * s2 + 1.0) * y0
            + (s3 - 2.0 * s2 + s) * h * self.slopes[after - 1]
            + (3.0 * s2 - 2.0 * s3) * y1
            + (s3 - s2) * h * self.slopes[after]
    }
}

/// The tone equalizer's graph of its bands: with nothing measured yet, over a mask that
/// fits the bands, over one that overflows them at both ends, which shows darktable's two
/// warnings, and with no curve fitting the bands, drawn in the warning colour.
fn equalizers(ui: &mut Ui, t: &Theme, demo: &mut Demo) {
    let width = (ui.available_width() - t.spacing.l) / 2.0;
    let names = [
        "−8 EV", "−7 EV", "−6 EV", "−5 EV", "−4 EV", "−3 EV", "−2 EV", "−1 EV", "+0 EV",
    ];
    let curve = stand_in_curve(&demo.bands);
    fn histogram((bins, percentiles): &(Vec<f32>, [f32; 2])) -> MaskHistogram<'_> {
        MaskHistogram {
            bins,
            range: [-8.0, 0.0],
            percentiles: *percentiles,
        }
    }
    let mut actions = Vec::new();
    for row in [[0, 1], [2, 3]] {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = t.spacing.l;
            for k in row {
                let (name, mask) = match k {
                    0 => ("Tone equalizer, nothing measured", None),
                    1 => ("Tone equalizer, its mask", Some(histogram(&demo.mask))),
                    2 => (
                        "Tone equalizer, a mask past both ends",
                        Some(histogram(&demo.overflowing)),
                    ),
                    _ => ("Tone equalizer, no curve fits", None),
                };
                let mut graph = Equalizer::new(name, &demo.bands, &names).histogram(mask);
                graph = if k == 3 {
                    graph.failed(true)
                } else {
                    graph.curve(&curve)
                };
                if let Some(action) = ui.allocate_ui(vec2(width, 0.0), |ui| graph.show(ui)).inner {
                    actions.push(action);
                }
            }
        });
        ui.add_space(t.spacing.l);
    }
    for action in actions {
        match &action {
            EqualizerAction::Move { to, .. } => demo.bands.clone_from(to),
            EqualizerAction::ResetBand(i) => demo.bands[*i] = 0.0,
            EqualizerAction::Reset => demo.bands = vec![0.0; 9],
        }
        demo.last = format!("Tone equalizer: {action:?}");
    }
}

/// A smooth curve near the bands, 256 samples from −8 EV to +0 EV: a Gaussian-weighted
/// mean of them, a stand-in for the engine's fit, which the graph leaves to its caller.
fn stand_in_curve(bands: &[f32]) -> Vec<f32> {
    (0..256)
        .map(|k| {
            let exposure = -8.0 + 8.0 * k as f32 / 255.0;
            let (mut sum, mut weights) = (0.0, 0.0);
            for (i, &band) in bands.iter().enumerate() {
                let d = exposure - (i as f32 - 8.0);
                let weight = (-d * d / 0.8).exp();
                sum += weight * band;
                weights += weight;
            }
            sum / weights
        })
        .collect()
}

/// A sample of the tone equalizer's mask: 256 bins from −8 EV to +0 EV, a photo's tones
/// in three humps, or one whose shadows and highlights pile into the end bins, with its
/// 5th and 95th percentiles in EV.
fn mask_histogram(overflowing: bool) -> (Vec<f32>, [f32; 2]) {
    let hump = |e: f32, at: f32, width: f32| (-(e - at) * (e - at) / (2.0 * width * width)).exp();
    let bins: Vec<f32> = (0..256)
        .map(|k| {
            let e = -8.0 + 8.0 * k as f32 / 255.0;
            // A little grain, as a photo's counts have, from an integer hash: a sine's last
            // bit, which each system's maths library rounds its own way, would move it.
            let hash = (k as u32).wrapping_mul(2_654_435_761) >> 16;
            let grain = 0.9 + 0.2 * hash as f32 / 65_535.0;
            let counts = if overflowing {
                if !(2..=253).contains(&k) {
                    return 1.0;
                }
                0.06 * hump(e, -4.0, 1.2) + 0.4 * (e + 0.3).exp() + 0.4 * (-(e + 7.7)).exp()
            } else {
                0.45 * hump(e, -5.4, 0.9) + 0.8 * hump(e, -3.2, 0.8) + 0.3 * hump(e, -1.6, 0.4)
            };
            counts * grain
        })
        .collect();
    let total: f32 = bins.iter().sum();
    let percentile = |share: f32| {
        let mut sum = 0.0;
        let k = bins
            .iter()
            .position(|&count| {
                sum += count;
                sum >= share * total
            })
            .unwrap_or(255);
        -8.0 + 8.0 * k as f32 / 255.0
    };
    let percentiles = [percentile(0.05), percentile(0.95)];
    (bins, percentiles)
}

/// The side panel as the Base tab shows it: Scopes and Layers pinned, the modules below.
/// Photos and History, with nothing behind them yet, are drawn disabled. Its edge, here
/// at its end as for a panel at the window's start, resizes it.
fn side_panel(page: &mut Ui, t: &Theme, demo: &mut Demo) {
    let ids = [
        (Icon::Folder, "tab-photos", false),
        (Icon::Base, "tab-base", true),
        (Icon::Color, "tab-color", true),
        (Icon::Correct, "tab-correct", true),
        (Icon::Effect, "tab-effect", true),
        (Icon::History, "tab-history", false),
    ];
    let names: Vec<String> = ids
        .iter()
        .map(|(_, id, _)| crate::tr(page.ctx(), id))
        .collect();
    let tabs: Vec<PanelTab> = ids
        .iter()
        .zip(&names)
        .enumerate()
        .map(|(i, ((icon, _, enabled), name))| PanelTab {
            icon: *icon,
            name,
            shortcut: Some(format!("Ctrl+{}", i + 1)),
            enabled: *enabled,
        })
        .collect();
    // The app's widths: down to 240, opening at the theme's 320.
    let widths = 240.0..=page.available_width().clamp(240.0, 600.0);
    let width = demo.panel_width.unwrap_or(t.sizes.side_panel);
    let name = crate::tr(page.ctx(), "panel-width");
    let edge = PanelEdge::new(&name, width, widths).at_start(true);
    let (rect, _) = page.allocate_exact_size(vec2(edge.width(page.ctx()), 560.0), Sense::hover());
    page.painter().rect_filled(rect, 0.0, t.colors.panel);
    // The sections stop short of the edge's line, as the app's panel keeps room for it.
    let inside = rect.with_max_x(rect.right() - t.shape.hairline);
    let mut child = page.new_child(egui::UiBuilder::new().max_rect(inside));
    let ui = &mut child;
    ui.set_clip_rect(inside);
    match TabBar::new(&tabs, demo.panel_tab, demo.tab_names).show(ui) {
        Some(TabBarAction::Select(i)) => demo.panel_tab = i,
        Some(TabBarAction::ShowNames(on)) => demo.tab_names = on,
        Some(TabBarAction::Customise) => demo.last = "Customise tabs".to_owned(),
        None => {}
    }
    ui.spacing_mut().item_spacing.y = 0.0;

    // The sections report; the gallery applies what they report after drawing them. It
    // reads H itself, where the app's actions registry would.
    let (open, exposure_tab) = (demo.open, demo.exposure_tab);
    let peek = ui.input(|i| i.key_down(egui::Key::H)) && !ui.ctx().text_edit_focused();
    let mut pinned_out = Vec::new();
    let mut scrolling_out = Vec::new();
    let (view, gradient) = (demo.scopes, &demo.gradient);
    let mut scope_action = None;
    PanelBody::new()
        .pinned(|ui| {
            let out = Section::new("Scopes", open[0])
                .peek(peek)
                .show(ui, |ui| scope_action = gradient.scopes(view).show(ui));
            pinned_out.push((0, "Scopes", out));
            let out = Section::new("Layers", open[1]).peek(peek).show(ui, |_| {});
            pinned_out.push((1, "Layers", out));
        })
        .show(ui, |ui| {
            let modules: [&'static str; 5] = [
                "White balance",
                "Exposure",
                "High dynamic range",
                "Colour grading",
                "Vignette",
            ];
            for (k, title) in modules.into_iter().enumerate() {
                let i = k + 2;
                let mut section = Section::new(title, open[i]).module().peek(peek);
                if title == "Exposure" {
                    section = section.tabs(&["Basic", "Advanced"], exposure_tab);
                }
                let out = section.show(ui, |ui| {
                    if title == "Exposure" {
                        exposure_body(ui, exposure_tab);
                    }
                });
                scrolling_out.push((i, title, out));
            }
        });
    if let Some(action) = scope_action {
        demo.scopes.apply(action);
        demo.last = format!("Scopes: {action:?}");
    }
    demo.peeking = None;
    for (i, title, out) in pinned_out.into_iter().chain(scrolling_out) {
        for action in out.actions {
            match action {
                SectionAction::Toggle => demo.open[i] = !demo.open[i],
                SectionAction::Tab(tab) => demo.exposure_tab = tab,
                other => demo.last = format!("{title}: {other:?}"),
            }
        }
        if out.peeking {
            demo.peeking = Some(title);
        }
    }
    if let Some(action) = edge.show(page, rect) {
        demo.panel_width = match action {
            PanelEdgeAction::Resize(width) => Some(width),
            PanelEdgeAction::Reset => None,
        };
    }
}

/// The four graphs of a photo that clips at both ends, with their controls shown, as the
/// pointer over them would show them.
///
/// The first two have the clipping overlays to toggle, the next two not yet. The last has
/// nothing measured yet and an app that cannot measure the vectorscope.
fn scopes_in_use(ui: &mut Ui, t: &Theme, demo: &mut Demo) {
    let width = (ui.available_width() - t.spacing.l) / 2.0;
    let measured = &demo.clipping;
    let mut actions = Vec::new();
    for row in [0..2, 2..4, 4..5] {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = t.spacing.l;
            for k in row.clone() {
                let scopes = if k < 4 {
                    measured.scopes(demo.in_use[k]).overlays(k < 2)
                } else {
                    Histogram::new(demo.in_use[k]).available(&[
                        Scope::Histogram,
                        Scope::Waveform,
                        Scope::Parade,
                    ])
                }
                .show_controls(true)
                .id_salt(("scopes in use", k));
                let action = ui.allocate_ui(vec2(width, 0.0), |ui| scopes.show(ui)).inner;
                if let Some(action) = action {
                    actions.push((k, action));
                }
            }
        });
        ui.add_space(t.spacing.l);
    }
    for (k, action) in actions {
        demo.in_use[k].apply(action);
        demo.last = format!("Scopes {}: {action:?}", k + 1);
    }
}

/// The gallery's photo for the scopes: brightness rising across it, from near black to
/// near white, under a cool sky that warms down to sunlit ground. Its red, green and blue
/// at a point from 0 to 1 across and down.
fn gradient(x: f32, y: f32) -> [f32; 3] {
    let value = 0.04 + 0.9 * x.powf(1.4);
    let sky = [0.70, 0.86, 1.0];
    let ground = [1.0, 0.80, 0.55];
    std::array::from_fn(|k| value * (sky[k] + (ground[k] - sky[k]) * y))
}

/// A photo measured as the engine is to measure the shown frame for the scopes: the counts
/// of a histogram, a waveform and a vectorscope, and how much clips.
#[derive(Clone, Debug)]
struct Measured {
    /// Red, green, blue and luminance.
    histogram: [Vec<f32>; 4],
    waveform: [Vec<f32>; 4],
    vectorscope: Vec<f32>,
    /// Each vectorscope cell's hue at full saturation and brightness, as the engine gives
    /// darktable's.
    colours: Vec<Color32>,
    clipped: Clipped,
}

const BINS: usize = 128;
const COLUMNS: usize = 64;
const LEVELS: usize = 64;
const GRID: usize = 48;

impl Measured {
    /// Measures a 1024 × 96 photo whose pixel at a point, 0 to 1 across and down, is `pixel`
    /// before it is clipped to the display.
    fn of(pixel: impl Fn(f32, f32) -> [f32; 3]) -> Measured {
        let (width, height) = (1024, 96);
        let bin = |v: f32, n: usize| ((v * n as f32) as usize).min(n - 1);
        let mut histogram: [Vec<f32>; 4] = std::array::from_fn(|_| vec![0.0; BINS]);
        let mut waveform: [Vec<f32>; 4] = std::array::from_fn(|_| vec![0.0; COLUMNS * LEVELS]);
        let mut vectorscope = vec![0.0; GRID * GRID];
        let (mut shadows, mut highlights) = (0, 0);
        for y in 0..height {
            for x in 0..width {
                let raw = pixel(
                    (x as f32 + 0.5) / width as f32,
                    (y as f32 + 0.5) / height as f32,
                );
                shadows += usize::from(raw.iter().any(|&c| c <= 0.0));
                highlights += usize::from(raw.iter().any(|&c| c >= 1.0));
                let [r, g, b] = raw.map(|c| c.clamp(0.0, 1.0));
                let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                let column = x * COLUMNS / width;
                for (k, value) in [r, g, b, luminance].into_iter().enumerate() {
                    histogram[k][bin(value, BINS)] += 1.0;
                    waveform[k][column * LEVELS + bin(value, LEVELS)] += 1.0;
                }
                // The hue plane, red to the right and green a third of a turn on.
                let (a, c) = (r - (g + b) / 2.0, (g - b) * 3f32.sqrt() / 2.0);
                let cell = bin((1.0 - c) / 2.0, GRID) * GRID + bin((a + 1.0) / 2.0, GRID);
                vectorscope[cell] += 1.0;
            }
        }
        let colours = (0..GRID * GRID)
            .map(|cell| {
                // The hue at the cell's centre, in sixths of a turn from red.
                let centre = |k: usize| 2.0 * (k as f32 + 0.5) / GRID as f32 - 1.0;
                let (a, c) = (centre(cell % GRID), -centre(cell / GRID));
                let hue = c.atan2(a).to_degrees().rem_euclid(360.0) / 60.0;
                let channel = |n: f32| {
                    let k = (n + hue) % 6.0;
                    1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
                };
                let byte = |v: f32| (v * 255.0).round() as u8;
                Color32::from_rgb(byte(channel(5.0)), byte(channel(3.0)), byte(channel(1.0)))
            })
            .collect();
        let pixels = (width * height) as f32;
        Measured {
            histogram,
            waveform,
            vectorscope,
            colours,
            clipped: Clipped {
                shadows: shadows as f32 / pixels,
                highlights: highlights as f32 / pixels,
            },
        }
    }

    fn scopes(&self, view: ScopeView) -> Histogram<'_> {
        let [red, green, blue, luminance] = &self.histogram;
        let [w_red, w_green, w_blue, w_luminance] = &self.waveform;
        Histogram::new(view)
            .histogram(HistogramBins {
                red,
                green,
                blue,
                luminance,
            })
            .waveform(WaveformBins {
                columns: COLUMNS,
                levels: LEVELS,
                red: w_red,
                green: w_green,
                blue: w_blue,
                luminance: w_luminance,
            })
            .vectorscope(VectorscopeBins {
                size: GRID,
                counts: &self.vectorscope,
                colours: &self.colours,
            })
            .clipped(self.clipped)
    }
}

/// Exposure's two tabs, with sample values.
fn exposure_body(ui: &mut Ui, tab: usize) {
    if tab == 0 {
        SliderRow::new("Exposure", 0.42, -4.0..=4.0)
            .decimals(2)
            .unit(EV)
            .show(ui);
        SliderRow::new("Contrast", 12.0, -100.0..=100.0)
            .unit(PERCENT)
            .mark(Mark::Kept)
            .show(ui);
        SliderRow::new("Brightness", 0.0, -100.0..=100.0)
            .unit(PERCENT)
            .show(ui);
    } else {
        group_label(ui, "Contrast");
        SliderRow::new("Pivot", 18.0, 1.0..=50.0)
            .name("Contrast pivot")
            .default_value(18.0)
            .decimals(1)
            .unit(PERCENT)
            .no_fill()
            .show(ui);
    }
}

/// A sample of the menus as the ☰ opens them, with access keys at the top, submenus,
/// keys, check marks, separators and disabled items. The words stand in for the app's.
fn sample_menu(anchor: egui::Rect) -> Request {
    let entries = vec![
        Submenu::new(
            "file",
            "&File",
            vec![
                Item::new("open", "Open…").shortcut("Ctrl+O").into(),
                Submenu::new(
                    "open-recent",
                    "Open Recent",
                    vec![
                        Item::new("recent-1", "Holidays 2026").into(),
                        Item::new("recent-2", "Studio").into(),
                    ],
                )
                .into(),
                Entry::Separator,
                Item::new("export", "Export…")
                    .shortcut("Ctrl+E")
                    .enabled(false)
                    .into(),
                Entry::Separator,
                Item::new("quit", "Quit").shortcut("Ctrl+Q").into(),
            ],
        )
        .into(),
        Submenu::new(
            "edit",
            "&Edit",
            vec![
                Item::new("undo", "Undo").shortcut("Ctrl+Z").into(),
                Item::new("redo", "Redo")
                    .shortcut("Ctrl+Shift+Z")
                    .enabled(false)
                    .into(),
                Entry::Separator,
                Item::new("settings", "Settings…").shortcut("Ctrl+,").into(),
            ],
        )
        .into(),
        Submenu::new(
            "view",
            "&View",
            vec![
                Item::new("guides", "Guides")
                    .shortcut("G")
                    .checked(true)
                    .into(),
                Item::new("proof-margin", "Proof Margin")
                    .checked(false)
                    .into(),
                Item::new("before-after", "Before and After")
                    .shortcut("\\")
                    .checked(false)
                    .enabled(false)
                    .into(),
                Entry::Separator,
                Submenu::new(
                    "interface-size",
                    "Interface Size",
                    vec![
                        Item::new("larger", "Larger").shortcut("Ctrl+=").into(),
                        Item::new("smaller", "Smaller").shortcut("Ctrl+-").into(),
                    ],
                )
                .into(),
                Entry::Separator,
                Item::new("full-screen", "Full Screen")
                    .shortcut("F11")
                    .checked(false)
                    .into(),
            ],
        )
        .into(),
        Submenu::new(
            "help",
            "&Help",
            vec![Item::new("about", "About Apertrust").into()],
        )
        .into(),
    ];
    Request::pull_down(anchor, "Menu", entries)
}

/// The jobs spinner's states, as the top panel draws them: turning while jobs run, and with
/// none running the warning sign where one failed, nothing where none did; then the jobs
/// window's content, newest first, a row in each state the wireframe draws.
fn jobs_section(ui: &mut Ui, t: &Theme, demo: &mut Demo) {
    let mut failed = sample_jobs();
    failed.retain(|job| !job.phase.active());
    ui.horizontal(|ui| {
        for (i, jobs) in [sample_jobs(), failed].iter().enumerate() {
            ui.push_id(("spinner", i), |ui| {
                if let Some(spinner) = Jobs::new(jobs).spinner(ui)
                    && spinner.clicked()
                {
                    demo.last = "Jobs spinner: opened".to_owned();
                }
            });
            ui.add_space(t.spacing.l);
        }
    });
    ui.add_space(t.spacing.m);
    let jobs = sample_jobs();
    let mut asked = None;
    Frame::popup(ui.style())
        .fill(t.colors.panel)
        .inner_margin(t.spacing.m)
        .show(ui, |ui| {
            ui.set_width(380.0);
            asked = Jobs::new(&jobs)
                .file_manager(FileManager::Finder)
                .window(ui);
        });
    if let Some(action) = asked {
        demo.last = format!("Jobs window: {action:?}");
    }
}

/// Sample jobs, as the app would mirror them from the session: the oldest first, a
/// finished export, a failed one, a folder being read with no total, a running export with
/// another waiting.
fn sample_jobs() -> Vec<Job> {
    let folder = "~/Pictures/apertrust_exported";
    let wedding = "~/Pictures/Wedding/apertrust_exported";
    let title = |number: f64, total: f64| {
        JobMessage::new("job-export-photo")
            .number("number", number)
            .number("total", total)
            .text("storage", "disk")
    };
    let export = |id, phase, done: u64, total: u64| {
        Job::new(id, phase, title(done as f64, total as f64))
            .kind("export")
            .progress(done, Some(total))
    };
    // 14:02 on the local clock, whatever the time zone the gallery is drawn in.
    let two = chrono::NaiveDate::from_ymd_opt(2026, 10, 14)
        .and_then(|day| day.and_hms_opt(14, 2, 0))
        .and_then(|at| at.and_local_timezone(chrono::Local).single())
        .map_or(std::time::SystemTime::UNIX_EPOCH, Into::into);
    vec![
        export(1, JobPhase::Finished, 8, 8)
            .destination(folder)
            .outcome(
                JobMessage::new("job-export-finished")
                    .number("written", 8.0)
                    .number("skipped", 0.0)
                    .number("total", 8.0),
            )
            .ended_at(two),
        export(2, JobPhase::Failed, 11, 12)
            .destination(folder)
            .outcome(
                JobMessage::new("job-export-failed-outcome")
                    .number("written", 11.0)
                    .number("total", 12.0),
            )
            .failure(JobFailure {
                item: "IMG_2047.ARW".to_owned(),
                file: "IMG_2047.jpg".to_owned(),
                cause: "the disk is full".to_owned(),
            })
            .retry(true),
        Job::new(
            3,
            JobPhase::Running,
            JobMessage::new("job-folder").text("folder", "2026-10 Lisbon"),
        )
        .kind("folder")
        .progress(120, None)
        .cancellable(true),
        export(4, JobPhase::Running, 3, 12)
            .cancellable(true)
            .detail(
                JobMessage::new("job-export-now")
                    .number("done", 3.0)
                    .number("total", 12.0)
                    .text("photo", "IMG_2044.ARW"),
            )
            .destination(folder),
        export(5, JobPhase::Queued, 0, 5)
            .cancellable(true)
            .detail(
                JobMessage::new("job-export-queued")
                    .number("count", 5.0)
                    .text("folder", wedding),
            )
            .destination(wedding),
    ]
}

/// The shell's parts, with sample state: the top panel, the central bar, the caption and
/// the grid panel.
fn shell(ui: &mut Ui, t: &Theme, demo: &mut Demo) {
    let ctx = ui.ctx().clone();
    let available = Available {
        redo: false,
        apply: false,
        ..Available::default()
    };
    let jobs = sample_jobs();
    if let Some(action) = TopPanel::new()
        .available(available)
        .saved(SaveState::Saved)
        .jobs(Jobs::new(&jobs).file_manager(FileManager::Finder))
        .undo_tip("Undo Exposure: Brightness +8")
        .shortcut(TopPanelAction::Export, "Ctrl+E")
        .shortcut(TopPanelAction::Undo, "Ctrl+Z")
        .show(ui)
    {
        demo.last = format!("Top panel: {action:?}");
    }
    // As the title bar on Windows and Linux, with the ☰ at its start and the window's
    // caption buttons at its end: the window as it is, its menu open at first with View
    // expanded where the snapshots photograph it, then maximised. Without Saved, as with no
    // photo open.
    let menu = TopPanel::menu_popup();
    for maximised in [false, true] {
        ui.add_space(t.spacing.s);
        let action = TopPanel::new()
            .available(available)
            .menu_button(true)
            .title_bar(true)
            .caption_buttons(true)
            .maximised(maximised)
            .shortcut(TopPanelAction::Menu, "F10")
            .show(ui);
        let anchor = TopPanel::menu_anchor(&ctx);
        let first = !maximised && demo.open_menu;
        if let Some(anchor) = anchor.filter(|_| first || action == Some(TopPanelAction::Menu)) {
            let mut request = sample_menu(anchor);
            if first {
                request = request.expanded("view");
                demo.open_menu = false;
            }
            popup::open(&ctx, menu, request);
        } else if let Some(action) = action {
            demo.last = format!("Top panel: {action:?}");
        }
        if !maximised {
            // Room for the menu, so it covers nothing else.
            ui.allocate_space(vec2(ui.available_width(), 210.0));
        }
    }
    popup::show(&ctx);
    match popup::answer(&ctx, menu) {
        Some(Answer::Chosen(id)) => demo.last = format!("Menu: {id}"),
        Some(Answer::Dismissed) => demo.last = "Menu: dismissed".to_owned(),
        None => {}
    }
    ui.add_space(t.spacing.s);
    let image = crate::tr(&ctx, "layer-image");
    if let Some(action) = CentralBar::new(&image, demo.zoom)
        .toggles(
            Toggle::new(false, true),
            Toggle::new(demo.proof, true),
            Toggle::new(false, false),
            Toggle::new(demo.guides, true),
        )
        .layers(true)
        .values(Some([142, 128, 113]))
        .zoom_enabled(true)
        .zoomable(true)
        .shortcut(CentralBarAction::Zoom(Zoom::Fit), "Ctrl+0")
        .shortcut(CentralBarAction::Zoom(Zoom::Percent(100.0)), "Z")
        .shortcut(CentralBarAction::ZoomIn, "Ctrl+=")
        .shortcut(CentralBarAction::ZoomOut, "Ctrl+−")
        .show(ui)
    {
        match action {
            CentralBarAction::Guides => demo.guides = !demo.guides,
            CentralBarAction::ProofMargin => demo.proof = !demo.proof,
            CentralBarAction::Zoom(zoom) => demo.zoom = zoom,
            other => demo.last = format!("Central bar: {other:?}"),
        }
    }
    // The photo frame in its slot: a 3:2 photo fitted under the margin, or under Proof
    // margin's with its white border, and its caption under it.
    let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
    ui.painter().rect_filled(slot, 0.0, t.colors.photo_surround);
    let margin = if demo.proof {
        Margin::Proof(0.1)
    } else {
        Margin::Points(10.0)
    };
    let frame = PhotoFrame::new(slot, margin);
    let room = frame.bounds();
    let width = room.x.min(room.y * 1.5);
    let placed = frame.place(vec2(width, width / 1.5), ui.pixels_per_point());
    ui.painter().rect_filled(placed.photo, 0.0, t.colors.sunken);
    let exposure = ["1/250 s", "f/8", "35 mm", "ISO 100"].map(str::to_owned);
    PhotoCaption::new("IMG_2041.ARW")
        .view_only(true)
        .exposure(&exposure)
        .show(ui, placed.caption);
    let names: Vec<String> = (0..40)
        .map(|i| format!("IMG_{:04}.ARW", 2036 + i))
        .collect();
    let thumb = |i: usize| Thumb {
        name: &names[i],
        edited: [0, 2, 3, 5, 9].contains(&i),
        status: match i {
            4 => Some(CellStatus::ViewOnly),
            7 => Some(CellStatus::Unreadable("not an Apertrust edit")),
            _ => None,
        },
        selected: demo.selected.contains(&i),
        picture: None,
        rating: None,
    };
    let panel = GridPanel::new(names.len(), Some(demo.active), thumb).show(ui);
    if let Some(action) = panel.action {
        match action {
            GridPanelAction::Open(i) => {
                demo.active = i;
                demo.selected = vec![i];
            }
            GridPanelAction::ToggleSelected(i) => {
                if let Some(k) = demo.selected.iter().position(|&s| s == i) {
                    demo.selected.remove(k);
                } else {
                    demo.selected.push(i);
                }
            }
            GridPanelAction::SelectRange(i) => {
                let (a, b) = (demo.active.min(i), demo.active.max(i));
                demo.selected = (a..=b).collect();
            }
        }
    }
}
