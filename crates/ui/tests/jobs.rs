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

//! The jobs: the spinner in the top panel and the jobs window's content, for each state,
//! the actions they report, what screen readers hear and how the panel keeps the spinner
//! in place (`docs/gui.md`, The jobs window; decision 57), driven through AccessKit.

use std::time::SystemTime;

use apertrust_ui::components::{
    ActivityAction, Available, FileManager, Job, JobFailure, JobMessage, JobPhase, Jobs, SaveState,
    TopPanel, TopPanelAction, kind_icon,
};
use apertrust_ui::{Icon, Locale, Theme, Translator};
use egui::accesskit::{Live, Role};
use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const FOLDER: &str = "~/Pictures/apertrust_exported";

/// 14:02 on the local clock, whatever the time zone the tests run in.
fn two() -> SystemTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 14)
        .and_then(|day| day.and_hms_opt(14, 2, 0))
        .and_then(|at| at.and_local_timezone(chrono::Local).single())
        .expect("a local time")
        .into()
}

fn export_message(done: usize, total: usize) -> JobMessage {
    JobMessage::new("job-export-photo")
        .number("number", done as f64)
        .number("total", total as f64)
        .text("storage", "disk")
}

/// What an export's outcome counts: `written` of 12.
fn outcome(id: &str, written: usize) -> JobMessage {
    JobMessage::new(id)
        .number("written", written as f64)
        .number("skipped", 0.0)
        .number("total", 12.0)
}

/// An export of 12 photos with id `id`, in `phase`, 3 in, as the gallery has it: the parts
/// the export job reports of itself.
fn export(id: u64, phase: JobPhase) -> Job {
    let done = match phase {
        JobPhase::Queued => 0,
        JobPhase::Running | JobPhase::Cancelled => 3,
        JobPhase::Finished => 12,
        JobPhase::Failed => 11,
    };
    let job = Job::new(id, phase, export_message(done, 12))
        .kind("export")
        .progress(done as u64, Some(12))
        .destination(FOLDER);
    match phase {
        JobPhase::Queued => job.cancellable(true).detail(
            JobMessage::new("job-export-queued")
                .number("count", 12.0)
                .text("folder", FOLDER),
        ),
        JobPhase::Running => job.cancellable(true).detail(
            JobMessage::new("job-export-now")
                .number("done", 3.0)
                .number("total", 12.0)
                .text("photo", "IMG_2044.ARW"),
        ),
        JobPhase::Finished => job.outcome(outcome("job-export-finished", 12)),
        JobPhase::Failed => job
            .outcome(outcome("job-export-failed-outcome", 11))
            .failure(JobFailure {
                item: "IMG_2047.ARW".to_owned(),
                file: "IMG_2047.jpg".to_owned(),
                cause: "the disk is full".to_owned(),
            })
            .retry(true),
        JobPhase::Cancelled => job.outcome(outcome("job-export-cancelled", 3)),
    }
}

/// A job that is not an export: thumbnails, or a delete that cannot be stopped.
fn other(id: u64, phase: JobPhase, cancellable: bool) -> Job {
    Job::new(id, phase, JobMessage::new("job-thumbnails"))
        .kind("thumbnails")
        .progress(6, Some(10))
        .cancellable(cancellable)
}

/// What the harness stands in for: the app's mirror of the session's jobs, and the actions
/// the panel reported.
struct Run {
    jobs: Vec<Job>,
    saved: SaveState<'static>,
    actions: Vec<TopPanelAction>,
}

fn panel_in(locale: Locale, width: f32, jobs: Vec<Job>) -> Harness<'static, Run> {
    Harness::builder()
        .with_size(vec2(width, 400.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, run: &mut Run| {
                let jobs = run.jobs.clone();
                let action = TopPanel::new()
                    .available(Available {
                        redo: false,
                        apply: false,
                        ..Available::default()
                    })
                    .saved(run.saved)
                    .title_bar(true)
                    .jobs(Jobs::new(&jobs).file_manager(FileManager::Finder))
                    .show(ui);
                if let Some(action) = action {
                    run.actions.push(action);
                }
            },
            Run {
                jobs,
                saved: SaveState::Saved,
                actions: Vec::new(),
            },
        )
}

fn panel(jobs: Vec<Job>) -> Harness<'static, Run> {
    panel_in(Locale::EnGb, 1200.0, jobs)
}

/// The spinner: the one button named Jobs.
fn spinner<'a>(harness: &'a Harness<'_, Run>) -> egui_kittest::Node<'a> {
    spinner_named(harness, "Jobs")
}

fn spinner_named<'a>(harness: &'a Harness<'_, Run>, name: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, name)
}

fn spinners(harness: &Harness<'_, Run>) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, "Jobs")
        .count()
}

fn value(node: &egui_kittest::Node<'_>) -> Option<String> {
    node.accesskit_node().value()
}

