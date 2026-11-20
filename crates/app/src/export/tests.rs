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

//! The Export tool in a harness that draws it as the Photos tab's Tools panel does: every
//! row, the preview's lines as the session's answers arrive, the rows drawn disabled, the
//! rows reflowed to the panel's width, and the tool photographed at the panel's widths and
//! in the languages.

use apertrust_engine::export_preview::{ActiveSize, FolderCheck, OutputSize, PhotoName};
use apertrust_export::disk::Action;
use apertrust_ui::components::PanelBody;
use apertrust_ui::{Builtin, Locale, Theme};
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;

/// The Tools panel's width by default, in points, and the widest it can be dragged to.
const PANEL: f32 = 320.0;
const WIDEST: f32 = 600.0;
/// The narrowest it can be dragged to.
const NARROWEST: f32 = 240.0;

/// What the app made of a frame of the tool: the settings it kept, when they changed, and
/// whether a press started an export or asked first.
#[derive(Clone, Debug, Default, PartialEq)]
struct Seen {
    values: Option<Values>,
    start: bool,
    confirm: bool,
}

/// The tool as the Photos tab draws it, and what each frame asked of the app.
struct Bench {
    content: Export,
    /// Whether its section is open, as the shell's header has it, and what the app tells
    /// the tool each frame; the bench follows the toggles it reports.
    open: bool,
    shown: Shown,
    /// The app's settings as the tool's page reads them: Global options open or closed.
    settings: crate::settings::Settings,
    asked: Vec<Seen>,
    toggled: usize,
    /// How many times Show jobs was pressed.
    jobs_shown: usize,
    /// What the session was last asked to preview, as the app keeps it, and the photo open.
    previews: Previews,
    opened: Option<PathBuf>,
    /// Where the section ended at the last frame.
    bottom: f32,
    /// The folder button's dialog, in place of the platform's.
    picker: Picker,
    /// Whether Export… reveals the tool this frame, as the Photos tab tells it.
    reveal: bool,
}

/// A stand-in for the folder button's dialog and the disk under it: `folders` are the
/// folders there, `answer` what the person will choose, a folder or None to cancel; `asked`
/// holds each dialog asked for, with the folder it opened at.
#[derive(Default)]
struct Picker {
    folders: Vec<PathBuf>,
    answer: Option<Option<PathBuf>>,
    asked: Vec<(crate::dialogs::Pick, Option<PathBuf>)>,
    asking: bool,
}

impl crate::dialogs::OpenDialog for Picker {
    fn asking(&self) -> bool {
        self.asking
    }

    fn ask(&mut self, pick: crate::dialogs::Pick, folder: Option<&Path>) {
        self.asking = true;
        self.asked.push((pick, folder.map(Path::to_owned)));
    }

    fn chosen(&mut self) -> Option<crate::dialogs::Chosen> {
        if !self.asking {
            return None;
        }
        let answer = self.answer.take()?;
        self.asking = false;
        let (pick, _) = self.asked.last().cloned()?;
        let folder = answer?;
        Some(crate::dialogs::Chosen {
            pick,
            paths: vec![folder],
        })
    }
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
                // As the Tools panel does before it draws the Photos tab.
                bench
                    .content
                    .retire(&bench.previews, bench.opened.as_deref());
                let shown = bench.shown.clone();
                // As the Photos tab gives every utility its context.
                let mirror = crate::mirror::tests::mirror();
                let (words, mut commands) = (apertrust_ui::translator(ui.ctx()), Vec::new());
                let mut cx = crate::utility::Context {
                    mirror: &mirror,
                    settings: &bench.settings,
                    words: &words,
                    commands: &mut commands,
                };
                let head = crate::tools::Head {
                    open: bench.open,
                    ..crate::tools::Head::default()
                };
                let (mut changes, mut asked) = (Vec::new(), Asked::default());
                // As the Photos tab tells it Export… revealed it.
                if std::mem::take(&mut bench.reveal) {
                    bench.content.reveal(ui.ctx());
                }
                PanelBody::new().show(ui, |ui| {
                    let section = ui.scope(|ui| bench.content.show(ui, head, &shown, &mut cx));
                    bench.bottom = section.response.rect.bottom();
                    let drawn = section.inner;
                    bench.toggled += usize::from(drawn.toggle);
                    bench.jobs_shown += usize::from(drawn.show_jobs);
                    bench.open ^= drawn.toggle;
                    bench.shown.summary ^= drawn.toggle_summary;
                    asked = drawn.asked;
                    changes = drawn.changes;
                });
                // As the app carries them out: a setting routed back to the tool, kept as
                // the settings it changed, and Global options opened or closed; then what
                // the tool asked, a press deciding from the settings the export takes.
                for change in changes {
                    match change {
                        Change::Setting { setting, value, .. } => {
                            UtilityTool::set(&mut bench.content, setting, &value);
                            bench.asked.push(Seen {
                                values: Some(bench.content.values().clone()),
                                ..Seen::default()
                            });
                            ui.ctx().request_repaint();
                        }
                        Change::Group { tool, group } => {
                            let first = apertrust_tools::utility::opens(tool, group);
                            let groups = bench.settings.edit.groups.entry(tool.to_owned());
                            let open = groups.or_default().entry(group.to_owned());
                            let open = open.or_insert(first);
                            *open = !*open;
                        }
                        _ => {}
                    }
                }
                // The folder button, as the app looks at the disk and asks its dialog
                // (`App::export`, `App::finish_frame`), the disk and the dialog the bench's.
                let look = asked.choose_folder.then(|| {
                    let folders = &bench.picker.folders;
                    let is_dir = |path: &Path| folders.iter().any(|f| f == path);
                    let (path, home) = (&bench.content.values().path, bench.content.home());
                    folder::look(path, home, false, is_dir)
                });
                let pick =
                    (look.as_ref()).map(|l| crate::dialogs::Pick::Destination(l.file.clone()));
                let start = look.and_then(|look| look.start);
                let chosen = crate::dialogs::open(&mut bench.picker, pick, || start);
                if let Some(chosen) = chosen
                    && let crate::dialogs::Pick::Destination(file) = &chosen.pick
                    && bench.content.choose_folder(&chosen.paths[0], file)
                {
                    bench.asked.push(Seen {
                        values: Some(bench.content.values().clone()),
                        ..Seen::default()
                    });
                    ui.ctx().request_repaint();
                }
                if asked.changed || asked.pressed {
                    let press = asked.pressed.then(|| press(bench.content.values()));
                    if press == Some(Press::Start) {
                        bench.content.note_started(ui.ctx().input(|i| i.time));
                    }
                    bench.asked.push(Seen {
                        values: asked.changed.then(|| bench.content.values().clone()),
                        start: press == Some(Press::Start),
                        confirm: press == Some(Press::Confirm),
                    });
                }
            },
            Bench {
                content: export,
                open: true,
                settings: crate::settings::Settings::default(),
                shown: Shown {
                    summary: SUMMARY_OPEN,
                    photo: Photo::Ready {
                        name: "IMG_2041.ARW".to_owned(),
                    },
                    others: Others::default(),
                    jobs: Jobs::default(),
                },
                asked: Vec::new(),
                toggled: 0,
                jobs_shown: 0,
                previews,
                opened: Some(photo),
                bottom: 0.0,
                picker: Picker::default(),
                reveal: false,
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

/// What the path field says to screen readers beyond its name: the resolved file name, the
/// sequence rule and the variables with no value here, which its tooltip shows too.
fn path_tip(harness: &Harness<'_, Bench>) -> String {
    let path = harness.get_by_role_and_label(Role::TextInput, "Path");
    path.accesskit_node()
        .description()
        .unwrap_or_default()
        .to_owned()
}

/// What the Set size row's tooltip and description say: what the photo will be, and in the
/// print modes and by scale what the fields come to.
fn size_tip(harness: &Harness<'_, Bench>) -> String {
    let row = harness.get_by_role_and_label(Role::ComboBox, "Set size");
    row.accesskit_node().description().unwrap_or_default()
}

/// Whether the path field's tooltip and description say `text`.
fn tip_says(harness: &Harness<'_, Bench>, text: &str) -> bool {
    path_tip(harness).contains(text)
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
        crate::export_again::export(vec![photo.to_owned()], &held, &places),
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
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/five/IMG_2041.jpg"
    ));
    // One that finished after a newer command was sent still arrives, with a lower number.
    answer(
        &mut harness,
        4,
        named("/Users/anna/Pictures/four/IMG_2041.jpg"),
    );
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/five/IMG_2041.jpg"
    ));
    answer(
        &mut harness,
        6,
        named("/Users/anna/Pictures/six/IMG_2041.jpg"),
    );
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/six/IMG_2041.jpg"
    ));
}

