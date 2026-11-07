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

//! Values set for one run, darktable's `--conf KEY=VALUE` (decision 43;
//! `docs/settings.md`, The settings schema, Overrides): [`Overrides`], read against a
//! program's registry before anything else is, which its [`Store`](super::Store) then
//! holds over the file ([`Store::with_overrides`](super::Store::with_overrides)).

use std::collections::BTreeMap;
use std::fmt;

use toml::{Table, Value};

use super::{Declared, Kind};

/// The flag that sets a setting's value for one run, darktable's.
pub const CONF_FLAG: &str = "--conf";

/// Settings' values for one run, each given as `--conf KEY=VALUE`: KEY a setting's id, as
/// `settings.toml` has it (`scale`, `engine.memory_budget`), VALUE its value, read by the
/// setting's kind with the file's rules, so a number out of range is read as the nearer
/// end, with a warning, and a choice by its id. A text or a choice is VALUE as it is, with
/// no quotes and nothing special, `~` included; anything else is written as in the file: a
/// number, `true` or `false`, `[1280, 800]` for a list, `{ undo = ["Ctrl+K"] }` for a
/// table. Given twice for one setting, the last counts.
///
/// A store that holds them gives their values in place of the file's and the defaults,
/// and never writes them ([`Store::with_overrides`](super::Store::with_overrides)); a flag
/// of a setting's own, as `--adapter`, comes before them, which its owner decides.
#[derive(Clone, Default)]
pub struct Overrides {
    /// Each setting given, by id: the last given for it.
    given: BTreeMap<&'static str, Given>,
}

/// One setting's value for the run.
#[derive(Clone)]
struct Given {
    setting: &'static dyn Declared,
    /// As `settings.toml` would hold it, which the store reads again by the same rules.
    value: Value,
    /// What reading it warned of: a number out of range, or a table's entry that cannot be
    /// read, which is left out.
    warnings: Vec<String>,
}

impl Overrides {
    /// Reads each of `given`, in order, as `KEY=VALUE`, against `registry`, the program's
    /// settings, and logs what reading warned of under `-d settings`, which shows without
    /// it. A key that is none of them, or a value its kind cannot read, is an error, which
    /// stops the program's start; the first is the one returned.
    pub fn read<S: AsRef<str>>(
        given: impl IntoIterator<Item = S>,
        registry: &[&'static dyn Declared],
    ) -> Result<Overrides, OverrideError> {
        let mut overrides = Overrides::default();
        for given in given {
            let given = given.as_ref();
            let (key, text) = given
                .split_once('=')
                .ok_or_else(|| OverrideError::Form(given.to_owned()))?;
            let setting = registry
                .iter()
                .find(|setting| setting.id() == key)
                .ok_or_else(|| OverrideError::Unknown(key.to_owned()))?;
            let unreadable = || OverrideError::Unreadable {
                key: key.to_owned(),
                value: text.to_owned(),
            };
            let value = value(setting.kind(), text).ok_or_else(unreadable)?;
            let mut warnings = Vec::new();
            setting.read(&value, &mut warnings).ok_or_else(unreadable)?;
            let given = Given {
                setting: *setting,
                value,
                warnings,
            };
            overrides.given.insert(setting.id(), given);
        }
        for warning in overrides.warnings() {
            tracing::warn!(target: "settings", "{CONF_FLAG}: {warning}");
        }
        Ok(overrides)
    }

    /// What reading the values that count warned of, each as the file's are: the
    /// setting's id first.
    pub fn warnings(&self) -> impl Iterator<Item = &str> {
        let given = self.given.values();
        given.flat_map(|given| given.warnings.iter().map(String::as_str))
    }

    /// `setting`'s value for the run as `settings.toml` would keep it, for the log, or
    /// None where none is given.
    pub fn value(&self, setting: &dyn Declared) -> Option<Value> {
        let given = self.given.get(setting.id())?;
        let read = given.setting.read(&given.value, &mut Vec::new())?;
        given.setting.write(&*read)
    }

    /// Each setting given, by id, with its value as `settings.toml` would hold it.
    pub(super) fn given(&self) -> impl Iterator<Item = (&'static str, &Value)> {
        self.given.iter().map(|(id, given)| (*id, &given.value))
    }
}

impl fmt::Debug for Overrides {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.given()).finish()
    }
}

/// `text` as `settings.toml` would hold a value of `kind`: a text or a choice's id as it
/// is; anything else as one TOML value, or None where it is not one.
fn value(kind: &Kind, text: &str) -> Option<Value> {
    match kind {
        Kind::Text | Kind::Choice(_) => Some(Value::String(text.to_owned())),
        Kind::Boolean | Kind::Number(_) | Kind::List { .. } | Kind::Table { .. } => {
            let mut table: Table = format!("v = {text}").parse().ok()?;
            let value = table.remove("v")?;
            // Not a value and more after it, as a newline and another key.
            table.is_empty().then_some(value)
        }
    }
}

/// Why a `--conf` cannot be taken: the program does not start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverrideError {
    /// Not `KEY=VALUE`.
    Form(String),
    /// A key that is no setting the program reads.
    Unknown(String),
    /// A value the setting's kind cannot read.
    Unreadable { key: String, value: String },
}

impl fmt::Display for OverrideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverrideError::Form(given) => write!(f, "{CONF_FLAG} {given:?} is not KEY=VALUE"),
            OverrideError::Unknown(key) => {
                write!(f, "{CONF_FLAG} {key}: no setting this program reads")
            }
            OverrideError::Unreadable { key, value } => {
                write!(f, "{CONF_FLAG} {key}: cannot be {value:?}")
            }
        }
    }
}

impl std::error::Error for OverrideError {}