#[test]
fn the_spinner_is_one_button_named_jobs_with_the_count_in_its_value() {
    let cases: [(&str, Vec<Job>, &str); 5] = [
        (
            "two running",
            vec![
                other(1, JobPhase::Running, true),
                export(2, JobPhase::Running),
            ],
            "2 running",
        ),
        (
            "one running, one waiting",
            vec![
                export(1, JobPhase::Running),
                export(2, JobPhase::Queued),
                other(3, JobPhase::Finished, false),
            ],
            "1 running, 1 waiting",
        ),
        ("waiting", vec![export(1, JobPhase::Queued)], "1 waiting"),
        (
            "failed",
            vec![export(1, JobPhase::Failed), export(2, JobPhase::Finished)],
            "1 failed",
        ),
        (
            "ended, then another starts",
            vec![export(1, JobPhase::Failed), export(2, JobPhase::Running)],
            "1 running",
        ),
    ];
    for (what, jobs, said) in cases {
        let mut harness = panel(jobs);
        harness.run();
        assert_eq!(spinners(&harness), 1, "{what}");
        assert_eq!(value(&spinner(&harness)).as_deref(), Some(said), "{what}");
    }

    // The name stays while jobs come, run and end; only the value follows them.
    let mut harness = panel(Vec::new());
    harness.run();
    assert_eq!(spinners(&harness), 0, "no job, no spinner");
    for jobs in [
        vec![export(1, JobPhase::Queued)],
        vec![export(1, JobPhase::Running)],
        vec![
            export(1, JobPhase::Running),
            other(2, JobPhase::Running, true),
        ],
        vec![
            export(1, JobPhase::Failed),
            other(2, JobPhase::Finished, false),
        ],
    ] {
        harness.state_mut().jobs = jobs;
        harness.run();
        assert_eq!(spinners(&harness), 1);
        assert_eq!(
            spinner(&harness).accesskit_node().label().as_deref(),
            Some("Jobs")
        );
    }
}

