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

use std::path::PathBuf;

use apertrust_engine::session::{Event, Photo, Selection};
use apertrust_tools::Reset;
use apertrust_ui::components as parts;
use apertrust_ui::{Builtin, Locale, translator};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::metadata::tests::photo;
use crate::mirror::PhotoState;
use crate::settings::Settings;
use crate::tools::Head;

/// The Lisbon photo with its white balance recorded as auto.
fn recorded() -> Photo {
    let mut photo = photo();
    photo.exif.white_balance = Some(0);
    photo
}

fn card(photo: &Photo) -> Card {
    Card::of(photo, &Translator::new(Locale::EnGb))
}

fn values(card: &Card) -> Vec<Option<&str>> {
    card.strip
        .iter()
        .map(|cell| cell.value.as_deref())
        .collect()
}

/// A photo with every value: each line in the Metadata tool's words, what the tooltips add.
#[test]
fn the_lines_of_a_photo_with_every_value() {
    let card = card(&recorded());
    assert_eq!(card.name.value.as_deref(), Some("R0001225.DNG"));
    assert_eq!(
        card.name.tip,
        "/Users/ana/Pictures/2026-07 Lisbon/R0001225.DNG"
    );
    // The offset and the sub-seconds in the date's tooltip only.
    assert_eq!(
        card.date.value.as_deref(),
        Some("Sat, 4 Jul 2026, 08:10:01")
    );
    assert!(
        card.date
            .tip
            .starts_with("Sat, 4 Jul 2026, 08:10:01 (+01:00)\n")
    );
    assert!(card.date.tip.contains("08:10:01.120"), "{}", card.date.tip);
    assert_eq!(card.camera, "RICOH GR IV");
    let (tag, tip) = card.white_balance.clone().unwrap();
    assert_eq!(tag, "AWB");
    assert!(tip.starts_with("Auto\n"), "{tip}");
    assert_eq!(
        card.flash.as_ref().map(|(flash, _)| *flash),
        Some(Flash::NotFired)
    );
    assert_eq!(card.lens.value.as_deref(), Some("GR Lens 18.3mm F2.8"));
    assert_eq!(
        (card.dimensions.as_str(), card.weight.as_str()),
        ("6000 × 4000", "· 32 MB")
    );
    assert_eq!(
        card.format,
        ("DNG".to_owned(), "Digital Negative".to_owned())
    );
    assert_eq!(
        values(&card),
        [
            Some("ISO 200"),
            Some("18.3 mm"),
            Some("−0.33 EV"),
            Some("f/5.6"),
            Some("1/500 s")
        ]
    );
    // Each value's tooltip names it and its tag, the focal length's its 35 mm equivalent.
    assert_eq!(
        card.strip[1].tip,
        "18.3 mm\n35 mm equivalent: 28 mm\nFocalLength, EXIF 0x920A"
    );
}

/// No lens, no flash function, no white balance and no exposure bias: the lens line says
/// so, the camera line's end holds nothing, the strip's cell is "—" saying why.
#[test]
fn the_lines_with_no_lens_no_flash_no_white_balance_and_no_exposure_bias() {
    let mut photo = photo();
    photo.exif.lens = None;
    photo.exif.flash = Some(0x20);
    photo.exif.exposure_bias = None;
    let card = card(&photo);
    assert_eq!(card.lens.value, None);
    assert_eq!(
        card.lens_line(&Translator::new(Locale::EnGb))
            .value
            .as_deref(),
        Some("No lens recorded")
    );
    assert_eq!(card.flash, None);
    assert_eq!(card.white_balance, None);
    assert_eq!(card.strip[2].value, None);
    assert!(
        card.strip[2]
            .tip
            .starts_with("The camera recorded no exposure bias"),
        "{}",
        card.strip[2].tip
    );
}

/// A value the file's format keeps from Apertrust says so, as the Metadata tool's row does.
#[test]
fn a_value_not_read_from_the_format_says_so() {
    let mut photo = photo();
    photo.exif.exposure_bias = None;
    photo.exif.unread = vec!["exposure_bias"];
    let card = card(&photo);
    assert!(
        card.strip[2]
            .tip
            .starts_with("Apertrust does not read this from DNG files yet\n"),
        "{}",
        card.strip[2].tip
    );
}

/// The flash fired, did not, or is not there (no flash function, or nothing recorded);
/// the white balance auto or manual.
#[test]
fn the_flash_s_three_states_and_the_white_balance_s_two() {
    let flash = |bits: Option<u16>| {
        let mut photo = photo();
        photo.exif.flash = bits;
        card(&photo).flash.map(|(flash, _)| flash)
    };
    assert_eq!(flash(Some(0x19)), Some(Flash::Fired));
    assert_eq!(flash(Some(0x10)), Some(Flash::NotFired));
    assert_eq!(flash(Some(0x20)), None);
    assert_eq!(flash(None), None);
    let balance = |code: Option<u16>| {
        let mut photo = photo();
        photo.exif.white_balance = code;
        card(&photo).white_balance.map(|(tag, _)| tag)
    };
    assert_eq!(balance(Some(0)).as_deref(), Some("AWB"));
    assert_eq!(balance(Some(1)).as_deref(), Some("WB"));
    assert_eq!(balance(None), None);
}

