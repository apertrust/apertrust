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

//! Themes: every colour, size and duration a component uses, named by its role (a
//! token), and read from TOML (`docs/gui.md`, decision 36).
//!
//! Three themes are built in: dark, the default, light and high contrast. A user's theme
//! file names one of them in `extends` and overrides any of its tokens. Every theme is
//! checked when it loads: text against its backgrounds for WCAG AA contrast, and the
//! photo's surround for neutrality. A theme that fails still loads; the issues name the
//! tokens.

use std::fmt;
use std::sync::Arc;

use apertrust_base::settings::Choice;
use egui::{Color32, CornerRadius, Id, Stroke, TextStyle, vec2};
use serde::Deserialize;

/// The built-in themes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// A neutral dark grey, the default.
    #[default]
    Dark,
    Light,
    HighContrast,
}

impl Builtin {
    pub const ALL: [Builtin; 3] = [Builtin::Dark, Builtin::Light, Builtin::HighContrast];

    /// The name a theme file's `extends` uses, and settings keep ([`Choice`]).
    pub const fn id(self) -> &'static str {
        match self {
            Builtin::Dark => "dark",
            Builtin::Light => "light",
            Builtin::HighContrast => "high-contrast",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Builtin::Dark => include_str!("../themes/dark.toml"),
            Builtin::Light => include_str!("../themes/light.toml"),
            Builtin::HighContrast => include_str!("../themes/high-contrast.toml"),
        }
    }

    fn from_id(id: &str) -> Option<Builtin> {
        Builtin::ALL.into_iter().find(|b| b.id() == id)
    }
}

/// The theme setting's options (decision 43), by the names theme files use.
impl Choice for Builtin {
    const OPTIONS: &'static [(Builtin, &'static str)] = &[
        (Builtin::Dark, Builtin::Dark.id()),
        (Builtin::Light, Builtin::Light.id()),
        (Builtin::HighContrast, Builtin::HighContrast.id()),
    ];
}

/// The central panel's backgrounds: six neutral greys from black to white, the same in
/// every theme (`docs/gui.md`, The background). Black, Very dark, Dark and Medium are
/// within ISO 12646's limit for judging colour, at most a fifth of white's luminance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Background {
    Black,
    VeryDark,
    Dark,
    Medium,
    Light,
    White,
}

impl Background {
    pub const ALL: [Background; 6] = [
        Background::Black,
        Background::VeryDark,
        Background::Dark,
        Background::Medium,
        Background::Light,
        Background::White,
    ];

    /// Its grey, R = G = B, so its chromaticity is the display's white: 0%, 1%, 2.6%, 18%,
    /// 50% and 100% of white's luminance.
    pub const fn grey(self) -> Color32 {
        let v = match self {
            Background::Black => 0x00,
            Background::VeryDark => 0x19,
            Background::Dark => 0x2D,
            Background::Medium => 0x77,
            Background::Light => 0xBC,
            Background::White => 0xFF,
        };
        Color32::from_rgb(v, v, v)
    }

    /// The one whose grey `colour` is: a theme's own surround, as its menu shows checked.
    pub fn of(colour: Color32) -> Option<Background> {
        Background::ALL.into_iter().find(|b| b.grey() == colour)
    }
}

/// The background setting's options (decision 43).
impl Choice for Background {
    const OPTIONS: &'static [(Background, &'static str)] = &[
        (Background::Black, "black"),
        (Background::VeryDark, "very-dark"),
        (Background::Dark, "dark"),
        (Background::Medium, "medium"),
        (Background::Light, "light"),
        (Background::White, "white"),
    ];
}

/// A full set of tokens.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    /// The theme's name in code and in settings; the interface shows a translated one.
    pub name: String,
    /// Whether egui's own widgets start from its dark visuals.
    pub dark: bool,
    pub colors: Colors,
    pub text: TextSizes,
    pub spacing: Spacing,
    pub sizes: Sizes,
    pub shape: Shape,
    pub elevation: Elevation,
    pub motion: Motion,
    /// The built-in theme this one is or extends, whatever it is named; none for a theme
    /// file that extends none. The high-contrast theme's own rules follow it.
    #[serde(skip)]
    base: Option<Builtin>,
}

