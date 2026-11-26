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

//! Tool values, what an edit stores (decision 28): per control a value where one is
//! set, and a mark; and, for a switched tool (decision 32), whether it is on.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::definition::Published;

/// One control's value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ControlValue {
    #[serde(deserialize_with = "apertrust_base::wire::finite")]
    Number(f32),
    /// One of the control's options, by index; edits save it by the option's id.
    Choice(u32),
    Bool(bool),
    /// A curve's points, `[x, y]` from left to right.
    #[serde(deserialize_with = "apertrust_base::wire::finite")]
    Points(Vec<[f32; 2]>),
}

/// What the marker beside a control says (principle 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mark {
    /// The user set it, or kept it at its default: Auto and Match leave it alone.
    Kept,
    /// Auto or Match set it.
    Auto,
}

/// A tool's values: for each control a value where one is set, its default for the
/// photo otherwise, and a mark. A control can be kept without a value, held at its
/// default. Controls are named by id; values read from an older edit may name controls
/// the tool no longer has, until [`crate::ProcessingTool::migrate`] maps them.
///
/// A switched tool (decision 32, [`crate::ProcessingDefinition::switched`]) is on while
/// [`Self::enabled`] holds: off, it holds no value, only marks, and renders nothing; on,
/// it renders with darktable's defaults for the controls it holds no value for. A tool
/// that is not switched never sets it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolValues {
    values: BTreeMap<String, ControlValue>,
    marks: BTreeMap<String, Mark>,
    enabled: bool,
}

impl ToolValues {
    pub fn new() -> Self {
        Self::default()
    }

    /// Neither a value nor a mark, and not on: the tool is untouched.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.marks.is_empty() && !self.enabled
    }

    /// Whether a switched tool is on (decision 32); false for one off, and for every
    /// tool that is not switched, which is always on.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Turns a switched tool on or off, its values and marks as they are.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get(&self, control: &str) -> Option<&ControlValue> {
        self.values.get(control)
    }

    pub fn number(&self, control: &str) -> Option<f32> {
        match self.get(control)? {
            ControlValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn choice(&self, control: &str) -> Option<u32> {
        match self.get(control)? {
            ControlValue::Choice(index) => Some(*index),
            _ => None,
        }
    }

    pub fn flag(&self, control: &str) -> Option<bool> {
        match self.get(control)? {
            ControlValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn points(&self, control: &str) -> Option<&[[f32; 2]]> {
        match self.get(control)? {
            ControlValue::Points(points) => Some(points),
            _ => None,
        }
    }

    /// Sets the value, leaving the mark as it is.
    pub fn set(&mut self, control: &str, value: ControlValue) {
        self.values.insert(control.to_owned(), value);
    }

    /// Takes the value away, so the control has its default; the mark stays.
    pub fn remove(&mut self, control: &str) -> Option<ControlValue> {
        self.values.remove(control)
    }

    pub fn mark(&self, control: &str) -> Option<Mark> {
        self.marks.get(control).copied()
    }

    pub fn set_mark(&mut self, control: &str, mark: Option<Mark>) {
        match mark {
            Some(mark) => self.marks.insert(control.to_owned(), mark),
            None => self.marks.remove(control),
        };
    }

    /// The values set, by control id.
    pub fn values(&self) -> impl Iterator<Item = (&str, &ControlValue)> {
        self.values.iter().map(|(id, value)| (id.as_str(), value))
    }

    /// The marks, by control id.
    pub fn marks(&self) -> impl Iterator<Item = (&str, Mark)> {
        self.marks.iter().map(|(id, mark)| (id.as_str(), *mark))
    }

    /// Only the values that differ from `defaults`, without marks, on or off as these
    /// are: what a binding is given, so a control set back to its default sets nothing.
    pub fn off_defaults(&self, defaults: &Controls) -> ToolValues {
        let values = self
            .values
            .iter()
            .filter(|(id, value)| defaults.get(id) != Some(*value))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        ToolValues {
            values,
            marks: BTreeMap::new(),
            enabled: self.enabled,
        }
    }
}

/// The facts other tools publish that an action reads ([`crate::ActionEnd::reads`];
/// decision 107), each from its tool's own [`crate::ProcessingTool::publish`] on the same
/// edit: a fact its tool gives none of is left out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PublishedValues(Vec<(Published, f32)>);

impl PublishedValues {
    pub fn new() -> Self {
        Self::default()
    }

    /// Holds `value` for `fact`, in place of any before it.
    pub fn set(&mut self, fact: Published, value: f32) {
        self.0.retain(|(held, _)| *held != fact);
        self.0.push((fact, value));
    }

    /// The value `fact` was given, if it was read.
    pub fn get(&self, fact: &Published) -> Option<f32> {
        (self.0.iter()).find_map(|(held, value)| (held == fact).then_some(*value))
    }
}

/// Every control's untouched value for a photo ([`crate::ProcessingTool::defaults`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Controls(BTreeMap<&'static str, ControlValue>);

impl Controls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, control: &'static str, value: ControlValue) {
        self.0.insert(control, value);
    }

    pub fn get(&self, control: &str) -> Option<&ControlValue> {
        self.0.get(control)
    }

    /// Each control's value, by id.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &ControlValue)> {
        self.0.iter().map(|(&control, value)| (control, value))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// A number control's default; a tool asks only for its own controls, so a
    /// missing or mistyped one is a bug in the tool.
    pub fn number(&self, control: &str) -> f32 {
        match self.get(control) {
            Some(ControlValue::Number(value)) => *value,
            other => panic!("{control}: expected a number, got {other:?}"),
        }
    }

    /// A choice control's default; see [`Self::number`].
    pub fn choice(&self, control: &str) -> u32 {
        match self.get(control) {
            Some(ControlValue::Choice(index)) => *index,
            other => panic!("{control}: expected a choice, got {other:?}"),
        }
    }

    /// A boolean control's default; see [`Self::number`].
    pub fn flag(&self, control: &str) -> bool {
        match self.get(control) {
            Some(ControlValue::Bool(value)) => *value,
            other => panic!("{control}: expected true or false, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binding_sees_only_values_off_their_defaults() {
        let mut defaults = Controls::new();
        defaults.set("exposure", ControlValue::Number(0.0));
        defaults.set("mode", ControlValue::Choice(1));
        let mut values = ToolValues::new();
        values.set("exposure", ControlValue::Number(0.0));
        values.set("mode", ControlValue::Choice(2));
        values.set_mark("exposure", Some(Mark::Kept));
        let off = values.off_defaults(&defaults);
        assert_eq!(off.get("exposure"), None);
        assert_eq!(off.choice("mode"), Some(2));
        assert_eq!(off.mark("exposure"), None);
        assert!(!values.is_empty());
        assert!(ToolValues::new().off_defaults(&defaults).is_empty());
    }

    #[test]
    fn a_mark_stays_when_its_value_goes() {
        let mut values = ToolValues::new();
        values.set("contrast", ControlValue::Number(0.2));
        values.set_mark("contrast", Some(Mark::Auto));
        values.remove("contrast");
        assert_eq!(values.mark("contrast"), Some(Mark::Auto));
        values.set_mark("contrast", None);
        assert!(values.is_empty());
    }

    #[test]
    fn a_switched_tool_on_with_no_value_is_not_untouched() {
        let mut values = ToolValues::new();
        values.set_enabled(true);
        assert!(!values.is_empty());
        assert!(values.off_defaults(&Controls::new()).enabled());
        values.set_enabled(false);
        assert!(values.is_empty());
    }
}