/// The maker's first word once, before a model that does not start with it.
#[test]
fn the_camera_names_its_maker_once() {
    let named = |make: &str, model: &str| camera(Some(make.to_owned()), Some(model.to_owned()));
    assert_eq!(
        named("RICOH IMAGING COMPANY, LTD.", "RICOH GR IV"),
        "RICOH GR IV"
    );
    assert_eq!(named("SONY", "ILCE-7RM5"), "SONY ILCE-7RM5");
    assert_eq!(named("NIKON CORPORATION", "NIKON Z 8"), "NIKON Z 8");
    assert_eq!(named("Canon", "Canon EOS R5"), "Canon EOS R5");
    assert_eq!(named("", "X-T5"), "X-T5");
    assert_eq!(named("FUJIFILM", ""), "FUJIFILM");
}

/// Copy Information: the lines as shown, "Label: value" a line; screen readers hear the
/// values in a sentence.
#[test]
fn copy_information_and_what_screen_readers_hear() {
    let card = card(&recorded());
    assert_eq!(
        card.copied,
        "File name: R0001225.DNG\n\
         Date taken: Sat, 4 Jul 2026, 08:10:01\n\
         Camera: RICOH GR IV\n\
         White balance: Auto\n\
         Flash: Off, did not fire\n\
         Lens: GR Lens 18.3mm F2.8\n\
         Size: 6000 × 4000 · 32 MB\n\
         Type: DNG\n\
         ISO: ISO 200\n\
         Focal length: 18.3 mm\n\
         Exposure bias: −0.33 EV\n\
         Aperture: f/5.6\n\
         Exposure time: 1/500 s"
    );
    assert_eq!(
        card.said,
        "R0001225.DNG, Saturday, 4 July 2026, 08:10, RICOH GR IV, White balance: Auto, \
         Flash: Off, did not fire, GR Lens 18.3mm F2.8, 6000 × 4000, 32 MB, ISO 200, \
         18.3 millimetres, −0.33 EV, f 5.6, 1/500 second"
    );
}

/// What the tool is drawn with, and what it asked for.
struct Scene {
    mirror: Mirror,
    out: crate::PhotosDrawn,
}

fn scene(photo: Option<Photo>) -> Scene {
    let mut mirror = crate::mirror::tests::mirror();
    if let Some(photo) = photo {
        mirror.photo = PhotoState::Open(Box::new(photo));
    }
    Scene {
        mirror,
        out: crate::PhotosDrawn::default(),
    }
}

fn drawn(which: Builtin, width: f32, state: Scene) -> Harness<'static, Scene> {
    crate::testing::drawn(which, (width, 260.0), state, |ui, scene: &mut Scene| {
        let settings = Settings::default();
        let (words, mut commands) = (translator(ui.ctx()), Vec::new());
        let head = Head {
            open: true,
            options: options(&words, &scene.mirror),
            enabled: true,
            ..Head::default()
        };
        let mut cx = Context {
            mirror: &scene.mirror,
            settings: &settings,
            words: &words,
            commands: &mut commands,
        };
        let mut out = crate::PhotosDrawn::default();
        parts::PanelBody::new().show(ui, |ui| {
            crate::info_section(ui, &mut cx, head, &mut out);
        });
        scene.out.copied = out.copied.or(scene.out.copied.take());
        scene.out.show_metadata |= out.show_metadata;
    })
}

fn region(harness: &Harness<'_, Scene>) -> String {
    let region = harness.get_by_role_and_label(Role::Region, "Info");
    region.accesskit_node().description().unwrap_or_default()
}

/// It declares no presets, no ＋ and no ↺; the card is a region named Info, described.
#[test]
fn the_card_is_a_region_described_and_the_tool_has_no_reset_or_presets() {
    let declared = definition();
    assert!(!declared.tool.presets && !declared.rows_shown);
    assert_eq!(declared.tool.reset, Reset::None);
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(recorded())));
    harness.run();
    assert!(region(&harness).starts_with("R0001225.DNG, Saturday, 4 July 2026, 08:10"));
}

