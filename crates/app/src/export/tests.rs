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

//! The Export dialog in the dialog host's harness: every row, the keys, the preview's
//! lines as the session's answers arrive, the rows drawn disabled, and the three languages
//! and a narrow window photographed.

use apertrust_engine::export_preview::{ActiveSize, FolderCheck, OutputSize, PhotoName};
use apertrust_ui::{Builtin, Locale, Theme};
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::dialog_host::{self, Output};

/// The dialog as the host draws it in its sheet, over the panel's colour, and what each
/// frame asked of the app and whether it asked to close.
struct Bench {
    content: Export,
    asked: Vec<Asked>,
    closed: bool,
}

impl Bench {
    fn export(&mut self) -> &mut Export {
        &mut self.content
    }
}

fn dialog(
    values: Values,
    which: Builtin,
    locale: Locale,
    size: egui::Vec2,
) -> Harness<'static, Bench> {
    dialog_of(home(values), which, locale, size)
}

/// The dialog over `values`, with Anna's home.
fn home(values: Values) -> Export {
    Export::with_home(values, Some(PathBuf::from("/Users/anna")))
}

/// `export` in the bench, drawn by the host on the Mac.
fn dialog_of(
    export: Export,
    which: Builtin,
    locale: Locale,
    size: egui::Vec2,
) -> Harness<'static, Bench> {
    dialog_on(export, which, locale, size, OperatingSystem::Mac)
}

/// `export` in the bench, drawn by the host on `os`.
fn dialog_on(
    export: Export,
    which: Builtin,
    locale: Locale,
    size: egui::Vec2,
    os: OperatingSystem,
) -> Harness<'static, Bench> {
    let mut harness = Harness::builder()
        .with_size(size)
        .with_os(os)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                let drawn = dialog_host::draw(ui, &mut bench.content);
                if let Some(Output::Export(asked)) = drawn.output {
                    bench.asked.push(asked);
                }
                bench.closed |= drawn.close;
            },
            Bench {
                content: export,
                asked: Vec::new(),
                closed: false,
            },
        );
    harness.run();
    harness
}

fn open() -> Harness<'static, Bench> {
    dialog(
        Values::default(),
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 640.0),
    )
}

/// What the session answers for the active photo `IMG_2041.ARW` and the default path.
fn preview() -> ExportPreview {
    ExportPreview {
        problem: None,
        sequence_added: false,
        not_here: Vec::new(),
        names: vec![PhotoName {
            source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
            exact: true,
            fate: Fate::File {
                file: PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2041.jpg"),
                exists: false,
                action: Action::Write,
            },
        }],
        size: Some(ActiveSize {
            source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
            size: Ok(OutputSize {
                extent: [6000, 4000],
                width: 6000,
                height: 4000,
                scale: 1.0,
            }),
        }),
        folders: vec![FolderCheck {
            folder: PathBuf::from("/Users/anna/Pictures/apertrust_exported"),
            preflight: Preflight::Ready,
        }],
    }
}

/// The session answers the preview numbered `through`.
fn answer(harness: &mut Harness<'_, Bench>, through: u64, preview: ExportPreview) {
    harness.state_mut().export().receive(through, preview);
    harness.run();
}

/// What the dialog holds now.
fn held(harness: &mut Harness<'_, Bench>) -> Values {
    harness.state_mut().export().values().clone()
}

/// Every frame runs to its end, though a tooltip under the pointer keeps asking for more.
fn settle(harness: &mut Harness<'_, Bench>) {
    harness.run_steps(8);
}

/// Chooses `option` in the dropdown row `row`, as a click on the row and one on the option do.
fn choose(harness: &mut Harness<'_, Bench>, row: &str, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, row).click();
    settle(harness);
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, option)
        .click();
    settle(harness);
}

fn disabled(harness: &Harness<'_, Bench>, role: Role, label: &str) -> bool {
    harness
        .get_by_role_and_label(role, label)
        .accesskit_node()
        .is_disabled()
}

/// Types `text` into the field named `name`, as the keyboard does, and leaves it with Tab.
fn type_into(harness: &mut Harness<'_, Bench>, role: Role, name: &str, text: &str) {
    harness.get_by_role_and_label(role, name).focus();
    harness.run();
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
}

/// The text of the session's feedback, if the dialog shows it.
fn says(harness: &Harness<'_, Bench>, text: &str) -> bool {
    harness.query_by_label(text).is_some()
}

// What the first frame and each change ask for.

#[test]
fn what_the_dialog_asks_becomes_the_session_s_commands_for_the_photo_open() {
    let places = Places::default();
    let photo = Path::new("/Users/anna/Pictures/IMG_2041.ARW");
    let held = Values {
        format: FileFormat::Png,
        ..Values::default()
    };
    let preview = |values: &Values| Command::PreviewExport {
        photos: vec![photo.to_owned()],
        settings: values.settings(),
        places: places.clone(),
    };
    let export = Command::Export {
        photos: vec![photo.to_owned()],
        settings: held.settings(),
        places: places.clone(),
    };
    // A change: a preview of it, nothing else.
    let changed = Asked {
        values: Some(held.clone()),
        start: false,
        confirm: false,
        opened: false,
    };
    assert_eq!(commands(photo, &changed, &held, &places), [preview(&held)]);
    // Export pressed with a change in the same frame: the preview, then the export of the
    // settings the app holds.
    let both = Asked {
        start: true,
        ..changed
    };
    assert_eq!(
        commands(photo, &both, &held, &places),
        [preview(&held), export.clone()]
    );
    // Export alone: only the export.
    let only = Asked {
        values: None,
        start: true,
        confirm: false,
        opened: false,
    };
    assert_eq!(commands(photo, &only, &held, &places), [export]);
    // Nothing asked, nothing sent; and an export awaiting the overwrite question is not
    // sent until the app has the answer.
    assert!(commands(photo, &Asked::default(), &held, &places).is_empty());
    let confirming = Asked {
        confirm: true,
        ..Asked::default()
    };
    assert!(commands(photo, &confirming, &held, &places).is_empty());
}

