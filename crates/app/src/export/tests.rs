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

//! The Export tool in a harness that draws it as the Photos tab's side panel does: every
//! row, the preview's lines as the session's answers arrive, the rows drawn disabled, the
//! rows reflowed to the panel's width, and the tool photographed at the panel's widths and
//! in the languages.

use apertrust_engine::export_preview::{ActiveSize, FolderCheck, OutputSize, PhotoName};
use apertrust_ui::components::PanelBody;
use apertrust_ui::{Builtin, Locale, Theme};
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;

/// The side panel's width by default, in points, and the widest it can be dragged to.
const PANEL: f32 = 320.0;
const WIDEST: f32 = 600.0;
/// The narrowest it can be dragged to.
const NARROWEST: f32 = 240.0;

/// The tool as the Photos tab draws it, and what each frame asked of the app.
struct Bench {
    content: Export,
    open: bool,
    photo: bool,
    asked: Vec<Asked>,
    toggled: usize,
    /// What the session was last asked to preview, as the app keeps it, and the photo open.
    previews: Previews,
    opened: Option<PathBuf>,
    /// Where the section ended at the last frame.
    bottom: f32,
}

impl Bench {
    fn export(&mut self) -> &mut Export {
        &mut self.content
    }
}

/// The tool over `values`, in a panel `width` wide that is tall enough to show every row.
fn tool(values: Values, which: Builtin, locale: Locale, width: f32) -> Harness<'static, Bench> {
    tool_of(home(values), which, locale, width)
}

/// The tool over `values`, with Anna's home.
fn home(values: Values) -> Export {
    Export::with_home(values, Some(PathBuf::from("/Users/anna")))
}

/// `export` in the bench.
fn tool_of(export: Export, which: Builtin, locale: Locale, width: f32) -> Harness<'static, Bench> {
    // The photo open is the one the session was asked about, as the tests' answers are for.
    let photo = PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW");
    let mut previews = Previews::default();
    previews.next(Some(&photo), true, export.values(), 0);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(width, 1600.0))
        .with_os(egui::os::OperatingSystem::Mac)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                let (open, photo) = (bench.open, bench.photo);
                // As the side panel does before it draws the Photos tab.
                bench
                    .content
                    .retire(&bench.previews, bench.opened.as_deref());
                PanelBody::new().show(ui, |ui| {
                    let section = ui.scope(|ui| bench.content.show(ui, open, photo));
                    bench.bottom = section.response.rect.bottom();
                    let drawn = section.inner;
                    bench.toggled += usize::from(drawn.toggle);
                    if drawn.toggle {
                        bench.open = !bench.open;
                    }
                    let asked = drawn.asked;
                    if asked.values.is_some() || asked.start || asked.confirm {
                        bench.asked.push(asked);
                    }
                });
            },
            Bench {
                content: export,
                open: true,
                photo: true,
                asked: Vec::new(),
                toggled: 0,
                previews,
                opened: Some(photo),
                bottom: 0.0,
            },
        );
    harness.run();
    harness
}

fn open() -> Harness<'static, Bench> {
    tool(Values::default(), Builtin::Dark, Locale::EnGb, PANEL)
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

/// What the tool holds now.
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

/// The text of the session's feedback, if the tool shows it.
fn says(harness: &Harness<'_, Bench>, text: &str) -> bool {
    harness.query_by_label(text).is_some()
}

// What the first frame and each change ask for.

#[test]
fn the_preview_and_the_export_are_the_session_s_commands_for_the_photo_open() {
    let places = Places::default();
    let photo = Path::new("/Users/anna/Pictures/IMG_2041.ARW");
    let held = Values {
        format: FileFormat::Png,
        ..Values::default()
    };
    assert_eq!(
        preview_command(photo, &held, &places),
        Command::PreviewExport {
            photos: vec![photo.to_owned()],
            settings: held.settings(),
            places: places.clone(),
        }
    );
    assert_eq!(
        crate::export_again::export(photo, &held, &places),
        Command::Export {
            photos: vec![photo.to_owned()],
            settings: held.settings(),
            places,
        }
    );
}

