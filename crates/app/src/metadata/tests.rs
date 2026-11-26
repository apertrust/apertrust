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

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use apertrust_engine::session::Event;
use apertrust_engine::session::facts::{Facts, Summed, build};
use apertrust_engine::session::library::{DateTime, Shot};
use apertrust_engine::session::{Exif, Photo, Selection};
use apertrust_ui::{Builtin, Locale, translator};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::format::{Of, Shown, Source, fact, flash};
use super::*;
use crate::folder::{Folder, Listing};
use crate::mirror::{Mirror, PhotoState};
use crate::settings::Settings;
use crate::tools::Head;

const HOME: &str = "/Users/ana";
const FOLDER: &str = "/Users/ana/Pictures/2026-07 Lisbon";

/// A photo from the Lisbon folder whose EXIF records what the tool shows at first, a
/// location among it.
pub(crate) fn photo() -> Photo {
    let mut image = apertrust_tools::testing::image();
    if let Some(camera) = &mut image.camera {
        camera.iso = 200.0;
        camera.make = "Ricoh".to_owned();
        camera.alias = "GR IV".to_owned();
    }
    let exif = Exif {
        make_written: Some("RICOH IMAGING COMPANY, LTD.".to_owned()),
        model_written: Some("RICOH GR IV".to_owned()),
        taken: Some(DateTime {
            year: 2026,
            month: 7,
            day: 4,
            hour: 8,
            minute: 10,
            second: 1,
            millisecond: 120,
            offset_minutes: Some(60),
        }),
        exposure_time: Some(1.0 / 500.0),
        aperture: Some(5.6),
        focal_length: Some(18.3),
        exposure_bias: Some(-1.0 / 3.0),
        exposure_program: Some(3),
        metering_mode: Some(5),
        flash: Some(16),
        lens: Some("GR Lens 18.3mm F2.8".to_owned()),
        focal_length_35mm: Some(28.0),
        latitude: Some(38.71),
        longitude: Some(-9.13),
        elevation: Some(54.0),
        ..Exif::default()
    };
    Photo {
        path: PathBuf::from(format!("{FOLDER}/R0001225.DNG")),
        open: 1,
        camera: "Ricoh GR IV".to_owned(),
        width: 6000,
        height: 4000,
        view_only: None,
        exposure_time: exif.exposure_time,
        aperture: exif.aperture,
        focal_length: exif.focal_length,
        exif: Box::new(exif),
        format: "DNG".to_owned(),
        file_size: 32_400_000,
        sensor: [6064, 4040],
        image: Box::new(image),
    }
}

/// `photo`'s facts in `locale`.
fn shown(id: &str, photo: &Photo, locale: Locale) -> Shown {
    let words = Translator::new(locale);
    let source = Source {
        of: Of::photo(photo),
        history: None,
        home: Some(Path::new(HOME)),
        words: &words,
    };
    fact(id, &source)
}

fn value(id: &str, photo: &Photo) -> Option<String> {
    shown(id, photo, Locale::EnGb).value
}

/// Edited counts the person's steps applied: none undone, none of the defaults; Never
/// with none.
#[test]
fn edited_counts_the_steps_applied_but_the_defaults() {
    use apertrust_engine::session::{History, StepSource};

    use crate::history::tests::{history, step};
    let words = Translator::new(Locale::EnGb);
    let photo = photo();
    let edited = |history: &History| {
        let source = Source {
            of: Of::photo(&photo),
            history: Some(history),
            home: None,
            words: &words,
        };
        fact("edited", &source).value.unwrap_or_default()
    };
    let mut default = (*step(1, &["exposure"], None)).clone();
    default.source = StepSource::Default;
    let steps = vec![
        std::sync::Arc::new(default),
        step(2, &["exposure"], None),
        step(3, &["agx"], None),
        step(4, &["exposure"], None),
    ];
    let mut history = history(4, steps, None, None);
    assert!(
        edited(&history).ends_with(" · 3 steps"),
        "{}",
        edited(&history)
    );
    // The last undone.
    history.end = 3;
    assert!(
        edited(&history).ends_with(" · 2 steps"),
        "{}",
        edited(&history)
    );
    // The default step alone applied.
    history.end = 1;
    assert_eq!(edited(&history), "Never");
}

#[test]
fn every_fact_is_worded_from_the_photo() {
    let photo = photo();
    let rows = [
        ("file_name", "R0001225.DNG"),
        ("folder", "~/Pictures/2026-07 Lisbon"),
        ("type", "DNG"),
        ("size_on_disk", "32 MB"),
        ("dimensions", "6000 × 4000 · 24.0 MP"),
        ("make", "RICOH IMAGING COMPANY, LTD."),
        ("model", "RICOH GR IV"),
        ("date_taken", "Sat, 4 Jul 2026, 08:10:01 (+01:00)"),
        ("exposure_time", "1/500 s"),
        ("aperture", "f/5.6"),
        ("iso", "ISO 200"),
        ("exposure_bias", "−0.33 EV"),
        ("exposure_program", "Aperture priority"),
        ("metering_mode", "Multi-segment"),
        ("flash", "Off, did not fire"),
        ("lens", "GR Lens 18.3mm F2.8"),
        ("focal_length", "18.3 mm"),
        ("focal_length_35mm", "28 mm"),
        ("latitude", "38° 42.6′ N"),
        ("longitude", "9° 7.8′ W"),
        ("altitude", "54 m"),
    ];
    for (id, wanted) in rows {
        assert_eq!(value(id, &photo).as_deref(), Some(wanted), "{id}");
    }
    // The milliseconds in Date taken's tooltip, the readout in Dimensions'.
    let taken = shown("date_taken", &photo, Locale::EnGb);
    assert_eq!(
        taken.note.as_deref(),
        Some("To the millisecond: 08:10:01.120")
    );
    let size = shown("dimensions", &photo, Locale::EnGb);
    assert_eq!(size.note.as_deref(), Some("Sensor 6064 × 4040"));
    let latitude = shown("latitude", &photo, Locale::EnGb);
    assert_eq!(latitude.note.as_deref(), Some("38.710000"));
}

/// A file whose metadata cannot be read lists every field unread, and each fact read from
/// the EXIF finds its field there, so its tooltip says why it is empty; ISO is the
/// decoder's own.
#[test]
fn every_exif_fact_says_why_where_the_metadata_cannot_be_read() {
    let unread = Exif::unread_all().unread;
    let groups = ROWS
        .iter()
        .filter(|group| matches!(group.group, "camera" | "exposure" | "lens" | "gps"));
    for row in groups.flat_map(|group| group.rows.iter()) {
        if row.id != "iso" {
            assert!(unread.contains(&exif_field(row.id)), "{}", row.id);
        }
    }
}

