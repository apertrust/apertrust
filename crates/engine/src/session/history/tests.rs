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

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use apertrust_modules::exposure::Exposure;
use apertrust_modules::{ControlValue, Mark, testing};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::Value;

use super::*;
use crate::history::{HistoryStep, StepId};

/// A small deterministic generator, xorshift64*, so a failing sequence repeats.
struct Random(u64);

impl Random {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) % n
    }
}

/// A random change to `edit`, of the kinds an edit's commands make, and what it wrote: a
/// value, a mark, a switched module turned on or off, a module reset, an unowned parameter,
/// the whole edit reset. A mark taken away may leave a module holding nothing, as no
/// command does.
fn change(edit: &mut Edit, random: &mut Random) -> Wrote {
    let module = ["a", "b", "c"][random.below(3) as usize].to_owned();
    let wrote = Wrote::Module(module.clone());
    match random.below(12) {
        0..=3 => {
            let value = ControlValue::Number(random.below(4) as f32);
            edit.modules.entry(module).or_default().set("x", value);
        }
        4 | 5 => {
            let mark = [None, Some(Mark::Kept)][random.below(2) as usize];
            edit.modules.entry(module).or_default().set_mark("y", mark);
        }
        6 | 7 => {
            let values = edit.modules.entry(module).or_default();
            values.set_enabled(!values.enabled());
        }
        8 => {
            edit.modules.remove(&module);
        }
        9 | 10 => {
            let value = Value::Float(random.below(3) as f32);
            edit.params
                .entry("op".to_owned())
                .or_default()
                .set("p", value);
            return Wrote::Operation("op".to_owned());
        }
        _ => {
            *edit = Edit::default();
            return Wrote::All;
        }
    }
    wrote
}

/// `edit` without the modules and operations that hold nothing: the same state.
fn tidy(edit: &Edit) -> Edit {
    let mut tidy = edit.clone();
    tidy.modules.retain(|_, values| !values.is_empty());
    tidy.params
        .retain(|_, params| params.iter().next().is_some());
    tidy
}

/// A change that wrote `wrote`, of a random target of two controls of its module, or of
/// several or none, sent between 0 and 3 s after the last, a frame published after it half
/// the time: so some changes merge into the step before them, by the two times, and some
/// do not. An operation's parameters or `Reset` have no target, as the session gives them.
fn write(history: &mut PhotoHistory, wrote: Wrote, at: &mut Instant, random: &mut Random) {
    let pick = random.below(3) as usize;
    let target = match &wrote {
        Wrote::Module(module) => {
            let control = |control: &str| Target::Control {
                module: module.clone(),
                control: control.to_owned(),
            };
            [Some(control("x")), Some(control("y")), None][pick].clone()
        }
        Wrote::Operation(_) | Wrote::All => None,
    };
    *at += Duration::from_millis(random.below(3000));
    history.wrote(target, vec![wrote], *at);
    if random.below(2) == 0 {
        history.published(*at + Duration::from_millis(random.below(500)));
    }
}

