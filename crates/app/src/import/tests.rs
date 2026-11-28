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

use std::sync::Arc;

use apertrust_engine::session::library::{self, Places};
use apertrust_ui::{Builtin, Theme};
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::sheet::Content as _;

/// The listing's number in these tests: the first command the bench sends.
const LISTING: u64 = 1;

/// Anna's settings: Folder and the default folder in her Pictures.
fn settings() -> ImportSettings {
    ImportSettings {
        base_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
        default_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
        subfolder: "$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)".to_owned(),
        job_code: "Wedding".to_owned(),
        places: Places {
            home: Some(PathBuf::from("/Users/anna")),
            pictures: Some(PathBuf::from("/Users/anna/Pictures")),
            desktop: None,
            user_name: "anna".to_owned(),
        },
    }
}

/// The import's day in these tests, 15 November 2026.
fn today() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 11, 15).expect("a date")
}

/// The card EOS_DIGITAL, its DCIM as Source chooses it.
fn card() -> Source {
    Source {
        folder: PathBuf::from("/Volumes/EOS_DIGITAL/DCIM"),
        name: "EOS_DIGITAL".to_owned(),
        card: true,
    }
}

/// A raw of the card's `100CANON`, of `size` bytes, on the disk.
fn file(name: &str, size: u64) -> SourceFile {
    SourceFile {
        path: PathBuf::from("100CANON").join(name),
        size,
        modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(1_763_215_320)),
        local: true,
    }
}

/// A chunk of the listing numbered `open`.
fn listed(open: u64, files: Vec<SourceFile>, done: bool) -> LibraryEvent {
    LibraryEvent::SourceListed {
        open,
        folder: PathBuf::from("/Volumes/EOS_DIGITAL/DCIM"),
        files,
        done,
        left_out: done.then(Default::default),
    }
}

/// A clock's time on 15 November 2026, at `hour`:`minute`.
fn at(hour: u8, minute: u8) -> DateTime {
    DateTime {
        year: 2026,
        month: 11,
        day: 15,
        hour,
        minute,
        second: 0,
        millisecond: 0,
        offset_minutes: None,
    }
}

/// The capture times of the listing numbered `open`, each by a Canon EOS R5.
fn taken(open: u64, times: &[Option<DateTime>]) -> LibraryEvent {
    let shots = (times.iter())
        .map(|taken| SourceShot {
            taken: *taken,
            make: Some("Canon".to_owned()),
            model: Some("Canon EOS R5".to_owned()),
        })
        .collect();
    LibraryEvent::SourceTaken { open, shots }
}

/// A chunk of the check numbered `check` of the listing numbered `open`: `states` ending at
/// `checked` of `of`.
fn checked(
    open: u64,
    check: u64,
    states: Vec<SourceState>,
    checked: usize,
    of: usize,
) -> LibraryEvent {
    LibraryEvent::SourceChecked {
        open,
        check,
        folders: vec![PathBuf::from("/Users/anna/Pictures/Apertrust")],
        states,
        checked,
        of,
        done: checked == of,
    }
}

/// A photo imported before, its copy in the Studio's folder of the trees.
fn before(name: &str) -> SourceState {
    SourceState::ImportedBefore {
        copy: PathBuf::from("/Users/anna/Pictures/Apertrust/20261108_Studio").join(name),
    }
}

/// A list of the commands sent, numbered from 1, and what sends to it.
fn sender(sent: &mut Vec<Command>) -> impl FnMut(Command) -> u64 + '_ {
    |command| {
        sent.push(command);
        sent.len() as u64
    }
}

/// The names of the photos in the grid's order.
fn names(listing: &Listing) -> Vec<&str> {
    (0..listing.len())
        .map(|place| listing.at(place).name.as_str())
        .collect()
}

/// The places checked.
fn checked_places(listing: &Listing) -> Vec<usize> {
    listing.checks().checked().collect()
}

#[test]
fn the_listing_arrives_in_chunks_then_its_times_then_its_check() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    assert_eq!(
        sent,
        [Command::Library(LibraryCommand::ListSource {
            source: PathBuf::from("/Volumes/EOS_DIGITAL/DCIM"),
            include_subfolders: false,
            base_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
            default_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
            places: settings().places,
        })]
    );
    // By file name while no times have come.
    let first = vec![file("IMG_0003.CR3", 30), file("IMG_0001.CR3", 10)];
    assert!(listing.receive(&listed(LISTING, first, false)));
    assert_eq!(names(&listing), ["IMG_0001.CR3", "IMG_0003.CR3"]);
    assert!(listing.busy());
    assert!(listing.receive(&listed(LISTING, vec![file("IMG_0002.CR3", 20)], true)));
    assert_eq!(
        names(&listing),
        ["IMG_0001.CR3", "IMG_0002.CR3", "IMG_0003.CR3"]
    );
    // By capture time once they come, in the listing's order: a photo with none last.
    let times = [Some(at(9, 30)), Some(at(14, 2)), None];
    assert!(listing.receive(&taken(LISTING, &times)));
    assert_eq!(
        names(&listing),
        ["IMG_0003.CR3", "IMG_0001.CR3", "IMG_0002.CR3"]
    );
    // The check in chunks, nothing checked until it ends.
    let states = vec![SourceState::New, before("IMG_0001.CR3")];
    assert!(listing.receive(&checked(LISTING, 1, states, 2, 3)));
    assert!(listing.busy());
    assert!(!listing.opened());
    assert!(checked_places(&listing).is_empty());
    assert!(listing.receive(&checked(LISTING, 1, vec![SourceState::New], 3, 3)));
    assert!(!listing.busy());
    assert!(listing.opened());
    // The new ones checked, IMG_0001 imported before left out; the first checked active.
    assert_eq!(checked_places(&listing), [0, 2]);
    assert_eq!(listing.checks().active(), Some(0));
    assert!(listing.at(1).imported_before());
}

#[test]
fn checks_changed_by_hand_while_the_check_runs_are_kept_at_the_opening() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    let files = vec![file("IMG_0001.CR3", 10), file("IMG_0002.CR3", 20)];
    listing.receive(&listed(LISTING, files, true));
    listing.receive(&taken(LISTING, &[Some(at(9, 0)), Some(at(9, 1))]));
    listing.receive(&checked(LISTING, 1, vec![SourceState::New], 1, 2));
    listing.select(Select::All);
    listing.apply(CheckGesture::Toggle(1));
    listing.receive(&checked(LISTING, 1, vec![SourceState::New], 2, 2));
    assert!(listing.opened());
    assert_eq!(checked_places(&listing), [0]);
}

#[test]
fn a_second_check_after_folder_changes_replaces_the_first() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    let files = vec![file("A.CR3", 1), file("B.CR3", 2)];
    listing.receive(&listed(LISTING, files, true));
    listing.receive(&taken(LISTING, &[None, None]));
    let states = vec![before("A.CR3"), SourceState::New];
    listing.receive(&checked(LISTING, 1, states, 2, 2));
    assert_eq!(checked_places(&listing), [1]);
    // Folder changes: the listing shown checked again, the next check's alone kept.
    let mut moved = settings();
    moved.base_folder = "/Volumes/Photos NAS/Apertrust".to_owned();
    listing.recheck(&moved, &mut sender(&mut sent));
    assert_eq!(
        sent.last(),
        Some(&Command::Library(LibraryCommand::CheckSource {
            listing: LISTING,
            base_folder: "/Volumes/Photos NAS/Apertrust".to_owned(),
            default_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
            places: settings().places,
        }))
    );
    assert!(listing.busy());
    // The first check's late chunk changes nothing.
    listing.receive(&checked(
        LISTING,
        1,
        vec![before("A.CR3"), before("B.CR3")],
        2,
        2,
    ));
    assert!(listing.busy());
    assert!(!listing.at(1).imported_before());
    // The second's first chunk replaces every state; the one it has not told is unknown.
    listing.receive(&checked(LISTING, 2, vec![SourceState::New], 1, 2));
    assert!(!listing.at(0).imported_before());
    assert_eq!(listing.at(1).state, None);
    listing.receive(&checked(LISTING, 2, vec![before("B.CR3")], 2, 2));
    assert!(!listing.busy());
    assert!(listing.at(1).imported_before());
    // The checks stay as the person left them: the opening's are set once.
    assert_eq!(checked_places(&listing), [1]);
}

#[test]
fn an_older_listing_s_events_are_dropped() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    let older = LISTING;
    assert!(!listing.receive(&listed(older, vec![file("OLD.CR3", 1)], true)));
    assert!(!listing.receive(&taken(older, &[None])));
    assert!(!listing.receive(&checked(older, 1, vec![SourceState::New], 1, 1)));
    let thumbnail = LibraryEvent::Thumbnail {
        open: older,
        photo: "100CANON/OLD.CR3".to_owned(),
        size: 256,
        image: None,
    };
    assert!(!listing.receive(&thumbnail));
    assert_eq!(listing.len(), 0);
    assert!(listing.receive(&listed(2, vec![file("NEW.CR3", 1)], true)));
    assert_eq!(names(&listing), ["NEW.CR3"]);
    // Closing cancels the latest alone.
    assert_eq!(
        listing.cancel(),
        Some(Command::Library(LibraryCommand::CancelListing {
            listing: 2
        }))
    );
}

