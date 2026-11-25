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

//! The styles applied to a photo (decision 106, `docs/history.md`, The styles applied):
//! `ApplyStyle` and `ClearStyle`, each one change of source Style that leaves the list it
//! changes to the history, which its last step holds ([`super::history`]).
//!
//! A style is applied as a `Step` of a `SetTool` for each of its tools would be, `Reset`
//! first for Replace, and put at the top of the list with each tool's state before and
//! after it; applied again, a tool that still held what the earlier application set keeps
//! that one's state before it. Clear puts back each tool whose state is still the one the
//! style left, compared as stored, and takes the style off the list.
//!
//! One function applies a style's rules to a photo's edit and history, [`style_photo`],
//! whether the photo is open or not ([`style_log`], decision 109), so a style gives the
//! same log on a photo opened or not.

use std::time::Instant;

use apertrust_library::clock::{Clock, wall_clock};
use apertrust_plugin::ImageContext;
use apertrust_tools::{Moment, ProcessingTool, ToolValues};

use super::history::{PhotoHistory, as_read};
use super::{Effect, Engine, SessionError, StyleMode};
use crate::edit::{self, Edit, EditError};
use crate::history::{AppliedStyle, BatchId, Log, Note, StepSource, StyleTool, Wrote, on_top};

/// A style as [`super::Command::ApplyStyle`] names it: its name as applied, its file, its
/// mode, and each tool's values in the order it names them.
#[derive(Debug, Clone, Copy)]
pub(super) struct Style<'s> {
    pub(super) name: &'s str,
    pub(super) file: &'s str,
    pub(super) mode: StyleMode,
    pub(super) tools: &'s [(String, ToolValues)],
}

/// What one command applying a style gives every photo it changes (decision 109): its
/// batch, and the wall time read as it was applied, in milliseconds since 1970, each step's
/// clock taken at it after the photo's latest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Batch {
    pub(super) id: BatchId,
    pub(super) wall: u64,
}

/// [`super::Command::SetTool`]'s change of `edit`, as a style's too: the tool's values made
/// exactly `values` on the photo's `image`, each number its range moved warned of, then the
/// tool redrawn, as darktable's preset runs `gui_update` once its parameters are written.
pub(super) fn set_tool(
    edit: &mut Edit,
    (tools, image): (&[&dyn ProcessingTool], &ImageContext),
    tool: &str,
    values: &ToolValues,
) -> Result<(), EditError> {
    let found = edit::tool(tools, tool)?;
    for warning in edit.set_tool(found, values, image)? {
        tracing::warn!(target: "pipe", "{warning}");
    }
    edit.redraw(&[found], image);
    Ok(())
}

/// [`super::Command::Reset`]'s change of `edit`, as a Replace's too: every tool and operation
/// at its reset state.
pub(super) fn reset(edit: &mut Edit) -> Result<(), EditError> {
    *edit = Edit::default();
    Ok(())
}