#[test]
fn a_fact_the_photo_lacks_reads_none_and_the_make_falls_back() {
    let mut photo = photo();
    *photo.exif = Exif::default();
    for id in [
        "date_taken",
        "exposure_time",
        "subject_distance",
        "lens",
        "latitude",
        "crop_factor",
        "software",
    ] {
        assert_eq!(value(id, &photo), None, "{id}");
    }
    // The camera's names as rawler spells them where the file's cannot be read.
    assert_eq!(value("make", &photo).as_deref(), Some("Ricoh"));
    assert_eq!(value("model", &photo).as_deref(), Some("GR IV"));
    // The readout said only where it differs.
    photo.sensor = [6000, 4000];
    assert_eq!(shown("dimensions", &photo, Locale::EnGb).note, None);
}

#[test]
fn the_codes_read_in_words_and_an_unknown_one_says_so() {
    let mut photo = photo();
    photo.exif.exposure_mode = Some(2);
    photo.exif.white_balance = Some(1);
    photo.exif.light_source = Some(21);
    photo.exif.scene_type = Some(3);
    photo.exif.contrast = Some(1);
    photo.exif.colour_space = Some(65535);
    photo.exif.orientation_code = Some(6);
    assert_eq!(
        value("exposure_mode", &photo).as_deref(),
        Some("Auto bracket")
    );
    let balance = shown("white_balance", &photo, Locale::EnGb);
    assert_eq!(balance.value.as_deref(), Some("Manual"));
    assert_eq!(balance.note.as_deref(), Some("Light source: D65"));
    assert_eq!(value("scene_type", &photo).as_deref(), Some("Night scene"));
    assert_eq!(value("contrast", &photo).as_deref(), Some("Low"));
    assert_eq!(
        value("colour_space", &photo).as_deref(),
        Some("Uncalibrated")
    );
    assert_eq!(
        value("orientation", &photo).as_deref(),
        Some("Rotated 90° clockwise")
    );
    photo.exif.exposure_program = Some(7);
    assert_eq!(
        value("exposure_program", &photo).as_deref(),
        Some("Portrait mode")
    );
    photo.exif.exposure_program = Some(42);
    assert_eq!(
        value("exposure_program", &photo).as_deref(),
        Some("Unknown (42)")
    );
}

#[test]
fn the_flash_reads_from_its_bits() {
    let words = Translator::new(Locale::EnGb);
    let photo = photo();
    let source = Source {
        of: Of::photo(&photo),
        history: None,
        home: None,
        words: &words,
    };
    let read = |bits| flash(bits, &source);
    assert_eq!(read(0x10).value.as_deref(), Some("Off, did not fire"));
    assert_eq!(read(0x09).value.as_deref(), Some("On, fired"));
    assert_eq!(read(0x19).value.as_deref(), Some("Auto, fired"));
    assert_eq!(read(0x18).value.as_deref(), Some("Auto, did not fire"));
    assert_eq!(
        read(0x59).value.as_deref(),
        Some("Auto, fired, red-eye reduction")
    );
    assert_eq!(read(0x20).value.as_deref(), Some("No flash function"));
    // The return light in the tooltip.
    let returned = read(0x1f);
    assert_eq!(
        returned.note.as_deref(),
        Some("Strobe return light detected")
    );
}

#[test]
fn coordinates_altitude_and_distance_read_as_darktable_writes_them() {
    let mut photo = photo();
    for (latitude, longitude, wanted) in [
        (38.71, -9.13, ["38° 42.6′ N", "9° 7.8′ W"]),
        (-33.8688, 151.2093, ["33° 52.1′ S", "151° 12.6′ E"]),
        (0.0, 0.0, ["0° 0.0′ N", "0° 0.0′ E"]),
    ] {
        photo.exif.latitude = Some(latitude);
        photo.exif.longitude = Some(longitude);
        assert_eq!(value("latitude", &photo).as_deref(), Some(wanted[0]));
        assert_eq!(value("longitude", &photo).as_deref(), Some(wanted[1]));
    }
    photo.exif.elevation = Some(-12.0);
    assert_eq!(
        value("altitude", &photo).as_deref(),
        Some("12 m below sea level")
    );
    photo.exif.focus_distance = Some(3.2);
    assert_eq!(value("subject_distance", &photo).as_deref(), Some("3.20 m"));
    photo.exif.focus_distance = Some(4_294_967_295.0);
    assert_eq!(
        value("subject_distance", &photo).as_deref(),
        Some("Infinity")
    );
    assert_eq!(
        format::coordinates(&photo.exif).as_deref(),
        Some("0.000000, 0.000000")
    );
}

#[test]
fn the_lens_s_specification_leaves_out_what_it_lacks() {
    let mut photo = photo();
    photo.exif.lens_specification = Some([18.3, 18.3, 2.8, 2.8]);
    assert_eq!(
        value("lens_specification", &photo).as_deref(),
        Some("18.3 mm f/2.8")
    );
    photo.exif.lens_specification = Some([24.0, 70.0, 2.8, 4.0]);
    assert_eq!(
        value("lens_specification", &photo).as_deref(),
        Some("24–70 mm f/2.8–4")
    );
    photo.exif.lens_specification = Some([50.0, 50.0, 0.0, 0.0]);
    assert_eq!(
        value("lens_specification", &photo).as_deref(),
        Some("50 mm")
    );
    // The 35 mm equivalent from the crop factor where the camera records none.
    photo.exif.focal_length_35mm = None;
    photo.exif.crop_factor = Some(1.53);
    assert_eq!(value("focal_length_35mm", &photo).as_deref(), Some("28 mm"));
    assert_eq!(value("crop_factor", &photo).as_deref(), Some("1.53×"));
}

#[test]
fn a_size_on_disk_is_in_each_language_s_numbers() {
    let mut photo = photo();
    photo.file_size = 4_500_000;
    let size = |locale| shown("size_on_disk", &photo, locale).value;
    assert_eq!(size(Locale::EnGb).as_deref(), Some("4.5 MB"));
    assert_eq!(size(Locale::Uk).as_deref(), Some("4,5 МБ"));
    assert_eq!(size(Locale::Pl).as_deref(), Some("4,5 MB"));
}

#[test]
fn a_view_toggles_rows_against_the_declaration() {
    let mut view = View::default();
    assert!(view.shows("aperture") && !view.shows("software"));
    view.toggle("aperture");
    view.toggle("software");
    assert!(!view.shows("aperture") && view.shows("software"));
    assert_eq!(
        (view.shown.clone(), view.hidden.clone()),
        (vec!["software".to_owned()], vec!["aperture".to_owned()])
    );
    view.toggle("aperture");
    assert!(view.hidden.is_empty());
    view.set_open("camera", true);
    assert_eq!(view.open, ["basic", "camera", "exposure", "content"]);
    // A row this build lacks is no row to toggle.
    view.toggle("title");
    assert!(!view.shows("title"));
}