#[test]
fn with_every_photo_imported_before_none_is_checked_and_the_first_is_active() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    listing.receive(&listed(
        LISTING,
        vec![file("B.CR3", 1), file("A.CR3", 1)],
        true,
    ));
    listing.receive(&taken(LISTING, &[None, None]));
    let states = vec![
        before("B.CR3"),
        SourceState::ProbablyImported {
            candidate: PathBuf::from("/Users/anna/Pictures/Apertrust/2026/A.CR3"),
        },
    ];
    listing.receive(&checked(LISTING, 1, states, 2, 2));
    assert!(checked_places(&listing).is_empty());
    assert_eq!(listing.checks().active(), Some(0));
    assert_eq!(listing.new_count(), 0);
}

#[test]
fn the_checks_follow_their_photos_as_the_order_changes() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    listing.receive(&listed(
        LISTING,
        vec![file("A.CR3", 1), file("B.CR3", 1)],
        true,
    ));
    // Checked by hand while the times are on their way: B, the second by name.
    listing.apply(CheckGesture::Toggle(1));
    listing.apply(CheckGesture::Activate(1));
    listing.receive(&taken(LISTING, &[Some(at(12, 0)), Some(at(8, 0))]));
    assert_eq!(names(&listing), ["B.CR3", "A.CR3"]);
    assert_eq!(checked_places(&listing), [0]);
    assert_eq!(listing.checks().active(), Some(0));
}

#[test]
fn only_the_photos_in_sight_are_asked_for_once_each_a_cloud_file_never() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    let mut files: Vec<SourceFile> = (0..10)
        .map(|i| file(&format!("IMG_{i:04}.CR3"), 1))
        .collect();
    files[2].local = false;
    listing.receive(&listed(LISTING, files, true));
    let asked = |command: Option<Command>| match command {
        Some(Command::Library(LibraryCommand::SourceThumbnails {
            listing, photos, ..
        })) => {
            assert_eq!(listing, LISTING);
            photos.iter().map(|p| file_name(p)).collect::<Vec<_>>()
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(
        asked(listing.thumbnails(0..4, 256)),
        ["IMG_0000.CR3", "IMG_0001.CR3", "IMG_0003.CR3"]
    );
    // Still in sight and unanswered: not asked again.
    assert_eq!(listing.thumbnails(0..4, 256), None);
    // One answered with no picture keeps its box and is not asked again.
    let words = listing.generation();
    listing.receive(&LibraryEvent::Thumbnail {
        open: LISTING,
        photo: "100CANON/IMG_0000.CR3".to_owned(),
        size: 256,
        image: None,
    });
    assert_eq!(listing.at(0).thumbnail, Some(None));
    // A picture changes no cell's words.
    assert_eq!(listing.generation(), words);
    assert_eq!(listing.came(), 1);
    assert_eq!(
        asked(listing.thumbnails(4..6, 256)),
        ["IMG_0004.CR3", "IMG_0005.CR3"]
    );
    // Back in sight unanswered: asked again, the one answered never.
    assert_eq!(asked(listing.thumbnails(0..2, 256)), ["IMG_0001.CR3"]);
}

#[test]
fn a_cell_says_its_times_size_camera_and_where_it_was_imported_before() {
    let translator = Translator::new(Locale::EnGb);
    let photo = Photo {
        file: file("IMG_0042.CR3", 25_300_000),
        name: "IMG_0042.CR3".to_owned(),
        shot: Some(SourceShot {
            taken: Some(at(14, 2)),
            make: Some("Canon".to_owned()),
            model: Some("Canon EOS R5".to_owned()),
        }),
        state: Some(before("20261108_0042.CR3")),
        thumbnail: None,
    };
    let folders = [PathBuf::from("/Users/anna/Pictures/Apertrust")];
    let home = Path::new("/Users/anna");
    let words = cell_words(&photo, &translator, &folders, Some(home));
    let tip: Vec<&str> = words.tip.lines().collect();
    assert_eq!(tip[0], "IMG_0042.CR3");
    assert_eq!(tip[1], "Taken 15 Nov 2026, 14:02:00");
    assert!(tip[2].starts_with("Modified "), "{tip:?}");
    assert!(tip[2].ends_with("; this may differ from when it was taken"));
    assert_eq!(tip[3], "25 MB");
    assert_eq!(tip[4], "Canon EOS R5");
    assert_eq!(tip[5], "Imported before: 20261108_Studio/20261108_0042.CR3");
    assert_eq!(words.mark.as_deref(), Some(tip[5]));
    assert_eq!(words.check_name, "Import IMG_0042.CR3");
    assert_eq!(words.said, "Taken 15 Nov 2026, 14:02:00, imported before");
    // A copy outside the trees is shown whole, home as ~.
    let elsewhere = shown_in(Path::new("/Users/anna/Desktop/x.CR3"), &folders, Some(home));
    assert_eq!(elsewhere, "~/Desktop/x.CR3");
    assert_eq!(
        camera(&SourceShot {
            taken: None,
            make: Some("FUJIFILM".to_owned()),
            model: Some("X-T5".to_owned()),
        })
        .as_deref(),
        Some("FUJIFILM X-T5")
    );
}

// The sheet drawn.

struct Bench {
    sheet: ImportSheet,
    sent: Vec<Command>,
    closed: bool,
    /// The settings the shell keeps, its tool's groups among them, as the sheet writes them.
    settings: crate::settings::Settings,
    /// What the sheet wrote and asked for, in order.
    written: Vec<(&'static str, String)>,
    choose_folder: bool,
    folder_start: Option<PathBuf>,
    /// The column's width.
    column: f32,
}

/// The Import tool over Anna's settings, on 15 November 2026.
fn anna_s_tool() -> tool::Tool {
    tool::Tool::new(settings()).today(today())
}

/// Anna's tool with the card EOS_DIGITAL her place last used, its DCIM the folder.
fn on_the_card() -> tool::Tool {
    let card = "/Volumes/EOS_DIGITAL";
    anna_s_tool().source(card, &format!("{card}/DCIM"))
}

/// The volumes mounted: the drive Backup, then the card EOS_DIGITAL, as the system lists
/// them.
fn volumes() -> LibraryEvent {
    let volume = |name: &str, card: bool| library::Volume {
        name: name.to_owned(),
        mount_point: PathBuf::from("/Volumes").join(name),
        card,
        removable: true,
        available: 40_000_000_000,
        total: 64_000_000_000,
    };
    LibraryEvent::Volumes {
        volumes: vec![volume("Backup", false), volume("EOS_DIGITAL", true)],
    }
}

/// The subfolders of `folder`, the answer to the `SourceSubfolders` asking for them.
fn subfolders(folder: &str, names: &[&str]) -> LibraryEvent {
    LibraryEvent::SubfoldersOf {
        open: 0,
        folder: PathBuf::from(folder),
        names: names.iter().map(|name| (*name).to_owned()).collect(),
        unreadable: None,
    }
}

/// The sheet for `source` in a 1280 × 800 window, in the dark theme and British English.
fn bench(source: Option<Source>) -> Harness<'static, Bench> {
    bench_in(source, Builtin::Dark, Locale::EnGb)
}

/// The sheet for `source` in a 1280 × 800 window, in the theme `which` and `locale`.
fn bench_in(source: Option<Source>, which: Builtin, locale: Locale) -> Harness<'static, Bench> {
    let tool = match &source {
        Some(_) => on_the_card(),
        None => anna_s_tool(),
    };
    bench_of(source, tool, which, locale)
}

/// The sheet for `source` with the tool `tool` in a 1280 × 800 window, in the theme
/// `which` and `locale`.
fn bench_of(
    source: Option<Source>,
    tool: tool::Tool,
    which: Builtin,
    locale: Locale,
) -> Harness<'static, Bench> {
    let bench = Bench {
        sheet: ImportSheet::with(source, tool),
        sent: Vec::new(),
        closed: false,
        settings: crate::settings::Settings::default(),
        written: Vec::new(),
        choose_folder: false,
        folder_start: None,
        column: apertrust_ui::components::COLUMN_WIDTH,
    };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let place = Place {
                    column: bench.column,
                    at_start: false,
                };
                let sent = &mut bench.sent;
                let mirror = crate::mirror::tests::mirror();
                let given = sheet::Given {
                    mirror: &mirror,
                    settings: &bench.settings,
                };
                let shown = bench.sheet.show(ui, place, &given, &mut sender(sent));
                bench.closed |= shown.closed;
                bench.written.extend(shown.settings);
                bench.choose_folder |= shown.choose_folder;
                bench.folder_start = shown.folder_start.or(bench.folder_start.take());
                let groups = bench
                    .settings
                    .edit
                    .groups
                    .entry("import".to_owned())
                    .or_default();
                for (group, open) in shown.groups {
                    groups.insert(group.to_owned(), open);
                }
            },
            bench,
        );
    harness.run();
    harness
}

