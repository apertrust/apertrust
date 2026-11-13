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

//! An edit (decision 28): the values of the modules it touches, and the operation
//! parameters no module owns yet, only of operations an unedited photo runs. Nothing
//! derived is stored: [`instances`] turns an edit into what the pipeline plans, and every
//! place that builds instances goes through it (the session, export and the CLI, the
//! whole-pipeline test, the engine's tests of the default pipeline), so the viewport and
//! the export never differ and no optional operation slips into a default pipeline.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use apertrust_modules::check::{self, ModuleError};
use apertrust_modules::{
    Base, Control, ControlError, ControlKind, ControlValue, Controls, Coordinate, Mark, Module,
    ModuleValues, Picker, Range, Unit, View,
};
use apertrust_plugin::{ImageContext, Operation, Param, ParamError, ParamValues, Value};
use thiserror::Error;

use crate::pipeline::Instance;

/// What the user changed, as plain data. It changes only through its checked methods (and
/// the sidecar's reader, which checks the same), so no owned parameter, nor one of an
/// operation an unedited photo does not run, gets in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Edit {
    /// Each module the edit touches, by id: the controls set and their marks.
    pub(crate) modules: BTreeMap<String, ModuleValues>,
    /// Parameters no module owns, of operations an unedited photo runs, by operation id.
    pub(crate) params: BTreeMap<String, ParamValues>,
    /// The default modules not applied: the history's end lies before
    /// their default step, and darktable leaves them off until its workflow applies them
    /// (decision 78). Left out of the plan and reported off; derived from the log, never
    /// stored, and none once the edit's values move away from the end's.
    pub(crate) unapplied: BTreeSet<String>,
}

/// A change an edit refuses, or a binding that sets what it may not.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum EditError {
    #[error("no module {0}")]
    UnknownModule(String),
    #[error("{module} has no control {control}")]
    UnknownControl {
        module: &'static str,
        control: String,
    },
    #[error("{module}: {source}")]
    Control {
        module: &'static str,
        source: ControlError,
    },
    /// An option of the choice that this photo does not list (decision 30), as
    /// demosaic's other sensor's methods: only an edit from another photo holds one.
    #[error("{module}.{control}: this photo does not list {option}")]
    NotListed {
        module: &'static str,
        control: &'static str,
        option: &'static str,
    },
    #[error("{module} has no preset row {preset}")]
    UnknownPreset {
        module: &'static str,
        preset: String,
    },
    #[error("{module}.{preset} has no option {option}")]
    UnknownOption {
        module: &'static str,
        preset: &'static str,
        option: String,
    },
    #[error("{module}.{preset}: this photo cannot have {option}")]
    Unavailable {
        module: &'static str,
        preset: &'static str,
        option: &'static str,
    },
    /// The module's own fault, never the user's: its preset panicked for `option`, gave a
    /// value its control cannot take, or gave none for a control the row writes.
    #[error("{module}.{preset}: {option} failed: {message}")]
    PresetFailed {
        module: &'static str,
        preset: &'static str,
        option: &'static str,
        message: String,
    },
    /// A module's colour picker's pick (decision 79) the module got wrong: a write of a
    /// control the picker does not write, of a value its control cannot take, of a
    /// choice's option the photo does not list, or a pick that panicked. The module's
    /// fault; nothing is written.
    #[error("{module}'s picker {picker} failed: {message}")]
    PickFailed {
        module: &'static str,
        picker: &'static str,
        message: String,
    },
    /// A coordinate's value, within its own range, that would put one of its view's
    /// controls outside its range.
    #[error("{module}.{coordinate} refused: {source}")]
    Coordinate {
        module: &'static str,
        coordinate: &'static str,
        source: ControlError,
    },
    /// The module's own fault, never the user's: its view or its inverse panicked, gave no
    /// coordinates or not one for each, or gave no finite number for a control the view
    /// covers.
    #[error("{module}.{view} failed: {message}")]
    ViewFailed {
        module: &'static str,
        view: &'static str,
        message: String,
    },
    /// The module's own fault, never the user's: its follow writes for `control` panicked,
    /// named a control it lacks or one the change set itself, or gave a value its control
    /// cannot take (decision 27).
    #[error("{module}: following {control} failed: {message}")]
    FollowFailed {
        module: &'static str,
        control: &'static str,
        message: String,
    },
    /// The module's own fault, never the user's: a change moved the range of a control it
    /// wrote itself, the command's or a follow write's, leaving it outside (decision 30).
    #[error("{module}: the change moved {control}'s range away from the value it wrote")]
    ClampFailed {
        module: &'static str,
        control: &'static str,
    },
    #[error("no operation {0}")]
    UnknownOperation(String),
    #[error("{operation}: {source}")]
    Param {
        operation: &'static str,
        source: ParamError,
    },
    #[error("{operation}.{param} belongs to the module {module}, which sets it")]
    Owned {
        operation: &'static str,
        param: &'static str,
        module: &'static str,
    },
    /// A parameter its module owns with no control, holding it at the operation's value
    /// for the photo (`Module::fixed`), as Raw black/white point holds the crop.
    #[error("{operation}.{param} is fixed by the module {module}")]
    Fixed {
        operation: &'static str,
        param: &'static str,
        module: &'static str,
    },
    #[error("{operation} runs only while a module sets it, so only modules set its parameters")]
    Optional { operation: &'static str },
    #[error("the module {module}: {source}")]
    Binding {
        module: &'static str,
        source: ModuleError,
    },
}

impl Edit {
    /// Nothing set and nothing marked: the photo as Apertrust first shows it.
    pub fn is_empty(&self) -> bool {
        self.modules.values().all(ModuleValues::is_empty)
            && self.params.values().all(|p| p.iter().next().is_none())
    }

    /// The values of the module called `id`, if the edit touches it.
    pub fn module(&self, id: &str) -> Option<&ModuleValues> {
        self.modules.get(id)
    }

    /// Every module the edit touches, by id.
    pub fn modules(&self) -> impl Iterator<Item = (&str, &ModuleValues)> {
        self.modules
            .iter()
            .map(|(id, values)| (id.as_str(), values))
    }

    /// The parameters the edit sets on the operation called `id`.
    pub fn params(&self, id: &str) -> Option<&ParamValues> {
        self.params.get(id)
    }