/// Over random sequences of changes, completed changes, Undo, Redo, `GoTo` and reopening the
/// photo, which leaves no undo records, so Undo and Redo fall back to moving the end: the
/// fold is always the edit the history last stepped or moved to, and every change's fold,
/// up to and with its last step, is the edit as that change, or the last merged into it,
/// left it; Undo and Redo are each other's inverse, over a batch's steps whole, Undo takes
/// a `GoTo` back, and a merged step whole, and a merged change adds no record; a `GoTo`
/// inside a batch lands on the batch's steps up to it. A change not yet a step goes on
/// over several rounds, so it may touch several modules, a batch. Half the runs start from
/// an edit read from a file, a step of each module.
#[test]
fn the_fold_is_the_edit_after_every_sequence_and_undo_and_redo_are_inverses() {
    // How many times Undo and Redo fell back to moving the end, a `GoTo` moved it, and a
    // change merged.
    let mut fallbacks = [0, 0];
    let mut gone_to = 0;
    let mut merged = 0;
    // How many times Undo took back a batch, and a `GoTo` went inside one.
    let mut batches = 0;
    let mut inside = 0;
    for seed in 1..=40 {
        let mut random = Random(0x9e37_79b9_7f4a_7c15 ^ seed);
        let mut at = Instant::now();
        let mut edit = Edit::default();
        if seed % 2 == 1 {
            for _ in 0..4 {
                change(&mut edit, &mut random);
            }
        }
        let mut history = reopened(&edit);
        // The edit each step left, by id, with the step's clock then.
        let mut left: BTreeMap<StepId, (Clock, Edit)> = BTreeMap::new();
        record(&history, &tidy(&edit), &mut left);
        for round in 0..300 {
            let before = tidy(&edit);
            let context = format!("seed {seed}, round {round}");
            match random.below(8) {
                // The photo closed and opened again: its change not yet a step made one,
                // and the undo records and what may merge gone with the session's memory.
                6 => {
                    history.complete(&edit, StepSource::You, None);
                    record(&history, &before, &mut left);
                    history.done.clear();
                    history.undone.clear();
                    history.merge = None;
                }
                // A step, or Original, gone to: a change not yet a step is made one first,
                // which drops the steps after the end, so the step may be gone by then.
                7 => {
                    let steps = history.log.steps();
                    let at = random.below(steps.len() as u64 + 1) as usize;
                    let step = at.checked_sub(1).map(|at| steps[at].id);
                    let went = history.go_to(&edit, step);
                    record(&history, &before, &mut left);
                    let end = |history: &PhotoHistory, id: Option<StepId>| {
                        id.map_or(Some(0), |id| history.log.position(id).map(|at| at + 1))
                    };
                    match went {
                        Err(id) => assert_eq!(end(&history, Some(id)), None, "{context}"),
                        Ok(None) => {
                            assert_eq!(Some(history.log.end()), end(&history, step), "{context}");
                        }
                        // The edit as the step left it, or inside a batch, the batch's steps
                        // up to it; Undo takes it back to the edit as it stood, and Redo
                        // goes there again.
                        Ok(Some(gone)) => {
                            gone_to += 1;
                            let at = history.log.end();
                            if !last_of_change(&history.log, at) {
                                inside += 1;
                            }
                            assert_left(&history.log, at, &gone, &left, &context);
                            assert_eq!(Some(history.log.end()), end(&history, step), "{context}");
                            assert_eq!(history.undo(&gone), Some(before.clone()), "{context}");
                            assert_eq!(history.redo(&before), Some(gone.clone()), "{context}");
                            edit = gone;
                        }
                    }
                }
                // A change not yet a step, a gesture's or a `Set`'s.
                0 | 1 => {
                    let wrote = change(&mut edit, &mut random);
                    write(&mut history, wrote, &mut at, &mut random);
                }
                2 | 3 => {
                    let wrote = change(&mut edit, &mut random);
                    write(&mut history, wrote, &mut at, &mut random);
                    let (steps, done) = (ids(&history), history.done.len());
                    if history.complete(&edit, StepSource::You, None) && history.done.len() == done
                    {
                        // Merged: no record, and into the last step, at the end, which keeps
                        // its id.
                        merged += 1;
                        assert_eq!(ids(&history), steps, "{context}");
                        assert_eq!(history.log.end(), steps.len(), "{context}");
                    }
                }
                4 => {
                    // What Undo would take back, a change's steps made first.
                    history.complete(&edit, StepSource::You, None);
                    record(&history, &before, &mut left);
                    if let Some(Move::Steps { count: 2.., .. }) = history.next(true) {
                        batches += 1;
                    }
                    let Some(undone) = checked(&mut history, &edit, false, &context) else {
                        assert_eq!(history.log.end(), 0, "{context}");
                        assert_eq!(history.log.fold(), before, "{context}");
                        continue;
                    };
                    if matches!(history.undone.last(), Some(Record::Moved { .. })) {
                        fallbacks[0] += 1;
                    }
                    // Redo puts back exactly what Undo took, the change it made a step
                    // first included; Undo again takes it back again.
                    let redone = checked(&mut history, &undone, true, &context);
                    assert_eq!(redone, Some(before.clone()), "{context}");
                    record(&history, &before, &mut left);
                    let again = checked(&mut history, &before, false, &context);
                    assert_eq!(again, Some(undone.clone()), "{context}");
                    edit = undone;
                }
                // With a change not yet a step, Redo makes it one, which leaves nothing to
                // redo.
                _ => {
                    if let Some(redone) = checked(&mut history, &edit, true, &context) {
                        if matches!(history.done.last(), Some(Record::Moved { .. })) {
                            fallbacks[1] += 1;
                        }
                        let undone = checked(&mut history, &redone, false, &context);
                        assert_eq!(undone, Some(before.clone()), "{context}");
                        let again = checked(&mut history, &before, true, &context);
                        assert_eq!(again, Some(redone.clone()), "{context}");
                        edit = redone;
                    }
                }
            }
            record(&history, &tidy(&edit), &mut left);
            assert_eq!(history.log.fold(), tidy(&history.stepped), "{context}");
            for at in 1..=history.log.steps().len() {
                let fold = history.log.fold_to(at);
                assert_left(&history.log, at, &fold, &left, &context);
            }
        }
    }
    assert!(fallbacks.iter().all(|&count| count > 0), "{fallbacks:?}");
    assert!(
        gone_to > 0 && merged > 0 && batches > 0 && inside > 0,
        "{gone_to} gone to, {merged} merged, {batches} batches undone, {inside} inside one"
    );
}

/// Undo, or Redo where `redo`, on `edit`, a change not yet a step made one first, held to
/// what `next` said it would move: a fallback's or a moved record's end moved over `count`
/// steps from the one named, the log as it was; steps added taken out of the log, `count`
/// of them, the steps they dropped back, or put back after the end, the end after them. A
/// `GoTo` taken back or put back, or nothing where `next` names nothing. What it left.
fn checked(history: &mut PhotoHistory, edit: &Edit, redo: bool, context: &str) -> Option<Edit> {
    history.complete(edit, StepSource::You, None);
    let next = history.next(!redo);
    let records = if redo { &history.undone } else { &history.done };
    let held = match records.last() {
        Some(Record::Added { held, .. }) => Some(held.len()),
        _ => None,
    };
    let (end, before) = (history.log.end(), ids(history));
    let moved = if redo {
        history.redo(edit)
    } else {
        history.undo(edit)
    };
    let after = ids(history);
    match next {
        None => assert_eq!(moved, None, "{context}"),
        Some(Move::GoTo) => assert!(moved.is_some(), "{context}"),
        Some(Move::Steps { first, count }) => {
            assert!(moved.is_some() && count > 0, "{context}");
            if redo {
                assert_eq!(history.log.end(), end + count, "{context}");
                assert_eq!(after[end], first, "{context}");
                match held {
                    // The steps after the end the change dropped, dropped again.
                    Some(_) => assert_eq!(after.len(), end + count, "{context}"),
                    None => assert_eq!(after, before, "{context}"),
                }
            } else {
                let start = end - count;
                assert_eq!(history.log.end(), start, "{context}");
                assert_eq!(before[start], first, "{context}");
                match held {
                    // The change's steps out, the steps they dropped back.
                    Some(dropped) => {
                        assert_eq!(after.len(), before.len() - count + dropped, "{context}");
                        assert_eq!(after[..start], before[..start], "{context}");
                    }
                    None => assert_eq!(after, before, "{context}"),
                }
            }
        }
    }
    moved
}

