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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::Wake;

use apertrust_ui::{Builtin, Locale};
use eframe::egui::accesskit::Toggled;
use eframe::egui::{Event, OutputCommand};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use tracing::Dispatch;

use super::*;
use crate::dialog_host;

/// The window as the host draws it, and what it asked.
struct Bench {
    window: Window,
    asked: Vec<Asked>,
}

/// The harness's margin round what it draws, on each side.
const MARGIN: f32 = 8.0;

/// The window over `kept`, on the Mac, in the dark theme, at its design's size.
fn window_of(kept: &Kept) -> Harness<'static, Bench> {
    window_at(kept, SIZE)
}

/// The window over `kept` as [`window_of`] draws it, its content `size` points.
fn window_at(kept: &Kept, size: egui::Vec2) -> Harness<'static, Bench> {
    let mut harness = Harness::builder()
        .with_size(size + vec2(2.0 * MARGIN, 2.0 * MARGIN))
        .with_os(OperatingSystem::Mac)
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let drawn = dialog_host::draw(ui, &mut bench.window);
                if let Some(Output::Log(asked)) = drawn.output {
                    bench.asked.push(asked);
                }
            },
            Bench {
                window: Window::new(kept.clone(), None),
                asked: Vec::new(),
            },
        );
    harness.run();
    harness
}

/// A log of its own, keeping `chosen`'s domains, and the dispatch that logs to it.
fn kept(chosen: Option<&[Domain]>) -> (Kept, Dispatch, MutexGuard<'static, ()>) {
    let serial = serial();
    let (kept, dispatch) = Kept::scoped::<&str>(&[]).expect("a log");
    kept.keep(chosen, String::new);
    (kept, dispatch, serial)
}

/// Held while a test's log is in use: tracing's callsites keep one interest for every
/// dispatcher, rebuilt as a filter changes, and a rebuild in another test meanwhile can
/// lose an event of this one's.
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A session's lines as the terminal writes them, of known seconds.
const LINES: &[(&str, Severity)] = &[
    (
        "    0.0412 [settings] settings: ~/Library/Application Support/Apertrust/settings.toml",
        Severity::Debug,
    ),
    (
        "    0.1301 [gpu] adapter: Apple M4 (Metal, integrated)  <- chosen",
        Severity::Debug,
    ),
    (
        "    0.3120 [control] job 1 added: listing 2026-10 Lisbon",
        Severity::Debug,
    ),
    (
        "    0.4433 [raw] Canon EOS 5D Mark IV: 6880×4544 RGGB, active: 6744×4502 at 136,42",
        Severity::Debug,
    ),
    (
        "    0.5361 [perf] decoded B13A0729.CR2 in 277 ms",
        Severity::Debug,
    ),
    (
        "    0.6007 [pipe] plan: viewport, 14 ops, 31 passes, 1912×1275 RGBA",
        Severity::Debug,
    ),
    (
        "    0.6200 [raw] warning: R0001176.DNG: ARW makernote not found",
        Severity::Warning,
    ),
    (
        "    0.7012 [perf] rendered 22 passes from rawprepare in 164.0 ms, GPU: 98.0 ms",
        Severity::Debug,
    ),
    (
        "    2.4140 [event] sent: Rendered, frame 3",
        Severity::Debug,
    ),
    (
        "    2.4410 [perf] rendered 22 passes from rawprepare in 21.0 ms, GPU: 14.0 ms",
        Severity::Debug,
    ),
    (
        "    5.9022 [control] error: export of R0001180.DNG failed: the folder ~/Pictures/out is read-only",
        Severity::Error,
    ),
    (
        "    6.0884 [perf] decoded P1001482.RW2 in 71 ms",
        Severity::Debug,
    ),
];

/// `kept` with [`LINES`] in it.
fn with_lines(kept: &Kept) {
    for (text, severity) in LINES {
        kept.keep_written(text, *severity);
    }
}

fn toggled(harness: &Harness<'_, Bench>, name: &str) -> (bool, bool) {
    let node = harness.get_by_role_and_label(Role::CheckBox, name);
    let node = node.accesskit_node();
    (node.toggled() == Some(Toggled::True), node.is_disabled())
}