#[test]
fn an_answer_to_an_older_request_arriving_after_a_newer_one_is_dropped() {
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
    harness.state_mut().export().sent(5);
    answer(
        &mut harness,
        5,
        named("/Users/anna/Pictures/five/IMG_2041.jpg"),
    );
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/five/IMG_2041.jpg"
    ));
    // Two changes asked again; the first's answer, finished before it was cancelled, comes
    // after the second was sent, for settings no longer in place.
    harness.state_mut().export().sent(6);
    harness.state_mut().export().sent(7);
    answer(
        &mut harness,
        6,
        named("/Users/anna/Pictures/six/IMG_2041.jpg"),
    );
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/five/IMG_2041.jpg"
    ));
    assert!(!tip_says(&harness, "six"));
    answer(
        &mut harness,
        7,
        named("/Users/anna/Pictures/seven/IMG_2041.jpg"),
    );
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/seven/IMG_2041.jpg"
    ));
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
        "Allow upscaling",
        "High quality resampling",
        "Store masks",
    ] {
        harness.get_by_role_and_label(Role::ComboBox, combo);
    }
    // No row is a check box or a switch: darktable's No and Yes are choices.
    assert!(harness.query_by_role(Role::CheckBox).is_none());
    assert!(harness.query_by_role(Role::Switch).is_none());
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
    assert!(size_tip(&harness).contains("Which is equal to 2048 × 0 px"));
    // No line under the row says it.
    assert!(harness.query_by_label_contains("Which is equal").is_none());
    type_into(&mut harness, Role::TextInput, "Height, in cm", "10");
    harness.key_press(egui::Key::Enter);
    harness.run();
    // darktable's cm2pixels: 10 cm at 300 dpi, rounded up.
    assert_eq!(held(&mut harness).height, 1182);
    assert!(size_tip(&harness).contains("Which is equal to 2048 × 1182 px"));
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
fn allow_upscaling_and_high_quality_resampling_are_choices_of_no_and_yes() {
    let mut harness = open();
    // Each row reads No, darktable's default, its options No and Yes.
    for row in ["Allow upscaling", "High quality resampling"] {
        let value = harness.get_by_role_and_label(Role::ComboBox, row);
        assert_eq!(
            value.accesskit_node().value().as_deref(),
            Some("No"),
            "{row}"
        );
        value.click();
        settle(&mut harness);
        harness.get_by_role_and_label(Role::MenuItemCheckBox, "No");
        harness.get_by_role_and_label(Role::MenuItemCheckBox, "Yes");
        harness.key_press(egui::Key::Escape);
        settle(&mut harness);
    }
    choose(&mut harness, "Allow upscaling", "Yes");
    choose(&mut harness, "High quality resampling", "Yes");
    let now = held(&mut harness);
    assert!(now.upscale && now.high_quality);
    assert_eq!(
        harness
            .get_by_role_and_label(Role::ComboBox, "Allow upscaling")
            .accesskit_node()
            .value()
            .as_deref(),
        Some("Yes")
    );
    choose(&mut harness, "High quality resampling", "No");
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

/// A number typed into a row and left by a press of ↺ is committed in the frame Default
/// settings applies, which wins: the reset is what was asked for last.
#[test]
fn a_reset_wins_over_a_value_typed_and_left_by_its_click() {
    let values = Values {
        jpeg_quality: 80,
        ..Values::default()
    };
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
    harness
        .get_by_role_and_label(Role::Slider, "Quality")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.event(egui::Event::Text("65".to_owned()));
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Reset export settings")
        .click();
    settle(&mut harness);
    assert_eq!(held(&mut harness), Values::default());
    // And its ↺ rests.
    assert!(disabled(&harness, Role::Button, "Reset export settings"));
}

/// The settings `--conf` gives for this run, a setting row's and a custom row's.
const GIVEN: [&str; 3] = [
    "export.disk.on_conflict",
    "export.disk.path",
    "export.size.width",
];

/// The tool over `values`, its settings in [`GIVEN`] given by `--conf`.
fn given(values: Values) -> Harness<'static, Bench> {
    let export = home(values).given(GIVEN.map(str::to_owned));
    tool_of(export, Builtin::Dark, Locale::EnGb, PANEL)
}

#[test]
fn default_settings_keeps_each_value_conf_gives_and_resets_the_rest() {
    let values = Values {
        on_conflict: OnConflict::Skip,
        path: "/Users/anna/out/$(FILE_NAME)".to_owned(),
        width: 1200,
        format: FileFormat::Png,
        height: 800,
        ..Values::default()
    };
    let kept = Values {
        on_conflict: OnConflict::Skip,
        path: "/Users/anna/out/$(FILE_NAME)".to_owned(),
        width: 1200,
        ..Values::default()
    };
    let mut harness = given(values);
    assert!(!disabled(&harness, Role::Button, "Reset export settings"));
    harness.state_mut().asked.clear();
    harness
        .get_by_role_and_label(Role::Button, "Reset export settings")
        .click();
    settle(&mut harness);
    assert_eq!(held(&mut harness), kept);
    // What it gives is all Default settings can reach: its ↺ rests.
    assert!(disabled(&harness, Role::Button, "Reset export settings"));
    // The app keeps what the tool holds, `--conf`'s values with it.
    let asked = harness.state().asked.last().cloned().expect("asked");
    assert_eq!(asked.values, Some(kept));
}

#[test]
fn a_row_conf_gives_is_drawn_disabled_at_its_value_saying_so() {
    let values = Values {
        on_conflict: OnConflict::Skip,
        path: "/Users/anna/out/$(FILE_NAME)".to_owned(),
        width: 1200,
        ..Values::default()
    };
    let mut harness = given(values.clone());
    // A setting row, a custom row's field, and a field of the size line.
    assert!(disabled(&harness, Role::ComboBox, "On conflict"));
    assert!(disabled(&harness, Role::TextInput, "Path"));
    assert!(disabled(&harness, Role::TextInput, "Width, in px"));
    // The others stay editable.
    assert!(!disabled(&harness, Role::TextInput, "Height, in px"));
    assert!(!disabled(&harness, Role::ComboBox, "File format"));
    // The resolution and the scale say so too, in their modes.
    for (mode, field) in [
        (SizeMode::Cm, "Resolution, in dpi"),
        (SizeMode::Scale, "Scale"),
    ] {
        let values = Values {
            size_mode: mode,
            ..Values::default()
        };
        let keys = ["export.size.dpi", "export.size.scale"].map(str::to_owned);
        let export = home(values).given(keys);
        let harness = tool_of(export, Builtin::Dark, Locale::EnGb, PANEL);
        let node = harness.get_by_role_and_label(Role::TextInput, field);
        assert!(node.accesskit_node().is_disabled(), "{field}");
        assert_eq!(
            node.accesskit_node().description(),
            Some("Set by --conf for this run".to_owned()),
            "{field}"
        );
    }
    // Each says why, to screen readers too.
    for field in ["Path", "Width, in px"] {
        let field = harness.get_by_role_and_label(Role::TextInput, field);
        assert_eq!(
            field.accesskit_node().description(),
            Some("Set by --conf for this run".to_owned())
        );
    }
    let width = harness.get_by_role_and_label(Role::TextInput, "Width, in px");
    harness.hover_at(width.rect().center());
    harness.run_steps(40);
    assert!(says(&harness, "Set by --conf for this run"));
    // Typing into them changes nothing.
    type_into(&mut harness, Role::TextInput, "Path", "x");
    type_into(&mut harness, Role::TextInput, "Width, in px", "9");
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(held(&mut harness), values);
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
    assert!(disabled(&harness, Role::ComboBox, "Store masks"));
    assert_eq!(
        harness
            .get_by_role_and_label(Role::ComboBox, "Store masks")
            .accesskit_node()
            .value()
            .as_deref(),
        Some("No")
    );
    // The folder button is built: enabled.
    assert!(!disabled(&harness, Role::Button, "Choose folder"));
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
    let row = harness
        .get_by_role_and_label(Role::Button, "Metadata")
        .rect();
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
fn the_active_photo_s_file_and_size_are_said_in_their_rows_tooltips() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/apertrust_exported/IMG_2041.jpg"
    ));
    // No limit: the full size.
    assert!(
        size_tip(&harness)
            .contains("IMG_2041.ARW will be 6000 × 4000 px (full size: no limit set).")
    );
    assert!(
        harness.query_by_label_contains("will be").is_none(),
        "no line under the row"
    );
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
    assert!(size_tip(&harness).contains("IMG_2041.ARW will be 2048 × 1365 px."));
    // A limit larger than the photo, with upscaling off: kept at its full size, and said so.
    type_into(&mut harness, Role::TextInput, "Width, in px", "9000");
    harness.key_press(egui::Key::Enter);
    harness.run();
    answer(&mut harness, 3, preview());
    assert!(
        size_tip(&harness).contains(
            "IMG_2041.ARW will be 6000 × 4000 px (not upscaled: Allow upscaling is off)."
        )
    );
}