#[test]
fn tab_reaches_the_spinner_and_enter_asks_for_the_window() {
    let mut harness = panel(vec![export(1, JobPhase::Running)]);
    harness.run();
    let mut reached = false;
    for _ in 0..30 {
        harness.key_press(Key::Tab);
        harness.run();
        if spinner(&harness).accesskit_node().is_focused() {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab never reached the spinner");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().actions, vec![TopPanelAction::Jobs]);
    // A click does the same, and a second asks again, to bring the window forward.
    spinner(&harness).click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![TopPanelAction::Jobs, TopPanelAction::Jobs]
    );
}

/// The live region: a polite status.
fn live<'a>(harness: &'a Harness<'_, Run>) -> egui_kittest::Node<'a> {
    harness
        .query_all_by_role(Role::Status)
        .find(|n| n.accesskit_node().data().live() == Some(Live::Polite))
        .expect("no live region")
}

fn said(harness: &Harness<'_, Run>) -> String {
    live(harness)
        .accesskit_node()
        .label()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_polite_live_region_speaks_when_a_job_starts_and_ends_not_as_it_progresses() {
    let mut harness = panel(Vec::new());
    harness.run();
    // There from the start, saying nothing.
    assert_eq!(said(&harness), "");

    harness.state_mut().jobs = vec![export(1, JobPhase::Running)];
    harness.run();
    let start = said(&harness);
    assert_eq!(start, "Started: Exporting 3 / 12 to file on disk");

    // Photo after photo it says the same, so nothing is spoken again.
    for done in 4..12 {
        let mut running = export(1, JobPhase::Running);
        running.title = export_message(done, 12);
        running.detail = Some(
            JobMessage::new("job-export-now")
                .number("done", done as f64)
                .number("total", 12.0)
                .text("photo", format!("IMG_{}.ARW", 2040 + done)),
        );
        running.done = done as u64;
        harness.state_mut().jobs = vec![running];
        harness.run();
        assert_eq!(said(&harness), start, "{done}");
    }

    // A second job starts: its words, in the jobs' own message.
    harness.state_mut().jobs = vec![
        export(1, JobPhase::Running),
        other(2, JobPhase::Running, true),
    ];
    harness.run();
    assert_eq!(said(&harness), "Started: Creating thumbnails");

    // A job ends while another runs, and the end is said, by its outcome, the row's title,
    // which says how it went.
    for (phase, text) in [
        (JobPhase::Finished, "12 photos exported"),
        (JobPhase::Failed, "Export failed: 11 of 12 photos exported"),
        (
            JobPhase::Cancelled,
            "Export cancelled: 3 of 12 photos exported",
        ),
    ] {
        harness.state_mut().jobs = vec![
            export(1, JobPhase::Running),
            other(2, JobPhase::Running, true),
        ];
        harness.run();
        harness.state_mut().jobs = vec![export(1, phase), other(2, JobPhase::Running, true)];
        harness.run();
        assert_eq!(said(&harness), text, "{phase:?}");
    }

    // A job with no outcome says its title and how it ended.
    harness.state_mut().jobs = vec![other(2, JobPhase::Running, true)];
    harness.run();
    harness.state_mut().jobs = vec![other(2, JobPhase::Finished, true)];
    harness.run();
    assert_eq!(said(&harness), "Finished: Creating thumbnails");
    harness.state_mut().jobs = vec![other(3, JobPhase::Running, true)];
    harness.run();
    harness.state_mut().jobs = vec![other(3, JobPhase::Failed, true)];
    harness.run();
    assert_eq!(said(&harness), "Failed: Creating thumbnails");

    // Dismissed, it says nothing, and the region stays.
    harness.state_mut().jobs.clear();
    harness.run();
    assert_eq!(said(&harness), "");
}

#[test]
fn a_job_that_starts_again_under_the_id_it_ended_with_is_heard_to_start() {
    let mut harness = panel(vec![export(1, JobPhase::Running)]);
    harness.run();
    harness.state_mut().jobs = vec![export(1, JobPhase::Failed)];
    harness.run();
    assert_eq!(said(&harness), "Export failed: 11 of 12 photos exported");
    // Try again, with the same id: the start is said, not left unsaid because the id
    // was heard before.
    harness.state_mut().jobs = vec![export(1, JobPhase::Running)];
    harness.run();
    assert_eq!(said(&harness), "Started: Exporting 3 / 12 to file on disk");
}

#[test]
fn when_jobs_end_the_spinner_goes_but_a_failure_leaves_the_warning_in_its_place() {
    let mut harness = panel(vec![export(1, JobPhase::Running)]);
    harness.run();
    let at = spinner(&harness).rect();
    for (jobs, shows) in [
        (vec![export(1, JobPhase::Finished)], false),
        (vec![export(1, JobPhase::Failed)], true),
        (vec![export(1, JobPhase::Cancelled)], false),
        (
            vec![export(1, JobPhase::Cancelled), export(2, JobPhase::Failed)],
            true,
        ),
    ] {
        harness.state_mut().jobs = jobs;
        harness.run();
        assert_eq!(spinners(&harness), usize::from(shows));
        if shows {
            assert_eq!(spinner(&harness).rect(), at);
            assert_eq!(value(&spinner(&harness)).as_deref(), Some("1 failed"));
        }
    }
}

/// How many pixels of a rendered image are within a few steps of `colour` on every channel.
macro_rules! pixels_near {
    ($image:expr, $colour:expr) => {
        $image
            .pixels()
            .filter(|p| {
                p.0.iter()
                    .zip($colour.to_array())
                    .take(3)
                    .all(|(a, b)| a.abs_diff(b) <= 6)
            })
            .count()
    };
}

/// Which symbol is drawn: the ring turns while a job runs, and when they have ended it goes,
/// leaving nothing, no tick, or where one failed the warning sign in `warning`.
#[test]
fn the_spinner_stops_turning_and_leaves_nothing_or_the_warning_when_jobs_end() {
    struct Spin {
        jobs: Vec<Job>,
    }
    let warning = Theme::default().colors.warning;
    let mut harness = Harness::builder()
        .with_size(vec2(120.0, 60.0))
        .with_step_dt(0.1)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, spin: &mut Spin| {
                // The harness zeroes the animation time after the setup; the theme sets it.
                ui.ctx().all_styles_mut(|s| s.animation_time = 0.12);
                Jobs::new(&spin.jobs).spinner(ui);
            },
            Spin {
                jobs: vec![export(1, JobPhase::Running)],
            },
        );
    let frame = |harness: &mut Harness<'_, Spin>| {
        harness.run_steps(3);
        harness.render().expect("render")
    };
    let (a, b) = (frame(&mut harness), frame(&mut harness));
    assert_ne!(a, b, "the ring does not turn while a job runs");
    assert_eq!(pixels_near!(a, warning), 0);

    harness.state_mut().jobs = Vec::new();
    let nothing = frame(&mut harness);
    let mut ended = Vec::new();
    for (phase, warns) in [(JobPhase::Finished, false), (JobPhase::Failed, true)] {
        harness.state_mut().jobs = vec![export(1, phase)];
        let (c, d) = (frame(&mut harness), frame(&mut harness));
        assert_eq!(c, d, "{phase:?}: it keeps turning");
        assert_ne!(c, a, "{phase:?}: the ring is still drawn");
        assert_eq!(pixels_near!(c, warning) > 0, warns, "{phase:?}");
        ended.push(c);
    }
    assert_eq!(ended[0], nothing, "something is left where all finished");
    assert_ne!(ended[1], nothing, "no warning after a failure");
}

/// The save states, from no photo to a failed write.
const SAVE_STATES: [SaveState<'static>; 4] = [
    SaveState::None,
    SaveState::Saved,
    SaveState::Saving,
    SaveState::Failed("disk full"),
];

#[test]
fn saving_never_moves_the_spinner() {
    let mut harness = panel(vec![export(1, JobPhase::Running)]);
    harness.run();
    let at = spinner(&harness).rect();
    for saved in SAVE_STATES {
        harness.state_mut().saved = saved;
        harness.run();
        assert_eq!(spinner(&harness).rect(), at, "{saved:?}");
    }
}