#[test]
fn the_first_frame_asks_for_a_preview_and_a_change_for_another() {
    let mut harness = open();
    let asked = harness.state().asked.clone();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].opened && !asked[0].start);
    assert_eq!(asked[0].values, Some(Values::default()));
    // Nothing changes, nothing is asked.
    harness.run();
    assert_eq!(harness.state().asked.len(), 1);
    // A letter typed in the path is a new preview, with the whole path.
    type_into(&mut harness, Role::TextInput, "Path", "x");
    let asked = harness.state().asked.last().cloned().expect("asked");
    assert!(!asked.opened && !asked.start);
    let path = asked.values.expect("the path changed").path;
    assert!(
        path.contains('x') && path != Values::default().path,
        "{path}"
    );
}

#[test]
fn the_newest_answer_is_kept_and_an_older_one_never_replaces_it() {
    let mut harness = open();
    let named = |file: &str| {
        let mut p = preview();
        p.names[0].fate = Fate::File {
            file: PathBuf::from(file),
            exists: false,
            action: Action::Write,
        };
        p
    };
    answer(
        &mut harness,
        5,
        named("/Users/anna/Pictures/five/IMG_2041.jpg"),
    );
    assert!(says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/five/IMG_2041.jpg"
    ));
    // One that finished after a newer command was sent still arrives, with a lower number.
    answer(
        &mut harness,
        4,
        named("/Users/anna/Pictures/four/IMG_2041.jpg"),
    );
    assert!(says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/five/IMG_2041.jpg"
    ));
    answer(
        &mut harness,
        6,
        named("/Users/anna/Pictures/six/IMG_2041.jpg"),
    );
    assert!(says(&harness, "IMG_2041.ARW → ~/Pictures/six/IMG_2041.jpg"));
}

// Every row.

#[test]
fn every_row_of_the_first_card_is_there_by_its_name() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    harness.get_by_role_and_label(Role::Dialog, "Export 1 photo");
    for combo in [
        "Target storage",
        "On conflict",
        "File format",
        "Chroma subsampling",
        "Set size",
        "Allow upscaling",
        "High quality resampling",
        "Store masks",
        "Profile",
        "Intent",
        "Style",
    ] {
        harness.get_by_role_and_label(Role::ComboBox, combo);
    }
    harness.get_by_role_and_label(Role::TextInput, "Path");
    harness.get_by_role_and_label(Role::Slider, "Quality");
    // The defaults are darktable's.
    let defaults = Values::default();
    assert_eq!(
        defaults.path,
        "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)"
    );
    assert_eq!(defaults.on_conflict, OnConflict::CreateUniqueFilename);
    assert_eq!(defaults.format, FileFormat::Jpeg);
    assert_eq!((defaults.jpeg_quality, defaults.png_compression), (95, 5));
    assert_eq!(defaults.size_mode, SizeMode::Pixels);
    assert_eq!((defaults.width, defaults.height, defaults.dpi), (0, 0, 300));
    assert!(!defaults.upscale && !defaults.high_quality);
    assert_eq!(defaults.profile, Profile::ImageSettings);
}

#[test]
fn file_on_disk_takes_a_path_and_on_conflict_changes() {
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.event(egui::Event::Text("$(HOME)/proofs/$(FILE_NAME)".to_owned()));
    harness.run();
    assert_eq!(held(&mut harness).path, "$(HOME)/proofs/$(FILE_NAME)");
    choose(&mut harness, "On conflict", "Skip");
    assert_eq!(held(&mut harness).on_conflict, OnConflict::Skip);
    choose(&mut harness, "On conflict", "Overwrite if changed");
    assert_eq!(
        held(&mut harness).on_conflict,
        OnConflict::OverwriteIfChanged
    );
}

#[test]
fn typing_a_variable_offers_the_list_and_a_click_inserts_one() {
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.event(egui::Event::Text("$(FILE.F".to_owned()));
    harness.run();
    // darktable's list, filtered by what is typed, each with its description.
    harness.get_by_role_and_label(Role::ListBox, "Variables");
    let option = harness.get_by_role_and_label(Role::ListBoxOption, "FILE.FOLDER");
    option.click();
    harness.run();
    assert_eq!(held(&mut harness).path, "$(FILE.FOLDER)");
}

#[test]
fn the_format_chooses_its_own_rows() {
    let mut harness = open();
    harness.get_by_role_and_label(Role::Slider, "Quality");
    assert!(harness.query_by_label("Bit depth").is_none());
    choose(&mut harness, "File format", "PNG");
    assert_eq!(held(&mut harness).format, FileFormat::Png);
    assert!(harness.query_by_label("Quality").is_none());
    harness.get_by_role_and_label(Role::Slider, "Compression");
    choose(&mut harness, "Bit depth", "16 bit");
    assert_eq!(held(&mut harness).png_bit_depth, BitDepth::Sixteen);
    // And back.
    choose(&mut harness, "File format", "JPEG (8-bit)");
    choose(&mut harness, "Chroma subsampling", "4:2:0");
    assert_eq!(held(&mut harness).jpeg_subsample, Subsample::S420);
}

