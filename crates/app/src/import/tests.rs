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

use apertrust_engine::session::library::Places;
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

/// The card EOS_DIGITAL.
fn card() -> Source {
    Source {
        folder: PathBuf::from("/Volumes/EOS_DIGITAL"),
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
            source: PathBuf::from("/Volumes/EOS_DIGITAL"),
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
}

/// The sheet for `source` in a 1280 × 800 window, in the dark theme and British English.
fn bench(source: Option<Source>) -> Harness<'static, Bench> {
    let bench = Bench {
        sheet: ImportSheet::new(source, settings()),
        sent: Vec::new(),
        closed: false,
    };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let place = Place {
                    column: apertrust_ui::components::COLUMN_WIDTH,
                    at_start: false,
                };
                let sent = &mut bench.sent;
                let shown = bench.sheet.show(ui, place, &mut sender(sent));
                bench.closed |= shown.closed;
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
                if source == Path::new("/Volumes/EOS_DIGITAL")
        ),
        "{sent:?}"
    );
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
    assert!(harness.state().sent.is_empty());
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
    let button = primary(&harness, "Import 1 photo");
    assert!(!button.accesskit_node().is_disabled());
}

#[test]
fn forty_photos_ten_imported_before_open_with_thirty_checked_and_the_first_new_active() {
    let mut harness = bench(Some(card()));
    forty(&mut harness);
    let listing = harness.state().sheet.listing();
    assert_eq!(listing.checks().checked_count(), 30);
    assert_eq!(listing.checks().active(), Some(10));
    assert_eq!(checked_places(listing), (10..40).collect::<Vec<_>>());
    assert!(
        !primary(&harness, "Import 30 photos")
            .accesskit_node()
            .is_disabled()
    );
    // The ten imported before: unchecked, dimmed, with their mark's words.
    for i in 0..10 {
        let photo = listing.at(i);
        assert!(photo.imported_before() && !listing.checks().is_checked(i));
    }
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
    let mut harness = bench(Some(card()));
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
    let shown = harness
        .query_all_by_role(Role::ListItem)
        .filter(|node| {
            node.accesskit_node()
                .bounding_box()
                .is_some_and(|b| b.y1 > 0.0 && b.y0 < 800.0)
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
    assert!(matches!(
        sent.last(),
        Some(Command::Library(LibraryCommand::CheckSource { listing: LISTING, base_folder, .. }))
            if base_folder == "/Volumes/Photos NAS"
    ));
    // Checking, Import waits; the import name alone changing checks nothing again.
    assert!(harness.state().sheet.listing().busy());
    harness.state_mut().sheet.settings_mut().job_code = "Studio".to_owned();
    harness.run();
    assert_eq!(rechecks(&harness.state().sent), 1);
}