/// What the panel shows of the status items and everything else: the spinner's rectangle,
/// the save state's, and every other control's by name.
type Placed = (
    Option<egui::Rect>,
    Option<egui::Rect>,
    Vec<(String, egui::Rect)>,
);

fn placed(harness: &Harness<'_, Run>) -> Placed {
    let spinner = harness
        .query_by_role_and_label(Role::Button, "Jobs")
        .map(|n| n.rect());
    let save = (harness.query_by_label("Saving…"))
        .or_else(|| harness.query_by_role_and_label(Role::Button, "Not saved"))
        .map(|n| n.rect());
    let others = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| {
            let name = n.accesskit_node().label()?.to_string();
            (name != "Jobs" && name != "Not saved").then(|| (name, n.rect()))
        })
        .collect();
    (spinner, save, others)
}

/// The two status items at the panel's end each keep a slot of their own, as wide as their
/// widest form at the fold, shown or not: neither a job coming and going nor the save state
/// changing moves the other, or anything else, at any fold step.
#[test]
fn the_status_slots_never_move_anything() {
    let mut harness = panel(Vec::new());
    let mut folds = std::collections::BTreeSet::new();
    for width in (150..=1400).rev().step_by(10) {
        harness.set_size(vec2(width as f32, 400.0));
        let mut spinner_at = None;
        let mut others_at = None;
        let mut save_at = std::collections::HashMap::new();
        for jobs in [Vec::new(), vec![export(1, JobPhase::Running)]] {
            for saved in SAVE_STATES {
                harness.state_mut().jobs = jobs.clone();
                harness.state_mut().saved = saved;
                harness.run();
                folds.insert(fold(&harness));
                let (spinner, save, others) = placed(&harness);
                let at = format!("{width} points, {} jobs, {saved:?}", jobs.len());
                assert_eq!(spinner.is_some(), !jobs.is_empty(), "{at}");
                if let Some(spinner) = spinner {
                    assert_eq!(*spinner_at.get_or_insert(spinner), spinner, "{at}");
                }
                let shows = matches!(saved, SaveState::Saving | SaveState::Failed(_));
                assert_eq!(save.is_some(), shows, "{at}");
                if let Some(save) = save {
                    let key = format!("{saved:?}");
                    assert_eq!(*save_at.entry(key).or_insert(save), save, "{at}");
                }
                assert_eq!(*others_at.get_or_insert(others.clone()), others, "{at}");
            }
        }
        // Saving… and the warning end where the slot ends.
        let ends: Vec<f32> = save_at.values().map(|r| r.right()).collect();
        assert!(ends.windows(2).all(|w| w[0] == w[1]), "{width}: {ends:?}");
    }
    // Every fold step was driven: words, icons, ⋯, and Undo under ⋯.
    assert_eq!(folds.len(), 4, "{folds:?}");
}

/// Whether the panel at `width` shows Import's words, ⋯, and Undo.
fn fold(harness: &Harness<'_, Run>) -> (bool, bool, bool) {
    (
        harness
            .query_by_label("Import…")
            .is_some_and(|n| n.rect().width() > 30.0),
        harness.query_by_label("More actions").is_some(),
        harness.query_by_label("Undo").is_some(),
    )
}

#[test]
fn the_spinner_has_no_words_so_it_never_folds_and_leaves_room_to_move_the_window() {
    let parts = [
        "top-panel-more",
        "action-import",
        "action-export",
        "action-reset",
        "action-undo",
        "action-redo",
        "action-copy-edit",
        "action-apply-edit",
    ];
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        let names = parts.map(|id| words.text(id));
        let jobs_name = words.text("jobs-name");
        let mut harness = panel_in(locale, 1400.0, vec![export(1, JobPhase::Running)]);
        for width in (200..=1400).rev().step_by(5) {
            harness.set_size(vec2(width as f32, 400.0));
            // The fold is the same whatever the jobs are doing, so one ending never moves
            // the buttons, and 24 points are left to move the window by.
            let mut seen = None;
            for jobs in [
                vec![export(1, JobPhase::Running)],
                vec![export(1, JobPhase::Failed)],
                vec![
                    export(1, JobPhase::Finished),
                    other(2, JobPhase::Running, true),
                ],
            ] {
                for saved in SAVE_STATES {
                    harness.state_mut().jobs = jobs.clone();
                    harness.state_mut().saved = saved;
                    harness.run();
                    let folded = fold(&harness);
                    assert_eq!(
                        *seen.get_or_insert(folded),
                        folded,
                        "{locale:?}, {width} points, {saved:?}"
                    );
                    let last = names
                        .iter()
                        .filter_map(|name| harness.query_by_label(name))
                        .map(|node| node.rect().right())
                        .fold(0.0, f32::max);
                    let rect = spinner_named(&harness, &jobs_name).rect();
                    assert!(
                        rect.left() - last >= 24.0,
                        "{locale:?}, {width} points, {saved:?}: {last} to {}",
                        rect.left()
                    );
                    assert!(rect.right() <= width as f32, "{width}");
                    // A button, not a chip: it is as wide as the others.
                    assert!(rect.width() <= 30.5, "{width}: {rect:?}");
                }
            }
        }
    }
}