#[test]
fn set_size_has_four_modes_with_the_fields_each_needs() {
    let mut harness = open();
    // In pixels: a width and a height, 0 for no limit.
    harness.get_by_label("Width, in px");
    harness.get_by_label("Height, in px");
    type_into(&mut harness, Role::TextInput, "Width, in px", "2048");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(held(&mut harness).width, 2048);
    // In cm: the same two fields in centimetres at the resolution, and what that is in
    // pixels.
    choose(&mut harness, "Set size", "In cm (for print)");
    assert_eq!(held(&mut harness).size_mode, SizeMode::Cm);
    harness.get_by_label("Width, in cm");
    harness.get_by_label("Resolution, in dpi");
    assert!(says(&harness, "Which is equal to 2048 × 0 px"));
    type_into(&mut harness, Role::TextInput, "Height, in cm", "10");
    harness.key_press(egui::Key::Enter);
    harness.run();
    // darktable's cm2pixels: 10 cm at 300 dpi, rounded up.
    assert_eq!(held(&mut harness).height, 1182);
    assert!(says(&harness, "Which is equal to 2048 × 1182 px"));
    // In inch.
    choose(&mut harness, "Set size", "In inch (for print)");
    harness.get_by_label("Width, in in");
    // By scale: one text, darktable's.
    choose(&mut harness, "Set size", "By scale (for file)");
    assert!(harness.query_by_label("Width, in in").is_none());
    type_into(&mut harness, Role::TextInput, "Scale", "/2");
    assert_eq!(held(&mut harness).scale, "1/2");
}

#[test]
fn a_resolution_keeps_the_print_size_and_moves_the_pixels() {
    let values = Values {
        size_mode: SizeMode::Inch,
        width: 3000,
        height: 1500,
        ..Values::default()
    };
    let mut harness = dialog(
        values,
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 560.0),
    );
    type_into(&mut harness, Role::TextInput, "Resolution, in dpi", "600");
    harness.key_press(egui::Key::Enter);
    harness.run();
    let now = held(&mut harness);
    assert_eq!((now.dpi, now.width, now.height), (600, 6000, 3000));
}

#[test]
fn changing_the_resolution_and_back_returns_the_pixels_as_darktable_keeps_the_entered_lengths() {
    // 1001 × 501 px at 300 dpi shows as 8.48 × 4.24 cm, which darktable's print fields then
    // hold: the first change reads them off the pixels, and from then on the same lengths
    // give the same pixels at each resolution, where deriving them from the pixels each time
    // drifted (1001, 1005, 1002, 1006).
    let values = Values {
        size_mode: SizeMode::Cm,
        width: 1001,
        height: 501,
        ..Values::default()
    };
    let mut harness = dialog(
        values,
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 560.0),
    );
    for (dpi, width, height) in [
        (301, 1005, 503),
        (300, 1002, 501),
        (301, 1005, 503),
        (300, 1002, 501),
    ] {
        type_into(
            &mut harness,
            Role::TextInput,
            "Resolution, in dpi",
            &dpi.to_string(),
        );
        harness.key_press(egui::Key::Enter);
        harness.run();
        let now = held(&mut harness);
        assert_eq!((now.dpi, now.width, now.height), (dpi, width, height));
    }
    // A length typed is the new physical size, and the next change of resolution keeps it.
    type_into(&mut harness, Role::TextInput, "Width, in cm", "10");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(held(&mut harness).width, 1182);
    type_into(&mut harness, Role::TextInput, "Resolution, in dpi", "301");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(held(&mut harness).width, 1186);
}

#[test]
fn allow_upscaling_and_high_quality_resampling_are_choices_between_no_and_yes() {
    let mut harness = open();
    choose(&mut harness, "Allow upscaling", "Yes");
    choose(&mut harness, "High quality resampling", "Yes");
    let now = held(&mut harness);
    assert!(now.upscale && now.high_quality);
    choose(&mut harness, "High quality resampling", "No");
    assert!(!held(&mut harness).high_quality);
}

#[test]
fn profile_takes_image_settings_and_srgb_and_lists_the_rest_disabled() {
    let mut harness = open();
    choose(&mut harness, "Profile", "sRGB (web-safe)");
    assert_eq!(held(&mut harness).profile, Profile::Srgb);
    harness
        .get_by_role_and_label(Role::ComboBox, "Profile")
        .click();
    settle(&mut harness);
    for option in [
        "Adobe RGB (compatible)",
        "Linear Rec709 RGB",
        "Display P3 RGB",
        "BRG (for testing)",
    ] {
        let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, option);
        assert!(node.accesskit_node().is_disabled(), "{option}");
    }
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Image settings")
        .click();
    settle(&mut harness);
    assert_eq!(held(&mut harness).profile, Profile::ImageSettings);
}