#[test]
fn a_preview_is_asked_only_while_the_tool_is_open_and_shown_and_afresh_when_it_comes_back() {
    let (one, other) = (
        Path::new("/Users/anna/Pictures/IMG_2041.ARW"),
        Path::new("/Users/anna/Pictures/IMG_2042.ARW"),
    );
    let values = Values::default();
    let png = Values {
        format: FileFormat::Png,
        ..Values::default()
    };
    let mut previews = Previews::default();
    // Not shown, or no photo: nothing to ask.
    assert_eq!(previews.next(Some(one), false, &values, 0), Ask::Nothing);
    assert_eq!(previews.next(None, true, &values, 0), Ask::Nothing);
    // Shown: a first preview, then none until something changes.
    let fresh = Ask::Fresh(one.to_owned(), values.clone());
    assert_eq!(previews.next(Some(one), true, &values, 0), fresh);
    assert_eq!(previews.next(Some(one), true, &values, 0), Ask::Nothing);
    // A setting changes: a preview of it, for the same photo.
    assert_eq!(
        previews.next(Some(one), true, &png, 0),
        Ask::Changed(one.to_owned(), png.clone())
    );
    // Another photo is open: a first preview again.
    assert_eq!(
        previews.next(Some(other), true, &png, 0),
        Ask::Fresh(other.to_owned(), png.clone())
    );
    // Out of view, closed or on another tab: nothing, and the way back asks afresh.
    assert_eq!(previews.next(Some(other), false, &png, 0), Ask::Nothing);
    assert_eq!(previews.next(Some(other), false, &png, 0), Ask::Nothing);
    assert_eq!(
        previews.next(Some(other), true, &png, 0),
        Ask::Fresh(other.to_owned(), png.clone())
    );
    // The photo closes, then opens again: afresh as well.
    assert_eq!(previews.next(None, true, &values, 0), Ask::Nothing);
    assert_eq!(
        previews.next(Some(other), true, &values, 0),
        Ask::Fresh(other.to_owned(), values.clone())
    );
}