/// A preset's settings, read and written back; Essentials shows its rows alone, whatever
/// rows the build has, a row it names that the build lacks skipped.
#[test]
fn a_preset_s_settings_read_and_write_back() {
    let essentials = View::from_settings(&vec![
        (
            "metadata.shown".to_owned(),
            "file_name rating title aperture".to_owned(),
        ),
        ("metadata.shown_only".to_owned(), "true".to_owned()),
        ("metadata.open".to_owned(), "basic exposure".to_owned()),
    ]);
    for (id, shows) in [
        ("file_name", true),
        ("aperture", true),
        ("rating", true),
        ("folder", false),
        ("iso", false),
        ("software", false),
    ] {
        assert_eq!(essentials.shows(id), shows, "{id}");
    }
    assert_eq!(View::from_settings(&essentials.settings()), essentials);
    // A preset of no settings is Default settings.
    assert_eq!(View::from_settings(&Vec::new()), View::default());
    let builtin = presets::builtins(ID);
    let everything = View::from_settings(&builtin[1].1);
    let every = ROWS.iter().flat_map(|group| group.rows.iter());
    assert!(every.into_iter().all(|row| everything.shows(row.id)));
}

/// What the tool's section shows, and what it asked for.
struct Scene {
    tool: Metadata,
    mirror: Mirror,
    folder: PathBuf,
    actions: Vec<library::Action>,
    copied: Vec<String>,
    /// The commands it sent, the first numbered 1.
    asked: Vec<Command>,
    /// The registry actions it ran: the rating's, from Rating's and Colour labels' marks.
    ran: Vec<crate::actions::ActionId>,
    /// The commands its fields sent, `SetMetadata` and `RenameCopy`, in order.
    sent: Vec<Command>,
}

fn scene(photo: Option<Photo>) -> Scene {
    let mut mirror = crate::mirror::tests::mirror();
    if let Some(photo) = photo {
        mirror.photo = PhotoState::Open(Box::new(photo));
    }
    Scene {
        tool: Metadata::new(View::default()).with_home(Some(PathBuf::from(HOME))),
        mirror,
        folder: std::env::temp_dir().join("apertrust-metadata-tests-presets"),
        actions: Vec::new(),
        copied: Vec::new(),
        asked: Vec::new(),
        ran: Vec::new(),
        sent: Vec::new(),
    }
}

fn drawn(which: Builtin, width: f32, state: Scene) -> Harness<'static, Scene> {
    crate::testing::drawn(which, (width, 900.0), state, |ui, scene: &mut Scene| {
        let settings = Settings::default();
        let (words, mut commands) = (translator(ui.ctx()), Vec::new());
        let head = Head {
            open: true,
            options: scene.tool.options(&words, &scene.mirror),
            enabled: true,
            presets: Some(presets::Menu {
                folder: scene.folder.clone(),
                chosen: None,
                alerts: true,
            }),
            ..Head::default()
        };
        let mut cx = Context {
            mirror: &scene.mirror,
            settings: &settings,
            words: &words,
            commands: &mut commands,
        };
        // The app's own routing; then its preset applied, as `App::metadata_preset`.
        let mut out = crate::PhotosDrawn::default();
        parts::PanelBody::new().show(ui, |ui| {
            crate::metadata_section(ui, &mut scene.tool, &mut cx, head, &mut out);
        });
        if let Some(item) = out.metadata_preset {
            scene.tool.apply_item(&scene.folder, &item);
        }
        scene.copied.extend(out.copied);
        scene.actions.extend(out.actions);
        scene.ran.extend(out.run);
        // Each a change the save state waits a write for, as `App::send_edit` sends it.
        for command in commands {
            scene.sent.push(command);
            let number = scene.sent.len() as u64;
            (scene.mirror.saving).sent(number, true, std::time::Instant::now());
        }
        // Shown and open, it asks once drawn, as the app does.
        if let Some(command) = scene.tool.ask(true, &scene.mirror) {
            scene.asked.push(command);
            scene.tool.sent(scene.asked.len() as u64);
        }
    })
}

fn fact_named(harness: &Harness<'_, Scene>, label: &str) -> bool {
    harness.query_all_by_role(Role::ListItem).any(|node| {
        node.accesskit_node()
            .label()
            .is_some_and(|l| l.starts_with(label))
    })
}

#[test]
fn a_click_focuses_a_fact_and_ctrl_c_copies_it_the_arrows_moving() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness.get_by_label("Aperture: f/5.6").click();
    harness.run();
    harness.event(egui::Event::Copy);
    harness.step();
    let copied = &harness.output().platform_output.commands;
    let wanted = egui::OutputCommand::CopyText("f/5.6".to_owned());
    assert!(copied.contains(&wanted), "{copied:?}");
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert!(
        harness
            .get_by_label("ISO: ISO 200")
            .accesskit_node()
            .is_focused()
    );
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(
        !harness
            .get_by_label("ISO: ISO 200")
            .accesskit_node()
            .is_focused()
    );
}

#[test]
fn rows_shown_hides_a_row_and_a_group_with_none_shown() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness.get_by_label("Rows Shown").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Exposure")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Aperture")
        .click();
    harness.run();
    assert!(!fact_named(&harness, "Aperture:"));
    assert_eq!(harness.state().tool.view().hidden, ["aperture"]);
    // The menu stays open at Exposure until Esc closes it.
    assert!(
        harness
            .query_by_role_and_label(Role::MenuItemCheckBox, "Flash")
            .is_some()
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    // Every row of Lens hidden, its group goes; one shown again brings it back.
    for id in ["lens", "focal_length", "focal_length_35mm"] {
        harness.state_mut().tool.view.toggle(id);
    }
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Lens")
            .is_none()
    );
    harness.state_mut().tool.view.toggle("lens");
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Lens")
            .is_some()
    );
}

/// Show on Map sits one button high under the GPS facts, whatever room is left below it.
#[test]
fn show_on_map_takes_one_line_under_the_gps_facts() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness.get_by_role_and_label(Role::Button, "GPS").click();
    harness.run();
    let last = harness
        .query_all_by_role(Role::ListItem)
        .filter(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|l| l.starts_with("Altitude:"))
        })
        .map(|node| node.rect())
        .next()
        .expect("Altitude is shown");
    let map = harness.get_by_label("Show on Map").rect();
    let control = theme(&harness.ctx).sizes.control;
    assert!(map.top() - last.bottom() < control, "{last:?} then {map:?}");
}

#[test]
fn closing_a_group_writes_the_groups_open() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Exposure")
        .click();
    harness.run();
    assert_eq!(harness.state().tool.view().open, ["basic", "content"]);
    assert!(!fact_named(&harness, "Aperture:"));
}