/// `event` told to the bench's sheet, then a frame.
fn tell(harness: &mut Harness<'static, Bench>, event: LibraryEvent) {
    harness.state_mut().sheet.receive(&event);
    harness.run();
}

/// Forty photos of the card, DSCF1001 to DSCF1040 by capture time, the first ten imported
/// before, told to the bench whose listing is numbered 1.
fn forty(harness: &mut Harness<'static, Bench>) {
    let files: Vec<SourceFile> = (0..40)
        .map(|i| file(&format!("DSCF{}.RAF", 1001 + i), 30_000_000))
        .collect();
    tell(harness, listed(LISTING, files, true));
    let times: Vec<Option<DateTime>> = (0..40).map(|i| Some(at(10, i as u8))).collect();
    tell(harness, taken(LISTING, &times));
    let states = (0..40)
        .map(|i| {
            if i < 10 {
                before(&format!("DSCF{}.RAF", 1001 + i))
            } else {
                SourceState::New
            }
        })
        .collect();
    tell(harness, checked(LISTING, 1, states, 40, 40));
    harness.run();
}

/// The check box named `name`, "Import DSCF1001.RAF".
fn check_box<'h>(harness: &'h Harness<'static, Bench>, name: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::CheckBox, name)
}

/// The primary button, by its words.
fn primary<'h>(harness: &'h Harness<'static, Bench>, words: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::Button, words)
}

#[test]
fn the_sheet_lists_its_source_as_it_opens_and_cancels_the_listing_as_it_closes() {
    let mut harness = bench(Some(card()));
    let sent = &harness.state().sent;
    assert!(
        matches!(
            sent.first(),
            Some(Command::Library(LibraryCommand::ListSource { source, .. }))
                if source == Path::new("/Volumes/EOS_DIGITAL/DCIM")
        ),
        "{sent:?}"
    );
    // The first Esc leaves Import name, which a card's sheet opens in; the second closes.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().closed);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    let closing = harness.state_mut().sheet.closing();
    assert_eq!(
        closing,
        [Command::Library(LibraryCommand::CancelListing {
            listing: LISTING
        })]
    );
}

#[test]
fn a_sheet_with_no_source_lists_nothing_and_cannot_import() {
    let harness = bench(None);
    let listing = |c: &&Command| matches!(c, Command::Library(LibraryCommand::ListSource { .. }));
    assert!(!harness.state().sent.iter().any(|c| listing(&c)));
    let button = primary(&harness, "Import 0 photos");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("Choose where to import from.")
    );
}

#[test]
fn import_is_disabled_while_listing_and_checking() {
    let mut harness = bench(Some(card()));
    let files = vec![file("DSCF1001.RAF", 1)];
    tell(&mut harness, listed(LISTING, files, false));
    let still = Some("Still listing EOS_DIGITAL");
    let button = primary(&harness, "Import 0 photos");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(button.accesskit_node().description().as_deref(), still);
    tell(&mut harness, listed(LISTING, Vec::new(), true));
    tell(&mut harness, taken(LISTING, &[None]));
    let button = primary(&harness, "Import 0 photos");
    assert_eq!(button.accesskit_node().description().as_deref(), still);
    tell(
        &mut harness,
        checked(LISTING, 1, vec![SourceState::New], 1, 1),
    );
    // Then until the preview of the photos checked comes.
    let button = primary(&harness, "Import 1 photo");
    assert!(button.accesskit_node().is_disabled());
    let (asked, _) = last_preview(&harness).expect("asked at opening");
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    answer(&mut harness, asked, preview_into(&[(wedding, None)], 0));
    let button = primary(&harness, "Import 1 photo");
    assert!(!button.accesskit_node().is_disabled());
}

/// While a newer preview is on its way, the settings changed since the one shown, Import
/// waits for it, so Return right after typing never starts what the footer has not shown.
#[test]
fn import_waits_for_the_preview_of_settings_just_changed() {
    let mut harness = bench(Some(card()));
    ready(&mut harness);
    let button = primary(&harness, "Import 30 photos");
    assert!(!button.accesskit_node().is_disabled());
    harness.state_mut().sheet.settings_mut().job_code = "Studio".to_owned();
    harness.run();
    let button = primary(&harness, "Import 30 photos");
    assert!(button.accesskit_node().is_disabled());
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    let import = |c: &Command| matches!(c, Command::Library(LibraryCommand::Import { .. }));
    assert!(!harness.state().sent.iter().any(import));
    let (open, _) = last_preview(&harness).expect("asked again");
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Studio";
    answer(&mut harness, open, preview_into(&[(wedding, None)], 0));
    assert!(
        !primary(&harness, "Import 30 photos")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn import_waits_for_the_preview_of_the_photos_checked_now() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let previewing = Some("Working out where the photos go…");
    let button = primary(&harness, "Import 30 photos");
    assert_eq!(button.accesskit_node().description().as_deref(), previewing);
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    let (first, _) = last_preview(&harness).expect("asked at opening");
    answer(&mut harness, first, preview_into(&[(wedding, None)], 0));
    assert!(
        !primary(&harness, "Import 30 photos")
            .accesskit_node()
            .is_disabled()
    );
    // A photo checked: the preview held is the old checks', so Import waits again.
    check_box(&harness, "Import DSCF1001.RAF").click();
    harness.run();
    let button = primary(&harness, "Import 31 photos");
    assert_eq!(button.accesskit_node().description().as_deref(), previewing);
    let (second, _) = last_preview(&harness).expect("asked again");
    answer(&mut harness, second, preview_into(&[(wedding, None)], 0));
    assert!(
        !primary(&harness, "Import 31 photos")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn forty_photos_ten_imported_before_open_with_thirty_checked_and_the_first_new_active() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let listing = harness.state().sheet.listing();
    assert_eq!(listing.checks().checked_count(), 30);
    assert_eq!(listing.checks().active(), Some(10));
    assert_eq!(checked_places(listing), (10..40).collect::<Vec<_>>());
    // The ten imported before: unchecked, dimmed, with their mark's words.
    for i in 0..10 {
        let photo = listing.at(i);
        assert!(photo.imported_before() && !listing.checks().is_checked(i));
    }
    let (asked, _) = last_preview(&harness).expect("asked at opening");
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    answer(&mut harness, asked, preview_into(&[(wedding, None)], 0));
    assert!(
        !primary(&harness, "Import 30 photos")
            .accesskit_node()
            .is_disabled()
    );
    let cell = harness.get_by_role_and_label(Role::ListItem, "DSCF1001.RAF");
    assert_eq!(
        cell.accesskit_node().description().as_deref(),
        Some("Imported before: 20261108_Studio/DSCF1001.RAF")
    );
    let check = check_box(&harness, "Import DSCF1001.RAF");
    assert!(check.accesskit_node().is_selected() != Some(true));
    assert_eq!(
        check.accesskit_node().description().as_deref(),
        Some("Taken 15 Nov 2026, 10:00:00, imported before")
    );
    // The grid's name, and what screen readers hear as the listing ends.
    harness.get_by_role_and_label(Role::List, "EOS_DIGITAL, 40 photos, 30 checked");
    assert!(
        harness.query_all_by_role(Role::Status).any(|node| node
            .accesskit_node()
            .label()
            .as_deref()
            == Some("40 photos listed, 30 new, checked"))
    );
}

#[test]
fn a_check_box_click_space_and_ctrl_a_change_the_checks() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    check_box(&harness, "Import DSCF1001.RAF").click();
    harness.run();
    let checks = harness.state().sheet.listing().checks();
    assert!(checks.is_checked(0));
    assert_eq!(checks.checked_count(), 31);
    // Its cell has the focus: Space turns its check back.
    harness.key_press(egui::Key::Space);
    harness.run();
    let checks = harness.state().sheet.listing().checks();
    assert!(!checks.is_checked(0));
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    assert_eq!(harness.state().sheet.listing().checks().checked_count(), 40);
    harness.get_by_role_and_label(Role::Button, "Import 40 photos");
}

#[test]
fn the_keys_reach_the_active_cell_once_the_listing_opens_with_no_click() {
    // A folder's sheet: a card's opens in Import name.
    let trips = Source {
        folder: PathBuf::from("/Users/anna/Trips"),
        name: "Trips".to_owned(),
        card: false,
    };
    let mut harness = bench_of(Some(trips), anna_s_tool(), Builtin::Dark, Locale::EnGb);
    forty(&mut harness);
    harness.run();
    // DSCF1011, the first new photo, is active and checked: Space unchecks it.
    harness.key_press(egui::Key::Space);
    harness.run();
    let checks = harness.state().sheet.listing().checks();
    assert_eq!(checks.active(), Some(10));
    assert!(!checks.is_checked(10));
    assert_eq!(checks.checked_count(), 29);
}

#[test]
fn a_source_still_there_whose_listing_fails_says_why() {
    let mut harness = bench(Some(Source {
        folder: std::env::temp_dir(),
        name: "EOS_DIGITAL".to_owned(),
        card: true,
    }));
    let reason = "the card could not be read".to_owned();
    tell(
        &mut harness,
        LibraryEvent::SourceFailed {
            open: LISTING,
            reason,
        },
    );
    assert!(!harness.state().sheet.listing().gone());
    let button = primary(&harness, "Import 0 photos");
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("EOS_DIGITAL can’t be listed: the card could not be read")
    );
}