#[test]
fn the_settings_become_the_session_s_export_settings() {
    let defaults = Values::default().settings();
    assert_eq!(
        defaults.path,
        "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)"
    );
    assert_eq!(defaults.on_conflict, OnConflict::CreateUniqueFilename);
    assert_eq!(
        defaults.format,
        Format::Jpeg(jpeg::Options {
            quality: 95,
            subsample: Subsample::Auto,
            dpi: 300
        })
    );
    // The dialog's rule for a side left at 0 applies.
    assert_eq!(
        defaults.size,
        Request {
            width: 0,
            height: 0,
            scale: None,
            upscale: false,
            high_quality: false,
            dialog: true
        }
    );
    let png = Values {
        format: FileFormat::Png,
        png_bit_depth: BitDepth::Sixteen,
        png_compression: 9,
        size_mode: SizeMode::Scale,
        scale: "1/2".to_owned(),
        width: 800,
        upscale: true,
        high_quality: true,
        ..Values::default()
    }
    .settings();
    assert_eq!(
        png.format,
        Format::Png(png::Options {
            bit_depth: BitDepth::Sixteen,
            compression: 9
        })
    );
    // By scale the largest size is not used, as darktable's.
    assert_eq!(
        png.size,
        Request {
            width: 0,
            height: 0,
            scale: Some("1/2".to_owned()),
            upscale: true,
            high_quality: true,
            dialog: true
        }
    );
    // A JPEG's density is the print dpi in the cm and inch modes.
    let print = Values {
        size_mode: SizeMode::Cm,
        dpi: 600,
        ..Values::default()
    }
    .settings();
    assert!(matches!(print.format, Format::Jpeg(o) if o.dpi == 600));
}

#[test]
fn the_defaults_are_what_an_empty_store_holds_and_a_change_is_there_when_it_is_reopened() {
    use crate::settings::{Keeper, Settings, testing::Folder};

    let folder = Folder::new("export-reopened");
    let (mut keeper, settings) = Keeper::open(folder.storage());
    assert_eq!(settings.export, Values::default());
    let changed = Values {
        format: FileFormat::Png,
        png_compression: 2,
        path: "~/out/$(FILE_NAME)".to_owned(),
        ..Values::default()
    };
    keeper.frame(
        std::time::Instant::now(),
        &Settings {
            export: changed.clone(),
            ..settings
        },
    );
    keeper.flush();
    // The file holds the three, in darktable's keys' places, and nothing else of the dialog.
    let text = folder.text();
    assert!(
        text.contains("format = \"png\"") && text.contains("compression = 2"),
        "{text}"
    );
    assert!(text.contains("path = \"~/out/$(FILE_NAME)\""), "{text}");
    assert!(
        !text.contains("quality") && !text.contains("[export.size]"),
        "{text}"
    );
    // A new run reads them back.
    let (_, reopened) = Keeper::open(folder.storage());
    assert_eq!(reopened.export, changed);
}

#[test]
fn a_setting_conf_gives_is_what_the_dialog_opens_with_and_only_the_other_change_is_kept() {
    use crate::settings::{Keeper, Settings, overrides, testing::Folder};

    let folder = Folder::new("export-conf");
    let given = overrides(&["export.size.mode=cm".to_owned()]).expect("a known key");
    let (mut keeper, settings) = Keeper::open_with(folder.storage(), &given);
    assert_eq!(settings.export.size_mode, SizeMode::Cm);
    // The dialog opens with it, and the person changes that row and another.
    let mut harness = dialog(
        settings.export.clone(),
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 560.0),
    );
    harness.get_by_label("Width, in cm");
    choose(&mut harness, "Set size", "In inch (for print)");
    choose(&mut harness, "File format", "PNG");
    let left = held(&mut harness);
    assert_eq!(
        (left.size_mode, left.format),
        (SizeMode::Inch, FileFormat::Png)
    );
    keeper.frame(
        std::time::Instant::now(),
        &Settings {
            export: left,
            ..settings
        },
    );
    keeper.flush();
    // The store keeps `--conf`'s value for the run and never writes it; the other reaches
    // the file.
    let text = folder.text();
    assert!(text.contains("format = \"png\""), "{text}");
    assert!(!text.contains("mode"), "{text}");
    // The next run, with no flag, finds the file's: the default mode, the new format.
    let (_, next) = Keeper::open(folder.storage());
    assert_eq!(next.export.size_mode, SizeMode::Pixels);
    assert_eq!(next.export.format, FileFormat::Png);
}

// The presets column.

#[test]
fn default_settings_is_ticked_while_the_settings_are_the_defaults_and_resets_them() {
    let mut harness = open();
    assert!(harness.query_by_label("Default settings").is_some());
    assert!(!says(&harness, "The settings match no preset."));
    choose(&mut harness, "On conflict", "Skip");
    assert!(says(&harness, "The settings match no preset."));
    harness.get_by_label("Default settings").click();
    settle(&mut harness);
    assert_eq!(held(&mut harness), Values::default());
    assert!(!says(&harness, "The settings match no preset."));
}

// The rows that are drawn disabled.

#[test]
fn every_row_not_built_is_drawn_disabled() {
    let mut harness = open();
    for combo in ["Store masks", "Intent", "Style"] {
        assert!(disabled(&harness, Role::ComboBox, combo), "{combo}");
    }
    harness.get_by_label("Choose…");
    assert!(
        harness
            .get_by_label("Choose…")
            .accesskit_node()
            .is_disabled()
    );
    // The groups, closed, that come later.
    for group in ["Metadata", "Multi-preset export"] {
        assert!(
            disabled(&harness, Role::Button, group),
            "{group} is drawn disabled"
        );
    }
    // The storages other than File on disk, and the formats other than JPEG and PNG.
    harness
        .get_by_role_and_label(Role::ComboBox, "Target storage")
        .click();
    settle(&mut harness);
    for option in ["LaTeX book template", "Send as email", "Website gallery"] {
        let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, option);
        assert!(node.accesskit_node().is_disabled(), "{option}");
    }
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::ComboBox, "File format")
        .click();
    settle(&mut harness);
    for option in ["AVIF", "TIFF", "WebP", "OpenEXR", "Copy"] {
        let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, option);
        assert!(node.accesskit_node().is_disabled(), "{option}");
    }
    for option in ["JPEG (8-bit)", "PNG"] {
        let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, option);
        assert!(!node.accesskit_node().is_disabled(), "{option}");
    }
}

