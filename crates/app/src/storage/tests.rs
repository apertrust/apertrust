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

//! Settings › Storage drawn in the Settings dialog, as the host draws it, over views the
//! tests make, and its mirror fed the session's events.

use std::path::Path;

use apertrust_base::cache::Cache;
use apertrust_engine::jobs::{Queue, State};
use apertrust_ui::{Builtin, Locale, Theme};
use eframe::egui::os::OperatingSystem;
use eframe::egui::{Event as Input, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::dialog_host::{self, Output};
use crate::jobs::tests::{added, ended, ids};
use crate::settings::Keeper;
use crate::settings_dialog::{self, Settings};

/// The person's home in the tests' folders.
const HOME: &str = "/Users/anna";

/// The folders as an app started with none named would have them on a Mac.
pub(crate) fn folders() -> Folders {
    Folders {
        user_data: Some(Folder {
            path: PathBuf::from("/Users/anna/Library/Application Support/Apertrust"),
            chosen: Chosen::Platform,
        }),
        cache: Some(Folder {
            path: PathBuf::from("/Users/anna/Library/Caches/Apertrust"),
            chosen: Chosen::Flag("--cache-dir"),
        }),
        home: Some(PathBuf::from(HOME)),
        file_manager: FileManager::Finder,
    }
}

/// The sizes of a cache of twelve thumbnails of 10 kB and one library's cache of 5 kB,
/// within the default limit, as the session answers them.
pub(crate) fn sizes() -> CacheEvent {
    // A folder of each call's own, as the tests run at once.
    static CALLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("apertrust-app-storage-{}-{call}", std::process::id());
    let folder = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&folder);
    let cache = Cache::open(&folder);
    for slot in 0u8..12 {
        cache
            .write(THUMBNAILS, &[slot], b"v", &[0; 10_000])
            .unwrap();
    }
    let library = cache.kind_folder(LIBRARIES).join("0f3a");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("index"), [0; 5_000]).unwrap();
    let sizes = cache.walk(&|| false).unwrap().unwrap();
    let _ = std::fs::remove_dir_all(&folder);
    CacheEvent::sizes(sizes, Limit::Bytes(4_000_000_000))
}

/// The page as Settings shows it with [`sizes`].
pub(crate) fn sample() -> View {
    View {
        folders: folders(),
        cache: Some(sizes()),
        clearing: Vec::new(),
        refused: None,
    }
}

/// The dialog as the host draws it, and what the page asked.
struct Bench {
    content: Settings,
    asked: Vec<Asked>,
}

/// Settings on its Storage page showing `view`, in British English.
fn page(view: View) -> Harness<'static, Bench> {
    let (keeper, _) = Keeper::nowhere();
    let mut content = Settings::new(&keeper, false).with_storage(view);
    content.page = apertrust_base::settings::Page::Storage;
    let mut harness = Harness::builder()
        .with_size(vec2(settings_dialog::WIDTH, 700.0))
        .with_os(OperatingSystem::Mac)
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let drawn = dialog_host::draw(ui, &mut bench.content);
                if let Some(Output::Storage(asked)) = drawn.output {
                    bench.asked.push(asked);
                }
            },
            Bench {
                content,
                asked: Vec::new(),
            },
        );
    harness.run();
    harness
}

fn enabled(harness: &Harness<'_, Bench>, name: &str) -> bool {
    !harness.get_by_label(name).accesskit_node().is_disabled()
}

/// The page shows each folder in full under the home as `~`, how it was chosen, each
/// kind's entries and size in the language's words, the total against the limit and a
/// Clear for each limited kind; and it asks for the sizes as it is shown.
#[test]
fn the_page_shows_the_folders_and_the_sizes_and_asks_for_them() {
    let harness = page(sample());
    for text in [
        "User data folder",
        "~/Library/Application Support/Apertrust",
        "The system's folder",
        "Cache folder",
        "~/Library/Caches/Apertrust",
        "Chosen by --cache-dir",
        "Thumbnails",
        "12 thumbnails, 120 kB",
        "Pipelines",
        "0 pipelines, 0 kB",
        "Library caches",
        "1 library, 5 kB",
        "120 kB of 4 GB",
    ] {
        harness.get_by_label(text);
    }
    assert!(enabled(&harness, "Clear the thumbnails"));
    assert!(enabled(&harness, "Clear the pipelines"));
    assert!(enabled(&harness, "Show in Finder: Cache folder"));
    assert_eq!(harness.state().asked, [Asked::Sizes]);

    // Unlimited, the total alone.
    let mut view = sample();
    if let Some(CacheEvent::Sizes { limit, .. }) = &mut view.cache {
        *limit = Limit::Unlimited;
    }
    page(view).get_by_label("120 kB, unlimited");

    // Before the first answer, counting, and nothing to clear yet.
    let view = View {
        cache: None,
        ..sample()
    };
    let harness = page(view);
    assert!(!enabled(&harness, "Clear the thumbnails"));
    assert!(harness.query_all_by_label("Counting…").count() >= 3);
}