#[test]
fn only_the_cells_in_sight_ask_for_their_thumbnails() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let asked: Vec<PathBuf> = (harness.state().sent.iter())
        .filter_map(|command| match command {
            Command::Library(LibraryCommand::SourceThumbnails { photos, .. }) => Some(photos),
            _ => None,
        })
        .flatten()
        .cloned()
        .collect();
    // The grid's cells, not Source's places.
    let shown = harness
        .query_all_by_role(Role::ListItem)
        .filter(|node| {
            let node = node.accesskit_node();
            node.label().is_some_and(|label| label.contains(".RAF"))
                && (node.bounding_box()).is_some_and(|b| b.y1 > 0.0 && b.y0 < 800.0)
        })
        .count();
    assert!(!asked.is_empty() && asked.len() < 40, "{}", asked.len());
    assert_eq!(asked.len(), shown, "{asked:?}");
    // A thumbnail answered shows in its cell.
    let image = Thumbnail {
        width: 3,
        height: 2,
        pixels: Arc::from(vec![128u8; 24]),
    };
    let path = asked[0].to_string_lossy().into_owned();
    tell(
        &mut harness,
        LibraryEvent::Thumbnail {
            open: LISTING,
            photo: path,
            size: 256,
            image: Some(image),
        },
    );
    assert_eq!(harness.state().sheet.textures.len(), 1);
}

#[test]
fn as_folder_changes_the_sheet_checks_its_listing_again() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    harness.state_mut().sheet.settings_mut().base_folder = "/Volumes/Photos NAS".to_owned();
    harness.run();
    let rechecks = |sent: &[Command]| {
        (sent.iter())
            .filter(|c| matches!(c, Command::Library(LibraryCommand::CheckSource { .. })))
            .count()
    };
    let sent = &harness.state().sent;
    assert_eq!(rechecks(sent), 1, "{sent:?}");
    assert!(sent.iter().any(|command| matches!(
        command,
        Command::Library(LibraryCommand::CheckSource { listing: LISTING, base_folder, .. })
            if base_folder == "/Volumes/Photos NAS"
    )));
    // Checking, Import waits; the import name alone changing checks nothing again.
    assert!(harness.state().sheet.listing().busy());
    harness.state_mut().sheet.settings_mut().job_code = "Studio".to_owned();
    harness.run();
    assert_eq!(rechecks(&harness.state().sent), 1);
}

// The bar and the footer.

#[test]
fn select_new_all_and_none_act_on_every_cell() {
    let mut sent = Vec::new();
    let mut listing = Listing::default();
    listing.list(&card().folder, &settings(), &mut sender(&mut sent));
    let files = vec![file("A.CR3", 1), file("B.CR3", 1), file("C.CR3", 1)];
    listing.receive(&listed(LISTING, files, true));
    listing.receive(&taken(LISTING, &[None, None, None]));
    let states = vec![SourceState::New, before("B.CR3"), SourceState::New];
    listing.receive(&checked(LISTING, 1, states, 3, 3));
    let at_opening = listing.changed();
    listing.select(Select::All);
    assert_eq!(checked_places(&listing), [0, 1, 2]);
    listing.select(Select::None);
    assert!(checked_places(&listing).is_empty());
    listing.apply(CheckGesture::Toggle(1));
    // Exactly the photos not imported before, the one checked by hand unchecked.
    listing.select(Select::New);
    assert_eq!(checked_places(&listing), [0, 2]);
    assert_eq!(listing.checks().last_touched(), None);
    assert!(listing.changed() > at_opening);
    // Nothing changed, nothing to preview again.
    let now = listing.changed();
    listing.select(Select::New);
    assert_eq!(listing.changed(), now);
}

/// Anna's home.
fn home() -> Option<&'static Path> {
    Some(Path::new("/Users/anna"))
}

/// A preview of `photos` photos into `folders`, each new or with its raws there, 6.2 GB on
/// Macintosh HD's 412 GB free.
fn preview_into(folders: &[(&str, Option<usize>)], imported_before: usize) -> ImportPreview {
    ImportPreview {
        folders: (folders.iter())
            .map(|(folder, there)| library::FolderPreview {
                folder: PathBuf::from(folder),
                photos: 1,
                there: *there,
            })
            .collect(),
        imported_before,
        size: 6_200_000_000,
        free: Some(library::FreeSpace {
            volume: "Macintosh HD".to_owned(),
            available: 412_000_000_000,
            total: 994_000_000_000,
        }),
        ..ImportPreview::default()
    }
}

#[test]
fn the_footer_says_the_count_the_size_the_free_space_and_the_destination() {
    let words = Translator::new(Locale::EnGb);
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    let preview = preview_into(&[(wedding, None)], 0);
    let footer = footer(&words, "EOS_DIGITAL", (412, 209), Some(&preview), home());
    assert_eq!(footer.lead, "209 of 412 photos · 6.2 GB of 412 GB free → ");
    assert_eq!(footer.path, "~/Pictures/Apertrust/20261115_Wedding");
    assert_eq!(footer.tail, ", a new folder");
    assert_eq!(
        footer.tip.as_deref(),
        Some("~/Pictures/Apertrust/20261115_Wedding\n6.2 GB of 412 GB free on Macintosh HD")
    );
    assert_eq!(
        footer.note,
        "Copies only; nothing on EOS_DIGITAL is changed."
    );
    assert!(!footer.warning);
    // Into a folder there already.
    let lisbon = "/Users/anna/Pictures/Apertrust/2026-10 Lisbon";
    let preview = preview_into(&[(lisbon, Some(86))], 0);
    let footer = self::footer(&words, "EOS_DIGITAL", (412, 209), Some(&preview), home());
    assert_eq!(footer.tail, ", already there: 86 photos");
    // Two new folders by the day taken, under the folder they share.
    let mut preview = preview_into(
        &[
            ("/Users/anna/Pictures/Apertrust/20261114_Wedding", None),
            ("/Users/anna/Pictures/Apertrust/20261115_Wedding", None),
        ],
        0,
    );
    preview.subfolder_uses.taken = true;
    let footer = self::footer(&words, "EOS_DIGITAL", (412, 209), Some(&preview), home());
    assert_eq!(footer.path, "~/Pictures/Apertrust");
    assert_eq!(footer.tail, ", 2 new folders by the day taken");
}

#[test]
fn the_footer_warns_of_photos_copied_again_and_says_when_nothing_is_checked() {
    let words = Translator::new(Locale::EnGb);
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    let preview = preview_into(&[(wedding, None)], 380);
    let footer = footer(&words, "EOS_DIGITAL", (412, 400), Some(&preview), home());
    assert_eq!(
        footer.note,
        "380 of these were imported before; they will be copied again."
    );
    assert!(footer.warning);
    let none = self::footer(&words, "EOS_DIGITAL", (412, 0), Some(&preview), home());
    // Import's reason says it on the second line; the first stays empty.
    assert!(none.lead.is_empty() && none.path.is_empty() && none.tail.is_empty());
    assert!(!none.warning);
    // Before the first answer, the count alone; with no free space read, the size alone.
    let waiting = self::footer(&words, "EOS_DIGITAL", (412, 209), None, home());
    assert_eq!(waiting.lead, "209 of 412 photos");
    let mut unknown = preview_into(&[(wedding, None)], 0);
    unknown.free = None;
    let footer = self::footer(&words, "EOS_DIGITAL", (412, 209), Some(&unknown), home());
    assert_eq!(footer.lead, "209 of 412 photos · 6.2 GB → ");
    let whole = "~/Pictures/Apertrust/20261115_Wedding";
    assert_eq!(footer.tip.as_deref(), Some(whole));
}

#[test]
fn a_problem_in_the_footer_names_its_row() {
    let words = Translator::new(Locale::EnGb);
    let unknown = ImportProblem {
        row: Row::Subfolder,
        problem: Problem::Pattern(PatternError::Unknown {
            name: "MONT".to_owned(),
            at: 9..13,
            suggestion: None,
        }),
        fix: None,
    };
    assert_eq!(
        problem_words(&unknown, &words, home()),
        "Subfolder: unknown variable $(MONT)."
    );
    let space = ImportProblem {
        row: Row::Folder,
        problem: Problem::Space {
            needed: 6_200_000_000,
            available: 4_100_000_000,
            volume: "Macintosh HD".to_owned(),
        },
        fix: None,
    };
    assert_eq!(
        problem_words(&space, &words, home()),
        "Folder: needs 6.2 GB; 4.1 GB is free on Macintosh HD."
    );
}