#[test]
fn a_file_already_there_is_a_warning_when_it_is_replaced_and_never_a_note_under_on_conflict() {
    let mut harness = open();
    let mut there = preview();
    let name = PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2041_01.jpg");
    there.names[0].fate = Fate::File {
        file: name.clone(),
        exists: true,
        action: Action::Number,
    };
    answer(&mut harness, 1, there.clone());
    // Saved under a number or skipped: no line anywhere says so.
    assert!(harness.query_by_label_contains("already there").is_none());
    there.names[0].fate = Fate::File {
        file: name.clone(),
        exists: true,
        action: Action::Skip(apertrust_export::disk::Why::NotChanged),
    };
    answer(&mut harness, 2, there.clone());
    assert!(harness.query_by_label_contains("already there").is_none());
    // The Summary's file name is the name the session resolved.
    harness.get_by_label("File name IMG_2041_01.jpg");
    // Replaced: the one warning, at the end.
    there.names[0].fate = Fate::File {
        file: name,
        exists: true,
        action: Action::Overwrite,
    };
    answer(&mut harness, 3, there);
    assert!(says(&harness, "Replaces 1 file already there"));
    // The export passing the photo open over, its file is none of the export's.
    harness.state_mut().shown.others = Others {
        taken: 2,
        skips_open: true,
        ..Others::default()
    };
    harness.run();
    assert!(!says(&harness, "Replaces 1 file already there"));
    harness.state_mut().shown.others = Others::default();
    harness.run();
    assert!(says(&harness, "Replaces 1 file already there"));
    // Nothing there, nothing said.
    answer(&mut harness, 4, preview());
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
    assert!(tip_says(&harness, "and 2 more, the same way"));
    assert!(tip_says(&harness, "_$(SEQUENCE) is added"));
    // What On conflict does is the summary's to say, no line under the row.
    assert!(harness.query_by_label_contains("already there").is_none());
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
    assert!(tip_says(
        &harness,
        "$(OPENCL.ACTIVATED): darktable's, with no value here"
    ));
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
fn a_path_conf_gives_keeps_its_problem_but_offers_no_fix_and_takes_none() {
    let values = Values {
        path: "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAM)".to_owned(),
        ..Values::default()
    };
    let mut harness = given(values.clone());
    let mut p = preview();
    p.problem = Some(PathProblem::Pattern(PatternError::Unknown {
        name: "FILE_NAM".to_owned(),
        at: 36..44,
        suggestion: Some("FILE_NAME"),
    }));
    p.names.clear();
    answer(&mut harness, 1, p);
    // What stops it is still said, with no fix to press.
    assert!(says(
        &harness,
        "Unknown variable $(FILE_NAM). Did you mean $(FILE_NAME)?"
    ));
    assert!(harness.query_by_label("Use $(FILE_NAME)").is_none());
    // Nor does the fix change the path when asked for.
    let row = path_row();
    let words = Translator::new(Locale::EnGb);
    UtilityTool::fix(harness.state_mut().export(), row, &words);
    assert_eq!(held(&mut harness), values);
}

/// The path's custom row on Export's page.
fn path_row() -> &'static Row {
    fn find(rows: &'static [Row]) -> Option<&'static Row> {
        rows.iter().find_map(|row| match row {
            Row::Custom {
                widget: Widget::ExportPath,
                ..
            } => Some(row),
            _ => find(row.rows()),
        })
    }
    find(definition().tool.tabs[0].rows).expect("the path's row")
}

/// What the field `name` shows.
fn shows(harness: &Harness<'_, Bench>, name: &str) -> Option<String> {
    let field = harness.get_by_role_and_label(Role::TextInput, name);
    field
        .accesskit_node()
        .value()
        .map(|value| value.to_string())
}

#[test]
fn a_new_resolution_keeps_the_pixels_of_a_side_conf_gives_and_shows_its_new_length() {
    // 1182 × 591 px at 300 dpi: 10.01 × 5.00 cm; the width `--conf` gives.
    let values = Values {
        size_mode: SizeMode::Cm,
        width: 1182,
        height: 591,
        dpi: 300,
        ..Values::default()
    };
    let mut harness = given(values);
    assert_eq!(shows(&harness, "Width, in cm").as_deref(), Some("10.01"));
    type_into(&mut harness, Role::TextInput, "Resolution, in dpi", "600");
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    let now = held(&mut harness);
    // The width keeps its pixels, the height its length.
    assert_eq!((now.dpi, now.width, now.height), (600, 1182, 1182));
    assert_eq!(shows(&harness, "Width, in cm").as_deref(), Some("5.00"));
    assert_eq!(shows(&harness, "Height, in cm").as_deref(), Some("5.00"));
}

#[test]
fn default_settings_changing_the_resolution_shows_the_lengths_at_the_new_one() {
    // The mode and the pixels `--conf` gives, the resolution not: 1182 × 591 px at
    // 600 dpi shows as 5.00 × 2.50 cm, at the default 300 as 10.01 × 5.00 cm.
    let values = Values {
        size_mode: SizeMode::Cm,
        width: 1182,
        height: 591,
        dpi: 600,
        ..Values::default()
    };
    let keys = [
        "export.size.mode",
        "export.size.width",
        "export.size.height",
    ];
    let export = home(values).given(keys.map(str::to_owned));
    let mut harness = tool_of(export, Builtin::Dark, Locale::EnGb, PANEL);
    assert_eq!(shows(&harness, "Width, in cm").as_deref(), Some("5.00"));
    harness
        .get_by_role_and_label(Role::Button, "Reset export settings")
        .click();
    settle(&mut harness);
    let now = held(&mut harness);
    assert_eq!((now.dpi, now.width, now.height), (300, 1182, 591));
    assert_eq!(shows(&harness, "Width, in cm").as_deref(), Some("10.01"));
    assert_eq!(shows(&harness, "Height, in cm").as_deref(), Some("5.00"));
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
fn with_no_photo_open_export_rests() {
    let mut harness = open();
    harness.state_mut().shown.photo = Photo::None;
    harness.run();
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    assert!(harness.state().asked.is_empty());
    // No tooltip either: the button is disabled and that is all.
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    harness.hover_at(button.rect().center());
    harness.run_steps(40);
    assert!(harness.query_by_role(Role::Tooltip).is_none());
}

#[test]
fn return_exports_nothing_of_its_own_and_leaves_the_rows_their_keys() {
    let mut harness = open();
    harness.key_press(egui::Key::Enter);
    harness.run();
    // Ctrl+Return is the tool's only while the focus is in it.
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

fn focused(harness: &Harness<'_, Bench>, role: Role, label: &str) -> bool {
    harness.get_by_role_and_label(role, label).is_focused()
}

#[test]
fn ctrl_return_exports_from_a_row_as_the_button_would() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    settle(&mut harness);
    assert!(harness.state().asked.last().is_some_and(|a| a.start));
    // The row's own Return, its list, does not open; the focus is on the button, as a
    // click leaves it.
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert!(focused(&harness, Role::Button, "Export 1 photo"));
    // A value being typed is given first, and exported.
    let mut harness = open();
    answer(&mut harness, 1, preview());
    type_into(&mut harness, Role::TextInput, "Width, in px", "2048");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    settle(&mut harness);
    let pressed = harness.state().asked.iter().find(|a| a.start).cloned();
    assert_eq!(pressed.and_then(|a| a.values).map(|v| v.width), Some(2048));
}

#[test]
fn ctrl_return_does_nothing_while_export_is_held_off() {
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
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    settle(&mut harness);
    assert!(harness.state().asked.is_empty());
}

#[test]
fn the_reveal_focuses_the_button_and_escape_gives_the_focus_back() {
    // Nothing focused before: Esc gives the focus back to the Viewer, letting it go.
    let mut harness = open();
    answer(&mut harness, 1, preview());
    harness.state_mut().reveal = true;
    harness.run();
    harness.run();
    assert!(focused(&harness, Role::Button, "Export 1 photo"));
    // Return then presses it.
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert!(harness.state().asked.last().is_some_and(|a| a.start));
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(harness.ctx.memory(|m| m.focused()).is_none());
    // A row focused before: Esc gives it the focus back, from anywhere in the tool.
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::ComboBox, "Target storage")
        .focus();
    harness.run();
    harness.state_mut().reveal = true;
    harness.run();
    harness.run();
    assert!(focused(&harness, Role::Button, "Export 1 photo"));
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(focused(&harness, Role::ComboBox, "Target storage"));
    // Given back once: Esc again lets it go, to the Viewer.
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(harness.ctx.memory(|m| m.focused()).is_none());
}

#[test]
fn escape_on_a_row_never_revealed_gives_the_focus_to_the_viewer() {
    let mut harness = open();
    harness
        .get_by_role_and_label(Role::ComboBox, "On conflict")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(harness.ctx.memory(|m| m.focused()).is_none());
    assert!(harness.state().open && harness.state().toggled == 0);
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

// The groups, the path and the header.

/// Sets the photo and jobs the app tells the tool.
fn tell(harness: &mut Harness<'_, Bench>, photo: Photo, jobs: Jobs) {
    harness.state_mut().shown.photo = photo;
    harness.state_mut().shown.jobs = jobs;
    harness.run();
}

fn ready() -> Photo {
    Photo::Ready {
        name: "IMG_2041.ARW".to_owned(),
    }
}

/// Whether the group or section row named `title` shows as expanded to screen readers.
fn expanded(harness: &Harness<'_, Bench>, title: &str) -> bool {
    let node = harness.get_by_role_and_label(Role::Button, title);
    node.accesskit_node().data().is_expanded() == Some(true)
}

/// Storage options is the tool's first row, so it draws no space above it (decision 100):
/// it starts the 4 points under the header every tool's first row starts, a slider's
/// included (`crates/ui/tests/sections.rs`), not 12.
#[test]
fn the_gap_under_the_header_is_every_tool_s() {
    let harness = open();
    let header = harness.get_by_role_and_label(Role::Button, "Export").rect();
    let storage = harness
        .get_by_role_and_label(Role::Group, "Storage options")
        .rect();
    let t = Theme::builtin(Builtin::Dark);
    // The bench keeps the theme's item spacing, 4, after the header, which the Tools panel
    // sets to 0; past it, the group starts where the body's 4-point margin ends.
    let item_spacing = t.spacing.xs;
    assert_eq!(storage.top() - header.bottom(), item_spacing + t.spacing.xs);
}

#[test]
fn storage_and_format_options_are_plain_groups_and_global_options_folds() {
    let mut harness = open();
    // The two plain groups are groups named by their titles, never buttons, and their rows,
    // inside them, are always there (decision 100).
    for (title, row) in [
        ("Storage options", "On conflict"),
        ("Format options", "File format"),
    ] {
        let group = harness.get_by_role_and_label(Role::Group, title);
        group.get_by_role_and_label(Role::ComboBox, row);
        assert!(
            harness
                .query_by_role_and_label(Role::Button, title)
                .is_none(),
            "{title} has no fold"
        );
    }
    // Global options folds and starts open.
    assert!(expanded(&harness, "Global options"));
    harness.get_by_role_and_label(Role::ComboBox, "Set size");
    harness
        .get_by_role_and_label(Role::Button, "Global options")
        .click();
    settle(&mut harness);
    // Kept as `global` under `export`, as every tool's group is.
    let kept = harness.state().settings.edit.groups.get(ID);
    let kept = kept.and_then(|groups| groups.get("global")).copied();
    assert_eq!(kept, Some(false));
    assert!(!expanded(&harness, "Global options"));
    assert!(harness.query_by_label("Set size").is_none());
    assert!(harness.query_by_label("Allow upscaling").is_none());
    // The plain groups' rows stay.
    harness.get_by_role_and_label(Role::ComboBox, "On conflict");
    harness
        .get_by_role_and_label(Role::Button, "Global options")
        .click();
    settle(&mut harness);
    harness.get_by_role_and_label(Role::ComboBox, "Set size");
}

#[test]
fn a_closed_global_options_says_how_many_of_its_settings_are_off_their_defaults() {
    let mut harness = open();
    let group = |harness: &Harness<'_, Bench>| {
        harness
            .get_by_role_and_label(Role::Button, "Global options")
            .accesskit_node()
            .description()
    };
    assert_eq!(group(&harness), None, "nothing set");
    choose(&mut harness, "Set size", "In cm (for print)");
    harness.get_by_label("Allow upscaling");
    choose(&mut harness, "Allow upscaling", "Yes");
    harness
        .get_by_role_and_label(Role::Button, "Global options")
        .click();
    settle(&mut harness);
    assert_eq!(group(&harness).as_deref(), Some("2 set"));
    // Set size and Allow upscaling, two setting rows; the size line's own count none.
    let values = held(&mut harness);
    assert_eq!((values.size_mode, values.upscale), (SizeMode::Cm, true));
    assert_eq!(values.set_in_size(), 0);
    // A size typed is the size line's, counted with them.
    let sized = Values {
        width: 2048,
        dpi: 240,
        ..Values::default()
    };
    assert_eq!(sized.set_in_size(), 2);
}

#[test]
fn metadata_and_multi_preset_export_are_closed_and_disabled_groups() {
    let harness = open();
    for group in ["Metadata", "Multi-preset export"] {
        assert!(disabled(&harness, Role::Button, group), "{group}");
        assert!(!expanded(&harness, group), "{group}");
    }
}

#[test]
fn the_path_is_one_line_with_no_visible_label_and_the_folder_button_at_its_end() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    // Named "Path" to screen readers, with no label drawn: the only node of that name is
    // the field.
    assert_eq!(harness.query_all_by_label("Path").count(), 1);
    let field = harness
        .get_by_role_and_label(Role::TextInput, "Path")
        .rect();
    let folder = harness.get_by_label("Choose folder").rect();
    assert!(folder.left() >= field.right(), "{folder:?} {field:?}");
    assert!((folder.center().y - field.center().y).abs() < 4.0);
    // The resolved file name is its description, not a note.
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/apertrust_exported/IMG_2041.jpg"
    ));
    assert!(
        harness
            .query_by_label("IMG_2041.ARW → ~/Pictures/apertrust_exported/IMG_2041.jpg")
            .is_none()
    );
    // At every width it stays on one line.
    for width in [NARROWEST, PANEL, WIDEST] {
        let mut harness = tool(Values::default(), Builtin::Dark, Locale::EnGb, width);
        answer(&mut harness, 1, preview());
        let field = harness
            .get_by_role_and_label(Role::TextInput, "Path")
            .rect();
        assert!(field.height() < 26.0, "{width}: {field:?}");
    }
}