#[test]
fn reduced_motion_holds_the_spinner_still() {
    // The theme's motion tokens zero egui's animation time; the spinner turns otherwise.
    for (animation, turns) in [(0.12, true), (0.0, false)] {
        let mut harness = Harness::builder()
            .with_size(vec2(400.0, 60.0))
            .with_step_dt(0.1)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui(move |ui| {
                // The harness zeroes the animation time after the setup; the theme sets it.
                ui.ctx().all_styles_mut(|s| s.animation_time = animation);
                let jobs = [export(1, JobPhase::Running)];
                Jobs::new(&jobs).spinner(ui);
            });
        harness.run_steps(2);
        let before = harness.render().expect("render");
        harness.run_steps(3);
        let after = harness.render().expect("render");
        assert_eq!(before != after, turns, "animation time {animation}");
    }
}

/// What the harness stands in for in the window: the jobs and the actions.
struct Win {
    jobs: Vec<Job>,
    file_manager: FileManager,
    actions: Vec<ActivityAction>,
}

fn window_in(locale: Locale, width: f32, jobs: Vec<Job>) -> Harness<'static, Win> {
    Harness::builder()
        .with_size(vec2(width, 700.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, win: &mut Win| {
                let jobs = win.jobs.clone();
                // On the window's surface, inside its margin, as the app's window draws it.
                let t = apertrust_ui::theme(ui.ctx());
                let frame = egui::Frame::new()
                    .fill(t.colors.raised)
                    .inner_margin(t.spacing.m);
                let asked = frame.show(ui, |ui| {
                    Jobs::new(&jobs).file_manager(win.file_manager).window(ui)
                });
                if let Some(action) = asked.inner {
                    win.actions.push(action);
                }
            },
            Win {
                jobs,
                file_manager: FileManager::Finder,
                actions: Vec::new(),
            },
        )
}

fn window(jobs: Vec<Job>) -> Harness<'static, Win> {
    window_in(Locale::EnGb, 420.0, jobs)
}

/// Whether anything is named or labelled `label`.
fn has(harness: &Harness<'_, Win>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// A row, the group named by its message.
fn row<'a>(harness: &'a Harness<'_, Win>, title: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Group, title)
}

fn has_in(row: &egui_kittest::Node<'_>, label: &str) -> bool {
    row.query_all_by_label(label).next().is_some()
}

#[test]
fn each_export_state_says_its_words_and_offers_the_buttons_it_allows() {
    // The row's title, its lines, and its buttons in order, the ✕ first.
    type Case<'a> = (JobPhase, &'a str, &'a [&'a str], &'a [&'a str]);
    let to = format!("To {FOLDER}");
    let cases: [Case; 5] = [
        (
            JobPhase::Running,
            "Exporting 3 / 12 to file on disk",
            &["3 of 12 done · now IMG_2044.ARW", &to],
            &["Cancel", "Show in Finder"],
        ),
        (
            JobPhase::Finished,
            "12 photos exported",
            &[&to],
            &["Show in Finder"],
        ),
        (
            JobPhase::Failed,
            "Export failed: 11 of 12 photos exported",
            &["IMG_2047.ARW: could not write IMG_2047.jpg: the disk is full."],
            &["Try again", "Show in Finder", "Dismiss"],
        ),
        (
            JobPhase::Cancelled,
            "Export cancelled: 3 of 12 photos exported",
            &[&to],
            &["Show in Finder"],
        ),
        (
            JobPhase::Queued,
            "Exporting 0 / 12 to file on disk",
            &["Next in line: 12 photos, to ~/Pictures/apertrust_exported"],
            &["Cancel"],
        ),
    ];
    let all = ["Show in Finder", "Cancel", "Try again", "Dismiss"];
    for (phase, title, lines, buttons) in cases {
        let mut harness = window(vec![export(7, phase)]);
        harness.run();
        // The row is a group named by its title, once ended its outcome.
        row(&harness, title);
        for line in lines {
            assert!(has(&harness, line), "{phase:?}: {line}");
        }
        // The destination, while it runs and once it finished or was cancelled: a queued
        // one's detail names it, and a failure takes its place.
        let shown = matches!(
            phase,
            JobPhase::Running | JobPhase::Finished | JobPhase::Cancelled
        );
        assert_eq!(has(&harness, &to), shown, "{phase:?}: the folder");
        for button in all {
            assert_eq!(
                has(&harness, button),
                buttons.contains(&button),
                "{phase:?}: {button}"
            );
        }
        // Each button reports its action, for the job it belongs to.
        for button in buttons {
            harness.get_by_role_and_label(Role::Button, button).click();
            harness.run();
            let expected = match *button {
                "Show in Finder" => ActivityAction::ShowFolder(7),
                "Cancel" => ActivityAction::Cancel(7),
                "Try again" => ActivityAction::TryAgain(7),
                _ => ActivityAction::Dismiss(7),
            };
            assert_eq!(
                harness.state().actions.last(),
                Some(&expected),
                "{phase:?}: {button}"
            );
        }
        // A queued or running export's row has a bar, named by its title, with its
        // percentage, empty while it waits.
        let bar = harness.query_by_role(Role::ProgressIndicator);
        assert_eq!(bar.is_some(), phase.active(), "{phase:?}");
        if let Some(bar) = bar {
            let percent = if phase == JobPhase::Queued { 0.0 } else { 25.0 };
            assert_eq!(bar.accesskit_node().numeric_value(), Some(percent));
            assert_eq!(bar.accesskit_node().label().as_deref(), Some(title));
        }
        // Clear Finished, in the footer, takes a finished or cancelled row, never another.
        let clear = harness.get_by_role_and_label(Role::Button, "Clear Finished");
        let enabled = matches!(phase, JobPhase::Finished | JobPhase::Cancelled);
        assert_eq!(!clear.accesskit_node().is_disabled(), enabled, "{phase:?}");
    }
}