/// The number of the last `PreviewImport` the bench sent, and the files it asked about.
fn last_preview(harness: &Harness<'static, Bench>) -> Option<(u64, usize)> {
    let sent = &harness.state().sent;
    (sent.iter().enumerate())
        .rev()
        .find_map(|(at, command)| match command {
            Command::Library(LibraryCommand::PreviewImport { photos, .. }) => {
                Some((at as u64 + 1, photos.files.len()))
            }
            _ => None,
        })
}

/// The answer to the preview numbered `open`.
fn answer(harness: &mut Harness<'static, Bench>, open: u64, preview: ImportPreview) {
    let preview = Box::new(preview);
    tell(harness, LibraryEvent::ImportPreview { open, preview });
}

/// The footer's first line, as screen readers read it.
fn summary_line(harness: &Harness<'static, Bench>) -> bool {
    harness
        .query_by_label(
            "30 of 40 photos · 6.2 GB of 412 GB free → ~/Pictures/Apertrust/20261115_Wedding, a new folder",
        )
        .is_some()
}

#[test]
fn the_preview_is_asked_for_as_the_checks_change_its_latest_answer_alone_shown() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let (first, files) = last_preview(&harness).expect("asked at opening");
    assert_eq!(files, 30);
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    check_box(&harness, "Import DSCF1001.RAF").click();
    harness.run();
    let (second, files) = last_preview(&harness).expect("asked again");
    assert!(second > first);
    assert_eq!(files, 31);
    // The first's answer, overtaken, is not shown; nor asked again with nothing changed.
    answer(&mut harness, first, preview_into(&[(wedding, None)], 0));
    assert!(harness.state().sheet.preview.is_none());
    assert_eq!(last_preview(&harness).map(|(n, _)| n), Some(second));
    // Unchecked again, then the latest answer shown.
    check_box(&harness, "Import DSCF1001.RAF").click();
    harness.run();
    let (third, _) = last_preview(&harness).expect("asked again");
    answer(&mut harness, third, preview_into(&[(wedding, None)], 0));
    assert!(summary_line(&harness));
    // The settings changing ask too.
    harness.state_mut().sheet.settings_mut().job_code = "Studio".to_owned();
    harness.run();
    assert!(last_preview(&harness).is_some_and(|(n, _)| n > third));
}

#[test]
fn the_bar_counts_the_listing_the_check_and_what_was_imported_before() {
    let mut harness = bench(Some(card()));
    harness.get_by_role_and_label(Role::Toolbar, "Import grid bar");
    harness.get_by_label("Import · EOS_DIGITAL");
    tell(&mut harness, listed(LISTING, vec![file("A.CR3", 1)], false));
    harness.get_by_label("Listing EOS_DIGITAL… 1 so far");
    tell(&mut harness, listed(LISTING, vec![file("B.CR3", 1)], true));
    tell(&mut harness, taken(LISTING, &[None, None]));
    harness.get_by_label("Checking… 0 of 2");
    tell(
        &mut harness,
        checked(LISTING, 1, vec![before("A.CR3")], 1, 2),
    );
    harness.get_by_label("Checking against ~/Pictures/Apertrust… 1 of 2");
    tell(
        &mut harness,
        checked(LISTING, 1, vec![SourceState::New], 2, 2),
    );
    harness.get_by_label("2 photos, 1 imported before");
}

#[test]
fn the_select_buttons_check_new_all_and_none() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let count = |harness: &Harness<'static, Bench>| {
        harness.state().sheet.listing().checks().checked_count()
    };
    harness
        .get_by_role_and_label(Role::Button, "Select all")
        .click();
    harness.run();
    assert_eq!(count(&harness), 40);
    harness
        .get_by_role_and_label(Role::Button, "Select none")
        .click();
    harness.run();
    assert_eq!(count(&harness), 0);
    harness
        .get_by_role_and_label(Role::Button, "Select new")
        .click();
    harness.run();
    assert_eq!(count(&harness), 30);
}

#[test]
fn with_no_source_the_grid_says_where_to_start() {
    let harness = bench(None);
    // In the grid, and as Import's reason.
    assert_eq!(
        harness
            .query_all_by_label("Choose where to import from.")
            .count(),
        2
    );
    harness.get_by_label(
        "A phone connected by cable doesn’t show here yet: copy its photos to a folder first.",
    );
}

#[test]
fn an_empty_card_a_folder_and_a_card_removed_say_so_and_cannot_import() {
    let mut harness = bench(Some(card()));
    tell(&mut harness, listed(LISTING, Vec::new(), true));
    tell(&mut harness, taken(LISTING, &[]));
    tell(&mut harness, checked(LISTING, 1, Vec::new(), 0, 0));
    assert!(
        harness
            .query_all_by_label("No photos on EOS_DIGITAL.")
            .count()
            >= 2
    );
    let button = primary(&harness, "Import 0 photos");
    assert!(button.accesskit_node().is_disabled());
    let wedding = Source {
        folder: PathBuf::from("/Users/anna/Desktop/Wedding"),
        name: "Wedding".to_owned(),
        card: false,
    };
    let mut harness = bench(Some(wedding));
    tell(&mut harness, listed(LISTING, Vec::new(), true));
    tell(&mut harness, taken(LISTING, &[]));
    tell(&mut harness, checked(LISTING, 1, Vec::new(), 0, 0));
    assert!(harness.query_all_by_label("No photos in Wedding.").count() >= 2);
    let mut harness = bench(Some(card()));
    tell(&mut harness, listed(LISTING, vec![file("A.CR3", 1)], false));
    let reason = "/Volumes/EOS_DIGITAL is not there".to_owned();
    tell(
        &mut harness,
        LibraryEvent::SourceFailed {
            open: LISTING,
            reason,
        },
    );
    assert!(
        harness
            .query_all_by_label("EOS_DIGITAL was removed.")
            .count()
            >= 2
    );
    assert_eq!(
        harness.state().sheet.counts(&Translator::new(Locale::EnGb)),
        ""
    );
}

/// Forty photos of the card, every one imported before.
fn all_imported(harness: &mut Harness<'static, Bench>) {
    let files: Vec<SourceFile> = (0..40)
        .map(|i| file(&format!("DSCF{}.RAF", 1001 + i), 30_000_000))
        .collect();
    tell(harness, listed(LISTING, files, true));
    let times: Vec<Option<DateTime>> = (0..40).map(|i| Some(at(10, i as u8))).collect();
    tell(harness, taken(LISTING, &times));
    let states = (0..40)
        .map(|i| before(&format!("DSCF{}.RAF", 1001 + i)))
        .collect();
    tell(harness, checked(LISTING, 1, states, 40, 40));
    harness.run();
}

#[test]
fn with_every_photo_imported_before_the_grid_says_so_and_none_is_checked() {
    let mut harness = bench(Some(card()));
    all_imported(&mut harness);
    harness.get_by_label("All 40 photos on EOS_DIGITAL are already imported.");
    let button = primary(&harness, "Import 0 photos");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("No photos checked")
    );
    assert_eq!(last_preview(&harness), None);
}

/// The ready sheet: forty photos, ten imported before, with their pictures and its
/// preview's answer.
fn ready(harness: &mut Harness<'static, Bench>) {
    tell(harness, volumes());
    tell(
        harness,
        subfolders("/Volumes/EOS_DIGITAL", &["DCIM", "MISC"]),
    );
    forty(harness);
    for i in 0..40u8 {
        // A small gradient, a little different for each photo.
        let pixels: Vec<u8> = (0..12u8)
            .flat_map(|p| [40 + 4 * i, 90 + 8 * (p % 4), 140 - 2 * i, 255])
            .collect();
        let image = Thumbnail {
            width: 4,
            height: 3,
            pixels: Arc::from(pixels),
        };
        let photo = format!("100CANON/DSCF{}.RAF", 1001 + u32::from(i));
        let thumbnail = LibraryEvent::Thumbnail {
            open: LISTING,
            photo,
            size: 256,
            image: Some(image),
        };
        harness.state_mut().sheet.receive(&thumbnail);
    }
    let (open, _) = last_preview(harness).expect("asked at opening");
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    answer(harness, open, preview_into(&[(wedding, None)], 0));
}

/// The ready sheet with a problem on Subfolder.
fn with_a_problem(harness: &mut Harness<'static, Bench>) {
    forty(harness);
    let (open, _) = last_preview(harness).expect("asked at opening");
    let mut preview = preview_into(&[], 0);
    preview.problems.push(ImportProblem {
        row: Row::Subfolder,
        problem: Problem::Pattern(PatternError::Unknown {
            name: "MONT".to_owned(),
            at: 9..13,
            suggestion: None,
        }),
        fix: None,
    });
    answer(harness, open, preview);
}

#[test]
fn a_problem_disables_import_naming_its_row() {
    let mut harness = bench(Some(card()));
    with_a_problem(&mut harness);
    let button = primary(&harness, "Import 30 photos");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(
        button.accesskit_node().description().as_deref(),
        Some("Subfolder: unknown variable $(MONT).")
    );
}

// How it looks.