/// A style's rules on one photo's `edit` and `history`, on the photo's `image`, the photo
/// open or not (decisions 106 and 109): `Reset` first for [`StyleMode::Replace`], then each
/// of the style's tools set as `SetTool` sets it, all or none, the first refusal returned
/// and nothing changed; a step of each tool it changes, of source Style noted
/// `history-style` with the style's name, each in the batch, at its wall time; the style
/// at the top of the list the change leaves, matched as [`AppliedStyle::is`] matches it,
/// with each tool's state before and after it. `edit` becomes the edit the change leaves,
/// its tools not applied as the history's end leaves them. Returns what it wrote, in order:
/// [`Wrote::All`] for a reset that changed the edit, then each tool whose values moved;
/// and whether it made any step.
pub(super) fn style_photo(
    history: &mut PhotoHistory,
    edit: &mut Edit,
    style: Style<'_>,
    (tools, image): (&[&'static dyn ProcessingTool], &ImageContext),
    batch: Batch,
) -> Result<(Vec<Wrote>, bool), EditError> {
    let replace = style.mode == StyleMode::Replace;
    // What the tools held just before: a Replace's reset leaves each at its reset state,
    // and the list empty.
    let (start, list) = match replace {
        true => (Edit::default(), Vec::new()),
        false => (edit.clone(), history.styles().to_vec()),
    };
    let mut styled = edit.clone();
    let mut wrote = Vec::new();
    // Each command as the session applies it on its own: its change, then the numbers a
    // moved range leaves outside brought back inside (decision 30).
    let mut command = |styled: &mut Edit,
                       change: &dyn Fn(&mut Edit) -> Result<(), EditError>,
                       writes: Wrote|
     -> Result<(), EditError> {
        let before = styled.clone();
        change(styled)?;
        styled.clamp(&before, tools, image)?;
        if *styled != before {
            wrote.push(writes);
        }
        Ok(())
    };
    if replace {
        command(&mut styled, &reset, Wrote::All)?;
    }
    // Each tool once, in the order the style first names it, its last values set.
    let mut order: Vec<&str> = Vec::new();
    for (tool, values) in style.tools {
        if !order.contains(&tool.as_str()) {
            order.push(tool);
        }
        let set = |edit: &mut Edit| set_tool(edit, (tools, image), tool, values);
        command(&mut styled, &set, Wrote::Tool(tool.clone()))?;
    }
    let state = |edit: &Edit, tool: &str| edit.tool(tool).cloned().unwrap_or_default();
    let set = (order.iter())
        .map(|tool| StyleTool {
            tool: (*tool).to_owned(),
            before: state(&start, tool),
            after: state(&styled, tool),
        })
        .collect();
    let earlier = list.iter().find(|kept| kept.is(style.name, style.file));
    let applied = AppliedStyle::applied(style.name, style.file, earlier, set);
    history.leave_styles(on_top(&list, applied));
    if replace {
        history.replacing();
        history.reset_styles();
    }
    // A change that changes nothing counts as none. Its time serves only a merge, which a
    // noted step never makes.
    if styled != *edit {
        history.wrote(None, wrote.clone(), Instant::now());
    }
    *edit = styled;
    let note = Some(Note::style(style.name));
    let stepped = history.complete(edit, (StepSource::Style, note), Some(batch.id), batch.wall);
    edit.unapplied = history.unapplied(edit);
    Ok((wrote, stepped))
}

/// A style's rules on a photo that is not open (decision 109): its log and its copy's clock
/// vector as its sidecar holds them, both empty for a photo never edited, which is given the
/// default steps `defaults` first, at the batch's wall time; the fold read as the open
/// reads it, clamped and redrawn on the photo's `image`, so a clamp is no step; then
/// [`style_photo`], as on the open photo, the steps after the end dropped, which no Undo
/// brings back. The log and vector to write, `None` where the log holds the batch already,
/// the batch having reached it, or the style changes nothing.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the apply-style job's, #373's next step")
)]
pub(super) fn style_log(
    (log, clocks): (Log, Vec<Clock>),
    defaults: &[&str],
    style: Style<'_>,
    (tools, image): (&[&'static dyn ProcessingTool], &ImageContext),
    batch: Batch,
) -> Result<Option<(Log, Vec<Clock>)>, EditError> {
    if (log.steps().iter()).any(|step| step.batch == Some(batch.id)) {
        return Ok(None);
    }
    let mut history = PhotoHistory::read_at(log, clocks, defaults, batch.wall);
    history.set_reader(tools, image);
    let mut edit = history.stepped().clone();
    as_read(&mut edit, tools, image);
    let (_, stepped) = style_photo(&mut history, &mut edit, style, (tools, image), batch)?;
    Ok(stepped.then(|| history.into_log()))
}

impl Engine {
    /// [`super::Command::ApplyStyle`] on the open photo: [`style_photo`] with a batch of its
    /// own log's and the wall clock now, a refusal its command's own error, then the tools'
    /// memories as a preset's leave them. Its steps are made here, not by
    /// [`Self::apply_edit`].
    pub(super) fn apply_style(&mut self, style: Style<'_>) -> Result<Effect, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(Effect::default());
        };
        let batch = Batch {
            id: open.history.new_batch(),
            wall: wall_clock(),
        };
        let (edit, styles) = (open.edit.clone(), open.history.styles().to_vec());
        let image = open.image.clone();
        let (wrote, _) = style_photo(
            &mut open.history,
            &mut open.edit,
            style,
            (&self.tools, &image),
            batch,
        )?;
        open.held.prune(&open.edit);
        let changed = open.edit != edit;
        open.unsaved |= changed
            || style.mode == StyleMode::Replace
            || open.history.styles() != styles.as_slice();
        // darktable's preset and reset run `gui_update` once their parameters are written.
        for wrote in &wrote {
            match wrote {
                Wrote::All => open.remember(&self.tools, Moment::Redrawn, None),
                Wrote::Tool(tool) => open.remember(&self.tools, Moment::Redrawn, Some(tool)),
                Wrote::Operation(_) => {}
            }
        }
        Ok(Effect {
            render: changed,
            save: true,
            cancelled: false,
            wrote: Vec::new(),
        })
    }

    /// [`super::Command::ClearStyle`]: each tool the style `name` from `file` set whose state
    /// is still the one it left goes back to its state before it, redrawn as `SetTool`'s is,
    /// and the style leaves the list the change leaves; [`SessionError::UnknownStyle`] where
    /// the list lacks it. Its steps are left to [`Self::apply_edit`].
    pub(super) fn clear_style(&mut self, name: &str, file: &str) -> Result<Effect, SessionError> {
        let Some(open) = &self.open else {
            return Ok(Effect::default());
        };
        // The item that is the style, by its file for a built-in (decision 109): it alone
        // leaves the list, never a person's style named as a built-in.
        let mut styles = open.history.styles().to_vec();
        let Some(at) = styles.iter().position(|style| style.is(name, file)) else {
            return Err(SessionError::UnknownStyle {
                name: name.to_owned(),
            });
        };
        let style = styles.remove(at);
        let mut back = Vec::new();
        let changed = self.change(|edit, tools, _, image| {
            let mut redrawn = Vec::new();
            for kept in &style.tools {
                // A tool this session lacks holds nothing it could put back.
                let Ok(found) = edit::tool(tools, &kept.tool) else {
                    continue;
                };
                let now = edit.tool(&kept.tool).cloned().unwrap_or_default();
                if now != kept.after {
                    continue;
                }
                match kept.before.is_empty() {
                    true => edit.tools.remove(&kept.tool),
                    false => edit.tools.insert(kept.tool.clone(), kept.before.clone()),
                };
                back.push(kept.tool.clone());
                redrawn.push(found);
            }
            edit.redraw(&redrawn, image);
            Ok(())
        })?;
        for tool in &back {
            self.remember_redrawn(Some(tool), changed);
        }
        if let Some(open) = &mut self.open {
            open.unsaved = true;
            open.history.leave_styles(styles);
        }
        Ok(Effect {
            render: changed,
            save: true,
            cancelled: false,
            wrote: back.into_iter().map(Wrote::Tool).collect(),
        })
    }

    /// The tools the style `name` from `file` set, by id, as a
    /// [`super::Command::ClearStyle`] may change them, none where the open photo's list lacks
    /// it.
    pub(super) fn style_tools(&self, name: &str, file: &str) -> Vec<Option<String>> {
        (self.open.iter())
            .flat_map(|open| open.history.styles())
            .find(|style| style.is(name, file))
            .into_iter()
            .flat_map(|style| &style.tools)
            .map(|tool| Some(tool.tool.clone()))
            .collect()
    }
}