/// The tool over `path`, the disk holding `folders`, and the person choosing `answer` in
/// the folder button's dialog, None to cancel.
fn choosing(path: &str, folders: &[&str], answer: Option<&str>) -> Harness<'static, Bench> {
    let values = Values {
        path: path.to_owned(),
        ..Values::default()
    };
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
    let picker = &mut harness.state_mut().picker;
    picker.folders = folders.iter().map(PathBuf::from).collect();
    picker.answer = Some(answer.map(PathBuf::from));
    harness
}

/// Clicks the folder button, and returns the dialog it asked for and where it opened.
fn choose_folder(harness: &mut Harness<'_, Bench>) -> (crate::dialogs::Pick, Option<PathBuf>) {
    harness
        .get_by_role_and_label(Role::Button, "Choose folder")
        .click();
    settle(harness);
    let asked = &harness.state().picker.asked;
    assert_eq!(asked.len(), 1, "one dialog");
    asked[0].clone()
}

#[test]
fn the_folder_button_puts_the_folder_chosen_before_the_pattern_s_file_name() {
    // darktable's example: the pattern's folder has variables, so the dialog opens at home.
    let mut harness = choosing(
        "$(FILE_FOLDER)/darktable_exported/$(FILE_NAME)",
        &["/Users/anna"],
        Some("/Volumes/Photos/Out"),
    );
    let (pick, start) = choose_folder(&mut harness);
    assert_eq!(
        pick,
        crate::dialogs::Pick::Destination("$(FILE_NAME)".to_owned())
    );
    assert_eq!(start, Some(PathBuf::from("/Users/anna")));
    assert_eq!(held(&mut harness).path, "/Volumes/Photos/Out/$(FILE_NAME)");
    // Through the path's value: the app keeps it and asks for its preview, and the
    // Summary's Destination follows at once.
    let kept = harness.state().asked.last().cloned().expect("kept");
    assert_eq!(
        kept.values.map(|v| v.path).as_deref(),
        Some("/Volumes/Photos/Out/$(FILE_NAME)")
    );
    assert!(summary_says(&harness, "Destination /Volumes/Photos/Out"));
}

#[test]
fn the_folder_button_keeps_a_file_part_of_the_person_s_own_and_writes_home_as_tilde() {
    let mut harness = choosing(
        "/Users/anna/Pictures/$(FILE_NAME)_web",
        &["/Users/anna", "/Users/anna/Pictures"],
        Some("/Users/anna/Desktop"),
    );
    let (pick, start) = choose_folder(&mut harness);
    assert_eq!(
        pick,
        crate::dialogs::Pick::Destination("$(FILE_NAME)_web".to_owned())
    );
    // It opens at the path's folder.
    assert_eq!(start, Some(PathBuf::from("/Users/anna/Pictures")));
    assert_eq!(held(&mut harness).path, "~/Desktop/$(FILE_NAME)_web");
    assert!(summary_says(&harness, "Destination ~/Desktop"));
}

#[test]
fn a_path_that_is_a_folder_alone_takes_the_file_name_after_the_folder_chosen() {
    let mut harness = choosing(
        "~/Pictures",
        &["/Users/anna", "/Users/anna/Pictures"],
        Some("/Volumes/Photos/Out"),
    );
    let (pick, start) = choose_folder(&mut harness);
    assert_eq!(
        pick,
        crate::dialogs::Pick::Destination("$(FILE_NAME)".to_owned())
    );
    // At the path itself, `~` expanded.
    assert_eq!(start, Some(PathBuf::from("/Users/anna/Pictures")));
    assert_eq!(held(&mut harness).path, "/Volumes/Photos/Out/$(FILE_NAME)");
}

#[test]
fn a_cancelled_folder_dialog_changes_nothing() {
    let path = "$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)";
    let mut harness = choosing(path, &["/Users/anna"], None);
    choose_folder(&mut harness);
    assert_eq!(held(&mut harness).path, path);
    assert!(harness.state().asked.is_empty());
    assert!(summary_says(
        &harness,
        "Destination $(FILE_FOLDER)/apertrust_exported"
    ));
}

#[test]
fn return_or_space_on_the_focused_folder_button_opens_the_dialog() {
    for key in [egui::Key::Enter, egui::Key::Space] {
        let mut harness = choosing("$(FILE_NAME)", &[], Some("/Volumes/Out"));
        harness
            .get_by_role_and_label(Role::Button, "Choose folder")
            .focus();
        harness.run();
        harness.key_press(key);
        settle(&mut harness);
        assert_eq!(harness.state().picker.asked.len(), 1, "{key:?}");
        assert_eq!(
            held(&mut harness).path,
            "/Volumes/Out/$(FILE_NAME)",
            "{key:?}"
        );
    }
}

#[test]
fn the_folder_button_is_disabled_while_conf_gives_the_path() {
    let mut harness = given(Values {
        path: "/Users/anna/out/$(FILE_NAME)".to_owned(),
        ..Values::default()
    });
    harness.state_mut().picker.answer = Some(Some(PathBuf::from("/Volumes/Out")));
    assert!(disabled(&harness, Role::Button, "Choose folder"));
    assert!(disabled(&harness, Role::TextInput, "Path"));
    harness
        .get_by_role_and_label(Role::Button, "Choose folder")
        .click();
    settle(&mut harness);
    assert!(harness.state().picker.asked.is_empty());
    assert_eq!(held(&mut harness).path, "/Users/anna/out/$(FILE_NAME)");
    // Nor does an answer that comes all the same change it.
    assert!(
        !harness
            .state_mut()
            .export()
            .choose_folder(Path::new("/Volumes/Out"), "IMG")
    );
}

#[test]
fn a_name_only_a_decode_gives_is_never_shown_as_exact() {
    let mut harness = open();
    let mut p = preview();
    p.names[0].exact = false;
    answer(&mut harness, 1, p);
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → $(FILE_FOLDER)/apertrust_exported/$(FILE_NAME), filled in as it is exported"
    ));
    assert!(!tip_says(&harness, "IMG_2041.jpg"));
}

#[test]
fn with_several_photos_a_name_only_a_decode_gives_stays_the_pattern() {
    // The first photo named is another selected one, not the open one: its name needs a
    // decode, so its guess is never shown, and the others are summed up.
    let mut harness = open();
    let mut p = preview();
    let open_one = p.names[0].clone();
    p.names[0] = PhotoName {
        source: PathBuf::from("/Users/anna/Pictures/IMG_2040.ARW"),
        exact: false,
        fate: Fate::File {
            file: PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2040.jpg"),
            exists: false,
            action: Action::Write,
        },
    };
    p.names.push(open_one);
    answer(&mut harness, 1, p);
    assert!(tip_says(
        &harness,
        "IMG_2040.ARW → $(FILE_FOLDER)/apertrust_exported/$(FILE_NAME), filled in as it is exported"
    ));
    assert!(tip_says(&harness, "and 1 more, the same way"));
    assert!(!tip_says(&harness, "IMG_2040.jpg"));
    // The Summary's file name is the pattern's, as the tooltip has it.
    assert!(summary_says(&harness, "File name $(FILE_NAME).jpg"));
    // The open photo first, its name exact, the next one's guess never in view.
    let mut harness = open();
    let mut p = preview();
    let mut guessed = p.names[0].clone();
    guessed.source = PathBuf::from("/Users/anna/Pictures/IMG_2042.ARW");
    guessed.exact = false;
    guessed.fate = Fate::File {
        file: PathBuf::from("/Users/anna/Pictures/apertrust_exported/IMG_2042.jpg"),
        exists: false,
        action: Action::Write,
    };
    p.names.push(guessed);
    answer(&mut harness, 1, p);
    assert!(tip_says(
        &harness,
        "IMG_2041.ARW → ~/Pictures/apertrust_exported/IMG_2041.jpg"
    ));
    assert!(!tip_says(&harness, "IMG_2042"));
    assert!(summary_says(&harness, "File name IMG_2041.jpg"));
}

