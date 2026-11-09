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

use apertrust_ui::components::{
    ActivityAction, Available, ExportDetails, ExportFailure, FileManager, Job, JobMessage,
    JobPhase, Jobs, SaveState, TopPanel, TopPanelAction,
};
use apertrust_ui::{Locale, Theme, Translator};
use egui::accesskit::{Live, Role};
use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const FOLDER: &str = "~/Pictures/apertrust_exported";

fn export_message(done: usize, total: usize) -> JobMessage {
    JobMessage::new("job-export")
        .number("done", done as f64)
        .number("total", total as f64)
        .text("target", "file on disk")
}

/// An export of 12 photos with id `id`, in `phase`, 3 in, as the gallery has it.
fn export(id: u64, phase: JobPhase) -> Job {
    let mut details = ExportDetails::new(12, FOLDER);
    let mut cancellable = false;
    match phase {
        JobPhase::Running => {
            details.done = 3;
            details.current = Some("IMG_2044.ARW".to_owned());
            cancellable = true;
        }
        JobPhase::Finished => details.done = 12,
        JobPhase::Failed => {
            details.done = 11;
            details.failed = 1;
            details.failure = Some(ExportFailure {
                photo: "IMG_2047.ARW".to_owned(),
                file: "IMG_2047.jpg".to_owned(),
                cause: "the disk is full".to_owned(),
            });
        }
        JobPhase::Cancelled => details.done = 3,
        JobPhase::Queued => cancellable = true,
    }
    let message = export_message(details.done, details.total);
    Job::new(id, phase, message)
        .fraction(details.done as f32 / 12.0)
        .cancellable(cancellable)
        .export(details)
}