#[test]
fn logging_off_it_says_so_and_opens_diagnostics_settings() {
    let (kept, _, _serial) = kept(None);
    let mut harness = window_of(&kept);
    harness.get_by_label("No log is being kept");
    harness.get_by_label(
        "Turn it on in Settings, and lines are kept from then on, in memory, for this session.",
    );
    assert!(harness.query_by_label("Copy to clipboard").is_none());
    harness.get_by_label("Open Diagnostics settings…").click();
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Settings]);
}

/// A line kept wakes the window, once until it reads again, and it then shows it; turning
/// logging on or off shows at once too.
#[test]
fn a_line_kept_wakes_the_window_and_logging_turned_on_shows() {
    struct Count(AtomicUsize);
    impl Wake for Count {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let (kept, dispatch, _serial) = kept(None);
    let mut harness = window_of(&kept);
    let count = Arc::new(Count(AtomicUsize::new(0)));
    harness
        .state_mut()
        .window
        .wake_with(Waker::from(count.clone()));
    kept.keep(Some(&[Domain::PERF]), String::new);
    assert_eq!(count.0.load(Ordering::Relaxed), 1);
    harness.run();
    harness.get_by_label("Copy to clipboard");
    tracing::dispatcher::with_default(&dispatch, || {
        tracing::debug!(target: "perf", "decoded B13A0729.CR2 in 277 ms");
        tracing::debug!(target: "perf", "rendered 22 passes in 682.5 ms");
    });
    assert_eq!(count.0.load(Ordering::Relaxed), 2);
    harness.run();
    let text = harness.state().window.text();
    assert!(
        text.ends_with("[perf] rendered 22 passes in 682.5 ms\n"),
        "{text}"
    );
    // Gone, the window is woken no more.
    drop(harness);
    tracing::dispatcher::with_default(&dispatch, || {
        tracing::debug!(target: "perf", "rendered again");
    });
    assert_eq!(count.0.load(Ordering::Relaxed), 2);
}

/// The checkboxes filter what is shown among what is kept; a domain not kept is disabled,
/// saying where to keep it.
#[test]
fn the_checkboxes_filter_what_is_shown_and_a_domain_not_kept_is_disabled() {
    let domains = [
        Domain::GPU,
        Domain::RAW,
        Domain::PIPE,
        Domain::PERF,
        Domain::SETTINGS,
        Domain::CONTROL,
        Domain::EVENT,
    ];
    let (kept, _, _serial) = kept(Some(&domains));
    with_lines(&kept);
    let mut harness = window_of(&kept);
    assert_eq!(toggled(&harness, "Warnings and errors"), (true, false));
    assert_eq!(toggled(&harness, "Raw"), (true, false));
    assert_eq!(toggled(&harness, "Memory"), (false, true));
    assert_eq!(toggled(&harness, "Verbose"), (false, true));
    harness.get_by_label("Memory").hover();
    harness.run_steps(10);
    harness.get_by_label("Not kept: turn it on in Settings › General › Diagnostics");
    // Every line is shown, as kept, a row each.
    let all = harness.state().window.text();
    let expected: String = LINES.iter().map(|(line, _)| format!("{line}\n")).collect();
    assert_eq!(all, expected);
    harness.get_by_label(LINES[4].0);
    // Raw off: its lines go, but its warning, which warnings show.
    harness.get_by_role_and_label(Role::CheckBox, "Raw").click();
    harness.run();
    let text = harness.state().window.text();
    assert!(!text.contains("[raw] Canon"), "{text}");
    assert!(text.contains("[raw] warning:"), "{text}");
    assert!(harness.query_by_label(LINES[3].0).is_none());
    // Warnings off: the warning and the error go too.
    harness
        .get_by_role_and_label(Role::CheckBox, "Warnings and errors")
        .click();
    harness.run();
    let text = harness.state().window.text();
    assert!(
        !text.contains("warning:") && !text.contains("error:"),
        "{text}"
    );
    harness.get_by_label("12 lines kept · 9 shown · this session only, in memory");
    // No setting changed.
    assert!(harness.state().asked.is_empty());
}

/// Copy to clipboard copies the lines shown as the terminal writes them, and Save to file…
/// hands the same text to the app.
#[test]
fn copy_and_save_take_the_lines_shown_as_the_terminal_writes_them() {
    let (kept, _, _serial) = kept(Some(&[Domain::PERF]));
    with_lines(&kept);
    let mut harness = window_of(&kept);
    let shown = "    0.5361 [perf] decoded B13A0729.CR2 in 277 ms\n";
    let text = harness.state().window.text();
    assert!(text.starts_with(shown), "{text}");
    harness.get_by_label("Copy to clipboard").click();
    harness.step();
    let copied: Vec<&OutputCommand> = harness.output().platform_output.commands.iter().collect();
    assert!(
        copied.contains(&&OutputCommand::CopyText(text.clone())),
        "{copied:?}"
    );
    harness.run();
    harness.get_by_label("Save to file…").click();
    harness.run();
    assert_eq!(harness.state().asked, [Asked::Save(text)]);
}

/// The view follows the end while it is there; scrolled up it stays, and the pill offers
/// the lines that came meanwhile, going back to the end.
#[test]
fn the_view_follows_the_end_and_the_pill_offers_newer_lines() {
    let (kept, dispatch, _serial) = kept(Some(&[Domain::PERF]));
    let log = |from: usize, to: usize| {
        tracing::dispatcher::with_default(&dispatch, || {
            for i in from..to {
                tracing::debug!(target: "perf", "rendered frame {i}");
            }
        });
    };
    log(0, 60);
    let mut harness = window_of(&kept);
    // At the end: the newest line is in view, the oldest is not drawn at all.
    harness.get_by_label_contains("rendered frame 59");
    assert!(
        harness
            .query_by_label_contains("rendered frame 0")
            .is_none()
    );
    log(60, 62);
    harness.run();
    harness.get_by_label_contains("rendered frame 61");
    assert!(harness.query_by_label_contains("newer line").is_none());
    // Scrolled up, it stays, and the pill counts what came.
    let lines = harness.get_by_role(Role::Log).rect().center();
    harness.hover_at(lines);
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 2000.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    // The wheel scrolls smoothly, over some frames.
    harness.run_steps(60);
    harness.get_by_label_contains("rendered frame 0");
    log(62, 65);
    harness.run();
    assert!(
        harness
            .query_by_label_contains("rendered frame 64")
            .is_none()
    );
    harness.get_by_label("↓ 3 newer lines").click();
    harness.run();
    harness.get_by_label_contains("rendered frame 64");
    assert!(harness.query_by_label_contains("newer line").is_none());
    // The footer stays under the lines.
    let footer = harness.get_by_label("Copy to clipboard").rect();
    assert!(footer.top() > harness.get_by_role(Role::Log).rect().bottom() - 1.0);
}

/// The footer's numbers in the language's format, their digits grouped.
#[test]
fn the_counts_are_written_in_the_language_s_number_format() {
    let counts_in = |locale| counts(&apertrust_ui::Translator::new(locale), 1284, 1107);
    assert_eq!(
        counts_in(Locale::EnGb),
        "1,284 lines kept · 1,107 shown · this session only, in memory"
    );
    // Ukrainian groups with a space; Polish, as CLDR has it, not a number of four digits.
    let uk = counts_in(Locale::Uk);
    assert!(uk.contains("284") && !uk.contains("1284"), "{uk}");
    assert!(counts_in(Locale::Pl).contains("1284 · pokazane: 1107"));
    assert_eq!(
        counts(&apertrust_ui::Translator::new(Locale::EnGb), 1, 1),
        "1 line kept · 1 shown · this session only, in memory"
    );
}

#[test]
fn the_saved_log_is_named_by_the_session_s_start_in_the_last_folder_or_downloads() {
    let started = DateTime {
        year: 2026,
        month: 11,
        day: 7,
        hour: 9,
        minute: 5,
        second: 59,
        microsecond: 0,
    };
    assert_eq!(file_name(&started), "apertrust-log-2026-11-07-09-05.txt");
    let (last, downloads) = (Path::new("/photos/logs"), Path::new("/Users/you/Downloads"));
    assert_eq!(
        save_folder(Some(last), Some(downloads)),
        Some(last.to_owned())
    );
    assert_eq!(
        save_folder(None, Some(downloads)),
        Some(downloads.to_owned())
    );
    assert_eq!(save_folder(None, None), None);
}

// How it looks.

#[test]
fn the_domain_column_holds_the_longest_domain() {
    for domain in apertrust_base::log::DOMAINS {
        let bracket = format!("[{}]", domain.name);
        assert!(bracket.chars().count() <= DOMAIN, "{bracket}");
    }
}

#[test]
fn the_log_window_in_the_dark_theme() {
    let domains = [
        Domain::GPU,
        Domain::RAW,
        Domain::PIPE,
        Domain::PERF,
        Domain::SETTINGS,
        Domain::CONTROL,
        Domain::EVENT,
    ];
    let (kept, _, _serial) = kept(Some(&domains));
    kept.keep_written(
        "Apertrust 0.1.0+12~g28d396c [macos aarch64 release]",
        Severity::Debug,
    );
    with_lines(&kept);
    let mut harness = window_of(&kept);
    harness.event(Event::PointerGone);
    harness.run();
    harness.snapshot("log-window-dark");
}

#[test]
fn the_log_window_with_logging_off_in_the_dark_theme() {
    let (kept, _, _serial) = kept(None);
    let mut harness = window_at(&kept, LEAST);
    harness.event(Event::PointerGone);
    harness.run();
    harness.snapshot("log-window-off-dark");
}

/// Resized, the window's parts fit at its least size and fill a large one: the filters
/// wrap inside it, the lines take the height left, and the footer stays one line, its
/// note giving way first and its buttons whole.
#[test]
fn the_window_lays_out_at_its_least_size_and_at_a_large_one() {
    let domains = [
        Domain::GPU,
        Domain::RAW,
        Domain::PIPE,
        Domain::PERF,
        Domain::SETTINGS,
        Domain::CONTROL,
        Domain::EVENT,
    ];
    let filters = [
        "Warnings and errors",
        "GPU",
        "Raw",
        "Pipe",
        "Performance",
        "Memory",
        "Settings",
        "Control",
        "Events",
        "Verbose",
    ];
    for size in [LEAST, vec2(1600.0, 1000.0)] {
        let (kept, _, _serial) = kept(Some(&domains));
        with_lines(&kept);
        let mut harness = window_at(&kept, size);
        harness.run();
        let window = Rect::from_min_size(pos2(MARGIN, MARGIN), size);
        let inside = |rect: Rect| window.expand(0.5).contains_rect(rect);
        let mut lowest = window.top();
        for name in filters {
            let rect = harness.get_by_role_and_label(Role::CheckBox, name).rect();
            assert!(inside(rect), "{size:?}: {name} at {rect:?}");
            lowest = lowest.max(rect.bottom());
        }
        let save = harness.get_by_label("Save to file…").rect();
        let copy = harness.get_by_label("Copy to clipboard").rect();
        assert!(inside(save) && inside(copy), "{size:?}: {save:?} {copy:?}");
        assert_eq!(save.center().y, copy.center().y, "{size:?}: one line");
        assert!(save.right() <= copy.left(), "{size:?}: in order");
        let lines = harness.get_by_role(Role::Log).rect();
        assert!(
            lines.top() >= lowest,
            "{size:?}: the lines under the filters"
        );
        assert!(
            lines.bottom() <= save.top(),
            "{size:?}: and over the footer"
        );
        assert!(lines.height() >= 3.0 * ROW, "{size:?}");
        assert!(lines.width() >= size.x - 1.0, "{size:?}: across the window");
        // The footer at the window's foot: the lines took the rest.
        assert!(
            window.bottom() - save.bottom() <= 12.0,
            "{size:?}: {save:?}"
        );
        let note = harness.query_by_label("Lines name your files and folders.");
        assert_eq!(
            note.is_some(),
            size.x > LEAST.x,
            "{size:?}: the note gives way"
        );
        if size == LEAST {
            harness.snapshot("log-window-least-dark");
        }
    }
    // Two rows of filters at the least size, one at the design's.
    let row_count = |size| {
        let (kept, _, _serial) = kept(Some(&domains));
        let harness = window_at(&kept, size);
        let middles = filters.iter().map(|name| {
            let rect = harness.get_by_role_and_label(Role::CheckBox, name).rect();
            rect.center().y
        });
        // A new row wherever a box sits a line lower than the one before.
        let mut rows = 0;
        let mut last = f32::MIN;
        for middle in middles {
            if middle > last + ROW / 2.0 {
                rows += 1;
            }
            last = middle;
        }
        rows
    };
    assert_eq!(row_count(SIZE), 1);
    assert!(row_count(LEAST) >= 2);
}
