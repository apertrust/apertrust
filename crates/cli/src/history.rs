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

//! `history`: the edit's history saved beside a raw, listed and moved on the sidecar alone,
//! with no session and no GPU (decision 78; `docs/history.md`, The CLI), through the
//! engine's `history` and `sidecar` as `edit` changes it.
//!
//! Each move is one write of the sidecar, the copy's clock vector advanced with a clock of
//! its own, only over the file as it was read. The CLI keeps no undo records, so Undo and
//! Redo are the session's fallback, which moves the end; Compress, Truncate and Discard
//! History rewrite the log by the engine's own functions, a log left with no step written
//! with none, as the session writes it, the default steps given again as the photo next
//! opens.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use apertrust_engine::history::{
    Clock, Described, HistoryStep, Log, MachineId, Note, StepSource, advance, default_modules,
    latest,
};
use apertrust_engine::sidecar::{self, Reason, ViewOnly};
use apertrust_engine::{edit::Edit, raw};
use apertrust_library::clock::format_when;
use apertrust_modules::{ControlValue, Module};
use apertrust_plugin::{ImageContext, Operation};

/// `history FILE` and its moves, one at most.
#[derive(Debug, clap::Args)]
pub struct Args {
    pub file: PathBuf,
    #[command(flatten)]
    pub moves: Moves,
}

/// The moves `history` takes, one at most; with none it lists the history.
#[derive(Debug, Default, clap::Args)]
#[group(multiple = false)]
pub struct Moves {
    /// Move the end back over the step before it, and the rest of its batch, as the app's
    /// Undo after reopening the photo; never over a default step.
    #[arg(long)]
    undo: bool,
    /// Move the end forward over the step after it, and the rest of its batch.
    #[arg(long)]
    redo: bool,
    /// Move the end just after step N as the listing numbers it, 0 for Original. The later
    /// steps stay, for --redo; the next edit drops them.
    #[arg(long, value_name = "N")]
    go_to: Option<usize>,
    /// Keep the default steps and, of the steps up to the end, each module's last, as
    /// darktable's compress history stack; the steps after the end are dropped. The photo
    /// shows what it showed, but at Original, where every step is dropped.
    #[arg(long)]
    compress: bool,
    /// Drop the steps after the end, keeping every step up to it and the default steps, as
    /// darktable's Ctrl-click on compress history; at Original, every step.
    #[arg(long)]
    truncate: bool,
    /// Discard the history, as Discard History does: every step dropped, the edit reset,
    /// the photo given its default steps again as it next opens. The one move taken on an
    /// edit part of whose history cannot be read: its log starts afresh from the edit as
    /// read, and what could not be read goes with the old log.
    #[arg(long)]
    discard: bool,
}

/// One move of the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Undo,
    Redo,
    /// The end just after the step numbered this, 0 for Original.
    GoTo(usize),
    Compress,
    Truncate,
    Discard,
}

impl Moves {
    /// The move asked for, `None` for the listing.
    pub fn chosen(&self) -> Option<Move> {
        [
            (self.undo, Move::Undo),
            (self.redo, Move::Redo),
            (self.compress, Move::Compress),
            (self.truncate, Move::Truncate),
            (self.discard, Move::Discard),
        ]
        .into_iter()
        .find_map(|(asked, chosen)| asked.then_some(chosen))
        .or(self.go_to.map(Move::GoTo))
    }
}

