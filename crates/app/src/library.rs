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

//! What the open folder looks like (`docs/gui.md`, The side panel, The central panel's
//! states): the Library tab's Folders section, with Albums and Smart albums drawn disabled
//! until the library comes, and the empty states of the central panel and the grid panel.
//! Both draw from the mirror's [`Folder`] and say what the person asked for as [`Action`]s,
//! which the app carries out.

use std::path::{Path, PathBuf};

use apertrust_ui::components::{
    ActionRow, Button, Section, SectionAction, focus_ring_centred, group_label,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::{theme, tr, translator};
use eframe::egui::{self, Align, Layout, Rect, Sense, UiBuilder, vec2};

use crate::folder::{Folder, Listing};

/// The Folders section's id, by which settings keep whether it is open.
pub const ID: &str = "folders";

/// Whether the section is open when settings have not kept it otherwise.
pub const OPEN: bool = true;

/// What the person asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// File › Open Folder…'s dialog.
    OpenFolder,
    /// File › Open Image…'s dialog.
    OpenImage,
    /// A folder, a subfolder of the open one or a recent one, opened at once.
    Open(PathBuf),
    /// The section's header: open or close it.
    Toggle,
}

/// A recent folder, with why it could not be opened when it was tried, if it could not.
#[derive(Clone, Copy, Debug)]
pub struct Recent<'a> {
    pub path: &'a Path,
    pub missing: Option<&'a str>,
}

/// The keys of the two ways to open, as the platform writes them, where the keymap has any.
#[derive(Clone, Debug, Default)]
pub struct Keys {
    pub folder: Option<String>,
    pub image: Option<String>,
}

/// What the central panel says while no photo is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Empty<'a> {
    /// Nothing has been opened: where to start.
    Start,
    /// The folder is being read.
    Reading(&'a Path),
    /// It could not be read, or its reading was cancelled (`None`).
    Failed(&'a Path, Option<&'a str>),
    /// It has no raws.
    NoRaws(&'a str),
    /// It has photos, and none is open.
    NoneChosen,
}

impl<'a> Empty<'a> {
    /// What `folder` makes of an empty central panel.
    pub fn of(folder: &'a Folder) -> Empty<'a> {
        match folder {
            Folder::None => Empty::Start,
            Folder::Opening { path } => Empty::Reading(path),
            Folder::Failed { path, reason } => Empty::Failed(path, reason.as_deref()),
            Folder::Open(listing) if !listing.done => Empty::Reading(&listing.path),
            Folder::Open(listing) if listing.photos.is_empty() => Empty::NoRaws(&listing.name),
            Folder::Open(_) => Empty::NoneChosen,
        }
    }

    /// What the grid panel says with it: "No raw photos in …" where the folder has none, and
    /// nothing else, since it is empty in every other state, as gui.md has it.
    pub fn grid_says(&self, ctx: &egui::Context) -> Option<String> {
        match self {
            Empty::NoRaws(name) => Some(with_folder(ctx, "folder-empty", name)),
            _ => None,
        }
    }
}

fn with_folder(ctx: &egui::Context, id: &str, folder: &str) -> String {
    let mut args = Args::new();
    args.set("folder", folder);
    translator(ctx).text_with(id, Some(&args))
}

/// The open folder's own line: its name with its photo count, or that it is being read.
fn current(ctx: &egui::Context, folder: &Folder) -> Option<String> {
    let words = translator(ctx);
    match folder {
        Folder::None | Folder::Failed { .. } => None,
        Folder::Opening { path } => Some(with_folder(ctx, "folders-reading", &name(path))),
        Folder::Open(listing) if !listing.done => {
            Some(with_folder(ctx, "folders-reading", &listing.name))
        }
        Folder::Open(listing) => {
            let mut args = Args::new();
            args.set("name", listing.name.as_str());
            args.set("count", listing.photos.len() as f64);
            Some(words.text_with("folders-current", Some(&args)))
        }
    }
}

/// A folder's last part, or the whole path where it has none.
fn name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// One folder in a list: a line, its name cut with … where it is too long, the rows' `hover`
/// band under the pointer and the rows' focus ring, both 4 points beyond it at each end; the
/// whole path in its tooltip, disabled with `missing` as its tooltip when it could not be
/// opened.
fn row(ui: &mut egui::Ui, label: &str, tip: &str, missing: Option<&str>) -> bool {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let response = ui
        .add_enabled_ui(missing.is_none(), |ui| {
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let band = rect.expand2(vec2(t.spacing.xs, 0.0));
                if response.hovered() && ui.is_enabled() {
                    painter.rect_filled(band, t.shape.radius, t.colors.hover);
                }
                let font = egui::FontId::proportional(t.text.body);
                let mut job =
                    egui::text::LayoutJob::simple_singleline(label.to_owned(), font, t.colors.text);
                job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width());
                let galley = painter.layout_job(job);
                let at = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
                painter.galley(at, galley, t.colors.text);
                if response.has_focus() {
                    focus_ring_centred(painter, band, &t);
                }
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label)
            });
            response
        })
        .inner;
    match missing {
        Some(why) => {
            let mut args = Args::new();
            args.set("reason", why);
            let text = translator(ui.ctx()).text_with("folders-recent-missing", Some(&args));
            response.on_disabled_hover_text(format!("{tip}\n{text}"));
            false
        }
        None => response.on_hover_text(tip).clicked(),
    }
}