#[test]
fn overwrite_can_be_chosen_and_nothing_stops_export_for_it() {
    let mut harness = open();
    choose(&mut harness, "On conflict", "Overwrite");
    assert_eq!(held(&mut harness).on_conflict, OnConflict::Overwrite);
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
    // Set outside the dialog, as `--conf` or the file can, it is the same.
    let values = Values {
        on_conflict: OnConflict::Overwrite,
        ..Values::default()
    };
    let harness = dialog(
        values,
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 560.0),
    );
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn on_linux_the_host_draws_the_alert_so_overwrite_is_listed_enabled() {
    let alerts = dialog_host::platform::shows_alerts(egui::os::OperatingSystem::Nix);
    let mut harness = dialog_of(
        home(Values::default()).alerts(alerts),
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 640.0),
    );
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .click();
    settle(&mut harness);
    let overwrite = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Overwrite");
    assert!(!overwrite.accesskit_node().is_disabled());
    overwrite.click();
    settle(&mut harness);
    assert_eq!(held(&mut harness).on_conflict, OnConflict::Overwrite);
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn where_no_alert_can_ask_overwrite_is_listed_disabled_and_export_rests_while_it_is_kept() {
    let mut harness = dialog_of(
        home(Values::default()).alerts(false),
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 640.0),
    );
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .click();
    settle(&mut harness);
    let overwrite = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Overwrite");
    assert!(overwrite.accesskit_node().is_disabled());
    for option in ["Create unique filename", "Overwrite if changed", "Skip"] {
        let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, option);
        assert!(!node.accesskit_node().is_disabled(), "{option}");
    }
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Overwrite")
        .click();
    settle(&mut harness);
    assert_eq!(
        held(&mut harness).on_conflict,
        OnConflict::CreateUniqueFilename
    );
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
    // Kept from a platform that asks: shown, but Export rests, saying why, and Return
    // exports nothing.
    let mut harness = dialog_of(
        home(Values {
            on_conflict: OnConflict::Overwrite,
            ..Values::default()
        })
        .alerts(false),
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 640.0),
    );
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!harness.state().asked.iter().any(|a| a.start || a.confirm));
}

// Export asks before it replaces a file.

/// The preview where the photo's file is already there and `action` says what On conflict
/// does with it.
fn preview_with_file_there(action: Action) -> ExportPreview {
    let mut preview = preview();
    preview.names[0].fate = Fate::File {
        file: PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2041.jpg"),
        exists: true,
        action,
    };
    preview
}

fn overwriting(conflict: OnConflict) -> Harness<'static, Bench> {
    dialog(
        Values {
            on_conflict: conflict,
            ..Values::default()
        },
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 640.0),
    )
}

/// Presses Export and says what the frame asked: whether the export starts, whether the
/// overwrite question is to be asked, and whether the dialog closed.
fn press_export(harness: &mut Harness<'_, Bench>) -> (bool, bool, bool) {
    let before = harness.state().asked.len();
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    let state = harness.state();
    let asked = &state.asked[before..];
    (
        asked.iter().any(|a| a.start),
        asked.iter().any(|a| a.confirm),
        state.closed,
    )
}

#[test]
fn export_in_overwrite_mode_asks_first_whether_or_not_a_file_is_there_and_the_dialog_stays() {
    // darktable's rule (`ask_user_confirmation`): On conflict Overwrite asks, with no
    // preview yet, with one that finds no file and with one that finds a file to replace.
    let mut harness = overwriting(OnConflict::Overwrite);
    // Not a start, and not closed: the app asks, and sends the export on a yes.
    assert_eq!(press_export(&mut harness), (false, true, false));
    answer(&mut harness, 1, preview());
    assert_eq!(press_export(&mut harness), (false, true, false));
    answer(&mut harness, 2, preview_with_file_there(Action::Overwrite));
    assert_eq!(press_export(&mut harness), (false, true, false));
    // Asked again, the same question: a no leaves the dialog as it was.
    assert_eq!(press_export(&mut harness), (false, true, false));
    // Changing the path does not change the rule, whatever the preview says.
    type_into(&mut harness, Role::TextInput, "Path", "x");
    assert_eq!(press_export(&mut harness), (false, true, false));
}

#[test]
fn return_and_cmd_return_ask_the_same_question() {
    let mut harness = overwriting(OnConflict::Overwrite);
    answer(&mut harness, 1, preview_with_file_there(Action::Overwrite));
    harness.key_press(egui::Key::Enter);
    harness.run();
    let state = harness.state();
    assert!(!state.closed);
    assert!(state.asked.iter().any(|a| a.confirm) && state.asked.iter().all(|a| !a.start));
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    assert!(!harness.state().closed);
    assert!(harness.state().asked.iter().all(|a| !a.start));
}

