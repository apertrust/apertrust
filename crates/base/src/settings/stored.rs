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

//! How a setting's value is read from `settings.toml` and written into it, kind by kind:
//! [`Stored`], which the type of every generated static implements, and [`Declared`], a
//! setting of any type, as a registry lists it for the store.

use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::fmt;

use toml::Value;

use super::{Applies, Choice, Kind, Number, Page, Setting, Store};

/// A type a setting's static is typed by, and how its value is read from `settings.toml`
/// and written into it: `bool`, `f32`, a [`Choice`], `&'static str`, a list as an array (of
/// a set length) or a slice, a [`Table`](super::Table), and `Option` of one for an optional
/// setting.
///
/// The store hands out and takes [`Owned`](Stored::Owned) values: the static's own type for
/// a boolean, a number or a choice, and `String`, `Vec`, an array of them or a `BTreeMap`
/// for text, lists and tables.
pub trait Stored: Copy + Sync + 'static {
    type Owned: Clone + PartialEq + fmt::Debug + Send + 'static;

    /// The static's value, a default, as the store hands it out.
    fn owned(self) -> Self::Owned;

    /// `value` as `kind` declares it, or None when it cannot be read. `at` is its dotted id,
    /// for a warning about part of it: a number out of range, read as the nearer end, or an
    /// entry of a table that cannot be read, left out.
    fn read(
        kind: &Kind,
        value: &Value,
        at: &str,
        warnings: &mut Vec<String>,
    ) -> Option<Self::Owned>;

    /// `value` as `settings.toml` keeps it, as `kind` declares it: a number with its
    /// decimals, a choice by its id; None for none, an optional setting's.
    fn write(kind: &Kind, value: &Self::Owned) -> Option<Value>;

    /// A choice's options' ids, in its type's order, as the Settings dialog lists them, and
    /// a list of a choice's; none for another kind.
    fn options() -> Vec<&'static str> {
        Vec::new()
    }
}

/// The warning for a value that cannot be read, which then gets its default.
pub(super) fn unreadable(at: &str, value: &Value) -> String {
    format!("{at} cannot be {value}; the default is used")
}

/// A number's range and decimals, or none for another kind.
fn number(kind: &Kind) -> Number {
    match kind {
        Kind::Number(number) => *number,
        _ => Number::default(),
    }
}

/// The kind of a list's items or a table's values.
fn of(kind: &Kind) -> &Kind {
    match kind {
        Kind::List { of, .. } | Kind::Table { of, .. } => of,
        other => other,
    }
}

impl Stored for bool {
    type Owned = bool;

    fn owned(self) -> bool {
        self
    }

    fn read(_: &Kind, value: &Value, _: &str, _: &mut Vec<String>) -> Option<bool> {
        value.as_bool()
    }

    fn write(_: &Kind, value: &bool) -> Option<Value> {
        Some(Value::Boolean(*value))
    }
}

/// An integer or a float that an `f32` holds; one that is not more than the number's
/// `above` cannot be read, and one out of its range is read as the nearer end.
impl Stored for f32 {
    type Owned = f32;

    fn owned(self) -> f32 {
        self
    }

    fn read(kind: &Kind, value: &Value, at: &str, warnings: &mut Vec<String>) -> Option<f32> {
        let read = value
            .as_float()
            .or_else(|| value.as_integer().map(|i| i as f64))? as f32;
        let number = number(kind);
        if !read.is_finite() || number.above.is_some_and(|above| read <= above) {
            return None;
        }
        let range = number.range();
        let used = read.clamp(*range.start(), *range.end());
        if used != read {
            warnings.push(format!("{at} {read} is out of range; {used} is used"));
        }
        Some(used)
    }

    /// Whole with no decimals, rounded to them with some: 1.1 is kept as 1.1, not as
    /// 1.100000023841858, the `f32`'s own.
    fn write(kind: &Kind, value: &f32) -> Option<Value> {
        Some(match number(kind).decimals {
            Some(0) => Value::Integer(value.round() as i64),
            Some(decimals) => {
                let scale = 10f64.powi(decimals as i32);
                Value::Float((f64::from(*value) * scale).round() / scale)
            }
            None => Value::Float(f64::from(*value)),
        })
    }
}