/// The Library tab's Folders section: Open Folder…, the open folder with its photo count,
/// its subfolders and the recent folders; open or closed as `open` says. Albums and Smart
/// albums follow, drawn disabled.
pub fn library(
    ui: &mut egui::Ui,
    folder: &Folder,
    recent: &[Recent<'_>],
    open: bool,
) -> Vec<Action> {
    let t = theme(ui.ctx());
    let mut actions = Vec::new();
    let title = tr(ui.ctx(), "section-folders");
    let out = Section::new(&title, open).options(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(t.spacing.s);
        let margin = t.spacing.l;
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(margin as i8, 0))
            .show(ui, |ui| {
                if ActionRow::new(&tr(ui.ctx(), "action-open-folder")).show(ui) {
                    actions.push(Action::OpenFolder);
                }
                if let Some(line) = current(ui.ctx(), folder) {
                    ui.add_space(t.spacing.s);
                    ui.label(egui::RichText::new(line).strong());
                }
                if let Folder::Open(Listing {
                    path, subfolders, ..
                }) = folder
                    && !subfolders.is_empty()
                {
                    group_label(ui, &tr(ui.ctx(), "folders-subfolders"));
                    for sub in subfolders {
                        let full = path.join(sub);
                        if row(ui, sub, &full.display().to_string(), None) {
                            actions.push(Action::Open(full));
                        }
                    }
                }
                if !recent.is_empty() {
                    group_label(ui, &tr(ui.ctx(), "folders-recent"));
                    for entry in recent {
                        let tip = entry.path.display().to_string();
                        if row(ui, &name(entry.path), &tip, entry.missing) {
                            actions.push(Action::Open(entry.path.to_owned()));
                        }
                    }
                }
                ui.add_space(t.spacing.m);
            });
    });
    if out.actions.contains(&SectionAction::Toggle) {
        actions.push(Action::Toggle);
    }
    // Albums and Smart albums come with the library: their headers only, disabled.
    for id in ["section-albums", "section-smart-albums"] {
        let title = tr(ui.ctx(), id);
        let later = tr(ui.ctx(), "section-tab-later");
        ui.scope_builder(UiBuilder::new().sense(Sense::hover()), |ui| {
            ui.add_enabled_ui(false, |ui| {
                Section::new(&title, false).options(false).show(ui, |_| {});
            });
        })
        .response
        .on_disabled_hover_text(later);
    }
    actions
}

