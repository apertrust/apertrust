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

//! `apertrust-ui`: Apertrust's design system, the tokens and themes, fonts, icons and
//! components every screen is built from (`docs/gui.md`, architecture decision 36).
//!
//! It knows nothing of photos, modules or the engine: a component draws the value it is
//! given and reports what the user did, and the app turns that into a session command.
//! Components read the current theme from egui's context ([`theme()`]) and never hold a
//! colour or size of their own.
//!
//! [`gallery`] shows everything here in every theme; `cargo run -p apertrust-ui
//! --example gallery` opens it.

pub mod alert;
pub mod calculator;
pub mod components;
pub mod fonts;
pub mod gallery;
pub mod i18n;
pub mod icons;
pub mod popup;
pub mod theme;

use std::sync::Arc;

pub use i18n::{Locale, Translator};
pub use icons::{Icon, Picture};
pub use theme::{Background, Builtin, Theme, set_theme, theme};

/// Prepares a context for Apertrust's interface: the fonts, `theme`, and focus rings
/// that show only after the keyboard is used.
pub fn install(ctx: &egui::Context, theme: Theme) {
    fonts::install(ctx);
    set_theme(ctx, theme);
    ctx.add_plugin(components::FocusVisible::default());
}

/// Makes `locale` the interface's language: messages and numbers switch on the next frame,
/// with no restart.
pub fn set_locale(ctx: &egui::Context, locale: Locale) {
    let translator = Arc::new(Translator::new(locale));
    ctx.data_mut(|data| data.insert_temp(translator_id(), translator));
}

/// The current language's translator; British English until [`set_locale`].
pub fn translator(ctx: &egui::Context) -> Arc<Translator> {
    if let Some(translator) = ctx.data(|data| data.get_temp::<Arc<Translator>>(translator_id())) {
        return translator;
    }
    let translator = Arc::new(Translator::new(Locale::default()));
    ctx.data_mut(|data| data.insert_temp(translator_id(), Arc::clone(&translator)));
    translator
}

/// The message `id` in the current language: `tr(ctx, "action-undo")`. Every word the
/// interface shows goes through here or [`Translator::text_with`]; a CI check refuses
/// English literals in labels, buttons and tooltips (`tools/check-ui-strings.sh`).
pub fn tr(ctx: &egui::Context, id: &str) -> String {
    translator(ctx).text(id)
}

fn translator_id() -> egui::Id {
    egui::Id::unique("apertrust-ui translator")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_locale_switches_live() {
        let ctx = egui::Context::default();
        assert_eq!(tr(&ctx, "action-undo"), "Undo");
        set_locale(&ctx, Locale::Uk);
        assert_eq!(tr(&ctx, "action-undo"), "Скасувати");
        set_locale(&ctx, Locale::Pl);
        assert_eq!(tr(&ctx, "action-undo"), "Cofnij");
    }

    #[test]
    fn the_fonts_draw_every_message_in_every_locale() {
        use egui::epaint::text::{Fonts, TextOptions};
        let mut fonts = Fonts::new(TextOptions::default(), fonts::definitions());
        let font = egui::FontId::proportional(13.0);
        let mut args = i18n::Args::new();
        args.set("count", 1234.5);
        args.set("section", "Exposure");
        args.set("control", "Contrast");
        args.set("value", "+0.42");
        args.set("view", "Print");
        args.set("layer", "Sky");
        args.set("shadows", 0.12);
        args.set("highlights", 0.4);
        args.set("done", 3);
        args.set("total", 12);
        args.set("failed", 1);
        args.set("photo", "IMG_2047.ARW");
        args.set("file", "IMG_2047.jpg");
        args.set("folder", "~/Pictures/apertrust_exported");
        args.set("cause", "the disk is full");
        for locale in Locale::ALL {
            let t = Translator::new(locale);
            for id in i18n::message_ids() {
                // A line break is the layout's, no glyph: the start window's two lines.
                let text = t.text_with(&id, Some(&args)).replace('\n', "");
                assert!(
                    fonts.has_glyphs(&font, &text),
                    "{} {id}: {text}",
                    locale.id()
                );
            }
        }
    }
}
