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

//! An edit (decision 28): the values of the tools it touches, and the operation
//! parameters no tool owns yet, only of operations an unedited photo runs. Nothing
//! derived is stored: [`instances`] turns an edit into what the pipeline plans, and every
//! place that builds instances goes through it (the session, export and the CLI, the
//! whole-pipeline test, the engine's tests of the default pipeline), so the viewport and
//! the export never differ and no optional operation slips into a default pipeline.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use apertrust_plugin::{ImageContext, Operation, Param, ParamError, ParamValues, Value};
use apertrust_tools::check::{self, ToolError};
use apertrust_tools::{
    Base, Control, ControlError, ControlKind, ControlValue, Controls, Coordinate, LabelValue, Mark,
    Picker, ProcessingTool, Range, ToolValues, Unit, View,
};
use thiserror::Error;

use crate::pipeline::Instance;

/// What the user changed, as plain data. It changes only through its checked methods (and
/// the sidecar's reader, which checks the same), so no owned parameter, nor one of an
/// operation an unedited photo does not run, gets in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Edit {
    /// Each tool the edit touches, by id: the controls set and their marks.
    pub(crate) tools: BTreeMap<String, ToolValues>,
    /// Parameters no tool owns, of operations an unedited photo runs, by operation id.
    pub(crate) params: BTreeMap<String, ParamValues>,
    /// The default tools not applied: the history's end lies before
    /// their default step, and darktable leaves them off until its workflow applies them
    /// (decision 78). Left out of the plan and reported off; derived from the log, never
    /// stored, and none once the edit's values move away from the end's.
    pub(crate) unapplied: BTreeSet<String>,
}