#[test]
fn clear_finished_asks_for_every_finished_and_cancelled_row_to_go() {
    let mut harness = window(vec![
        export(3, JobPhase::Failed),
        export(2, JobPhase::Cancelled),
        export(1, JobPhase::Finished),
    ]);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Clear Finished")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, vec![ActivityAction::ClearFinished]);
}

#[test]
fn a_finished_or_cancelled_row_says_when_it_ended_after_its_destination() {
    let mut harness = window(vec![
        export(3, JobPhase::Failed).ended_at(two()),
        export(2, JobPhase::Cancelled).ended_at(two()),
        export(1, JobPhase::Finished).ended_at(two()),
    ]);
    harness.run();
    let at = format!("To {FOLDER} · 14:02");
    assert_eq!(harness.query_all_by_label(&at).count(), 2);
    // A failed row's failure takes the place of its destination and its time.
    let failed = row(&harness, "Export failed: 11 of 12 photos exported");
    assert!(!has_in(&failed, &at));
}

#[test]
fn a_job_with_no_total_has_a_bar_with_no_value_that_holds_still_without_motion() {
    let reading = |id| {
        Job::new(
            id,
            JobPhase::Running,
            JobMessage::new("job-folder").text("folder", "2026-10 Lisbon"),
        )
        .kind("folder")
        .progress(120, None)
        .cancellable(true)
    };
    let mut harness = window(vec![reading(1)]);
    harness.run();
    let bar = harness.get_by_role(Role::ProgressIndicator);
    assert_eq!(
        bar.accesskit_node().label().as_deref(),
        Some("Reading 2026-10 Lisbon")
    );
    assert_eq!(bar.accesskit_node().numeric_value(), None);
    // Its segment moves while the theme has motion, and holds still where it has none.
    for (animation, moves) in [(0.12, true), (0.0, false)] {
        let mut harness = Harness::builder()
            .with_size(vec2(420.0, 200.0))
            .with_step_dt(0.1)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui(move |ui| {
                // The harness zeroes the animation time after the setup; the theme sets it.
                ui.ctx().all_styles_mut(|s| s.animation_time = animation);
                let jobs = [reading(1)];
                Jobs::new(&jobs).window(ui);
            });
        harness.run_steps(2);
        let before = harness.render().expect("render");
        harness.run_steps(3);
        let after = harness.render().expect("render");
        assert_eq!(before != after, moves, "animation time {animation}");
    }
}

#[test]
fn each_kind_has_its_icon_and_a_kind_not_listed_a_generic_one() {
    assert_eq!(kind_icon("export"), Icon::Export);
    assert_eq!(kind_icon("import"), Icon::Import);
    assert_eq!(kind_icon("folder"), Icon::Library);
    let generic = kind_icon("a kind from a later release");
    assert_eq!(kind_icon(""), generic);
    assert!(![Icon::Export, Icon::Import, Icon::Library].contains(&generic));
}

/// The rows' list: as tall as its rows up to five, and with more, as tall as the first five,
/// the rest scrolling, with the footer in view below it.
#[test]
fn the_list_shows_five_rows_and_scrolls_past_them_with_the_footer_in_view() {
    let rows = |count: u64| {
        (1..=count)
            .map(|id| other(id, JobPhase::Running, true))
            .collect::<Vec<_>>()
    };
    // Where the footer ends, the window's height: in view, below every row shown.
    let height = |count| {
        let mut harness = window(rows(count));
        harness.run();
        let clear = harness
            .get_by_role_and_label(Role::Button, "Clear Finished")
            .rect();
        assert!(clear.bottom() <= 700.0, "{count}: {clear:?}");
        clear.bottom()
    };
    let (four, five, six, twelve) = (height(4), height(5), height(6), height(12));
    assert!(four < five, "{four} {five}");
    assert!((five - six).abs() < 0.5, "{five} {six}");
    assert!((six - twelve).abs() < 0.5, "{six} {twelve}");
}