/// Colours by role, authored in sRGB as `#RRGGBB` or `#RRGGBBAA`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    /// The top panel, the central bar and the grid panel. The same as the
    /// panel in the built-in themes, so regions part by separators, not by tone.
    #[serde(with = "hex")]
    pub surface: Color32,
    /// The side panel and its sections.
    #[serde(with = "hex")]
    pub panel: Color32,
    /// Menus, popovers and dialogs.
    #[serde(with = "hex")]
    pub raised: Color32,
    /// Over the window under About, Apertrust's own modal: translucent, so the window shows
    /// through, dimmed. Also the drop shadow round the dialog host's windows on Wayland.
    #[serde(with = "hex")]
    pub scrim: Color32,
    /// Value fields, choice lists and other wells.
    #[serde(with = "hex")]
    pub sunken: Color32,
    /// Around the photo in Edit and Compare. Neutral grey only, so it never tints how a
    /// colour is judged.
    #[serde(with = "hex")]
    pub photo_surround: Color32,
    /// Text on a dark surround, such as the file name and exposure under the photo; read
    /// through [`Colors::text_on_surround`], which picks it or `surround_text_on_light`.
    #[serde(with = "hex")]
    pub surround_text: Color32,
    /// Text on a light surround, from Medium up ([`Background`]).
    #[serde(with = "hex")]
    pub surround_text_on_light: Color32,
    #[serde(with = "hex")]
    pub text: Color32,
    /// Text that stands out from `text`: the active photo's name under its thumbnail.
    #[serde(with = "hex")]
    pub text_strong: Color32,
    /// Summaries, captions, labels of lesser weight.
    #[serde(with = "hex")]
    pub text_secondary: Color32,
    #[serde(with = "hex")]
    pub text_disabled: Color32,
    /// Text on an accent fill.
    #[serde(with = "hex")]
    pub text_on_accent: Color32,
    #[serde(with = "hex")]
    pub border: Color32,
    /// The faint edge of a menu, drawn popup or tooltip.
    #[serde(with = "hex")]
    pub menu_edge: Color32,
    /// The soft shadow under a menu, drawn popup or tooltip.
    #[serde(with = "hex")]
    pub menu_shadow: Color32,
    /// The edge of a control that is only an outline at rest, as an unticked checkbox: 3:1
    /// against the panel and the well it is drawn on.
    #[serde(with = "hex")]
    pub control_outline: Color32,
    /// Lines inside a control: a plot's grid, a tab row's rule.
    #[serde(with = "hex")]
    pub divider: Color32,
    /// Between regions and between sections: darker than the surfaces on either side, so
    /// they read as a gap, not a drawn line.
    #[serde(with = "hex")]
    pub separator: Color32,
    /// The fill of a primary button.
    #[serde(with = "hex")]
    pub accent: Color32,
    /// Accent-coloured text and marks on the panel: a selected tab, a link.
    #[serde(with = "hex")]
    pub accent_text: Color32,
    /// Behind something selected or pressed.
    #[serde(with = "hex")]
    pub accent_subtle: Color32,
    #[serde(with = "hex")]
    pub focus_ring: Color32,
    /// Behind selected text.
    #[serde(with = "hex")]
    pub selection: Color32,
    /// Behind selected text in a value field: neutral, since typing a value is no mode.
    #[serde(with = "hex")]
    pub field_selection: Color32,
    /// Behind a control under the pointer.
    #[serde(with = "hex")]
    pub hover: Color32,
    /// Behind the chosen line of a menu or dropdown: neutral, never the accent.
    #[serde(with = "hex")]
    pub menu_hover: Color32,
    /// The words, key and marks of the chosen line on `menu_hover`: `text_strong` in the
    /// dark and light themes; dark on a light band in high contrast, as the system's own
    /// high-contrast mode draws it.
    #[serde(with = "hex")]
    pub text_on_menu_hover: Color32,
    /// Behind the chosen line of a list of places, as the Settings dialog's chosen page:
    /// apart from both `sunken`, the list's ground, and `hover`.
    #[serde(with = "hex")]
    pub list_chosen: Color32,
    /// Behind the window's close button under the pointer, where the app draws the
    /// window's caption buttons: red, as the systems draw it, whatever the theme.
    #[serde(with = "hex")]
    pub close_hover: Color32,
    /// The close button's icon on `close_hover`.
    #[serde(with = "hex")]
    pub text_on_close_hover: Color32,
    #[serde(with = "hex")]
    pub slider_track: Color32,
    /// The part of a plain track moved from the default.
    #[serde(with = "hex")]
    pub slider_fill: Color32,
    #[serde(with = "hex")]
    pub slider_handle: Color32,
    /// Around the handle, so it reads on colour tracks: the panel's colour.
    #[serde(with = "hex")]
    pub slider_handle_outline: Color32,
    /// The handle under the pointer.
    #[serde(with = "hex")]
    pub slider_handle_hover: Color32,
    /// A border just inside the handle's edge while it is dragged.
    #[serde(with = "hex")]
    pub slider_handle_active: Color32,
    /// ● on a slider you have kept.
    #[serde(with = "hex")]
    pub kept_marker: Color32,
    /// A scroll bar's thumb on its `sunken` track: quiet, about 2.5:1 against the track in
    /// the dark and light themes, so that a scroll bar is never bright.
    #[serde(with = "hex")]
    pub scroll_thumb: Color32,
    /// The thumb under the pointer: a clear step up, and at least 3:1 against the track.
    #[serde(with = "hex")]
    pub scroll_thumb_hover: Color32,
    /// The thumb while dragged: another step up.
    #[serde(with = "hex")]
    pub scroll_thumb_active: Color32,
    /// Text naming a colour channel's value: the central bar's red, green and blue under
    /// the cursor. Readable on the bars, so lighter than the scopes' channel colours in
    /// dark.
    #[serde(with = "hex")]
    pub channel_red: Color32,
    #[serde(with = "hex")]
    pub channel_green: Color32,
    #[serde(with = "hex")]
    pub channel_blue: Color32,
    /// A histogram behind a control's graph, as the tone equalizer's mask behind its bands:
    /// translucent, at darktable's weight, its `inset_histogram`, 50% of its grey 65.
    #[serde(with = "hex")]
    pub graph_histogram: Color32,
    #[serde(with = "hex")]
    pub warning: Color32,
    #[serde(with = "hex")]
    pub error: Color32,
    #[serde(with = "hex")]
    pub success: Color32,
    /// The clipping overlays on the photo, which users may change for their colour vision.
    #[serde(with = "hex")]
    pub clip_shadows: Color32,
    #[serde(with = "hex")]
    pub clip_highlights: Color32,
    /// A layer's mask over the photo.
    #[serde(with = "hex")]
    pub mask_overlay: Color32,
    /// Guides over the photo, such as the grid: translucent and neutral, since they lie on
    /// the photo whatever the theme.
    #[serde(with = "hex")]
    pub guides: Color32,
    /// The ring round a selected photo's cell in the grid panel, and round the active one
    /// in a thicker line: the icons' grey (`text_secondary`), never the accent, so the cell's
    /// own colours are not judged against it; the high-contrast theme's brighter.
    #[serde(with = "hex")]
    pub frame_ring: Color32,
    /// Under a cell's marks where they lie over the photo: dark in every theme, 60% black,
    /// so white marks read on any photo.
    #[serde(with = "hex")]
    pub thumb_scrim: Color32,
    /// Marks and text on `thumb_scrim`.
    #[serde(with = "hex")]
    pub text_on_thumb_scrim: Color32,
    /// The warning mark on `thumb_scrim`, the same in every theme, as the scrim is: an
    /// edit file that could not be read.
    #[serde(with = "hex")]
    pub warning_on_thumb_scrim: Color32,
    /// darktable's colour labels, in its order (red, yellow, green, blue, purple): the
    /// circle under a thumbnail and the marks of its list. Readable as a control on the
    /// panels in each theme.
    #[serde(with = "hex")]
    pub label_red: Color32,
    #[serde(with = "hex")]
    pub label_yellow: Color32,
    #[serde(with = "hex")]
    pub label_green: Color32,
    #[serde(with = "hex")]
    pub label_blue: Color32,
    #[serde(with = "hex")]
    pub label_purple: Color32,
}