    /// Sets a control, refusing a value of another kind, a number outside the range the
    /// module's values give it now (`Module::range`, decision 30), the definition's where
    /// the module's defaults for the photo panic, and a choice's option the photo, `image`,
    /// does not list (decision 30). Setting a control keeps it, so Auto and Match leave it
    /// alone, and turns a switched module on, whatever the value (decision 32).
    pub fn set_control(
        &mut self,
        module: &dyn Module,
        control: &str,
        value: ControlValue,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let (id, control) = find_control(module, control)?;
        control
            .check(&value)
            .map_err(|source| EditError::Control { module: id, source })?;
        // A module's code may panic, as its binding is guarded against: the range is then
        // the definition's, which the check above holds the value to.
        let range = || {
            let every = catch_unwind(AssertUnwindSafe(|| self.every(module, image))).ok()?;
            range_now(module, control.id, &every)
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
            return Err(EditError::Control { module: id, source });
        }
        if let Some(option) = unlisted(module, control, &value, image) {
            return Err(EditError::NotListed {
                module: id,
                control: control.id,
                option,
            });
        }
        let values = self.modules.entry(id.to_owned()).or_default();
        values.set(control.id, value);
        values.set_mark(control.id, Some(Mark::Kept));
        self.switch_on(module);
        Ok(())
    }

    /// Turns a switched module on (decision 32), as every write of one of its controls
    /// does; a module that is not switched is always on, and stays as it is.
    fn switch_on(&mut self, module: &dyn Module) {
        let definition = module.definition();
        if definition.switched {
            let values = self.modules.entry(definition.id.to_owned()).or_default();
            values.set_enabled(true);
        }
    }

    /// Whether the module renders: a switched module while it is on (decision 32), every
    /// other but while the history's end lies before its default step (decision 78).
    pub fn is_on(&self, module: &dyn Module) -> bool {
        let definition = module.definition();
        if definition.switched {
            (self.modules.get(definition.id)).is_some_and(ModuleValues::enabled)
        } else {
            !self.unapplied.contains(definition.id)
        }
    }

    /// Takes a control back to its default for the photo, no longer kept. A switched
    /// module on stays on (decision 32).
    pub fn reset_control(&mut self, module: &dyn Module, control: &str) -> Result<(), EditError> {
        let (id, control) = find_control(module, control)?;
        if let Some(values) = self.modules.get_mut(id) {
            values.remove(control.id);
            values.set_mark(control.id, None);
        }
        self.tidy(id);
        Ok(())
    }

    /// Applies `option` of the module's preset row `preset` on `image` (decision 2): each
    /// control the row writes takes the option's value for the photo and is kept, since
    /// choosing an option is the user's own action, and the module's other controls stay
    /// as they are. An option the photo cannot have, or the row lacks, is refused, and so
    /// is one whose module fails, giving values its controls refuse, a choice's option the
    /// photo does not list (decision 30), or panicking: every value is checked before any
    /// is set, so a refused option changes nothing. Applied, it turns a switched module on
    /// (decision 32).
    pub fn apply_preset(
        &mut self,
        module: &dyn Module,
        preset: &str,
        option: &str,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let definition = module.definition();
        let id = definition.id;
        let row = definition
            .preset(preset)
            .ok_or_else(|| EditError::UnknownPreset {
                module: id,
                preset: preset.to_owned(),
            })?;
        let option = row
            .options
            .iter()
            .copied()
            .find(|candidate| *candidate == option)
            .ok_or_else(|| EditError::UnknownOption {
                module: id,
                preset: row.id,
                option: option.to_owned(),
            })?;
        let failed = |message: String| EditError::PresetFailed {
            module: id,
            preset: row.id,
            option,
            message,
        };
        // A module's code may panic, as its binding is guarded against: the option is
        // refused, never the photo.
        let given = catch_unwind(AssertUnwindSafe(|| module.preset(row.id, option, image)))
            .map_err(|panic| failed(panic_message(panic.as_ref())))?
            .ok_or(EditError::Unavailable {
                module: id,
                preset: row.id,
                option,
            })?;
        let mut checked = Vec::with_capacity(row.controls.len());
        for &control in row.controls {
            let value = given
                .get(control)
                .ok_or_else(|| failed(format!("no value for {control}")))?;
            let (_, found) = find_control(module, control)?;
            found
                .check(value)
                .map_err(|error| failed(error.to_string()))?;
            if let Some(unlisted) = unlisted(module, found, value, image) {
                return Err(failed(format!(
                    "{control} {unlisted}, which this photo does not list"
                )));
            }
            checked.push((control, value.clone()));
        }
        let values = self.modules.entry(id.to_owned()).or_default();
        for (control, value) in checked {
            values.set(control, value);
            values.set_mark(control, Some(Mark::Kept));
        }
        self.switch_on(module);
        Ok(())
    }