#[test]
fn a_failure_names_the_photo_and_file_only_where_it_has_them() {
    let failed = |item: &str, file: &str| {
        export(7, JobPhase::Failed).failure(JobFailure {
            item: item.to_owned(),
            file: file.to_owned(),
            cause: "the raw file cannot be read".to_owned(),
        })
    };
    for (photo, file, line) in [
        (
            "IMG_1.ARW",
            "IMG_1.jpg",
            "IMG_1.ARW: could not write IMG_1.jpg: the raw file cannot be read.",
        ),
        ("IMG_1.ARW", "", "IMG_1.ARW: the raw file cannot be read."),
        ("", "", "the raw file cannot be read."),
    ] {
        let mut harness = window(vec![failed(photo, file)]);
        harness.run();
        assert!(has(&harness, line), "{line}");
    }
}

#[test]
fn the_file_manager_names_the_button_by_platform() {
    for (manager, label) in [
        (FileManager::Finder, "Show in Finder"),
        (FileManager::Explorer, "Show in Explorer"),
        (FileManager::Files, "Show in Files"),
    ] {
        let mut harness = window(vec![export(1, JobPhase::Finished)]);
        harness.state_mut().file_manager = manager;
        harness.run();
        assert!(has(&harness, label), "{label}");
    }
}

/// A job that counts, as an import or a delete does.
fn counted(id: u64, phase: JobPhase, message: &str, done: f64, total: f64) -> Job {
    let message = JobMessage::new(message)
        .number("done", done)
        .number("total", total);
    Job::new(id, phase, message).progress(done as u64, Some(total as u64))
}

#[test]
fn jobs_are_listed_newest_first_whatever_order_they_come_in() {
    let jobs = vec![
        counted(2, JobPhase::Running, "job-import", 5.0, 9.0),
        other(9, JobPhase::Running, true),
        export(5, JobPhase::Queued),
        counted(1, JobPhase::Finished, "job-delete", 9.0, 9.0),
    ];
    let mut harness = window(jobs);
    harness.run();
    let top = |title: &str| {
        harness
            .query_all_by_label(title)
            .find(|n| n.accesskit_node().role() != Role::Group)
            .unwrap_or_else(|| panic!("no row {title}"))
            .rect()
            .top()
    };
    let order = [
        top("Creating thumbnails"),
        top("Exporting 0 / 12 to file on disk"),
        top("Importing 5 / 9"),
        top("Finished: Deleting 9 / 9"),
    ];
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "not newest first: {order:?}"
    );
}

#[test]
fn a_queued_job_says_so_and_one_that_cannot_be_cancelled_has_no_cancel() {
    let mut harness = window(vec![
        counted(1, JobPhase::Queued, "job-import", 0.0, 9.0),
        counted(2, JobPhase::Running, "job-delete", 4.0, 9.0),
        other(3, JobPhase::Running, true),
    ]);
    harness.run();
    let waiting = row(&harness, "Importing 0 / 9");
    assert!(has_in(&waiting, "Waiting for other jobs"));
    // Its bar is empty while it waits.
    let bar = waiting.query_by_role(Role::ProgressIndicator);
    assert_eq!(
        bar.and_then(|b| b.accesskit_node().numeric_value()),
        Some(0.0)
    );
    let thumbnails = row(&harness, "Creating thumbnails");
    assert!(!has_in(&thumbnails, "Waiting for other jobs"));
    // The import is not cancellable here; the thumbnails are, and say so.
    assert!(!has_in(&waiting, "Cancel"));
    assert!(has_in(&thumbnails, "Cancel"));
    // The delete cannot be stopped: a bar, and no button.
    let delete = row(&harness, "Deleting 4 / 9");
    assert!(!has_in(&delete, "Cancel"));
    assert_eq!(delete.query_all_by_role(Role::Button).count(), 0);
    assert!(delete.query_by_role(Role::ProgressIndicator).is_some());

    // A queued job that can be stopped offers Cancel.
    harness.state_mut().jobs = vec![other(5, JobPhase::Queued, true)];
    harness.run();
    assert!(has(&harness, "Waiting for other jobs"));
    harness
        .get_by_role_and_label(Role::Button, "Cancel")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, vec![ActivityAction::Cancel(5)]);
    harness.state_mut().actions.clear();

    // A job with no outcome, destination or retry that failed says so with its cause, and
    // can only be dismissed.
    harness.state_mut().jobs = vec![other(4, JobPhase::Failed, true).failure(JobFailure {
        item: String::new(),
        file: String::new(),
        cause: "the folder is not readable".to_owned(),
    })];
    harness.run();
    row(&harness, "Failed: Creating thumbnails");
    assert!(has(&harness, "the folder is not readable."));
    assert!(has(&harness, "Dismiss"));
    assert!(!has(&harness, "Cancel") && !has(&harness, "Try again"));
    assert!(!has(&harness, "Show in Finder"));
    harness
        .get_by_role_and_label(Role::Button, "Dismiss")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, vec![ActivityAction::Dismiss(4)]);
}

#[test]
fn no_job_says_so() {
    let mut harness = window(Vec::new());
    harness.run();
    assert!(has(&harness, "No jobs."));
}

