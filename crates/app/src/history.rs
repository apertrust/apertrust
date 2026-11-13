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

//! The edit's history as the app words it (`docs/history.md`, In the app; decision 78):
//! what Undo and Redo would do, named as History names a step, for Edit's items.
//!
//! A step with a note reads its note, Reset's among them, and one without its module's
//! title; a batch, the steps one change made on several modules, its note, else the name
//! of its first step the app knows and how many more it knows; a `GoTo` taken back or put back, Go to Step. darktable's
//! label after a • and a layer after a · join a module's title with layers (Phase 4).

use std::sync::Arc;

use apertrust_engine::session::{History, HistoryStep, Move, Note};
use apertrust_modules::Module;
use apertrust_ui::Translator;
use apertrust_ui::i18n::Args;

use crate::menus::Steps;

/// What Undo and Redo would do in `history`, the open photo's, in `tr`'s words: none where
/// there is nothing to do, or nothing to name. `modules` are the session's, whose titles
/// name a step.
pub fn steps(history: Option<&History>, modules: &[&dyn Module], tr: &Translator) -> Steps {
    let side = |wanted: fn(&History) -> Option<Move>| {
        let history = history?;
        named(history, wanted(history)?, modules, tr)
    };
    Steps {
        undo: side(|h| h.undo),
        redo: side(|h| h.redo),
    }
}

