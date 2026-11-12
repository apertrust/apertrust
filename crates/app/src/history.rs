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
//! A step with a note reads its note; one touching one module, the module's title; one
//! touching several, how many; a `GoTo` taken back or put back, Go to Step. darktable's
//! label after a • and a layer after a · join a module's title with layers (Phase 4).

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

/// What `step` does, as History names it; none for a step that names no module the app
/// knows, as an operation's unowned parameters alone.
fn named(
    history: &History,
    step: Move,
    modules: &[&dyn Module],
    tr: &Translator,
) -> Option<String> {
    match step {
        Move::GoTo => Some(tr.text("history-go-to-step")),
        Move::Step(id) => {
            let step = history.steps.iter().find(|s| s.id == id)?;
            name(step, modules, tr)
        }
    }
}

/// A step's name: its note, else its one module's title, else how many modules it touched.
fn name(step: &HistoryStep, modules: &[&dyn Module], tr: &Translator) -> Option<String> {
    match &step.note {
        Some(Note::Text(text)) => return Some(text.clone()),
        Some(Note::Message { id, args }) => {
            let mut fluent = Args::new();
            for (key, value) in args {
                fluent.set(key.as_str(), value.as_str());
            }
            return Some(tr.text_with(id, Some(&fluent)));
        }
        None => {}
    }
    match step.changes.as_slice() {
        [] => None,
        [one] => modules
            .iter()
            .any(|m| m.definition().id == one.module)
            .then(|| tr.text(&apertrust_modules::messages::title(one.module))),
        several => {
            let mut args = Args::new();
            args.set("count", several.len());
            Some(tr.text_with("history-modules", Some(&args)))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use apertrust_engine::history::{Clock, MachineId};
    use apertrust_engine::session::{ModuleChange, StepId, StepSource};
    use apertrust_ui::Locale;

    use super::*;

    /// A step of source You touching `modules`, with `note`.
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

    fn words(history: Option<&History>, locale: Locale) -> Steps {
        let tr = Translator::new(locale);
        steps(history, apertrust_modules::MODULES, &tr)
    }

    #[test]
    fn undo_and_redo_name_the_step_as_history_does() {
        let steps = vec![
            step(1, &["exposure"], None),
            step(2, &["exposure", "colorbalancergb"], None),
            step(3, &[], Some(Note::reset())),
            step(4, &["exposure"], Some(Note::Text("Brighter".to_owned()))),
            // An operation's unowned parameters, which no module's title names.
            step(5, &["colorin"], None),
            step(6, &[], None),
        ];
        let named = |undo, redo| {
            let history = history(9, steps.clone(), undo, redo);
            words(Some(&history), Locale::EnGb)
        };
        let undo = |id| named(Some(Move::Step(StepId(id))), None).undo;
        assert_eq!(undo(1).as_deref(), Some("Exposure"));
        assert_eq!(undo(2).as_deref(), Some("2 modules"));
        assert_eq!(undo(3).as_deref(), Some("Reset"));
        assert_eq!(undo(4).as_deref(), Some("Brighter"));
        // Nothing the app can name, or a step not in the log: Undo reads Undo alone.
        assert_eq!(undo(5), None);
        assert_eq!(undo(6), None);
        assert_eq!(undo(7), None);
        let go_to = named(Some(Move::Step(StepId(1))), Some(Move::GoTo));
        assert_eq!(go_to.redo.as_deref(), Some("Go to Step"));
        // No history, or nothing to do.
        assert_eq!(words(None, Locale::EnGb), Steps::default());
        assert_eq!(named(None, None), Steps::default());
    }

    #[test]
    fn a_step_is_named_in_the_interface_s_language() {
        let steps = vec![
            step(1, &["exposure"], None),
            step(2, &["exposure", "colorbalancergb", "agx"], None),
        ];
        let undo = |id, locale| {
            let history = history(3, steps.clone(), Some(Move::Step(StepId(id))), None);
            words(Some(&history), locale).undo
        };
        assert_eq!(undo(1, Locale::Uk).as_deref(), Some("Експозиція"));
        assert_eq!(undo(1, Locale::Pl).as_deref(), Some("Ekspozycja"));
        assert_eq!(undo(2, Locale::Uk).as_deref(), Some("3 модулі"));
        assert_eq!(undo(2, Locale::Pl).as_deref(), Some("3 moduły"));
    }
}