/// Type sizes in points, before scaling. The weights are fonts of their own
/// ([`crate::fonts`]).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextSizes {
    pub body: f32,
    pub small: f32,
    pub caption: f32,
    /// About's version line, and the other titles outside a section's header.
    pub title: f32,
    /// A module's or utility's title in its section header.
    pub section_title: f32,
    pub heading: f32,
}

/// The spacing scale, in points.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spacing {
    pub xxs: f32,
    pub xs: f32,
    pub s: f32,
    pub m: f32,
    pub l: f32,
    pub xl: f32,
    pub xxl: f32,
}

/// Fixed sizes, in points: the wireframe's grid.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sizes {
    pub row: f32,
    /// A one-line row in a module panel: a choice, a toggle, an action, a collapsible
    /// group's row, and a row of the Settings dialog, so that every one-line row is one
    /// height. `row` stays for popups' lines, text fields and segments.
    pub panel_line: f32,
    /// A control's side outside a module's rows: the bars' buttons, a section header's, a
    /// segmented control, egui's own widgets.
    pub control: f32,
    /// The height of a module row's parts: the value field, the row's own buttons (Auto,
    /// the picker), the kept marker and a dropdown row's box.
    pub field: f32,
    pub icon: f32,
    pub icon_large: f32,
    /// A section's header, whose title and buttons are centred in it; the whole height is
    /// the click target.
    pub section_header: f32,
    /// The top panel's height; theme files keep its old name.
    pub top_bar: f32,
    /// A slider row's height, its padding included, and the strip the track runs in,
    /// under a top line a field high (`docs/gui.md`, How the rows are laid out).
    pub slider_row: f32,
    pub track_strip: f32,
    /// The marker beside a row's label, and a value field's width, the narrowest a
    /// dropdown row's box can be.
    pub marker_column: f32,
    pub value_column: f32,
    pub side_panel: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub radius_small: f32,
    pub radius: f32,
    pub radius_large: f32,
    /// The corners of a menu and every list the popup service draws, a choice's included:
    /// one look for every drawn popup. Its chosen line uses `radius_large`.
    pub menu_radius: f32,
    /// The corners of a tooltip, About, and egui's own windows and menus.
    pub popup_radius: f32,
    pub hairline: f32,
    /// The faint edge of a menu, drawn popup or tooltip: a half point, a full one in the
    /// high-contrast theme.
    pub popup_hairline: f32,
    /// The border between the window's panels (the top panel, the side panel, the central
    /// bar, the grid panel), in `separator`'s colour: 2 points. Lines inside a panel stay
    /// `hairline`.
    pub separator_width: f32,
    /// A slider's track, and the moved part drawn on it.
    pub track: f32,
}

/// The shadow beneath menus, drawn popups and tooltips, in points.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elevation {
    pub shadow_offset: [i8; 2],
    pub shadow_blur: u8,
    pub shadow_spread: u8,
}

/// Durations in milliseconds; zero when motion is reduced.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    pub hover_ms: f32,
    pub open_ms: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("the theme file is not valid TOML or misses a token: {0}")]
    Toml(#[from] toml::de::Error),
    #[error(
        "the theme extends \"{0}\", which is not a built-in theme (dark, light, high-contrast)"
    )]
    UnknownBase(String),
    #[error("`extends` must name a built-in theme, as text")]
    BadExtends,
}

/// Something a theme's check found.
#[derive(Clone, Debug, PartialEq)]
pub enum Issue {
    /// A token against its background falls short of the contrast its use needs.
    Contrast {
        token: &'static str,
        against: &'static str,
        ratio: f32,
        minimum: f32,
    },
    /// A token is the very colour of the background it marks something on.
    Unseen {
        token: &'static str,
        against: &'static str,
    },
    /// The photo's surround has a tint, or is transparent.
    NotNeutral { token: &'static str },
    /// A size smaller than the parts it holds, which would then spill out of it.
    TooSmall {
        token: &'static str,
        size: f32,
        parts: &'static str,
        minimum: f32,
    },
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Issue::Contrast {
                token,
                against,
                ratio,
                minimum,
            } => write!(
                f,
                "{token} on {against} has a contrast of {ratio:.1}:1, below {minimum}:1"
            ),
            Issue::Unseen { token, against } => {
                write!(
                    f,
                    "{token} is the same colour as {against}, so it cannot be seen"
                )
            }
            Issue::NotNeutral { token } => {
                write!(f, "{token} must be an opaque neutral grey")
            }
            Issue::TooSmall {
                token,
                size,
                parts,
                minimum,
            } => write!(
                f,
                "{token} is {size} points, less than {parts} ({minimum} points), which it holds"
            ),
        }
    }
}