/// The sheet in the theme `which` and `locale`, made as `state` makes it, photographed as
/// `name`.
fn photographed(
    which: Builtin,
    locale: Locale,
    source: Option<Source>,
    state: impl FnOnce(&mut Harness<'static, Bench>),
    name: &str,
) {
    let mut harness = bench_in(source, which, locale);
    state(&mut harness);
    harness.run();
    harness.snapshot(name);
}

/// One snapshot test for each look.
macro_rules! looks {
    ($($test:ident: $which:expr, $locale:expr, $source:expr, $state:expr, $file:literal;)*) => {
        $(
            #[test]
            fn $test() {
                photographed($which, $locale, $source, $state, $file);
            }
        )*
    };
}

looks! {
    ready_in_the_dark_theme: Builtin::Dark, Locale::EnGb, Some(card()), ready, "import-sheet-ready-dark";
    ready_in_the_light_theme: Builtin::Light, Locale::EnGb, Some(card()), ready, "import-sheet-ready-light";
    ready_in_american_english: Builtin::Dark, Locale::EnUs, Some(card()), ready, "import-sheet-ready-en-us";
    ready_in_ukrainian: Builtin::Dark, Locale::Uk, Some(card()), ready, "import-sheet-ready-uk";
    ready_in_polish: Builtin::Dark, Locale::Pl, Some(card()), ready, "import-sheet-ready-pl";
    ready_in_the_pseudo_locale: Builtin::Dark, Locale::Pseudo, Some(card()), ready, "import-sheet-ready-pseudo";
    all_imported_in_the_dark_theme: Builtin::Dark, Locale::EnGb, Some(card()), all_imported, "import-sheet-all-imported-dark";
    all_imported_in_the_light_theme: Builtin::Light, Locale::EnGb, Some(card()), all_imported, "import-sheet-all-imported-light";
    no_source_in_the_dark_theme: Builtin::Dark, Locale::EnGb, None, |_| {}, "import-sheet-no-source-dark";
    no_source_in_the_light_theme: Builtin::Light, Locale::EnGb, None, |_| {}, "import-sheet-no-source-light";
    a_problem_in_the_dark_theme: Builtin::Dark, Locale::EnGb, Some(card()), with_a_problem, "import-sheet-problem-dark";
    a_problem_in_the_light_theme: Builtin::Light, Locale::EnGb, Some(card()), with_a_problem, "import-sheet-problem-light";
}

/// Decision 93: every key the Import tool's declaration names is an import setting of the
/// settings schema, a text, as its row shows it; a setting row with no key is a later
/// slice's.
#[test]
fn every_key_the_import_tool_names_is_a_text_setting_of_the_schema() {
    use apertrust_base::settings::Kind;
    use apertrust_library::settings::registry::ALL;
    use apertrust_tools::utility::import;
    for (widget, key) in import::KEYS {
        let declared = ALL.iter().find(|declared| declared.id() == *key);
        let declared = declared.unwrap_or_else(|| panic!("{key} is no import setting"));
        assert_eq!(*declared.kind(), Kind::Text, "{key}");
        assert_eq!(import::key(*widget), Some(*key));
    }
    fn keyed(rows: &'static [apertrust_tools::Row]) -> Vec<&'static str> {
        let settings = rows.iter().flat_map(|row| row.setting_rows());
        let mut keys: Vec<&str> = settings.filter_map(|setting| setting.key).collect();
        keys.extend(rows.iter().flat_map(|row| keyed(row.rows())));
        keys
    }
    assert!(keyed(import::IMPORT.tool.tabs[0].rows).is_empty());
}

// The Import tool.

/// The field named `name` in the bench's column.
fn field<'h>(harness: &'h Harness<'static, Bench>, name: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::TextInput, name)
}

/// The settings the bench's tool holds now.
fn held(harness: &mut Harness<'static, Bench>) -> ImportSettings {
    harness.state_mut().sheet.settings_mut().clone()
}

/// 20a1's rows, from the declaration: the text rows live, Folder's ⌄ and File name
/// disabled, and every setting row of a later slice disabled at the value 20a1 gives it,
/// saying why.
#[test]
fn the_rows_of_20a1_are_live_or_disabled_saying_why() {
    let harness = bench(Some(card()));
    for name in ["Import name", "Folder", "Subfolder"] {
        assert!(
            !field(&harness, name).accesskit_node().is_disabled(),
            "{name}"
        );
    }
    let file_name = field(&harness, "File name");
    assert!(file_name.accesskit_node().is_disabled());
    let later = harness.get_by_role_and_label(Role::Button, "Folder’s choices");
    assert!(later.accesskit_node().is_disabled());
    let keep = harness.get_by_role_and_label(Role::Switch, "Keep original file name");
    assert!(keep.accesskit_node().is_disabled());
    assert_eq!(
        keep.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    let verify = harness.get_by_role_and_label(Role::Switch, "Verify the copies");
    assert!(verify.accesskit_node().is_disabled());
    assert_eq!(
        verify.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    for group in ["Metadata", "Filters"] {
        let row = harness.get_by_role_and_label(Role::Button, group);
        assert!(row.accesskit_node().is_disabled(), "{group}");
    }
    let presets = harness.get_by_role_and_label(Role::Button, "Import presets");
    assert!(presets.accesskit_node().is_disabled());
}

/// Subfolder's ⌄ sets Subfolder alone to each of its four patterns, each written at once.
#[test]
fn subfolder_s_choices_set_each_pattern() {
    let mut harness = bench(Some(card()));
    let words = Translator::new(Locale::EnGb);
    let mut order = tool::SUBFOLDERS.to_vec();
    order.rotate_left(1);
    for (_, name, pattern) in order {
        harness
            .get_by_role_and_label(Role::Button, "Subfolder’s choices")
            .click();
        harness.run();
        let name = words.text(name);
        harness
            .get_by_role_and_label(Role::MenuItemRadio, &name)
            .click();
        harness.run();
        assert_eq!(held(&mut harness).subfolder, pattern, "{name}");
        let last = harness.state().written.last().cloned();
        assert_eq!(last, Some(("import.subfolder", pattern.to_owned())));
    }
    assert_eq!(held(&mut harness).base_folder, settings().base_folder);
}

/// ↺ puts back darktable's defaults, Folder the default folder, written at once, then
/// rests.
#[test]
fn reset_puts_back_the_defaults_and_folder_the_default_folder() {
    let mut settings = settings();
    settings.base_folder = "/Volumes/Photos NAS".to_owned();
    settings.default_folder = "$(PICTURES_FOLDER)/Studio".to_owned();
    settings.subfolder = "$(IMPORT_NAME)".to_owned();
    let tool = tool::Tool::new(settings).today(today());
    let mut harness = bench_of(Some(card()), tool, Builtin::Dark, Locale::EnGb);
    harness
        .get_by_role_and_label(Role::Button, "Reset import settings")
        .click();
    harness.run();
    let held = held(&mut harness);
    assert_eq!(held.base_folder, "$(PICTURES_FOLDER)/Studio");
    assert_eq!(held.subfolder, "$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)");
    assert_eq!(held.job_code, "no_name");
    let written: Vec<&str> = harness.state().written.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        written,
        ["import.base_folder", "import.subfolder", "import.job_code"]
    );
    harness.run();
    let reset = harness.get_by_role_and_label(Role::Button, "Reset import settings");
    assert!(reset.accesskit_node().is_disabled());
}

/// Import name is disabled where neither Folder nor Subfolder, not empty, uses the import
/// name, `$(IMPORT_NAME)` or darktable's `$(JOBCODE)`, saying so.
#[test]
fn import_name_is_disabled_where_no_folder_uses_it() {
    let mut harness = bench(Some(card()));
    harness.state_mut().sheet.settings_mut().subfolder = String::new();
    harness.run();
    let name = field(&harness, "Import name");
    assert!(name.accesskit_node().is_disabled());
    assert_eq!(
        name.accesskit_node().description().as_deref(),
        Some("No folder or file name uses the import name")
    );
    harness.state_mut().sheet.settings_mut().base_folder = "/Volumes/Photos/$(JOBCODE)".to_owned();
    harness.run();
    let name = field(&harness, "Import name");
    assert!(!name.accesskit_node().is_disabled());
    assert_eq!(
        name.accesskit_node().description().as_deref(),
        Some("Fills $(IMPORT_NAME) where the folder or file name uses it")
    );
}

/// The rows' tooltips come from the latest preview: Folder's the sample path for the active
/// photo, a new folder, that nothing is replaced and the clashes; Subfolder's the date.
#[test]
fn folder_and_subfolder_say_where_the_photos_go_from_the_preview() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let (open, _) = last_preview(&harness).expect("asked at opening");
    let wedding = "/Users/anna/Pictures/Apertrust/20261115_Wedding";
    let mut preview = preview_into(&[(wedding, None)], 0);
    preview.subfolder_uses.today = true;
    // A photo of the same name in another folder of the card, first: the active photo is
    // matched by its path, not its name.
    preview.photos = vec![
        library::PhotoPreview {
            source: PathBuf::from("101CANON/DSCF1011.RAF"),
            fate: library::ImportFate::Copy {
                to: PathBuf::from(wedding).join("DSCF1011_01.RAF"),
                renamed: true,
            },
        },
        library::PhotoPreview {
            source: PathBuf::from("100CANON/DSCF1011.RAF"),
            fate: library::ImportFate::Copy {
                to: PathBuf::from(wedding).join("DSCF1011.RAF"),
                renamed: false,
            },
        },
        library::PhotoPreview {
            source: PathBuf::from("100CANON/DSCF1012.RAF"),
            fate: library::ImportFate::Diverged {
                at: PathBuf::from(wedding).join("DSCF1012.RAF"),
                sidecar: PathBuf::from(wedding).join("DSCF1012.RAF.apxmp"),
            },
        },
    ];
    answer(&mut harness, open, preview);
    let folder = field(&harness, "Folder");
    assert_eq!(
        folder.accesskit_node().description().as_deref(),
        Some(
            "DSCF1011.RAF → ~/Pictures/Apertrust/20261115_Wedding/DSCF1011.RAF\n\
             A new folder in Apertrust\n\
             Never replaces a file: a name taken gets _01; a photo already there is skipped\n\
             1 is already there with an edit of its own; its edit on EOS_DIGITAL stays there"
        )
    );
    let subfolder = field(&harness, "Subfolder");
    assert_eq!(
        subfolder.accesskit_node().description().as_deref(),
        Some("One folder for this import, dated today, 15 Nov")
    );
}

/// A problem in the latest preview is its row's warning button, its menu the fix: Use
/// $(MONTH) puts the variable in place of the name typed.
#[test]
fn a_problem_shows_its_warning_button_and_its_fix_applies() {
    let mut harness = bench(Some(card()));
    harness.state_mut().sheet.settings_mut().subfolder = "$(YEAR)$(MONT)$(DAY)".to_owned();
    forty(&mut harness);
    let (open, _) = last_preview(&harness).expect("asked at opening");
    let mut preview = preview_into(&[], 0);
    preview.problems.push(ImportProblem {
        row: Row::Subfolder,
        problem: Problem::Pattern(PatternError::Unknown {
            name: "MONT".to_owned(),
            at: 9..13,
            suggestion: Some("MONTH"),
        }),
        fix: Some(library::Fix::Use { variable: "MONTH" }),
    });
    answer(&mut harness, open, preview);
    let problem = "Subfolder: unknown variable $(MONT). Did you mean $(MONTH)?";
    harness.get_by_role_and_label(Role::Button, problem).click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Use $(MONTH)")
        .click();
    harness.run();
    assert_eq!(held(&mut harness).subfolder, "$(YEAR)$(MONTH)$(DAY)");
}

/// A closed group with a problem opens once, as the problem appears, and is kept open as a
/// tool's groups are; closed again, the same problem leaves it closed.
#[test]
fn a_closed_group_opens_once_for_a_problem() {
    let closed = [("destination".to_owned(), false)].into_iter().collect();
    let tool = anna_s_tool().groups(closed);
    let mut harness = bench_of(Some(card()), tool, Builtin::Dark, Locale::EnGb);
    assert!(
        harness
            .query_by_role_and_label(Role::TextInput, "Subfolder")
            .is_none()
    );
    with_a_problem(&mut harness);
    harness.run();
    field(&harness, "Subfolder");
    assert_eq!(
        harness.state().settings.edit.groups["import"].get("destination"),
        Some(&true)
    );
    harness
        .get_by_role_and_label(Role::Button, "Destination")
        .click();
    harness.run();
    let (open, _) = last_preview(&harness).expect("asked");
    let preview = harness.state().sheet.preview.clone().expect("answered");
    answer(&mut harness, open, preview);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::TextInput, "Subfolder")
            .is_none()
    );
}

