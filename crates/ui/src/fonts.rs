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

//! The UI's typeface, IBM Plex Sans (architecture decision 36), in place of egui's
//! default fonts. The files ship unmodified, with their licence, in `assets/fonts`.
//! It covers Latin, Cyrillic and Greek, so English, Ukrainian and Polish need no other
//! font (`docs/gui.md`).

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, TextStyle};

const REGULAR: &str = "IBM Plex Sans";
/// The family of the Medium face: `FontFamily::Name(MEDIUM.into())`.
pub const MEDIUM: &str = "IBM Plex Sans Medium";
/// The family of the SemiBold face, which marks a choice: the chosen tab, the match in a
/// dropdown's filter. Headings are Medium.
pub const SEMIBOLD: &str = "IBM Plex Sans SemiBold";

const FACES: [(&str, &[u8]); 3] = [
    (
        REGULAR,
        include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf"),
    ),
    (
        MEDIUM,
        include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf"),
    ),
    (
        SEMIBOLD,
        include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    ),
];

/// Installs the fonts and sets headings in Medium: egui has no font weight, so each
/// weight is a family of its own.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
    ctx.all_styles_mut(|style| {
        if let Some(heading) = style.text_styles.get_mut(&TextStyle::Heading) {
            heading.family = FontFamily::Name(MEDIUM.into());
        }
    });
}

/// Regular serves both of egui's families. Its digits, `+` and `−` share one width, so a
/// changing value keeps its place and egui's monospace style needs no second font.
/// Medium and SemiBold are families named after their face.
pub(crate) fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    for (name, bytes) in FACES {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.insert(family, vec![REGULAR.to_owned()]);
    }
    for name in [MEDIUM, SEMIBOLD] {
        fonts
            .families
            .insert(FontFamily::Name(name.into()), vec![name.to_owned()]);
    }
    fonts
}

#[cfg(test)]
mod tests {
    use egui::FontId;
    use egui::epaint::text::{Fonts, TextOptions};

    use super::*;

    fn families() -> [FontFamily; 4] {
        [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name(MEDIUM.into()),
            FontFamily::Name(SEMIBOLD.into()),
        ]
    }

    #[test]
    fn every_family_covers_the_ui_text() {
        // English, Ukrainian and Polish, with the symbols labels and values use.
        let text =
            "Exposure, contrast Експозиція, ґ є і ї Ekspozycja ą ć ę ł ń ó ś ź ż × − ± … ° ΔE „”«»";
        let mut fonts = Fonts::new(TextOptions::default(), definitions());
        for family in families() {
            let font = FontId::new(13.0, family.clone());
            assert!(fonts.has_glyphs(&font, text), "{family:?}");
        }
    }

    #[test]
    fn digits_and_signs_share_one_width() {
        let mut fonts = Fonts::new(TextOptions::default(), definitions());
        let mut view = fonts.with_pixels_per_point(1.0);
        for family in families() {
            let font = FontId::new(13.0, family.clone());
            let width = view.glyph_width(&font, '0');
            for c in "123456789+−".chars() {
                assert_eq!(view.glyph_width(&font, c), width, "{family:?} {c}");
            }
        }
    }

    #[test]
    fn a_frame_lays_out_with_only_these_fonts() {
        let ctx = egui::Context::default();
        install(&ctx);
        let mut exposure = 0.35_f32;
        // Fonts take effect from the next pass. No renderer takes the font atlas's
        // texture updates here, so they are dropped.
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.heading("Apertrust");
                ui.add(egui::Slider::new(&mut exposure, -4.0..=4.0).text("EV"));
                ui.monospace("2.6 ms");
            });
            output.textures_delta.clear();
        }
    }
}