/// Lists the history of the edit saved beside `photo`, after making `chosen` on it, if any.
pub fn run(
    photo: &Path,
    chosen: Option<Move>,
    operations: &[&'static dyn Operation],
    modules: &[&'static dyn Module],
) -> Result<()> {
    if !photo.is_file() {
        bail!("no file {}", photo.display());
    }
    let (log, view_only) = match chosen {
        Some(chosen) => {
            let made = make(photo, chosen, operations, modules)?;
            println!("{}", made.said);
            if made.written {
                println!("Wrote {}", sidecar::path_for(photo).display());
            }
            (made.log, None)
        }
        None => match crate::saved(photo, operations, modules)? {
            Some(loaded) => {
                let view_only = loaded.view_only();
                (loaded.log, view_only)
            }
            None => (Log::default(), None),
        },
    };
    let image = image_of(photo);
    let steps = Described::default().steps(&log, modules, operations, &image);
    let name = apertrust_base::log::file_name(photo);
    print!("{}", listing(&name, &steps, log.end()));
    if let Some(view_only) = view_only {
        println!("{}", viewing_only(&view_only));
    }
    Ok(())
}

/// What the listing says under an edit open for viewing only: why, and what changes it.
fn viewing_only(view_only: &ViewOnly) -> String {
    let what = match view_only.reason {
        Reason::Newer(_) => "this build changes nothing of it",
        Reason::Unreadable(_) => "only --discard changes it, starting its history afresh",
    };
    let said = format!("{view_only}; {what}");
    let mut letters = said.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// What a move made: the log the file now holds, what to say of it, and whether the file
/// was written.
#[derive(Debug)]
pub struct Made {
    pub log: Log,
    pub said: String,
    pub written: bool,
}

/// Makes `chosen` on the edit saved beside `photo` and writes it, where it changes
/// anything, only over the file as it was read ([`prepare`], [`Prepared::write`]).
pub fn make(
    photo: &Path,
    chosen: Move,
    operations: &[&'static dyn Operation],
    modules: &[&'static dyn Module],
) -> Result<Made> {
    prepare(photo, chosen, operations, modules)?.write()
}

/// A move made on the log read, still to be written over the file as it was read.
pub struct Prepared<'a> {
    photo: &'a Path,
    /// The file's stamp as it was read.
    read: crate::Stamp,
    /// The file as text, which the log is written into.
    file: sidecar::Edits,
    /// The log read, and the log the move leaves; `None` where it changes nothing.
    before: Log,
    after: Option<Log>,
    /// The copy's clock vector, advanced with the move's clock.
    clocks: Vec<Clock>,
    said: String,
    operations: &'a [&'static dyn Operation],
    modules: &'a [&'static dyn Module],
}

/// Reads the edit saved beside `photo` and makes `chosen` on its log, to be written.
/// Refused, writing nothing: on an edit a newer Apertrust wrote; on one part of whose
/// history cannot be read but for Discard, which starts its log afresh; and for a step the
/// log does not have.
pub fn prepare<'a>(
    photo: &'a Path,
    chosen: Move,
    operations: &'a [&'static dyn Operation],
    modules: &'a [&'static dyn Module],
) -> Result<Prepared<'a>> {
    let path = sidecar::path_for(photo);
    let read = crate::stamp(&path)?;
    let (file, before, mut clocks, afresh) = match crate::saved(photo, operations, modules)? {
        Some(loaded) => {
            let afresh = match loaded.view_only() {
                None => None,
                Some(view_only) if matches!(view_only.reason, Reason::Newer(_)) => {
                    return Err(view_only)
                        .with_context(|| format!("not changing {}", path.display()));
                }
                Some(_) if chosen == Move::Discard => Some(loaded.edit.clone()),
                Some(view_only) => {
                    return Err(view_only).with_context(|| {
                        format!(
                            "not changing {}: only --discard changes an edit whose history \
                             cannot be read, starting it afresh",
                            path.display()
                        )
                    });
                }
            };
            let clocks = loaded.clocks().to_vec();
            (loaded.file, loaded.log, clocks, afresh)
        }
        None => (sidecar::Edits::new(""), Log::default(), Vec::new(), None),
    };
    let machine = MachineId::current();
    let latest = latest(
        (before.steps().iter().map(|step| &step.clock)).chain(&clocks),
        machine,
    );
    let clock = Clock::now(latest.as_ref(), machine);
    let (after, said) = match afresh {
        Some(edit) => {
            let (after, said) = started_afresh(&edit, operations, modules, clock);
            (Some(after), said)
        }
        None => match moved(&before, chosen)? {
            Moved::To(after, said) => (Some(after), said),
            Moved::Nothing(said) => (None, said),
        },
    };
    advance(&mut clocks, clock);
    Ok(Prepared {
        photo,
        read,
        file,
        before,
        after,
        clocks,
        said,
        operations,
        modules,
    })
}

impl Prepared<'_> {
    /// Writes the move where it changes anything, and only where the file still is as it
    /// was read: refused, writing nothing, where another program changed it since.
    pub fn write(self) -> Result<Made> {
        let Some(after) = self.after else {
            return Ok(Made {
                log: self.before,
                said: self.said,
                written: false,
            });
        };
        let written = crate::write_history(
            self.photo,
            self.read,
            self.file,
            (&after, &self.clocks),
            (self.operations, self.modules),
        )?;
        if !written {
            bail!(
                "{} changed since it was read: not written; run history again",
                sidecar::path_for(self.photo).display()
            );
        }
        Ok(Made {
            log: after,
            said: self.said,
            written,
        })
    }
}

