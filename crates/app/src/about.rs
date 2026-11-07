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

//! About Apertrust (`docs/gui.md`, Help): a modal dialog of Apertrust's own, which the
//! `about` action opens, from the top panel's mark among others. Its page shows the full
//! logo, the version as the startup banner gives it, the copyright and the licence, with
//! two buttons, Credits and Licence, each a page one step in: `CREDITS.md` drawn as
//! Markdown, and the GNU GPL's text.
//!
//! On About a click anywhere but on its buttons closes it, and so does Esc; on Credits and
//! Licence only a click outside closes it, and Esc or the back button goes back.

use apertrust_ui::components::{IconButton, quiet_scroll};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Icon, Picture, fonts, theme, tr, translator};
use eframe::egui::{self, Key, Modifiers, Role, Sense, UiBuilder, WidgetInfo};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

const CREDITS: &str = include_str!("../../../CREDITS.md");
const LICENCE: &str = include_str!("../../../LICENSE");

/// The full logo, ink letters for a light theme and paper letters for a dark one.
const LOGO: Picture = Picture::new(
    "apertrust-logo",
    include_str!("../assets/icons/svg/apertrust-logo.svg"),
);
const LOGO_DARK: Picture = Picture::new(
    "apertrust-logo-dark",
    include_str!("../assets/icons/svg/apertrust-logo-dark.svg"),
);

/// The dialog's width on About, and the logo's, in points, where the window has room.
const ABOUT_WIDTH: f32 = 440.0;
const LOGO_WIDTH: f32 = 260.0;
/// Credits' and Licence's width, and the height of the text that scrolls in them, where
/// the window has room.
const PAGE_WIDTH: f32 = 560.0;
const PAGE_HEIGHT: f32 = 440.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    About,
    Credits,
    Licence,
}

/// The dialog: closed, or open on one of its pages.
pub struct About {
    /// The page shown; none while closed.
    page: Option<Page>,
    /// Which page's button takes the keyboard focus on the next frame, after a step taken
    /// by keyboard: on About, the button to the page the user came back from; on Credits or
    /// Licence, as it opens, its back button. Nothing takes it as the dialog opens or after
    /// a click, so no button is drawn highlighted there.
    focus: Option<Page>,
    /// The control that opened the dialog, which gets the keyboard focus back as it
    /// closes: the top panel's mark.
    opener: Option<egui::Id>,
    /// The version with the commit, as the startup banner gives it.
    version: String,
    /// egui_commonmark's state for Credits.
    markdown: CommonMarkCache,
}

impl Default for About {
    fn default() -> Self {
        About {
            page: None,
            focus: None,
            opener: None,
            version: apertrust_base::log::full_version(apertrust_base::program!()),
            markdown: CommonMarkCache::default(),
        }
    }
}

impl About {
    /// Opens the dialog on About, nothing focused, and remembers what opened it: the
    /// control clicked this frame, else the one focused.
    pub fn open(&mut self, ctx: &egui::Context) {
        self.page = Some(Page::About);
        self.focus = None;
        self.opener = ctx
            .viewport(|v| v.interact_widgets.clicked)
            .or_else(|| ctx.memory(|m| m.focused()));
    }

    pub fn is_open(&self) -> bool {
        self.page.is_some()
    }

    /// The page shown, if the dialog is open.
    #[cfg(test)]
    pub fn page(&self) -> Option<Page> {
        self.page
    }