#[test]
fn a_photo_opened_before_the_next_frame_shows_nothing_of_the_one_before() {
    let values = Values {
        path: "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAM)".to_owned(),
        ..Values::default()
    };
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
    let mut stopped = preview();
    stopped.problem = Some(PathProblem::Pattern(PatternError::Unknown {
        name: "FILE_NAM".to_owned(),
        at: 36..44,
        suggestion: Some("FILE_NAME"),
    }));
    stopped.names.clear();
    answer(&mut harness, 1, stopped);
    assert!(says(
        &harness,
        "Unknown variable $(FILE_NAM). Did you mean $(FILE_NAME)?"
    ));
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    // Another photo opens: the frame that draws it comes before the preview for it is
    // asked, and has nothing of the first photo's answer to say or to rest Export for.
    harness.state_mut().opened = Some(PathBuf::from("/Users/anna/Pictures/IMG_2042.ARW"));
    harness.run();
    assert!(!says(
        &harness,
        "Unknown variable $(FILE_NAM). Did you mean $(FILE_NAME)?"
    ));
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn a_preview_is_asked_again_when_the_edit_changes_or_an_export_ends() {
    let photo = Path::new("/Users/anna/Pictures/IMG_2041.ARW");
    let values = Values::default();
    let mut previews = Previews::default();
    assert!(matches!(
        previews.next(Some(photo), true, &values, 3),
        Ask::Fresh(..)
    ));
    assert_eq!(previews.next(Some(photo), true, &values, 3), Ask::Nothing);
    // The generation moves, as an Undo or the end of an export moves it: the same photo and
    // settings, asked again as a change, so the notes stay in view until the answer.
    let again = Ask::Changed(photo.to_owned(), values.clone());
    assert_eq!(previews.next(Some(photo), true, &values, 4), again);
    assert_eq!(previews.next(Some(photo), true, &values, 4), Ask::Nothing);
    assert_eq!(previews.next(Some(photo), true, &values, 5), again);
    // Out of view, the generation changing asks nothing; showing it asks afresh.
    assert_eq!(previews.next(Some(photo), false, &values, 6), Ask::Nothing);
    assert!(matches!(
        previews.next(Some(photo), true, &values, 6),
        Ask::Fresh(..)
    ));
}

#[test]
fn a_change_asks_for_its_settings_and_the_first_frame_asks_nothing() {
    let mut harness = open();
    // The app asks for the first preview as the tool is shown, not the tool.
    assert!(harness.state().asked.is_empty());
    harness.run();
    assert!(harness.state().asked.is_empty());
    // A letter typed in the path is a new preview, with the whole path.
    type_into(&mut harness, Role::TextInput, "Path", "x");
    let asked = harness.state().asked.last().cloned().expect("asked");
    assert!(!asked.start && !asked.confirm);
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
    // The section's header, and its button.
    harness.get_by_role_and_label(Role::Button, "Export");
    harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    for combo in [
        "Target storage",
        "On conflict",
        "File format",
        "Chroma subsampling",
        "Set size",
        "Profile",
        "Intent",
        "Style",
    ] {
        harness.get_by_role_and_label(Role::ComboBox, combo);
    }
    for toggle in ["Allow upscaling", "High quality resampling", "Store masks"] {
        harness.get_by_role_and_label(Role::CheckBox, toggle);
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
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
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
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
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
fn allow_upscaling_and_high_quality_resampling_are_toggles() {
    let mut harness = open();
    for toggle in ["Allow upscaling", "High quality resampling"] {
        let node = harness.get_by_role_and_label(Role::CheckBox, toggle);
        node.scroll_to_me();
        settle(&mut harness);
        harness
            .get_by_role_and_label(Role::CheckBox, toggle)
            .click();
        settle(&mut harness);
    }
    let now = held(&mut harness);
    assert!(now.upscale && now.high_quality);
    harness
        .get_by_role_and_label(Role::CheckBox, "High quality resampling")
        .click();
    settle(&mut harness);
    let now = held(&mut harness);
    assert!(now.upscale && !now.high_quality);
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
    // The tool's rule for a side left at 0 applies.
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
    // The file holds the three, in darktable's keys' places, and nothing else of the tool.
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
fn a_setting_conf_gives_is_what_the_tool_starts_with_and_only_the_other_change_is_kept() {
    use crate::settings::{Keeper, Settings, overrides, testing::Folder};

    let folder = Folder::new("export-conf");
    let given = overrides(&["export.size.mode=cm".to_owned()]).expect("a known key");
    let (mut keeper, settings) = Keeper::open_with(folder.storage(), &given);
    assert_eq!(settings.export.size_mode, SizeMode::Cm);
    // The tool starts with it, and the person changes that row and another.
    let mut harness = tool(settings.export.clone(), Builtin::Dark, Locale::EnGb, PANEL);
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

// The header's reset.

#[test]
fn the_header_s_reset_applies_default_settings_and_rests_while_they_are_in_place() {
    let mut harness = open();
    assert!(disabled(&harness, Role::Button, "Reset export settings"));
    choose(&mut harness, "On conflict", "Skip");
    assert!(!disabled(&harness, Role::Button, "Reset export settings"));
    harness.state_mut().asked.clear();
    harness
        .get_by_role_and_label(Role::Button, "Reset export settings")
        .click();
    settle(&mut harness);
    assert_eq!(held(&mut harness), Values::default());
    assert!(disabled(&harness, Role::Button, "Reset export settings"));
    // The change is asked of the app as any other: it keeps the settings and previews them.
    let asked = harness.state().asked.last().cloned().expect("asked");
    assert_eq!(asked.values, Some(Values::default()));
    assert!(!asked.start && !asked.confirm);
}

#[test]
fn the_header_opens_and_closes_the_tool_and_a_closed_one_draws_no_rows() {
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::Button, "Export")
        .click();
    settle(&mut harness);
    assert_eq!(harness.state().toggled, 1);
    assert!(!harness.state().open);
    assert!(harness.query_by_label("Target storage").is_none());
    assert!(harness.query_by_label("Export 1 photo").is_none());
    harness
        .get_by_role_and_label(Role::Button, "Export")
        .click();
    settle(&mut harness);
    assert!(harness.state().open);
    harness.get_by_label("Target storage");
}

// The rows that are drawn disabled.

#[test]
fn every_row_not_built_is_drawn_disabled() {
    let mut harness = open();
    for combo in ["Intent", "Style"] {
        assert!(disabled(&harness, Role::ComboBox, combo), "{combo}");
    }
    assert!(disabled(&harness, Role::CheckBox, "Store masks"));
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
    // Set outside the tool, as `--conf` or the file can, it is the same.
    let values = Values {
        on_conflict: OnConflict::Overwrite,
        ..Values::default()
    };
    let harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn on_linux_the_host_draws_the_alert_so_overwrite_is_listed_enabled() {
    let alerts = crate::dialog_host::platform::shows_alerts(egui::os::OperatingSystem::Nix);
    let mut harness = tool_of(
        home(Values::default()).alerts(alerts),
        Builtin::Dark,
        Locale::EnGb,
        PANEL,
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
    let mut harness = tool_of(
        home(Values::default()).alerts(false),
        Builtin::Dark,
        Locale::EnGb,
        PANEL,
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
    // Kept from a platform that asks: shown, but Export rests, saying why, and a press
    // exports nothing.
    let mut harness = tool_of(
        home(Values {
            on_conflict: OnConflict::Overwrite,
            ..Values::default()
        })
        .alerts(false),
        Builtin::Dark,
        Locale::EnGb,
        PANEL,
    );
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
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
    tool(
        Values {
            on_conflict: conflict,
            ..Values::default()
        },
        Builtin::Dark,
        Locale::EnGb,
        PANEL,
    )
}

/// Presses Export and says what the frame asked: whether the export starts and whether the
/// overwrite question is to be asked. Nothing closes: the tool is never closed by it.
fn press_export(harness: &mut Harness<'_, Bench>) -> (bool, bool) {
    let before = harness.state().asked.len();
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    let state = harness.state();
    assert!(state.open && state.toggled == 0, "Export closes nothing");
    let asked = &state.asked[before..];
    (
        asked.iter().any(|a| a.start),
        asked.iter().any(|a| a.confirm),
    )
}

#[test]
fn export_in_overwrite_mode_asks_first_whether_or_not_a_file_is_there_and_the_tool_stays() {
    // darktable's rule (`ask_user_confirmation`): On conflict Overwrite asks, with no
    // preview yet, with one that finds no file and with one that finds a file to replace.
    let mut harness = overwriting(OnConflict::Overwrite);
    // Not a start: the app asks, and sends the export on a yes.
    assert_eq!(press_export(&mut harness), (false, true));
    answer(&mut harness, 1, preview());
    assert_eq!(press_export(&mut harness), (false, true));
    answer(&mut harness, 2, preview_with_file_there(Action::Overwrite));
    assert_eq!(press_export(&mut harness), (false, true));
    // Asked again, the same question: a no leaves the tool as it was.
    assert_eq!(press_export(&mut harness), (false, true));
    // Changing the path does not change the rule, whatever the preview says.
    type_into(&mut harness, Role::TextInput, "Path", "x");
    assert_eq!(press_export(&mut harness), (false, true));
}

#[test]
fn export_asks_nothing_in_the_other_ways_to_deal_with_a_file_there() {
    // Overwrite if changed never asks, as darktable's does not, a file replaced or not.
    let mut harness = overwriting(OnConflict::OverwriteIfChanged);
    answer(&mut harness, 1, preview_with_file_there(Action::Overwrite));
    assert_eq!(press_export(&mut harness), (true, false));
    let mut harness = overwriting(OnConflict::OverwriteIfChanged);
    answer(
        &mut harness,
        1,
        preview_with_file_there(Action::Skip(apertrust_export::disk::Why::NotChanged)),
    );
    assert_eq!(press_export(&mut harness), (true, false));
    // Nor do the other two, with no preview or with one.
    let mut harness = overwriting(OnConflict::CreateUniqueFilename);
    assert_eq!(press_export(&mut harness), (true, false));
    for (conflict, action) in [
        (OnConflict::CreateUniqueFilename, Action::Number),
        (
            OnConflict::Skip,
            Action::Skip(apertrust_export::disk::Why::Exists),
        ),
    ] {
        let mut harness = overwriting(conflict);
        answer(&mut harness, 1, preview_with_file_there(action));
        assert_eq!(press_export(&mut harness), (true, false), "{conflict:?}");
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
        "IMG_2041.ARW will be 6000 × 4000 px (not upscaled: Allow upscaling is off)."
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
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
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

// The button and the keys.

#[test]
fn export_asks_for_the_export_and_closes_nothing() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    let state = harness.state();
    assert!(state.asked.last().is_some_and(|a| a.start));
    assert!(state.open && state.toggled == 0);
    // And it can be pressed again, as the settings stand.
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    assert_eq!(harness.state().asked.iter().filter(|a| a.start).count(), 2);
}

#[test]
fn with_no_photo_open_export_rests_and_says_why() {
    let mut harness = open();
    harness.state_mut().photo = false;
    harness.run();
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    assert!(harness.state().asked.is_empty());
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    harness.hover_at(button.rect().center());
    harness.run_steps(40);
    assert!(
        harness
            .query_by_label("Open a photo to export it.")
            .is_some()
    );
}

#[test]
fn return_exports_nothing_of_its_own_and_leaves_the_rows_their_keys() {
    let mut harness = open();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    assert!(harness.state().asked.is_empty());
    // A dropdown row takes Return itself, to open its list.
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert!(harness.query_by_role(Role::Menu).is_some());
    assert!(harness.state().asked.is_empty());
}

#[test]
fn escape_closes_a_list_and_a_row_being_typed_in_and_nothing_else() {
    // A dropdown's list.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .click();
    settle(&mut harness);
    assert!(harness.query_by_role(Role::Menu).is_some());
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(harness.query_by_role(Role::Menu).is_none());
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
    // The tool stays open throughout.
    assert!(harness.state().open && harness.state().toggled == 0);
}

#[test]
fn tab_goes_through_the_header_then_the_rows_in_the_order_they_read() {
    let mut harness = open();
    let mut order = Vec::new();
    for _ in 0..6 {
        harness.key_press(egui::Key::Tab);
        harness.run();
        for label in ["Export", "Target storage", "Path", "On conflict"] {
            let found = harness
                .query_all_by_label(label)
                .any(|node| node.is_focused());
            if found && !order.contains(&label) {
                order.push(label);
            }
        }
    }
    assert_eq!(order, ["Export", "Target storage", "Path", "On conflict"]);
}

#[test]
fn no_focus_ring_shows_as_the_tool_is_drawn() {
    let harness = open();
    assert!(harness.ctx.memory(|m| m.focused()).is_none());
}

// The rows reflowed to the panel's width.

/// The vertical middle of the first node labelled `label`.
fn middle(harness: &Harness<'_, Bench>, label: &str) -> f32 {
    harness.get_by_label(label).rect().center().y
}

#[test]
fn a_row_whose_label_and_fields_do_not_fit_puts_its_label_above_its_fields() {
    let values = Values {
        size_mode: SizeMode::Cm,
        ..Values::default()
    };
    // The narrowest panel: "Size in centimetres" has no room beside its two fields.
    let narrow = tool(values.clone(), Builtin::Dark, Locale::EnGb, NARROWEST);
    let label = middle(&narrow, "Size in centimetres");
    let field = middle(&narrow, "Width, in cm");
    assert!(field > label + 10.0, "label {label}, field {field}");
    // The widest: the label and its fields share a line.
    let wide = tool(values, Builtin::Dark, Locale::EnGb, WIDEST);
    let label = middle(&wide, "Size in centimetres");
    let field = middle(&wide, "Width, in cm");
    assert!((field - label).abs() < 4.0, "label {label}, field {field}");
}

#[test]
fn every_row_stays_inside_the_panel_at_each_width_in_each_language() {
    for locale in [Locale::EnGb, Locale::Uk, Locale::Pl, Locale::Pseudo] {
        for width in [NARROWEST, PANEL, WIDEST] {
            for mode in [SizeMode::Pixels, SizeMode::Cm, SizeMode::Scale] {
                let values = Values {
                    size_mode: mode,
                    ..Values::default()
                };
                let mut harness = tool(values, Builtin::Dark, locale, width);
                answer(&mut harness, 1, preview());
                let outside: Vec<String> = harness
                    .root()
                    .children_recursive()
                    // A text run's box is the whole text, clipped by its field.
                    .filter(|node| node.accesskit_node().role() != Role::TextRun)
                    .filter_map(|node| {
                        let at = node.accesskit_node().bounding_box()?;
                        let out = at.x1 > f64::from(width) + 0.5 || at.x0 < -0.5;
                        out.then(|| format!("{:?} {at:?}", node.accesskit_node().label()))
                    })
                    .collect();
                assert!(
                    outside.is_empty(),
                    "{locale:?} {width} {mode:?}: {outside:#?}"
                );
            }
        }
    }
}

// How it looks.

/// The panel as tall as the tool, and a little under it, for the snapshot `name`.
fn photograph(harness: &mut Harness<'static, Bench>, name: &str) {
    let width = harness.ctx.content_rect().width();
    let height = harness.state().bottom + 12.0;
    harness.set_size(egui::vec2(width, height));
    harness.run();
    harness.snapshot(name);
}

fn photographed(
    which: Builtin,
    locale: Locale,
    width: f32,
    name: &str,
    changes: impl FnOnce(&mut Harness<'static, Bench>),
) {
    let mut harness = tool(Values::default(), which, locale, width);
    answer(&mut harness, 1, preview());
    changes(&mut harness);
    photograph(&mut harness, name);
}

#[test]
fn the_tool_at_the_panel_s_width_in_the_dark_theme() {
    photographed(
        Builtin::Dark,
        Locale::EnGb,
        PANEL,
        "export-tool-320-dark",
        |_| {},
    );
}

#[test]
fn the_tool_at_the_panel_s_width_in_the_light_theme() {
    photographed(
        Builtin::Light,
        Locale::EnGb,
        PANEL,
        "export-tool-320-light",
        |_| {},
    );
}

#[test]
fn the_tool_at_the_widest_panel_in_the_dark_theme() {
    photographed(
        Builtin::Dark,
        Locale::EnGb,
        WIDEST,
        "export-tool-600-dark",
        |_| {},
    );
}

#[test]
fn the_tool_at_the_widest_panel_in_the_light_theme() {
    photographed(
        Builtin::Light,
        Locale::EnGb,
        WIDEST,
        "export-tool-600-light",
        |_| {},
    );
}

#[test]
fn the_tool_in_ukrainian() {
    photographed(Builtin::Dark, Locale::Uk, PANEL, "export-tool-uk", |_| {});
}

#[test]
fn the_tool_in_polish() {
    photographed(Builtin::Dark, Locale::Pl, PANEL, "export-tool-pl", |_| {});
}

#[test]
fn the_tool_with_the_longest_words_the_pseudo_locale_gives() {
    photographed(
        Builtin::Dark,
        Locale::Pseudo,
        PANEL,
        "export-tool-pseudo",
        |_| {},
    );
}

#[test]
fn the_tool_with_an_error_under_its_path() {
    photographed(
        Builtin::Dark,
        Locale::EnGb,
        PANEL,
        "export-tool-error",
        |harness| {
            let mut p = preview();
            p.problem = Some(PathProblem::Empty);
            answer(harness, 2, p);
        },
    );
}

#[test]
fn the_tool_in_print_sizes_at_the_narrowest_panel() {
    let values = Values {
        size_mode: SizeMode::Cm,
        ..Values::default()
    };
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, NARROWEST);
    answer(&mut harness, 1, preview());
    photograph(&mut harness, "export-tool-240-narrow");
}