/// A change an edit refuses, or a binding that sets what it may not.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum EditError {
    #[error("no tool {0}")]
    UnknownTool(String),
    #[error("{tool} has no control {control}")]
    UnknownControl { tool: &'static str, control: String },
    #[error("{tool}: {source}")]
    Control {
        tool: &'static str,
        source: ControlError,
    },
    /// An option of the choice that this photo does not list (decision 30), as
    /// demosaic's other sensor's methods: only an edit from another photo holds one.
    #[error("{tool}.{control}: this photo does not list {option}")]
    NotListed {
        tool: &'static str,
        control: &'static str,
        option: &'static str,
    },
    #[error("{tool} has no preset row {preset}")]
    UnknownPreset { tool: &'static str, preset: String },
    #[error("{tool}.{preset} has no option {option}")]
    UnknownOption {
        tool: &'static str,
        preset: &'static str,
        option: String,
    },
    #[error("{tool}.{preset}: this photo cannot have {option}")]
    Unavailable {
        tool: &'static str,
        preset: &'static str,
        option: &'static str,
    },
    /// The tool's own fault, never the user's: its preset panicked for `option`, gave a
    /// value its control cannot take, or gave none for a control the row writes.
    #[error("{tool}.{preset}: {option} failed: {message}")]
    PresetFailed {
        tool: &'static str,
        preset: &'static str,
        option: &'static str,
        message: String,
    },
    /// A tool's colour picker's pick (decision 79) the tool got wrong: a write of a
    /// control the picker does not write, of a value its control cannot take, of a
    /// choice's option the photo does not list, or a pick that panicked. The tool's
    /// fault; nothing is written.
    #[error("{tool}'s picker {picker} failed: {message}")]
    PickFailed {
        tool: &'static str,
        picker: &'static str,
        message: String,
    },
    /// A coordinate's value, within its own range, that would put one of its view's
    /// controls outside its range.
    #[error("{tool}.{coordinate} refused: {source}")]
    Coordinate {
        tool: &'static str,
        coordinate: &'static str,
        source: ControlError,
    },
    /// The tool's own fault, never the user's: its view or its inverse panicked, gave no
    /// coordinates or not one for each, or gave no finite number for a control the view
    /// covers.
    #[error("{tool}.{view} failed: {message}")]
    ViewFailed {
        tool: &'static str,
        view: &'static str,
        message: String,
    },
    /// The tool's own fault, never the user's: its follow writes for `control` panicked,
    /// named a control it lacks or one the change set itself, or gave a value its control
    /// cannot take (decision 27).
    #[error("{tool}: following {control} failed: {message}")]
    FollowFailed {
        tool: &'static str,
        control: &'static str,
        message: String,
    },
    /// The tool's own fault, never the user's: a change moved the range of a control it
    /// wrote itself, the command's or a follow write's, leaving it outside (decision 30).
    #[error("{tool}: the change moved {control}'s range away from the value it wrote")]
    ClampFailed {
        tool: &'static str,
        control: &'static str,
    },
    #[error("no operation {0}")]
    UnknownOperation(String),
    #[error("{operation}: {source}")]
    Param {
        operation: &'static str,
        source: ParamError,
    },
    #[error("{operation}.{param} belongs to the tool {tool}, which sets it")]
    Owned {
        operation: &'static str,
        param: &'static str,
        tool: &'static str,
    },
    /// A parameter its tool owns with no control, holding it at the operation's value
    /// for the photo (`ProcessingTool::fixed`), as Raw black/white point holds the crop.
    #[error("{operation}.{param} is fixed by the tool {tool}")]
    Fixed {
        operation: &'static str,
        param: &'static str,
        tool: &'static str,
    },
    #[error("{operation} runs only while a tool sets it, so only tools set its parameters")]
    Optional { operation: &'static str },
    #[error("the tool {tool}: {source}")]
    Binding {
        tool: &'static str,
        source: ToolError,
    },
}

impl Edit {
    /// Nothing set and nothing marked: the photo as Apertrust first shows it.
    pub fn is_empty(&self) -> bool {
        self.tools.values().all(ToolValues::is_empty)
            && self.params.values().all(|p| p.iter().next().is_none())
    }

    /// Applies every tool the history's end leaves not applied, as a change made there
    /// does (decision 78): an override `apertrust-cli` applies to a saved edit, as
    /// `export --set`, is such a change.
    pub fn apply_all(&mut self) {
        self.unapplied.clear();
    }

    /// The values of the tool called `id`, if the edit touches it.
    pub fn tool(&self, id: &str) -> Option<&ToolValues> {
        self.tools.get(id)
    }

    /// Every tool the edit touches, by id.
    pub fn tools(&self) -> impl Iterator<Item = (&str, &ToolValues)> {
        self.tools.iter().map(|(id, values)| (id.as_str(), values))
    }

    /// The parameters the edit sets on the operation called `id`.
    pub fn params(&self, id: &str) -> Option<&ParamValues> {
        self.params.get(id)
    }

    /// Sets a control, refusing a value of another kind, a number outside the range the
    /// tool's values give it now (`ProcessingTool::range`, decision 30), the definition's
    /// where the tool's defaults for the photo panic, and a choice's option the photo,
    /// `image`, does not list (decision 30). Setting a control keeps it, so Auto and Match
    /// leave it alone, and turns a switched tool on, whatever the value (decision 32).
    pub fn set_control(
        &mut self,
        tool: &dyn ProcessingTool,
        control: &str,
        value: ControlValue,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let (id, control) = find_control(tool, control)?;
        control
            .check(&value)
            .map_err(|source| EditError::Control { tool: id, source })?;
        // A tool's code may panic, as its binding is guarded against: the range is then
        // the definition's, which the check above holds the value to.
        let range = || {
            let every = catch_unwind(AssertUnwindSafe(|| self.every(tool, image))).ok()?;
            range_now(tool, control.id, &every)
        };
        if let ControlValue::Number(number) = value
            && let Some(range) = range()
            && !range.contains(number)
        {
            let source = ControlError::OutOfRange {
                control: control.id,
                value: f64::from(number),
                min: f64::from(range.min),
                max: f64::from(range.max),
            };
            return Err(EditError::Control { tool: id, source });
        }
        if let Some(option) = unlisted(tool, control, &value, image) {
            return Err(EditError::NotListed {
                tool: id,
                control: control.id,
                option,
            });
        }
        let values = self.tools.entry(id.to_owned()).or_default();
        values.set(control.id, value);
        values.set_mark(control.id, Some(Mark::Kept));
        self.switch_on(tool);
        Ok(())
    }

    /// Turns a switched tool on (decision 32), as every write of one of its controls
    /// does; a tool that is not switched is always on, and stays as it is.
    fn switch_on(&mut self, tool: &dyn ProcessingTool) {
        let definition = tool.definition();
        if definition.switched {
            let values = self.tools.entry(definition.tool.id.to_owned()).or_default();
            values.set_enabled(true);
        }
    }

    /// Whether the tool renders: a switched tool while it is on (decision 32), every
    /// other but while the history's end lies before its default step (decision 78).
    pub fn is_on(&self, tool: &dyn ProcessingTool) -> bool {
        let definition = tool.definition();
        if definition.switched {
            (self.tools.get(definition.tool.id)).is_some_and(ToolValues::enabled)
        } else {
            !self.unapplied.contains(definition.tool.id)
        }
    }

    /// Takes a control back to its default for the photo, no longer kept. A switched
    /// tool on stays on (decision 32).
    pub fn reset_control(
        &mut self,
        tool: &dyn ProcessingTool,
        control: &str,
    ) -> Result<(), EditError> {
        let (id, control) = find_control(tool, control)?;
        if let Some(values) = self.tools.get_mut(id) {
            values.remove(control.id);
            values.set_mark(control.id, None);
        }
        self.tidy(id);
        Ok(())
    }

    /// Applies `option` of the tool's preset row `preset` on `image` (decision 2): each
    /// control the row writes takes the option's value for the photo and is kept, since
    /// choosing an option is the user's own action, and the tool's other controls stay
    /// as they are. An option the photo cannot have, or the row lacks, is refused, and so
    /// is one whose tool fails, giving values its controls refuse, a choice's option the
    /// photo does not list (decision 30), or panicking: every value is checked before any
    /// is set, so a refused option changes nothing. Applied, it turns a switched tool on
    /// (decision 32).
    pub fn apply_preset(
        &mut self,
        tool: &dyn ProcessingTool,
        preset: &str,
        option: &str,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let definition = tool.definition();
        let id = definition.tool.id;
        let row = definition
            .preset(preset)
            .ok_or_else(|| EditError::UnknownPreset {
                tool: id,
                preset: preset.to_owned(),
            })?;
        let option = row
            .options
            .iter()
            .copied()
            .find(|candidate| *candidate == option)
            .ok_or_else(|| EditError::UnknownOption {
                tool: id,
                preset: row.id,
                option: option.to_owned(),
            })?;
        let failed = |message: String| EditError::PresetFailed {
            tool: id,
            preset: row.id,
            option,
            message,
        };
        // A tool's code may panic, as its binding is guarded against: the option is
        // refused, never the photo.
        let given = catch_unwind(AssertUnwindSafe(|| tool.preset(row.id, option, image)))
            .map_err(|panic| failed(panic_message(panic.as_ref())))?
            .ok_or(EditError::Unavailable {
                tool: id,
                preset: row.id,
                option,
            })?;
        let mut checked = Vec::with_capacity(row.controls.len());
        for &control in row.controls {
            let value = given
                .get(control)
                .ok_or_else(|| failed(format!("no value for {control}")))?;
            let (_, found) = find_control(tool, control)?;
            found
                .check(value)
                .map_err(|error| failed(error.to_string()))?;
            if let Some(unlisted) = unlisted(tool, found, value, image) {
                return Err(failed(format!(
                    "{control} {unlisted}, which this photo does not list"
                )));
            }
            checked.push((control, value.clone()));
        }
        let values = self.tools.entry(id.to_owned()).or_default();
        for (control, value) in checked {
            values.set(control, value);
            values.set_mark(control, Some(Mark::Kept));
        }
        self.switch_on(tool);
        Ok(())
    }

    /// Writes what the tool's colour picker `picker` picked (decision 79; `docs/pickers.md`,
    /// What a pick writes): each control of `writes` takes its value and is kept, with no
    /// follow writes, since a pick writes every control darktable's `color_picker_apply`
    /// keeps consistent. A value is stored as the tool's rule gave it, with one bound:
    /// a number past the range the tool's values give its control now, the
    /// parameter's hard limits, is held at the nearer limit, and a hue, a control in
    /// degrees over a whole turn, wrapped onto its turn instead, the same angle. A control
    /// the picker does not write, a value its control cannot take or a choice's option
    /// the photo does not list (decision 30) is the tool's failure, and nothing is
    /// written. It turns a switched tool on (decision 32).
    pub fn pick(
        &mut self,
        tool: &dyn ProcessingTool,
        picker: &Picker,
        writes: &Controls,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let id = tool.definition().tool.id;
        let failed = |message: String| EditError::PickFailed {
            tool: id,
            picker: picker.id,
            message,
        };
        let every = catch_unwind(AssertUnwindSafe(|| self.every(tool, image))).ok();
        let mut checked = Vec::new();
        for (control, value) in writes.iter() {
            if !picker.writes.contains(&control) {
                return Err(failed(format!("it writes {control}, which it may not")));
            }
            let (_, found) = find_control(tool, control)?;
            let value = match (value, found.kind) {
                (&ControlValue::Number(number), ControlKind::Number { min, max, .. }) => {
                    let (min, max) = every
                        .as_ref()
                        .and_then(|every| range_now(tool, control, every))
                        .map_or((min, max), |range| (range.min, range.max));
                    let turn = found.unit == Some(Unit::Degrees) && max - min == 360.0;
                    ControlValue::Number(if turn {
                        min + (number - min).rem_euclid(360.0)
                    } else {
                        number.clamp(min, max)
                    })
                }
                (value, _) => value.clone(),
            };
            found
                .check(&value)
                .map_err(|error| failed(error.to_string()))?;
            if let Some(unlisted) = unlisted(tool, found, &value, image) {
                return Err(failed(format!(
                    "{control} {unlisted}, which this photo does not list"
                )));
            }
            checked.push((control, value));
        }
        let values = self.tools.entry(id.to_owned()).or_default();
        for (control, value) in checked {
            values.set(control, value);
            values.set_mark(control, Some(Mark::Kept));
        }
        self.switch_on(tool);
        Ok(())
    }

    /// Turns a switched tool on (decision 32), as its picker turning on does, darktable
    /// turning the tool on with its picker; a tool that is not switched is always on.
    pub fn turn_on(&mut self, tool: &dyn ProcessingTool) {
        self.switch_on(tool);
    }

    /// Sets `coordinate`, a coordinate of one of the tool's lossless views, to `value`
    /// on `image` (decision 2), holding the view's other coordinates: those `held` keeps
    /// while the view's controls hold what it last wrote, else the tool's view of the
    /// edit's values. The view's controls take [`ProcessingTool::unview`]'s values, each
    /// kept, and `held` keeps the coordinates. A value outside the coordinate's range, or
    /// one that would put a control outside its own, is refused, and so is one whose tool
    /// fails: every value is checked before any is set, so a refused one changes nothing.
    /// Set to the value it has, it writes no value and keeps the controls, as setting a
    /// control keeps it, so the stored values move only when the view is edited. Either way
    /// it turns a switched tool on, as setting a control does (decision 32).
    pub fn set_coordinate(
        &mut self,
        held: &mut Held,
        tool: &dyn ProcessingTool,
        coordinate: &str,
        value: ControlValue,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let (id, view, at, found) = find_coordinate(tool, coordinate)?;
        let value = found
            .check(&value)
            .map_err(|source| EditError::Control { tool: id, source })?;
        let mut coordinates = self.coordinates(held, tool, view, image)?;
        if coordinates[at] == value {
            self.keep_coordinate(tool, found.id, true)?;
            self.switch_on(tool);
            return Ok(());
        }
        coordinates[at] = value;
        self.write_view(held, tool, view, coordinates, found.id, image)
    }

    /// Takes `coordinate` back to its value at the tool's defaults for the photo,
    /// holding the view's other coordinates, as [`Self::set_coordinate`] does. Once every
    /// coordinate of the view is at its defaults' value, the view's controls are reset
    /// exactly, values and marks, so an untouched tool stays untouched.
    pub fn reset_coordinate(
        &mut self,
        held: &mut Held,
        tool: &dyn ProcessingTool,
        coordinate: &str,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let (id, view, at, found) = find_coordinate(tool, coordinate)?;
        let defaults = default_coordinates(tool, view, image)?;
        let mut coordinates = self.coordinates(held, tool, view, image)?;
        let unchanged = coordinates[at] == defaults[at];
        coordinates[at] = defaults[at];
        if coordinates == defaults {
            if let Some(values) = self.tools.get_mut(id) {
                for control in view.controls {
                    values.remove(control);
                    values.set_mark(control, None);
                }
            }
            held.0.remove(&(id, view.id));
            self.tidy(id);
            return Ok(());
        }
        if unchanged {
            return Ok(());
        }
        self.write_view(held, tool, view, coordinates, found.id, image)
    }

    /// Keeps or releases the controls of the view `coordinate` belongs to: a view's marks
    /// are its controls'.
    pub fn keep_coordinate(
        &mut self,
        tool: &dyn ProcessingTool,
        coordinate: &str,
        kept: bool,
    ) -> Result<(), EditError> {
        let (_, view, _, _) = find_coordinate(tool, coordinate)?;
        for control in view.controls {
            self.keep(tool, control, kept)?;
        }
        Ok(())
    }

    /// The coordinates of the tool's view `view` on `image`: those `held` keeps while
    /// the view's controls hold what it last wrote, else [`ProcessingTool::view`] of every
    /// control, the edit's value over its default for the photo.
    pub fn coordinates(
        &self,
        held: &Held,
        tool: &dyn ProcessingTool,
        view: &View,
        image: &ImageContext,
    ) -> Result<Vec<f32>, EditError> {
        let definition = tool.definition();
        if let Some(coordinates) = held.get(self, definition.tool.id, view) {
            return Ok(coordinates.to_vec());
        }
        viewed(tool, view, image, || self.every(tool, image))
    }

    /// Every control of the tool on `image`: the edit's value over its default for the
    /// photo.
    pub(crate) fn every(&self, tool: &dyn ProcessingTool, image: &ImageContext) -> Controls {
        let definition = tool.definition();
        let mut values = tool.defaults(image);
        if let Some(set) = self.tools.get(definition.tool.id) {
            for control in definition.controls {
                if let Some(value) = set.get(control.id) {
                    values.set(control.id, value.clone());
                }
            }
        }
        values
    }

    /// The tool's follow writes (decision 27; `docs/tools/README.md`, rule 13) for a
    /// change from `before` that set `set`, the controls a command wrote ([`written`]):
    /// for each of them whose value the change moved, in the definition's order, what
    /// [`ProcessingTool::follow`] gives on `image`, a later control's write over an earlier
    /// one's, each written and kept, as the command's own are, but a write of the control's
    /// default for the photo resets it, value and mark, as a reset does. A write that
    /// leaves a control as it is changes nothing. A write to a control the tool lacks or
    /// the change set, of a value its control cannot take or a choice's option the photo
    /// does not list (decision 30), or a follow that panics is the tool's failure,
    /// [`EditError::FollowFailed`], and the edit is `before` again: a refused change
    /// changes nothing.
    pub fn follow(
        &mut self,
        before: &Edit,
        tool: &dyn ProcessingTool,
        set: &[&str],
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let result = self.write_follows(before, tool, set, image);
        if result.is_err() {
            *self = before.clone();
        }
        result
    }

    /// [`Self::follow`]'s writes, every one checked before any is made.
    fn write_follows(
        &mut self,
        before: &Edit,
        tool: &dyn ProcessingTool,
        set: &[&str],
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let definition = tool.definition();
        let id = definition.tool.id;
        let failed = |control: &'static str, message: String| EditError::FollowFailed {
            tool: id,
            control,
            message,
        };
        let by = definition
            .controls
            .iter()
            .find(|control| set.contains(&control.id))
            .map_or("defaults", |control| control.id);
        let (now, then) = catch_unwind(AssertUnwindSafe(|| {
            (self.every(tool, image), before.every(tool, image))
        }))
        .map_err(|panic| failed(by, panic_message(panic.as_ref())))?;
        // Each write with the control whose follow wrote it.
        let mut writes: Vec<(&'static str, &'static Control, ControlValue)> = Vec::new();
        let moved = definition.controls.iter().filter(|control| {
            set.contains(&control.id) && now.get(control.id) != then.get(control.id)
        });
        for control in moved {
            let failed = |message: String| failed(control.id, message);
            // A tool's code may panic, as its binding is guarded against: the change is
            // refused, never the photo.
            let given = catch_unwind(AssertUnwindSafe(|| tool.follow(control.id, &now, image)))
                .map_err(|panic| failed(panic_message(panic.as_ref())))?;
            for (written, value) in given.iter() {
                if set.contains(&written) {
                    return Err(failed(format!("rewrites {written}, which the change set")));
                }
                let found = definition
                    .control(written)
                    .ok_or_else(|| failed(format!("writes {written}, which it lacks")))?;
                found
                    .check(value)
                    .map_err(|error| failed(error.to_string()))?;
                writes.retain(|(_, other, _)| other.id != written);
                writes.push((control.id, found, value.clone()));
            }
        }
        writes.retain(|(_, control, value)| now.get(control.id) != Some(value));
        // Of the writes that change a control, one of an option the photo does not list
        // (decision 30); a write a later one replaces, or one of the value a control holds
        // already, from another photo's edit, changes nothing and fails nothing.
        for (by, control, value) in &writes {
            if let Some(unlisted) = unlisted(tool, control, value, image) {
                let written = control.id;
                return Err(failed(
                    by,
                    format!("writes {written} {unlisted}, which this photo does not list"),
                ));
            }
        }
        if writes.is_empty() {
            return Ok(());
        }
        // A write of a control's default for the photo resets it, value and mark, as a
        // command's own reset does, so a change and its reset leave the edit untouched.
        let defaults = tool.defaults(image);
        let values = self.tools.entry(id.to_owned()).or_default();
        for (_, control, value) in writes {
            let control = control.id;
            if defaults.get(control) == Some(&value) {
                values.remove(control);
                values.set_mark(control, None);
            } else {
                values.set(control, value);
                values.set_mark(control, Some(Mark::Kept));
            }
        }
        self.tidy(id);
        Ok(())
    }

    /// After a change from `before` on `image`, its own writes and its follow writes made,
    /// brings back inside its range every number a moved range left outside (decision 30;
    /// `docs/engine.md`, A range that follows other controls): for each of `tools`,
    /// [`check::clamp`]'s writes, each kept, as follow writes are, but a write of the
    /// control's default for the photo resets it, value and mark. A clamp of a control the
    /// change wrote itself, the command's or a follow write's, is the tool's failure,
    /// [`EditError::ClampFailed`], and the edit is `before` again: a refused change changes
    /// nothing. A tool whose defaults or ranges panic clamps nothing, its ranges being
    /// the definition's, which every value holds.
    pub fn clamp(
        &mut self,
        before: &Edit,
        tools: &[&dyn ProcessingTool],
        image: &ImageContext,
    ) -> Result<(), EditError> {
        for &tool in tools {
            let id = tool.definition().tool.id;
            // A tool's code may panic, as its binding is guarded against: that tool
            // clamps nothing, and the change goes on.
            let Ok((now, then, writes, defaults)) = catch_unwind(AssertUnwindSafe(|| {
                let now = self.every(tool, image);
                let writes = check::clamp(tool, &now);
                (now, before.every(tool, image), writes, tool.defaults(image))
            })) else {
                continue;
            };
            if let Some((control, _)) = writes
                .iter()
                .find(|(control, _)| now.get(control) != then.get(control))
            {
                let error = EditError::ClampFailed { tool: id, control };
                *self = before.clone();
                return Err(error);
            }
            for (control, value) in writes.iter() {
                let values = self.tools.entry(id.to_owned()).or_default();
                if defaults.get(control) == Some(value) {
                    values.remove(control);
                    values.set_mark(control, None);
                } else {
                    values.set(control, value.clone());
                    values.set_mark(control, Some(Mark::Kept));
                }
            }
            self.tidy(id);
        }
        Ok(())
    }

    /// Brings a saved edit's numbers inside the ranges its tools' values give on `image`
    /// (decision 30), as a number outside its control's range is clamped as it is read
    /// (decision 21): each written at its range's nearer end, its mark as it was. Returns a
    /// warning for each, empty when every number holds; only a hand-written file holds one.
    /// A tool whose defaults or ranges panic is left as it is.
    pub fn clamp_read(
        &mut self,
        tools: &[&dyn ProcessingTool],
        image: &ImageContext,
    ) -> Vec<String> {
        let mut warnings = Vec::new();
        for &tool in tools {
            let id = tool.definition().tool.id;
            if !self.tools.contains_key(id) {
                continue;
            }
            // A tool's code may panic, as its binding is guarded against: that tool is
            // left as it is, and the photo opens.
            let Ok((now, writes, defaults)) = catch_unwind(AssertUnwindSafe(|| {
                let now = self.every(tool, image);
                let writes = check::clamp(tool, &now);
                (now, writes, tool.defaults(image))
            })) else {
                continue;
            };
            let values = self.tools.entry(id.to_owned()).or_default();
            for (control, value) in writes.iter() {
                let (Some(ControlValue::Number(was)), ControlValue::Number(is)) =
                    (now.get(control), value)
                else {
                    continue;
                };
                warnings.push(format!(
                    "{id}.{control} is {was}, outside the range its tool's values give; {is} instead"
                ));
                if defaults.get(control) == Some(value) {
                    values.remove(control);
                } else {
                    values.set(control, value.clone());
                }
            }
            self.tidy(id);
        }
        warnings
    }

    /// Writes `coordinates` into the view's controls through [`ProcessingTool::unview`],
    /// each kept, once every value is checked, and holds them; `coordinate` is the one set.
    fn write_view(
        &mut self,
        held: &mut Held,
        tool: &dyn ProcessingTool,
        view: &'static View,
        coordinates: Vec<f32>,
        coordinate: &'static str,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let id = tool.definition().tool.id;
        let failed = |message: String| EditError::ViewFailed {
            tool: id,
            view: view.id,
            message,
        };
        // A tool's code may panic, as its binding is guarded against: the value is
        // refused, never the photo.
        let given = catch_unwind(AssertUnwindSafe(|| {
            tool.unview(view.id, &coordinates, image)
        }))
        .map_err(|panic| failed(panic_message(panic.as_ref())))?
        .ok_or_else(|| failed("no values".to_owned()))?;
        let mut wrote = Vec::with_capacity(view.controls.len());
        for &control in view.controls {
            let value = match given.get(control) {
                // Infinite or NaN is the tool's fault, never the coordinate's.
                Some(ControlValue::Number(v)) if v.is_finite() => ControlValue::Number(*v),
                _ => return Err(failed(format!("no finite number for {control}"))),
            };
            let (_, found) = find_control(tool, control)?;
            found
                .check(&value)
                .map_err(|source| EditError::Coordinate {
                    tool: id,
                    coordinate,
                    source,
                })?;
            wrote.push(value);
        }
        let values = self.tools.entry(id.to_owned()).or_default();
        for (control, value) in view.controls.iter().zip(&wrote) {
            values.set(control, value.clone());
            values.set_mark(control, Some(Mark::Kept));
        }
        self.switch_on(tool);
        held.0.insert(
            (id, view.id),
            Hold {
                controls: view.controls,
                wrote,
                coordinates,
            },
        );
        Ok(())
    }

    /// Takes every control of the tool back to its default, with no marks, and turns a
    /// switched tool off (decision 32).
    pub fn reset_tool(&mut self, tool: &dyn ProcessingTool) {
        self.tools.remove(tool.definition().tool.id);
    }

    /// Keeps a control or releases it. Keeping an untouched control holds it at its
    /// default; releasing a set one leaves its value, free for Auto and Match. It never
    /// turns a switched tool on, nor off (decision 32).
    pub fn keep(
        &mut self,
        tool: &dyn ProcessingTool,
        control: &str,
        kept: bool,
    ) -> Result<(), EditError> {
        let (id, control) = find_control(tool, control)?;
        let values = self.tools.entry(id.to_owned()).or_default();
        values.set_mark(control.id, kept.then_some(Mark::Kept));
        self.tidy(id);
        Ok(())
    }

    /// Sets a parameter no tool owns, of an operation an unedited photo runs, refusing
    /// a value out of its range ([`unowned`]).
    pub fn set_param(
        &mut self,
        operations: &[&dyn Operation],
        tools: &[&dyn ProcessingTool],
        operation: &str,
        param: &str,
        value: Value,
    ) -> Result<(), EditError> {
        let (operation, param) = unowned(operations, tools, operation, param)?;
        param
            .check(value)
            .map_err(|source| EditError::Param { operation, source })?;
        self.params
            .entry(operation.to_owned())
            .or_default()
            .set(param.name, value);
        Ok(())
    }

    /// Takes a parameter [`Self::set_param`] could set back to its default.
    pub fn reset_param(
        &mut self,
        operations: &[&dyn Operation],
        tools: &[&dyn ProcessingTool],
        operation: &str,
        param: &str,
    ) -> Result<(), EditError> {
        let (operation, param) = unowned(operations, tools, operation, param)?;
        if let Some(params) = self.params.get_mut(operation) {
            params.remove(param.name);
            if params.iter().next().is_none() {
                self.params.remove(operation);
            }
        }
        Ok(())
    }

    /// Forgets a tool the edit no longer touches.
    fn tidy(&mut self, tool: &str) {
        if self.tools.get(tool).is_some_and(ToolValues::is_empty) {
            self.tools.remove(tool);
        }
    }
}

/// The tool called `id`.
pub fn tool<'m>(
    tools: &[&'m dyn ProcessingTool],
    id: &str,
) -> Result<&'m dyn ProcessingTool, EditError> {
    tools
        .iter()
        .copied()
        .find(|tool| tool.definition().tool.id == id)
        .ok_or_else(|| EditError::UnknownTool(id.to_owned()))
}