/// A choice by its option's id.
impl<T: Choice + Sync + Send + PartialEq + fmt::Debug> Stored for T {
    type Owned = T;

    fn owned(self) -> T {
        self
    }

    fn read(_: &Kind, value: &Value, _: &str, _: &mut Vec<String>) -> Option<T> {
        T::from_id(value.as_str()?)
    }

    fn write(_: &Kind, value: &T) -> Option<Value> {
        let (_, id) = T::OPTIONS.iter().find(|(option, _)| option == value)?;
        Some(Value::String((*id).to_owned()))
    }

    fn options() -> Vec<&'static str> {
        T::OPTIONS.iter().map(|(_, id)| *id).collect()
    }
}

impl Stored for &'static str {
    type Owned = String;

    fn owned(self) -> String {
        self.to_owned()
    }

    fn read(_: &Kind, value: &Value, _: &str, _: &mut Vec<String>) -> Option<String> {
        value.as_str().map(str::to_owned)
    }

    fn write(_: &Kind, value: &String) -> Option<Value> {
        Some(Value::String(value.clone()))
    }
}

/// Reads each of `items`, or none when one cannot be read: then what reading the others
/// warned of is left out, since the whole list gets its default.
fn items<T: Stored>(
    kind: &Kind,
    items: &[Value],
    at: &str,
    warnings: &mut Vec<String>,
) -> Option<Vec<T::Owned>> {
    let mut theirs = Vec::new();
    let read = items
        .iter()
        .map(|item| T::read(of(kind), item, at, &mut theirs))
        .collect::<Option<Vec<_>>>()?;
    warnings.append(&mut theirs);
    Some(read)
}

/// A list of a set length, which only that many items can be.
impl<T: Stored, const N: usize> Stored for [T; N] {
    type Owned = [T::Owned; N];

    fn owned(self) -> [T::Owned; N] {
        self.map(T::owned)
    }

    fn read(
        kind: &Kind,
        value: &Value,
        at: &str,
        warnings: &mut Vec<String>,
    ) -> Option<[T::Owned; N]> {
        let read = items::<T>(kind, value.as_array()?, at, warnings)?;
        read.try_into().ok()
    }

    fn write(kind: &Kind, value: &[T::Owned; N]) -> Option<Value> {
        let items = value.iter().filter_map(|item| T::write(of(kind), item));
        Some(Value::Array(items.collect()))
    }
}

/// A list of any length; a single item may be written without its list.
impl<T: Stored> Stored for &'static [T] {
    type Owned = Vec<T::Owned>;

    fn owned(self) -> Vec<T::Owned> {
        self.iter().map(|item| item.owned()).collect()
    }

    fn read(
        kind: &Kind,
        value: &Value,
        at: &str,
        warnings: &mut Vec<String>,
    ) -> Option<Vec<T::Owned>> {
        match value.as_array() {
            Some(list) => items::<T>(kind, list, at, warnings),
            None => items::<T>(kind, std::slice::from_ref(value), at, warnings),
        }
    }

    fn write(kind: &Kind, value: &Vec<T::Owned>) -> Option<Value> {
        let items = value.iter().filter_map(|item| T::write(of(kind), item));
        Some(Value::Array(items.collect()))
    }

    /// A list of a choice's: the choice's options, which the Settings dialog ticks.
    fn options() -> Vec<&'static str> {
        T::options()
    }
}

/// A table keyed by another registry's ids, in their order: an entry that cannot be read
/// is left out with a warning, and the rest are read.
impl<V: Stored> Stored for &'static [(&'static str, V)] {
    type Owned = BTreeMap<String, V::Owned>;

    fn owned(self) -> BTreeMap<String, V::Owned> {
        let entries = self
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.owned()));
        entries.collect()
    }

    fn read(
        kind: &Kind,
        value: &Value,
        at: &str,
        warnings: &mut Vec<String>,
    ) -> Option<BTreeMap<String, V::Owned>> {
        let mut read = BTreeMap::new();
        for (key, entry) in value.as_table()? {
            let at = format!("{at}.{key}");
            match V::read(of(kind), entry, &at, warnings) {
                Some(entry) => _ = read.insert(key.clone(), entry),
                None => warnings.push(unreadable(&at, entry)),
            }
        }
        Some(read)
    }

    fn write(kind: &Kind, value: &BTreeMap<String, V::Owned>) -> Option<Value> {
        let entries = value
            .iter()
            .filter_map(|(key, entry)| Some((key.clone(), V::write(of(kind), entry)?)));
        Some(Value::Table(entries.collect()))
    }
}

