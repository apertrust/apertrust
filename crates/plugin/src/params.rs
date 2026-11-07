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

//! Operation parameters as data: each operation's schema, and values checked against
//! it. Edits, the session protocol and the CLI all set parameters this way, by name;
//! darktable ports use darktable's field names.

use std::collections::BTreeMap;

/// An operation's parameters.
#[derive(Debug)]
pub struct Schema {
    /// Raised when a parameter is added, removed, or changes meaning or range.
    pub version: u32,
    pub params: &'static [Param],
}

#[derive(Debug)]
pub struct Param {
    pub name: &'static str,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// A number in `min..=max`.
    Float {
        default: f32,
        min: f32,
        max: f32,
    },
    /// A whole number in `min..=max`, such as a crop margin or a raw level.
    Int {
        default: i64,
        min: i64,
        max: i64,
    },
    /// One of the named choices, by index.
    Choice {
        default: u32,
        choices: &'static [&'static str],
    },
    Bool {
        default: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Float(f32),
    Int(i64),
    Choice(u32),
    Bool(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Float(value) => write!(f, "{value}"),
            Value::Int(value) => write!(f, "{value}"),
            Value::Choice(index) => write!(f, "choice {index}"),
            Value::Bool(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParamError {
    #[error("no parameter {0}")]
    Unknown(String),
    #[error("{name} takes {expected}")]
    WrongKind {
        name: &'static str,
        expected: &'static str,
    },
    #[error("{name} is {value}, outside {min} to {max}")]
    OutOfRange {
        name: &'static str,
        value: f64,
        min: f64,
        max: f64,
    },
    #[error("{name} has no choice {value}")]
    NoChoice { name: &'static str, value: String },
    #[error("{name}: cannot read {text:?}")]
    Invalid { name: &'static str, text: String },
}

/// Parameter values by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParamValues(BTreeMap<String, Value>);

impl ParamValues {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.0.insert(name.to_owned(), value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.0.get(name).copied()
    }

    /// Takes the value away, so the parameter keeps its default.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.0.remove(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Value)> {
        self.0.iter().map(|(name, value)| (name.as_str(), *value))
    }

    /// A float parameter's value. Values come resolved against the schema
    /// ([`Schema::resolve`]), so a missing or mistyped one is a bug in the operation.
    pub fn float(&self, name: &str) -> f32 {
        match self.get(name) {
            Some(Value::Float(value)) => value,
            other => panic!("{name}: expected a resolved float, got {other:?}"),
        }
    }

    /// A whole-number parameter's value; see [`Self::float`].
    pub fn int(&self, name: &str) -> i64 {
        match self.get(name) {
            Some(Value::Int(value)) => value,
            other => panic!("{name}: expected a resolved whole number, got {other:?}"),
        }
    }

    /// A choice parameter's index; see [`Self::float`].
    pub fn choice(&self, name: &str) -> u32 {
        match self.get(name) {
            Some(Value::Choice(value)) => value,
            other => panic!("{name}: expected a resolved choice, got {other:?}"),
        }
    }

    /// A boolean parameter's value; see [`Self::float`].
    pub fn flag(&self, name: &str) -> bool {
        match self.get(name) {
            Some(Value::Bool(value)) => value,
            other => panic!("{name}: expected a resolved boolean, got {other:?}"),
        }
    }
}

impl Schema {
    pub fn param(&self, name: &str) -> Option<&'static Param> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn defaults(&self) -> ParamValues {
        let mut values = ParamValues::new();
        for param in self.params {
            values.set(param.name, param.default());
        }
        values
    }

    /// The defaults with `values` applied over them, each checked against its
    /// parameter: what `Operation::prepare` takes.
    pub fn resolve(&self, values: &ParamValues) -> Result<ParamValues, ParamError> {
        self.resolve_over(&self.defaults(), values)
    }

    /// `base`, an operation's defaults for an image (`Operation::defaults`), with
    /// `values` applied over it; every value is checked against its parameter, and a
    /// parameter `base` lacks takes the schema's default.
    pub fn resolve_over(
        &self,
        base: &ParamValues,
        values: &ParamValues,
    ) -> Result<ParamValues, ParamError> {
        let mut resolved = self.defaults();
        for (name, value) in base.iter().chain(values.iter()) {
            let param = self
                .param(name)
                .ok_or_else(|| ParamError::Unknown(name.to_owned()))?;
            param.check(value)?;
            resolved.set(name, value);
        }
        Ok(resolved)
    }
}

impl Param {
    pub fn default(&self) -> Value {
        match self.kind {
            Kind::Float { default, .. } => Value::Float(default),
            Kind::Int { default, .. } => Value::Int(default),
            Kind::Choice { default, .. } => Value::Choice(default),
            Kind::Bool { default } => Value::Bool(default),
        }
    }

    /// Whether `value` has this parameter's kind and lies in its range.
    pub fn check(&self, value: Value) -> Result<(), ParamError> {
        let name = self.name;
        match (self.kind, value) {
            (Kind::Float { min, max, .. }, Value::Float(value)) => {
                // Also refuses NaN, which no range contains.
                if (min..=max).contains(&value) {
                    Ok(())
                } else {
                    Err(ParamError::OutOfRange {
                        name,
                        value: value.into(),
                        min: min.into(),
                        max: max.into(),
                    })
                }
            }
            (Kind::Int { min, max, .. }, Value::Int(value)) => {
                if (min..=max).contains(&value) {
                    Ok(())
                } else {
                    Err(ParamError::OutOfRange {
                        name,
                        value: value as f64,
                        min: min as f64,
                        max: max as f64,
                    })
                }
            }
            (Kind::Choice { choices, .. }, Value::Choice(index)) => {
                if (index as usize) < choices.len() {
                    Ok(())
                } else {
                    Err(ParamError::NoChoice {
                        name,
                        value: index.to_string(),
                    })
                }
            }
            (Kind::Bool { .. }, Value::Bool(_)) => Ok(()),
            (kind, _) => Err(ParamError::WrongKind {
                name,
                expected: kind.describe(),
            }),
        }
    }

    /// Reads a value from text: a number, a choice by name or index, or `true` or
    /// `false`. The result still needs [`Self::check`].
    pub fn parse(&self, text: &str) -> Result<Value, ParamError> {
        let name = self.name;
        let invalid = || ParamError::Invalid {
            name,
            text: text.to_owned(),
        };
        match self.kind {
            Kind::Float { .. } => text.parse().map(Value::Float).map_err(|_| invalid()),
            Kind::Int { .. } => text.parse().map(Value::Int).map_err(|_| invalid()),
            Kind::Choice { choices, .. } => choices
                .iter()
                .position(|choice| choice.eq_ignore_ascii_case(text))
                .map(|index| index as u32)
                .or_else(|| text.parse().ok())
                .map(Value::Choice)
                .ok_or_else(|| ParamError::NoChoice {
                    name,
                    value: text.to_owned(),
                }),
            Kind::Bool { .. } => text.parse().map(Value::Bool).map_err(|_| invalid()),
        }
    }

    /// The value as text that [`Self::parse`] reads back exactly: a number in its
    /// shortest exact form, a choice by name, `true` or `false`.
    pub fn text(&self, value: Value) -> String {
        match (self.kind, value) {
            (Kind::Choice { choices, .. }, Value::Choice(index)) => choices
                .get(index as usize)
                .map_or_else(|| index.to_string(), |choice| (*choice).to_owned()),
            (_, value) => value.to_string(),
        }
    }

    /// A value read from a saved edit, made usable as decision 21 asks: a number outside
    /// the range is brought to its nearer end. A choice the parameter lacks, a value of
    /// the wrong kind and NaN stay refused.
    pub fn clamp(&self, value: Value) -> Result<Value, ParamError> {
        let value = match (self.kind, value) {
            (Kind::Float { min, max, .. }, Value::Float(v)) if !v.is_nan() => {
                Value::Float(v.clamp(min, max))
            }
            (Kind::Int { min, max, .. }, Value::Int(v)) => Value::Int(v.clamp(min, max)),
            _ => value,
        };
        self.check(value).map(|()| value)
    }
}

impl Kind {
    fn describe(&self) -> &'static str {
        match self {
            Kind::Float { .. } => "a number",
            Kind::Int { .. } => "a whole number",
            Kind::Choice { .. } => "a choice",
            Kind::Bool { .. } => "true or false",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static SCHEMA: Schema = Schema {
        version: 1,
        params: &[
            Param {
                name: "exposure",
                kind: Kind::Float {
                    default: 0.0,
                    min: -18.0,
                    max: 18.0,
                },
            },
            Param {
                name: "black",
                kind: Kind::Int {
                    default: 0,
                    min: 0,
                    max: 65535,
                },
            },
            Param {
                name: "primaries",
                kind: Kind::Choice {
                    default: 1,
                    choices: &["sRGB", "Rec2020"],
                },
            },
            Param {
                name: "enabled",
                kind: Kind::Bool { default: true },
            },
        ],
    };

    #[test]
    fn resolve_applies_values_over_the_defaults() {
        let mut values = ParamValues::new();
        values.set("exposure", Value::Float(1.5));
        let resolved = SCHEMA.resolve(&values).unwrap();
        assert_eq!(resolved.float("exposure"), 1.5);
        assert_eq!(resolved.int("black"), 0);
        assert_eq!(resolved.choice("primaries"), 1);
        assert!(resolved.flag("enabled"));
    }

    #[test]
    fn resolve_refuses_unknown_mistyped_and_out_of_range_values() {
        let refuse = |name: &str, value| {
            let mut values = ParamValues::new();
            values.set(name, value);
            SCHEMA.resolve(&values).unwrap_err()
        };
        assert_eq!(
            refuse("contrast", Value::Float(1.0)),
            ParamError::Unknown("contrast".into())
        );
        assert!(matches!(
            refuse("exposure", Value::Bool(true)),
            ParamError::WrongKind { .. }
        ));
        assert!(matches!(
            refuse("exposure", Value::Float(20.0)),
            ParamError::OutOfRange { .. }
        ));
        assert!(matches!(
            refuse("exposure", Value::Float(f32::NAN)),
            ParamError::OutOfRange { .. }
        ));
        assert!(matches!(
            refuse("black", Value::Int(65536)),
            ParamError::OutOfRange { .. }
        ));
        assert!(matches!(
            refuse("black", Value::Float(1.0)),
            ParamError::WrongKind { .. }
        ));
        assert!(matches!(
            refuse("primaries", Value::Choice(2)),
            ParamError::NoChoice { .. }
        ));
    }

    #[test]
    fn parse_reads_numbers_whole_numbers_choices_and_booleans() {
        let param = |name| SCHEMA.param(name).unwrap();
        assert_eq!(param("exposure").parse("-0.5"), Ok(Value::Float(-0.5)));
        assert_eq!(param("black").parse("1024"), Ok(Value::Int(1024)));
        assert!(param("black").parse("1024.5").is_err());
        assert_eq!(param("primaries").parse("srgb"), Ok(Value::Choice(0)));
        assert_eq!(param("primaries").parse("1"), Ok(Value::Choice(1)));
        assert_eq!(param("enabled").parse("false"), Ok(Value::Bool(false)));
        assert!(param("exposure").parse("bright").is_err());
        assert!(param("primaries").parse("P3").is_err());
    }

    #[test]
    fn text_reads_back_exactly() {
        for (name, value) in [
            ("exposure", Value::Float(0.1 + 0.2)),
            ("exposure", Value::Float(-17.999_998)),
            ("black", Value::Int(1023)),
            ("primaries", Value::Choice(1)),
            ("enabled", Value::Bool(false)),
        ] {
            let param = SCHEMA.param(name).unwrap();
            let text = param.text(value);
            assert_eq!(param.parse(&text), Ok(value), "{name}: {text}");
        }
        assert_eq!(
            SCHEMA.param("primaries").unwrap().text(Value::Choice(0)),
            "sRGB"
        );
    }

    #[test]
    fn a_saved_value_out_of_range_is_clamped() {
        let exposure = SCHEMA.param("exposure").unwrap();
        assert_eq!(exposure.clamp(Value::Float(30.0)), Ok(Value::Float(18.0)));
        assert_eq!(exposure.clamp(Value::Float(-30.0)), Ok(Value::Float(-18.0)));
        assert!(exposure.clamp(Value::Float(f32::NAN)).is_err());
        let black = SCHEMA.param("black").unwrap();
        assert_eq!(black.clamp(Value::Int(-1)), Ok(Value::Int(0)));
        let primaries = SCHEMA.param("primaries").unwrap();
        assert!(primaries.clamp(Value::Choice(5)).is_err());
        assert!(exposure.clamp(Value::Bool(true)).is_err());
    }
}