#[test]
fn essentials_shows_its_rows_alone_and_default_settings_puts_them_back() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness.get_by_label("Metadata presets").click();
    harness.run();
    harness.get_by_label("Essentials").click();
    harness.run();
    assert!(fact_named(&harness, "Aperture:"));
    assert!(!fact_named(&harness, "Folder:"));
    assert!(!fact_named(&harness, "Metering mode:"));
    harness.get_by_label("Metadata presets").click();
    harness.run();
    let essentials = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Essentials");
    assert_eq!(
        essentials.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    harness.get_by_label("Default settings").click();
    harness.run();
    assert_eq!(*harness.state().tool.view(), View::default());
    assert!(fact_named(&harness, "Folder:"));
}

#[test]
fn copy_information_lists_every_row_shown() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness.get_by_label("Metadata options").click();
    harness.run();
    harness.get_by_label("Copy Information").click();
    harness.run();
    let copied = harness.state().copied.first().cloned().unwrap_or_default();
    let lines: Vec<&str> = copied.lines().collect();
    assert_eq!(lines[0], "File name: R0001225.DNG");
    assert_eq!(lines[1], "Folder: ~/Pictures/2026-07 Lisbon");
    assert!(lines.contains(&"Rating: —"));
    assert!(lines.contains(&"Aperture: f/5.6"));
    assert!(lines.contains(&"Make: RICOH IMAGING COMPANY, LTD."));
    assert!(!lines.iter().any(|line| line.starts_with("Software")));
    assert!(lines.contains(&"Altitude: 54 m"));
    // The fields shown come after, Location's last.
    assert!(lines.contains(&"Title: —"));
    assert_eq!(lines.last(), Some(&"Country: —"));
    harness.get_by_label("Metadata options").click();
    harness.run();
    harness.get_by_label("Copy Coordinates").click();
    harness.run();
    assert_eq!(
        harness.state().copied.last().unwrap(),
        "38.710000, -9.130000"
    );
}

#[test]
fn copy_coordinates_rests_without_a_location() {
    let enabled = |state: &Scene| -> Vec<bool> {
        let words = Translator::new(Locale::EnGb);
        (state.tool.options(&words, &state.mirror).iter())
            .map(|entry| matches!(entry, Entry::Item(item) if item.enabled))
            .collect()
    };
    let mut photo = photo();
    photo.exif.latitude = None;
    assert_eq!(enabled(&scene(Some(photo))), [true, false]);
    assert_eq!(enabled(&scene(None)), [false, false]);
}

#[test]
fn open_this_folder_opens_the_photo_s_folder_and_rests_on_the_open_one() {
    let mut harness = drawn(Builtin::Dark, 320.0, scene(Some(photo())));
    harness.run();
    harness.get_by_label("Open This Folder").click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [library::Action::Open(PathBuf::from(FOLDER))]
    );
    let mut state = scene(Some(photo()));
    state.mirror.folder.folder = Folder::Open(Listing {
        name: "2026-07 Lisbon".to_owned(),
        path: PathBuf::from(FOLDER),
        subfolders: Vec::new(),
        photos: Vec::new(),
        done: true,
        taken: None,
        thumbnails: HashMap::new(),
    });
    let mut harness = drawn(Builtin::Dark, 320.0, state);
    harness.run();
    let open = harness.get_by_label("Open This Folder");
    assert!(open.accesskit_node().is_disabled());
}

/// Twelve photos selected, the first active.
fn twelve() -> Scene {
    let mut state = scene(Some(photo()));
    let photos: Vec<PathBuf> = (0..12)
        .map(|n| PathBuf::from(format!("{FOLDER}/R00012{n:02}.DNG")))
        .collect();
    state.mirror.apply(Event::Selection(Selection {
        photos: photos.clone(),
        active: Some(photos[0].clone()),
        previous: false,
        through: 0,
    }));
    // Their listing's ratings: seven three stars, five one star; red on all, green on
    // three.
    let listed = (0..12)
        .map(|n| apertrust_engine::session::library::Photo {
            name: format!("R00012{n:02}.DNG"),
            size: 32_000_000,
            modified: None,
            edit: apertrust_engine::session::library::EditState::None,
            rating: apertrust_engine::session::Rating {
                stars: if n < 7 { 3 } else { 1 },
                rejected: false,
                labels: if n < 3 { 0b101 } else { 0b001 },
            },
        })
        .collect();
    state.mirror.folder.folder = Folder::Open(Listing {
        name: "2026-07 Lisbon".to_owned(),
        path: PathBuf::from(FOLDER),
        subfolders: Vec::new(),
        photos: listed,
        done: true,
        taken: None,
        thumbnails: HashMap::new(),
    });
    // The open photo rated as the first of them, so the rating's actions can run.
    let open = apertrust_engine::session::Rating {
        stars: 3,
        rejected: false,
        labels: 0b101,
    };
    rated(state, open)
}

/// Several photos' Rating reads Different, its tooltip each rating's count, and their
/// labels show red on all.
#[test]
fn several_photos_rating_reads_different() {
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    assert!(harness.query_by_label("Rating: Different").is_some());
}

/// The twelve photos' summary, as the session sends it: taken over one morning, at three
/// exposure times and apertures, the last three with another lens, seven edited; in two
/// places where `apart`.
fn twelve_summarised(apart: bool) -> Arc<Summary> {
    twelve_located(apart.then_some(Some(40.0)))
}

/// The twelve photos' summary, the odd photos' latitude `odd` where it is given: another
/// place, or none.
fn twelve_located(odd: Option<Option<f64>>) -> Arc<Summary> {
    let shots: Vec<Arc<Shot>> = (0..12u8)
        .map(|n| {
            let mut exif = (*photo().exif).clone();
            if let Some(taken) = &mut exif.taken {
                taken.hour = 8 + n / 6;
                taken.minute = 10 + n;
            }
            exif.exposure_time = Some([1.0 / 2000.0, 1.0 / 500.0, 1.0 / 60.0][usize::from(n % 3)]);
            exif.aperture = Some([2.8, 5.6, 8.0][usize::from(n % 3)]);
            if n >= 9 {
                exif.lens = Some("GR Lens 26.1mm F2.8".to_owned());
            }
            if let Some(latitude) = odd
                && n % 2 == 1
            {
                exif.latitude = latitude;
            }
            Arc::new(Shot {
                exif,
                iso: Some(200.0),
                size: [6000, 4000],
                sensor: [6064, 4040],
                format: "DNG".to_owned(),
                make: "Ricoh".to_owned(),
                alias: "GR IV".to_owned(),
                file_size: 32_000_000,
            })
        })
        .collect();
    let names: Vec<String> = (0..12).map(|n| format!("R00012{n:02}.DNG")).collect();
    let folder = Path::new(FOLDER);
    let photos: Vec<Summed<'_>> = (0..12)
        .map(|n| Summed {
            name: &names[n],
            folder,
            size: 32_000_000,
            edited: n < 7,
            facts: Facts::Read(&shots[n]),
        })
        .collect();
    Arc::new(build(&photos))
}