/// What a move does to a log.
#[derive(Debug, PartialEq)]
pub enum Moved {
    /// The log it leaves, and what to say of it.
    To(Log, String),
    /// Nothing, as the session's move that would change nothing, and why.
    Nothing(String),
}

/// `chosen` made on `log`, by the session's own rules (`docs/history.md`, Behaviour), or
/// an error for a step the log does not have.
pub fn moved(log: &Log, chosen: Move) -> Result<Moved> {
    let steps = log.steps().len();
    let end_said = |end: usize| match end {
        0 => "the end at Original".to_owned(),
        end => format!("the end at step {end}"),
    };
    let at = |end: usize| {
        let mut moved = log.clone();
        moved.set_end(end);
        moved
    };
    let nothing = |said: &str| Ok(Moved::Nothing(said.to_owned()));
    match chosen {
        Move::Undo => match log.undo_end() {
            Some(end) => Ok(Moved::To(
                at(end),
                format!("Undid {}: {}", numbers(end, log.end()), end_said(end)),
            )),
            None => nothing("Nothing to undo"),
        },
        Move::Redo => match log.redo_end() {
            Some(end) => Ok(Moved::To(
                at(end),
                format!("Redid {}: {}", numbers(log.end(), end), end_said(end)),
            )),
            None => nothing("Nothing to redo"),
        },
        Move::GoTo(end) if end > steps => bail!(
            "no step {end}: the history has {}; 0 is Original",
            counted(steps, "step")
        ),
        Move::GoTo(end) if end == log.end() => {
            Ok(Moved::Nothing(format!("Already at {}", at_step(end))))
        }
        Move::GoTo(end) => Ok(Moved::To(at(end), format!("Went to {}", at_step(end)))),
        Move::Compress | Move::Truncate => {
            let (rewritten, verb) = match chosen {
                Move::Compress => (log.compressed(), "Compressed"),
                _ => (log.truncated(), "Truncated"),
            };
            if rewritten == *log {
                return match chosen {
                    Move::Compress => nothing("Nothing to compress"),
                    _ => nothing("Nothing to truncate: no step of yours after the end"),
                };
            }
            // At Original every step goes, as Discard History's do.
            let said = match rewritten.steps().len() {
                0 => format!(
                    "{verb} the history from {} to none: {GIVEN_DEFAULTS}",
                    counted(steps, "step"),
                ),
                kept => format!(
                    "{verb} the history from {} to {kept}: {}",
                    counted(steps, "step"),
                    end_said(rewritten.end())
                ),
            };
            Ok(Moved::To(rewritten, said))
        }
        Move::Discard if steps == 0 => nothing("No history to discard"),
        Move::Discard => Ok(Moved::To(
            Log::default(),
            format!("Discarded the history: {GIVEN_DEFAULTS}"),
        )),
    }
}