#[test]
fn no_explanation_is_a_line_inside_the_tool_it_is_a_tooltip() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    // The note about metadata is the File format row's tooltip, after its words.
    let format = harness.get_by_role_and_label(Role::ComboBox, "File format");
    assert_eq!(
        format.accesskit_node().description().as_deref(),
        Some("This version writes no EXIF, location or copyright into exported files.")
    );
    assert!(harness.query_by_label_contains("writes no EXIF").is_none());
    // The size's readout is the Set size row's.
    assert!(size_tip(&harness).contains("will be 6000 × 4000 px"));
    assert!(harness.query_by_label_contains("will be").is_none());
    // The scale's rule is the row's, in the same tooltip.
    choose(&mut harness, "Set size", "By scale (for file)");
    assert!(size_tip(&harness).contains("a simple fraction, such as 1/2"));
    assert!(harness.query_by_label_contains("simple fraction").is_none());
    // What On conflict does with files already there is no note: the warnings and the
    // summary say it. A problem that stops Export keeps its line, in `error`.
    let mut there = preview_with_file_there(Action::Overwrite);
    there.problem = Some(PathProblem::Empty);
    answer(&mut harness, 2, there);
    assert!(says(&harness, "Enter where to put the exported photos."));
}

#[test]
fn the_header_has_its_reset_a_presets_button_drawn_disabled_and_its_options() {
    let mut harness = open();
    harness.get_by_role_and_label(Role::Button, "Reset export settings");
    assert!(disabled(&harness, Role::Button, "Export presets"));
    assert!(disabled(&harness, Role::Button, "Export options"));
    // In that order, from the start: reset, presets, options.
    let at = |label: &str| {
        harness
            .get_by_role_and_label(Role::Button, label)
            .rect()
            .center()
            .x
    };
    assert!(at("Reset export settings") < at("Export presets"));
    assert!(at("Export presets") < at("Export options"));
    // Pointing at the presets button says why it is off.
    let presets = harness.get_by_role_and_label(Role::Button, "Export presets");
    harness.hover_at(presets.rect().center());
    harness.run_steps(40);
    assert!(
        harness
            .query_by_label("Presets come in a later version")
            .is_some()
    );
}

// The Summary, the warnings, the button and the line under it.

/// What the Summary's rows say, as screen readers read them: each label with its full value.
fn summary_says(harness: &Harness<'_, Bench>, row: &str) -> bool {
    harness.query_by_label(row).is_some()
}

#[test]
fn the_summary_is_a_group_open_at_first_with_a_labelled_list_after_the_groups_to_come() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    // A disclosure like Global options, open at first, its list after Multi-preset export.
    assert!(expanded(&harness, "Summary"));
    let at = |harness: &Harness<'_, Bench>, label: &str| {
        harness
            .get_by_role_and_label(Role::Button, label)
            .rect()
            .top()
    };
    assert!(at(&harness, "Multi-preset export") < at(&harness, "Summary"));
    for row in [
        "Preset Default settings",
        "File name IMG_2041.jpg",
        "Size 6000 × 4000 px",
        "Profile Image settings",
        "Format JPEG, quality 95",
        "Destination ~/Pictures/apertrust_exported",
    ] {
        assert!(summary_says(&harness, row), "{row}");
    }
    // Only Set size's Scale has a Scale row; and no line is a one-line form or an arrow.
    assert!(harness.query_by_label_contains("Scale").is_none());
    assert!(harness.query_by_label_contains("→").is_none());
    // The warnings and the button are outside it, shown with it closed too; its state is kept.
    harness
        .get_by_role_and_label(Role::Button, "Summary")
        .click();
    settle(&mut harness);
    assert!(!harness.state().shown.summary);
    assert!(!expanded(&harness, "Summary"));
    assert!(harness.query_by_label_contains("Preset ").is_none());
    harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    harness
        .get_by_role_and_label(Role::Button, "Summary")
        .click();
    settle(&mut harness);
    assert!(harness.state().shown.summary);
    assert!(summary_says(&harness, "Preset Default settings"));
}

/// A preview of a photo whose output is `width` × `height`.
fn sized(width: u32, height: u32) -> ExportPreview {
    let mut p = preview();
    p.size = Some(ActiveSize {
        source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
        size: Ok(OutputSize {
            extent: [6000, 4000],
            width,
            height,
            scale: 1.0,
        }),
    });
    p
}

#[test]
fn the_summary_says_the_preset_or_custom_the_size_in_pixels_or_print_and_the_scale() {
    let mut harness = open();
    answer(&mut harness, 1, sized(2048, 1365));
    assert!(summary_says(&harness, "Size 2048 × 1365 px"));
    choose(&mut harness, "File format", "PNG");
    assert!(summary_says(&harness, "Preset Custom"));
    assert!(summary_says(&harness, "Format PNG, 8 bit"));
    // The name the session gave for JPEG has PNG's extension at once, before its new answer.
    assert!(summary_says(&harness, "File name IMG_2041.png"));
    assert!(!summary_says(&harness, "File name IMG_2041.jpg"));
    // In cm, the physical size and the pixels it is: the language's numbers, two decimals.
    let print = |mode| Values {
        size_mode: mode,
        width: 2362,
        height: 1574,
        ..Values::default()
    };
    let mut harness = tool(print(SizeMode::Cm), Builtin::Dark, Locale::EnGb, PANEL);
    answer(&mut harness, 1, sized(2362, 1574));
    assert!(summary_says(
        &harness,
        "Size 20.00 × 13.33 cm (2362 × 1574 px)"
    ));
    assert!(harness.query_by_label_contains("Scale").is_none());
    let mut harness = tool(print(SizeMode::Inch), Builtin::Dark, Locale::EnGb, PANEL);
    answer(&mut harness, 1, sized(2400, 1500));
    assert!(summary_says(
        &harness,
        "Size 8.00 × 5.00 in (2400 × 1500 px)"
    ));
    // Ukrainian writes the comma.
    let mut harness = tool(print(SizeMode::Cm), Builtin::Dark, Locale::Uk, PANEL);
    answer(&mut harness, 1, sized(2362, 1574));
    assert!(summary_says(
        &harness,
        "Розмір 20,00 × 13,33 см (2362 × 1574 px)"
    ));
    // By scale, a Scale row follows Size.
    let scaled = Values {
        size_mode: SizeMode::Scale,
        scale: "1/2".to_owned(),
        ..Values::default()
    };
    let mut harness = tool(scaled, Builtin::Dark, Locale::EnGb, PANEL);
    answer(&mut harness, 1, sized(3000, 2000));
    assert!(summary_says(&harness, "Size 3000 × 2000 px"));
    assert!(summary_says(&harness, "Scale 50%"));
    let at = |label: &str| harness.get_by_label_contains(label).rect().top();
    assert!(at("Size ") < at("Scale ") && at("Scale ") < at("Profile "));
    // The profile is the Profile row's value.
    choose(&mut harness, "Profile", "sRGB (web-safe)");
    assert!(summary_says(&harness, "Profile sRGB (web-safe)"));
}

#[test]
fn the_summary_has_no_value_until_the_session_answers_and_guesses_a_name_as_the_path_says() {
    let mut harness = open();
    assert!(summary_says(&harness, "Size —"));
    assert!(summary_says(&harness, "File name $(FILE_NAME).jpg"));
    let mut guessed = preview();
    guessed.names[0].exact = false;
    answer(&mut harness, 1, guessed);
    assert!(summary_says(&harness, "File name $(FILE_NAME).jpg"));
    answer(&mut harness, 2, preview());
    assert!(summary_says(&harness, "File name IMG_2041.jpg"));
    // A problem with the size leaves none.
    let mut over = preview();
    over.size = Some(ActiveSize {
        source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
        size: Err(SizeProblem::TooLarge {
            format: "JPEG",
            largest: 65535,
            width: 70000,
            height: 50000,
        }),
    });
    answer(&mut harness, 3, over);
    assert!(summary_says(&harness, "Size —"));
}

/// The Summary's title in `locale`.
fn summary_title(locale: Locale) -> String {
    apertrust_ui::Translator::new(locale).text("export-summary-title")
}

/// The texts a frame painted, with where.
fn painted(harness: &Harness<'_, Bench>) -> Vec<(String, egui::Rect)> {
    crate::dialog_host::painted(harness.output())
}

#[test]
fn a_value_too_long_for_its_row_is_cut_at_its_end_and_whole_in_its_tooltip() {
    let long = "A_very_long_file_name_that_goes_on_and_on_IMG_2041.jpg";
    let named = || {
        let mut p = preview();
        p.names[0].fate = Fate::File {
            file: PathBuf::from(format!("/Users/anna/Pictures/apertrust_exported/{long}")),
            exists: false,
            action: Action::Write,
        };
        p
    };
    for width in [NARROWEST, PANEL] {
        let mut harness = tool(Values::default(), Builtin::Dark, Locale::EnGb, width);
        answer(&mut harness, 1, named());
        // Read whole, painted cut at its end with …, inside the panel.
        assert!(
            summary_says(&harness, &format!("File name {long}")),
            "{width}"
        );
        let cut = painted(&harness)
            .into_iter()
            .find(|(text, _)| text.starts_with("A_very_long") && text.ends_with('…'))
            .unwrap_or_else(|| panic!("{width}: the name is cut at its end"));
        assert!(cut.1.right() <= width, "{width}: {cut:?}");
        // Pointing at the row shows the whole value.
        let row = harness.get_by_label(&format!("File name {long}")).rect();
        harness.hover_at(row.center());
        harness.run_steps(40);
        assert!(
            harness.query_by_label(long).is_some(),
            "{width}: the tooltip"
        );
    }
    // A destination is cut in its middle: its end, the folder's name, stays.
    let deep = PathBuf::from(
        "/Users/anna/Pictures/2026-10 Wedding in the countryside/Selects/apertrust_exported/IMG_2041.jpg",
    );
    let mut p = preview();
    p.names[0].fate = Fate::File {
        file: deep,
        exists: false,
        action: Action::Write,
    };
    let mut harness = tool(Values::default(), Builtin::Dark, Locale::EnGb, NARROWEST);
    answer(&mut harness, 1, p);
    let cut = painted(&harness)
        .into_iter()
        .find(|(text, _)| text.starts_with("~/") && text.contains('…'))
        .expect("the destination is cut");
    assert!(cut.0.ends_with("exported"), "{}", cut.0);
}