#[test]
fn export_asks_nothing_in_the_other_ways_to_deal_with_a_file_there() {
    // Overwrite if changed never asks, as darktable's does not, a file replaced or not.
    let mut harness = overwriting(OnConflict::OverwriteIfChanged);
    answer(&mut harness, 1, preview_with_file_there(Action::Overwrite));
    assert_eq!(press_export(&mut harness), (true, false, true));
    let mut harness = overwriting(OnConflict::OverwriteIfChanged);
    answer(
        &mut harness,
        1,
        preview_with_file_there(Action::Skip(apertrust_export::disk::Why::NotChanged)),
    );
    assert_eq!(press_export(&mut harness), (true, false, true));
    // Nor do the other two, with no preview or with one.
    let mut harness = overwriting(OnConflict::CreateUniqueFilename);
    assert_eq!(press_export(&mut harness), (true, false, true));
    for (conflict, action) in [
        (OnConflict::CreateUniqueFilename, Action::Number),
        (
            OnConflict::Skip,
            Action::Skip(apertrust_export::disk::Why::Exists),
        ),
    ] {
        let mut harness = overwriting(conflict);
        answer(&mut harness, 1, preview_with_file_there(action));
        assert_eq!(
            press_export(&mut harness),
            (true, false, true),
            "{conflict:?}"
        );
    }
}

#[test]
fn a_row_not_built_says_why_in_its_tooltip() {
    let mut harness = open();
    // Scrolled into view, as a person reaches it.
    harness
        .get_by_role_and_label(Role::ComboBox, "Intent")
        .scroll_to_me();
    settle(&mut harness);
    let row = harness
        .get_by_role_and_label(Role::ComboBox, "Intent")
        .rect();
    harness.hover_at(row.center());
    harness.run_steps(40);
    assert!(
        harness.query_by_label_contains("no ICC files").is_some(),
        "Intent says it waits for ICC files"
    );
    let row = harness.get_by_label("Choose…").rect();
    harness.hover_at(row.center());
    harness.run_steps(40);
    assert!(
        harness
            .query_by_label("Coming in a later version")
            .is_some()
    );
}

// What the session says.

#[test]
fn the_active_photo_s_file_and_size_are_said_under_their_rows() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    assert!(says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/apertrust_exported/IMG_2041.jpg"
    ));
    // No limit: the full size.
    assert!(says(
        &harness,
        "IMG_2041.ARW will be 6000 × 4000 px (full size: no limit set)."
    ));
    // A limit, and the size it gives.
    type_into(&mut harness, Role::TextInput, "Width, in px", "2048");
    harness.key_press(egui::Key::Enter);
    harness.run();
    let mut shrunk = preview();
    shrunk.size = Some(ActiveSize {
        source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
        size: Ok(OutputSize {
            extent: [6000, 4000],
            width: 2048,
            height: 1365,
            scale: 0.341,
        }),
    });
    answer(&mut harness, 2, shrunk);
    assert!(says(&harness, "IMG_2041.ARW will be 2048 × 1365 px."));
    // A limit larger than the photo, with upscaling off: kept at its full size, and said so.
    type_into(&mut harness, Role::TextInput, "Width, in px", "9000");
    harness.key_press(egui::Key::Enter);
    harness.run();
    answer(&mut harness, 3, preview());
    assert!(says(
        &harness,
        "IMG_2041.ARW will be 6000 × 4000 px (not upscaled: Allow upscaling is No)."
    ));
}

#[test]
fn a_file_already_there_is_said_with_what_on_conflict_does() {
    let mut harness = open();
    let mut there = preview();
    let name = PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2041_01.jpg");
    there.names[0].fate = Fate::File {
        file: name.clone(),
        exists: true,
        action: Action::Number,
    };
    answer(&mut harness, 1, there.clone());
    assert!(says(
        &harness,
        "IMG_2041_01.jpg is already there, so this one is saved with a number added."
    ));
    there.names[0].fate = Fate::File {
        file: name.clone(),
        exists: true,
        action: Action::Skip(apertrust_export::disk::Why::NotChanged),
    };
    answer(&mut harness, 2, there.clone());
    assert!(says(
        &harness,
        "Skips IMG_2041_01.jpg, which is already there and newer than the photo's edit."
    ));
    // Nothing there, nothing said.
    answer(&mut harness, 3, preview());
    assert!(harness.query_by_label_contains("already there").is_none());
}

#[test]
fn several_photos_are_summed_up_and_the_sequence_rule_is_said() {
    let mut harness = open();
    let mut many = preview();
    many.sequence_added = true;
    for n in 2..=3 {
        let mut name = many.names[0].clone();
        name.source = PathBuf::from(format!("/Users/anna/Pictures/IMG_204{n}.ARW"));
        many.names.push(name);
    }
    many.names[1].fate = Fate::File {
        file: PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2042.jpg"),
        exists: true,
        action: Action::Number,
    };
    answer(&mut harness, 1, many);
    assert!(says(&harness, "and 2 more, the same way"));
    assert!(
        harness
            .query_by_label_contains("_$(SEQUENCE) is added")
            .is_some()
    );
    assert!(says(
        &harness,
        "1 of the 3 are already there; those are saved with _01 added."
    ));
}