/// A key `--conf` gives for the run is drawn disabled at its value, saying so.
#[test]
fn a_conf_key_is_drawn_disabled_saying_so() {
    let tool = anna_s_tool().given(["import.subfolder".to_owned()]);
    let harness = bench_of(Some(card()), tool, Builtin::Dark, Locale::EnGb);
    let subfolder = field(&harness, "Subfolder");
    assert!(subfolder.accesskit_node().is_disabled());
    let said = subfolder.accesskit_node().description().unwrap_or_default();
    assert!(said.contains("--conf"), "{said}");
    assert!(!field(&harness, "Folder").accesskit_node().is_disabled());
}

/// The folder button asks the shell for the system's folder chooser, whose answer is
/// Folder, written at once.
#[test]
fn the_folder_button_asks_for_a_folder_which_becomes_folder() {
    let mut harness = bench(Some(card()));
    harness
        .get_by_role_and_label(Role::Button, "Choose a folder")
        .click();
    harness.run();
    assert!(harness.state().choose_folder);
    // It starts at Folder, its variables filled in.
    let start = harness.state().folder_start.clone();
    assert_eq!(start, Some(PathBuf::from("/Users/anna/Pictures/Apertrust")));
    let sheet = &mut harness.state_mut().sheet;
    sheet::Content::folder_chosen(sheet, Path::new("/Volumes/Photos NAS/2026"));
    harness.run();
    assert_eq!(held(&mut harness).base_folder, "/Volumes/Photos NAS/2026");
    let last = harness.state().written.last().cloned();
    assert_eq!(
        last,
        Some(("import.base_folder", "/Volumes/Photos NAS/2026".to_owned()))
    );
}

/// The ready sheet with its column at its narrowest, 296 points.
fn ready_at_296(harness: &mut Harness<'static, Bench>) {
    harness.state_mut().column = apertrust_ui::components::COLUMN_LEAST;
    ready(harness);
}

looks! {
    ready_at_296_in_the_dark_theme: Builtin::Dark, Locale::EnGb, Some(card()), ready_at_296, "import-sheet-ready-296-dark";
    ready_at_296_in_the_light_theme: Builtin::Light, Locale::EnGb, Some(card()), ready_at_296, "import-sheet-ready-296-light";
}

// The Source group.

/// The sources the bench's commands listed, in order.
fn listed_sources(harness: &Harness<'static, Bench>) -> Vec<PathBuf> {
    (harness.state().sent.iter())
        .filter_map(|command| match command {
            Command::Library(LibraryCommand::ListSource { source, .. }) => Some(source.clone()),
            _ => None,
        })
        .collect()
}