/// An optional setting: None is not written, so it is missing from the file.
impl<T: Stored> Stored for Option<T> {
    type Owned = Option<T::Owned>;

    fn owned(self) -> Option<T::Owned> {
        self.map(T::owned)
    }

    fn read(
        kind: &Kind,
        value: &Value,
        at: &str,
        warnings: &mut Vec<String>,
    ) -> Option<Option<T::Owned>> {
        T::read(kind, value, at, warnings).map(Some)
    }

    fn write(kind: &Kind, value: &Option<T::Owned>) -> Option<Value> {
        T::write(kind, value.as_ref()?)
    }
}

/// A setting of any type, as a registry lists it for the store (`Store::open`): each
/// owner's generated `ALL`, every setting it owns in the schema's order.
pub trait Declared: Sync {
    fn id(&self) -> &'static str;

    fn kind(&self) -> &Kind;

    /// The type of its values, its static's `Owned`, which the store checks a static
    /// given it against.
    fn value_type(&self) -> TypeId;

    /// `value` read as the setting's `Owned`, or None when it cannot be read.
    fn read(&self, value: &Value, warnings: &mut Vec<String>) -> Option<Box<dyn Any + Send>>;

    /// A value [`read`](Declared::read) gave, or the store was set to, as `settings.toml`
    /// keeps it.
    fn write(&self, value: &(dyn Any + Send)) -> Option<Value>;

    /// When a change to it takes effect.
    fn applies(&self) -> Applies;

    /// The Settings dialog's page that shows it, and its group there; None for neither.
    fn page(&self) -> Option<(Page, &'static str)>;

    /// Its place among every owner's settings, in the schema's order.
    fn place(&self) -> u32;

    /// Its default as `settings.toml` would keep it, what it computes now for a computed
    /// one; None for an optional setting's.
    fn default_value(&self) -> Option<Value>;

    /// A choice's options' ids, in its type's order, and a list of a choice's; none for
    /// another kind.
    fn options(&self) -> Vec<&'static str>;

    /// Its value in `store`, as `settings.toml` keeps it ([`Store::value`]).
    #[doc(hidden)]
    fn value_in(&self, store: &Store) -> Option<Value>;

    /// Sets it in `store` to `value`, read as `settings.toml`'s; false when it cannot be
    /// read ([`Store::set_value`]).
    #[doc(hidden)]
    fn set_in(&self, store: &mut Store, value: &Value) -> bool;

    /// Resets it in `store` ([`Store::reset_value`]).
    #[doc(hidden)]
    fn reset_in(&self, store: &mut Store);
}

impl<T: Stored> Declared for Setting<T> {
    fn id(&self) -> &'static str {
        self.id
    }

    fn kind(&self) -> &Kind {
        &self.kind
    }

    fn value_type(&self) -> TypeId {
        TypeId::of::<T::Owned>()
    }

    fn read(&self, value: &Value, warnings: &mut Vec<String>) -> Option<Box<dyn Any + Send>> {
        let read = T::read(&self.kind, value, self.id, warnings)?;
        Some(Box::new(read))
    }

    fn write(&self, value: &(dyn Any + Send)) -> Option<Value> {
        T::write(&self.kind, value.downcast_ref::<T::Owned>()?)
    }

    fn applies(&self) -> Applies {
        self.applies
    }

    fn page(&self) -> Option<(Page, &'static str)> {
        Some((self.page?, self.group?))
    }

    fn place(&self) -> u32 {
        self.place
    }

    fn default_value(&self) -> Option<Value> {
        T::write(&self.kind, &self.default().owned())
    }