/// The summary the app asked for last, by its number.
fn answer(harness: &mut Harness<'_, Scene>, apart: bool) {
    let through = harness.state().asked.len() as u64;
    harness
        .state_mut()
        .tool
        .receive(through, twelve_summarised(apart));
    harness.run();
}

#[test]
fn several_photos_show_what_they_share_their_ranges_and_where_they_differ() {
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    let asked = &harness.state().asked;
    assert!(matches!(&asked[..], [Command::Summarise { photos }] if photos.len() == 12));
    assert!(fact_named(&harness, "File name: 12 photos"));
    // Nothing until the summary comes.
    assert!(fact_named(&harness, "Aperture: "));
    assert!(!fact_named(&harness, "Aperture: f"));
    answer(&mut harness, false);
    for row in [
        "Folder: ~/Pictures/2026-07 Lisbon",
        "Size on disk: 384 MB in all",
        "Dimensions: 6000 × 4000 · 24.0 MP",
        "Edited: Different",
        "Date taken: 4 Jul 2026, 08:10 – 09:21",
        "Exposure time: 1/2000 – 1/60 s",
        "Aperture: f/2.8 – f/8.0",
        "ISO: ISO 200",
    ] {
        assert!(fact_named(&harness, row), "{row}");
    }
}

#[test]
fn a_new_selection_blanks_the_rows_and_an_older_summary_is_not_drawn() {
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    answer(&mut harness, false);
    assert!(fact_named(&harness, "Aperture: f/2.8 – f/8.0"));
    let photos: Vec<PathBuf> = (0..11)
        .map(|n| PathBuf::from(format!("{FOLDER}/R00012{n:02}.DNG")))
        .collect();
    harness
        .state_mut()
        .mirror
        .apply(Event::Selection(Selection {
            photos: photos.clone(),
            active: Some(photos[0].clone()),
            previous: false,
            through: 1,
        }));
    // The frame drawn before the new selection is asked for shows nothing of the old.
    harness.step();
    assert!(!fact_named(&harness, "Aperture: f"));
    harness.run();
    assert_eq!(harness.state().asked.len(), 2);
    assert!(!fact_named(&harness, "Aperture: f"));
    // The first request's answer, late, is not this selection's.
    harness
        .state_mut()
        .tool
        .receive(1, twelve_summarised(false));
    harness.run();
    assert!(!fact_named(&harness, "Aperture: f"));
    answer(&mut harness, false);
    assert!(fact_named(&harness, "Aperture: f/2.8 – f/8.0"));
}

/// A photo taken out of the selection, the session not told yet: the rows show nothing of
/// the summary of the selection before.
#[test]
fn a_selection_on_its_way_shows_no_summary() {
    use apertrust_engine::session::SelectionCommand;
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    answer(&mut harness, false);
    assert!(fact_named(&harness, "Aperture: f/2.8 – f/8.0"));
    let out = PathBuf::from(format!("{FOLDER}/R0001211.DNG"));
    (harness.state_mut().mirror).selecting(&SelectionCommand::Remove(out), 2);
    harness.step();
    assert!(fact_named(&harness, "File name: 11 photos"));
    assert!(!fact_named(&harness, "Aperture: f"));
}

#[test]
fn rows_waiting_show_dots_after_150_ms() {
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    std::thread::sleep(several::BLANK + Duration::from_millis(20));
    harness.run();
    assert!(fact_named(&harness, "Aperture: …"));
}

#[test]
fn copy_information_and_coordinates_with_several_photos() {
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    answer(&mut harness, false);
    harness.get_by_label("Metadata options").click();
    harness.run();
    harness.get_by_label("Copy Information").click();
    harness.run();
    let copied = harness.state().copied.first().cloned().unwrap_or_default();
    let lines: Vec<&str> = copied.lines().collect();
    assert_eq!(lines[0], "File name: 12 photos");
    assert!(lines.contains(&"Lens: Different"), "{lines:?}");
    // Rating and Colour labels as their rows show them.
    assert!(lines.contains(&"Rating: Different"), "{lines:?}");
    assert!(
        lines.contains(&"Colour labels: Red, Green (some photos)"),
        "{lines:?}"
    );
    assert!(lines.contains(&"Aperture: f/2.8 – f/8.0"));
    // One place: its coordinates.
    harness.get_by_label("Metadata options").click();
    harness.run();
    harness.get_by_label("Copy Coordinates").click();
    harness.run();
    assert_eq!(
        harness.state().copied.last().unwrap(),
        "38.710000, -9.130000"
    );
    // Two places: disabled.
    answer(&mut harness, true);
    harness.get_by_label("Metadata options").click();
    harness.run();
    let coordinates = harness.get_by_label("Copy Coordinates");
    assert!(coordinates.accesskit_node().is_disabled());
}

/// Copy Coordinates' tooltip says the photos were taken in different places only where
/// they were: one place, some photos with none, disables it saying nothing of places.
#[test]
fn copy_coordinates_says_different_places_only_for_two() {
    let tip = |summary: Arc<Summary>| {
        let mut state = twelve();
        assert!(state.tool.ask(true, &state.mirror).is_some());
        state.tool.sent(1);
        state.tool.receive(1, summary);
        let words = Translator::new(Locale::EnGb);
        let options = state.tool.options(&words, &state.mirror);
        match &options[1] {
            Entry::Item(item) => (item.enabled, item.tip.clone()),
            other => panic!("{other:?}"),
        }
    };
    let (enabled, apart) = tip(twelve_located(Some(Some(40.0))));
    assert!(!enabled);
    assert_eq!(
        apart.as_deref(),
        Some("The photos were taken in different places")
    );
    let (enabled, some_none) = tip(twelve_located(Some(None)));
    assert_eq!((enabled, some_none), (false, None));
}

/// Every fact row of the raw's of the declaration has its summary, and every summary is a
/// row.
#[test]
fn every_fact_row_is_summarised() {
    use apertrust_engine::session::facts::FACTS;
    let marks = [Widget::MetadataRating, Widget::MetadataLabels].map(widget_row);
    let rows: BTreeSet<&str> = (ROWS.iter())
        .flat_map(|group| group.rows.iter())
        .map(|row| row.id)
        .filter(|id| !marks.contains(&Some(*id)))
        // Fields are no facts; the preserved file name is the sidecar's, not the raw's.
        .filter(|id| field_row(id).is_none() && *id != "preserved_file_name")
        .collect();
    assert_eq!(rows, FACTS.iter().copied().collect());
}