#[test]
fn variables_with_no_value_here_are_said_to_expand_to_nothing() {
    let mut harness = open();
    let mut p = preview();
    p.not_here = vec![apertrust_export::variables::Unavailable {
        name: "OPENCL.ACTIVATED",
        why: apertrust_export::variables::NotHere::OpenCl,
    }];
    answer(&mut harness, 1, p);
    assert!(
        harness
            .query_by_label_contains("$(OPENCL.ACTIVATED): darktable's, with no value here")
            .is_some()
    );
    // Export stays allowed.
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn an_unknown_variable_stops_export_and_the_fix_replaces_it() {
    let values = Values {
        path: "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAM)".to_owned(),
        ..Values::default()
    };
    let mut harness = dialog(
        values,
        Builtin::Dark,
        Locale::EnGb,
        egui::vec2(660.0, 560.0),
    );
    let mut p = preview();
    let at = 36..44;
    p.problem = Some(PathProblem::Pattern(PatternError::Unknown {
        name: "FILE_NAM".to_owned(),
        at,
        suggestion: Some("FILE_NAME"),
    }));
    p.names.clear();
    answer(&mut harness, 1, p);
    assert!(says(
        &harness,
        "Unknown variable $(FILE_NAM). Did you mean $(FILE_NAME)?"
    ));
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    harness.get_by_label("Use $(FILE_NAME)").click();
    settle(&mut harness);
    assert_eq!(
        held(&mut harness).path,
        "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)"
    );
}

#[test]
fn a_fix_is_not_offered_for_a_path_that_has_changed_since_the_answer() {
    let mut harness = open();
    let mut p = preview();
    // The answer is for another text: the place it names holds something else here.
    p.problem = Some(PathProblem::Pattern(PatternError::Unknown {
        name: "FILE_NAM".to_owned(),
        at: 3..11,
        suggestion: Some("FILE_NAME"),
    }));
    answer(&mut harness, 1, p);
    assert!(says(
        &harness,
        "Unknown variable $(FILE_NAM). Did you mean $(FILE_NAME)?"
    ));
    assert!(harness.query_by_label("Use $(FILE_NAME)").is_none());
}

#[test]
fn an_empty_path_an_unclosed_variable_and_a_disk_that_is_gone_each_stop_export_in_words() {
    let mut harness = open();
    let mut p = preview();
    p.problem = Some(PathProblem::Empty);
    answer(&mut harness, 1, p);
    assert!(says(&harness, "Enter where to put the exported photos."));
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));

    let mut p = preview();
    p.problem = Some(PathProblem::Pattern(PatternError::Unclosed {
        variable: "$(FILE_NAME".to_owned(),
        expected: ')',
        at: 11,
    }));
    answer(&mut harness, 2, p);
    assert!(says(&harness, "Missing “)” after “$(FILE_NAME”."));

    let mut p = preview();
    p.folders = vec![FolderCheck {
        folder: PathBuf::from("/Volumes/Photos NAS/Exports"),
        preflight: Preflight::Blocked {
            at: PathBuf::from("/Volumes/Photos NAS"),
            why: Blocked::NotConnected,
        },
    }];
    answer(&mut harness, 3, p);
    assert!(says(
        &harness,
        "Can’t write to “/Volumes/Photos NAS/Exports”: the disk isn’t connected. Connect it, or choose another folder."
    ));
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    // The answer after it, with the disk back, allows it.
    answer(&mut harness, 4, preview());
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn a_size_over_the_format_s_limit_stops_export() {
    let mut harness = open();
    let mut p = preview();
    p.size = Some(ActiveSize {
        source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
        size: Err(SizeProblem::TooLarge {
            format: "JPEG",
            largest: 65535,
            width: 70000,
            height: 50000,
        }),
    });
    answer(&mut harness, 1, p);
    assert!(says(&harness, "JPEG is at most 65535 px a side."));
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
}

// The footer and the keys.

#[test]
fn export_asks_for_the_export_and_closes_and_close_only_closes() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    harness.get_by_label("Close").click();
    harness.run();
    assert!(harness.state().closed);
    assert!(harness.state().asked.iter().all(|a| !a.start));

    let mut harness = open();
    answer(&mut harness, 1, preview());
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    let state = harness.state();
    assert!(state.closed);
    assert!(state.asked.last().is_some_and(|a| a.start));
}

#[test]
fn return_exports_when_nothing_has_the_focus_and_leaves_it_to_a_control_that_has() {
    let mut harness = open();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(harness.state().closed && harness.state().asked.iter().any(|a| a.start));

    // A dropdown row takes Return itself, to open its list.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert!(!harness.state().closed && harness.state().asked.iter().all(|a| !a.start));

    // Cmd+Return exports from anywhere.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    assert!(harness.state().closed && harness.state().asked.iter().any(|a| a.start));
}

#[test]
fn return_does_not_export_what_the_session_has_found_wrong() {
    let mut harness = open();
    let mut p = preview();
    p.problem = Some(PathProblem::Empty);
    answer(&mut harness, 1, p);
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!harness.state().closed);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    assert!(!harness.state().closed);
}

#[test]
fn escape_closes_the_innermost_thing_first_then_the_dialog() {
    // A dropdown's list first.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .click();
    settle(&mut harness);
    assert!(harness.query_by_role(Role::Menu).is_some());
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert!(!harness.state().closed, "the list closed, not the dialog");
    // The variable list in the path.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .focus();
    harness.run();
    harness.event(egui::Event::Text("$(FI".to_owned()));
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_some());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::ListBox).is_none());
    assert!(!harness.state().closed);
    // A field typed in.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::TextInput, "Width, in px")
        .focus();
    harness.run();
    harness.event(egui::Event::Text("12".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().closed);
    // Then the dialog.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().closed);
}

#[test]
fn cmd_period_and_cmd_w_close_it_too() {
    for key in [egui::Key::Period, egui::Key::W] {
        let mut harness = open();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, key);
        harness.run();
        assert!(harness.state().closed, "{key:?}");
        assert!(harness.state().asked.iter().all(|a| !a.start));
    }
}