    fn options(&self) -> Vec<&'static str> {
        T::options()
    }

    fn value_in(&self, store: &Store) -> Option<Value> {
        T::write(&self.kind, &store.get(self))
    }

    fn set_in(&self, store: &mut Store, value: &Value) -> bool {
        match T::read(&self.kind, value, self.id, &mut Vec::new()) {
            Some(read) => {
                store.set(self, read);
                true
            }
            None => false,
        }
    }

    fn reset_in(&self, store: &mut Store) {
        store.reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Theme {
        Dark,
        Light,
    }

    impl Choice for Theme {
        const OPTIONS: &'static [(Theme, &'static str)] =
            &[(Theme::Dark, "dark"), (Theme::Light, "light")];
    }

    const BOOLEAN: Kind = Kind::Boolean;
    const WHOLE: Kind = Kind::Number(Number {
        min: None,
        max: None,
        above: Some(0.0),
        step: None,
        decimals: Some(0),
        unit: None,
        factor: None,
    });

    fn read<T: Stored>(kind: &Kind, text: &str) -> (Option<T::Owned>, Vec<String>) {
        let value: Value = format!("v = {text}").parse::<toml::Table>().expect("TOML")["v"].clone();
        let mut warnings = Vec::new();
        (T::read(kind, &value, "s", &mut warnings), warnings)
    }

    #[test]
    fn each_kind_reads_what_it_writes() {
        let scale = Kind::Number(Number {
            min: Some(0.5),
            max: Some(3.0),
            decimals: Some(2),
            ..Number::default()
        });
        assert_eq!(f32::write(&scale, &1.1), Some(Value::Float(1.1)));
        assert_eq!(f32::write(&WHOLE, &1599.6), Some(Value::Integer(1600)));
        assert_eq!(f32::write(&Kind::Text, &2.0), Some(Value::Float(2.0)));
        assert_eq!(read::<f32>(&scale, "2"), (Some(2.0), vec![]));
        assert_eq!(
            read::<Theme>(&Kind::Text, "\"light\""),
            (Some(Theme::Light), vec![])
        );
        assert_eq!(
            Theme::write(&Kind::Text, &Theme::Light),
            Some("light".into())
        );
        let list = Kind::List {
            of: &Kind::Text,
            length: None,
        };
        let one = read::<&'static [&'static str]>(&list, "\"Ctrl+K\"");
        assert_eq!(one, (Some(vec!["Ctrl+K".to_owned()]), vec![]));
        assert_eq!(<Option<bool>>::write(&BOOLEAN, &None), None);
    }

    #[test]
    fn what_cannot_be_read_is_none_and_part_of_it_is_warned_of() {
        let range = Kind::Number(Number {
            min: Some(0.5),
            max: Some(3.0),
            ..Number::default()
        });
        assert_eq!(
            read::<f32>(&range, "9.0"),
            (Some(3.0), vec!["s 9 is out of range; 3 is used".to_owned()])
        );
        assert_eq!(read::<f32>(&WHOLE, "0"), (None, vec![]));
        // A choice's id exactly, unless its type reads ids otherwise.
        assert_eq!(read::<Theme>(&Kind::Text, "\"Light\""), (None, vec![]));
        assert_eq!(read::<f32>(&WHOLE, "1e39"), (None, vec![]));
        let size = Kind::List {
            of: &WHOLE,
            length: Some(2),
        };
        assert_eq!(read::<[f32; 2]>(&size, "[1200]"), (None, vec![]));
        assert_eq!(read::<[f32; 2]>(&size, "[1200, 0]"), (None, vec![]));
        // A list whose items cannot all be read warns of none of them.
        const UP_TO_1: Kind = Kind::Number(Number {
            min: None,
            max: Some(1.0),
            above: None,
            step: None,
            decimals: None,
            unit: None,
            factor: None,
        });
        let ranges = Kind::List {
            of: &UP_TO_1,
            length: None,
        };
        assert_eq!(read::<&'static [f32]>(&ranges, "[2, true]"), (None, vec![]));
        let table = Kind::Table {
            by: "sections",
            of: &BOOLEAN,
        };
        let (sections, warnings) =
            read::<&'static [(&'static str, bool)]>(&table, "{ a = true, b = 1 }");
        assert_eq!(sections, Some(BTreeMap::from([("a".to_owned(), true)])));
        assert_eq!(warnings, ["s.b cannot be 1; the default is used"]);
    }
}