#[test]
fn nothing_the_summary_paints_runs_past_the_panel_in_any_language() {
    for locale in [Locale::EnGb, Locale::Uk, Locale::Pl, Locale::Pseudo] {
        for width in [NARROWEST, PANEL, WIDEST] {
            let print = Values {
                size_mode: SizeMode::Cm,
                width: 2362,
                height: 1574,
                ..Values::default()
            };
            let mut harness = tool(print, Builtin::Dark, locale, width);
            answer(&mut harness, 1, sized(2362, 1574));
            // What the Summary paints, from its header down.
            let top = harness
                .get_by_role_and_label(Role::Button, &summary_title(locale))
                .rect()
                .top();
            for (text, at) in painted(&harness) {
                if at.top() < top {
                    continue;
                }
                assert!(
                    at.right() <= width + 0.5,
                    "{locale:?} {width}: {text:?} ends at {}",
                    at.right()
                );
            }
        }
    }
}

#[test]
fn a_path_that_gives_no_file_name_for_any_photo_holds_the_button_off_saying_so() {
    let mut harness = open();
    let mut nameless = preview();
    nameless.names[0].fate = Fate::NoName;
    answer(&mut harness, 1, nameless);
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("The path gives no file name for this photo.")
    );
    // A name again, and the button is live.
    answer(&mut harness, 2, preview());
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn with_nothing_to_export_the_button_is_simply_disabled() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    tell(&mut harness, Photo::None, Jobs::default());
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    // No reason is given: no description, and no stop for the keyboard.
    assert_eq!(button.accesskit_node().description(), None);
    button.focus();
    harness.run();
    assert!(
        !harness
            .get_by_role_and_label(Role::Button, "Export 1 photo")
            .is_focused()
    );
    // Nothing to sum up or warn of: the end is the button alone.
    assert!(harness.query_by_label_contains(" · ").is_none());
    assert!(
        harness
            .query_by_label_contains("Nothing to export")
            .is_none()
    );
    assert!(harness.query_by_label_contains("view-only").is_none());
    // A press neither exports nor asks.
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    assert!(harness.state().asked.is_empty());
}

#[test]
fn a_view_only_photo_is_left_out_with_a_warning_and_a_button_that_says_why() {
    let mut harness = open();
    let newer = Photo::ViewOnly {
        name: "IMG_2041.ARW".to_owned(),
        why: Why::Newer,
    };
    tell(&mut harness, newer, Jobs::default());
    assert!(says(&harness, "1 view-only photo left out"));
    // Nothing to sum up: no Summary, the button disabled and saying why.
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Summary")
            .is_none()
    );
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("A newer Apertrust wrote these edits.")
    );
    // Or its history cannot be read, naming the edit's file.
    let unreadable = Photo::ViewOnly {
        name: "IMG_2041.ARW".to_owned(),
        why: Why::Unreadable {
            file: "IMG_2041.ARW.apxmp".to_owned(),
        },
    };
    tell(&mut harness, unreadable, Jobs::default());
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("The history in IMG_2041.ARW.apxmp can’t be read.")
    );
    // Back to a photo that exports: no warning, an enabled button.
    tell(&mut harness, ready(), Jobs::default());
    assert!(!says(&harness, "1 view-only photo left out"));
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
}

/// With several selected the button counts every photo it exports, the photo open among
/// them, and the warning every one left out; the photo open being view-only holds nothing
/// off while others export.
#[test]
fn several_selected_photos_are_counted_on_the_button_and_in_the_warning() {
    let mut harness = open();
    harness.state_mut().shown.others = Others {
        taken: 11,
        ..Others::default()
    };
    tell(&mut harness, ready(), Jobs::default());
    assert!(!disabled(&harness, Role::Button, "Export 12 photos"));
    // A selection command not yet told: the photos to export unknown, Export waits.
    harness.state_mut().shown.others = Others {
        pending: true,
        ..Others::default()
    };
    tell(&mut harness, ready(), Jobs::default());
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    harness.state_mut().shown.others = Others {
        taken: 11,
        ..Others::default()
    };
    tell(&mut harness, ready(), Jobs::default());
    let newer = Photo::ViewOnly {
        name: "IMG_2041.ARW".to_owned(),
        why: Why::Newer,
    };
    harness.state_mut().shown.others = Others {
        taken: 10,
        left_out: 1,
        ..Others::default()
    };
    tell(&mut harness, newer, Jobs::default());
    assert!(says(&harness, "2 view-only photos left out"));
    assert!(!disabled(&harness, Role::Button, "Export 10 photos"));
    // The photo open's size problem holds nothing off where the export leaves it out.
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
    assert!(!disabled(&harness, Role::Button, "Export 10 photos"));
    // Nor does its folder's check, though the path's own problem, every photo's, does.
    let mut p = preview();
    p.folders = vec![FolderCheck {
        folder: PathBuf::from("/Volumes/Photos NAS/Exports"),
        preflight: Preflight::Blocked {
            at: PathBuf::from("/Volumes/Photos NAS"),
            why: Blocked::NotConnected,
        },
    }];
    answer(&mut harness, 2, p);
    assert!(!disabled(&harness, Role::Button, "Export 10 photos"));
    let mut p = preview();
    p.problem = Some(PathProblem::Empty);
    answer(&mut harness, 3, p);
    assert!(disabled(&harness, Role::Button, "Export 10 photos"));
    // The photo open ready but passed over, the listing lacking it: its problem holds
    // nothing off either.
    harness.state_mut().shown.others = Others {
        taken: 10,
        skips_open: true,
        ..Others::default()
    };
    tell(&mut harness, ready(), Jobs::default());
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
    answer(&mut harness, 4, p);
    assert!(!disabled(&harness, Role::Button, "Export 10 photos"));
    // Nothing else to export: no photo goes, and the button is disabled.
    harness.state_mut().shown.others = Others {
        skips_open: true,
        ..Others::default()
    };
    tell(&mut harness, ready(), Jobs::default());
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
    // Back to that photo alone, which the export takes: the words as before, and a
    // problem holds Export off.
    harness.state_mut().shown.others = Others::default();
    tell(&mut harness, ready(), Jobs::default());
    assert!(disabled(&harness, Role::Button, "Export 1 photo"));
}

#[test]
fn a_problem_that_stops_export_is_the_button_s_reason_and_description() {
    let mut harness = open();
    let mut p = preview();
    p.problem = Some(PathProblem::Empty);
    answer(&mut harness, 1, p);
    let button = harness.get_by_role_and_label(Role::Button, "Export 1 photo");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("Fix the path first: Enter where to put the exported photos.")
    );
}

#[test]
fn the_files_an_export_would_replace_are_a_warning_and_a_lower_bound_where_names_are_guessed() {
    let values = Values {
        on_conflict: OnConflict::Overwrite,
        ..Values::default()
    };
    let mut harness = tool(values.clone(), Builtin::Dark, Locale::EnGb, PANEL);
    answer(&mut harness, 1, preview_with_file_there(Action::Overwrite));
    assert!(says(&harness, "Replaces 1 file already there"));
    // A second photo whose name is a guess is left out of the count, so it is a lower bound.
    let mut guessed = preview_with_file_there(Action::Overwrite);
    let mut second = guessed.names[0].clone();
    second.exact = false;
    guessed.names.push(second);
    let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
    answer(&mut harness, 1, guessed);
    assert!(says(&harness, "Replaces at least 1 file already there"));
    // With nothing to replace there is no warning.
    let mut harness = open();
    answer(&mut harness, 1, preview());
    assert!(harness.query_by_label_contains("Replaces").is_none());
}

#[test]
fn only_a_failed_export_has_a_line_under_the_button() {
    let mut harness = open();
    answer(&mut harness, 1, preview());
    // Idle: nothing under the button.
    assert!(harness.query_by_label("Show jobs").is_none());
    assert!(harness.query_by_label_contains("Export failed").is_none());
    // Pressing Export says nothing under it either: the jobs window shows the progress.
    harness
        .get_by_role_and_label(Role::Button, "Export 1 photo")
        .click();
    harness.run();
    assert!(harness.state().asked.iter().any(|a| a.start));
    assert!(harness.query_by_label_contains("Started").is_none());
    assert!(harness.query_by_label_contains("running").is_none());
    assert!(harness.query_by_label("Show jobs").is_none());
    // The last export failed: its line in `error`, and Show jobs, which opens the jobs window.
    let failed = Jobs {
        failed: Some((11, 12)),
    };
    tell(&mut harness, ready(), failed);
    assert!(says(&harness, "Export failed: 11 of 12"));
    assert!(!disabled(&harness, Role::Button, "Export 1 photo"));
    harness.get_by_label("Show jobs").click();
    harness.run();
    assert_eq!(harness.state().jobs_shown, 1);
    // Until the next export, or the row's dismissal, which the app tells it.
    tell(&mut harness, ready(), Jobs::default());
    assert!(harness.query_by_label("Show jobs").is_none());
    assert!(harness.query_by_label_contains("Export failed").is_none());
}