/// Photographs the tool at `width` in the dark and light themes, `name-dark` and
/// `name-light`, its scene made by `make` and left as `prepare` leaves it.
fn photograph(name: &str, width: f32, make: fn() -> Scene, prepare: fn(&mut Harness<'_, Scene>)) {
    crate::testing::photograph(name, |which| drawn(which, width, make()), prepare);
}

fn with_photo() -> Scene {
    scene(Some(photo()))
}

/// One photo with the defaults: Basic and Exposure open, Content to Location disabled.
#[test]
fn one_photo_is_photographed() {
    photograph("metadata-one-photo", 320.0, with_photo, |_| {});
}

/// Camera, Lens and GPS open as well.
#[test]
fn camera_lens_and_gps_open_are_photographed() {
    fn open() -> Scene {
        let mut scene = with_photo();
        for group in ["camera", "lens", "gps"] {
            scene.tool.view.set_open(group, true);
        }
        scene
    }
    photograph("metadata-groups-open", 320.0, open, |_| {});
}

/// ＋ open at Lens's rows.
#[test]
fn rows_shown_menu_is_photographed() {
    photograph("metadata-rows-shown", 320.0, with_photo, |harness| {
        harness.get_by_label("Rows Shown").click();
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItem, "Lens")
            .hover();
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItem, "Lens")
            .click();
        harness.run();
    });
}

/// No photo selected: every fact "—" and disabled.
#[test]
fn no_photo_is_photographed() {
    photograph("metadata-no-photo", 320.0, || scene(None), |_| {});
}

/// Twelve photos selected, Basic and Exposure open: what they share, the ranges, and
/// Different (the board's "Twelve photos selected" frame).
#[test]
fn several_photos_are_photographed() {
    photograph("metadata-several", 320.0, twelve, |harness| {
        answer(harness, false);
    });
}

/// Twelve photos selected, their summary not come after 150 ms: "…".
#[test]
fn several_photos_waiting_are_photographed() {
    photograph("metadata-several-waiting", 320.0, twelve, |harness| {
        std::thread::sleep(several::BLANK + Duration::from_millis(20));
        harness.run();
    });
}

/// At the Tools panel's narrowest, 240 points, a long label cut at its cap.
#[test]
fn the_narrowest_panel_is_photographed() {
    fn shown() -> Scene {
        let mut scene = with_photo();
        scene.tool.view.toggle("image_description");
        scene.tool.view.set_open("camera", true);
        scene
    }
    photograph("metadata-240", 240.0, shown, |_| {});
}

/// The open photo `photo` rated `rating`, as the session tells it.
fn rated(mut state: Scene, rating: apertrust_engine::session::Rating) -> Scene {
    use apertrust_engine::session::RatingState;
    let path = match &state.mirror.photo {
        PhotoState::Open(photo) => photo.path.clone(),
        _ => unreachable!("a photo is open"),
    };
    state.mirror.apply(Event::Rating(RatingState {
        through: 0,
        photo: path,
        rating,
    }));
    state
}

/// Rating's stars and the labels' dots run the rating's actions, as the keys and the
/// caption do: a star its count, the photo's own star clearing it, a dot its label.
#[test]
fn rating_and_labels_run_the_rating_s_actions() {
    use crate::actions::ActionId;
    let rating = apertrust_engine::session::Rating {
        stars: 3,
        rejected: false,
        labels: 0b100,
    };
    let mut harness = drawn(Builtin::Dark, 320.0, rated(scene(Some(photo())), rating));
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Rate 4 stars")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Rate 3 stars")
        .click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Red").click();
    harness.run();
    assert_eq!(
        harness.state().ran,
        [ActionId::RATING[4], ActionId::RATING[0], ActionId::LABEL[0]]
    );
    // A rejected photo's Rating reads Rejected.
    let rejected = apertrust_engine::session::Rating {
        rejected: true,
        ..rating
    };
    let mut harness = drawn(Builtin::Dark, 320.0, rated(scene(Some(photo())), rejected));
    harness.run();
    assert!(harness.query_by_label("Rating: Rejected").is_some());
}

/// The person's notes follow the marks they name to screen readers, and Rating's and
/// Colour labels' tooltips name the rating and the labels shown that have one; Copy
/// Information's words stay the row's own.
#[test]
fn rating_and_labels_carry_the_person_s_notes() {
    let rating = apertrust_engine::session::Rating {
        stars: 3,
        rejected: false,
        labels: 0b100,
    };
    let mut harness = drawn(Builtin::Dark, 320.0, rated(scene(Some(photo())), rating));
    let mut notes = parts::RatingNotes::default();
    notes.stars[2] = "Print candidates".to_owned();
    notes.labels[2] = "Done".to_owned();
    parts::set_rating_notes(&harness.ctx, notes);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Rate 3 stars · Print candidates");
    harness.get_by_role_and_label(Role::Button, "Rate 4 stars");
    harness.get_by_role_and_label(Role::Button, "Green · Done");
    harness.get_by_role_and_label(Role::Button, "Red");
    assert!(harness.query_by_label("Rating: ★★★☆☆").is_some());
    // Over a row's label, its tooltip.
    let tip = |harness: &mut Harness<'_, Scene>, row: &str, said: &str| {
        let at = harness.get_by_label(row).rect().left_center() + egui::vec2(4.0, 0.0);
        harness.hover_at(at);
        harness.run();
        harness.output().shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text) if text.galley.text() == said)
        })
    };
    assert!(tip(
        &mut harness,
        "Rating: ★★★☆☆",
        "3 stars · Print candidates"
    ));
    assert!(tip(
        &mut harness,
        "Colour labels: Green · Done",
        "Green · Done"
    ));
}

/// A photo open for viewing only shows its rating with the marks disabled: they run
/// nothing, but screen readers still hear each one by name with its note.
#[test]
fn a_view_only_photo_s_marks_keep_their_names_and_notes() {
    use apertrust_engine::session::ViewOnly;
    use apertrust_engine::sidecar::{Newer, Reason};
    let photo = Photo {
        view_only: Some(ViewOnly {
            build: Some("9.9.9".to_owned()),
            reason: Reason::Newer(Newer::Tool("lens".to_owned())),
        }),
        ..photo()
    };
    let rating = apertrust_engine::session::Rating {
        stars: 3,
        rejected: false,
        labels: 0b100,
    };
    let mut harness = drawn(Builtin::Dark, 320.0, rated(scene(Some(photo)), rating));
    let mut notes = parts::RatingNotes::default();
    notes.stars[2] = "Print candidates".to_owned();
    notes.labels[2] = "Done".to_owned();
    parts::set_rating_notes(&harness.ctx, notes);
    harness.run();
    assert!(harness.query_by_label("Rating: ★★★☆☆").is_some());
    let star = harness.get_by_role_and_label(Role::Image, "Rate 3 stars · Print candidates");
    assert!(star.accesskit_node().is_disabled());
    harness.get_by_role_and_label(Role::Image, "Green · Done");
    harness.get_by_role_and_label(Role::Image, "Red");
    // No button: a mark runs nothing.
    let three = "Rate 3 stars · Print candidates";
    assert!(
        harness
            .query_by_role_and_label(Role::Button, three)
            .is_none()
    );
    harness
        .get_by_role_and_label(Role::Image, "Rate 4 stars")
        .click();
    harness.run();
    assert!(harness.state().ran.is_empty());
}

