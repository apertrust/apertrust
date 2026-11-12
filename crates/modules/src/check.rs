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

//! What the engine holds every module to (decisions 4 and 28): a definition that holds
//! together, one owner per operation parameter, parameters that exist, pickers that read
//! operations the module drives as they can (decision 79), and a binding that sets only
//! what its module owns, within the schema. The session accepts modules with [`accept`]
//! and leaves out the rest; the tests sweep every built-in module's binding with
//! [`sweep`].

use apertrust_plugin::{ImageContext, Operation, ParamError, Stage, Value};

use crate::definition::{ControlKind, DefinitionError, MAX_POINTS, Space};
use crate::{Base, Control, ControlValue, Controls, Module, ModuleValues, Owned, Range, Setting};

/// A number control's range while the module's controls hold `values`, every control,
/// as the session reports it (decision 30): the module's [`Module::range`] where it
/// holds inside the definition's ([`Range::holds_inside`]), else `Err` with the
/// definition's, where the module gives none or one that does not hold. `None` for a
/// control that is no number. A module's code may panic, which a caller that must go on,
/// as the session, guards against.
pub fn range(
    module: &dyn Module,
    control: &str,
    values: &Controls,
) -> Option<Result<Range, Range>> {
    let definition = module.definition().range(control)?;
    Some(match module.range(control, values) {
        Some(range) if range.holds_inside(&definition) => Ok(range),
        _ => Err(definition),
    })
}

/// The writes that bring every number control of `module` back inside its range while
/// the controls hold `values`, every control, the edit's over the defaults (decision
/// 30): each number outside its [`range`], the definition's where the module's does not
/// hold, written at the range's nearer end, from `values` as they are, in one pass. A
/// control not ported takes only its default, which its range always holds, and is left
/// alone. Empty while every number is inside. The edit writes them in the same change,
/// and the app's pending display shows them, so both clamp alike.
pub fn clamp(module: &dyn Module, values: &Controls) -> Controls {
    let mut writes = Controls::new();
    for control in module.definition().controls {
        let (Some(ControlValue::Number(value)), false) = (values.get(control.id), control.unported)
        else {
            continue;
        };
        let Some(range) = range(module, control.id, values).map(|r| r.unwrap_or_else(|d| d)) else {
            continue;
        };
        if !range.contains(*value) {
            writes.set(control.id, ControlValue::Number(range.clamp(*value)));
        }
    }
    writes
}

/// What is wrong with `listed` as the options the choice `control` lists on a photo whose
/// default for it is `default` (decision 30): no option, one the control lacks, one twice
/// or out of the definition's order, or a list without the default. `None` when it holds,
/// and for a control that is no choice, which lists nothing.
pub fn listed(control: &Control, listed: &[&str], default: &ControlValue) -> Option<String> {
    let ControlKind::Choice { options, .. } = control.kind else {
        return None;
    };
    let id = control.id;
    let mut at = Vec::with_capacity(listed.len());
    for option in listed {
        match options.iter().position(|o| o == option) {
            Some(i) => at.push(i),
            None => return Some(format!("{id} lists {option}, which it lacks")),
        }
    }
    if at.is_empty() {
        return Some(format!("{id} lists no option"));
    }
    if !at.windows(2).all(|pair| pair[0] < pair[1]) {
        return Some(format!("{id} lists an option twice or out of order"));
    }
    match default {
        ControlValue::Choice(i) if at.contains(&(*i as usize)) => None,
        _ => Some(format!("{id} lists no option for its default")),
    }
}

/// Whether a photo that lists `listed` for the choice `control` lists only the option
/// `value` holds, so the choice can change nothing there and is unavailable (decision
/// 30), as a monochrome sensor's one demosaic method. False for a control that is no
/// choice.
pub fn lists_only(control: &Control, listed: &[&str], value: &ControlValue) -> bool {
    match (control.kind, value) {
        (ControlKind::Choice { options, .. }, ControlValue::Choice(i)) => {
            listed.len() == 1 && options.get(*i as usize) == listed.first()
        }
        _ => false,
    }
}