/// Whether `end` is at the end of a change's steps: Original, or the step before it the
/// last of its batch, or in none.
fn last_of_change(log: &Log, end: usize) -> bool {
    let steps = log.steps();
    let Some(last) = end.checked_sub(1).map(|at| &steps[at]) else {
        return true;
    };
    last.batch.is_none() || steps.get(end).is_none_or(|next| next.batch != last.batch)
}

/// `edit` is the log's at `end` by the edit each change left, `left`: a step in no batch's,
/// its change's; inside a batch, or at the end of one whose last steps an edit after a
/// `GoTo` dropped, from the edit before the batch it changes only the batch's steps up to
/// `end`, each as the change left it.
fn assert_left(
    log: &Log,
    end: usize,
    edit: &Edit,
    left: &BTreeMap<StepId, (Clock, Edit)>,
    context: &str,
) {
    let steps = log.steps();
    let Some(step) = end.checked_sub(1).map(|at| &steps[at]) else {
        assert_eq!(*edit, Edit::default(), "{context}");
        return;
    };
    let left = &left[&step.id].1;
    if step.batch.is_none() {
        assert_eq!(edit, left, "{context}");
        return;
    }
    let start = log.batch_start(end).unwrap();
    let keys: Vec<String> = (steps[start..end].iter())
        .flat_map(|step| step.change.entries())
        .map(|entry| format!("{:?}", entry.key()))
        .collect();
    let done = |entry: &Entry| keys.contains(&format!("{:?}", entry.key()));
    for entry in Entry::between(&log.fold_to(start), edit) {
        assert!(done(&entry), "{context}: {entry:?} changed");
    }
    for entry in Entry::between(edit, left) {
        assert!(!done(&entry), "{context}: {entry:?} not as left");
    }
}

/// Records `edit` as what each step of `history`'s log not seen before, or whose clock
/// moved on as a change merged into it, left: a change's steps are made of the edit as it
/// stands.
fn record(history: &PhotoHistory, edit: &Edit, left: &mut BTreeMap<StepId, (Clock, Edit)>) {
    for step in history.log.steps() {
        let seen = (left.get(&step.id)).is_some_and(|(clock, _)| *clock == step.clock);
        if !seen {
            left.insert(step.id, (step.clock, edit.clone()));
        }
    }
}

/// An edit with Exposure's exposure at `ev`, kept.
fn exposed(ev: f32) -> Edit {
    let mut edit = Edit::default();
    let image = testing::image();
    (edit.set_control(&Exposure, "exposure", ControlValue::Number(ev), &image)).unwrap();
    edit
}

/// A history read from a file holding `edit`: a step of each module and operation it
/// holds, sharing a batch, as a version 1 file reads (the sidecar's tests hold that
/// reading), of the unknown machine's clock, which is the copy's vector.
fn reopened(edit: &Edit) -> PhotoHistory {
    let clock = Clock::at(1_773_497_331_120, None, MachineId::UNKNOWN);
    let mut log = Log::default();
    let changes = Change::between(&Edit::default(), edit, &[]);
    log.add_change(changes, StepSource::You, None, clock);
    PhotoHistory::read(log, vec![clock])
}

/// The steps of `history`'s log.
fn ids(history: &PhotoHistory) -> Vec<StepId> {
    history.log.steps().iter().map(|step| step.id).collect()
}

/// Undo or Redo moving the one step `id`.
fn one(id: StepId) -> Move {
    Move::Steps {
        first: id,
        count: 1,
    }
}

/// Undo of a step added takes it out of the log, as darktable's undo restores the history
/// without it, and Redo puts it back; a new step then clears what Redo would put back.
#[test]
fn undo_takes_a_step_out_of_the_log_and_redo_puts_it_back() {
    let mut history = reopened(&Edit::default());
    for ev in [1.0, 2.0] {
        history.complete(&exposed(ev), StepSource::You, None);
    }
    let two = ids(&history);
    assert_eq!(history.undo(&exposed(2.0)), Some(exposed(1.0)));
    assert_eq!((ids(&history), history.log.end()), (two[..1].to_vec(), 1));
    assert_eq!(history.next(false), Some(one(two[1])));
    assert_eq!(history.redo(&exposed(1.0)), Some(exposed(2.0)));
    assert_eq!((ids(&history), history.log.end()), (two.clone(), 2));
    assert_eq!(history.undo(&exposed(2.0)), Some(exposed(1.0)));
    history.complete(&exposed(3.0), StepSource::You, None);
    assert_eq!(history.redo(&exposed(3.0)), None);
    assert_eq!(ids(&history)[0], two[0]);
}