/// With no cache the page says so plainly, and has no Clear.
#[test]
fn with_no_cache_the_page_says_so_and_has_no_clear() {
    let view = View {
        cache: Some(CacheEvent::NoCache),
        ..sample()
    };
    let harness = page(view);
    harness.get_by_label(
        "There is no disk cache: its folder could not be written as Apertrust started, so \
         nothing is kept between runs.",
    );
    assert!(harness.query_by_label("Clear the thumbnails").is_none());
    assert!(harness.query_by_label("Total").is_none());
    // The folders are still shown.
    harness.get_by_label("~/Library/Caches/Apertrust");
}

/// Clear sends its command; its button is disabled from the press until the job it
/// started has ended, then enabled again; a refusal is worded under the lines.
#[test]
fn clear_asks_and_its_button_waits_for_its_job_to_end() {
    let mut harness = page(sample());
    harness.state_mut().asked.clear();
    harness.get_by_label("Clear the thumbnails").click();
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Clear(THUMBNAILS)]);

    // The app sends it as its command 7, which starts job 1.
    let mut mirror = Mirror::default();
    mirror.apply(&Event::Cache(sizes()));
    mirror.sent(THUMBNAILS, 7);
    let show = |harness: &mut Harness<'_, Bench>, mirror: &Mirror| {
        harness.state_mut().content.storage = mirror.view(&folders());
        harness.run();
    };
    show(&mut harness, &mirror);
    assert!(!enabled(&harness, "Clear the thumbnails"));
    assert!(enabled(&harness, "Clear the pipelines"));
    let job = ids(1)[0];
    mirror.apply(&added(job, Queue::UserBackground, Some(7)));
    // A sizes fact before the end leaves it disabled.
    mirror.apply(&Event::Cache(sizes()));
    show(&mut harness, &mirror);
    assert!(!enabled(&harness, "Clear the thumbnails"));
    mirror.apply(&ended(job, State::Finished, None));
    show(&mut harness, &mirror);
    assert!(enabled(&harness, "Clear the thumbnails"));

    // Refused, it is enabled again, and the page says why in words.
    mirror.sent(THUMBNAILS, 9);
    let error = Box::new(SessionError::CacheClearing { kind: THUMBNAILS });
    mirror.apply(&Event::Failed(SessionError::Refused { command: 9, error }));
    show(&mut harness, &mirror);
    assert!(enabled(&harness, "Clear the thumbnails"));
    harness.get_by_label("These are being cleared already.");
    mirror.sent(PIPELINES, 10);
    let error = Box::new(SessionError::NoCache);
    mirror.apply(&Event::Failed(SessionError::Refused { command: 10, error }));
    assert_eq!(
        mirror.view(&folders()).refused,
        Some("settings-storage-refused-no-cache")
    );
    // A refusal of another command is not the page's.
    let mut other = Mirror::default();
    let error = Box::new(SessionError::NoCache);
    other.apply(&Event::Failed(SessionError::Refused { command: 3, error }));
    assert_eq!(other.view(&folders()).refused, None);
}

/// The page asks for the sizes each time it is shown: as it opens, and after another
/// page.
#[test]
fn the_page_asks_for_the_sizes_each_time_it_is_shown() {
    let mut harness = page(sample());
    assert_eq!(harness.state().asked, [Asked::Sizes]);
    harness.run_steps(5);
    assert_eq!(harness.state().asked, [Asked::Sizes], "once while shown");
    harness.get_by_label("General").click();
    harness.run();
    harness.get_by_label("Storage").click();
    harness.run();
    harness.run_steps(3);
    assert_eq!(harness.state().asked, [Asked::Sizes, Asked::Sizes]);
}

/// A path too long for the page is cut in its middle, so its start and its end show.
#[test]
fn a_long_path_is_cut_in_its_middle() {
    let mut view = sample();
    let long = format!("{HOME}/{}/Apertrust", "a-very-long-folder-name/".repeat(12));
    view.folders.cache = Some(Folder {
        path: PathBuf::from(&long),
        chosen: Chosen::Variable("APERTRUST_CACHE_DIR"),
    });
    let mut harness = page(view);
    harness.event(Input::PointerGone);
    harness.run();
    // Its whole is still its name for screen readers.
    let shown = crate::export::shown(Path::new(&long), Some(Path::new(HOME)));
    harness.get_by_label(&shown);
    harness.get_by_label("Chosen by APERTRUST_CACHE_DIR");
    let ctx = harness.ctx.clone();
    let font = FontId::proportional(13.0);
    let painter = egui::Painter::new(ctx, egui::LayerId::background(), Rect::EVERYTHING);
    let cut = fitted(&painter, &shown, &font, 200.0, Cut::Middle);
    assert!(
        cut.starts_with("~/a-very") && cut.ends_with("/Apertrust"),
        "{cut}"
    );
    assert!(cut.contains('…'));
    assert_eq!(
        fitted(&painter, "short", &font, 200.0, Cut::Middle),
        "short"
    );
}