/// The log Discard History starts an edit whose history cannot be read with, as the
/// session's ([`Log::afresh`]): a new photo's default steps and a step of each module and
/// operation `edit` holds as read, noted `history-discard`. Default steps alone, every one
/// applied, are written as no step, as a new photo's are; with the end among them, as they
/// are, so the modules the edit as read holds not applied stay so as the photo reopens.
fn started_afresh(
    edit: &Edit,
    operations: &[&'static dyn Operation],
    modules: &[&'static dyn Module],
    clock: Clock,
) -> (Log, String) {
    let defaults = default_modules(modules, operations);
    let afresh = Log::afresh(edit, &defaults, clock);
    let defaults_alone = (afresh.steps().iter()).all(|step| step.source == StepSource::Default);
    if defaults_alone && afresh.end() == afresh.steps().len() {
        return (
            Log::default(),
            format!("Discarded the history: {GIVEN_DEFAULTS}"),
        );
    }
    let said = format!(
        "Started the history afresh from the edit as read: {}",
        counted(afresh.steps().len(), "step")
    );
    (afresh, said)
}

/// What a move that leaves the log with no step says of it: the file holds none, and the
/// photo is given its default steps as it next opens, as a new photo is.
const GIVEN_DEFAULTS: &str = "the photo is given its default steps as it next opens";

/// The steps numbered after `from` up to `to`: "step 4" or "steps 4–6".
fn numbers(from: usize, to: usize) -> String {
    match to - from {
        1 => format!("step {to}"),
        _ => format!("steps {}–{to}", from + 1),
    }
}

/// "step N", or Original for 0.
fn at_step(end: usize) -> String {
    match end {
        0 => "Original".to_owned(),
        end => format!("step {end}"),
    }
}

/// "1 step", "3 steps".
fn counted(count: usize, what: &str) -> String {
    match count {
        1 => format!("1 {what}"),
        count => format!("{count} {what}s"),
    }
}

/// The photo's image context, which a control's default for the photo reads, from its
/// metadata alone, in milliseconds; where it cannot be read, a photo with none, so the
/// listing still shows, a default that depends on the photo then the definition's.
fn image_of(photo: &Path) -> ImageContext {
    let read = apertrust_raw::RawFile::open(photo).and_then(|file| file.metadata());
    match read {
        Ok((raw, _)) => raw::image_context(&raw),
        Err(error) => {
            tracing::debug!(target: "raw", "{}: no metadata, described without: {error}", photo.display());
            ImageContext::full(0, 0)
        }
    }
}

/// One line of the listing.
struct Row {
    /// The end's line, or Original's at end 0.
    end: bool,
    /// Its number, a group's range, or none.
    number: String,
    /// Inside a group.
    inside: bool,
    text: String,
    /// The step's time, none for a group's line or the undone mark.
    when: Option<String>,
}

/// The listing of `steps`, `end` of them applied, of the photo `name`, as the History tab
/// lists them in words, newest first: each step's number, module, what changed and its
/// time, its source where it is not the person's own, its note; a batch, one change of
/// several modules, under a line of its own with its range, source and note; the default
/// steps under theirs; then Original. The end's line is marked `>`, and the steps after it
/// are counted under them. Times are in UTC.
pub fn listing(name: &str, steps: &[Arc<HistoryStep>], end: usize) -> String {
    let mut text = match (steps.len(), end) {
        (0, _) => format!("{name}: no history\n"),
        (count, 0) => format!(
            "{name}: {}, at Original; times in UTC\n",
            counted(count, "step")
        ),
        (count, end) => format!(
            "{name}: {}, the end at step {end}; times in UTC\n",
            counted(count, "step")
        ),
    };
    let rows = rows(steps, end);
    let width = (rows.iter())
        .map(|row| row.number.chars().count())
        .max()
        .unwrap_or(1);
    let room = (rows.iter())
        .filter(|row| row.when.is_some())
        .map(|row| row.text.chars().count() + 2 * usize::from(row.inside))
        .max()
        .unwrap_or(0);
    for row in rows {
        let mark = if row.end { '>' } else { ' ' };
        let indent = if row.inside { "  " } else { "" };
        let mut line = format!("{mark} {:>width$}  {indent}{}", row.number, row.text);
        if let Some(when) = row.when {
            let used = row.text.chars().count() + indent.len();
            line.push_str(&" ".repeat(room - used + 2));
            line.push_str(&when);
        }
        text.push_str(line.trim_end());
        text.push('\n');
    }
    text
}

/// The listing's lines, newest first ([`listing`]).
fn rows(steps: &[Arc<HistoryStep>], end: usize) -> Vec<Row> {
    let defaults = (steps.iter())
        .take_while(|step| step.source == StepSource::Default)
        .count();
    let mut rows = Vec::new();
    let mut at = steps.len();
    while at > 0 {
        // The group the step at `at - 1` closes, oldest first: the default steps, a batch's
        // run of two or more, or the step alone.
        let last = at - 1;
        let first = if last < defaults {
            0
        } else {
            match steps[last].batch {
                Some(batch) => {
                    let before =
                        (steps[defaults..last].iter()).rposition(|step| step.batch != Some(batch));
                    before.map_or(defaults, |before| defaults + before + 1)
                }
                None => last,
            }
        };
        let group = first < last;
        if group {
            let text = if last < defaults {
                format!(
                    "defaults: {} every photo runs",
                    counted(at - first, "module")
                )
            } else {
                let mut text = counted(at - first, "step");
                text.push_str(&tagged(&steps[last]));
                text
            };
            rows.push(Row {
                end: false,
                number: format!("{}–{at}", first + 1),
                inside: false,
                text,
                when: None,
            });
        }
        for number in (first + 1..=at).rev() {
            let step = &steps[number - 1];
            let mut text = line(step);
            // The Defaults' own steps untagged, a group of one among them.
            if !group && number > defaults {
                text.push_str(&tagged(step));
            }
            rows.push(Row {
                end: number == end,
                number: number.to_string(),
                inside: group,
                text,
                when: Some(when(&step.clock)),
            });
            if number == end + 1 {
                rows.push(Row {
                    end: false,
                    number: String::new(),
                    inside: group,
                    text: undone(steps, end),
                    when: None,
                });
            }
        }
        at = first;
    }
    rows.push(Row {
        end: end == 0,
        number: "0".to_owned(),
        inside: false,
        text: "Original".to_owned(),
        when: None,
    });
    rows
}

/// The mark under the steps after the end: how many the next edit drops, and how many
/// default steps after the end it applies, which it keeps.
fn undone(steps: &[Arc<HistoryStep>], end: usize) -> String {
    let after = &steps[end..];
    let defaults = (after.iter())
        .filter(|step| step.source == StepSource::Default)
        .count();
    let dropped = after.len() - defaults;
    let them = |count: usize| if count == 1 { "it" } else { "them" };
    let mut said = Vec::new();
    if dropped > 0 {
        let them = them(dropped);
        said.push(format!("{dropped} undone: the next edit drops {them}"));
    }
    if defaults > 0 {
        let what = counted(defaults, "default step");
        let them = them(defaults);
        said.push(format!("{what} not applied: the next edit applies {them}"));
    }
    said.join("; ")
}

/// A step's module and what changed, as the History tab's line says it: Reset for Reset's
/// step, the module's id, or an operation's for its unowned parameters, or a module this
/// build does not know by its stored name; then what changed ([`changed`]).
fn line(step: &HistoryStep) -> String {
    if is_reset(step) {
        return "Reset".to_owned();
    }
    let Some(module) = step.changes.first() else {
        return step.source.name().to_owned();
    };
    match changed(step) {
        Some(changed) => format!("{} {changed}", module.module),
        None => module.module.to_string(),
    }
}

/// What a step changed, as the History tab's line says it after the module: one number its
/// control and which way it went, `shadows ↑`, the arrow alone where the control is the
/// module's own, as Exposure's exposure; any other control alone; several controls how
/// many. Nothing for Reset, a default step or a module this build does not know.
fn changed(step: &HistoryStep) -> Option<String> {
    if step.source == StepSource::Default || is_reset(step) {
        return None;
    }
    let module = step.changes.first()?;
    match module.controls.as_slice() {
        [] => None,
        [one] => {
            let arrow = match (&one.before, &one.after) {
                (ControlValue::Number(before), ControlValue::Number(after)) if after > before => {
                    Some("↑")
                }
                (ControlValue::Number(before), ControlValue::Number(after)) if after < before => {
                    Some("↓")
                }
                _ => None,
            };
            let control = Some(one.control).filter(|control| *control != module.module);
            match (control, arrow) {
                (Some(control), Some(arrow)) => Some(format!("{control} {arrow}")),
                (Some(control), None) => Some(control.to_owned()),
                (None, arrow) => arrow.map(str::to_owned),
            }
        }
        several => Some(counted(several.len(), "control")),
    }
}

/// A step's source where it is not the person's own, and its note but Reset's, each after a
/// `·`: ` · assistant · "Warmer"`. A default step in the Defaults group has no tag; one
/// after it, as an edit started afresh leaves, reads ` · default`.
fn tagged(step: &HistoryStep) -> String {
    let mut text = String::new();
    if step.source != StepSource::You {
        text.push_str(" · ");
        text.push_str(step.source.name());
    }
    match &step.note {
        Some(_) if is_reset(step) => {}
        Some(Note::Text(words)) => text.push_str(&format!(" · {words:?}")),
        Some(Note::Message { id, args }) => {
            text.push_str(" · ");
            text.push_str(id);
            if !args.is_empty() {
                let args: Vec<String> = (args.iter())
                    .map(|(name, value)| format!("{name}: {value:?}"))
                    .collect();
                text.push_str(&format!(" ({})", args.join(", ")));
            }
        }
        None => {}
    }
    text
}

/// Whether a step is Reset's, by its note.
fn is_reset(step: &HistoryStep) -> bool {
    matches!(&step.note, Some(Note::Message { id, .. }) if id == Note::RESET)
}

/// A step's time in UTC to the minute, `2026-11-14 10:05`; `no date` for a version 1 edit's
/// whose file's time was unknown.
fn when(clock: &Clock) -> String {
    if clock.when == 0 {
        return "no date".to_owned();
    }
    let written = format_when(clock.when);
    written.get(..16).unwrap_or(&written).replacen('T', " ", 1)
}

#[cfg(test)]
mod tests;