/// The controls a command on `control`, one of the tool's controls or a coordinate of one
/// of its views, writes: the control, or the view's controls; none for an id the tool
/// lacks, which the command refuses.
pub fn written(tool: &dyn ProcessingTool, control: &str) -> &'static [&'static str] {
    let definition = tool.definition();
    match (definition.coordinate(control), definition.control(control)) {
        (Some((view, _)), _) => view.controls,
        (None, Some(found)) => std::slice::from_ref(&found.id),
        (None, None) => &[],
    }
}

/// The options the choice `control` lists on `image` (decision 30):
/// [`ProcessingTool::options`], or every option where the tool's code panics or gives a
/// list that does not hold together ([`check::listed`]), with why. A control that is no
/// choice lists none. The edit refuses any other option, and the session reports the list,
/// warning once of a failure, so both read it alike.
pub fn listed(
    tool: &dyn ProcessingTool,
    control: &Control,
    image: &ImageContext,
) -> (Vec<&'static str>, Option<String>) {
    let ControlKind::Choice { options, .. } = control.kind else {
        return (Vec::new(), None);
    };
    // A tool's code may panic, as its binding is guarded against: the photo lists every
    // option, never none.
    let given = catch_unwind(AssertUnwindSafe(|| {
        let listed = tool.options(control.id, image);
        let default = tool.defaults(image).get(control.id).cloned();
        (listed, default.unwrap_or_else(|| control.default()))
    }));
    match given {
        Ok((listed, default)) => match check::listed(control, &listed, &default) {
            None => (listed, None),
            problem => (options.to_vec(), problem),
        },
        Err(panic) => (options.to_vec(), Some(panic_message(panic.as_ref()))),
    }
}