/// WCAG AA: body text, and large text and the parts of controls.
const TEXT: f32 = 4.5;
const CONTROL: f32 = 3.0;
/// WCAG AAA for body text, which the high-contrast theme promises.
const TEXT_AAA: f32 = 7.0;

impl Theme {
    /// A built-in theme. They are compiled in and tested, so this cannot fail.
    pub fn builtin(which: Builtin) -> Theme {
        let theme: Theme =
            toml::from_str(which.source()).expect("the built-in themes parse; tested");
        Theme {
            base: Some(which),
            ..theme
        }
    }

    /// Reads a theme file: a full theme, or one that `extends` a built-in theme and
    /// overrides some of its tokens.
    pub fn from_toml(text: &str) -> Result<Theme, ThemeError> {
        let mut table: toml::Table = text.parse()?;
        let Some(base) = table.remove("extends") else {
            return Ok(table.try_into()?);
        };
        let base = base.as_str().ok_or(ThemeError::BadExtends)?;
        let builtin =
            Builtin::from_id(base).ok_or_else(|| ThemeError::UnknownBase(base.to_owned()))?;
        let mut merged: toml::Table = builtin.source().parse()?;
        merge(&mut merged, table);
        let theme: Theme = merged.try_into()?;
        Ok(Theme {
            base: Some(builtin),
            ..theme
        })
    }

    /// Checks the theme for readable text and controls, for a menu's chosen line that
    /// shows, for a neutral surround, and for a slider row tall enough for its parts.
    pub fn check(&self) -> Vec<Issue> {
        let c = &self.colors;
        let pairs = [
            ("text", c.text, "panel", c.panel, TEXT),
            ("text", c.text, "surface", c.surface, TEXT),
            ("text", c.text, "raised", c.raised, TEXT),
            ("text", c.text, "sunken", c.sunken, TEXT),
            ("text", c.text, "field_selection", c.field_selection, TEXT),
            ("text_strong", c.text_strong, "surface", c.surface, TEXT),
            (
                "text_strong",
                c.text_strong,
                "list_chosen",
                c.list_chosen,
                TEXT,
            ),
            ("text_secondary", c.text_secondary, "panel", c.panel, TEXT),
            ("text_secondary", c.text_secondary, "raised", c.raised, TEXT),
            (
                "surround_text",
                c.text_on_surround(),
                "photo_surround",
                c.photo_surround,
                TEXT,
            ),
            ("accent_text", c.accent_text, "panel", c.panel, TEXT),
            ("text_on_accent", c.text_on_accent, "accent", c.accent, TEXT),
            ("channel_red", c.channel_red, "surface", c.surface, TEXT),
            ("channel_green", c.channel_green, "surface", c.surface, TEXT),
            ("channel_blue", c.channel_blue, "surface", c.surface, TEXT),
            ("warning", c.warning, "panel", c.panel, TEXT),
            ("error", c.error, "panel", c.panel, TEXT),
            ("success", c.success, "panel", c.panel, TEXT),
            ("focus_ring", c.focus_ring, "panel", c.panel, CONTROL),
            (
                "text_on_close_hover",
                c.text_on_close_hover,
                "close_hover",
                c.close_hover,
                CONTROL,
            ),
            ("slider_handle", c.slider_handle, "panel", c.panel, CONTROL),
            (
                "slider_handle_hover",
                c.slider_handle_hover,
                "panel",
                c.panel,
                CONTROL,
            ),
            (
                "slider_handle_active",
                c.slider_handle_active,
                "panel",
                c.panel,
                CONTROL,
            ),
            ("kept_marker", c.kept_marker, "panel", c.panel, CONTROL),
            (
                "control_outline",
                c.control_outline,
                "panel",
                c.panel,
                CONTROL,
            ),
            (
                "control_outline",
                c.control_outline,
                "sunken",
                c.sunken,
                CONTROL,
            ),
            ("frame_ring", c.frame_ring, "surface", c.surface, CONTROL),
            ("label_red", c.label_red, "surface", c.surface, CONTROL),
            (
                "label_yellow",
                c.label_yellow,
                "surface",
                c.surface,
                CONTROL,
            ),
            ("label_green", c.label_green, "surface", c.surface, CONTROL),
            ("label_blue", c.label_blue, "surface", c.surface, CONTROL),
            (
                "label_purple",
                c.label_purple,
                "surface",
                c.surface,
                CONTROL,
            ),
            // The thumb at rest is quieter than a control, on purpose; once the pointer is
            // on it, it reads as one.
            (
                "scroll_thumb_hover",
                c.scroll_thumb_hover,
                "sunken",
                c.sunken,
                CONTROL,
            ),
            (
                "scroll_thumb_active",
                c.scroll_thumb_active,
                "sunken",
                c.sunken,
                CONTROL,
            ),
        ];
        // A menu's chosen line: a band that shows on `raised`, its text readable on it. The
        // high-contrast theme, and any theme extending it, holds the band to a control's
        // part's ratio, its text to AAA's and a disabled line to a control's.
        let high_contrast = self.base == Some(Builtin::HighContrast);
        let mut menu_band = vec![(
            "text_on_menu_hover",
            c.text_on_menu_hover,
            "menu_hover",
            c.menu_hover,
            if high_contrast { TEXT_AAA } else { TEXT },
        )];
        if high_contrast {
            menu_band.push((
                "text_disabled",
                c.text_disabled,
                "menu_hover",
                c.menu_hover,
                CONTROL,
            ));
        }
        let mut issues = Vec::new();
        if c.menu_hover == c.raised {
            issues.push(Issue::Unseen {
                token: "menu_hover",
                against: "raised",
            });
        } else if high_contrast {
            menu_band.push(("menu_hover", c.menu_hover, "raised", c.raised, CONTROL));
        }
        issues.extend(pairs.into_iter().chain(menu_band).filter_map(
            |(token, fg, against, bg, minimum)| {
                let ratio = contrast(fg, bg);
                (ratio < minimum).then_some(Issue::Contrast {
                    token,
                    against,
                    ratio,
                    minimum,
                })
            },
        ));
        // The caption reads on every background the user can choose.
        for &(background, id) in Background::OPTIONS {
            let grey = background.grey();
            let ratio = contrast(c.text_on(grey), grey);
            if ratio < TEXT {
                issues.push(Issue::Contrast {
                    token: "surround_text",
                    against: id,
                    ratio,
                    minimum: TEXT,
                });
            }
        }
        let [r, g, b, a] = c.photo_surround.to_srgba_unmultiplied();
        if r != g || g != b || a != 255 {
            issues.push(Issue::NotNeutral {
                token: "photo_surround",
            });
        }
        // The slider row is built upwards from its bottom: its padding under the track, the
        // track's strip, then the top line, a field high. What is left is its padding above,
        // which a shorter row would make negative, pushing the top line out of it.
        let s = &self.sizes;
        let minimum = s.field + s.track_strip + self.spacing.xxs;
        if s.slider_row < minimum {
            issues.push(Issue::TooSmall {
                token: "sizes.slider_row",
                size: s.slider_row,
                parts: "sizes.field + sizes.track_strip + spacing.xxs",
                minimum,
            });
        }
        issues
    }