/// What `moved` does, as History names it; none for steps not in `history`, or whose first
/// names no module the app knows, as an operation's unowned parameters alone.
fn named(
    history: &History,
    moved: Move,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Option<String> {
    match moved {
        Move::GoTo => Some(tr.text("history-go-to-step")),
        Move::Steps { first, count } => {
            let at = history.steps.iter().position(|s| s.id == first)?;
            let steps = history.steps.get(at..at + count)?;
            name(steps, modules, tr)
        }
    }
}

/// The name of one change's steps: its note, which a batch's steps share and its last
/// carries, a Reset's coming first; else the name of its first step the app knows, and how
/// many more of those it knows.
fn name(steps: &[Arc<HistoryStep>], modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    if let Some(note) = &steps.last()?.note {
        return Some(noted(note, tr));
    }
    let mut known = steps.iter().filter_map(|step| step_name(step, modules, tr));
    let first = known.next()?;
    let more = known.count();
    if more == 0 {
        return Some(first);
    }
    let mut args = Args::new();
    args.set("module", first);
    args.set("count", more);
    Some(tr.text_with("history-modules", Some(&args)))
}

/// One step's name: its note, else its module's title.
fn step_name(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    if let Some(note) = &step.note {
        return Some(noted(note, tr));
    }
    let change = step.changes.first()?;
    (modules.iter())
        .any(|m| m.definition().id == change.module)
        .then(|| tr.text(&apertrust_modules::messages::title(change.module)))
}

/// A note's words: an assistant's as written, Apertrust's own in `tr`'s.
fn noted(note: &Note, tr: &Translator) -> String {
    match note {
        Note::Text(text) => text.clone(),
        Note::Message { id, args } => {
            let mut fluent = Args::new();
            for (key, value) in args {
                fluent.set(key.as_str(), value.as_str());
            }
            tr.text_with(id, Some(&fluent))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use apertrust_engine::history::{Clock, MachineId};
    use apertrust_engine::session::{BatchId, ModuleChange, StepId, StepSource};
    use apertrust_ui::Locale;

    use super::*;

    /// A step of source You touching `modules`, one, or a reset's several, with `note`.
    pub fn step(id: u64, modules: &[&'static str], note: Option<Note>) -> Arc<HistoryStep> {
        Arc::new(HistoryStep {
            id: StepId(id),
            clock: Clock::at(0, None, MachineId::UNKNOWN),
            source: StepSource::You,
            note,
            batch: None,
            changes: modules
                .iter()
                .map(|module| ModuleChange {
                    module,
                    enabled: None,
                    controls: Vec::new(),
                })
                .collect(),
        })
    }

    /// A history including `through` commands, its `steps` all applied, with `undo` and
    /// `redo` as given.
    pub fn history(
        through: u64,
        steps: Vec<Arc<HistoryStep>>,
        undo: Option<Move>,
        redo: Option<Move>,
    ) -> History {
        History {
            through,
            end: steps.len(),
            steps,
            defaults: Vec::new(),
            undo,
            redo,
        }
    }

    /// `steps`, one change's of several modules, sharing the batch `batch`.
    pub fn batched(batch: u64, steps: Vec<Arc<HistoryStep>>) -> Vec<Arc<HistoryStep>> {
        (steps.into_iter())
            .map(|step| {
                Arc::new(HistoryStep {
                    batch: Some(BatchId(batch)),
                    ..(*step).clone()
                })
            })
            .collect()
    }

    /// Undo or Redo moving the `count` steps from the step `first` on.
    pub fn moving(first: u64, count: usize) -> Move {
        Move::Steps {
            first: StepId(first),
            count,
        }
    }

    fn words(history: Option<&History>, locale: Locale) -> Steps {
        let tr = Translator::new(locale);
        steps(history, apertrust_modules::MODULES, &tr)
    }

    #[test]
    fn undo_and_redo_name_the_steps_as_history_does() {
        let style = || Some(Note::Text("Style".to_owned()));
        let mut steps = vec![step(1, &["exposure"], None)];
        steps.extend(batched(
            9,
            vec![
                step(2, &["exposure"], None),
                step(3, &["colorbalancergb"], None),
                step(4, &["agx"], None),
            ],
        ));
        steps.push(step(5, &["exposure", "agx"], Some(Note::reset())));
        steps.extend(batched(
            8,
            vec![step(6, &["exposure"], style()), step(7, &["agx"], style())],
        ));
        steps.push(step(
            8,
            &["exposure"],
            Some(Note::Text("Brighter".to_owned())),
        ));
        // An operation's unowned parameters, which no module's title names.
        steps.push(step(9, &["colorin"], None));
        steps.push(step(10, &[], None));
        steps.extend(batched(
            7,
            vec![
                step(11, &["colorin"], None),
                step(12, &["exposure"], None),
                step(13, &["colorin"], None),
                step(14, &["agx"], None),
            ],
        ));
        let named = |undo, redo| {
            let history = history(15, steps.clone(), undo, redo);
            words(Some(&history), Locale::EnGb)
        };
        let undo = |first, count| named(Some(moving(first, count)), None).undo;
        assert_eq!(undo(1, 1).as_deref(), Some("Exposure"));
        // A batch: its first module and how many more; the part `GoTo` left of one alike.
        assert_eq!(undo(2, 3).as_deref(), Some("Exposure and 2 more"));
        assert_eq!(undo(3, 2).as_deref(), Some("Colour balance RGB and 1 more"));
        assert_eq!(undo(5, 1).as_deref(), Some("Reset"));
        // A batch with a note: its note.
        assert_eq!(undo(6, 2).as_deref(), Some("Style"));
        assert_eq!(undo(8, 1).as_deref(), Some("Brighter"));
        // A batch of steps the app does not all know: the first it knows, and how many more
        // it knows.
        assert_eq!(undo(11, 4).as_deref(), Some("Exposure and 1 more"));
        assert_eq!(undo(11, 2).as_deref(), Some("Exposure"));
        // Nothing the app can name, or steps not in the log: Undo reads Undo alone.
        assert_eq!(undo(9, 1), None);
        assert_eq!(undo(10, 1), None);
        assert_eq!(undo(13, 1), None);
        assert_eq!(undo(14, 2), None);
        assert_eq!(undo(15, 1), None);
        let go_to = named(Some(moving(1, 1)), Some(Move::GoTo));
        assert_eq!(go_to.redo.as_deref(), Some("Go to Step"));
        // No history, or nothing to do.
        assert_eq!(words(None, Locale::EnGb), Steps::default());
        assert_eq!(named(None, None), Steps::default());
    }

    #[test]
    fn a_step_is_named_in_the_interface_s_language() {
        let mut steps = vec![step(1, &["exposure"], None)];
        steps.extend(batched(
            9,
            vec![
                step(2, &["exposure"], None),
                step(3, &["colorbalancergb"], None),
                step(4, &["agx"], None),
            ],
        ));
        let undo = |first, count, locale| {
            let history = history(3, steps.clone(), Some(moving(first, count)), None);
            words(Some(&history), locale).undo
        };
        assert_eq!(undo(1, 1, Locale::Uk).as_deref(), Some("Експозиція"));
        assert_eq!(undo(1, 1, Locale::Pl).as_deref(), Some("Ekspozycja"));
        assert_eq!(
            undo(2, 3, Locale::Uk).as_deref(),
            Some("Експозиція та ще 2 модулі")
        );
        assert_eq!(
            undo(2, 3, Locale::Pl).as_deref(),
            Some("Ekspozycja i jeszcze 2 moduły")
        );
    }
}