    /// Draws the dialog while it is open, and goes where the user asks.
    pub fn show(&mut self, ctx: &egui::Context) {
        let Some(page) = self.page else {
            // The frame after it closed: egui keeps the focus off what lay under the
            // dialog until then.
            if let Some(opener) = self.opener.take() {
                ctx.memory_mut(|m| m.request_focus(opener));
            }
            return;
        };
        let t = theme(ctx);
        let name = tr(ctx, "action-about");
        let frame = egui::Frame::popup(&ctx.global_style()).inner_margin(t.spacing.xxl);
        // The room the window leaves the dialog, a margin round it.
        let room = ctx.content_rect().size() - egui::Vec2::splat(2.0 * t.spacing.xxl);
        let modal = egui::Modal::new(egui::Id::unique("about"))
            .backdrop_color(t.colors.scrim)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                // The dialog's own response, under its contents: a click on it is a click
                // inside that no button took. It senses clicks but takes no focus, as
                // egui's own modal does, so it is no Tab stop.
                let inside = ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                    frame
                        .show(ui, |ui| match page {
                            Page::About => self.about(ui, room),
                            Page::Credits | Page::Licence => self.text_page(ui, page, room),
                        })
                        .inner
                });
                inside
                    .response
                    .widget_info(|| WidgetInfo::labeled(Role::Dialog, true, &name));
                (inside.inner, closing_click(&inside.response))
            });
        let (step, clicked_inside) = modal.inner;
        let escape = modal.is_top_modal
            && !modal.any_popup_open
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let close = closing_click(&modal.backdrop_response)
            || (page == Page::About && (escape || clicked_inside));
        // A step taken by keyboard moves the focus with it; one taken by a click doesn't.
        let by_keyboard = !ctx.input(|i| i.pointer.any_click());
        match step {
            Step::To(next) => {
                self.page = Some(next);
                self.focus = by_keyboard.then_some(next);
            }
            Step::Back => self.back(page, by_keyboard),
            Step::Stay if close => self.page = None,
            Step::Stay if escape => self.back(page, true),
            Step::Stay => {}
        }
        if self.page != Some(page) {
            ctx.request_repaint();
        }
    }

    /// Back from `page` to About, the button to `page` focused when the user came back
    /// by keyboard.
    fn back(&mut self, page: Page, focus: bool) {
        self.page = Some(Page::About);
        self.focus = focus.then_some(page);
    }

    /// Whether the button for `page` is to take the keyboard focus now.
    fn take_focus(&mut self, page: Page) -> bool {
        let take = self.focus == Some(page);
        if take {
            self.focus = None;
        }
        take
    }

    /// About: the logo, the version, the copyright and the licence, centred, and the
    /// buttons to Credits and Licence; no wider than `room`.
    fn about(&mut self, ui: &mut egui::Ui, room: egui::Vec2) -> Step {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        ui.set_width((ABOUT_WIDTH.min(room.x) - 2.0 * t.spacing.xxl).max(0.0));
        // Text takes no clicks, so a click on it closes the dialog.
        ui.style_mut().interaction.selectable_labels = false;
        ui.vertical_centered(|ui| {
            let width = LOGO_WIDTH.min(ui.available_width());
            logo(&t).show(ui, width, &tr(&ctx, "app-name"));
            ui.add_space(t.spacing.xl);
            let mut args = Args::new();
            args.set("version", self.version.as_str());
            let version = translator(&ctx).text_with("about-version", Some(&args));
            let medium = egui::FontFamily::Name(fonts::MEDIUM.into());
            ui.label(
                egui::RichText::new(version)
                    .size(t.text.title)
                    .family(medium)
                    .color(t.colors.text),
            );
            ui.add_space(t.spacing.s);
            for id in ["about-copyright", "about-free-software"] {
                let line = egui::RichText::new(tr(&ctx, id))
                    .size(t.text.small)
                    .color(t.colors.text_secondary);
                ui.label(line);
            }
            ui.add_space(t.spacing.xl);
            self.buttons(ui)
        })
        .inner
    }

    /// Credits and Licence, side by side and centred together.
    fn buttons(&mut self, ui: &mut egui::Ui) -> Step {
        let pages = [
            (Page::Credits, "about-credits"),
            (Page::Licence, "about-licence"),
        ];
        let names = pages.map(|(_, id)| tr(ui.ctx(), id));
        let font = egui::TextStyle::Button.resolve(ui.style());
        let padding = ui.spacing().button_padding.x;
        let gap = ui.spacing().item_spacing.x;
        let width: f32 = names
            .iter()
            .map(|name| {
                let text = ui.painter().layout_no_wrap(
                    name.clone(),
                    font.clone(),
                    egui::Color32::PLACEHOLDER,
                );
                text.size().x + 2.0 * padding
            })
            .sum::<f32>()
            + gap;
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
            let mut step = Step::Stay;
            for ((page, _), name) in pages.into_iter().zip(&names) {
                let button = ui.button(name);
                if self.take_focus(page) {
                    button.request_focus();
                }
                if button.clicked() {
                    step = Step::To(page);
                }
            }
            step
        })
        .inner
    }

    /// Credits or Licence: a back button and the title, over the text, scrolling; the
    /// whole page within `room`.
    fn text_page(&mut self, ui: &mut egui::Ui, page: Page, room: egui::Vec2) -> Step {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let margins = 2.0 * t.spacing.xxl;
        ui.set_width((PAGE_WIDTH.min(room.x) - margins).max(0.0));
        let title = match page {
            Page::Licence => "about-licence",
            _ => "about-credits",
        };
        let header = ui.horizontal(|ui| {
            let back = IconButton::new(Icon::Back, &tr(&ctx, "about-back")).show(ui);
            if self.take_focus(page) {
                back.request_focus();
            }
            ui.heading(tr(&ctx, title));
            if back.clicked() {
                Step::Back
            } else {
                Step::Stay
            }
        });
        ui.add_space(t.spacing.m);
        // The text takes what the window leaves under the header, up to PAGE_HEIGHT, and
        // always that much: egui hands a dialog's contents only the height the dialog had
        // in the last frame, About's after a page change, so a maximum alone would keep it.
        let chrome = margins + header.response.rect.height() + t.spacing.m;
        let height = PAGE_HEIGHT.min(room.y - chrome).max(t.sizes.row);
        let area = egui::ScrollArea::vertical()
            .id_salt(title)
            .min_scrolled_height(height)
            .max_height(height)
            .auto_shrink([false, true]);
        quiet_scroll(ui, area, |ui| match page {
            Page::Licence => {
                // Its line breaks kept, in the body font: the interface has no monospace.
                ui.label(egui::RichText::new(LICENCE).color(t.colors.text));
            }
            _ => {
                CommonMarkViewer::new().show(ui, &mut self.markdown, credits());
            }
        });
        header.inner
    }
}