    /// The colour tokens with their names, for the gallery.
    pub fn named_colors(&self) -> Vec<(&'static str, Color32)> {
        let c = &self.colors;
        vec![
            ("surface", c.surface),
            ("panel", c.panel),
            ("raised", c.raised),
            ("scrim", c.scrim),
            ("sunken", c.sunken),
            ("photo_surround", c.photo_surround),
            ("surround_text", c.surround_text),
            ("surround_text_on_light", c.surround_text_on_light),
            ("text", c.text),
            ("text_strong", c.text_strong),
            ("text_secondary", c.text_secondary),
            ("text_disabled", c.text_disabled),
            ("text_on_accent", c.text_on_accent),
            ("border", c.border),
            ("menu_edge", c.menu_edge),
            ("menu_shadow", c.menu_shadow),
            ("control_outline", c.control_outline),
            ("divider", c.divider),
            ("separator", c.separator),
            ("accent", c.accent),
            ("accent_text", c.accent_text),
            ("accent_subtle", c.accent_subtle),
            ("focus_ring", c.focus_ring),
            ("selection", c.selection),
            ("field_selection", c.field_selection),
            ("hover", c.hover),
            ("menu_hover", c.menu_hover),
            ("text_on_menu_hover", c.text_on_menu_hover),
            ("list_chosen", c.list_chosen),
            ("close_hover", c.close_hover),
            ("text_on_close_hover", c.text_on_close_hover),
            ("slider_track", c.slider_track),
            ("slider_fill", c.slider_fill),
            ("slider_handle", c.slider_handle),
            ("slider_handle_outline", c.slider_handle_outline),
            ("slider_handle_hover", c.slider_handle_hover),
            ("slider_handle_active", c.slider_handle_active),
            ("kept_marker", c.kept_marker),
            ("scroll_thumb", c.scroll_thumb),
            ("scroll_thumb_hover", c.scroll_thumb_hover),
            ("scroll_thumb_active", c.scroll_thumb_active),
            ("channel_red", c.channel_red),
            ("channel_green", c.channel_green),
            ("channel_blue", c.channel_blue),
            ("graph_histogram", c.graph_histogram),
            ("warning", c.warning),
            ("error", c.error),
            ("success", c.success),
            ("clip_shadows", c.clip_shadows),
            ("clip_highlights", c.clip_highlights),
            ("mask_overlay", c.mask_overlay),
            ("guides", c.guides),
            ("frame_ring", c.frame_ring),
            ("thumb_scrim", c.thumb_scrim),
            ("text_on_thumb_scrim", c.text_on_thumb_scrim),
            ("warning_on_thumb_scrim", c.warning_on_thumb_scrim),
            ("label_red", c.label_red),
            ("label_yellow", c.label_yellow),
            ("label_green", c.label_green),
            ("label_blue", c.label_blue),
            ("label_purple", c.label_purple),
        ]
    }

