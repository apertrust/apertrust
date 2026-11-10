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

/// The dialog as the host draws it, what the page asked, and every output in order.
struct Bench {
    content: Settings,
    asked: Vec<Asked>,
    outputs: Vec<Output>,
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
                for output in drawn.outputs {
                    if let Output::Storage(asked) = &output {
                        bench.asked.push(asked.clone());
                    }
                    bench.outputs.push(output);
                }
            },
            Bench {
                content,
                asked: Vec::new(),
                outputs: Vec::new(),
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
        "0 pipelines, 0.0 kB",
        "Library caches",
        "1 library, 5.0 kB",
        "120 kB of 4.0 GB",
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
        harness
            .state_mut()
            .content
            .set_storage(mirror.view(&folders()));
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

fn cleared(job: JobId, state: State) -> Event {
    ended(job, state, None)
}

/// A refusal's words go with the next sizes, or with the end of a Clear the page tracks.
#[test]
fn a_refusal_s_words_go_with_the_next_sizes_or_a_clear_s_end() {
    let refuse = |mirror: &mut Mirror, command| {
        let error = Box::new(SessionError::CacheClearing { kind: THUMBNAILS });
        mirror.apply(&Event::Failed(SessionError::Refused { command, error }));
    };
    let mut mirror = Mirror::default();
    mirror.sent(THUMBNAILS, 1);
    refuse(&mut mirror, 1);
    assert!(mirror.view(&folders()).refused.is_some());
    mirror.apply(&Event::Cache(CacheEvent::NoCache));
    assert_eq!(mirror.view(&folders()).refused, None);

    // A Clear of pipelines runs while thumbnails' is refused; its end takes the words.
    let job = ids(1)[0];
    mirror.sent(PIPELINES, 2);
    mirror.apply(&added(job, Queue::UserBackground, Some(2)));
    mirror.sent(THUMBNAILS, 3);
    refuse(&mut mirror, 3);
    assert!(mirror.view(&folders()).refused.is_some());
    mirror.apply(&cleared(job, State::Finished));
    assert_eq!(mirror.view(&folders()).refused, None);
    // Another job's end leaves them.
    mirror.sent(THUMBNAILS, 4);
    refuse(&mut mirror, 4);
    let other = ids(2)[1];
    mirror.apply(&cleared(other, State::Finished));
    assert!(mirror.view(&folders()).refused.is_some());
}

/// A Clear's job ending cancelled or failed, and the engine stopping or panicking, each
/// enable its Clear again.
#[test]
fn every_end_of_a_clear_enables_it_again() {
    let ends = [
        cleared(ids(1)[0], State::Cancelled),
        ended(
            ids(1)[0],
            State::Finished,
            Some(apertrust_engine::jobs::Failure::new("cache-remove", "no")),
        ),
    ];
    for end in ends {
        let Event::Job {
            event: JobEvent::Ended { job, .. },
            ..
        } = &end
        else {
            unreachable!();
        };
        let mut mirror = Mirror::default();
        mirror.sent(THUMBNAILS, 7);
        mirror.apply(&added(*job, Queue::UserBackground, Some(7)));
        assert_eq!(mirror.view(&folders()).clearing, [THUMBNAILS]);
        mirror.apply(&end);
        assert!(mirror.view(&folders()).clearing.is_empty(), "{end:?}");
    }
    for stopped in [
        SessionError::Stopped,
        SessionError::Panicked("a bug".to_owned()),
    ] {
        let mut mirror = Mirror::default();
        mirror.sent(THUMBNAILS, 7);
        mirror.apply(&Event::Failed(stopped));
        assert!(mirror.view(&folders()).clearing.is_empty());
    }
}

/// A Clear the page did not send, as the jobs window's Try again starts, is told by its
/// job's kind and the cache kind its title names: its kind's Clear stays disabled until it
/// ends.
#[test]
fn a_clear_another_command_started_disables_its_kind_too() {
    let mut mirror = Mirror::default();
    let job = ids(1)[0];
    let title = apertrust_engine::jobs::Message::new("job-cache-clear").with("kind", "pipelines");
    let added = Event::Job {
        event: JobEvent::Added {
            job,
            kind: "cache-clear",
            queue: Some(Queue::UserBackground),
            title,
            cancellable: true,
            destination: None,
            detail: None,
            progress: apertrust_engine::jobs::Progress::default(),
            retries: Some(ids(1)[0]),
        },
        command: Some(31),
    };
    mirror.apply(&added);
    assert_eq!(mirror.view(&folders()).clearing, [PIPELINES]);
    mirror.apply(&cleared(job, State::Finished));
    assert!(mirror.view(&folders()).clearing.is_empty());
    // Any other job is not a Clear.
    mirror.apply(&crate::jobs::tests::added(job, Queue::UserBackground, None));
    assert!(mirror.view(&folders()).clearing.is_empty());
}

/// The page's ask goes to the session, and the number the session gave the Clear is the
/// one its button waits on.
#[test]
fn a_clear_asked_keeps_the_number_the_session_gave_it() {
    let mut sent = Vec::new();
    let mut send = |command| {
        sent.push(command);
        41 + sent.len() as u64
    };
    let mut mirror = Mirror::default();
    act(Asked::Sizes, &mut send, &mut mirror);
    act(Asked::Clear(THUMBNAILS), &mut send, &mut mirror);
    assert_eq!(
        sent,
        [
            Command::Cache(CacheCommand::Sizes),
            Command::Cache(CacheCommand::Clear { kind: THUMBNAILS }),
        ]
    );
    let job = ids(1)[0];
    mirror.apply(&added(job, Queue::UserBackground, Some(43)));
    mirror.apply(&cleared(job, State::Finished));
    assert!(
        mirror.view(&folders()).clearing.is_empty(),
        "job 43 was its"
    );
}

/// How each folder was chosen maps to the page's words: the flag, the variable, the user
/// data folder, the system's.
#[test]
fn how_each_folder_was_chosen_is_the_page_s() {
    assert_eq!(
        user_data_chosen(folders::Chosen::Flag("--configdir")),
        Chosen::Flag("--configdir")
    );
    assert_eq!(
        user_data_chosen(folders::Chosen::Variable),
        Chosen::Variable("APERTRUST_USER_DATA_DIR")
    );
    assert_eq!(
        user_data_chosen(folders::Chosen::Platform),
        Chosen::Platform
    );
    assert_eq!(
        cache_chosen(folders::CacheChosen::Flag("--cachedir")),
        Chosen::Flag("--cachedir")
    );
    assert_eq!(
        cache_chosen(folders::CacheChosen::Variable),
        Chosen::Variable("APERTRUST_CACHE_DIR")
    );
    assert_eq!(
        cache_chosen(folders::CacheChosen::UserData),
        Chosen::UserData
    );
    assert_eq!(
        cache_chosen(folders::CacheChosen::Platform),
        Chosen::Platform
    );
    let none = Folders::of(None, Some(PathBuf::from(HOME)));
    assert_eq!((none.user_data, none.cache), (None, None));
}

/// A second press before the app has handed the view asks nothing more: the button is
/// disabled from the press, until the view comes.
#[test]
fn a_fast_second_press_asks_nothing_more() {
    let mut harness = page(sample());
    harness.state_mut().asked.clear();
    harness.get_by_label("Clear the thumbnails").click();
    harness.run();
    assert!(!enabled(&harness, "Clear the thumbnails"));
    harness.get_by_label("Clear the thumbnails").click();
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Clear(THUMBNAILS)]);
    // The view the app hands next decides: here none is clearing.
    harness.state_mut().content.set_storage(sample());
    harness.run();
    assert!(enabled(&harness, "Clear the thumbnails"));
}