#[test]
fn a_keyboard_user_reaches_every_rows_buttons_and_presses_them() {
    let mut harness = window(vec![
        export(4, JobPhase::Failed),
        export(3, JobPhase::Running),
        other(2, JobPhase::Running, true),
        export(1, JobPhase::Finished),
    ]);
    harness.run();
    // Failed: Try again, Show, Dismiss; running export: its ✕, Show; thumbnails: its ✕;
    // finished: Show; then the footer's Clear Finished. Newest first, in reading order.
    let expected = [
        ("Try again", ActivityAction::TryAgain(4)),
        ("Show in Finder", ActivityAction::ShowFolder(4)),
        ("Dismiss", ActivityAction::Dismiss(4)),
        ("Cancel", ActivityAction::Cancel(3)),
        ("Show in Finder", ActivityAction::ShowFolder(3)),
        ("Cancel", ActivityAction::Cancel(2)),
        ("Show in Finder", ActivityAction::ShowFolder(1)),
        ("Clear Finished", ActivityAction::ClearFinished),
    ];
    assert_eq!(
        harness.query_all_by_role(Role::Button).count(),
        expected.len()
    );
    for (label, action) in expected {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness
            .query_all_by_role(Role::Button)
            .find(|n| n.accesskit_node().is_focused())
            .unwrap_or_else(|| panic!("Tab never reached {label}"));
        assert_eq!(focused.accesskit_node().label().as_deref(), Some(label));
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().actions.last(), Some(&action), "{label}");
    }
    assert_eq!(harness.state().actions.len(), expected.len());
}

#[test]
fn the_window_says_its_words_in_every_language() {
    for locale in Locale::ALL {
        let words = Translator::new(locale);
        let mut harness = window_in(
            locale,
            460.0,
            vec![
                export(2, JobPhase::Failed),
                other(1, JobPhase::Queued, true),
            ],
        );
        harness.run();
        let title = {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("written", 11);
            args.set("skipped", 0);
            args.set("total", 12);
            words.text_with("job-export-failed-outcome", Some(&args))
        };
        assert!(has(&harness, &title), "{locale:?}: {title}");
        for id in [
            "activity-try-again",
            "activity-show-finder",
            "activity-dismiss",
            "jobs-queued",
            "jobs-cancel",
            "jobs-clear-finished",
            "job-thumbnails",
        ] {
            assert!(
                locale == Locale::Pseudo || words.has(id),
                "{locale:?}: {id} is not translated"
            );
            let text = words.text(id);
            assert!(has(&harness, &text), "{locale:?}: {text}");
        }
    }
}

#[test]
fn the_window_stays_inside_a_narrow_window_in_every_language() {
    for locale in Locale::ALL {
        let mut harness = window_in(
            locale,
            300.0,
            vec![export(2, JobPhase::Failed), export(1, JobPhase::Running)],
        );
        harness.run();
        // Try again, Show and Dismiss; the ✕ and Show; Clear Finished.
        let buttons: Vec<_> = harness.query_all_by_role(Role::Button).collect();
        assert_eq!(buttons.len(), 6);
        for button in buttons {
            let rect = button.rect();
            assert!(
                rect.left() >= 0.0 && rect.right() <= 300.0,
                "{locale:?}: {:?} {rect:?}",
                button.accesskit_node().label()
            );
        }
    }
}

/// The window `width` wide, photographed.
fn snapshot(name: &str, locale: Locale, width: f32, jobs: Vec<Job>) {
    let mut harness = window_in(locale, width, jobs);
    harness.set_size(vec2(width, 760.0));
    harness.run();
    harness.fit_contents();
    harness.run();
    harness.snapshot(name);
}

/// A row in each state the wireframe draws, the oldest first: a finished export with the
/// time it ended, a failed one, a folder being read with no total, a running export and a
/// waiting one.
fn several() -> Vec<Job> {
    vec![
        export(1, JobPhase::Finished).ended_at(two()),
        export(2, JobPhase::Failed),
        Job::new(
            3,
            JobPhase::Running,
            JobMessage::new("job-folder").text("folder", "2026-10 Lisbon"),
        )
        .kind("folder")
        .progress(120, None)
        .cancellable(true),
        export(4, JobPhase::Running),
        export(5, JobPhase::Queued),
    ]
}

#[test]
fn the_window_with_several_jobs() {
    snapshot("jobs-window", Locale::EnGb, 400.0, several());
}

#[test]
fn the_window_in_ukrainian() {
    snapshot("jobs-window-uk", Locale::Uk, 400.0, several());
}

#[test]
fn the_window_in_polish() {
    snapshot("jobs-window-pl", Locale::Pl, 400.0, several());
}

/// At the narrowest, in the pseudo-locale, whose words are 40% longer: the buttons wrap and
/// stay inside the window.
#[test]
fn the_window_at_the_narrowest_in_the_pseudo_locale() {
    snapshot(
        "jobs-window-narrow-pseudo",
        Locale::Pseudo,
        300.0,
        vec![
            export(2, JobPhase::Failed),
            other(1, JobPhase::Running, false),
        ],
    );
}