    /// Writes the theme into egui's styles, for the egui widgets used directly (text
    /// fields, scroll bars, menus). Fonts are set apart ([`crate::fonts`]).
    fn apply(&self, style: &mut egui::Style) {
        let c = &self.colors;
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let hairline = |color| Stroke::new(self.shape.hairline, color);

        v.panel_fill = c.panel;
        v.window_fill = c.raised;
        v.window_stroke = Stroke::new(self.shape.popup_hairline, c.menu_edge);
        v.window_corner_radius = radius(self.shape.popup_radius);
        v.menu_corner_radius = radius(self.shape.popup_radius);
        let shadow = egui::Shadow {
            offset: self.elevation.shadow_offset,
            blur: self.elevation.shadow_blur,
            spread: self.elevation.shadow_spread,
            color: c.menu_shadow,
        };
        v.window_shadow = shadow;
        v.popup_shadow = shadow;
        v.faint_bg_color = c.surface;
        v.extreme_bg_color = c.sunken;
        v.text_edit_bg_color = Some(c.sunken);
        v.code_bg_color = c.sunken;
        v.hyperlink_color = c.accent_text;
        v.warn_fg_color = c.warning;
        v.error_fg_color = c.error;
        v.selection.bg_fill = c.selection;
        v.selection.stroke = Stroke::new(1.0, c.text);
        v.text_cursor.stroke = Stroke::new(2.0, c.accent_text);

        let w = &mut v.widgets;
        for (state, fill, border, text) in [
            (&mut w.noninteractive, c.panel, c.divider, c.text),
            (&mut w.inactive, c.sunken, Color32::TRANSPARENT, c.text),
            (&mut w.hovered, c.hover, c.border, c.text),
            (&mut w.active, c.accent_subtle, c.focus_ring, c.text),
            (&mut w.open, c.raised, c.border, c.text),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = hairline(border);
            state.fg_stroke = Stroke::new(1.0, text);
            state.corner_radius = radius(self.shape.radius);
        }
        style.visuals = v;

        style.spacing.item_spacing = vec2(self.spacing.m, self.spacing.xs);
        style.spacing.interact_size.y = self.sizes.control;
        // Solid scroll bars, always shown where content scrolls, as the wireframe draws them.
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.spacing.scroll.bar_width = 8.0;
        style.animation_time = self.motion.open_ms / 1000.0;
        for (text_style, size) in [
            (TextStyle::Small, self.text.small),
            (TextStyle::Body, self.text.body),
            (TextStyle::Button, self.text.body),
            (TextStyle::Monospace, self.text.body),
            (TextStyle::Heading, self.text.heading),
        ] {
            if let Some(font) = style.text_styles.get_mut(&text_style) {
                font.size = size;
            }
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::builtin(Builtin::default())
    }
}

/// Makes `theme` current: written into egui's styles and stored for components, which
/// read it with [`theme`]. Switching takes effect on the next frame, with no restart.
pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    ctx.all_styles_mut(|style| theme.apply(style));
    ctx.data_mut(|data| data.insert_temp(theme_id(), Arc::new(theme)));
}

/// The current theme; the default one until [`set_theme`] is called.
pub fn theme(ctx: &egui::Context) -> Arc<Theme> {
    ctx.data(|data| data.get_temp::<Arc<Theme>>(theme_id()))
        .unwrap_or_else(|| Arc::new(Theme::default()))
}

fn theme_id() -> Id {
    Id::unique("apertrust-ui theme")
}

impl Colors {
    /// The text on the photo's surround: whichever of `surround_text` and
    /// `surround_text_on_light` reads better on it.
    pub fn text_on_surround(&self) -> Color32 {
        self.text_on(self.photo_surround)
    }

    fn text_on(&self, surround: Color32) -> Color32 {
        let (dark, light) = (self.surround_text, self.surround_text_on_light);
        if contrast(light, surround) > contrast(dark, surround) {
            light
        } else {
            dark
        }
    }
}

impl Theme {
    /// This theme with `background` behind the photo in place of its own surround, or as
    /// it is for None.
    pub fn with_background(mut self, background: Option<Background>) -> Theme {
        if let Some(background) = background {
            self.colors.photo_surround = background.grey();
        }
        self
    }
}