/// What the photo shows of one control's row (decision 88): whether it shows at all
/// ([`ProcessingTool::visible`]) and a value of the photo's after its label
/// ([`ProcessingTool::label_value`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoRow {
    pub visible: bool,
    pub label_value: Option<LabelValue>,
}

impl Default for PhotoRow {
    /// Shown, with no value: what a tool that overrides neither gives.
    fn default() -> Self {
        Self {
            visible: true,
            label_value: None,
        }
    }
}

/// What `image` shows of each of the tool's controls' rows, in the definition's order
/// (decision 88): every row shown with no value where the tool's code panics, with why,
/// and a value that is not finite left out. Both read the photo alone, so the session
/// asks as the photo opens and again where its decode's context replaces the metadata's.
pub fn photo_rows(
    tool: &dyn ProcessingTool,
    image: &ImageContext,
) -> (Vec<PhotoRow>, Option<String>) {
    let controls = tool.definition().controls;
    // A tool's code may panic, as its binding is guarded against: every row shows, as
    // before the redesign.
    let given = catch_unwind(AssertUnwindSafe(|| {
        controls
            .iter()
            .map(|control| PhotoRow {
                visible: tool.visible(control.id, image),
                label_value: tool
                    .label_value(control.id, image)
                    .filter(|label| label.value.is_finite()),
            })
            .collect()
    }));
    match given {
        Ok(rows) => (rows, None),
        Err(panic) => (
            vec![PhotoRow::default(); controls.len()],
            Some(panic_message(panic.as_ref())),
        ),
    }
}

/// The range of the number control `control` while the tool's controls hold `values`
/// (decision 30), with why it is the definition's, where the tool's code panics or
/// gives one that does not hold ([`check::range`]). `None` for a control that is no
/// number. The edit refuses a value outside it and the session reports it, warning once of
/// a failure, so both read it alike.
pub fn range(
    tool: &dyn ProcessingTool,
    control: &str,
    values: &Controls,
) -> Option<(Range, Option<String>)> {
    let definition = tool.definition().range(control)?;
    // A tool's code may panic, as its binding is guarded against: the range is the
    // definition's, which every value it may hold lies in.
    Some(
        match catch_unwind(AssertUnwindSafe(|| check::range(tool, control, values))) {
            Ok(Some(Ok(range))) => (range, None),
            Ok(_) => (definition, Some("its range does not hold".to_owned())),
            Err(panic) => (definition, Some(panic_message(panic.as_ref()))),
        },
    )
}

/// [`range`]'s range alone.
fn range_now(tool: &dyn ProcessingTool, control: &str, values: &Controls) -> Option<Range> {
    range(tool, control, values).map(|(range, _)| range)
}

/// The option `value` holds, where `control` is a choice that `image` does not list it
/// for.
fn unlisted(
    tool: &dyn ProcessingTool,
    control: &Control,
    value: &ControlValue,
    image: &ImageContext,
) -> Option<&'static str> {
    let (ControlKind::Choice { options, .. }, ControlValue::Choice(i)) = (control.kind, value)
    else {
        return None;
    };
    let option = options.get(*i as usize).copied()?;
    let (listed, _) = listed(tool, control, image);
    (!listed.contains(&option)).then_some(option)
}

fn find_control(
    tool: &dyn ProcessingTool,
    control: &str,
) -> Result<(&'static str, &'static Control), EditError> {
    let definition = tool.definition();
    let found = definition
        .control(control)
        .ok_or_else(|| EditError::UnknownControl {
            tool: definition.tool.id,
            control: control.to_owned(),
        })?;
    Ok((definition.tool.id, found))
}

/// The coordinate called `coordinate` of one of the tool's views: the tool's id, the
/// view, the coordinate's place in it, and the coordinate.
fn find_coordinate(
    tool: &dyn ProcessingTool,
    coordinate: &str,
) -> Result<(&'static str, &'static View, usize, &'static Coordinate), EditError> {
    let definition = tool.definition();
    let (view, at) =
        definition
            .coordinate(coordinate)
            .ok_or_else(|| EditError::UnknownControl {
                tool: definition.tool.id,
                control: coordinate.to_owned(),
            })?;
    Ok((definition.tool.id, view, at, &view.coordinates[at]))
}

/// The coordinates of the tool's view `view` at its defaults for `image`: what a
/// coordinate's reset takes it back to.
pub fn default_coordinates(
    tool: &dyn ProcessingTool,
    view: &View,
    image: &ImageContext,
) -> Result<Vec<f32>, EditError> {
    viewed(tool, view, image, || tool.defaults(image))
}

/// [`ProcessingTool::view`] of the values `values` gives, one coordinate for each of the
/// view's; the tool's failure, a panic included, an error.
fn viewed(
    tool: &dyn ProcessingTool,
    view: &View,
    image: &ImageContext,
    values: impl FnOnce() -> Controls,
) -> Result<Vec<f32>, EditError> {
    let failed = |message: String| EditError::ViewFailed {
        tool: tool.definition().tool.id,
        view: view.id,
        message,
    };
    let coordinates = catch_unwind(AssertUnwindSafe(|| tool.view(view.id, &values(), image)))
        .map_err(|panic| failed(panic_message(panic.as_ref())))?
        .ok_or_else(|| failed("no coordinates".to_owned()))?;
    if coordinates.len() != view.coordinates.len() {
        let (given, wanted) = (coordinates.len(), view.coordinates.len());
        return Err(failed(format!("{given} coordinates for {wanted}")));
    }
    Ok(coordinates)
}

/// The coordinates a session holds through gestures on lossless views, by tool and view
/// (decision 2; `docs/engine.md`, Preset rows and lossless views). A view's are taken when
/// a gesture on one of its coordinates begins, from the tool's view of the stored
/// values, and last while the view's controls hold exactly what was last written through
/// it, so dragging one coordinate leaves the others exactly where they were however many
/// steps the drag takes. Any change that leaves those controls holding other values drops
/// them, and the next edit starts from the stored values again; a view reads only its own
/// controls ([`ProcessingTool::view`]), so no other change can move them. Nothing of it is
/// saved.
#[derive(Debug, Clone, Default)]
pub struct Held(BTreeMap<(&'static str, &'static str), Hold>);