/// The full logo in `t`'s colours: paper letters on a dark theme, ink on a light one.
fn logo(t: &apertrust_ui::Theme) -> Picture {
    if t.dark { LOGO_DARK } else { LOGO }
}

/// Whether `response` is a click that closes the dialog: not the second of a
/// double-click, whose first opened the dialog or a page under the pointer.
fn closing_click(response: &egui::Response) -> bool {
    response.clicked() && !response.double_clicked()
}

/// CREDITS.md without its title, which the page's header gives.
fn credits() -> &'static str {
    CREDITS
        .split_once('\n')
        .filter(|(first, _)| first.starts_with("# "))
        .map_or(CREDITS, |(_, rest)| rest.trim_start())
}

/// Where a page's buttons ask to go.
enum Step {
    Stay,
    To(Page),
    Back,
}

#[cfg(test)]
mod tests {
    use apertrust_ui::{Builtin, Theme};
    use eframe::egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use super::*;

    /// The dialog open on `page` as it opens, over the panel's colour, in a window `size`
    /// points large in the theme `which`; its version fixed, so its snapshots hold from
    /// one commit to the next.
    fn dialog(which: Builtin, size: egui::Vec2, page: Page) -> Harness<'static, About> {
        let about = About {
            page: Some(page),
            version: "0.1.0+197~g9201df0".to_owned(),
            ..About::default()
        };
        Harness::builder()
            .with_size(size)
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui_state(
                |ui, about: &mut About| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                    about.show(ui.ctx());
                },
                about,
            )
    }

    fn photograph(which: Builtin, name: &str) {
        let mut harness = dialog(which, egui::vec2(520.0, 320.0), Page::About);
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn about_in_the_dark_theme() {
        photograph(Builtin::Dark, "about-dark");
    }

    #[test]
    fn about_in_the_light_theme() {
        photograph(Builtin::Light, "about-light");
    }

    #[test]
    fn the_logo_is_drawn_from_the_theme_s_file() {
        for (which, file) in [
            (Builtin::Light, "apertrust-logo"),
            (Builtin::Dark, "apertrust-logo-dark"),
            (Builtin::HighContrast, "apertrust-logo-dark"),
        ] {
            let mut harness = dialog(which, egui::vec2(800.0, 600.0), Page::About);
            harness.run();
            let logo = harness.get_by_role_and_label(Role::Image, "Apertrust");
            assert!(logo.rect().height() > 0.0, "{which:?}");
            // The pictures rendered, by their textures' names: the theme's file alone.
            let pictures: Vec<String> = harness
                .ctx
                .tex_manager()
                .read()
                .allocated()
                .map(|(_, meta)| meta.name.clone())
                .filter(|name| name.starts_with("picture "))
                .collect();
            assert_eq!(pictures.len(), 1, "{which:?}: {pictures:?}");
            assert!(
                pictures[0].starts_with(&format!("picture {file} ")),
                "{which:?}: {pictures:?}"
            );
        }
    }

    #[test]
    fn every_page_fits_the_smallest_window() {
        // 500 by 600 pixels at an interface scale of 1.5.
        let size = egui::vec2(500.0, 600.0) / 1.5;
        for page in [Page::About, Page::Credits, Page::Licence] {
            let mut harness = dialog(Builtin::Dark, size, page);
            harness.run();
            let window = harness.ctx.content_rect();
            let shown = harness
                .get_by_role_and_label(Role::Dialog, "About Apertrust")
                .rect();
            assert!(
                window.contains_rect(shown),
                "{page:?}: {shown:?} in {window:?}"
            );
        }
    }
}