/// With the end below the last step, as Undo's fallback leaves it after reopening, an edit
/// drops the steps after the end, and Undo brings them back with the end where it was.
#[test]
fn an_edit_below_the_end_drops_the_later_steps_which_undo_brings_back() {
    let mut history = reopened(&Edit::default());
    for ev in [1.0, 2.0, 3.0] {
        history.complete(&exposed(ev), StepSource::You, None);
    }
    // As reopened, with no records.
    history.done.clear();
    let three = ids(&history);
    assert_eq!(history.undo(&exposed(3.0)), Some(exposed(2.0)));
    assert_eq!(history.undo(&exposed(2.0)), Some(exposed(1.0)));
    assert_eq!((ids(&history), history.log.end()), (three.clone(), 1));
    // An edit at step 1 drops steps 2 and 3, and clears Redo.
    history.complete(&exposed(-1.0), StepSource::You, None);
    assert_eq!((ids(&history).len(), history.log.end()), (2, 2));
    let new = ids(&history)[1];
    assert_eq!(history.redo(&exposed(-1.0)), None);
    // Undo brings them back, the end where it was.
    assert_eq!(history.undo(&exposed(-1.0)), Some(exposed(1.0)));
    assert_eq!((ids(&history), history.log.end()), (three, 1));
    // Redo puts the new step back first, then, with no record, moves the end.
    assert_eq!(history.next(false), Some(one(new)));
    assert_eq!(history.redo(&exposed(1.0)), Some(exposed(-1.0)));
    assert_eq!(history.undo(&exposed(-1.0)), Some(exposed(1.0)));
    history.undone.clear();
    assert_eq!(history.redo(&exposed(1.0)), Some(exposed(2.0)));
    assert_eq!(history.redo(&exposed(2.0)), Some(exposed(3.0)));
}

/// After reopening, with no records: Undo past the done stack moves the end, Redo reverses
/// it, and a Redo of an undone record still there puts its step back.
#[test]
fn the_fallback_mixed_with_records() {
    let mut history = reopened(&exposed(1.0));
    let read = history.log.steps()[0].id;
    assert_eq!(history.next(true), Some(one(read)));
    assert_eq!(history.next(false), None);
    history.complete(&exposed(2.0), StepSource::You, None);
    let added = history.log.steps()[1].id;
    // The record first, then the fallback over the step read.
    assert_eq!(history.undo(&exposed(2.0)), Some(exposed(1.0)));
    assert_eq!(history.next(true), Some(one(read)));
    assert_eq!(history.undo(&exposed(1.0)), Some(Edit::default()));
    assert_eq!(
        (history.log.end(), history.undo(&Edit::default())),
        (0, None)
    );
    assert_eq!(history.next(true), None);
    // Redo reverses the fallback, then puts the record's step back.
    assert_eq!(history.next(false), Some(one(read)));
    assert_eq!(history.redo(&Edit::default()), Some(exposed(1.0)));
    assert_eq!(history.next(false), Some(one(added)));
    assert_eq!(history.redo(&exposed(1.0)), Some(exposed(2.0)));
    assert_eq!((history.log.end(), history.redo(&exposed(2.0))), (2, None));
    // Undo takes back the step, then the fallback's move, recorded on the done stack.
    assert_eq!(history.undo(&exposed(2.0)), Some(exposed(1.0)));
    assert_eq!(history.undo(&exposed(1.0)), Some(Edit::default()));
    // The fallback forward with no undone record: back to the empty stack, Redo moves
    // the end once each, and Undo takes each move back.
    let mut fresh = reopened(&exposed(1.0));
    fresh.complete(&exposed(2.0), StepSource::You, None);
    fresh.done.clear();
    fresh.log.set_end(0);
    fresh.stepped = Edit::default();
    assert_eq!(fresh.redo(&Edit::default()), Some(exposed(1.0)));
    assert_eq!(fresh.redo(&exposed(1.0)), Some(exposed(2.0)));
    assert_eq!(fresh.redo(&exposed(2.0)), None);
    assert_eq!(fresh.undo(&exposed(2.0)), Some(exposed(1.0)));
    assert_eq!(fresh.redo(&exposed(1.0)), Some(exposed(2.0)));
}

/// `GoTo` moves the end just after a step, or to Original, and Undo takes it back. After it
/// an edit drops the later steps, which Undo brings back, then the end where `GoTo` found
/// it. A step not in the log is refused; the end where it is already changes nothing.
#[test]
fn go_to_moves_the_end_and_undo_takes_it_back_with_the_steps_an_edit_dropped() {
    let mut history = reopened(&Edit::default());
    for ev in [1.0, 2.0, 3.0] {
        history.complete(&exposed(ev), StepSource::You, None);
    }
    let three = ids(&history);
    assert_eq!(
        history.go_to(&exposed(3.0), Some(three[0])),
        Ok(Some(exposed(1.0)))
    );
    assert_eq!((ids(&history), history.log.end()), (three.clone(), 1));
    assert_eq!(history.next(true), Some(Move::GoTo));
    assert_eq!(history.undo(&exposed(1.0)), Some(exposed(3.0)));
    assert_eq!(history.next(false), Some(Move::GoTo));
    assert_eq!(history.redo(&exposed(3.0)), Some(exposed(1.0)));
    // An edit there drops steps 2 and 3; Undo brings them back, then the end.
    history.complete(&exposed(-1.0), StepSource::You, None);
    assert_eq!(ids(&history).len(), 2);
    assert_eq!(history.undo(&exposed(-1.0)), Some(exposed(1.0)));
    assert_eq!((ids(&history), history.log.end()), (three.clone(), 1));
    assert_eq!(history.undo(&exposed(1.0)), Some(exposed(3.0)));
    assert_eq!(history.log.end(), 3);
    // Original; there again, nothing; a step not in the log, refused.
    assert_eq!(
        history.go_to(&exposed(3.0), None),
        Ok(Some(Edit::default()))
    );
    assert_eq!(history.go_to(&Edit::default(), None), Ok(None));
    let unknown = StepId(!three[0].0);
    assert_eq!(history.go_to(&Edit::default(), Some(unknown)), Err(unknown));
    assert_eq!((ids(&history), history.log.end()), (three, 0));
}