/// A radius token as egui's corners, which are whole points from 0 to 255.
fn radius(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// The WCAG contrast ratio of two opaque sRGB colours, from 1:1 to 21:1.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// WCAG's relative luminance.
fn luminance(c: Color32) -> f32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let linear = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/// Merges `over` into `base`, table by table, so a theme file can override single tokens.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

/// Colours as `#RRGGBB` or `#RRGGBBAA` in sRGB.
mod hex {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, de::Error};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color32, D::Error> {
        let text = String::deserialize(d)?;
        parse(&text).ok_or_else(|| {
            D::Error::custom(format!(
                "\"{text}\" is not a colour: write #RRGGBB or #RRGGBBAA"
            ))
        })
    }

    pub fn parse(text: &str) -> Option<Color32> {
        let digits = text.strip_prefix('#')?;
        if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
        let alpha = if digits.len() == 8 { byte(6)? } else { 255 };
        Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            alpha,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_theme_is_an_option() {
        let options: Vec<Builtin> = Builtin::OPTIONS.iter().map(|(b, _)| *b).collect();
        assert_eq!(options, Builtin::ALL);
    }

    #[test]
    fn the_built_in_themes_parse_and_pass_their_checks() {
        for which in Builtin::ALL {
            let theme = Theme::builtin(which);
            assert_eq!(theme.check(), vec![], "{}", which.id());
        }
    }

    #[test]
    fn the_scroll_bar_s_thumb_is_quiet_and_brightens_in_steps() {
        for which in Builtin::ALL {
            let c = Theme::builtin(which).colors;
            let [rest, hover, active] =
                [c.scroll_thumb, c.scroll_thumb_hover, c.scroll_thumb_active]
                    .map(|thumb| contrast(thumb, c.sunken));
            let name = which.id();
            // Seen at rest, but never bright where the theme is not high contrast.
            assert!(rest >= 2.4, "{name}: {rest:.2}");
            if which != Builtin::HighContrast {
                assert!(rest <= 2.7, "{name}: {rest:.2}");
            }
            // Each state a clear step up from the last.
            assert!(hover >= rest * 1.25, "{name}: {rest:.2} then {hover:.2}");
            assert!(
                active >= hover * 1.25,
                "{name}: {hover:.2} then {active:.2}"
            );
        }
    }

    #[test]
    fn dark_is_the_default() {
        assert_eq!(Theme::default(), Theme::builtin(Builtin::Dark));
        assert!(Theme::default().dark);
    }

    #[test]
    fn high_contrast_meets_aaa_for_text() {
        let t = Theme::builtin(Builtin::HighContrast);
        let c = &t.colors;
        for (fg, bg) in [
            (c.text, c.panel),
            (c.text_secondary, c.panel),
            (c.accent_text, c.panel),
            (c.text_on_accent, c.accent),
        ] {
            assert!(contrast(fg, bg) >= 7.0, "{fg:?} on {bg:?}");
        }
    }

    #[test]
    fn a_user_theme_overrides_single_tokens() {
        let theme = Theme::from_toml(
            r##"
            extends = "dark"
            name = "Mine"
            [colors]
            accent_text = "#FFB000"
            [text]
            body = 14.0
            "##,
        )
        .unwrap();
        let dark = Theme::builtin(Builtin::Dark);
        assert_eq!(theme.name, "Mine");
        assert_eq!(
            theme.colors.accent_text,
            Color32::from_rgb(0xFF, 0xB0, 0x00)
        );
        assert_eq!(theme.text.body, 14.0);
        assert_eq!(theme.colors.panel, dark.colors.panel);
        assert_eq!(theme.sizes, dark.sizes);
    }

    #[test]
    fn a_failing_theme_still_loads_and_names_the_token() {
        let theme = Theme::from_toml(
            r##"
            extends = "light"
            [colors]
            text_secondary = "#C8C8C8"
            photo_surround = "#808090"
            "##,
        )
        .unwrap();
        let issues = theme.check();
        assert!(issues.iter().any(|i| matches!(
            i,
            Issue::Contrast {
                token: "text_secondary",
                against: "panel",
                ..
            }
        )));
        assert!(issues.contains(&Issue::NotNeutral {
            token: "photo_surround"
        }));
        assert!(issues.iter().all(|i| !i.to_string().is_empty()));
    }

    #[test]
    fn the_caption_reads_on_every_background_and_a_choice_replaces_the_surround() {
        for which in Builtin::ALL {
            let theme = Theme::builtin(which);
            for background in Background::ALL {
                let colors = theme.clone().with_background(Some(background)).colors;
                let ratio = contrast(colors.text_on_surround(), background.grey());
                assert!(ratio >= TEXT, "{which:?} on {background:?}: {ratio}");
            }
            // Each built-in theme's own surround is one of the six.
            assert!(Background::of(theme.colors.photo_surround).is_some());
            assert_eq!(theme.clone().with_background(None), theme);
        }
        let dark = Theme::builtin(Builtin::Dark).colors;
        assert_eq!(dark.text_on_surround(), dark.surround_text);
        let medium = Theme::builtin(Builtin::Dark).with_background(Some(Background::Medium));
        assert_eq!(
            medium.colors.text_on_surround(),
            dark.surround_text_on_light
        );
    }

    #[test]
    fn a_menu_s_chosen_line_shows_and_stands_out_in_high_contrast() {
        let same = Theme::from_toml(
            r##"
            extends = "dark"
            [colors]
            menu_hover = "#363636"
            "##,
        )
        .unwrap();
        let unseen = Issue::Unseen {
            token: "menu_hover",
            against: "raised",
        };
        assert_eq!(same.check(), vec![unseen.clone()]);
        assert_eq!(
            unseen.to_string(),
            "menu_hover is the same colour as raised, so it cannot be seen"
        );
        // A dark band with light text, as the dark theme's, apart from raised but far under
        // 3:1, would be too faint in high contrast.
        let faint = Theme::from_toml(
            r##"
            extends = "high-contrast"
            [colors]
            menu_hover = "#333333"
            text_on_menu_hover = "#FFFFFF"
            "##,
        )
        .unwrap();
        assert!(matches!(
            faint.check().as_slice(),
            [Issue::Contrast {
                token: "menu_hover",
                against: "raised",
                minimum: CONTROL,
                ..
            }]
        ));
        let dark = Theme::builtin(Builtin::Dark).colors;
        assert!(contrast(dark.menu_hover, dark.raised) < CONTROL);
        // Its text is held to AAA there, which AA's 4.5:1 does not meet; text failing both
        // is named once, against the stricter.
        for grey in ["#666666", "#BBBBBB"] {
            let grey_text = Theme::from_toml(&format!(
                "extends = \"high-contrast\"\n[colors]\ntext_on_menu_hover = \"{grey}\"\n"
            ))
            .unwrap();
            assert!(
                matches!(
                    grey_text.check().as_slice(),
                    [Issue::Contrast {
                        token: "text_on_menu_hover",
                        against: "menu_hover",
                        minimum: TEXT_AAA,
                        ..
                    }]
                ),
                "{grey}"
            );
        }
        // A disabled line on the band stays readable there.
        let light_disabled = Theme::from_toml(
            r##"
            extends = "high-contrast"
            [colors]
            text_disabled = "#B0B0B0"
            "##,
        )
        .unwrap();
        assert!(matches!(
            light_disabled.check().as_slice(),
            [Issue::Contrast {
                token: "text_disabled",
                against: "menu_hover",
                minimum: CONTROL,
                ..
            }]
        ));
        // A band the very colour of raised is named once, as unseen, in high contrast too.
        let unseen_there = Theme::from_toml(
            r##"
            extends = "high-contrast"
            [colors]
            menu_hover = "#0A0A0A"
            text_on_menu_hover = "#FFFFFF"
            text_disabled = "#8C8C8C"
            "##,
        )
        .unwrap();
        assert_eq!(unseen_there.check(), vec![unseen.clone()]);
        // The rules follow the theme a file extends, whatever it is named.
        let renamed = Theme::from_toml(
            r##"
            extends = "high-contrast"
            name = "Mine"
            [colors]
            menu_hover = "#333333"
            text_on_menu_hover = "#FFFFFF"
            "##,
        )
        .unwrap();
        assert_eq!(renamed.check(), faint.check());
        // The dark and light themes draw the chosen line's text as before, in text_strong.
        for which in [Builtin::Dark, Builtin::Light] {
            let c = Theme::builtin(which).colors;
            assert_eq!(c.text_on_menu_hover, c.text_strong, "{which:?}");
        }
    }

    #[test]
    fn a_slider_row_too_short_for_its_parts_is_named_with_them() {
        // 18 + 16 + 2 points of parts in a row of 35.
        let theme = Theme::from_toml(
            r##"
            extends = "dark"
            [spacing]
            xxs = 2.0
            [sizes]
            field = 18.0
            track_strip = 16.0
            slider_row = 35.0
            "##,
        )
        .unwrap();
        let issue = Issue::TooSmall {
            token: "sizes.slider_row",
            size: 35.0,
            parts: "sizes.field + sizes.track_strip + spacing.xxs",
            minimum: 36.0,
        };
        assert_eq!(theme.check(), vec![issue.clone()]);
        assert_eq!(
            issue.to_string(),
            "sizes.slider_row is 35 points, less than sizes.field + sizes.track_strip + \
             spacing.xxs (36 points), which it holds"
        );
        // Exactly the parts' height is enough, with no padding above; the built-in themes
        // leave a point there.
        let mut exact = theme;
        exact.sizes.slider_row = 36.0;
        assert_eq!(exact.check(), vec![]);
        let dark = Theme::builtin(Builtin::Dark);
        let s = &dark.sizes;
        assert_eq!(
            s.slider_row - (s.field + s.track_strip + dark.spacing.xxs),
            1.0
        );
    }

    #[test]
    fn bad_files_are_refused_with_a_reason() {
        assert!(matches!(
            Theme::from_toml("extends = \"sepia\""),
            Err(ThemeError::UnknownBase(b)) if b == "sepia"
        ));
        assert!(matches!(
            Theme::from_toml("extends = \"dark\"\n[colors]\ntext = \"white\""),
            Err(ThemeError::Toml(_))
        ));
        assert!(matches!(
            Theme::from_toml("extends = \"dark\"\n[colors]\nglow = \"#FFFFFF\""),
            Err(ThemeError::Toml(_))
        ));
        // A full theme has to name every token.
        assert!(matches!(
            Theme::from_toml("name = \"Half\"\ndark = true"),
            Err(ThemeError::Toml(_))
        ));
    }

    #[test]
    fn contrast_matches_wcag() {
        assert!((contrast(Color32::WHITE, Color32::BLACK) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 1e-6);
        // #767676 on white is the classic 4.54:1.
        let grey = Color32::from_rgb(0x76, 0x76, 0x76);
        assert!((contrast(grey, Color32::WHITE) - 4.54).abs() < 0.01);
    }

    #[test]
    fn hex_colours() {
        assert_eq!(
            hex::parse("#1D1D1B"),
            Some(Color32::from_rgb(0x1D, 0x1D, 0x1B))
        );
        assert_eq!(
            hex::parse("#FF3B3080"),
            Some(Color32::from_rgba_unmultiplied(0xFF, 0x3B, 0x30, 0x80))
        );
        assert_eq!(hex::parse("1D1D1B"), None);
        assert_eq!(hex::parse("#1D1D1"), None);
        assert_eq!(hex::parse("#GGGGGG"), None);
    }

    #[test]
    fn setting_a_theme_makes_it_current() {
        let ctx = egui::Context::default();
        assert_eq!(*theme(&ctx), Theme::default());
        let light = Theme::builtin(Builtin::Light);
        set_theme(&ctx, light.clone());
        assert_eq!(*theme(&ctx), light);
        assert_eq!(ctx.global_style().visuals.panel_fill, light.colors.panel);
        assert!(!ctx.global_style().visuals.dark_mode);
    }

    #[test]
    fn menus_popups_and_tooltips_use_the_themed_edge_and_shadow() {
        for which in Builtin::ALL {
            let theme = Theme::builtin(which);
            let ctx = egui::Context::default();
            set_theme(&ctx, theme.clone());
            let visuals = &ctx.global_style().visuals;
            assert_eq!(visuals.window_fill, theme.colors.raised);
            assert_eq!(
                visuals.window_stroke,
                Stroke::new(theme.shape.popup_hairline, theme.colors.menu_edge)
            );
            assert_eq!(
                visuals.window_corner_radius,
                radius(theme.shape.popup_radius)
            );
            assert_eq!(visuals.menu_corner_radius, radius(theme.shape.popup_radius));
            let shadow = egui::Shadow {
                offset: theme.elevation.shadow_offset,
                blur: theme.elevation.shadow_blur,
                spread: theme.elevation.shadow_spread,
                color: theme.colors.menu_shadow,
            };
            assert_eq!(visuals.window_shadow, shadow);
            assert_eq!(visuals.popup_shadow, shadow);
        }
    }

    #[test]
    fn a_radius_token_becomes_whole_egui_corners() {
        for (token, corners) in [
            (6.0, 6),
            (8.0, 8),
            (5.4, 5),
            (5.5, 6),
            (0.0, 0),
            (-1.0, 0),
            (300.0, 255),
        ] {
            assert_eq!(radius(token), CornerRadius::same(corners), "{token}");
        }
    }
}