/// Where Rating shows words in place of its stars, screen readers hear the notes its
/// tooltip says: Rejected with the reject's, Different with each rating's line; and Colour
/// labels each label with its note. Copy Information keeps the words alone.
#[test]
fn the_rating_rows_words_carry_the_notes_to_screen_readers() {
    let rejected = apertrust_engine::session::Rating {
        stars: 3,
        rejected: true,
        labels: 0,
    };
    let mut harness = drawn(Builtin::Dark, 320.0, rated(scene(Some(photo())), rejected));
    let notes = parts::RatingNotes {
        reject: "Out".to_owned(),
        ..parts::RatingNotes::default()
    };
    parts::set_rating_notes(&harness.ctx, notes);
    harness.run();
    assert!(harness.query_by_label("Rating: Rejected · Out").is_some());
    // Several photos: seven of three stars, five of one; red on all, green on some.
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    let mut notes = parts::RatingNotes::default();
    notes.stars[2] = "Print candidates".to_owned();
    notes.labels[0] = "Urgent".to_owned();
    parts::set_rating_notes(&harness.ctx, notes);
    harness.run();
    let different = "Rating: Different: 3 stars · Print candidates · 7, 1 star · 5";
    assert!(harness.query_by_label(different).is_some());
    let labels = "Colour labels: Red · Urgent, Green (some photos)";
    assert!(harness.query_by_label(labels).is_some());
    // With no notes, the words alone.
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    assert!(harness.query_by_label("Rating: Different").is_some());
}

/// Rating's and Colour labels' rows read their value to screen readers, as Copy
/// Information words it; and while a selection command is untold they describe no photo.
#[test]
fn the_rating_rows_read_their_value_and_wait_for_the_selection() {
    use apertrust_engine::session::SelectionCommand;
    let rating = apertrust_engine::session::Rating {
        stars: 3,
        rejected: false,
        labels: 0b100,
    };
    let mut harness = drawn(Builtin::Dark, 320.0, rated(scene(Some(photo())), rating));
    harness.run();
    assert!(harness.query_by_label("Rating: ★★★☆☆").is_some());
    assert!(harness.query_by_label("Colour labels: Green").is_some());
    let path = PathBuf::from(format!("{FOLDER}/R0001225.DNG"));
    (harness.state_mut().mirror).selecting(&SelectionCommand::Remove(path), 2);
    harness.step();
    assert!(harness.query_by_label("Rating: Select a photo").is_some());
    assert!(harness.query_by_label("Rating: ★★★☆☆").is_none());
}

/// The open photo rated three stars, green, as the Basic rows show it.
#[test]
fn a_rated_photo_is_photographed() {
    fn three_green() -> Scene {
        let rating = apertrust_engine::session::Rating {
            stars: 3,
            rejected: false,
            labels: 0b100,
        };
        rated(with_photo(), rating)
    }
    photograph("metadata-rated", 320.0, three_green, |_| {});
}

/// A batch's end, which tells no new selection, asks for the summary again, keeping the
/// rows meanwhile: Edited follows what the batch changed.
#[test]
fn a_batch_s_end_asks_for_the_summary_again() {
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    answer(&mut harness, false);
    assert_eq!(harness.state().asked.len(), 1);
    harness.state_mut().mirror.batches_ended += 1;
    harness.run();
    assert_eq!(harness.state().asked.len(), 2);
    assert!(fact_named(&harness, "Aperture: f/2.8 – f/8.0"));
}

/// Rejected photos read Rejected whatever stars they keep, in the row and in Copy
/// Information.
#[test]
fn rejected_photos_read_rejected_whatever_their_stars() {
    let mut state = twelve();
    if let crate::folder::Folder::Open(listing) = &mut state.mirror.folder.folder {
        for (n, photo) in listing.photos.iter_mut().enumerate() {
            photo.rating.rejected = true;
            photo.rating.stars = (n % 3) as u8;
        }
    }
    let mut harness = drawn(Builtin::Dark, 320.0, state);
    harness.run();
    assert!(harness.query_by_label("Rating: Rejected").is_some());
    answer(&mut harness, false);
    harness.get_by_label("Metadata options").click();
    harness.run();
    harness.get_by_label("Copy Information").click();
    harness.run();
    let copied = harness.state().copied.first().cloned().unwrap_or_default();
    assert!(
        copied.lines().any(|line| line == "Rating: Rejected"),
        "{copied}"
    );
}

/// `state` told the session's `Metadata` for its photo: `fields`, each a property and a
/// value as typed, and the copy's name, number and count.
fn titled(mut state: Scene, fields: &[(&str, &str)], copy: (&str, usize, usize)) -> Scene {
    use apertrust_engine::session::{CopyState, FieldValue, MetadataState};
    use apertrust_library::fields as library_fields;
    let path = match &state.mirror.photo {
        PhotoState::Open(photo) => photo.path.clone(),
        _ => unreachable!("a photo is open"),
    };
    let fields = (fields.iter())
        .map(|(property, typed)| {
            let def = library_fields::field(property).unwrap();
            let value = FieldValue::from_typed(def, typed, None).unwrap();
            ((*property).to_owned(), value)
        })
        .collect();
    state.mirror.apply(Event::Metadata(MetadataState {
        through: 0,
        photo: path,
        fields,
        copy: CopyState {
            name: copy.0.to_owned(),
            number: copy.1,
            count: copy.2,
        },
        preserved_file_name: None,
        darktable: false,
    }));
    state
}

/// The board's photo: its title, description and notes.
fn lisbon() -> Scene {
    titled(
        scene(Some(photo())),
        &[
            ("dc:title", "Morning market, Alfama"),
            (
                "dc:description",
                "Fish stalls before the crowd, the light coming in low over the river.",
            ),
            ("apertrust:notes", "Try a warmer crop"),
        ],
        ("", 1, 1),
    )
}

/// Every field row edits a field `FIELDS` has, by its property, its id the field's, or the
/// copy's name.
#[test]
fn every_field_row_names_a_field_of_the_library() {
    use apertrust_library::fields as library_fields;
    let rows = ROWS.iter().flat_map(|group| group.rows.iter());
    let mut fields = 0;
    for row in rows {
        let Some(field) = field_row(row.id) else {
            continue;
        };
        fields += 1;
        if field.field == library_fields::COPY_NAME {
            continue;
        }
        let def = library_fields::field(field.field)
            .unwrap_or_else(|| panic!("{} names no field", field.id));
        assert_eq!(def.id, field.id);
        assert!(!library_fields::is_rating(def.property));
    }
    assert_eq!(fields, 34);
}