/// Why a module is left out, or a setting refused.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ModuleError {
    #[error(transparent)]
    Definition(#[from] DefinitionError),
    #[error("another module is called {0}")]
    Duplicate(&'static str),
    #[error("there is no operation {0}")]
    UnknownOperation(&'static str),
    #[error("{operation} has no parameter {param}")]
    UnknownParam {
        operation: &'static str,
        param: &'static str,
    },
    #[error("{operation}.{param} is already owned by {owner}")]
    OwnedTwice {
        operation: &'static str,
        param: &'static str,
        owner: &'static str,
    },
    #[error("its binding sets {operation}.{param}, which it does not own")]
    NotOwned {
        operation: &'static str,
        param: &'static str,
    },
    #[error("its binding sets {operation}: {source}")]
    Param {
        operation: &'static str,
        source: ParamError,
    },
    /// A bug in the module: its defaults or its binding panicked.
    #[error("its binding failed: {0}")]
    Panicked(String),
    /// A switched module owning a parameter of an operation every photo runs, which it
    /// could not leave untouched while off (decision 32).
    #[error("it is switched but owns {operation}.{param}, of an operation every photo runs")]
    SwitchedOverBase {
        operation: &'static str,
        param: &'static str,
    },
    /// A picker reading an operation the module owns no parameter of (decision 79).
    #[error("its picker {picker} reads {operation}, which it owns no parameter of")]
    PickerOperation {
        picker: &'static str,
        operation: &'static str,
    },
    /// A picker reading a mosaic other than in its operation's space or denoised, which
    /// darktable's 4-channel conversions and B-spline cannot (decision 79).
    #[error("its picker {0} reads a mosaic, which it can only in its own space and not denoised")]
    PickerMosaic(&'static str),
}

/// A module left out, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Rejected {
    pub module: &'static str,
    pub error: ModuleError,
}

/// The modules that hold, in order, and the others with what is wrong with them. A
/// parameter belongs to the first module that owns it; a later one that owns it too is
/// left out whole, so a faulty module never takes a parameter.
pub fn accept<'m>(
    modules: &[&'m dyn Module],
    operations: &[&dyn Operation],
) -> (Vec<&'m dyn Module>, Vec<Rejected>) {
    let mut accepted: Vec<&'m dyn Module> = Vec::new();
    let mut rejected = Vec::new();
    for &module in modules {
        match admit(module, &accepted, operations) {
            Ok(()) => accepted.push(module),
            Err(error) => rejected.push(Rejected {
                module: module.definition().id,
                error,
            }),
        }
    }
    (accepted, rejected)
}

fn admit(
    module: &dyn Module,
    accepted: &[&dyn Module],
    operations: &[&dyn Operation],
) -> Result<(), ModuleError> {
    let definition = module.definition();
    definition.check()?;
    if accepted
        .iter()
        .any(|other| other.definition().id == definition.id)
    {
        return Err(ModuleError::Duplicate(definition.id));
    }
    let owns = module.owns();
    for (i, &Owned { operation, param }) in owns.iter().enumerate() {
        let found = operations
            .iter()
            .find(|candidate| candidate.manifest().id == operation)
            .ok_or(ModuleError::UnknownOperation(operation))?;
        if found.schema().param(param).is_none() {
            return Err(ModuleError::UnknownParam { operation, param });
        }
        if definition.switched && found.manifest().default_enabled {
            return Err(ModuleError::SwitchedOverBase { operation, param });
        }
        let owner = if owns[..i].contains(&owns[i]) {
            Some(definition.id)
        } else {
            owner(accepted, operation, param).map(|other| other.definition().id)
        };
        if let Some(owner) = owner {
            return Err(ModuleError::OwnedTwice {
                operation,
                param,
                owner,
            });
        }
    }
    for picker in definition.pickers {
        let operation = picker.operation;
        let picked = || ModuleError::PickerOperation {
            picker: picker.id,
            operation,
        };
        if !owns.iter().any(|owned| owned.operation == operation) {
            return Err(picked());
        }
        let found = operations
            .iter()
            .find(|candidate| candidate.manifest().id == operation)
            .ok_or_else(picked)?;
        let mosaic = found.manifest().slot.stage == Stage::Raw;
        if mosaic && (picker.space != Space::Operation || picker.denoise) {
            return Err(ModuleError::PickerMosaic(picker.id));
        }
    }
    Ok(())
}

/// The module among `modules` that owns `operation.param`.
pub fn owner<'m>(
    modules: &[&'m dyn Module],
    operation: &str,
    param: &str,
) -> Option<&'m dyn Module> {
    modules.iter().copied().find(|module| {
        module
            .owns()
            .iter()
            .any(|owned| owned.operation == operation && owned.param == param)
    })
}

/// Whether `module` may set what `setting` sets: a parameter it owns, of one of
/// `operations`, with a value its schema takes.
pub fn setting(
    module: &dyn Module,
    operations: &[&dyn Operation],
    setting: &Setting,
) -> Result<(), ModuleError> {
    let &Setting {
        operation, param, ..
    } = setting;
    if !module.owns().contains(&Owned { operation, param }) {
        return Err(ModuleError::NotOwned { operation, param });
    }
    let schema = operations
        .iter()
        .find(|candidate| candidate.manifest().id == operation)
        .ok_or(ModuleError::UnknownOperation(operation))?
        .schema();
    schema
        .param(param)
        .ok_or(ModuleError::UnknownParam { operation, param })?
        .check(setting.value)
        .map_err(|source| ModuleError::Param { operation, source })
}

/// The parameters whose default a switched module's control, made explicit, binds one
/// f32 step away, as a percentage darktable's slider shows divided by 100 in f32 does:
/// Colour balance RGB's two fulcrums at 18.45 %, which bind 0.18450001 for darktable's
/// 0.1845 (its row B6). The render is exact all the same: a module on at its defaults
/// binds no value, so its operations run at their own defaults.
pub const ONE_STEP_FROM_DEFAULT: [(&str, &str); 2] = [
    ("colorbalancergb", "grey_fulcrum"),
    ("colorbalancergb", "mask_grey_fulcrum"),
];

/// What is wrong with a switched module's defaults on `image` (decision 32): each
/// control's default for the photo, made explicit, must bind its operations' own defaults
/// for the photo exactly, so the module on and untouched is darktable's new instance, but
/// those of [`ONE_STEP_FROM_DEFAULT`], within one f32 step. Empty when they hold, and for a
/// module that is not switched.
pub fn switched_defaults(
    module: &dyn Module,
    operations: &[&dyn Operation],
    image: &ImageContext,
) -> Vec<String> {
    if !module.definition().switched {
        return Vec::new();
    }
    let base = Base::new(operations, image);
    let mut values = ModuleValues::new();
    for (control, value) in module.defaults(image).iter() {
        values.set(control, value.clone());
    }
    values.set_enabled(true);
    module
        .bind(&values, image, &base)
        .into_iter()
        .filter(|setting| {
            let default = base.get(setting.operation, setting.param);
            let one_step = ONE_STEP_FROM_DEFAULT.contains(&(setting.operation, setting.param));
            match (default, setting.value) {
                (Some(Value::Float(default)), Value::Float(bound)) if one_step => {
                    default.to_bits().abs_diff(bound.to_bits()) > 1
                        || default.is_sign_negative() != bound.is_sign_negative()
                }
                _ => default != Some(setting.value),
            }
        })
        .map(|setting| {
            format!(
                "its defaults bind {}.{} to {}, where the operation's default is {:?}",
                setting.operation,
                setting.param,
                setting.value,
                base.get(setting.operation, setting.param),
            )
        })
        .collect()
}

/// What [`sweep`] tries a control at, from one end of its range to the other: every
/// option of a choice but those not ported, and only its default for a control not
/// ported, since no command or edit can set anything else.
fn sweep_values(control: &Control) -> Vec<ControlValue> {
    if control.unported {
        return vec![control.default()];
    }
    match control.kind {
        ControlKind::Number { min, max, .. } => (0..=16)
            .map(|i| ControlValue::Number(min + (max - min) * i as f32 / 16.0))
            .chain([ControlValue::Number(max)])
            .collect(),
        ControlKind::Choice {
            options, unported, ..
        } => (0..options.len() as u32)
            .filter(|&i| !unported.contains(&options[i as usize]))
            .map(ControlValue::Choice)
            .collect(),
        ControlKind::Bool { .. } => vec![ControlValue::Bool(false), ControlValue::Bool(true)],
        ControlKind::Points { default } => vec![
            ControlValue::Points(default.to_vec()),
            ControlValue::Points(vec![[0.5, 0.75]]),
            ControlValue::Points(vec![[0.0025, 0.1], [0.997, 0.9]]),
            ControlValue::Points(
                (1..=MAX_POINTS)
                    .map(|k| [k as f32 / 19.0, 1.0 - k as f32 / 19.0])
                    .collect(),
            ),
        ],
    }
}

/// Sweeps `module`'s binding over `image`: at its defaults it must set nothing; with
/// each control alone across its range, and with every control at one end of its range
/// together, it must set only parameters it owns, each within its schema. With each
/// control alone across its range, its follow writes ([`Module::follow`]) must name other
/// controls of the module, each within its range. Over the same states each number's
/// range must hold (decision 30's rules). Returns what fails, empty when the binding
/// holds.
pub fn sweep(
    module: &dyn Module,
    operations: &[&dyn Operation],
    image: &ImageContext,
) -> Vec<String> {
    let definition = module.definition();
    let defaults = module.defaults(image);
    let base = Base::new(operations, image);
    let mut problems = Vec::new();
    if !module.bind(&ModuleValues::new(), image, &base).is_empty() {
        problems.push("at its defaults it sets parameters".to_owned());
    }
    let mut tries: Vec<(String, ModuleValues)> = Vec::new();
    for control in definition.controls {
        for value in sweep_values(control) {
            let mut values = ModuleValues::new();
            let what = format!("{} {}", control.id, control.text(&value));
            let mut every = defaults.clone();
            every.set(control.id, value.clone());
            problems.extend(follow_problems(module, control.id, &every, image, &what));
            values.set(control.id, value);
            tries.push((what, values));
        }
    }
    for (what, end) in [
        ("every control at its start", 0),
        ("every control at its end", 1),
    ] {
        let mut values = ModuleValues::new();
        for control in definition.controls {
            let sweep = sweep_values(control);
            let value = if end == 0 {
                &sweep[0]
            } else {
                &sweep[sweep.len() - 1]
            };
            values.set(control.id, value.clone());
        }
        tries.push((what.to_owned(), values));
    }
    let states: Vec<(String, Controls)> =
        std::iter::once(("its defaults".to_owned(), defaults.clone()))
            .chain(tries.iter().map(|(what, values)| {
                let mut every = defaults.clone();
                for (control, value) in values.values() {
                    if let Some(found) = definition.control(control) {
                        every.set(found.id, value.clone());
                    }
                }
                (what.clone(), every)
            }))
            .collect();
    problems.extend(range_problems(module, image, &states));
    for (what, values) in tries {
        let values = values.off_defaults(&defaults);
        if values.is_empty() {
            continue;
        }
        for set in module.bind(&values, image, &base) {
            if let Err(error) = setting(module, operations, &set) {
                problems.push(format!("{}, {what}: {error}", definition.id));
            }
        }
    }
    problems
}

/// What is wrong with `module`'s ranges (decision 30) over `states`, the first its
/// defaults on `image`, each every control's value: at the defaults each number's track
/// and step must be the definition's; in every state its range must hold inside the
/// definition's, hold its default, stay where it is whatever its own value, and stay the
/// definition's for a control a lossless view covers; one [`clamp`] pass must bring every
/// number inside; and no preset option written over the defaults may need one.
fn range_problems(
    module: &dyn Module,
    image: &ImageContext,
    states: &[(String, Controls)],
) -> Vec<String> {
    let definition = module.definition();
    let id = definition.id;
    let mut problems = Vec::new();
    let numbers: Vec<&Control> = definition
        .controls
        .iter()
        .filter(|c| matches!(c.kind, ControlKind::Number { .. }))
        .collect();
    let viewed = |control: &str| {
        definition
            .views
            .iter()
            .any(|view| view.controls.contains(&control))
    };
    let Some((_, defaults)) = states.first() else {
        return problems;
    };
    for control in &numbers {
        let (Some(ours), Some(theirs)) = (
            definition.range(control.id),
            module.range(control.id, defaults),
        ) else {
            continue;
        };
        if ours.soft != theirs.soft || ours.step != theirs.step {
            let c = control.id;
            problems.push(format!(
                "{id}.{c}: at its defaults its track and step are not the definition's"
            ));
        }
    }
    for (what, every) in states {
        for control in &numbers {
            let c = control.id;
            let Some(ours) = definition.range(c) else {
                continue;
            };
            let Some(range) = module.range(c, every) else {
                problems.push(format!("{id}.{c}, {what}: gives no range"));
                continue;
            };
            if !range.holds_inside(&ours) {
                problems.push(format!(
                    "{id}.{c}, {what}: its range does not hold inside the definition's"
                ));
            }
            let mut own = every.clone();
            if let Some(default) = defaults.get(c) {
                if let ControlValue::Number(default) = default
                    && !range.contains(*default)
                {
                    problems.push(format!(
                        "{id}.{c}, {what}: its default {default} is outside"
                    ));
                }
                own.set(control.id, default.clone());
            }
            if module.range(c, &own) != Some(range) {
                problems.push(format!(
                    "{id}.{c}, {what}: its range moves with its own value"
                ));
            }
            if viewed(c) && range != ours {
                problems.push(format!(
                    "{id}.{c}, {what}: a view covers it and its range moves"
                ));
            }
        }
        let mut clamped = every.clone();
        for (control, value) in clamp(module, every).iter() {
            clamped.set(control, value.clone());
        }
        if !clamp(module, &clamped).is_empty() {
            problems.push(format!("{id}, {what}: one clamp pass is not enough"));
        }
    }
    for row in definition.presets {
        for &option in row.options {
            let Some(written) = module.preset(row.id, option, image) else {
                continue;
            };
            let mut every = defaults.clone();
            for (control, value) in written.iter() {
                every.set(control, value.clone());
            }
            if !clamp(module, &every).is_empty() {
                problems.push(format!("{id}.{}: {option} needs a clamp", row.id));
            }
        }
    }
    problems
}

/// What is wrong with the follow writes of `control` taking the value `values` holds: a
/// write to the control itself, to a control the module lacks, or of a value its control
/// cannot take.
fn follow_problems(
    module: &dyn Module,
    control: &str,
    values: &crate::Controls,
    image: &ImageContext,
    what: &str,
) -> Vec<String> {
    let definition = module.definition();
    let mut problems = Vec::new();
    for (id, value) in module.follow(control, values, image).iter() {
        let problem = match definition.control(id) {
            _ if id == control => format!("rewrites {id}"),
            None => format!("writes {id}, which it lacks"),
            Some(found) => match found.check(value) {
                Ok(()) => continue,
                Err(error) => error.to_string(),
            },
        };
        problems.push(format!("{}, following {what}: {problem}", definition.id));
    }
    problems
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_plugin::Value;

    use super::*;
    use crate::definition::{Definition, Picker, PickerKind, Quad, Row, Slider, Tab};
    use crate::testing::{self, Sample};

    /// A module with one control, owning `owns`, whose binding sets `sets` whenever the
    /// control is off its default.
    struct Fake {
        definition: &'static Definition,
        owns: &'static [Owned],
        sets: Vec<Setting>,
    }

    impl Module for Fake {
        fn definition(&self) -> &'static Definition {
            self.definition
        }

        fn owns(&self) -> &'static [Owned] {
            self.owns
        }

        fn bind(&self, values: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
            if values.is_empty() {
                Vec::new()
            } else {
                self.sets.clone()
            }
        }
    }

    fn fake(id: &'static str, owns: &[Owned], sets: Vec<Setting>) -> Fake {
        static CONTROLS: &[Control] = &[Control {
            id: "amount",
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        }];
        static TABS: &[Tab] = &[Tab {
            id: None,
            rows: &[Row::Slider(Slider::new("amount"))],
        }];
        Fake {
            definition: Box::leak(Box::new(Definition {
                id,
                version: 1,
                layers: true,
                auto: false,
                switched: false,
                controls: CONTROLS,
                tabs: TABS,
                before: &[],
                after: &[],
                presets: &[],
                views: &[],
                pickers: &[],
            })),
            owns: Box::leak(owns.into()),
            sets,
        }
    }

    #[test]
    fn a_sweep_never_tries_an_option_not_ported() {
        let method = Sample.definition().control("method").unwrap();
        // Even, biased and camera, whichever photo lists it (decision 30); film, which
        // darktable lists, is not ported.
        assert_eq!(
            sweep_values(method),
            [
                ControlValue::Choice(0),
                ControlValue::Choice(1),
                ControlValue::Choice(3)
            ]
        );
        let size = Sample.definition().control("size").unwrap();
        assert_eq!(sweep_values(size).len(), 3);
    }

    #[test]
    fn the_sample_module_holds_against_the_built_in_operations() {
        let (accepted, rejected) = accept(&[&Sample], OPERATIONS);
        assert_eq!(accepted.len(), 1);
        assert!(rejected.is_empty(), "{rejected:?}");
        let problems = sweep(&Sample, OPERATIONS, &testing::image());
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_parameter_has_one_owner_and_must_exist() {
        let second = fake("second", &[Owned::new("exposure", "exposure")], Vec::new());
        let unknown = fake("unknown", &[Owned::new("lens", "distortion")], Vec::new());
        let no_param = fake("no_param", &[Owned::new("exposure", "gamma")], Vec::new());
        let twice = fake(
            "twice",
            &[
                Owned::new("vignette", "unbound"),
                Owned::new("vignette", "unbound"),
            ],
            Vec::new(),
        );
        let (accepted, rejected) = accept(
            &[&Sample, &second, &unknown, &no_param, &twice, &Sample],
            OPERATIONS,
        );
        assert_eq!(accepted.len(), 1);
        let errors: Vec<_> = rejected.iter().map(|r| (r.module, &r.error)).collect();
        assert_eq!(
            errors,
            [
                (
                    "second",
                    &ModuleError::OwnedTwice {
                        operation: "exposure",
                        param: "exposure",
                        owner: "sample"
                    }
                ),
                ("unknown", &ModuleError::UnknownOperation("lens")),
                (
                    "no_param",
                    &ModuleError::UnknownParam {
                        operation: "exposure",
                        param: "gamma"
                    }
                ),
                (
                    "twice",
                    &ModuleError::OwnedTwice {
                        operation: "vignette",
                        param: "unbound",
                        owner: "twice"
                    }
                ),
                ("sample", &ModuleError::Duplicate("sample")),
            ]
        );
    }

    #[test]
    fn a_picker_reads_an_operation_its_module_drives_a_mosaic_as_it_is() {
        // A module owning `owns` with one picker on its slider's quad.
        let picking = |owns: &[Owned], picker: Picker| {
            let mut module = fake("picking", owns, Vec::new());
            module.definition = Box::leak(Box::new(Definition {
                tabs: Box::leak(Box::new([Tab {
                    id: None,
                    rows: Box::leak(Box::new([Row::Slider(
                        Slider::new("amount").quad(Quad::Picker(picker.id)),
                    )])),
                }])),
                pickers: Box::leak(Box::new([picker])),
                ..*module.definition
            }));
            let (_, rejected) = accept(&[&module], OPERATIONS);
            rejected.into_iter().map(|r| r.error).collect::<Vec<_>>()
        };
        let exposure = [Owned::new("exposure", "exposure")];
        let pick = Picker {
            id: "pick",
            operation: "exposure",
            kind: PickerKind::PointArea,
            denoise: true,
            output: true,
            space: Space::Hsl,
            keep: false,
            writes: &["amount"],
            tools: &[],
        };
        assert_eq!(picking(&exposure, pick), []);
        let elsewhere = Picker {
            operation: "grain",
            ..pick
        };
        assert_eq!(
            picking(&exposure, elsewhere),
            [ModuleError::PickerOperation {
                picker: "pick",
                operation: "grain"
            }]
        );
        // White balance's: the mosaic, per CFA colour, as it is.
        let temperature = [Owned::new("temperature", "red")];
        let spot = Picker {
            operation: "temperature",
            denoise: false,
            space: Space::Operation,
            ..pick
        };
        assert_eq!(picking(&temperature, spot), []);
        for wrong in [
            Picker {
                denoise: true,
                ..spot
            },
            Picker {
                space: Space::Lch,
                ..spot
            },
        ] {
            assert_eq!(
                picking(&temperature, wrong),
                [ModuleError::PickerMosaic("pick")]
            );
        }
    }

    #[test]
    fn rows_following_a_control_that_is_not_a_choice_or_a_toggle_are_refused() {
        const AMOUNT: &[Row] = &[Row::Slider(Slider::new("amount"))];
        let following = |id: &'static str, control: &'static str| {
            let mut module = fake(id, &[], Vec::new());
            let rows: &'static [Row] = Box::leak(Box::new([Row::When {
                control,
                is: &["true"],
                rows: AMOUNT,
            }]));
            module.definition = Box::leak(Box::new(Definition {
                tabs: Box::leak(Box::new([Tab { id: None, rows }])),
                ..*module.definition
            }));
            module
        };
        let (missing, number) = (following("missing", "gone"), following("number", "amount"));
        let (accepted, rejected) = accept(&[&Sample, &missing, &number], OPERATIONS);
        assert_eq!(accepted.len(), 1);
        let errors: Vec<_> = rejected.iter().map(|r| (r.module, &r.error)).collect();
        assert_eq!(
            errors,
            [
                (
                    "missing",
                    &ModuleError::Definition(DefinitionError::Condition("gone"))
                ),
                (
                    "number",
                    &ModuleError::Definition(DefinitionError::Condition("amount"))
                ),
            ]
        );
    }

    #[test]
    fn a_sweep_finds_follow_writes_that_rewrite_their_control_or_name_none_in_range() {
        use crate::testing::{Follow, Following};
        let image = testing::image();
        assert!(sweep(&Following(Follow::Size), OPERATIONS, &image).is_empty());
        // Once for each of the 18 values grain's sweep tries, 0 to 100.
        for (follow, says) in [
            (
                Follow::OutOfRange,
                "sample, following grain 100: bias is 150, outside 0 to 100",
            ),
            (
                Follow::Unknown,
                "sample, following grain 100: writes contrast, which it lacks",
            ),
            (
                Follow::Itself,
                "sample, following grain 100: rewrites grain",
            ),
        ] {
            let problems = sweep(&Following(follow), OPERATIONS, &image);
            assert_eq!(problems.len(), 18, "{follow:?}: {problems:?}");
            assert_eq!(problems.last().map(String::as_str), Some(says));
        }
    }

    /// Sample's coarseness reaches up to 100 % plus the bias (decision 30): a bias of 20 %
    /// leaves 150 % outside, which the clamp writes at 120 %, and nothing else.
    #[test]
    fn a_range_follows_the_module_s_values_and_the_clamp_brings_a_number_inside() {
        let mut values = Sample.definition().defaults();
        values.set("coarseness", ControlValue::Number(150.0));
        let at = |values: &Controls| range(&Sample, "coarseness", values).unwrap().unwrap();
        assert_eq!((at(&values).max, at(&values).soft), (200.0, (50.0, 200.0)));
        assert!(clamp(&Sample, &values).is_empty());
        values.set("bias", ControlValue::Number(20.0));
        assert_eq!((at(&values).max, at(&values).soft), (120.0, (50.0, 120.0)));
        let writes = clamp(&Sample, &values);
        assert_eq!(
            writes.iter().collect::<Vec<_>>(),
            [("coarseness", &ControlValue::Number(120.0))]
        );
        // A control that is no number has none; one with a static range, the definition's.
        assert_eq!(range(&Sample, "size", &values), None);
        assert_eq!(
            range(&Sample, "grain", &values),
            Some(Ok(Sample.definition().range("grain").unwrap()))
        );
    }

    /// [`Sample`] with ranges that break a rule, the one its name says, for the sweep to
    /// find; its presets and pair are Sample's.
    struct Ranged(&'static str);

    impl Module for Ranged {
        fn definition(&self) -> &'static Definition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
            Sample.bind(values, image, base)
        }

        fn preset(&self, preset: &str, option: &str, image: &ImageContext) -> Option<Controls> {
            Sample.preset(preset, option, image)
        }

        fn range(&self, control: &str, values: &Controls) -> Option<Range> {
            let range = Sample.range(control, values)?;
            let up_to = |max: f32| Range {
                max,
                soft: (range.min, max),
                ..range
            };
            let (grain, coarseness) = (values.number("grain"), values.number("coarseness"));
            Some(match (self.0, control) {
                // Up to its own value, at least just above its minimum.
                ("its own value", "coarseness") => up_to(coarseness.max(60.0)),
                // Below its default of 100 % while the bias is under 50 %.
                ("its default outside", "coarseness") if values.number("bias") < 50.0 => {
                    up_to(90.0)
                }
                // Beyond the definition's 200 %.
                ("beyond the definition's", "coarseness") => up_to(300.0),
                // A track at the defaults other than the definition's.
                ("a track", "coarseness") => Range {
                    soft: (50.0, 150.0),
                    ..range
                },
                // Coarseness up to 200 % less the grain, and brightness up to 1 EV while
                // coarseness is at most 100 % and the grain over 50 %: every control at its
                // end clamps coarseness from 200 % to 100 %, which then leaves brightness,
                // at 4 EV, outside.
                ("a chain", "coarseness") => up_to(200.0 - grain),
                ("a chain", "brightness") if coarseness <= 100.0 && grain > 50.0 => up_to(1.0),
                // The bias, which the pair covers, from a tenth of the grain.
                ("a view's control", "bias") => Range {
                    min: grain / 10.0,
                    soft: (grain / 10.0, range.max),
                    ..range
                },
                // Coarseness up to 99 % on coarse grain neither off nor at its end, as the
                // grainy look writes it, so that the look needs a clamp; no state the
                // sweep tries has one.
                ("a look", "coarseness")
                    if values.choice("size") == 2 && grain > 0.0 && grain < 100.0 =>
                {
                    up_to(99.0)
                }
                _ => range,
            })
        }
    }

    #[test]
    fn a_sweep_finds_a_range_that_breaks_a_rule() {
        let image = testing::image();
        assert!(sweep(&Sample, OPERATIONS, &image).is_empty());
        for (fault, says) in [
            ("its own value", "its range moves with its own value"),
            ("its default outside", "its default 100 is outside"),
            (
                "beyond the definition's",
                "does not hold inside the definition's",
            ),
            (
                "a track",
                "at its defaults its track and step are not the definition's",
            ),
            (
                "a chain",
                "sample, every control at its end: one clamp pass is not enough",
            ),
            (
                "a view's control",
                "sample.bias, grain 100: a view covers it and its range moves",
            ),
            ("a look", "sample.look: grainy needs a clamp"),
        ] {
            let problems = sweep(&Ranged(fault), OPERATIONS, &image);
            assert!(
                problems.iter().any(|problem| problem.contains(says)),
                "{fault}: {problems:?}"
            );
        }
    }

    #[test]
    fn a_sweep_finds_a_binding_that_sets_what_it_does_not_own_or_too_much() {
        let image = testing::image();
        let strays = fake(
            "strays",
            &[Owned::new("grain", "strength")],
            vec![Setting::new("exposure", "black", Value::Float(0.0))],
        );
        let too_much = fake(
            "too_much",
            &[Owned::new("grain", "strength")],
            vec![Setting::new("grain", "strength", Value::Float(150.0))],
        );
        let wrong_kind = fake(
            "wrong_kind",
            &[Owned::new("grain", "strength")],
            vec![Setting::new("grain", "strength", Value::Bool(true))],
        );
        for module in [&strays, &too_much, &wrong_kind] {
            assert!(
                !sweep(module, OPERATIONS, &image).is_empty(),
                "{}",
                module.definition.id
            );
        }
        let fine = fake(
            "fine",
            &[Owned::new("grain", "strength")],
            vec![Setting::new("grain", "strength", Value::Float(50.0))],
        );
        assert!(sweep(&fine, OPERATIONS, &image).is_empty());
    }
}