#[test]
fn undo_makes_a_change_not_yet_a_step_one_first_and_takes_it_back() {
    let mut history = reopened(&Edit::default());
    history.complete(&exposed(1.0), StepSource::You, None);
    // A drag under way, or a `Set`, not yet a step.
    let pending = exposed(1.5);
    assert_eq!(history.undo(&pending), Some(exposed(1.0)));
    assert_eq!(history.log.steps().len(), 1);
    assert_eq!(history.redo(&exposed(1.0)), Some(pending));
    assert_eq!(history.log.steps().len(), 2);
}

/// `edit` with the module `b`'s control at `x`.
fn with_b(edit: &Edit, x: f32) -> Edit {
    let mut edit = edit.clone();
    (edit.modules.entry("b".to_owned()).or_default()).set("x", ControlValue::Number(x));
    edit
}

fn module(id: &str) -> Wrote {
    Wrote::Module(id.to_owned())
}

/// A change of two modules is a step of each, in the order written, of its source and note,
/// sharing a batch: one Undo takes both back and one Redo puts both back, and, with no
/// records, as after reopening, Undo's and Redo's fallback moves over both.
#[test]
fn a_change_of_two_modules_is_two_steps_in_one_batch_which_undo_and_redo_move_whole() {
    let mut history = reopened(&Edit::default());
    history.complete(&exposed(1.0), StepSource::You, None);
    let both = with_b(&exposed(2.0), 1.0);
    let note = Some(Note::Text("Style".to_owned()));
    history.wrote(None, vec![module("b"), module("exposure")], Instant::now());
    assert!(history.complete(&both, StepSource::Style, note.clone()));
    let steps = history.log.steps();
    let made: Vec<_> = (steps.iter())
        .map(|step| (format!("{:?}", step.change.entries()[0].key()), step.source))
        .collect();
    assert_eq!(
        made[1..],
        [
            (r#"Module("b")"#.to_owned(), StepSource::Style),
            (r#"Module("exposure")"#.to_owned(), StepSource::Style),
        ]
    );
    let batch = steps[1].batch;
    assert!(batch.is_some() && steps[2].batch == batch && steps[0].batch.is_none());
    assert!(steps[1..].iter().all(|step| step.note == note));
    let (first, second) = (steps[1].id, steps[2].id);
    let both_steps = Move::Steps { first, count: 2 };
    assert_eq!(history.next(true), Some(both_steps));
    assert_eq!(history.undo(&both), Some(exposed(1.0)));
    assert_eq!(history.log.steps().len(), 1);
    assert_eq!(history.next(false), Some(both_steps));
    assert_eq!(history.redo(&exposed(1.0)), Some(both.clone()));
    assert_eq!(ids(&history)[1..], [first, second]);
    // With no records: the fallback over the whole batch, each way.
    history.done.clear();
    assert_eq!(history.next(true), Some(both_steps));
    assert_eq!(history.undo(&both), Some(exposed(1.0)));
    assert_eq!(history.log.end(), 1);
    history.undone.clear();
    assert_eq!(history.next(false), Some(both_steps));
    assert_eq!(history.redo(&exposed(1.0)), Some(both.clone()));
    assert_eq!(history.log.end(), 3);
}

/// A change of one target that touched another module too, as a tone mapper turning another
/// off: a batch, its written module first, which the target's next change does not merge
/// into.
#[test]
fn a_batch_s_step_is_merged_into_by_none() {
    let start = Instant::now();
    let mut history = reopened(&Edit::default());
    history.wrote(Some(exposure()), vec![module("exposure")], start);
    history.complete(&with_b(&exposed(1.0), 1.0), StepSource::You, None);
    let steps = history.log.steps();
    assert_eq!(steps.len(), 2);
    assert!(steps[0].batch.is_some());
    assert!(matches!(
        &steps[0].change,
        Change::Entry(Entry::Module { module, .. }) if module == "exposure"
    ));
    history.wrote(
        Some(exposure()),
        vec![module("exposure")],
        start + secs(0.5),
    );
    history.complete(&with_b(&exposed(2.0), 1.0), StepSource::You, None);
    assert_eq!(history.log.steps().len(), 3);
    assert_eq!(history.log.steps()[2].batch, None);
}

/// `GoTo` lands on a step inside a batch, as darktable's click on any item: the batch's
/// steps up to it apply. Undo takes it back and Redo puts it back; with no records, Undo's
/// fallback moves back over the part that applied and Redo's forward to the batch's end. An
/// edit there drops the rest of the batch, which Undo brings back.
#[test]
fn go_to_lands_inside_a_batch() {
    let mut history = reopened(&Edit::default());
    history.complete(&exposed(1.0), StepSource::You, None);
    let both = with_b(&exposed(2.0), 1.0);
    history.wrote(None, vec![module("b"), module("exposure")], Instant::now());
    history.complete(&both, StepSource::You, None);
    let three = ids(&history);
    // Inside: `b` applied, Exposure's second step not.
    let inside = with_b(&exposed(1.0), 1.0);
    assert_eq!(
        history.go_to(&both, Some(three[1])),
        Ok(Some(inside.clone()))
    );
    assert_eq!(history.log.end(), 2);
    assert_eq!(history.undo(&inside), Some(both.clone()));
    assert_eq!(history.redo(&both), Some(inside.clone()));
    // With no records, as after reopening.
    history.done.clear();
    assert_eq!(history.next(true), Some(one(three[1])));
    assert_eq!(history.undo(&inside), Some(exposed(1.0)));
    // Its Redo puts back the part it took; with no record, Redo's fallback the whole.
    assert_eq!(history.next(false), Some(one(three[1])));
    history.undone.clear();
    let rest = Move::Steps {
        first: three[1],
        count: 2,
    };
    assert_eq!(history.next(false), Some(rest));
    assert_eq!(history.redo(&exposed(1.0)), Some(both.clone()));
    // An edit inside the batch drops its rest; Undo brings it back.
    assert_eq!(
        history.go_to(&both, Some(three[1])),
        Ok(Some(inside.clone()))
    );
    let edited = with_b(&exposed(1.0), 2.0);
    history.complete(&edited, StepSource::You, None);
    assert_eq!(history.log.steps().len(), 3);
    assert_ne!(ids(&history)[2], three[2]);
    assert_eq!(history.log.fold(), edited);
    assert_eq!(history.undo(&edited), Some(inside));
    assert_eq!(ids(&history), three);
}

/// The target of Exposure's exposure control, which [`exposed`] writes.
fn exposure() -> Target {
    Target::Control {
        module: "exposure".to_owned(),
        control: "exposure".to_owned(),
    }
}

/// A step of `exposure` written at a time, its first frame published `published` after it
/// where given, then a change of it written `after` it: whether the change merged.
fn merges(published: Option<Duration>, after: Duration) -> bool {
    let start = Instant::now();
    let mut history = reopened(&Edit::default());
    history.wrote(Some(exposure()), vec![module("exposure")], start);
    history.complete(&exposed(1.0), StepSource::You, None);
    if let Some(published) = published {
        history.published(start + published);
    }
    history.wrote(Some(exposure()), vec![module("exposure")], start + after);
    history.complete(&exposed(2.0), StepSource::You, None);
    history.log.steps().len() == 1
}

fn secs(secs: f64) -> Duration {
    Duration::from_secs_f64(secs)
}

/// A change merges into the step before it of its target when its first write comes less
/// than 10 s after the step's first and, once a frame including the step's last change is
/// published, less than 2 s after that frame; until then only the 10 s count. At either
/// time, it is a step of its own.
#[test]
fn a_change_merges_by_the_two_times_and_splits_at_each() {
    assert!(merges(None, secs(9.999)));
    assert!(!merges(None, MERGE_WITHIN));
    assert!(merges(Some(secs(0.5)), secs(2.499)));
    assert!(!merges(Some(secs(0.5)), secs(0.5) + REVIEW));
    // Within 2 s of the frame, but not within 10 s of the step's first.
    assert!(merges(Some(secs(9.0)), secs(9.999)));
    assert!(!merges(Some(secs(9.0)), MERGE_WITHIN));
}

/// The 10 s count from the step's first write, however many changes merged into it since,
/// and the 2 s from the first frame including the last of them. A merged step keeps its id
/// and takes the change's state and the latest clock, adding no undo record: one Undo takes
/// it back whole.
#[test]
fn a_merged_step_keeps_its_id_and_one_undo_takes_it_back_whole() {
    let start = Instant::now();
    let mut history = reopened(&Edit::default());
    let nudge = |history: &mut PhotoHistory, ev: f32, after: f64| {
        history.wrote(
            Some(exposure()),
            vec![module("exposure")],
            start + secs(after),
        );
        history.complete(&exposed(ev), StepSource::You, None);
        history.published(start + secs(after + 0.1));
        ids(history)
    };
    let first = nudge(&mut history, 0.5, 0.0);
    let clock = history.log.steps()[0].clock;
    // Each less than 2 s after the frame of the one before, more than 2 s after the first's.
    for (ev, after) in [(1.0, 1.5), (1.5, 3.0), (2.0, 4.5)] {
        assert_eq!(nudge(&mut history, ev, after), first);
    }
    let step = &history.log.steps()[0];
    assert!(step.clock > clock);
    assert_eq!(
        step.change.entries(),
        Entry::between(&Edit::default(), &exposed(2.0))
    );
    assert_eq!(history.done.len(), 1);
    // Ten seconds after the step's first write: a step of its own.
    for (ev, after) in [(2.25, 6.0), (2.5, 7.5), (2.75, 9.0)] {
        assert_eq!(nudge(&mut history, ev, after), first);
    }
    let split = nudge(&mut history, 3.0, 10.0);
    assert_eq!((split.len(), split[0]), (2, first[0]));
    // One Undo each.
    assert_eq!(history.undo(&exposed(3.0)), Some(exposed(2.75)));
    assert_eq!(history.undo(&exposed(2.75)), Some(Edit::default()));
    assert_eq!(history.undo(&Edit::default()), None);
}

/// No merge across another target, a control or a preset row, another source, a note, a
/// change of several targets or of none, nor another command between; and a step of
/// Style or Assistant, or noted, is merged into by none.
#[test]
fn no_merge_across_another_target_source_note_or_command() {
    let preset = Target::Preset {
        module: "exposure".to_owned(),
        preset: "look".to_owned(),
    };
    let black = Target::Control {
        module: "exposure".to_owned(),
        control: "black".to_owned(),
    };
    let note = || Some(Note::Text("Brighter".to_owned()));
    // A change after a step of `exposure` by the person with no note, both sent at once,
    // `edit` the edit the step left: whether it merged.
    let merged = |change: &dyn Fn(&mut PhotoHistory, &Edit)| {
        let mut history = reopened(&Edit::default());
        history.wrote(Some(exposure()), vec![module("exposure")], Instant::now());
        history.complete(&exposed(1.0), StepSource::You, None);
        change(&mut history, &exposed(1.0));
        history.log.steps().len() == 1
    };
    let you = |target: Option<Target>| {
        move |history: &mut PhotoHistory, _: &Edit| {
            history.wrote(target.clone(), vec![module("exposure")], Instant::now());
            history.complete(&exposed(2.0), StepSource::You, None);
        }
    };
    assert!(merged(&you(Some(exposure()))));
    assert!(!merged(&you(Some(black.clone()))));
    assert!(!merged(&you(Some(preset))));
    assert!(!merged(&you(None)));
    assert!(!merged(&|history, _| {
        history.wrote(Some(exposure()), vec![module("exposure")], Instant::now());
        history.wrote(
            Some(black.clone()),
            vec![module("exposure")],
            Instant::now(),
        );
        history.complete(&exposed(2.0), StepSource::You, None);
    }));
    for (source, note) in [
        (StepSource::Auto, None),
        (StepSource::You, note()),
        (StepSource::Assistant, None),
    ] {
        assert!(!merged(&|history, _| {
            history.wrote(Some(exposure()), vec![module("exposure")], Instant::now());
            history.complete(&exposed(2.0), source, note.clone());
        }));
    }
    // A command between, then the same change.
    assert!(!merged(&|history, edit| {
        history.interrupted(edit);
        you(Some(exposure()))(history, edit);
    }));
    // Undo and Redo between.
    assert!(!merged(&|history, edit| {
        history.undo(edit);
        history.redo(&Edit::default());
        you(Some(exposure()))(history, edit);
    }));
    // Auto merges with Auto; a step of Style or Assistant, or with a note, is merged into
    // by none, even of its own source and note.
    for (source, note, merges) in [
        (StepSource::Auto, None, true),
        (StepSource::Style, None, false),
        (StepSource::Assistant, None, false),
        (StepSource::You, note(), false),
    ] {
        let mut history = reopened(&Edit::default());
        for ev in [1.0, 2.0] {
            history.wrote(Some(exposure()), vec![module("exposure")], Instant::now());
            history.complete(&exposed(ev), source, note.clone());
        }
        assert_eq!(
            history.log.steps().len() == 1,
            merges,
            "{source:?} {note:?}"
        );
    }
}

/// A gesture cancelled leaves nothing written, and is a command between: the next change,
/// of the step's target, is a step of its own, timed from its own first write.
#[test]
fn a_cancelled_gesture_is_forgotten() {
    let start = Instant::now();
    let mut history = reopened(&Edit::default());
    history.wrote(Some(exposure()), vec![module("exposure")], start);
    history.complete(&exposed(1.0), StepSource::You, None);
    // A drag begun at once, cancelled: the edit back where the step left it.
    history.wrote(Some(exposure()), vec![module("exposure")], start);
    history.interrupted(&exposed(1.0));
    assert!(history.written.is_none() && history.merge.is_none());
    // The next change, 12 s on, merges with the one after it: timed from its own first.
    history.wrote(
        Some(exposure()),
        vec![module("exposure")],
        start + secs(12.0),
    );
    history.complete(&exposed(2.0), StepSource::You, None);
    history.wrote(
        Some(exposure()),
        vec![module("exposure")],
        start + secs(13.0),
    );
    history.complete(&exposed(3.0), StepSource::You, None);
    assert_eq!(history.log.steps().len(), 2);
}

/// The fact's budgets (`docs/history.md`, The `History` event): at 200 and at 1,000 steps,
/// each one control changed, building the fact a batch sends takes under 1 ms, and its
/// JSON is to be under 256 kB. A measurement, run on its own:
/// `cargo test --release -p apertrust-engine the_fact_s_budgets -- --ignored --nocapture`.
/// The JSON is printed, not held to its budget: at 1,000 steps it is over it, which the
/// planned fallback answers (`docs/history.md`).
#[test]
#[ignore = "a measurement, run on its own in release"]
fn the_fact_s_budgets() {
    let image = testing::image();
    let modules = apertrust_modules::MODULES;
    for steps in [200, 1000] {
        let mut history = reopened(&Edit::default());
        let ev = |i: usize| -1.0 + (i % 300) as f32 * 0.01;
        for i in 0..steps {
            history.complete(&exposed(ev(i)), StepSource::You, None);
        }
        // Every step described anew, as for a log read with the photo.
        let cold = median(
            (0..5)
                .map(|_| {
                    history.described = Described::default();
                    timed(|| history.fact(1, modules, OPERATIONS, &image)).0
                })
                .collect(),
        );
        // A batch's: one more step, the one before it taken back first so the log keeps
        // its length, then the fact.
        history.fact(1, modules, OPERATIONS, &image);
        let mut fact = None;
        let warm = median(
            (steps..steps + 51)
                .map(|i| {
                    let last = history.stepped.clone();
                    history.undo(&last);
                    history.complete(&exposed(ev(i)), StepSource::You, None);
                    let (time, built) = timed(|| history.fact(1, modules, OPERATIONS, &image));
                    fact = Some(built);
                    time
                })
                .collect(),
        );
        let fact = fact.unwrap();
        assert_eq!(fact.steps.len(), steps);
        let bytes = json(&fact);
        // As the derive writes them with `skip_serializing_if = "Option::is_none"`.
        let without_nulls = bytes
            .replace(r#""note":null,"#, "")
            .replace(r#""batch":null,"#, "")
            .replace(r#""enabled":null,"#, "");
        println!(
            "{steps} steps: a batch's fact {:.3} ms, every step described {:.3} ms, JSON {:.1} kB \
             ({} bytes a step), {:.1} kB without its nulls",
            ms(warm),
            ms(cold),
            bytes.len() as f64 / 1000.0,
            bytes.len() / steps,
            without_nulls.len() as f64 / 1000.0,
        );
        assert!(warm < Duration::from_millis(1), "{warm:?}");
    }
}

/// How long `work` took, and what it gave.
fn timed<T>(work: impl FnOnce() -> T) -> (Duration, T) {
    let start = Instant::now();
    let given = work();
    (start.elapsed(), given)
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

/// `history` as JSON, as serde's derive writes these types with their ids, sources and
/// machine as text: what the remote protocol will send, to measure its size.
fn json(history: &History) -> String {
    let mut out = String::new();
    let moves = |next: &Option<Move>| match next {
        Some(Move::Steps { first, count }) => {
            format!(r#"{{"Steps":{{"first":"{first}","count":{count}}}}}"#)
        }
        Some(Move::GoTo) => r#""GoTo""#.to_owned(),
        None => "null".to_owned(),
    };
    write!(
        out,
        r#"{{"through":{},"end":{},"defaults":[{}],"undo":{},"redo":{},"steps":["#,
        history.through,
        history.end,
        (history.defaults.iter())
            .map(|id| format!("{id:?}"))
            .collect::<Vec<_>>()
            .join(","),
        moves(&history.undo),
        moves(&history.redo),
    )
    .unwrap();
    for (index, step) in history.steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        step_json(&mut out, step);
    }
    out.push_str("]}");
    out
}

fn step_json(out: &mut String, step: &HistoryStep) {
    let note = match &step.note {
        None => "null".to_owned(),
        Some(Note::Text(text)) => format!(r#"{{"Text":{text:?}}}"#),
        Some(Note::Message { id, args }) => {
            let args: Vec<String> = args.iter().map(|(k, v)| format!("[{k:?},{v:?}]")).collect();
            format!(
                r#"{{"Message":{{"id":{id:?},"args":[{}]}}}}"#,
                args.join(",")
            )
        }
    };
    let batch = step
        .batch
        .map_or("null".to_owned(), |b| format!(r#""{b}""#));
    write!(
        out,
        r#"{{"id":"{}","clock":{{"when":{},"counter":{},"machine":"{}"}},"source":"{}","note":{note},"batch":{batch},"changes":["#,
        step.id,
        step.clock.when,
        step.clock.counter,
        step.clock.machine,
        step.source.name(),
    )
    .unwrap();
    for (index, change) in step.changes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let enabled = change
            .enabled
            .map_or("null".to_owned(), |[a, b]| format!("[{a},{b}]"));
        write!(
            out,
            r#"{{"module":"{}","enabled":{enabled},"controls":["#,
            change.module
        )
        .unwrap();
        for (index, control) in change.controls.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let mark =
                |mark: Option<Mark>| mark.map_or("null".to_owned(), |m| format!(r#""{m:?}""#));
            write!(
                out,
                r#"{{"control":"{}","before":{},"after":{},"mark":[{},{}]}}"#,
                control.control,
                value_json(&control.before),
                value_json(&control.after),
                mark(control.mark[0]),
                mark(control.mark[1]),
            )
            .unwrap();
        }
        out.push_str("]}");
    }
    out.push_str("]}");
}

fn value_json(value: &ControlValue) -> String {
    match value {
        ControlValue::Number(number) => format!(r#"{{"Number":{number}}}"#),
        ControlValue::Choice(index) => format!(r#"{{"Choice":{index}}}"#),
        ControlValue::Bool(flag) => format!(r#"{{"Bool":{flag}}}"#),
        ControlValue::Points(points) => {
            let points: Vec<String> = points.iter().map(|[x, y]| format!("[{x},{y}]")).collect();
            format!(r#"{{"Points":[{}]}}"#, points.join(","))
        }
    }
}