    /// Writes what the module's colour picker `picker` picked (decision 79; `docs/pickers.md`,
    /// What a pick writes): each control of `writes` takes its value and is kept, with no
    /// follow writes, since a pick writes every control darktable's `color_picker_apply`
    /// keeps consistent. A value is stored as the module's rule gave it, with one bound:
    /// a number past the range the module's values give its control now, the
    /// parameter's hard limits, is held at the nearer limit, and a hue, a control in
    /// degrees over a whole turn, wrapped onto its turn instead, the same angle. A control
    /// the picker does not write, a value its control cannot take or a choice's option
    /// the photo does not list (decision 30) is the module's failure, and nothing is
    /// written. It turns a switched module on (decision 32).
    pub fn pick(
        &mut self,
        module: &dyn Module,
        picker: &Picker,
        writes: &Controls,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let id = module.definition().id;
        let failed = |message: String| EditError::PickFailed {
            module: id,
            picker: picker.id,
            message,
        };
        let every = catch_unwind(AssertUnwindSafe(|| self.every(module, image))).ok();
        let mut checked = Vec::new();
        for (control, value) in writes.iter() {
            if !picker.writes.contains(&control) {
                return Err(failed(format!("it writes {control}, which it may not")));
            }
            let (_, found) = find_control(module, control)?;
            let value = match (value, found.kind) {
                (&ControlValue::Number(number), ControlKind::Number { min, max, .. }) => {
                    let (min, max) = every
                        .as_ref()
                        .and_then(|every| range_now(module, control, every))
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
            if let Some(unlisted) = unlisted(module, found, &value, image) {
                return Err(failed(format!(
                    "{control} {unlisted}, which this photo does not list"
                )));
            }
            checked.push((control, value));
        }
        let values = self.modules.entry(id.to_owned()).or_default();
        for (control, value) in checked {
            values.set(control, value);
            values.set_mark(control, Some(Mark::Kept));
        }
        self.switch_on(module);
        Ok(())
    }

    /// Turns a switched module on (decision 32), as its picker turning on does, darktable
    /// turning the module on with its picker; a module that is not switched is always on.
    pub fn turn_on(&mut self, module: &dyn Module) {
        self.switch_on(module);
    }

    /// Sets `coordinate`, a coordinate of one of the module's lossless views, to `value`
    /// on `image` (decision 2), holding the view's other coordinates: those `held` keeps
    /// while the view's controls hold what it last wrote, else the module's view of the
    /// edit's values. The view's controls take [`Module::unview`]'s values, each kept, and
    /// `held` keeps the coordinates. A value outside the coordinate's range, or one that
    /// would put a control outside its own, is refused, and so is one whose module fails:
    /// every value is checked before any is set, so a refused one changes nothing. Set to
    /// the value it has, it writes no value and keeps the controls, as setting a control
    /// keeps it, so the stored values move only when the view is edited. Either way it
    /// turns a switched module on, as setting a control does (decision 32).
    pub fn set_coordinate(
        &mut self,
        held: &mut Held,
        module: &dyn Module,
        coordinate: &str,
        value: ControlValue,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let (id, view, at, found) = find_coordinate(module, coordinate)?;
        let value = found
            .check(&value)
            .map_err(|source| EditError::Control { module: id, source })?;
        let mut coordinates = self.coordinates(held, module, view, image)?;
        if coordinates[at] == value {
            self.keep_coordinate(module, found.id, true)?;
            self.switch_on(module);
            return Ok(());
        }
        coordinates[at] = value;
        self.write_view(held, module, view, coordinates, found.id, image)
    }

    /// Takes `coordinate` back to its value at the module's defaults for the photo,
    /// holding the view's other coordinates, as [`Self::set_coordinate`] does. Once every
    /// coordinate of the view is at its defaults' value, the view's controls are reset
    /// exactly, values and marks, so an untouched module stays untouched.
    pub fn reset_coordinate(
        &mut self,
        held: &mut Held,
        module: &dyn Module,
        coordinate: &str,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let (id, view, at, found) = find_coordinate(module, coordinate)?;
        let defaults = default_coordinates(module, view, image)?;
        let mut coordinates = self.coordinates(held, module, view, image)?;
        let unchanged = coordinates[at] == defaults[at];
        coordinates[at] = defaults[at];
        if coordinates == defaults {
            if let Some(values) = self.modules.get_mut(id) {
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
        self.write_view(held, module, view, coordinates, found.id, image)
    }

    /// Keeps or releases the controls of the view `coordinate` belongs to: a view's marks
    /// are its controls'.
    pub fn keep_coordinate(
        &mut self,
        module: &dyn Module,
        coordinate: &str,
        kept: bool,
    ) -> Result<(), EditError> {
        let (_, view, _, _) = find_coordinate(module, coordinate)?;
        for control in view.controls {
            self.keep(module, control, kept)?;
        }
        Ok(())
    }

    /// The coordinates of the module's view `view` on `image`: those `held` keeps while
    /// the view's controls hold what it last wrote, else [`Module::view`] of every control,
    /// the edit's value over its default for the photo.
    pub fn coordinates(
        &self,
        held: &Held,
        module: &dyn Module,
        view: &View,
        image: &ImageContext,
    ) -> Result<Vec<f32>, EditError> {
        let definition = module.definition();
        if let Some(coordinates) = held.get(self, definition.id, view) {
            return Ok(coordinates.to_vec());
        }
        viewed(module, view, image, || self.every(module, image))
    }

    /// Every control of the module on `image`: the edit's value over its default for the
    /// photo.
    pub(crate) fn every(&self, module: &dyn Module, image: &ImageContext) -> Controls {
        let definition = module.definition();
        let mut values = module.defaults(image);
        if let Some(set) = self.modules.get(definition.id) {
            for control in definition.controls {
                if let Some(value) = set.get(control.id) {
                    values.set(control.id, value.clone());
                }
            }
        }
        values
    }

    /// The module's follow writes (decision 27; `docs/modules/README.md`, rule 13) for a
    /// change from `before` that set `set`, the controls a command wrote ([`written`]):
    /// for each of them whose value the change moved, in the definition's order, what
    /// [`Module::follow`] gives on `image`, a later control's write over an earlier one's,
    /// each written and kept, as the command's own are, but a write of the control's
    /// default for the photo resets it, value and mark, as a reset does. A write that
    /// leaves a control as it is changes nothing. A write to a control the module lacks or
    /// the change set, of a value its control cannot take or a choice's option the photo
    /// does not list (decision 30), or a follow that panics is the module's failure,
    /// [`EditError::FollowFailed`], and the edit is `before` again: a refused change
    /// changes nothing.
    pub fn follow(
        &mut self,
        before: &Edit,
        module: &dyn Module,
        set: &[&str],
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let result = self.write_follows(before, module, set, image);
        if result.is_err() {
            *self = before.clone();
        }
        result
    }

    /// [`Self::follow`]'s writes, every one checked before any is made.
    fn write_follows(
        &mut self,
        before: &Edit,
        module: &dyn Module,
        set: &[&str],
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let definition = module.definition();
        let id = definition.id;
        let (now, then) = (self.every(module, image), before.every(module, image));
        let failed = |control: &'static str, message: String| EditError::FollowFailed {
            module: id,
            control,
            message,
        };
        // Each write with the control whose follow wrote it.
        let mut writes: Vec<(&'static str, &'static Control, ControlValue)> = Vec::new();
        let moved = definition.controls.iter().filter(|control| {
            set.contains(&control.id) && now.get(control.id) != then.get(control.id)
        });
        for control in moved {
            let failed = |message: String| failed(control.id, message);
            // A module's code may panic, as its binding is guarded against: the change is
            // refused, never the photo.
            let given = catch_unwind(AssertUnwindSafe(|| module.follow(control.id, &now, image)))
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
            if let Some(unlisted) = unlisted(module, control, value, image) {
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
        let defaults = module.defaults(image);
        let values = self.modules.entry(id.to_owned()).or_default();
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
    /// `docs/engine.md`, A range that follows other controls): for each of `modules`,
    /// [`check::clamp`]'s writes, each kept, as follow writes are, but a write of the
    /// control's default for the photo resets it, value and mark. A clamp of a control the
    /// change wrote itself, the command's or a follow write's, is the module's failure,
    /// [`EditError::ClampFailed`], and the edit is `before` again: a refused change changes
    /// nothing. A module whose defaults or ranges panic clamps nothing, its ranges being
    /// the definition's, which every value holds.
    pub fn clamp(
        &mut self,
        before: &Edit,
        modules: &[&dyn Module],
        image: &ImageContext,
    ) -> Result<(), EditError> {
        for &module in modules {
            let id = module.definition().id;
            // A module's code may panic, as its binding is guarded against: that module
            // clamps nothing, and the change goes on.
            let Ok((now, then, writes, defaults)) = catch_unwind(AssertUnwindSafe(|| {
                let now = self.every(module, image);
                let writes = check::clamp(module, &now);
                (
                    now,
                    before.every(module, image),
                    writes,
                    module.defaults(image),
                )
            })) else {
                continue;
            };
            if let Some((control, _)) = writes
                .iter()
                .find(|(control, _)| now.get(control) != then.get(control))
            {
                let error = EditError::ClampFailed {
                    module: id,
                    control,
                };
                *self = before.clone();
                return Err(error);
            }
            for (control, value) in writes.iter() {
                let values = self.modules.entry(id.to_owned()).or_default();
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

    /// Brings a saved edit's numbers inside the ranges its modules' values give on `image`
    /// (decision 30), as a number outside its control's range is clamped as it is read
    /// (decision 21): each written at its range's nearer end, its mark as it was. Returns a
    /// warning for each, empty when every number holds; only a hand-written file holds one.
    /// A module whose defaults or ranges panic is left as it is.
    pub fn clamp_read(&mut self, modules: &[&dyn Module], image: &ImageContext) -> Vec<String> {
        let mut warnings = Vec::new();
        for &module in modules {
            let id = module.definition().id;
            if !self.modules.contains_key(id) {
                continue;
            }
            // A module's code may panic, as its binding is guarded against: that module is
            // left as it is, and the photo opens.
            let Ok((now, writes, defaults)) = catch_unwind(AssertUnwindSafe(|| {
                let now = self.every(module, image);
                let writes = check::clamp(module, &now);
                (now, writes, module.defaults(image))
            })) else {
                continue;
            };
            let values = self.modules.entry(id.to_owned()).or_default();
            for (control, value) in writes.iter() {
                let (Some(ControlValue::Number(was)), ControlValue::Number(is)) =
                    (now.get(control), value)
                else {
                    continue;
                };
                warnings.push(format!(
                    "{id}.{control} is {was}, outside the range its module's values give; {is} instead"
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

    /// Writes `coordinates` into the view's controls through [`Module::unview`], each
    /// kept, once every value is checked, and holds them; `coordinate` is the one set.
    fn write_view(
        &mut self,
        held: &mut Held,
        module: &dyn Module,
        view: &'static View,
        coordinates: Vec<f32>,
        coordinate: &'static str,
        image: &ImageContext,
    ) -> Result<(), EditError> {
        let id = module.definition().id;
        let failed = |message: String| EditError::ViewFailed {
            module: id,
            view: view.id,
            message,
        };
        // A module's code may panic, as its binding is guarded against: the value is
        // refused, never the photo.
        let given = catch_unwind(AssertUnwindSafe(|| {
            module.unview(view.id, &coordinates, image)
        }))
        .map_err(|panic| failed(panic_message(panic.as_ref())))?
        .ok_or_else(|| failed("no values".to_owned()))?;
        let mut wrote = Vec::with_capacity(view.controls.len());
        for &control in view.controls {
            let value = match given.get(control) {
                // Infinite or NaN is the module's fault, never the coordinate's.
                Some(ControlValue::Number(v)) if v.is_finite() => ControlValue::Number(*v),
                _ => return Err(failed(format!("no finite number for {control}"))),
            };
            let (_, found) = find_control(module, control)?;
            found
                .check(&value)
                .map_err(|source| EditError::Coordinate {
                    module: id,
                    coordinate,
                    source,
                })?;
            wrote.push(value);
        }
        let values = self.modules.entry(id.to_owned()).or_default();
        for (control, value) in view.controls.iter().zip(&wrote) {
            values.set(control, value.clone());
            values.set_mark(control, Some(Mark::Kept));
        }
        self.switch_on(module);
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

    /// Takes every control of the module back to its default, with no marks, and turns a
    /// switched module off (decision 32).
    pub fn reset_module(&mut self, module: &dyn Module) {
        self.modules.remove(module.definition().id);
    }

    /// Keeps a control or releases it. Keeping an untouched control holds it at its
    /// default; releasing a set one leaves its value, free for Auto and Match. It never
    /// turns a switched module on, nor off (decision 32).
    pub fn keep(
        &mut self,
        module: &dyn Module,
        control: &str,
        kept: bool,
    ) -> Result<(), EditError> {
        let (id, control) = find_control(module, control)?;
        let values = self.modules.entry(id.to_owned()).or_default();
        values.set_mark(control.id, kept.then_some(Mark::Kept));
        self.tidy(id);
        Ok(())
    }

    /// Sets a parameter no module owns, of an operation an unedited photo runs, refusing
    /// a value out of its range ([`unowned`]).
    pub fn set_param(
        &mut self,
        operations: &[&dyn Operation],
        modules: &[&dyn Module],
        operation: &str,
        param: &str,
        value: Value,
    ) -> Result<(), EditError> {
        let (operation, param) = unowned(operations, modules, operation, param)?;
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
        modules: &[&dyn Module],
        operation: &str,
        param: &str,
    ) -> Result<(), EditError> {
        let (operation, param) = unowned(operations, modules, operation, param)?;
        if let Some(params) = self.params.get_mut(operation) {
            params.remove(param.name);
            if params.iter().next().is_none() {
                self.params.remove(operation);
            }
        }
        Ok(())
    }

    /// Forgets a module the edit no longer touches.
    fn tidy(&mut self, module: &str) {
        if self.modules.get(module).is_some_and(ModuleValues::is_empty) {
            self.modules.remove(module);
        }
    }
}

/// The module called `id`.
pub fn module<'m>(modules: &[&'m dyn Module], id: &str) -> Result<&'m dyn Module, EditError> {
    modules
        .iter()
        .copied()
        .find(|module| module.definition().id == id)
        .ok_or_else(|| EditError::UnknownModule(id.to_owned()))
}

/// The controls a command on `control`, one of the module's controls or a coordinate of one
/// of its views, writes: the control, or the view's controls; none for an id the module
/// lacks, which the command refuses.
pub fn written(module: &dyn Module, control: &str) -> &'static [&'static str] {
    let definition = module.definition();
    match (definition.coordinate(control), definition.control(control)) {
        (Some((view, _)), _) => view.controls,
        (None, Some(found)) => std::slice::from_ref(&found.id),
        (None, None) => &[],
    }
}

/// The options the choice `control` lists on `image` (decision 30): [`Module::options`],
/// or every option where the module's code panics or gives a list that does not hold
/// together ([`check::listed`]), with why. A control that is no choice lists none. The
/// edit refuses any other option, and the session reports the list, warning once of a
/// failure, so both read it alike.
pub fn listed(
    module: &dyn Module,
    control: &Control,
    image: &ImageContext,
) -> (Vec<&'static str>, Option<String>) {
    let ControlKind::Choice { options, .. } = control.kind else {
        return (Vec::new(), None);
    };
    // A module's code may panic, as its binding is guarded against: the photo lists every
    // option, never none.
    let given = catch_unwind(AssertUnwindSafe(|| {
        let listed = module.options(control.id, image);
        let default = module.defaults(image).get(control.id).cloned();
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

/// The range of the number control `control` while the module's controls hold `values`
/// (decision 30), with why it is the definition's, where the module's code panics or
/// gives one that does not hold ([`check::range`]). `None` for a control that is no
/// number. The edit refuses a value outside it and the session reports it, warning once of
/// a failure, so both read it alike.
pub fn range(
    module: &dyn Module,
    control: &str,
    values: &Controls,
) -> Option<(Range, Option<String>)> {
    let definition = module.definition().range(control)?;
    // A module's code may panic, as its binding is guarded against: the range is the
    // definition's, which every value it may hold lies in.
    Some(
        match catch_unwind(AssertUnwindSafe(|| check::range(module, control, values))) {
            Ok(Some(Ok(range))) => (range, None),
            Ok(_) => (definition, Some("its range does not hold".to_owned())),
            Err(panic) => (definition, Some(panic_message(panic.as_ref()))),
        },
    )
}

/// [`range`]'s range alone.
fn range_now(module: &dyn Module, control: &str, values: &Controls) -> Option<Range> {
    range(module, control, values).map(|(range, _)| range)
}

/// The option `value` holds, where `control` is a choice that `image` does not list it
/// for.
fn unlisted(
    module: &dyn Module,
    control: &Control,
    value: &ControlValue,
    image: &ImageContext,
) -> Option<&'static str> {
    let (ControlKind::Choice { options, .. }, ControlValue::Choice(i)) = (control.kind, value)
    else {
        return None;
    };
    let option = options.get(*i as usize).copied()?;
    let (listed, _) = listed(module, control, image);
    (!listed.contains(&option)).then_some(option)
}

fn find_control(
    module: &dyn Module,
    control: &str,
) -> Result<(&'static str, &'static Control), EditError> {
    let definition = module.definition();
    let found = definition
        .control(control)
        .ok_or_else(|| EditError::UnknownControl {
            module: definition.id,
            control: control.to_owned(),
        })?;
    Ok((definition.id, found))
}

/// The coordinate called `coordinate` of one of the module's views: the module's id, the
/// view, the coordinate's place in it, and the coordinate.
fn find_coordinate(
    module: &dyn Module,
    coordinate: &str,
) -> Result<(&'static str, &'static View, usize, &'static Coordinate), EditError> {
    let definition = module.definition();
    let (view, at) =
        definition
            .coordinate(coordinate)
            .ok_or_else(|| EditError::UnknownControl {
                module: definition.id,
                control: coordinate.to_owned(),
            })?;
    Ok((definition.id, view, at, &view.coordinates[at]))
}

/// The coordinates of the module's view `view` at its defaults for `image`: what a
/// coordinate's reset takes it back to.
pub fn default_coordinates(
    module: &dyn Module,
    view: &View,
    image: &ImageContext,
) -> Result<Vec<f32>, EditError> {
    viewed(module, view, image, || module.defaults(image))
}

/// [`Module::view`] of the values `values` gives, one coordinate for each of the view's;
/// the module's failure, a panic included, an error.
fn viewed(
    module: &dyn Module,
    view: &View,
    image: &ImageContext,
    values: impl FnOnce() -> Controls,
) -> Result<Vec<f32>, EditError> {
    let failed = |message: String| EditError::ViewFailed {
        module: module.definition().id,
        view: view.id,
        message,
    };
    let coordinates = catch_unwind(AssertUnwindSafe(|| module.view(view.id, &values(), image)))
        .map_err(|panic| failed(panic_message(panic.as_ref())))?
        .ok_or_else(|| failed("no coordinates".to_owned()))?;
    if coordinates.len() != view.coordinates.len() {
        let (given, wanted) = (coordinates.len(), view.coordinates.len());
        return Err(failed(format!("{given} coordinates for {wanted}")));
    }
    Ok(coordinates)
}

/// The coordinates a session holds through gestures on lossless views, by module and view
/// (decision 2; `docs/engine.md`, Preset rows and lossless views). A view's are taken when
/// a gesture on one of its coordinates begins, from the module's view of the stored
/// values, and last while the view's controls hold exactly what was last written through
/// it, so dragging one coordinate leaves the others exactly where they were however many
/// steps the drag takes. Any change that leaves those controls holding other values drops
/// them, and the next edit starts from the stored values again; a view reads only its own
/// controls ([`Module::view`]), so no other change can move them. Nothing of it is saved.
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
    /// The module's view's held coordinates, while its controls hold what was written.
    fn get(&self, edit: &Edit, module: &'static str, view: &View) -> Option<&[f32]> {
        let hold = self.0.get(&(module, view.id))?;
        hold.holds(edit, module).then_some(&hold.coordinates[..])
    }

    /// Drops the coordinates of every view whose controls no longer hold what was last
    /// written through it: after any change but through the view, its gesture is over.
    pub fn prune(&mut self, edit: &Edit) {
        self.0.retain(|&(module, _), hold| hold.holds(edit, module));
    }
}

impl Hold {
    fn holds(&self, edit: &Edit, module: &str) -> bool {
        let values = edit.modules.get(module);
        self.controls
            .iter()
            .zip(&self.wrote)
            .all(|(control, wrote)| values.and_then(|v| v.get(control)) == Some(wrote))
    }
}

/// `operation.param`, if an edit may set it at operation level: a parameter no module
/// owns, of an operation an unedited photo runs. Any other is refused, so a saved value
/// can never wait unseen on an optional operation and appear once a module adds it.
pub fn unowned(
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
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
    if let Some(owner) = check::owner(modules, manifest.id, param.name) {
        let (operation, param, module) = (manifest.id, param.name, owner.definition().id);
        let fixed = owner
            .fixed()
            .iter()
            .any(|fixed| fixed.operation == operation && fixed.param == param);
        return Err(if fixed {
            EditError::Fixed {
                operation,
                param,
                module,
            }
        } else {
            EditError::Owned {
                operation,
                param,
                module,
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

/// The instances an edit renders, and the modules left out of them.
pub struct Bound<'a> {
    pub instances: Vec<Instance<'a>>,
    /// Each module whose binding set a parameter it does not own or a value its schema
    /// refuses, or panicked: none of its settings are in `instances`.
    pub failed: Vec<(&'static str, ModuleError)>,
}

impl<'a> Bound<'a> {
    /// The instances, or the first module that failed: for an export, which renders the
    /// edit as it is or not at all.
    pub fn all(self) -> Result<Vec<Instance<'a>>, EditError> {
        match self.failed.into_iter().next() {
            Some((module, source)) => Err(EditError::Binding { module, source }),
            None => Ok(self.instances),
        }
    }
}

/// One module's settings for `values`, each checked: what it may set, or why not. A
/// switched module off is never bound; one on is bound with its values off its defaults,
/// possibly none (decision 32).
fn bind(
    module: &dyn Module,
    edit: &Edit,
    operations: &[&dyn Operation],
    image: &ImageContext,
    base: &mut Option<Base>,
) -> Result<Vec<apertrust_modules::Setting>, ModuleError> {
    let Some(values) = edit.modules.get(module.definition().id) else {
        return Ok(Vec::new());
    };
    let switched = module.definition().switched;
    if switched && !values.enabled() {
        return Ok(Vec::new());
    }
    // A module's code may panic, as `Base::float` does on a parameter it lacks: that
    // module is left out, never the photo.
    let settings = catch_unwind(AssertUnwindSafe(|| {
        let values = values.off_defaults(&module.defaults(image));
        if values.is_empty() && !switched {
            return Vec::new();
        }
        let base = base.get_or_insert_with(|| Base::new(operations, image));
        module.bind(&values, image, base)
    }))
    .map_err(|panic| ModuleError::Panicked(panic_message(panic.as_ref())))?;
    for setting in &settings {
        check::setting(module, operations, setting)?;
    }
    Ok(settings)
}

/// The instances `edit` renders on `image` (decision 28): every operation an unedited
/// photo runs, with the edit's parameters no module owns, and every other operation only
/// while a binding sets one of its parameters or a switched module owning one of them is
/// on (decision 32), each with what the bindings set, in the order of `operations`. A
/// module is bound only with its controls off their defaults for the photo, and not at
/// all when it has none, but a switched module on, bound with none, so its operations run
/// at their own defaults; a switched module off is never bound. So an empty edit, or one
/// whose modules are all at their defaults and off, gives exactly the default pipeline.
/// A module whose binding
/// sets a parameter it does not own or a value its schema refuses, or panics, is left
/// out whole and reported in [`Bound::failed`]; the others still render.
pub fn instances<'a>(
    edit: &Edit,
    operations: &[&'a dyn Operation],
    modules: &[&dyn Module],
    image: &ImageContext,
) -> Bound<'a> {
    let mut bound: BTreeMap<&'static str, ParamValues> = BTreeMap::new();
    let mut failed = Vec::new();
    let mut base = None;
    for &module in modules {
        match bind(module, edit, operations, image, &mut base) {
            Ok(settings) => {
                // A switched module on plans each operation it owns a parameter of.
                if module.definition().switched && edit.is_on(module) {
                    for owned in module.owns() {
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
            Err(error) => failed.push((module.definition().id, error)),
        }
    }
    // A module not applied leaves out each operation it owns a parameter of (decision 78).
    let unapplied: BTreeSet<&str> = (modules.iter())
        .filter(|module| edit.unapplied.contains(module.definition().id))
        .flat_map(|module| module.owns().iter().map(|owned| owned.operation))
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
    use apertrust_modules::testing::{self, Defaultless, Pushing, Sample};
    use apertrust_modules::{Controls, Definition, Owned, Setting};
    use apertrust_operations::OPERATIONS;

    use super::*;

    const MODULES: &[&dyn Module] = &[&Sample];

    /// Each instance's operation and the parameters it sets.
    fn planned(edit: &Edit) -> Vec<(&'static str, Vec<(String, Value)>)> {
        instances(edit, OPERATIONS, MODULES, &testing::image())
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
    fn a_module_at_its_defaults_adds_nothing() {
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
    fn module_values_become_the_parameters_their_binding_derives() {
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
        edit.set_param(OPERATIONS, MODULES, "exposure", "black", Value::Float(0.01))
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
        edit.reset_module(&Sample);
        edit.reset_param(OPERATIONS, MODULES, "exposure", "black")
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
            module(MODULES, "exposure").map(|_| ()),
            Err(EditError::UnknownModule(_))
        ));
        assert!(edit.is_empty());
    }

    #[test]
    fn only_unowned_parameters_of_base_operations_are_set_at_operation_level() {
        let mut edit = Edit::default();
        let mut set =
            |operation, param, value| edit.set_param(OPERATIONS, MODULES, operation, param, value);
        assert_eq!(
            set("exposure", "exposure", Value::Float(1.0)),
            Err(EditError::Owned {
                operation: "exposure",
                param: "exposure",
                module: "sample"
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
        // A parameter its module fixes, with no control, is refused as such; one of its
        // controls' as one it sets.
        let modules = apertrust_modules::MODULES;
        assert_eq!(
            edit.set_param(OPERATIONS, modules, "rawprepare", "left", Value::Int(4)),
            Err(EditError::Fixed {
                operation: "rawprepare",
                param: "left",
                module: "rawprepare"
            })
        );
        assert_eq!(
            EditError::Fixed {
                operation: "rawprepare",
                param: "left",
                module: "rawprepare"
            }
            .to_string(),
            "rawprepare.left is fixed by the module rawprepare"
        );
        assert_eq!(
            edit.set_param(
                OPERATIONS,
                modules,
                "rawprepare",
                "raw_white_point",
                Value::Int(16000)
            ),
            Err(EditError::Owned {
                operation: "rawprepare",
                param: "raw_white_point",
                module: "rawprepare"
            })
        );
        assert!(matches!(
            edit.set_param(
                OPERATIONS,
                modules,
                "channelmixerrgb",
                "version",
                Value::Choice(2)
            ),
            Err(EditError::Fixed { .. })
        ));
        // Without modules nothing is owned.
        assert_eq!(
            edit.set_param(OPERATIONS, &[], "exposure", "exposure", Value::Float(1.0)),
            Ok(())
        );
    }

    #[test]
    fn a_misbehaving_binding_leaves_out_only_its_own_module() {
        use apertrust_modules::testing::Faulty;
        let modules: &[&dyn Module] = &[&Faulty, &Sample];
        let mut edit = Edit::default();
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.5),
            &testing::image(),
        )
        .unwrap();
        let derived = |edit: &Edit| {
            let bound = instances(edit, OPERATIONS, modules, &testing::image());
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
                ModuleError::NotOwned {
                    operation: "exposure",
                    param: "black",
                },
            ),
            (
                "panic",
                ModuleError::Panicked(
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
            // Sample's brightness still renders, and nothing of the faulty module does.
            assert_eq!(exposure.get("exposure"), Some(Value::Float(0.7 + 0.5)));
            assert_eq!(exposure.get("black"), None);
            assert!(
                instances(&edit, OPERATIONS, modules, &testing::image())
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
        let values = &edit.modules["sample"];
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
        let values = &edit.modules["sample"];
        assert_eq!(
            (values.number("grain"), values.mark("grain")),
            (Some(10.0), None)
        );
    }

    /// [`Sample`] with a look its module gets wrong, as a bug may: `grainy` gives a
    /// brightness in range, then a grain past its own, and `untouched` gives no size.
    struct Wrong;

    impl Module for Wrong {
        fn definition(&self) -> &'static Definition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
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
    fn an_option_its_module_gets_wrong_changes_nothing_and_is_the_module_s_fault() {
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
                module,
                preset,
                option: named,
                ..
            } = &error
            else {
                panic!("{error:?}");
            };
            assert_eq!((*module, *preset, *named), ("sample", "look", option));
            assert_eq!(error.to_string(), says);
            // Not even the brightness that held: all or nothing.
            assert_eq!(edit, before, "{option}");
        }
    }

    /// [`Sample`] with a pair whose inverse overflows, as a bug may make it: grain comes
    /// back infinite, bias as Sample gives it.
    struct Overflowing;

    impl Module for Overflowing {
        fn definition(&self) -> &'static Definition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
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
        module: &dyn Module,
        control: &str,
        value: ControlValue,
    ) -> Result<(), EditError> {
        let before = edit.clone();
        edit.set_control(module, control, value, &testing::image())?;
        edit.follow(&before, module, written(module, control), &testing::image())
    }

    #[test]
    fn a_change_s_follow_writes_are_written_with_it_each_kept() {
        use apertrust_modules::testing::{Follow, Following};
        let module = &Following(Follow::Size);
        let mut edit = Edit::default();
        set_following(&mut edit, module, "grain", ControlValue::Number(60.0)).unwrap();
        let values = &edit.modules["sample"];
        assert_eq!(
            (values.choice("size"), values.mark("size")),
            (Some(2), Some(Mark::Kept))
        );
        // Released, then a grain that writes the size it holds: nothing changes, so the
        // size stays released.
        edit.keep(module, "size", false).unwrap();
        set_following(&mut edit, module, "grain", ControlValue::Number(70.0)).unwrap();
        assert_eq!(edit.modules["sample"].mark("size"), None);
        // A grain set to the value it holds moves nothing, so writes nothing.
        edit.set_control(module, "size", ControlValue::Choice(1), &testing::image())
            .unwrap();
        set_following(&mut edit, module, "grain", ControlValue::Number(70.0)).unwrap();
        assert_eq!(edit.modules["sample"].choice("size"), Some(1));
        // A reset is a change to the default, with its follow writes: a fine grain.
        let before = edit.clone();
        edit.reset_control(module, "grain").unwrap();
        edit.follow(&before, module, written(module, "grain"), &testing::image())
            .unwrap();
        let values = &edit.modules["sample"];
        assert_eq!(
            (values.get("grain"), values.choice("size")),
            (None, Some(0))
        );
        // What a command writes: a control, or the controls of the view of a coordinate.
        assert_eq!(written(module, "grain"), ["grain"]);
        assert_eq!(written(module, "sum"), ["grain", "bias"]);
        assert!(written(module, "contrast").is_empty());
    }

    #[test]
    fn a_follow_write_its_module_gets_wrong_is_refused_and_changes_nothing() {
        use apertrust_modules::testing::{Follow, Following};
        for (follow, says) in [
            (Follow::OutOfRange, "bias is 150, outside 0 to 100"),
            (Follow::Unknown, "writes contrast, which it lacks"),
            (Follow::Itself, "rewrites grain, which the change set"),
            (Follow::Panics, "no follow writes for grain"),
        ] {
            let module = &Following(follow);
            let mut edit = Edit::default();
            edit.set_control(
                module,
                "brightness",
                ControlValue::Number(1.0),
                &testing::image(),
            )
            .unwrap();
            let before = edit.clone();
            let error =
                set_following(&mut edit, module, "grain", ControlValue::Number(60.0)).unwrap_err();
            assert!(
                matches!(
                    &error,
                    EditError::FollowFailed {
                        module: "sample",
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
        module: &dyn Module,
        control: &str,
        value: f32,
    ) -> Result<(), EditError> {
        let before = edit.clone();
        set_following(edit, module, control, ControlValue::Number(value))?;
        edit.clamp(&before, &[module], &testing::image())
    }

    /// Sample's coarseness reaches up to 100 % plus the bias (decision 30): a value outside
    /// the range the module's values give now is refused; a bias that leaves it outside
    /// clamps it in the same change, kept, and one that clamps it to its default resets it.
    #[test]
    fn a_number_outside_its_moved_range_is_refused_and_one_a_change_leaves_outside_clamped() {
        let mut edit = Edit::default();
        let coarseness = |edit: &Edit| {
            let values = &edit.modules["sample"];
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
                module: "sample",
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
    fn a_clamp_of_a_value_the_change_wrote_is_the_module_s_failure_and_changes_nothing() {
        let mut edit = Edit::default();
        set_clamping(&mut edit, &Pushing, "grain", 10.0).unwrap();
        let before = edit.clone();
        let error = set_clamping(&mut edit, &Pushing, "bias", 20.0).unwrap_err();
        assert_eq!(
            error,
            EditError::ClampFailed {
                module: "sample",
                control: "coarseness"
            }
        );
        assert_eq!(edit, before);
        // A bias whose range holds the coarseness it writes is a change like any other.
        set_clamping(&mut edit, &Pushing, "bias", 95.0).unwrap();
        assert_eq!(edit.modules["sample"].number("coarseness"), Some(190.0));
    }

    /// A saved edit holding a number outside the range its module's values give, as only a
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
        let warnings = edit.clamp_read(MODULES, &image);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].starts_with("sample.coarseness is 150"),
            "{warnings:?}"
        );
        let values = &edit.modules["sample"];
        assert_eq!(
            (values.number("coarseness"), values.mark("coarseness")),
            (Some(120.0), None)
        );
        assert!(edit.clamp_read(MODULES, &image).is_empty());
    }

    /// A module whose defaults for the photo panic is passed over where every module's
    /// ranges are asked, never the change or the photo (decision 30): its own number is
    /// held to its definition's range, another module's change clamps as it would without
    /// it, and so does a saved edit as it is read, the faulty module's value as it was.
    #[test]
    fn a_module_whose_defaults_panic_leaves_the_others_ranges_to_hold() {
        let image = testing::image();
        let modules: &[&dyn Module] = &[&Defaultless, &Sample];
        let number = ControlValue::Number;
        let mut edit = Edit::default();
        edit.set_control(&Defaultless, "amount", number(5.0), &image)
            .unwrap();
        let outside = edit.set_control(&Defaultless, "amount", number(50.0), &image);
        assert!(
            matches!(
                outside,
                Err(EditError::Control {
                    module: "defaultless",
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
        edit.clamp(&before, modules, &image).unwrap();
        assert_eq!(edit.modules["sample"].number("coarseness"), Some(120.0));

        // Saved in an order no change can reach, as a hand-written file may hold it.
        let mut saved = before;
        saved
            .set_control(&Sample, "bias", number(20.0), &image)
            .unwrap();
        let warnings = saved.clamp_read(modules, &image);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(saved.modules["sample"].number("coarseness"), Some(120.0));
        assert_eq!(saved.modules["defaultless"].number("amount"), Some(5.0));
    }

    #[test]
    fn an_infinite_value_from_a_view_s_inverse_is_the_module_s_fault_and_changes_nothing() {
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
                    module: "sample",
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
    /// a module's lists may follow the photo (decision 30), and its size's list panicking
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

    impl Module for Narrow {
        fn definition(&self) -> &'static Definition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
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
                module: "sample",
                control: "method",
                option: "camera",
            })
        );
        assert!(edit.is_empty());
        edit.set_control(&Sample, "method", camera.clone(), &image)
            .unwrap();
        assert_eq!(edit.module("sample").unwrap().get("method"), Some(&camera));
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
    fn a_preset_or_a_follow_write_of_an_option_the_photo_does_not_list_is_the_module_s_failure() {
        let plain = testing::image();
        let module = Narrow(Fault::None);
        let mut edit = Edit::default();
        let error = edit
            .apply_preset(&module, "look", "grainy", &plain)
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
        edit.apply_preset(&module, "look", "grainy", &biased(-0.7))
            .unwrap();
        // Grain above 50 % writes size coarse, which this photo does not list.
        let mut edit = Edit::default();
        let before = edit.clone();
        edit.set_control(&module, "grain", ControlValue::Number(60.0), &plain)
            .unwrap();
        let error = edit
            .follow(&before, &module, &["grain"], &plain)
            .unwrap_err();
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
        edit.set_control(&module, "size", ControlValue::Choice(2), &biased(-0.7))
            .unwrap();
        let before = edit.clone();
        edit.set_control(&module, "grain", ControlValue::Number(60.0), &plain)
            .unwrap();
        edit.follow(&before, &module, &["grain"], &plain).unwrap();
        let size = edit.module("sample").unwrap().get("size");
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

    /// Each instance's operation planned for `edit` with Grain, a switched module, alone.
    fn planned_with_grain(edit: &Edit) -> Vec<(&'static str, Vec<(String, Value)>)> {
        let modules: &[&dyn Module] = &[&apertrust_modules::grain::Grain];
        instances(edit, OPERATIONS, modules, &testing::image())
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

    /// A switched module (decision 32): off until a control is written, whatever the
    /// value, and then on, its operation planned at its own defaults for every control not
    /// set; a control reset or kept leaves it on, and its reset turns it off.
    #[test]
    fn a_switched_module_is_on_from_a_control_s_write_until_its_reset() {
        use apertrust_modules::grain::Grain;
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
        // The module's reset turns it off, as no edit.
        edit.reset_module(&Grain);
        assert!(!edit.is_on(&Grain));
        assert_eq!(edit, Edit::default());
        assert_eq!(planned_with_grain(&edit), none);
    }

    /// A module that is not switched is always on, and a write leaves no `enabled` in its
    /// values.
    #[test]
    fn a_module_not_switched_is_always_on() {
        let mut edit = Edit::default();
        assert!(edit.is_on(&Sample));
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.5),
            &testing::image(),
        )
        .unwrap();
        assert!(!edit.module("sample").unwrap().enabled());
    }
}