#[derive(Debug, Clone)]
struct Hold {
    /// The view's controls, and the values last written into them, in its order.
    controls: &'static [&'static str],
    wrote: Vec<ControlValue>,
    coordinates: Vec<f32>,
}

impl Held {
    /// The tool's view's held coordinates, while its controls hold what was written.
    fn get(&self, edit: &Edit, tool: &'static str, view: &View) -> Option<&[f32]> {
        let hold = self.0.get(&(tool, view.id))?;
        hold.holds(edit, tool).then_some(&hold.coordinates[..])
    }

    /// Drops the coordinates of every view whose controls no longer hold what was last
    /// written through it: after any change but through the view, its gesture is over.
    pub fn prune(&mut self, edit: &Edit) {
        self.0.retain(|&(tool, _), hold| hold.holds(edit, tool));
    }
}

impl Hold {
    fn holds(&self, edit: &Edit, tool: &str) -> bool {
        let values = edit.tools.get(tool);
        self.controls
            .iter()
            .zip(&self.wrote)
            .all(|(control, wrote)| values.and_then(|v| v.get(control)) == Some(wrote))
    }
}

/// `operation.param`, if an edit may set it at operation level: a parameter no tool
/// owns, of an operation an unedited photo runs. Any other is refused, so a saved value
/// can never wait unseen on an optional operation and appear once a tool adds it.
pub fn unowned(
    operations: &[&dyn Operation],
    tools: &[&dyn ProcessingTool],
    operation: &str,
    param: &str,
) -> Result<(&'static str, &'static Param), EditError> {
    let found = operations
        .iter()
        .find(|candidate| candidate.manifest().id == operation)
        .ok_or_else(|| EditError::UnknownOperation(operation.to_owned()))?;
    let manifest = found.manifest();
    let param = found
        .schema()
        .param(param)
        .ok_or_else(|| EditError::Param {
            operation: manifest.id,
            source: ParamError::Unknown(param.to_owned()),
        })?;
    if !manifest.default_enabled {
        return Err(EditError::Optional {
            operation: manifest.id,
        });
    }
    if let Some(owner) = check::owner(tools, manifest.id, param.name) {
        let (operation, param, tool) = (manifest.id, param.name, owner.definition().tool.id);
        let fixed = owner
            .fixed()
            .iter()
            .any(|fixed| fixed.operation == operation && fixed.param == param);
        return Err(if fixed {
            EditError::Fixed {
                operation,
                param,
                tool,
            }
        } else {
            EditError::Owned {
                operation,
                param,
                tool,
            }
        });
    }
    Ok((manifest.id, param))
}

/// What a panic carried, as text.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

/// The instances an edit renders, and the tools left out of them.
pub struct Bound<'a> {
    pub instances: Vec<Instance<'a>>,
    /// Each tool whose binding set a parameter it does not own or a value its schema
    /// refuses, or panicked: none of its settings are in `instances`.
    pub failed: Vec<(&'static str, ToolError)>,
}

impl<'a> Bound<'a> {
    /// The instances, or the first tool that failed: for an export, which renders the
    /// edit as it is or not at all.
    pub fn all(self) -> Result<Vec<Instance<'a>>, EditError> {
        match self.failed.into_iter().next() {
            Some((tool, source)) => Err(EditError::Binding { tool, source }),
            None => Ok(self.instances),
        }
    }
}

/// One tool's settings for `values`, each checked: what it may set, or why not. A
/// switched tool off is never bound; one on is bound with its values off its defaults,
/// possibly none (decision 32).
fn bind(
    tool: &dyn ProcessingTool,
    edit: &Edit,
    operations: &[&dyn Operation],
    image: &ImageContext,
    base: &mut Option<Base>,
) -> Result<Vec<apertrust_tools::Setting>, ToolError> {
    let Some(values) = edit.tools.get(tool.definition().tool.id) else {
        return Ok(Vec::new());
    };
    let switched = tool.definition().switched;
    if switched && !values.enabled() {
        return Ok(Vec::new());
    }
    // A tool's code may panic, as `Base::float` does on a parameter it lacks: that
    // tool is left out, never the photo.
    let settings = catch_unwind(AssertUnwindSafe(|| {
        let values = values.off_defaults(&tool.defaults(image));
        if values.is_empty() && !switched {
            return Vec::new();
        }
        let base = base.get_or_insert_with(|| Base::new(operations, image));
        tool.bind(&values, image, base)
    }))
    .map_err(|panic| ToolError::Panicked(panic_message(panic.as_ref())))?;
    for setting in &settings {
        check::setting(tool, operations, setting)?;
    }
    Ok(settings)
}