/// It shows the photo open, `mirror.photo`, the active one, whatever is selected: with
/// three selected, where the Metadata tool describes no one photo.
#[test]
fn it_shows_the_active_photo_whatever_is_selected() {
    let mut state = scene(Some(recorded()));
    let photos: Vec<PathBuf> = ["R0001224.DNG", "R0001225.DNG", "R0001226.DNG"]
        .map(|name| PathBuf::from(format!("/Users/ana/Pictures/2026-07 Lisbon/{name}")))
        .to_vec();
    state.mirror.apply(Event::Selection(Selection {
        photos: photos.clone(),
        active: Some(photos[1].clone()),
        previous: false,
        through: 1,
    }));
    let shown = metadata::active_photo(&state.mirror).map(|photo| photo.path.clone());
    assert_eq!(shown, Some(photos[1].clone()));
    assert!(metadata::described(&state.mirror).is_none());
}

/// While the next photo opens, the card stays the photo left's, as the Tools panel keeps its
/// values, never "No photo"; the next photo's metadata replaces it.
#[test]
fn the_photo_left_stays_until_the_next_one_s_metadata() {
    let mut state = scene(None);
    let left = recorded();
    state.mirror.apply(Event::Opened(left.clone()));
    state
        .mirror
        .apply(Event::Values(crate::mirror::tests::report(0.0, left.open)));
    let next = PathBuf::from("/Users/ana/Pictures/2026-07 Lisbon/R0001226.DNG");
    state
        .mirror
        .opening(next.clone(), 9, std::time::Instant::now());
    let mut harness = drawn(Builtin::Dark, 320.0, state);
    harness.run();
    assert!(region(&harness).starts_with("R0001225.DNG, "));
    let opened = Photo {
        path: next,
        open: 9,
        ..recorded()
    };
    harness.state_mut().mirror.apply(Event::Opened(opened));
    harness.run();
    assert!(region(&harness).starts_with("R0001226.DNG, "));
}

/// No photo: one line, and Copy Information disabled.
#[test]
fn no_photo_says_so() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(None));
    harness.run();
    harness.get_by_label("No photo");
    assert!(harness.query_by_role(Role::Region).is_none());
    harness.get_by_label("Info options").click();
    harness.run();
    let copy = harness.get_by_label("Copy Information");
    assert!(copy.accesskit_node().is_disabled());
}

/// ⋯'s Copy Information copies the card; Show in Metadata asks for the Metadata tool.
#[test]
fn copy_information_and_show_in_metadata_from_the_options() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(recorded())));
    harness.run();
    harness.get_by_label("Info options").click();
    harness.run();
    harness.get_by_label("Copy Information").click();
    harness.run();
    let copied = harness.state().out.copied.clone().unwrap_or_default();
    assert!(copied.starts_with("File name: R0001225.DNG\n"), "{copied}");
    harness.get_by_label("Info options").click();
    harness.run();
    harness.get_by_label("Show in Metadata").click();
    harness.run();
    assert!(harness.state().out.show_metadata);
}

/// The card is one Tab stop; Ctrl+C on it copies what Copy Information copies.
#[test]
fn the_card_is_one_tab_stop_and_ctrl_c_copies_it() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(recorded())));
    harness.run();
    harness.get_by_role_and_label(Role::Region, "Info").focus();
    harness.run();
    assert!(
        harness
            .get_by_role_and_label(Role::Region, "Info")
            .accesskit_node()
            .is_focused()
    );
    harness.event(egui::Event::Copy);
    harness.step();
    let copied = &harness.output().platform_output.commands;
    let wanted = egui::OutputCommand::CopyText(card(&recorded()).copied);
    assert!(copied.contains(&wanted), "{copied:?}");
    harness.run();
    // Tab leaves it: the tags and tooltips take no focus.
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(
        !harness
            .get_by_role_and_label(Role::Region, "Info")
            .accesskit_node()
            .is_focused()
    );
}

/// Photographs the tool at `width` in the dark and light themes, `name-dark` and
/// `name-light`.
fn photograph(name: &str, width: f32, make: fn() -> Scene) {
    crate::testing::photograph(name, |which| drawn(which, width, make()), |_| {});
}

/// A photo with every value, the flash fired, at the Tools panel's width.
fn fired() -> Scene {
    let mut photo = recorded();
    photo.exif.flash = Some(0x19);
    scene(Some(photo))
}

#[test]
fn a_photo_is_photographed() {
    photograph("info-320", 320.0, fired);
}

/// At the Tools panel's narrowest, a long lens's name cut at its end, the strip's cells cut.
#[test]
fn the_narrowest_panel_is_photographed() {
    fn long() -> Scene {
        let mut photo = recorded();
        photo.exif.lens = Some("XF16-55mmF2.8 R LM WR II with a long name".to_owned());
        photo.exif.exposure_bias = None;
        scene(Some(photo))
    }
    photograph("info-240", 240.0, long);
}

#[test]
fn no_photo_is_photographed() {
    photograph("info-no-photo", 320.0, || scene(None));
}