/// A change and the page's ask in one frame both reach the app, in order: choosing Storage
/// keeps the page and asks for the sizes.
#[test]
fn a_change_and_an_ask_in_one_frame_both_arrive_in_order() {
    let mut harness = page(sample());
    harness.get_by_label("General").click();
    harness.run();
    harness.state_mut().outputs.clear();
    harness.get_by_label("Storage").click();
    harness.run();
    let outputs = &harness.state().outputs;
    assert!(
        matches!(
            outputs.as_slice(),
            [
                Output::Settings(settings_dialog::Change::Set("settings_window.page", _)),
                Output::Storage(Asked::Sizes),
            ]
        ),
        "{outputs:?}"
    );
}

/// As its window comes back to the front on Storage, the page asks for the sizes again.
#[test]
fn the_page_asks_again_as_its_window_comes_back() {
    let mut harness = page(sample());
    assert_eq!(harness.state().asked, [Asked::Sizes]);
    harness.input_mut().focused = false;
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Sizes]);
    harness.input_mut().focused = true;
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Sizes, Asked::Sizes]);
}

/// Sizes below 10 are written with their decimal, in each language's numbers, and the
/// counts take each language's forms.
#[test]
fn sizes_below_ten_keep_their_decimal_in_every_language() {
    let say = |locale, kind: Kind, entries: u64, bytes: u64| {
        usage(&Translator::new(locale), kind, entries, bytes)
    };
    assert_eq!(
        say(Locale::EnGb, THUMBNAILS, 1, 1_000_000),
        "1 thumbnail, 1.0 MB"
    );
    assert_eq!(
        say(Locale::EnGb, THUMBNAILS, 2, 5_000_000),
        "2 thumbnails, 5.0 MB"
    );
    assert_eq!(
        say(Locale::EnGb, THUMBNAILS, 2, 50_000_000),
        "2 thumbnails, 50 MB"
    );
    assert_eq!(
        say(Locale::Uk, THUMBNAILS, 1, 1_500_000),
        "1 мініатюра, 1,5 МБ"
    );
    assert_eq!(
        say(Locale::Uk, THUMBNAILS, 3, 1_500_000),
        "3 мініатюри, 1,5 МБ"
    );
    assert_eq!(
        say(Locale::Uk, THUMBNAILS, 5, 5_000_000),
        "5 мініатюр, 5,0 МБ"
    );
    assert_eq!(
        say(Locale::Pl, THUMBNAILS, 1, 1_500_000),
        "1 miniatura, 1,5 MB"
    );
    assert_eq!(
        say(Locale::Pl, THUMBNAILS, 3, 1_500_000),
        "3 miniatury, 1,5 MB"
    );
    assert_eq!(
        say(Locale::Pl, LIBRARIES, 5, 5_000_000),
        "5 bibliotek, 5,0 MB"
    );
    let total = |locale| {
        total(
            &Translator::new(locale),
            1_500_000,
            Limit::Bytes(4_000_000_000),
        )
    };
    assert_eq!(total(Locale::EnGb), "1.5 MB of 4.0 GB");
    assert_eq!(total(Locale::Uk), "1,5 МБ із 4,0 ГБ");
}