#[test]
fn a_second_press_just_after_the_first_is_ignored() {
    let mut export = home(Values::default());
    assert!(!export.too_soon(10.0), "nothing pressed yet");
    export.note_started(10.0);
    assert!(export.too_soon(10.2));
    assert!(!export.too_soon(10.6));
}

// The rows laid out to the panel's width.

/// Where the size row's fields and its group are: the fields named `names` and the group.
fn size_line_of(harness: &Harness<'_, Bench>, group: &str, names: &[&str]) -> Vec<egui::Rect> {
    let group = harness.get_by_role_and_label(Role::Group, group).rect();
    let mut rects = vec![group];
    rects.extend(
        names
            .iter()
            .map(|name| harness.get_by_role_and_label(Role::TextInput, name).rect()),
    );
    rects
}

#[test]
fn the_size_row_is_one_line_with_no_label_its_fields_sharing_the_width() {
    for width in [NARROWEST, PANEL, WIDEST] {
        // Pixels: [w] × [h] px, a group named "Size in pixels" and no label drawn.
        let mut harness = tool(Values::default(), Builtin::Dark, Locale::EnGb, width);
        answer(&mut harness, 1, preview());
        let rects = size_line_of(
            &harness,
            "Size in pixels",
            &["Width, in px", "Height, in px"],
        );
        let (group, w, h) = (rects[0], rects[1], rects[2]);
        assert!(
            painted(&harness)
                .iter()
                .all(|(text, _)| text != "Size in pixels"),
            "{width}: no label"
        );
        assert!(group.height() <= 26.0, "{width}: one line {group:?}");
        assert!((w.center().y - h.center().y).abs() < 0.5, "{width}");
        assert!(w.right() < h.left(), "{width}");
        assert!(
            (w.width() - h.width()).abs() <= 1.0,
            "{width}: they share the width"
        );
        // The unit follows them, in the row, and the fields are no wider than it.
        assert!(
            h.right() < group.right() && group.right() <= width,
            "{width}"
        );
        // Centimetres: [w] × [h] cm @ [dpi] dpi, on one line, with no row of the resolution's.
        let print = Values {
            size_mode: SizeMode::Cm,
            ..Values::default()
        };
        let mut harness = tool(print, Builtin::Dark, Locale::EnGb, width);
        answer(&mut harness, 1, preview());
        let rects = size_line_of(
            &harness,
            "Size in centimetres",
            &["Width, in cm", "Height, in cm", "Resolution, in dpi"],
        );
        let (group, w, h, dpi) = (rects[0], rects[1], rects[2], rects[3]);
        assert!(group.height() <= 26.0, "{width}: one line {group:?}");
        for field in [h, dpi] {
            assert!((w.center().y - field.center().y).abs() < 0.5, "{width}");
        }
        assert!(w.right() < h.left() && h.right() < dpi.left(), "{width}");
        assert!(
            dpi.right() < group.right() && group.right() <= width,
            "{width}"
        );
        assert!(harness.query_by_label("Resolution").is_none());
        // Inches the same, by its own name.
        let inches = Values {
            size_mode: SizeMode::Inch,
            ..Values::default()
        };
        let mut harness = tool(inches, Builtin::Dark, Locale::EnGb, width);
        answer(&mut harness, 1, preview());
        size_line_of(
            &harness,
            "Size in inches",
            &["Width, in in", "Height, in in", "Resolution, in dpi"],
        );
    }
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

/// A preview with a problem under the path.
fn with_error(harness: &mut Harness<'static, Bench>) {
    let mut p = preview();
    p.problem = Some(PathProblem::Empty);
    answer(harness, 2, p);
}

/// One snapshot test for each look: dark, light, an error under the path, Ukrainian, Polish
/// and the pseudo-locale, each at the panel's width and its widest.
macro_rules! looks {
    ($($test:ident: $which:expr, $locale:expr, $width:expr, $file:literal, $changes:expr;)*) => {
        $(
            #[test]
            fn $test() {
                photographed($which, $locale, $width, $file, $changes);
            }
        )*
    };
}

looks! {
    the_tool_at_320_in_the_dark_theme: Builtin::Dark, Locale::EnGb, PANEL, "export-tool-320-dark", |_| {};
    the_tool_at_600_in_the_dark_theme: Builtin::Dark, Locale::EnGb, WIDEST, "export-tool-600-dark", |_| {};
    the_tool_at_320_in_the_light_theme: Builtin::Light, Locale::EnGb, PANEL, "export-tool-320-light", |_| {};
    the_tool_at_600_in_the_light_theme: Builtin::Light, Locale::EnGb, WIDEST, "export-tool-600-light", |_| {};
    the_tool_at_320_with_an_error_under_its_path: Builtin::Dark, Locale::EnGb, PANEL, "export-tool-320-error", with_error;
    the_tool_at_600_with_an_error_under_its_path: Builtin::Dark, Locale::EnGb, WIDEST, "export-tool-600-error", with_error;
    the_tool_at_320_in_ukrainian: Builtin::Dark, Locale::Uk, PANEL, "export-tool-320-uk", |_| {};
    the_tool_at_600_in_ukrainian: Builtin::Dark, Locale::Uk, WIDEST, "export-tool-600-uk", |_| {};
    the_tool_at_320_in_polish: Builtin::Dark, Locale::Pl, PANEL, "export-tool-320-pl", |_| {};
    the_tool_at_600_in_polish: Builtin::Dark, Locale::Pl, WIDEST, "export-tool-600-pl", |_| {};
    the_tool_at_320_with_the_pseudo_locale: Builtin::Dark, Locale::Pseudo, PANEL, "export-tool-320-pseudo", |_| {};
    the_tool_at_600_with_the_pseudo_locale: Builtin::Dark, Locale::Pseudo, WIDEST, "export-tool-600-pseudo", |_| {};
}

/// The end of the tool in its busiest state: the summary open, a warning and a failed export.
fn busy_end(harness: &mut Harness<'static, Bench>) {
    let values = Values {
        on_conflict: OnConflict::Overwrite,
        width: 2048,
        height: 2048,
        ..Values::default()
    };
    harness.state_mut().content = home(values);
    harness.state_mut().shown.summary = true;
    // The photo at the limit's size: 2048 on its long side.
    let mut limited = preview_with_file_there(Action::Overwrite);
    limited.size = Some(ActiveSize {
        source: PathBuf::from("/Users/anna/Pictures/IMG_2041.ARW"),
        size: Ok(OutputSize {
            extent: [6000, 4000],
            width: 2048,
            height: 1365,
            scale: 0.341,
        }),
    });
    let jobs = Jobs {
        failed: Some((11, 12)),
    };
    tell(harness, ready(), jobs);
    answer(harness, 3, limited);
}

looks! {
    the_tool_s_end_with_a_failure_in_the_dark_theme: Builtin::Dark, Locale::EnGb, PANEL, "export-tool-end-dark", busy_end;
    the_tool_s_end_with_a_failure_in_the_light_theme: Builtin::Light, Locale::EnGb, PANEL, "export-tool-end-light", busy_end;
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

/// Where the path field and the folder button are, and the section's width: the sides of the
/// Export button and of the Set size row, which fill the content.
fn path_layout(harness: &Harness<'_, Bench>) -> [egui::Rect; 4] {
    let across =
        |rect: egui::Rect| egui::Rect::from_x_y_ranges(rect.x_range(), egui::Rangef::new(0.0, 0.0));
    [
        harness
            .get_by_role_and_label(Role::TextInput, "Path")
            .rect(),
        harness.get_by_label("Choose folder").rect(),
        across(
            harness
                .get_by_role_and_label(Role::Button, "Export 1 photo")
                .rect(),
        ),
        across(
            harness
                .get_by_role_and_label(Role::ComboBox, "Set size")
                .rect(),
        ),
    ]
}

#[test]
fn moving_the_caret_along_a_long_path_moves_nothing_across_the_row_or_the_panel() {
    let long = "$(FILE_FOLDER)/apertrust_exported/a_folder_with_a_very_long_name/another_folder_that_goes_on/$(FILE_NAME)_$(EXIF.ISO)_$(EXIF.EXPOSURE)";
    for width in [NARROWEST, PANEL, WIDEST] {
        let values = Values {
            path: long.to_owned(),
            ..Values::default()
        };
        let mut harness = tool(values, Builtin::Dark, Locale::EnGb, width);
        answer(&mut harness, 1, preview());
        let before = path_layout(&harness);
        // The field is the row's width less the button and the gap, whatever the text.
        assert!(before[0].right() <= before[1].left(), "{width}: {before:?}");
        assert!(before[1].right() <= width, "{width}: {before:?}");
        // The button is the field's height, a square, on the field's line.
        assert_eq!(
            (before[1].top(), before[1].bottom()),
            (before[0].top(), before[0].bottom()),
            "{width}: {before:?}"
        );
        assert_eq!(before[1].width(), before[1].height(), "{width}");
        harness
            .get_by_role_and_label(Role::TextInput, "Path")
            .focus();
        harness.run();
        assert_eq!(path_layout(&harness), before, "{width}: focused");
        // From the start to the end and back, a key at a time: the caret passes the
        // variable $(EXIF.EXPOSURE), where the list of variables opens under the row.
        let mut keys = vec![egui::Key::Home, egui::Key::End];
        keys.extend([egui::Key::ArrowLeft; 40]);
        keys.push(egui::Key::Home);
        keys.extend([egui::Key::ArrowRight; 60]);
        keys.push(egui::Key::End);
        keys.extend([egui::Key::ArrowLeft; 90]);
        let mut listed = false;
        for (step, key) in keys.into_iter().enumerate() {
            harness.key_press(key);
            harness.run_steps(2);
            listed |= harness.query_by_role(Role::ListBox).is_some();
            assert_eq!(path_layout(&harness), before, "{width}: {key:?} at {step}");
        }
        assert!(listed, "{width}: the list opened under the field");
    }
}

/// The rectangles a frame filled with `colour`.
fn filled(harness: &Harness<'_, Bench>, colour: egui::Color32) -> Vec<egui::Rect> {
    fn rects(shape: &egui::Shape, colour: egui::Color32, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Rect(rect) if rect.fill == colour => out.push(rect.rect),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| rects(s, colour, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        rects(&clipped.shape, colour, &mut out);
    }
    out
}

/// A slider of the tool: its format, how it sets its value in `Values`, its range, its
/// default and a value below that.
struct Slider {
    format: FileFormat,
    set: Setter,
    range: (u8, u8),
    default: u8,
    below: u8,
}

/// Sets one slider's value in `Values`.
type Setter = fn(&mut Values, u8);

#[test]
fn every_slider_paints_its_fill_between_the_handle_and_the_settings_default() {
    let theme = Theme::builtin(Builtin::Dark);
    let (track, fill) = (theme.colors.slider_track, theme.colors.slider_fill);
    // The row's painted fill at `value`: the one rectangle in the fill's colour, and the
    // track it runs along.
    let painted_fill = |format: FileFormat, set: Setter, value: u8| {
        let mut values = Values {
            format,
            ..Values::default()
        };
        set(&mut values, value);
        let mut harness = tool(values, Builtin::Dark, Locale::EnGb, PANEL);
        answer(&mut harness, 1, preview());
        let (lines, fills) = (filled(&harness, track), filled(&harness, fill));
        assert_eq!((lines.len(), fills.len()), (1, 1), "{format:?} {value}");
        (lines[0], fills[0])
    };
    let sliders = [
        Slider {
            format: FileFormat::Jpeg,
            set: |v, n| v.jpeg_quality = n,
            range: (5, 100),
            default: 95,
            below: 50,
        },
        Slider {
            format: FileFormat::Png,
            set: |v, n| v.png_compression = n,
            range: (0, 9),
            default: 5,
            below: 2,
        },
    ];
    for Slider {
        format,
        set,
        range: (low, high),
        default,
        below,
    } in sliders
    {
        let (line, at_default) = painted_fill(format, set, default);
        let (_, at_below) = painted_fill(format, set, below);
        let (_, at_high) = painted_fill(format, set, high);
        // At the default there is nothing to fill, and the fill sits at the default's place.
        assert_eq!(at_default.width(), 0.0, "{format:?}: empty at its default");
        let x_default = at_default.left();
        // Below it the fill runs from the handle up to the default's place, over it from
        // the default's place to the handle; never from the track's start.
        assert!(
            (at_below.right() - x_default).abs() < 0.01,
            "{format:?}: {at_below:?}"
        );
        assert!(
            (at_high.left() - x_default).abs() < 0.01,
            "{format:?}: {at_high:?}"
        );
        assert!(
            at_below.left() > line.left() + 1.0,
            "{format:?}: from the track's start"
        );
        // The handle's place is linear in the value: the high end gives the slope.
        let slope = (at_high.right() - x_default) / f32::from(high - default);
        let handle = x_default + slope * (f32::from(below) - f32::from(default));
        assert!(
            (at_below.left() - handle).abs() < 0.5,
            "{format:?}: the fill ends at {}, the handle at {handle}",
            at_below.left()
        );
        // And the default sits where its fraction of the range says, within the handle's
        // stop at each end.
        let fraction = f32::from(default - low) / f32::from(high - low);
        let along = (x_default - line.left()) / line.width();
        assert!(
            (along - fraction).abs() < 0.06,
            "{format:?}: {along} against {fraction}"
        );
    }
}

/// Every setting row of Export's page names a key the export settings declare, of the
/// row's kind and range, its default the settings' own, and binds to [`Values`] both ways
/// by id: each option a choice lists is one the settings hold under its id, or one not
/// built, drawn disabled; but Overwrite, which the settings hold and a platform that cannot
/// ask draws disabled (decision 93). The rows with no key have no setting yet and are drawn
/// disabled.
#[test]
fn every_setting_row_is_an_export_setting_of_its_kind_bound_both_ways() {
    use apertrust_base::settings::Kind;
    use apertrust_export::settings::registry::ALL;
    fn settings(rows: &'static [Row], into: &mut Vec<&'static SettingRow>) {
        for row in rows {
            if let Row::Setting(setting) = row {
                into.push(setting);
            }
            settings(row.rows(), into);
        }
    }
    let mut rows = Vec::new();
    settings(definition().tool.tabs[0].rows, &mut rows);
    let words = Translator::new(Locale::EnGb);
    let export = home(Values::default()).alerts(false);
    let mut keyed = 0;
    for row in rows {
        let Some(key) = row.key else {
            assert!(["masks", "intent", "style"].contains(&row.id), "{}", row.id);
            continue;
        };
        keyed += 1;
        let declared = ALL.iter().find(|declared| declared.id() == key);
        let declared = declared.unwrap_or_else(|| panic!("{key} is no export setting"));
        match (row.kind, declared.kind()) {
            (ControlKind::Number { min, max, .. }, Kind::Number(number)) => {
                assert_eq!((number.min, number.max), (Some(min), Some(max)), "{key}");
            }
            (ControlKind::Choice { .. }, Kind::Choice(_))
            | (ControlKind::Bool { .. }, Kind::Boolean) => {}
            (kind, declared) => panic!("{key}: {kind:?} is declared {declared:?}"),
        }
        let shown = UtilityTool::setting(&home(Values::default()), row);
        assert_eq!(shown, Some(row.default()), "{key}");
        let ControlKind::Choice { options, .. } = row.kind else {
            continue;
        };
        let built = Values::options(key);
        let disabled = UtilityTool::disabled(&export, row, &words).map(|(at, _)| at);
        for (at, option) in options.iter().enumerate() {
            let mut tool = home(Values::default());
            let chosen = ControlValue::Choice(at as u32);
            UtilityTool::set(&mut tool, key, &chosen);
            let bound = UtilityTool::setting(&tool, row) == Some(chosen);
            if bound {
                // Bound by id: the settings hold the option's own.
                let kept = tool.values().get(key);
                assert_eq!(kept, Some(Kept::Choice(stored_id(key, option))), "{key}");
                assert!(built.contains(&stored_id(key, option)), "{key}: {option}");
            }
            let later = disabled.as_ref().is_some_and(|later| later.contains(&at));
            if (key, *option) == ("export.disk.on_conflict", "overwrite") {
                // Held by the settings, and drawn disabled where no alert can ask first.
                assert!(bound && later, "{key}: {option}");
            } else {
                assert!(bound != later, "{key}: {option}");
            }
        }
    }
    assert_eq!(keyed, 10);
}

/// The options the page names apart from the settings' ids are held under the ids the
/// settings file has always written, and read back from them.
#[test]
fn renamed_options_are_held_under_the_settings_own_ids() {
    let renamed = [
        ("export.jpeg.subsample", "yuv444", "444"),
        ("export.jpeg.subsample", "yuv440", "440"),
        ("export.jpeg.subsample", "yuv422", "422"),
        ("export.jpeg.subsample", "yuv420", "420"),
        ("export.png.bit_depth", "bits_8", "8"),
        ("export.png.bit_depth", "bits_16", "16"),
    ];
    fn keyed(rows: &'static [Row], key: &str) -> Option<&'static SettingRow> {
        rows.iter().find_map(|row| match row {
            Row::Setting(found) if found.key == Some(key) => Some(found),
            _ => keyed(row.rows(), key),
        })
    }
    let rows = definition().tool.tabs[0].rows;
    let row = |key: &str| keyed(rows, key).unwrap_or_else(|| panic!("{key} has no row"));
    for (key, option, stored) in renamed {
        let row = row(key);
        let ControlKind::Choice { options, .. } = row.kind else {
            panic!("{key} is no choice");
        };
        let at = options.iter().position(|o| *o == option);
        let chosen = ControlValue::Choice(at.expect(option) as u32);
        let mut tool = home(Values::default());
        UtilityTool::set(&mut tool, key, &chosen);
        assert_eq!(
            tool.values().get(key),
            Some(Kept::Choice(stored)),
            "{option}"
        );
        // And back: the settings' id is the row's option.
        let mut values = Values::default();
        values.set(key, &Kept::Choice(stored));
        assert_eq!(UtilityTool::setting(&home(values), row), Some(chosen));
    }
}

/// One notch of the mouse wheel over the row `name` with `role`, `modifiers` held, up
/// positive.
fn wheel(
    harness: &mut Harness<'_, Bench>,
    role: Role,
    name: &str,
    notches: f32,
    with: egui::Modifiers,
) {
    harness.get_by_role_and_label(role, name).hover();
    settle(harness);
    let event = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, notches),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: with,
    };
    harness.event_modifiers(event, with);
    settle(harness);
}

/// Only the focused row takes the wheel, never one under the pointer, whether the wheel
/// scrolls the Tools panel, Option with it changing a row on the Mac, or changes a row
/// plain (decision 90).
#[test]
fn only_the_focused_row_takes_the_wheel() {
    use apertrust_ui::components::set_wheel_scrolls_panel;
    let (alt, none) = (egui::Modifiers::ALT, egui::Modifiers::NONE);
    for (scrolls, with) in [(true, alt), (false, none)] {
        let mut harness = open();
        set_wheel_scrolls_panel(&harness.ctx, scrolls);
        let before = held(&mut harness);
        wheel(&mut harness, Role::Slider, "Quality", 1.0, with);
        wheel(&mut harness, Role::ComboBox, "Allow upscaling", -1.0, with);
        assert_eq!(held(&mut harness), before, "{scrolls}");
        harness
            .get_by_role_and_label(Role::Slider, "Quality")
            .focus();
        settle(&mut harness);
        wheel(&mut harness, Role::Slider, "Quality", 1.0, with);
        assert_eq!(held(&mut harness).jpeg_quality, before.jpeg_quality + 1);
        let row = "Allow upscaling";
        harness.get_by_role_and_label(Role::ComboBox, row).focus();
        settle(&mut harness);
        wheel(&mut harness, Role::ComboBox, row, -1.0, with);
        assert!(held(&mut harness).upscale, "{scrolls}");
    }
}