/// Places lists the volumes as the sheet opened, cards first, then the drives, Pictures
/// and Home; with no place last used, the card is chosen at opening and listed, its DCIM
/// the folder, kept as the place and folder last used.
#[test]
fn places_list_the_card_first_and_the_card_is_chosen_at_opening() {
    let mut harness = bench_of(None, anna_s_tool(), Builtin::Dark, Locale::EnGb);
    let sent = &harness.state().sent;
    assert!(sent.contains(&Command::Library(LibraryCommand::ListVolumes)));
    assert!(listed_sources(&harness).is_empty());
    tell(&mut harness, volumes());
    harness.run();
    let tops: Vec<f32> = ["EOS_DIGITAL", "Backup", "Pictures", "Home"]
        .iter()
        .map(|name| {
            harness
                .get_by_role_and_label(Role::ListItem, name)
                .rect()
                .top()
        })
        .collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    assert_eq!(
        listed_sources(&harness),
        [PathBuf::from("/Volumes/EOS_DIGITAL/DCIM")]
    );
    let written = &harness.state().written;
    assert!(written.contains(&("import.place", "/Volumes/EOS_DIGITAL".to_owned())));
    assert!(written.contains(&("import.folder", "/Volumes/EOS_DIGITAL/DCIM".to_owned())));
    harness.get_by_label("Import · EOS_DIGITAL");
    // A card is listed whole: Include subfolders on and disabled, saying why.
    let include = harness.get_by_role_and_label(Role::Switch, "Include subfolders");
    assert_eq!(
        include.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert_eq!(
        include.accesskit_node().description().as_deref(),
        Some("A card’s DCIM is always listed whole")
    );
}

/// Choosing Home, then a folder of its tree, lists each in turn, the later listing's
/// events alone kept, so the first is dropped; Home's folders are on this computer.
#[test]
fn choosing_home_then_a_folder_lists_that_folder_in_place_of_the_first() {
    let mut harness = bench_of(None, anna_s_tool(), Builtin::Dark, Locale::EnGb);
    let mut no_card = volumes();
    if let LibraryEvent::Volumes { volumes } = &mut no_card {
        volumes.retain(|volume| !volume.card);
    }
    tell(&mut harness, no_card);
    harness
        .get_by_role_and_label(Role::ListItem, "Home")
        .click();
    harness.run();
    harness.run();
    let asked = Command::Library(LibraryCommand::SourceSubfolders {
        folder: PathBuf::from("/Users/anna"),
    });
    assert!(harness.state().sent.contains(&asked));
    tell(&mut harness, subfolders("/Users/anna", &["Trips", "Work"]));
    let first = harness.state().sent.len();
    let trips = harness.get_by_role_and_label(Role::TreeItem, "Trips");
    assert_eq!(
        trips.accesskit_node().description().as_deref(),
        Some("These photos are already on this computer")
    );
    trips.click();
    harness.run();
    harness.run();
    assert_eq!(
        listed_sources(&harness),
        [
            PathBuf::from("/Users/anna"),
            PathBuf::from("/Users/anna/Trips")
        ]
    );
    assert!(harness.state().sent.len() > first);
    harness.get_by_label("Import · Trips");
    let include = harness.get_by_role_and_label(Role::Switch, "Include subfolders");
    assert_eq!(
        include.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    // The first listing's events are dropped: its number is not the sheet's any more.
    let home = (harness.state().sent.iter())
        .position(|c| matches!(c, Command::Library(LibraryCommand::ListSource { .. })))
        .expect("Home listed") as u64
        + 1;
    tell(&mut harness, listed(home, vec![file("A.CR3", 1)], true));
    assert_eq!(harness.state().sheet.listing().len(), 0);
}

/// The place and folder last used come back as the source, with no card in.
#[test]
fn the_place_and_folder_last_used_are_the_source_with_no_card_in() {
    let tool = anna_s_tool().source("/Users/anna", "/Users/anna/Trips");
    let harness = bench_of(None, tool, Builtin::Dark, Locale::EnGb);
    assert_eq!(
        listed_sources(&harness),
        [PathBuf::from("/Users/anna/Trips")]
    );
}

/// A folder last used deep in its place opens the tree down to it: each folder from the
/// place to its parent asked for its subfolders and open, the folder's row in sight, and
/// the folder itself left closed.
#[test]
fn the_folder_last_used_opens_the_tree_down_to_it() {
    let tool = anna_s_tool().source("/Users/anna", "/Users/anna/Trips/Lisbon");
    let mut harness = bench_of(None, tool, Builtin::Dark, Locale::EnGb);
    let mut no_card = volumes();
    if let LibraryEvent::Volumes { volumes } = &mut no_card {
        volumes.retain(|volume| !volume.card);
    }
    tell(&mut harness, no_card);
    let asked = |harness: &Harness<'static, Bench>, folder: &str| {
        let ask = Command::Library(LibraryCommand::SourceSubfolders {
            folder: PathBuf::from(folder),
        });
        harness.state().sent.contains(&ask)
    };
    assert!(asked(&harness, "/Users/anna"));
    assert!(asked(&harness, "/Users/anna/Trips"));
    assert!(!asked(&harness, "/Users/anna/Trips/Lisbon"));
    tell(&mut harness, subfolders("/Users/anna", &["Trips", "Work"]));
    tell(
        &mut harness,
        subfolders("/Users/anna/Trips", &["Lisbon", "Porto"]),
    );
    harness.get_by_role_and_label(Role::TreeItem, "Lisbon");
}

/// Closed, Source says what it imports from after its title, to screen readers too.
#[test]
fn closed_source_says_the_card() {
    let mut harness = bench(Some(card()));
    tell(&mut harness, volumes());
    harness
        .get_by_role_and_label(Role::Button, "Source")
        .click();
    harness.run();
    let group = harness.get_by_role_and_label(Role::Button, "Source");
    assert_eq!(
        group.accesskit_node().description().as_deref(),
        Some("Card: EOS_DIGITAL")
    );
}

/// The ready sheet with Source closed.
fn ready_source_closed(harness: &mut Harness<'static, Bench>) {
    ready(harness);
    harness
        .get_by_role_and_label(Role::Button, "Source")
        .click();
    harness.run();
}

looks! {
    source_closed_in_the_dark_theme: Builtin::Dark, Locale::EnGb, Some(card()), ready_source_closed, "import-sheet-source-closed-dark";
    source_closed_in_the_light_theme: Builtin::Light, Locale::EnGb, Some(card()), ready_source_closed, "import-sheet-source-closed-light";
}

/// The card last used, opened again with no source given, is told for a card once the
/// volumes come: its name, Include subfolders on, Import name focused, and its listing,
/// of the same DCIM, kept.
#[test]
fn the_card_last_used_is_a_card_once_the_volumes_come() {
    let mut harness = bench_of(None, on_the_card(), Builtin::Dark, Locale::EnGb);
    assert_eq!(
        listed_sources(&harness),
        [PathBuf::from("/Volumes/EOS_DIGITAL/DCIM")]
    );
    tell(&mut harness, volumes());
    harness.run();
    harness.get_by_label("Import · EOS_DIGITAL");
    let include = harness.get_by_role_and_label(Role::Switch, "Include subfolders");
    assert_eq!(
        include.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert!(field(&harness, "Import name").is_focused());
    assert_eq!(listed_sources(&harness).len(), 1, "listed once");
}

/// Places is a list and Folders a tree to screen readers, each named by its label; → on a
/// focused folder opens it, asking for its subfolders, and ← closes it, as Collections'
/// tree's keys do.
#[test]
fn places_is_a_list_folders_a_tree_whose_arrows_open_and_close() {
    let tool = anna_s_tool().source("/Users/anna", "/Users/anna");
    let mut harness = bench_of(None, tool, Builtin::Dark, Locale::EnGb);
    // No card in, so Home stays chosen.
    let mut no_card = volumes();
    if let LibraryEvent::Volumes { volumes } = &mut no_card {
        volumes.retain(|volume| !volume.card);
    }
    tell(&mut harness, no_card);
    harness.get_by_role_and_label(Role::List, "Places");
    tell(&mut harness, subfolders("/Users/anna", &["Trips", "Work"]));
    harness.get_by_role_and_label(Role::Tree, "Folders");
    let trips = Path::new("/Users/anna/Trips");
    let asked = |harness: &Harness<'static, Bench>| {
        let ask = Command::Library(LibraryCommand::SourceSubfolders {
            folder: trips.to_owned(),
        });
        harness.state().sent.contains(&ask)
    };
    assert!(!asked(&harness));
    harness
        .get_by_role_and_label(Role::TreeItem, "Trips")
        .focus();
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    harness.run();
    assert!(asked(&harness));
    tell(&mut harness, subfolders("/Users/anna/Trips", &["Lisbon"]));
    harness.get_by_role_and_label(Role::TreeItem, "Lisbon");
    harness
        .get_by_role_and_label(Role::TreeItem, "Trips")
        .focus();
    harness.run();
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::TreeItem, "Lisbon")
            .is_none()
    );
}

/// With a card as the source the sheet opens with the focus in Import name, a card's
/// import being the name typed and Return; not for a folder, nor where Import name is
/// disabled.
#[test]
fn a_card_opens_with_the_focus_in_import_name_unless_it_is_disabled() {
    let harness = bench(Some(card()));
    assert!(field(&harness, "Import name").is_focused());
    let trips = Source {
        folder: PathBuf::from("/Users/anna/Trips"),
        name: "Trips".to_owned(),
        card: false,
    };
    let harness = bench_of(Some(trips), anna_s_tool(), Builtin::Dark, Locale::EnGb);
    assert!(!field(&harness, "Import name").is_focused());
    let mut unused = settings();
    unused.subfolder = String::new();
    let tool = tool::Tool::new(unused).today(today());
    let harness = bench_of(Some(card()), tool, Builtin::Dark, Locale::EnGb);
    assert!(!field(&harness, "Import name").is_focused());
}

// Starting the import.

/// The `Import` commands the bench sent.
fn imports(harness: &Harness<'static, Bench>) -> Vec<(ImportPhotos, ImportSettings)> {
    (harness.state().sent.iter())
        .filter_map(|command| match command {
            Command::Library(LibraryCommand::Import { photos, settings }) => {
                Some((photos.clone(), settings.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Import pressed sends `Import` with the photos checked and the settings, once, and
/// closes the sheet.
#[test]
fn import_sends_the_checked_photos_with_the_settings_and_closes() {
    let mut harness = bench(Some(card()));
    ready(&mut harness);
    primary(&harness, "Import 30 photos").click();
    harness.run();
    let sent = imports(&harness);
    assert_eq!(sent.len(), 1);
    let (photos, sent_settings) = &sent[0];
    assert_eq!(photos.files.len(), 30);
    assert_eq!(photos.folder, PathBuf::from("/Volumes/EOS_DIGITAL/DCIM"));
    assert_eq!(sent_settings, &settings());
    assert!(harness.state().closed);
    // A second press as it closes is ignored.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    assert_eq!(imports(&harness).len(), 1);
}

/// Return in Import name commits it and presses Import, a card's import being the name
/// typed and Return; Return in Subfolder commits the field and presses nothing.
#[test]
fn return_in_import_name_presses_import_and_in_subfolder_it_does_not() {
    let mut harness = bench(Some(card()));
    ready(&mut harness);
    field(&harness, "Subfolder").click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(imports(&harness).is_empty());
    assert!(!harness.state().closed);
    field(&harness, "Import name").click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(imports(&harness).len(), 1);
    assert!(harness.state().closed);
}