/// A job that is not an export: thumbnails, or a delete that cannot be stopped.
fn other(id: u64, phase: JobPhase, cancellable: bool) -> Job {
    Job::new(id, phase, JobMessage::new("job-thumbnails"))
        .fraction(0.6)
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
    let cases: [(&str, Vec<Job>, &str); 8] = [
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
            "finished",
            vec![
                export(1, JobPhase::Finished),
                export(2, JobPhase::Cancelled),
            ],
            "Finished",
        ),
        (
            "cancelled",
            vec![export(1, JobPhase::Cancelled)],
            "Cancelled",
        ),
        (
            "cancelled and finished",
            vec![
                export(1, JobPhase::Cancelled),
                export(2, JobPhase::Finished),
            ],
            "Finished",
        ),
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
    assert_eq!(start, "Exporting 12 photos");

    // Photo after photo it says the same, so nothing is spoken again.
    for done in 4..12 {
        let mut running = export(1, JobPhase::Running);
        if let Some(details) = running.export.as_mut() {
            details.done = done;
            details.current = Some(format!("IMG_{}.ARW", 2040 + done));
        }
        running.fraction = done as f32 / 12.0;
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

    // A job ends while another runs, and the end is said.
    for (phase, text) in [
        (JobPhase::Finished, "Export finished: 12 photos exported"),
        (
            JobPhase::Failed,
            "Export finished, 1 failed: 11 of 12 photos exported",
        ),
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

    // A job that is not an export says its own message and how it ended.
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
    assert_eq!(
        said(&harness),
        "Export finished, 1 failed: 11 of 12 photos exported"
    );
    // Try again, with the same id: the start is said, not left unsaid because the id
    // was heard before.
    harness.state_mut().jobs = vec![export(1, JobPhase::Running)];
    harness.run();
    assert_eq!(said(&harness), "Exporting 12 photos");
}

#[test]
fn what_is_left_when_jobs_end_shows_a_tick_or_a_warning_in_the_spinners_place() {
    let mut harness = panel(vec![export(1, JobPhase::Running)]);
    harness.run();
    let at = spinner(&harness).rect();
    for jobs in [
        vec![export(1, JobPhase::Finished)],
        vec![export(1, JobPhase::Failed)],
        vec![export(1, JobPhase::Cancelled)],
    ] {
        harness.state_mut().jobs = jobs;
        harness.run();
        assert_eq!(spinner(&harness).rect(), at);
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

/// Which symbol is drawn: the ring turns while a job runs and stops when they have ended,
/// leaving the tick or, where one failed, the warning sign in `warning`.
#[test]
fn the_spinner_stops_turning_and_shows_a_tick_or_a_warning_when_jobs_end() {
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

    let mut ended = Vec::new();
    for (phase, warns) in [(JobPhase::Finished, false), (JobPhase::Failed, true)] {
        harness.state_mut().jobs = vec![export(1, phase)];
        let (c, d) = (frame(&mut harness), frame(&mut harness));
        assert_eq!(c, d, "{phase:?}: it keeps turning");
        assert_ne!(c, a, "{phase:?}: the ring is still drawn");
        assert_eq!(pixels_near!(c, warning) > 0, warns, "{phase:?}");
        ended.push(c);
    }
    assert_ne!(ended[0], ended[1], "the tick and the warning look alike");
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
                ui.add_space(8.0);
                if let Some(action) = Jobs::new(&jobs).file_manager(win.file_manager).window(ui) {
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
    // The row's lines, and its buttons in order.
    type Case<'a> = (JobPhase, &'a [&'a str], &'a [&'a str]);
    let cases: [Case; 5] = [
        (
            JobPhase::Running,
            &["3 of 12 done · now IMG_2044.ARW"],
            &["Show in Finder", "Cancel export"],
        ),
        (
            JobPhase::Finished,
            &["12 photos exported"],
            &["Show in Finder", "Dismiss"],
        ),
        (
            JobPhase::Failed,
            &[
                "11 of 12 photos exported",
                "IMG_2047.ARW: could not write IMG_2047.jpg: the disk is full.",
            ],
            &["Try again", "Show in Finder", "Dismiss"],
        ),
        (
            JobPhase::Cancelled,
            &[
                "Export cancelled",
                "3 of 12 photos were exported before it stopped; they stay.",
            ],
            &["Show in Finder", "Dismiss"],
        ),
        (
            JobPhase::Queued,
            &["Next in line: 12 photos, to ~/Pictures/apertrust_exported"],
            &["Cancel export"],
        ),
    ];
    let all = ["Show in Finder", "Cancel export", "Try again", "Dismiss"];
    for (phase, lines, buttons) in cases {
        let job = export(7, phase);
        let title = format!(
            "Exporting {} / 12 to file on disk",
            job.export.as_ref().map_or(0, |d| d.done)
        );
        let mut harness = window(vec![job]);
        harness.run();
        assert!(has(&harness, &title), "{phase:?}: {title}");
        for line in lines {
            assert!(has(&harness, line), "{phase:?}: {line}");
        }
        if phase != JobPhase::Queued {
            assert!(
                has(&harness, &format!("To {FOLDER}")),
                "{phase:?}: the folder"
            );
        }
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
                "Cancel export" => ActivityAction::Cancel(7),
                "Try again" => ActivityAction::TryAgain(7),
                _ => ActivityAction::Dismiss(7),
            };
            assert_eq!(
                harness.state().actions.last(),
                Some(&expected),
                "{phase:?}: {button}"
            );
        }
        // A running export's row has a bar, named by its message, with its percentage.
        let bar = harness.query_by_role(Role::ProgressIndicator);
        assert_eq!(bar.is_some(), phase == JobPhase::Running, "{phase:?}");
        if let Some(bar) = bar {
            assert_eq!(bar.accesskit_node().numeric_value(), Some(25.0));
            assert_eq!(
                bar.accesskit_node().label().as_deref(),
                Some(title.as_str())
            );
        }
    }
}

#[test]
fn a_failure_names_the_photo_and_file_only_where_it_has_them() {
    let failed = |photo: &str, file: &str| {
        let mut job = export(7, JobPhase::Failed);
        if let Some(details) = job.export.as_mut() {
            details.failure = Some(ExportFailure {
                photo: photo.to_owned(),
                file: file.to_owned(),
                cause: "the raw file cannot be read".to_owned(),
            });
        }
        job
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
    Job::new(id, phase, message).fraction((done / total) as f32)
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
        top("Deleting 9 / 9"),
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
    assert!(waiting.query_by_role(Role::ProgressIndicator).is_none());
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

    // A non-export that failed says so with its reason, and can only be dismissed.
    harness.state_mut().jobs =
        vec![other(4, JobPhase::Failed, true).error("the folder is not readable")];
    harness.run();
    assert!(has(&harness, "Failed"));
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
        export(3, JobPhase::Failed),
        export(2, JobPhase::Running),
        other(1, JobPhase::Running, true),
    ]);
    harness.run();
    // Failed: Try again, Show, Dismiss; running export: Show, Cancel export; thumbnails:
    // Cancel. Newest first, in reading order.
    let expected = [
        ("Try again", ActivityAction::TryAgain(3)),
        ("Show in Finder", ActivityAction::ShowFolder(3)),
        ("Dismiss", ActivityAction::Dismiss(3)),
        ("Show in Finder", ActivityAction::ShowFolder(2)),
        ("Cancel export", ActivityAction::Cancel(2)),
        ("Cancel", ActivityAction::Cancel(1)),
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
            args.set("done", 11);
            args.set("total", 12);
            args.set("target", "file on disk");
            words.text_with("job-export", Some(&args))
        };
        assert!(has(&harness, &title), "{locale:?}: {title}");
        for id in [
            "activity-try-again",
            "activity-show-finder",
            "activity-dismiss",
            "jobs-queued",
            "jobs-cancel",
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
        let buttons: Vec<_> = harness.query_all_by_role(Role::Button).collect();
        assert_eq!(buttons.len(), 5);
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

fn several() -> Vec<Job> {
    vec![
        export(1, JobPhase::Failed),
        Job::new(
            2,
            JobPhase::Finished,
            JobMessage::new("job-import")
                .number("done", 212.0)
                .number("total", 212.0),
        )
        .fraction(1.0),
        Job::new(
            3,
            JobPhase::Running,
            JobMessage::new("job-delete")
                .number("done", 4.0)
                .number("total", 9.0),
        )
        .fraction(0.44),
        export(4, JobPhase::Running),
        export(5, JobPhase::Queued),
        other(6, JobPhase::Running, true),
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