#[test]
fn f6_moves_between_the_presets_the_settings_and_the_footer_and_stays_inside() {
    let mut harness = open();
    harness.key_press(egui::Key::F6);
    harness.run();
    assert!(harness.get_by_label("Default settings").is_focused());
    harness.key_press(egui::Key::F6);
    harness.run();
    assert!(
        harness
            .get_by_role_and_label(Role::ComboBox, "Target storage")
            .is_focused()
    );
    harness.key_press(egui::Key::F6);
    harness.run();
    assert!(harness.get_by_label("Close").is_focused());
    // Around, never out of the dialog.
    harness.key_press(egui::Key::F6);
    harness.run();
    assert!(harness.get_by_label("Default settings").is_focused());
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F6);
    harness.run();
    assert!(harness.get_by_label("Close").is_focused());
}

#[test]
fn tab_goes_through_the_presets_then_the_settings_in_the_order_they_read() {
    let mut harness = open();
    let mut order = Vec::new();
    for _ in 0..4 {
        harness.key_press(egui::Key::Tab);
        harness.run();
        for label in ["Default settings", "Target storage", "Path", "On conflict"] {
            let found = harness
                .query_all_by_label(label)
                .any(|node| node.is_focused());
            if found {
                order.push(label);
            }
        }
    }
    assert_eq!(
        order,
        ["Default settings", "Target storage", "Path", "On conflict"]
    );
}

#[test]
fn no_focus_ring_shows_as_the_dialog_opens() {
    let harness = open();
    assert!(harness.ctx.memory(|m| m.focused()).is_none());
}

// How it looks.

fn photographed(
    which: Builtin,
    locale: Locale,
    name: &str,
    changes: impl FnOnce(&mut Harness<'static, Bench>),
) {
    let mut harness = dialog(
        Values::default(),
        which,
        locale,
        egui::vec2(WIDTH + 20.0, HEADER + BODY + FOOTER + 20.0),
    );
    answer(&mut harness, 1, preview());
    changes(&mut harness);
    harness.snapshot(name);
}

#[test]
fn the_dialog_in_the_dark_theme() {
    photographed(Builtin::Dark, Locale::EnGb, "export-dark", |_| {});
}

#[test]
fn the_dialog_in_the_light_theme() {
    photographed(Builtin::Light, Locale::EnGb, "export-light", |_| {});
}

#[test]
fn the_dialog_in_ukrainian() {
    photographed(Builtin::Dark, Locale::Uk, "export-uk", |_| {});
}

#[test]
fn the_dialog_in_polish() {
    photographed(Builtin::Dark, Locale::Pl, "export-pl", |_| {});
}

#[test]
fn the_dialog_with_the_longest_words_the_pseudo_locale_gives() {
    photographed(Builtin::Dark, Locale::Pseudo, "export-pseudo", |_| {});
}

#[test]
fn the_dialog_with_an_error_under_its_path() {
    photographed(Builtin::Dark, Locale::EnGb, "export-error", |harness| {
        let mut p = preview();
        p.problem = Some(PathProblem::Empty);
        answer(harness, 2, p);
    });
}

#[test]
fn the_dialog_is_as_wide_as_designed_in_every_language() {
    for locale in [Locale::EnGb, Locale::Uk, Locale::Pl, Locale::Pseudo] {
        let mut harness = dialog(
            Values::default(),
            Builtin::Dark,
            locale,
            egui::vec2(WIDTH + 20.0, HEADER + BODY + FOOTER + 20.0),
        );
        answer(&mut harness, 1, preview());
        let size = harness.get_by_role(Role::Dialog).rect().size();
        assert_eq!(size.x, WIDTH, "{locale:?}");
        assert_eq!(size.y, HEADER + BODY + FOOTER, "{locale:?}");
    }
}

#[test]
fn its_title_is_its_window_s_frame_s_and_its_own_header_the_mac_sheet_s_alone() {
    let title = "Export 1 photo";
    let size = egui::vec2(WIDTH + 20.0, HEADER + BODY + FOOTER + 60.0);
    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ] {
        let mut harness = dialog_on(
            home(Values::default()),
            Builtin::Dark,
            Locale::EnGb,
            size,
            os,
        );
        answer(&mut harness, 1, preview());
        let window = harness.get_by_role_and_label(Role::Dialog, title).rect();
        let button = harness.get_by_role_and_label(Role::Button, title).rect();
        // The title painted above the body: the sheet's header on the Mac, the host's
        // strip on Linux, nothing on Windows, whose system title bar shows it.
        let above: Vec<egui::Rect> = dialog_host::painted(harness.output())
            .into_iter()
            .filter(|(text, at)| text == title && !button.contains_rect(*at))
            .map(|(_, at)| at)
            .collect();
        let (header, frame) = match os {
            OperatingSystem::Mac => (HEADER, 0.0),
            OperatingSystem::Windows => (0.0, 0.0),
            _ => (0.0, dialog_host::FRAME),
        };
        assert_eq!(
            above.len(),
            usize::from(os != OperatingSystem::Windows),
            "{os:?}"
        );
        if let Some(at) = above.first() {
            assert!(
                at.bottom() <= window.top() + header.max(frame),
                "{os:?} {at:?}"
            );
        }
        // The window is as tall as what it holds.
        assert_eq!(window.height(), frame + header + BODY + FOOTER, "{os:?}");
    }
}