/// The instances `edit` renders on `image` (decision 28): every operation an unedited
/// photo runs, with the edit's parameters no tool owns, and every other operation only
/// while a binding sets one of its parameters or a switched tool owning one of them is
/// on (decision 32), each with what the bindings set, in the order of `operations`. A
/// tool is bound only with its controls off their defaults for the photo, and not at
/// all when it has none, but a switched tool on, bound with none, so its operations run
/// at their own defaults; a switched tool off is never bound. So an empty edit, or one
/// whose tools are all at their defaults and off, gives exactly the default pipeline.
/// A tool whose binding
/// sets a parameter it does not own or a value its schema refuses, or panics, is left
/// out whole and reported in [`Bound::failed`]; the others still render.
pub fn instances<'a>(
    edit: &Edit,
    operations: &[&'a dyn Operation],
    tools: &[&dyn ProcessingTool],
    image: &ImageContext,
) -> Bound<'a> {
    let mut bound: BTreeMap<&'static str, ParamValues> = BTreeMap::new();
    let mut failed = Vec::new();
    let mut base = None;
    for &tool in tools {
        match bind(tool, edit, operations, image, &mut base) {
            Ok(settings) => {
                // A switched tool on plans each operation it owns a parameter of.
                if tool.definition().switched && edit.is_on(tool) {
                    for owned in tool.owns() {
                        bound.entry(owned.operation).or_default();
                    }
                }
                for setting in settings {
                    bound
                        .entry(setting.operation)
                        .or_default()
                        .set(setting.param, setting.value);
                }
            }
            Err(error) => failed.push((tool.definition().tool.id, error)),
        }
    }
    // A tool not applied leaves out each operation it owns a parameter of (decision 78).
    let unapplied: BTreeSet<&str> = (tools.iter())
        .filter(|tool| edit.unapplied.contains(tool.definition().tool.id))
        .flat_map(|tool| tool.owns().iter().map(|owned| owned.operation))
        .collect();
    let instances = operations.iter().filter_map(|&operation| {
        let manifest = operation.manifest();
        if unapplied.contains(manifest.id) {
            return None;
        }
        let set = bound.remove(manifest.id);
        if !manifest.default_enabled && set.is_none() {
            return None;
        }
        let mut params = match edit.params.get(manifest.id) {
            Some(params) if manifest.default_enabled => params.clone(),
            _ => ParamValues::new(),
        };
        for (name, value) in set.iter().flat_map(ParamValues::iter) {
            params.set(name, value);
        }
        Some(Instance { operation, params })
    });
    Bound {
        instances: instances.collect(),
        failed,
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_tools::testing::{self, Defaultless, Pushing, Sample};
    use apertrust_tools::{Controls, Owned, ProcessingDefinition, Setting};

    use super::*;

    const TOOLS: &[&dyn ProcessingTool] = &[&Sample];

    #[test]
    fn defaults_that_panic_during_follow_restore_the_previous_edit() {
        let image = testing::image();
        let before = Edit::default();
        let mut edit = before.clone();
        edit.set_control(&Defaultless, "amount", ControlValue::Number(5.0), &image)
            .unwrap();
        assert!(matches!(
            edit.follow(&before, &Defaultless, &["amount"], &image),
            Err(EditError::FollowFailed {
                tool: "defaultless",
                ..
            })
        ));
        assert_eq!(edit, before);
    }

    /// Each instance's operation and the parameters it sets.
    fn planned(edit: &Edit) -> Vec<(&'static str, Vec<(String, Value)>)> {
        instances(edit, OPERATIONS, TOOLS, &testing::image())
            .all()
            .unwrap()
            .into_iter()
            .map(|instance| {
                let params = instance
                    .params
                    .iter()
                    .map(|(name, value)| (name.to_owned(), value))
                    .collect();
                (instance.operation.manifest().id, params)
            })
            .collect()
    }

    const BASE: [&str; 10] = [
        "rawprepare",
        "temperature",
        "highlights",
        "demosaic",
        "flip",
        "exposure",
        "colorin",
        "channelmixerrgb",
        "agx",
        "colorout",
    ];

    #[test]
    fn an_empty_edit_gives_exactly_the_default_pipeline() {
        let planned = planned(&Edit::default());
        let ids: Vec<_> = planned.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, BASE);
        assert!(planned.iter().all(|(_, params)| params.is_empty()));
    }

    #[test]
    fn a_tool_at_its_defaults_adds_nothing() {
        let mut edit = Edit::default();
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.0),
            &testing::image(),
        )
        .unwrap();
        edit.set_control(&Sample, "size", ControlValue::Choice(2), &testing::image())
            .unwrap();
        edit.keep(&Sample, "grain", true).unwrap();
        assert!(!edit.is_empty());
        assert_eq!(planned(&edit), planned(&Edit::default()));
    }

    #[test]
    fn tool_values_become_the_parameters_their_binding_derives() {
        let mut edit = Edit::default();
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.5),
            &testing::image(),
        )
        .unwrap();
        edit.set_control(
            &Sample,
            "grain",
            ControlValue::Number(40.0),
            &testing::image(),
        )
        .unwrap();
        edit.set_control(&Sample, "size", ControlValue::Choice(2), &testing::image())
            .unwrap();
        edit.set_param(OPERATIONS, TOOLS, "exposure", "black", Value::Float(0.01))
            .unwrap();
        let planned = planned(&edit);
        let ids: Vec<_> = planned.iter().map(|(id, _)| *id).collect();
        // Grain joins at its place, before colorout.
        assert_eq!(ids[..9], BASE[..9]);
        assert_eq!(ids[9..], ["grain", "colorout"]);
        let params = |id| &planned.iter().find(|(i, _)| *i == id).unwrap().1;
        // Brightness is relative to the photo's default exposure, +0.7 EV.
        assert_eq!(
            params("exposure"),
            &[
                ("black".to_owned(), Value::Float(0.01)),
                ("exposure".to_owned(), Value::Float(0.7 + 0.5))
            ]
        );
        let scale = Base::new(OPERATIONS, &testing::image()).float("grain", "scale");
        assert_eq!(
            params("grain"),
            &[
                ("scale".to_owned(), Value::Float(2.0 * scale)),
                ("strength".to_owned(), Value::Float(40.0))
            ]
        );

        // Back to the defaults, the pipeline is the default one again.
        edit.reset_control(&Sample, "grain").unwrap();
        assert!(!planned_ids(&edit).contains(&"grain"));
        edit.reset_tool(&Sample);
        edit.reset_param(OPERATIONS, TOOLS, "exposure", "black")
            .unwrap();
        assert!(edit.is_empty());
        assert_eq!(edit, Edit::default());
    }

    fn planned_ids(edit: &Edit) -> Vec<&'static str> {
        planned(edit).into_iter().map(|(id, _)| id).collect()
    }

    #[test]
    fn values_out_of_range_of_another_kind_or_unknown_are_refused() {
        let mut edit = Edit::default();
        let refused = |result: Result<(), EditError>| result.unwrap_err();
        assert!(matches!(
            refused(edit.set_control(
                &Sample,
                "brightness",
                ControlValue::Number(4.5),
                &testing::image()
            )),
            EditError::Control {
                source: ControlError::OutOfRange { .. },
                ..
            }
        ));
        assert!(matches!(
            refused(edit.set_control(
                &Sample,
                "size",
                ControlValue::Number(1.0),
                &testing::image()
            )),
            EditError::Control {
                source: ControlError::WrongKind { .. },
                ..
            }
        ));
        assert!(matches!(
            refused(edit.set_control(
                &Sample,
                "contrast",
                ControlValue::Number(1.0),
                &testing::image()
            )),
            EditError::UnknownControl { .. }
        ));
        assert!(matches!(
            tool(TOOLS, "exposure").map(|_| ()),
            Err(EditError::UnknownTool(_))
        ));
        assert!(edit.is_empty());
    }

    #[test]
    fn only_unowned_parameters_of_base_operations_are_set_at_operation_level() {
        let mut edit = Edit::default();
        let mut set =
            |operation, param, value| edit.set_param(OPERATIONS, TOOLS, operation, param, value);
        assert_eq!(
            set("exposure", "exposure", Value::Float(1.0)),
            Err(EditError::Owned {
                operation: "exposure",
                param: "exposure",
                tool: "sample"
            })
        );
        assert_eq!(
            set("vignette", "unbound", Value::Bool(false)),
            Err(EditError::Optional {
                operation: "vignette"
            })
        );
        assert!(matches!(
            set("exposure", "black", Value::Float(2.0)),
            Err(EditError::Param { .. })
        ));
        assert!(matches!(
            set("lens", "distortion", Value::Float(0.0)),
            Err(EditError::UnknownOperation(_))
        ));
        assert_eq!(set("exposure", "black", Value::Float(0.02)), Ok(()));
        // A parameter its tool fixes, with no control, is refused as such; one of its
        // controls' as one it sets.
        let tools = apertrust_tools::TOOLS;
        assert_eq!(
            edit.set_param(OPERATIONS, tools, "rawprepare", "left", Value::Int(4)),
            Err(EditError::Fixed {
                operation: "rawprepare",
                param: "left",
                tool: "rawprepare"
            })
        );
        assert_eq!(
            EditError::Fixed {
                operation: "rawprepare",
                param: "left",
                tool: "rawprepare"
            }
            .to_string(),
            "rawprepare.left is fixed by the tool rawprepare"
        );
        assert_eq!(
            edit.set_param(
                OPERATIONS,
                tools,
                "rawprepare",
                "raw_white_point",
                Value::Int(16000)
            ),
            Err(EditError::Owned {
                operation: "rawprepare",
                param: "raw_white_point",
                tool: "rawprepare"
            })
        );
        assert!(matches!(
            edit.set_param(
                OPERATIONS,
                tools,
                "channelmixerrgb",
                "version",
                Value::Choice(2)
            ),
            Err(EditError::Fixed { .. })
        ));
        // Without tools nothing is owned.
        assert_eq!(
            edit.set_param(OPERATIONS, &[], "exposure", "exposure", Value::Float(1.0)),
            Ok(())
        );
    }

    #[test]
    fn a_misbehaving_binding_leaves_out_only_its_own_tool() {
        use apertrust_tools::testing::Faulty;
        let tools: &[&dyn ProcessingTool] = &[&Faulty, &Sample];
        let mut edit = Edit::default();
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.5),
            &testing::image(),
        )
        .unwrap();
        let derived = |edit: &Edit| {
            let bound = instances(edit, OPERATIONS, tools, &testing::image());
            let exposure = bound
                .instances
                .iter()
                .find(|i| i.operation.manifest().id == "exposure")
                .unwrap()
                .params
                .clone();
            (exposure, bound.failed)
        };
        for (control, error) in [
            (
                "stray",
                ToolError::NotOwned {
                    operation: "exposure",
                    param: "black",
                },
            ),
            (
                "panic",
                ToolError::Panicked(
                    "exposure.no_such_parameter: expected a number, got None".to_owned(),
                ),
            ),
        ] {
            let mut edit = edit.clone();
            edit.set_control(
                &Faulty,
                control,
                ControlValue::Bool(true),
                &testing::image(),
            )
            .unwrap();
            let (exposure, failed) = derived(&edit);
            assert_eq!(failed, [("faulty", error)]);
            // Sample's brightness still renders, and nothing of the faulty tool does.
            assert_eq!(exposure.get("exposure"), Some(Value::Float(0.7 + 0.5)));
            assert_eq!(exposure.get("black"), None);
            assert!(
                instances(&edit, OPERATIONS, tools, &testing::image())
                    .all()
                    .is_err()
            );
        }
        assert!(derived(&edit).1.is_empty());
    }

    #[test]
    fn keeping_marks_a_control_and_releasing_leaves_its_value() {
        let mut edit = Edit::default();
        edit.keep(&Sample, "grain", true).unwrap();
        let values = &edit.tools["sample"];
        assert_eq!(
            (values.get("grain"), values.mark("grain")),
            (None, Some(Mark::Kept))
        );
        edit.keep(&Sample, "grain", false).unwrap();
        assert!(edit.is_empty());
        edit.set_control(
            &Sample,
            "grain",
            ControlValue::Number(10.0),
            &testing::image(),
        )
        .unwrap();
        edit.keep(&Sample, "grain", false).unwrap();
        let values = &edit.tools["sample"];
        assert_eq!(
            (values.number("grain"), values.mark("grain")),
            (Some(10.0), None)
        );
    }

    /// [`Sample`] with a look its tool gets wrong, as a bug may: `grainy` gives a
    /// brightness in range, then a grain past its own, and `untouched` gives no size.
    struct Wrong;

    impl ProcessingTool for Wrong {
        fn definition(&self) -> &'static ProcessingDefinition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ToolValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
            Sample.bind(values, image, base)
        }

        fn preset(&self, _: &str, option: &str, _: &ImageContext) -> Option<Controls> {
            let grainy = option == "grainy";
            let mut values = Controls::new();
            values.set("brightness", ControlValue::Number(0.5));
            values.set(
                "grain",
                ControlValue::Number(if grainy { 150.0 } else { 0.0 }),
            );
            if grainy {
                values.set("size", ControlValue::Choice(2));
            }
            Some(values)
        }
    }

    #[test]
    fn an_option_its_tool_gets_wrong_changes_nothing_and_is_the_tool_s_fault() {
        let image = testing::image();
        let mut edit = Edit::default();
        edit.set_control(
            &Wrong,
            "brightness",
            ControlValue::Number(1.0),
            &testing::image(),
        )
        .unwrap();
        edit.keep(&Wrong, "grain", true).unwrap();
        let before = edit.clone();
        for (option, says) in [
            (
                "grainy",
                "sample.look: grainy failed: grain is 150, outside 0 to 100",
            ),
            (
                "untouched",
                "sample.look: untouched failed: no value for size",
            ),
        ] {
            let error = edit
                .apply_preset(&Wrong, "look", option, &image)
                .unwrap_err();
            let EditError::PresetFailed {
                tool,
                preset,
                option: named,
                ..
            } = &error
            else {
                panic!("{error:?}");
            };
            assert_eq!((*tool, *preset, *named), ("sample", "look", option));
            assert_eq!(error.to_string(), says);
            // Not even the brightness that held: all or nothing.
            assert_eq!(edit, before, "{option}");
        }
    }

    /// [`Sample`] with a pair whose inverse overflows, as a bug may make it: grain comes
    /// back infinite, bias as Sample gives it.
    struct Overflowing;

    impl ProcessingTool for Overflowing {
        fn definition(&self) -> &'static ProcessingDefinition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ToolValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
            Sample.bind(values, image, base)
        }

        fn view(&self, view: &str, values: &Controls, image: &ImageContext) -> Option<Vec<f32>> {
            Sample.view(view, values, image)
        }

        fn unview(
            &self,
            view: &str,
            coordinates: &[f32],
            image: &ImageContext,
        ) -> Option<Controls> {
            let mut values = Sample.unview(view, coordinates, image)?;
            values.set("grain", ControlValue::Number(f32::INFINITY));
            Some(values)
        }
    }

    /// `control` set to `value` with its follow writes, as the session sets one.
    fn set_following(
        edit: &mut Edit,
        tool: &dyn ProcessingTool,
        control: &str,
        value: ControlValue,
    ) -> Result<(), EditError> {
        let before = edit.clone();
        edit.set_control(tool, control, value, &testing::image())?;
        edit.follow(&before, tool, written(tool, control), &testing::image())
    }

    #[test]
    fn a_change_s_follow_writes_are_written_with_it_each_kept() {
        use apertrust_tools::testing::{Follow, Following};
        let tool = &Following(Follow::Size);
        let mut edit = Edit::default();
        set_following(&mut edit, tool, "grain", ControlValue::Number(60.0)).unwrap();
        let values = &edit.tools["sample"];
        assert_eq!(
            (values.choice("size"), values.mark("size")),
            (Some(2), Some(Mark::Kept))
        );
        // Released, then a grain that writes the size it holds: nothing changes, so the
        // size stays released.
        edit.keep(tool, "size", false).unwrap();
        set_following(&mut edit, tool, "grain", ControlValue::Number(70.0)).unwrap();
        assert_eq!(edit.tools["sample"].mark("size"), None);
        // A grain set to the value it holds moves nothing, so writes nothing.
        edit.set_control(tool, "size", ControlValue::Choice(1), &testing::image())
            .unwrap();
        set_following(&mut edit, tool, "grain", ControlValue::Number(70.0)).unwrap();
        assert_eq!(edit.tools["sample"].choice("size"), Some(1));
        // A reset is a change to the default, with its follow writes: a fine grain.
        let before = edit.clone();
        edit.reset_control(tool, "grain").unwrap();
        edit.follow(&before, tool, written(tool, "grain"), &testing::image())
            .unwrap();
        let values = &edit.tools["sample"];
        assert_eq!(
            (values.get("grain"), values.choice("size")),
            (None, Some(0))
        );
        // What a command writes: a control, or the controls of the view of a coordinate.
        assert_eq!(written(tool, "grain"), ["grain"]);
        assert_eq!(written(tool, "sum"), ["grain", "bias"]);
        assert!(written(tool, "contrast").is_empty());
    }

    #[test]
    fn a_follow_write_its_tool_gets_wrong_is_refused_and_changes_nothing() {
        use apertrust_tools::testing::{Follow, Following};
        for (follow, says) in [
            (Follow::OutOfRange, "bias is 150, outside 0 to 100"),
            (Follow::Unknown, "writes contrast, which it lacks"),
            (Follow::Itself, "rewrites grain, which the change set"),
            (Follow::Panics, "no follow writes for grain"),
        ] {
            let tool = &Following(follow);
            let mut edit = Edit::default();
            edit.set_control(
                tool,
                "brightness",
                ControlValue::Number(1.0),
                &testing::image(),
            )
            .unwrap();
            let before = edit.clone();
            let error =
                set_following(&mut edit, tool, "grain", ControlValue::Number(60.0)).unwrap_err();
            assert!(
                matches!(
                    &error,
                    EditError::FollowFailed {
                        tool: "sample",
                        control: "grain",
                        message,
                    } if message == says
                ),
                "{follow:?}: {error:?}"
            );
            // Not even the grain the command set: all or nothing.
            assert_eq!(edit, before, "{follow:?}");
        }
    }

    /// `control` set to `value` as the session sets one, its range's moves clamped after.
    fn set_clamping(
        edit: &mut Edit,
        tool: &dyn ProcessingTool,
        control: &str,
        value: f32,
    ) -> Result<(), EditError> {
        let before = edit.clone();
        set_following(edit, tool, control, ControlValue::Number(value))?;
        edit.clamp(&before, &[tool], &testing::image())
    }

    /// Sample's coarseness reaches up to 100 % plus the bias (decision 30): a value outside
    /// the range the tool's values give now is refused; a bias that leaves it outside
    /// clamps it in the same change, kept, and one that clamps it to its default resets it.
    #[test]
    fn a_number_outside_its_moved_range_is_refused_and_one_a_change_leaves_outside_clamped() {
        let mut edit = Edit::default();
        let coarseness = |edit: &Edit| {
            let values = &edit.tools["sample"];
            (values.number("coarseness"), values.mark("coarseness"))
        };
        set_clamping(&mut edit, &Sample, "coarseness", 150.0).unwrap();
        edit.keep(&Sample, "coarseness", false).unwrap();
        set_clamping(&mut edit, &Sample, "bias", 20.0).unwrap();
        assert_eq!(coarseness(&edit), (Some(120.0), Some(Mark::Kept)));
        // Inside the definition's 50 to 200 %, outside the range a bias of 20 % gives.
        let refused = set_clamping(&mut edit, &Sample, "coarseness", 150.0).unwrap_err();
        assert_eq!(
            refused,
            EditError::Control {
                tool: "sample",
                source: ControlError::OutOfRange {
                    control: "coarseness",
                    value: 150.0,
                    min: 50.0,
                    max: 120.0
                }
            }
        );
        assert_eq!(coarseness(&edit), (Some(120.0), Some(Mark::Kept)));
        // A bias back up leaves it where the clamp put it.
        set_clamping(&mut edit, &Sample, "bias", 100.0).unwrap();
        assert_eq!(coarseness(&edit).0, Some(120.0));
        // Clamped to its default, 100 %, by a bias of 0: reset, value and mark.
        set_clamping(&mut edit, &Sample, "bias", 0.0).unwrap();
        assert_eq!(coarseness(&edit), (None, None));
    }

    #[test]
    fn a_clamp_of_a_value_the_change_wrote_is_the_tool_s_failure_and_changes_nothing() {
        let mut edit = Edit::default();
        set_clamping(&mut edit, &Pushing, "grain", 10.0).unwrap();
        let before = edit.clone();
        let error = set_clamping(&mut edit, &Pushing, "bias", 20.0).unwrap_err();
        assert_eq!(
            error,
            EditError::ClampFailed {
                tool: "sample",
                control: "coarseness"
            }
        );
        assert_eq!(edit, before);
        // A bias whose range holds the coarseness it writes is a change like any other.
        set_clamping(&mut edit, &Pushing, "bias", 95.0).unwrap();
        assert_eq!(edit.tools["sample"].number("coarseness"), Some(190.0));
    }

    /// A saved edit holding a number outside the range its tool's values give, as only a
    /// hand-written file does, is clamped as it is read, its mark as it was, with a warning.
    #[test]
    fn a_saved_number_outside_its_moved_range_is_clamped_as_it_is_read() {
        let image = testing::image();
        let mut edit = Edit::default();
        // Set in an order no change can reach: the bias's clamp skipped.
        edit.set_control(&Sample, "coarseness", ControlValue::Number(150.0), &image)
            .unwrap();
        edit.keep(&Sample, "coarseness", false).unwrap();
        edit.set_control(&Sample, "bias", ControlValue::Number(20.0), &image)
            .unwrap();
        let warnings = edit.clamp_read(TOOLS, &image);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].starts_with("sample.coarseness is 150"),
            "{warnings:?}"
        );
        let values = &edit.tools["sample"];
        assert_eq!(
            (values.number("coarseness"), values.mark("coarseness")),
            (Some(120.0), None)
        );
        assert!(edit.clamp_read(TOOLS, &image).is_empty());
    }

    /// A tool whose defaults for the photo panic is passed over where every tool's
    /// ranges are asked, never the change or the photo (decision 30): its own number is
    /// held to its definition's range, another tool's change clamps as it would without
    /// it, and so does a saved edit as it is read, the faulty tool's value as it was.
    #[test]
    fn a_tool_whose_defaults_panic_leaves_the_others_ranges_to_hold() {
        let image = testing::image();
        let tools: &[&dyn ProcessingTool] = &[&Defaultless, &Sample];
        let number = ControlValue::Number;
        let mut edit = Edit::default();
        edit.set_control(&Defaultless, "amount", number(5.0), &image)
            .unwrap();
        let outside = edit.set_control(&Defaultless, "amount", number(50.0), &image);
        assert!(
            matches!(
                outside,
                Err(EditError::Control {
                    tool: "defaultless",
                    source: ControlError::OutOfRange { .. }
                })
            ),
            "{outside:?}"
        );
        edit.set_control(&Sample, "coarseness", number(150.0), &image)
            .unwrap();
        let before = edit.clone();
        edit.set_control(&Sample, "bias", number(20.0), &image)
            .unwrap();
        edit.clamp(&before, tools, &image).unwrap();
        assert_eq!(edit.tools["sample"].number("coarseness"), Some(120.0));

        // Saved in an order no change can reach, as a hand-written file may hold it.
        let mut saved = before;
        saved
            .set_control(&Sample, "bias", number(20.0), &image)
            .unwrap();
        let warnings = saved.clamp_read(tools, &image);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(saved.tools["sample"].number("coarseness"), Some(120.0));
        assert_eq!(saved.tools["defaultless"].number("amount"), Some(5.0));
    }

    #[test]
    fn an_infinite_value_from_a_view_s_inverse_is_the_tool_s_fault_and_changes_nothing() {
        let image = testing::image();
        let mut edit = Edit::default();
        edit.set_control(
            &Overflowing,
            "brightness",
            ControlValue::Number(1.0),
            &testing::image(),
        )
        .unwrap();
        let before = edit.clone();
        let mut held = Held::default();
        let error = edit
            .set_coordinate(
                &mut held,
                &Overflowing,
                "difference",
                ControlValue::Number(-40.0),
                &image,
            )
            .unwrap_err();
        assert!(
            matches!(
                &error,
                EditError::ViewFailed {
                    tool: "sample",
                    view: "pair",
                    message,
                } if message == "no finite number for grain"
            ),
            "{error:?}"
        );
        assert_eq!(edit, before);
        assert!(held.0.is_empty());
    }

    /// Sample, but its size lists coarse only on a photo that records an exposure bias, as
    /// a tool's lists may follow the photo (decision 30), and its size's list panicking
    /// or out of order where `Fault` says. So the look's grainy, and its grain above 50 %
    /// through Following's size, write coarse, which a photo may not list. Each test's
    /// thread counts the times it is asked for a list.
    struct Narrow(Fault);

    thread_local! {
        static NARROW_ASKED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Fault {
        None,
        Panics,
        Unordered,
    }

    impl ProcessingTool for Narrow {
        fn definition(&self) -> &'static ProcessingDefinition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ToolValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
            Sample.bind(values, image, base)
        }

        fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
            NARROW_ASKED.with(|asked| asked.set(asked.get() + 1));
            let bias = image.camera.as_ref().map_or(0.0, |c| c.exposure_bias);
            match (control, self.0) {
                ("size", Fault::Panics) => panic!("no list for size"),
                ("size", Fault::Unordered) => vec!["medium", "fine"],
                ("size", Fault::None) if bias == 0.0 => vec!["fine", "medium"],
                _ => Sample.options(control, image),
            }
        }

        fn preset(&self, preset: &str, option: &str, image: &ImageContext) -> Option<Controls> {
            Sample.preset(preset, option, image)
        }

        fn follow(&self, control: &str, values: &Controls, image: &ImageContext) -> Controls {
            testing::Following(testing::Follow::Size).follow(control, values, image)
        }
    }

    /// The photo `testing::image` gives, recording this exposure bias.
    fn biased(bias: f32) -> ImageContext {
        let mut image = testing::image();
        if let Some(camera) = &mut image.camera {
            camera.exposure_bias = bias;
        }
        image
    }

    #[test]
    fn an_option_the_photo_does_not_list_is_refused_and_one_it_lists_set() {
        let (plain, image) = (testing::image(), biased(-0.7));
        let mut edit = Edit::default();
        let camera = ControlValue::Choice(3);
        assert_eq!(
            edit.set_control(&Sample, "method", camera.clone(), &plain),
            Err(EditError::NotListed {
                tool: "sample",
                control: "method",
                option: "camera",
            })
        );
        assert!(edit.is_empty());
        edit.set_control(&Sample, "method", camera.clone(), &image)
            .unwrap();
        assert_eq!(edit.tool("sample").unwrap().get("method"), Some(&camera));
        edit.reset_control(&Sample, "method").unwrap();
        assert!(edit.is_empty());
        // A number reads no list; a choice reads its own.
        let grain = Sample.definition().control("grain").unwrap();
        assert_eq!(listed(&Sample, grain, &plain), (Vec::new(), None));
        let asked = || NARROW_ASKED.with(std::cell::Cell::get);
        let before = asked();
        let narrow = Narrow(Fault::None);
        edit.set_control(&narrow, "grain", ControlValue::Number(10.0), &plain)
            .unwrap();
        assert_eq!(asked(), before);
        edit.set_control(&narrow, "size", ControlValue::Choice(0), &plain)
            .unwrap();
        assert_eq!(asked(), before + 1);
    }

    #[test]
    fn a_preset_or_a_follow_write_of_an_option_the_photo_does_not_list_is_the_tool_s_failure() {
        let plain = testing::image();
        let tool = Narrow(Fault::None);
        let mut edit = Edit::default();
        let error = edit
            .apply_preset(&tool, "look", "grainy", &plain)
            .unwrap_err();
        assert!(
            matches!(
                &error,
                EditError::PresetFailed { option: "grainy", message, .. }
                    if message == "size coarse, which this photo does not list"
            ),
            "{error:?}"
        );
        assert!(edit.is_empty());
        // On a photo that lists coarse, grainy applies.
        edit.apply_preset(&tool, "look", "grainy", &biased(-0.7))
            .unwrap();
        // Grain above 50 % writes size coarse, which this photo does not list.
        let mut edit = Edit::default();
        let before = edit.clone();
        edit.set_control(&tool, "grain", ControlValue::Number(60.0), &plain)
            .unwrap();
        let error = edit.follow(&before, &tool, &["grain"], &plain).unwrap_err();
        assert!(
            matches!(
                &error,
                EditError::FollowFailed { control: "grain", message, .. }
                    if message == "writes size coarse, which this photo does not list"
            ),
            "{error:?}"
        );
        assert_eq!(edit, before);
        // A write of the option the size holds already, from another photo's edit, changes
        // nothing and fails nothing.
        let mut edit = Edit::default();
        edit.set_control(&tool, "size", ControlValue::Choice(2), &biased(-0.7))
            .unwrap();
        let before = edit.clone();
        edit.set_control(&tool, "grain", ControlValue::Number(60.0), &plain)
            .unwrap();
        edit.follow(&before, &tool, &["grain"], &plain).unwrap();
        let size = edit.tool("sample").unwrap().get("size");
        assert_eq!(size, Some(&ControlValue::Choice(2)));
    }

    #[test]
    fn a_list_that_panics_or_does_not_hold_together_lists_every_option() {
        let plain = testing::image();
        let size = Sample.definition().control("size").unwrap();
        let narrow = listed(&Narrow(Fault::None), size, &plain);
        assert_eq!(narrow, (vec!["fine", "medium"], None));
        for (fault, says) in [
            (Fault::Panics, "no list for size"),
            (
                Fault::Unordered,
                "size lists an option twice or out of order",
            ),
        ] {
            let (listed, failed) = listed(&Narrow(fault), size, &plain);
            assert_eq!(listed, ["fine", "medium", "coarse"]);
            assert_eq!(failed.as_deref(), Some(says));
            // So every option is taken.
            let mut edit = Edit::default();
            edit.set_control(&Narrow(fault), "size", ControlValue::Choice(2), &plain)
                .unwrap();
        }
    }

    /// Each instance's operation planned for `edit` with Grain, a switched tool, alone.
    fn planned_with_grain(edit: &Edit) -> Vec<(&'static str, Vec<(String, Value)>)> {
        let tools: &[&dyn ProcessingTool] = &[&apertrust_tools::grain::Grain];
        instances(edit, OPERATIONS, tools, &testing::image())
            .all()
            .unwrap()
            .into_iter()
            .map(|instance| {
                let params = instance
                    .params
                    .iter()
                    .map(|(name, value)| (name.to_owned(), value))
                    .collect();
                (instance.operation.manifest().id, params)
            })
            .collect()
    }

    /// A switched tool (decision 32): off until a control is written, whatever the
    /// value, and then on, its operation planned at its own defaults for every control not
    /// set; a control reset or kept leaves it on, and its reset turns it off.
    #[test]
    fn a_switched_tool_is_on_from_a_control_s_write_until_its_reset() {
        use apertrust_tools::grain::Grain;
        let image = testing::image();
        let none = planned_with_grain(&Edit::default());
        let grain = |edit: &Edit| {
            planned_with_grain(edit)
                .into_iter()
                .find(|(id, _)| *id == "grain")
                .map(|(_, params)| params)
        };
        let mut edit = Edit::default();
        assert!(!edit.is_on(&Grain));
        // Keeping a control never turns it on.
        edit.keep(&Grain, "scale", true).unwrap();
        assert!(!edit.is_on(&Grain));
        assert_eq!(planned_with_grain(&edit), none);
        // A write at the default turns it on, grain at the operation's defaults.
        edit.set_control(&Grain, "strength", ControlValue::Number(25.0), &image)
            .unwrap();
        assert!(edit.is_on(&Grain));
        assert_eq!(grain(&edit), Some(Vec::new()));
        // A move binds only the control moved.
        edit.set_control(&Grain, "strength", ControlValue::Number(40.0), &image)
            .unwrap();
        let strength = vec![("strength".to_owned(), Value::Float(40.0))];
        assert_eq!(grain(&edit), Some(strength));
        // Its reset, and every keep released, leave it on at darktable's defaults.
        edit.reset_control(&Grain, "strength").unwrap();
        edit.keep(&Grain, "scale", false).unwrap();
        assert!(edit.is_on(&Grain));
        assert!(!edit.is_empty());
        assert_eq!(grain(&edit), Some(Vec::new()));
        // The tool's reset turns it off, as no edit.
        edit.reset_tool(&Grain);
        assert!(!edit.is_on(&Grain));
        assert_eq!(edit, Edit::default());
        assert_eq!(planned_with_grain(&edit), none);
    }

    /// A tool that is not switched is always on, and a write leaves no `enabled` in its
    /// values.
    #[test]
    fn a_tool_not_switched_is_always_on() {
        let mut edit = Edit::default();
        assert!(edit.is_on(&Sample));
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.5),
            &testing::image(),
        )
        .unwrap();
        assert!(!edit.tool("sample").unwrap().enabled());
    }
}