/// Typing in Title and Return sends one `SetMetadata`, shown at once; Esc in another puts
/// it back and sends nothing.
#[test]
fn a_title_typed_and_return_sends_one_set_metadata_and_esc_sends_nothing() {
    use egui::Key;
    let mut harness = drawn(Builtin::Dark, 320.0, lisbon());
    harness.run();
    harness.get_by_label("Title").click();
    harness.run();
    // The caret where the click put it, then at the end.
    harness.key_press(Key::End);
    harness.run();
    harness.get_by_label("Title").type_text(" today");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().sent,
        [Command::SetMetadata {
            fields: vec![(
                "dc:title".to_owned(),
                Some("Morning market, Alfama today".to_owned())
            )]
        }]
    );
    let shown = harness.get_by_label("Title").accesskit_node().value();
    assert_eq!(shown.as_deref(), Some("Morning market, Alfama today"));
    harness.get_by_label("Notes").click();
    harness.run();
    harness.get_by_label("Notes").type_text(" and less");
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert_eq!(harness.state().sent.len(), 1, "Esc sends nothing");
    let notes = harness.get_by_label("Notes").accesskit_node().value();
    assert_eq!(notes.as_deref(), Some("Try a warmer crop"));
}

/// A photo open for viewing only, and several photos selected, draw every field disabled.
#[test]
fn fields_are_disabled_on_a_view_only_photo_and_with_several_selected() {
    use apertrust_engine::sidecar::{Newer, Reason, ViewOnly};
    let mut view_only = photo();
    view_only.view_only = Some(ViewOnly {
        reason: Reason::Newer(Newer::Version(3)),
        build: Some("0.9.0".to_owned()),
    });
    let state = titled(scene(Some(view_only)), &[], ("", 1, 1));
    let mut harness = drawn(Builtin::Dark, 320.0, state);
    harness.run();
    assert!(harness.get_by_label("Title").accesskit_node().is_disabled());
    let mut harness = drawn(Builtin::Dark, 320.0, twelve());
    harness.run();
    assert!(harness.get_by_label("Title").accesskit_node().is_disabled());
}

/// Basic's Copy shows only where the photo has several copies.
#[test]
fn copy_shows_only_with_several_copies() {
    let mut harness = drawn(Builtin::Dark, 320.0, lisbon());
    harness.run();
    assert!(harness.query_by_label("Copy").is_none());
    let state = titled(scene(Some(photo())), &[], ("Black and white", 2, 3));
    let mut harness = drawn(Builtin::Dark, 320.0, state);
    harness.run();
    let copy = harness.get_by_label("Copy").accesskit_node().value();
    assert_eq!(copy.as_deref(), Some("Black and white"));
}

/// The board's "One photo": Content's three fields filled, Copy with "2 of 3".
#[test]
fn one_photo_s_fields_are_photographed() {
    fn make() -> Scene {
        let mut state = lisbon();
        if let Some(metadata) = &mut state.mirror.metadata {
            metadata.copy = apertrust_engine::session::CopyState {
                name: "Black and white".to_owned(),
                number: 2,
                count: 3,
            };
        }
        state
    }
    photograph("metadata-fields-one-photo", 320.0, make, |_| {});
}

/// The board's "Editing a field": Description edited, its outline drawn.
#[test]
fn editing_a_field_is_photographed() {
    photograph("metadata-fields-editing", 320.0, lisbon, |harness| {
        harness.get_by_label("Description").click();
        harness.run();
    });
}

/// The board's "A write that failed": the Title committed, its write failing, the warning
/// button at its well's end.
#[test]
fn a_write_that_failed_is_photographed() {
    // Opened, so the save state follows its writes.
    fn opened() -> Scene {
        let mut state = scene(Some(photo()));
        state.mirror.apply(Event::Opened(photo()));
        titled(state, &[("dc:title", "Morning market, Alfama")], ("", 1, 1))
    }
    photograph("metadata-fields-failed", 320.0, opened, |harness| {
        harness.get_by_label("Title").click();
        harness.run();
        harness.key_press(egui::Key::End);
        harness.run();
        harness.get_by_label("Title").type_text(" today");
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        let sidecar = match &harness.state().mirror.photo {
            PhotoState::Open(photo) => apertrust_engine::sidecar::path_for(&photo.path),
            _ => unreachable!("a photo is open"),
        };
        let state = harness.state_mut();
        state.mirror.apply(Event::Failed(
            apertrust_engine::session::SessionError::Save {
                path: sidecar,
                through: 1,
                source: std::io::Error::other("The disk is full"),
            },
        ));
        harness.run();
        harness.run();
        assert!(harness.query_by_label("Title was not saved").is_some());
    });
}

/// `lisbon`, opened, so the save state follows its writes.
fn lisbon_opened() -> Scene {
    let mut state = scene(Some(photo()));
    state.mirror.apply(Event::Opened(photo()));
    titled(state, &[("dc:title", "Morning market, Alfama")], ("", 1, 1))
}

/// A commit that changes nothing as the session would take it, a space added, sends
/// nothing, so no write is waited for.
#[test]
fn a_commit_that_changes_nothing_leaves_no_save_waiting() {
    let mut harness = drawn(Builtin::Dark, 320.0, lisbon_opened());
    harness.run();
    harness.get_by_label("Title").click();
    harness.run();
    harness.key_press(egui::Key::End);
    harness.run();
    harness.get_by_label("Title").type_text("  ");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(harness.state().sent, []);
    assert!(!harness.state().mirror.saving.writing());
}

/// A draft begun on one photo, the active photo then changing with no press to end it, as
/// Photo › Next Photo from the menu bar does, is forgotten: nothing goes to the next photo,
/// which shows its own value.
#[test]
fn a_draft_never_goes_to_another_photo() {
    let mut harness = drawn(Builtin::Dark, 320.0, lisbon_opened());
    harness.run();
    harness.get_by_label("Title").click();
    harness.run();
    harness.key_press(egui::Key::End);
    harness.run();
    harness.get_by_label("Title").type_text(" today");
    harness.run();
    let mut next = photo();
    next.path = PathBuf::from(format!("{FOLDER}/R0001226.DNG"));
    {
        let state = harness.state_mut();
        state.mirror.apply(Event::Opened(next.clone()));
        let moved = std::mem::replace(state, scene(None));
        *state = titled(moved, &[("dc:title", "Harbour")], ("", 1, 1));
    }
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(harness.state().sent, []);
    let title = harness.get_by_label("Title").accesskit_node().value();
    assert_eq!(title.as_deref(), Some("Harbour"));
}
