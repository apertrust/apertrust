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

//! The styles applied to the open photo (decision 106, `docs/history.md`, The styles
//! applied): `ApplyStyle` and `ClearStyle`, each one change of source Style that leaves the
//! list it changes to the history, which its last step holds ([`super::history`]).
//!
//! A style is applied as a `Step` of a `SetTool` for each of its tools would be, `Reset`
//! first for Replace, and put at the top of the list with each tool's state before and
//! after it; applied again, a tool that still held what the earlier application set keeps
//! that one's state before it. Clear puts back each tool whose state is still the one the
//! style left, compared as stored, and takes the style off the list.

use apertrust_tools::ToolValues;

use super::{Command, Effect, Engine, SessionError, StyleMode};
use crate::edit;
use crate::history::{AppliedStyle, StyleTool, Wrote, on_top};

impl Engine {
    /// [`Command::ApplyStyle`]: `Reset` first for [`StyleMode::Replace`], then a `SetTool`
    /// of each of `tools`, all or none, as a `Step` applies them, a refusal its command's own
    /// error; the style at the top of the list the change leaves. Its steps are left to
    /// [`Self::apply_edit`].
    pub(super) fn apply_style(
        &mut self,
        name: &str,
        file: &str,
        mode: StyleMode,
        tools: Vec<(String, ToolValues)>,
    ) -> Result<Effect, SessionError> {
        let Some(open) = &self.open else {
            return Ok(Effect::default());
        };
        let replace = mode == StyleMode::Replace;
        // What the tools held just before: a Replace's reset leaves each at its reset state,
        // and the list empty.
        let (start, list) = match replace {
            true => (edit::Edit::default(), Vec::new()),
            false => (open.edit.clone(), open.history.styles().to_vec()),
        };
        let mut commands: Vec<Command> = Vec::new();
        if replace {
            commands.push(Command::Reset);
        }
        // Each tool once, in the order the style first names it, its last values set.
        let mut order: Vec<&str> = Vec::new();
        for (tool, values) in &tools {
            if !order.contains(&tool.as_str()) {
                order.push(tool);
            }
            commands.push(Command::SetTool {
                tool: tool.clone(),
                values: values.clone(),
            });
        }
        let effect = self.apply_step(commands).map_err(|error| match error {
            SessionError::InStep { error, .. } => *error,
            error => error,
        })?;
        let Some(open) = &mut self.open else {
            return Ok(effect);
        };
        let state = |edit: &edit::Edit, tool: &str| edit.tool(tool).cloned().unwrap_or_default();
        let set = (order.iter())
            .map(|tool| StyleTool {
                tool: (*tool).to_owned(),
                before: state(&start, tool),
                after: state(&open.edit, tool),
            })
            .collect();
        let earlier = list.iter().find(|style| style.is(name, file));
        let style = AppliedStyle::applied(name, file, earlier, set);
        let styles = on_top(&list, style);
        open.unsaved |= styles.as_slice() != open.history.styles();
        open.history.leave_styles(styles);
        if replace {
            open.history.replacing();
            open.unsaved = true;
        }
        Ok(Effect {
            save: true,
            ..effect
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