/// The central panel's empty state in the middle of `slot`: "No photo selected" alone on the
/// surround, in its text colour; else a card with the heading, and under the start's the two
/// ways to open with their keys and the recent folders, under a folder with no raws Open
/// Folder… again.
pub fn empty_state(
    ui: &mut egui::Ui,
    slot: Rect,
    what: Empty<'_>,
    recent: &[Recent<'_>],
    keys: &Keys,
) -> Vec<Action> {
    let t = theme(ui.ctx());
    let words = translator(ui.ctx());
    let ctx = ui.ctx().clone();
    let heading = match what {
        Empty::Start => words.text("start-title"),
        Empty::Reading(path) => with_folder(&ctx, "folder-reading", &name(path)),
        Empty::Failed(path, reason) => {
            let shown = path.display().to_string();
            match reason {
                Some(reason) => {
                    let mut args = Args::new();
                    args.set("folder", shown.as_str());
                    args.set("reason", reason);
                    words.text_with("folder-failed", Some(&args))
                }
                None => with_folder(&ctx, "folder-cancelled", &shown),
            }
        }
        Empty::NoRaws(name) => with_folder(&ctx, "folder-empty", name),
        Empty::NoneChosen => words.text("folder-no-photo"),
    };
    let buttons = matches!(what, Empty::Start | Empty::NoRaws(_) | Empty::Failed(..));
    let recents = matches!(what, Empty::Start) && !recent.is_empty();
    // A line alone where nothing is chosen yet; the card where there is more to say or do.
    let boxed = !matches!(what, Empty::NoneChosen);

    let width = (slot.width() - 2.0 * t.spacing.xl).clamp(0.0, 420.0);
    let margin = t.spacing.l;
    // The card's height from its parts, so it can be centred without a layout pass.
    let line = t.sizes.panel_line;
    let mut height = 2.0 * margin + t.text.heading * 1.6;
    if buttons {
        height += t.spacing.l + 2.0 * (t.sizes.control + t.spacing.s);
    }
    if recents {
        height += t.spacing.l + t.text.caption * 1.6 + recent.len() as f32 * line;
    }
    let height = height.min(slot.height() - 2.0 * t.spacing.m).max(0.0);
    let card = Rect::from_center_size(slot.center(), vec2(width, height));
    let mut actions = Vec::new();
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(card)
            .layout(Layout::top_down(Align::Center)),
        |ui| {
            if boxed {
                ui.painter()
                    .rect_filled(card, t.shape.radius, t.colors.raised);
            }
            ui.add_space(margin);
            ui.spacing_mut().item_spacing.y = t.spacing.s;
            ui.set_width(width);
            ui.add(
                egui::Label::new(egui::RichText::new(&heading).size(t.text.heading).color(
                    if boxed {
                        t.colors.text
                    } else {
                        t.colors.text_on_surround()
                    },
                ))
                .wrap(),
            );
            if buttons {
                ui.add_space(t.spacing.s);
                for (id, keys, action) in [
                    ("action-open-folder", &keys.folder, Action::OpenFolder),
                    ("action-open", &keys.image, Action::OpenImage),
                ] {
                    let words = tr(ui.ctx(), id);
                    let mut button = Button::new(&words).width(width - 2.0 * margin);
                    if let Some(keys) = keys {
                        button = button.keys(keys);
                    }
                    if button.show(ui).clicked() {
                        actions.push(action);
                    }
                }
            }
            if recents {
                let inset = egui::Margin::symmetric(margin as i8, 0);
                egui::Frame::new().inner_margin(inset).show(ui, |ui| {
                    ui.set_width(width - 2.0 * margin);
                    group_label(ui, &tr(ui.ctx(), "start-recent"));
                    for entry in recent {
                        let tip = entry.path.display().to_string();
                        if row(ui, &name(entry.path), &tip, entry.missing) {
                            actions.push(Action::Open(entry.path.to_owned()));
                        }
                    }
                });
            }
        },
    );
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::folder::Listing;

    fn listing(photos: usize, done: bool) -> Folder {
        let photo = |i: usize| apertrust_engine::session::library::Photo {
            name: format!("IMG_{i}.ARW"),
            size: 1,
            modified: None,
            edit: apertrust_engine::session::library::EditState::None,
        };
        Folder::Open(Listing {
            name: "Wedding".to_owned(),
            path: PathBuf::from("/photos/Wedding"),
            subfolders: Vec::new(),
            photos: (0..photos).map(photo).collect(),
            done,
            taken: None,
            thumbnails: Default::default(),
        })
    }

    #[test]
    fn the_empty_state_follows_the_folder() {
        assert_eq!(Empty::of(&Folder::None), Empty::Start);
        let reading = Folder::Opening {
            path: PathBuf::from("/photos/a"),
        };
        assert_eq!(Empty::of(&reading), Empty::Reading(Path::new("/photos/a")));
        let failed = Folder::Failed {
            path: PathBuf::from("/photos/a"),
            reason: Some("gone".to_owned()),
        };
        assert_eq!(
            Empty::of(&failed),
            Empty::Failed(Path::new("/photos/a"), Some("gone"))
        );
        // Listed but for a chunk: still reading; then none, or some.
        assert!(matches!(Empty::of(&listing(0, false)), Empty::Reading(_)));
        assert_eq!(Empty::of(&listing(0, true)), Empty::NoRaws("Wedding"));
        assert_eq!(Empty::of(&listing(3, true)), Empty::NoneChosen);
    }

    use apertrust_ui::components::PanelBody;
    use apertrust_ui::{Locale, Theme};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    /// An open folder with subfolders and every photo listed.
    fn wedding() -> Folder {
        let Folder::Open(mut listing) = listing(120, true) else {
            unreachable!()
        };
        listing.subfolders = vec!["Edits".to_owned(), "Selects".to_owned()];
        Folder::Open(listing)
    }

    /// Two recent folders, the second one that could not be opened.
    fn recent() -> Vec<Recent<'static>> {
        vec![
            Recent {
                path: Path::new("/photos/Portraits"),
                missing: None,
            },
            Recent {
                path: Path::new("/photos/Gone"),
                missing: Some("the folder is missing"),
            },
        ]
    }

    fn harness(
        draw: impl Fn(&mut egui::Ui, &mut Vec<Action>) + 'static,
    ) -> Harness<'static, Vec<Action>> {
        sized(egui::vec2(320.0, 700.0), draw)
    }

    fn sized(
        size: egui::Vec2,
        draw: impl Fn(&mut egui::Ui, &mut Vec<Action>) + 'static,
    ) -> Harness<'static, Vec<Action>> {
        Harness::builder()
            .with_size(size)
            .with_context_setup(|ctx| {
                apertrust_ui::install(ctx, Theme::default());
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui_state(
                move |ui, actions: &mut Vec<Action>| draw(ui, actions),
                Vec::new(),
            )
    }

    #[test]
    fn the_folders_section_names_the_folder_lists_its_subfolders_and_opens_them() {
        let mut harness = harness(|ui, actions| {
            PanelBody::new().show(ui, |ui| {
                actions.extend(library(ui, &wedding(), &recent(), true));
            });
        });
        harness.run();
        // The folder's name with how many photos it holds.
        harness.get_by_label("Wedding · 120 photos");
        harness.get_by_label("Edits").click();
        harness.run();
        harness.get_by_label("Open Folder…").click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [
                Action::Open(PathBuf::from("/photos/Wedding/Edits")),
                Action::OpenFolder
            ]
        );
    }

    #[test]
    fn a_recent_folder_opens_and_one_that_failed_is_drawn_disabled() {
        let mut harness = harness(|ui, actions| {
            PanelBody::new().show(ui, |ui| {
                actions.extend(library(ui, &Folder::None, &recent(), true));
            });
        });
        harness.run();
        let gone = harness.get_by_label("Gone");
        assert!(gone.accesskit_node().is_disabled());
        let portraits = harness.get_by_label("Portraits");
        assert!(!portraits.accesskit_node().is_disabled());
        portraits.click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [Action::Open(PathBuf::from("/photos/Portraits"))]
        );
    }

    #[test]
    fn albums_and_smart_albums_are_drawn_disabled_and_the_folders_header_toggles() {
        let mut harness = harness(|ui, actions| {
            PanelBody::new().show(ui, |ui| {
                actions.extend(library(ui, &Folder::None, &[], true));
            });
        });
        harness.run();
        for title in ["Albums", "Smart albums"] {
            let section = harness.get_by_role_and_label(egui::accesskit::Role::Button, title);
            assert!(section.accesskit_node().is_disabled(), "{title}");
        }
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Folders")
            .click();
        harness.run();
        assert_eq!(*harness.state(), [Action::Toggle]);
    }

    /// The central panel's card over a slot, with what it asked for.
    fn card(what: Folder, recent: Vec<(PathBuf, Option<String>)>) -> Harness<'static, Vec<Action>> {
        card_sized(egui::vec2(320.0, 700.0), what, recent)
    }

    fn card_sized(
        size: egui::Vec2,
        what: Folder,
        recent: Vec<(PathBuf, Option<String>)>,
    ) -> Harness<'static, Vec<Action>> {
        sized(size, move |ui, actions| {
            let slot = ui.ctx().content_rect();
            let surround = theme(ui.ctx()).colors.photo_surround;
            ui.painter().rect_filled(slot, 0.0, surround);
            let recent: Vec<Recent<'_>> = recent
                .iter()
                .map(|(path, missing)| Recent {
                    path,
                    missing: missing.as_deref(),
                })
                .collect();
            let keys = Keys {
                folder: Some("Ctrl+Shift+O".to_owned()),
                image: Some("Ctrl+O".to_owned()),
            };
            actions.extend(empty_state(ui, slot, Empty::of(&what), &recent, &keys));
        })
    }

    #[test]
    fn with_nothing_open_the_card_offers_both_ways_to_open_and_the_recent_folders() {
        let recent = vec![
            (PathBuf::from("/photos/Portraits"), None),
            (PathBuf::from("/photos/Gone"), Some("missing".to_owned())),
        ];
        let mut harness = card(Folder::None, recent);
        harness.run();
        harness.get_by_label("Open a folder to start");
        let folder = harness.get_by_label("Open Folder…");
        // The keys are the button's shortcut, not part of its name.
        assert_eq!(
            folder.accesskit_node().data().keyboard_shortcut(),
            Some("Ctrl+Shift+O")
        );
        folder.click();
        harness.run();
        harness.get_by_label("Open Image…").click();
        harness.run();
        harness.get_by_label("Portraits").click();
        harness.run();
        assert!(harness.get_by_label("Gone").accesskit_node().is_disabled());
        assert_eq!(
            *harness.state(),
            [
                Action::OpenFolder,
                Action::OpenImage,
                Action::Open(PathBuf::from("/photos/Portraits"))
            ]
        );
    }

    #[test]
    fn a_folder_with_no_raws_a_failed_one_and_one_with_no_photo_chosen_each_say_so() {
        let mut empty = card(listing(0, true), Vec::new());
        empty.run();
        empty.get_by_label("No raw photos in Wedding");
        empty.get_by_label("Open Folder…");

        let failed = Folder::Failed {
            path: PathBuf::from("/photos/gone"),
            reason: Some("the folder is missing".to_owned()),
        };
        let mut failed = card(failed, Vec::new());
        failed.run();
        failed.get_by_label("Could not open /photos/gone: the folder is missing");

        let cancelled = Folder::Failed {
            path: PathBuf::from("/photos/slow"),
            reason: None,
        };
        let mut cancelled = card(cancelled, Vec::new());
        cancelled.run();
        cancelled.get_by_label("Reading /photos/slow was cancelled");

        let mut chosen = card(wedding(), Vec::new());
        chosen.run();
        // A line alone: nothing more to read, and no card drawn under it.
        chosen.get_by_label("No photo selected");
        assert!(
            chosen
                .query_by_label("Choose one in the grid panel")
                .is_none()
        );
        let raised = theme(&chosen.ctx).colors.raised;
        let filled = |shape: &egui::epaint::ClippedShape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == raised);
        assert!(!chosen.output().shapes.iter().any(filled));
        // On the surround, so in the surround's text colour, which reads on every background.
        let colour = chosen
            .output()
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "No photo selected" => {
                    text.galley.job.sections.first().map(|s| s.format.color)
                }
                _ => None,
            });
        let on_surround = theme(&chosen.ctx).colors.text_on_surround();
        assert_eq!(colour, Some(on_surround));
        let mut start = card(Folder::None, Vec::new());
        start.run();
        assert!(
            start.output().shapes.iter().any(filled),
            "the start keeps its card"
        );
    }

    #[test]
    fn the_grid_panel_says_a_folder_has_no_raws_and_nothing_in_the_other_states() {
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        assert_eq!(
            Empty::of(&listing(0, true)).grid_says(&ctx).as_deref(),
            Some("No raw photos in Wedding")
        );
        for folder in [Folder::None, wedding(), listing(0, false)] {
            assert_eq!(Empty::of(&folder).grid_says(&ctx), None);
        }
    }

    /// The Library tab and the start card photographed, as the app draws them.
    #[test]
    fn the_library_tab_and_the_start_card_are_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        let mut tab = sized(egui::vec2(320.0, 520.0), |ui, actions| {
            let panel = theme(ui.ctx()).colors.panel;
            ui.painter().rect_filled(ui.max_rect(), 0.0, panel);
            PanelBody::new().show(ui, |ui| {
                actions.extend(library(ui, &wedding(), &recent(), true));
            });
        });
        tab.run();
        tab.snapshot("library-tab");
        results.extend(tab.take_snapshot_results());

        let recent = vec![
            (PathBuf::from("/photos/Portraits"), None),
            (PathBuf::from("/photos/Gone"), Some("missing".to_owned())),
        ];
        let mut card = card_sized(egui::vec2(640.0, 360.0), Folder::None, recent);
        card.run();
        card.snapshot("start-card");
        results.extend(card.take_snapshot_results());
    }
}
