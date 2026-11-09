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

//! The settings generator (decision 43): reads the settings schema, refuses what is wrong
//! in it, naming the line, and writes one owner's settings as typed statics, which the
//! owner's build script puts in `OUT_DIR` and its code includes. Behind the `codegen`
//! feature, which only build-dependencies turn on.
//!
//! toml_edit reads the schema: it keeps the file's order, where each key is and the
//! comment above each header. The code is plain text: a static per setting, its doc
//! comment the setting's note and its line; a choice's default looked up among its type's
//! options as the static is evaluated, so a default that is none of them stops the build
//! there; and for each choice type, a check that its ids differ.

use std::fmt::{self, Write as _};
use std::ops::Range;

use toml_edit::{Document, InlineTable, Item, Key, Table, TableLike, Value};

/// The schema, in the top-level `data/` (decision 45), embedded, so that a change to it
/// rebuilds this crate and so reruns every owner's build script. The workspace publishes
/// no crate, so the path may leave this crate's folder.
pub const SCHEMA: &str = include_str!("../../../../data/settings.schema.toml");

/// The schema's path from the workspace's root, as errors and the generated docs name it.
pub const SCHEMA_PATH: &str = "data/settings.schema.toml";

/// Where generated code finds this module's types.
const BASE: &str = "::apertrust_base::settings";

/// The name of the list of every setting an owner's code holds, for its store
/// (`Store::open`), which no setting's static may take.
const REGISTRY: &str = "ALL";

/// The registries whose ids may key a table.
const REGISTRIES: [&str; 4] = ["actions", "sections", "modules", "groups"];

/// `applies`' values, each with its `Applies` variant.
const APPLIES: [(&str, &str); 3] = [
    ("at-once", "AtOnce"),
    ("next-photo", "NextPhoto"),
    ("after-restart", "AfterRestart"),
];

/// The Settings dialog's pages, each with its `Page` variant, in the dialog's order: a
/// `page` must be one of them.
const PAGES: [(&str, &str); 9] = [
    ("general", "General"),
    ("import", "Import"),
    ("library", "Library"),
    ("editing", "Editing"),
    ("processing", "Processing"),
    ("storage", "Storage"),
    ("shortcuts", "Shortcuts"),
    ("presets", "Presets"),
    ("ai", "Ai"),
];

/// For an owner's build script: writes the settings `owner` reads (`app`, `engine`,
/// `export` or `module:<id>`) into `OUT_DIR/settings.rs`, or fails the build with each error in the
/// schema, naming its line.
pub fn build(owner: &str) {
    match parse(SCHEMA, SCHEMA_PATH) {
        Ok(schema) => {
            let out = std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR");
            let file = std::path::Path::new(&out).join("settings.rs");
            std::fs::write(&file, schema.generate(owner))
                .unwrap_or_else(|e| panic!("cannot write {}: {e}", file.display()));
        }
        Err(errors) => {
            for error in errors {
                println!("cargo::error={SCHEMA_PATH}:{error}");
            }
        }
    }
}

/// Something wrong in a schema: its line, from 1, and why.
#[derive(Debug, PartialEq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.line, self.message)
    }
}

/// A schema read and checked: each setting's static, in the file's order.
pub struct Schema {
    settings: Vec<Static>,
    path: String,
}

/// One setting's static in generated code.
struct Static {
    owner: String,
    /// Its name in Rust, the id in capitals: `SIDE_PANEL_WIDTH`.
    name: String,
    code: String,
    /// The choice types in it, whose ids the generated code checks.
    types: Vec<String>,
}

/// A kind as declared: what its values are checked against, and its Rust.
struct Spec {
    check: Check,
    /// The type of its values in generated code.
    ty: String,
    /// Its `Kind` in generated code.
    kind: String,
    /// The choice types in it.
    types: Vec<String>,
}

/// What a kind's values are checked against.
enum Check {
    Boolean,
    Number {
        min: Option<f64>,
        max: Option<f64>,
        above: Option<f64>,
    },
    /// The type's path.
    Choice(String),
    Text,
    List {
        of: Box<Spec>,
        length: Option<i64>,
    },
    Table(Box<Spec>),
}

/// Reads and checks a schema; `path` names it in the generated code.
pub fn parse(text: &str, path: &str) -> Result<Schema, Vec<Error>> {
    let reader = Reader { text, path };
    let document = Document::parse(text).map_err(|e| {
        let (line, message) = (
            reader.line(e.span(), 1),
            e.message().lines().collect::<Vec<_>>(),
        );
        vec![Error {
            line,
            message: message.join(": "),
        }]
    })?;
    let (mut settings, mut errors) = (Vec::new(), Vec::new());
    reader.walk(document.as_table(), "", &mut settings, &mut errors);
    let path = path.to_owned();
    if errors.is_empty() {
        Ok(Schema { settings, path })
    } else {
        Err(errors)
    }
}

/// Reads the schema's text.
struct Reader<'a> {
    text: &'a str,
    path: &'a str,
}

impl Reader<'_> {
    /// The line where `span` starts, or `or`.
    fn line(&self, span: Option<Range<usize>>, or: usize) -> usize {
        span.map_or(or, |span| self.text[..span.start].matches('\n').count() + 1)
    }

    /// An error at `field` in `table`, or at `line` when it is missing.
    fn fail(&self, table: &dyn TableLike, field: &str, line: usize, id: &str, why: &str) -> Error {
        let key = table.get_key_value(field).and_then(|(key, _)| key.span());
        Error {
            line: self.line(key, line),
            message: format!("[{id}]: {why}"),
        }
    }

    /// A value as the schema writes it.
    fn source(&self, span: Option<Range<usize>>) -> &str {
        span.map_or("", |span| self.text[span].trim())
    }

    /// Reads the settings under `table`, whose id is `prefix`.
    fn walk(&self, table: &Table, prefix: &str, out: &mut Vec<Static>, errors: &mut Vec<Error>) {
        for (name, item) in table.iter() {
            let id = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}.{name}")
            };
            let line = self.line(table.key(name).and_then(Key::span), 1);
            let fail = |message: String| Error { line, message };
            match item.as_table().filter(|inner| !inner.is_dotted()) {
                _ if !is_snake(name) => errors.push(fail(format!(
                    "[{id}]: each part of an id is snake_case: a to z, 0 to 9 and _, a letter first"
                ))),
                Some(inner) if inner.contains_key("owner") || inner.contains_key("kind") => {
                    match self.setting(&id, inner, line, out.len()) {
                        Ok(setting) => out.push(setting),
                        Err(error) => errors.push(error),
                    }
                }
                Some(inner) if !inner.is_empty() => self.walk(inner, &id, out, errors),
                _ => errors.push(fail(format!(
                    "{id}: not a setting; each setting is a table under its own header, [{id}]"
                ))),
            }
        }
    }

    /// The setting `id`, at `place` in the file, as its static.
    fn setting(&self, id: &str, table: &Table, line: usize, place: usize) -> Result<Static, Error> {
        let fail = |field: &str, why: &str| self.fail(table, field, line, id, why);
        let string = |field| table.get(field).and_then(Item::as_str);
        let owner = string("owner")
            .filter(|owner| is_owner(owner))
            .ok_or_else(|| fail("owner", "owner is app, engine, export or module:<id>"))?;
        let applies = APPLIES
            .into_iter()
            .find(|(name, _)| string("applies") == Some(name))
            .ok_or_else(|| fail("applies", "applies is at-once, next-photo or after-restart"))?
            .1;
        let spec = self.spec(
            id,
            table,
            line,
            &["owner", "applies", "default", "optional", "page", "group"],
        )?;
        let (page, group) = self.place_in_dialog(id, table, line)?;
        let optional = match table.get("optional") {
            None => false,
            Some(item) => item
                .as_bool()
                .ok_or_else(|| fail("optional", "optional is true or false"))?,
        };
        let name = id.replace('.', "_");
        let upper = name.to_uppercase();
        if upper == REGISTRY {
            let why = format!("{REGISTRY} names the list of every setting: choose another id");
            return Err(Error {
                line,
                message: format!("[{id}]: {why}"),
            });
        }
        let (ty, default) = match (table.get("default"), optional) {
            (Some(item), false) if item.as_str() == Some("computed") => {
                (spec.ty, format!("Computed(computed::{name})"))
            }
            (Some(item), false) => {
                let value = item
                    .as_value()
                    .ok_or_else(|| fail("default", "default is a value"))?;
                let at = format!("{}:{line}: [{id}]", self.path);
                let code = self.value(&spec, value, &at);
                let code = code.map_err(|why| fail("default", &format!("default: {why}")))?;
                (spec.ty, format!("Value({code})"))
            }
            (Some(_), true) => return Err(fail("default", "an optional setting has no default")),
            (None, true) => (format!("Option<{}>", spec.ty), "Value(None)".to_owned()),
            (None, false) => {
                let why = "has no default: give a value, \"computed\" or optional = true";
                return Err(fail("default", why));
            }
        };
        let owner_code = match owner.strip_prefix("module:") {
            Some(module) => format!("Module({module:?})"),
            None if owner == "engine" => "Engine".to_owned(),
            None if owner == "export" => "Export".to_owned(),
            None => "App".to_owned(),
        };
        let mut code = self.notes(table);
        let _ = writeln!(
            code,
            "/// `[{id}]`, declared at line {line} of `{path}`.\n\
             pub static {upper}: {BASE}::Setting<{ty}> = {BASE}::Setting {{\n    \
             id: {id:?},\n    \
             owner: {BASE}::Owner::{owner_code},\n    \
             kind: {kind},\n    \
             default: {BASE}::DefaultValue::{default},\n    \
             applies: {BASE}::Applies::{applies},\n    \
             page: {page},\n    \
             group: {group},\n    \
             place: {place},\n    \
             line: {line},\n\
             }};",
            path = self.path,
            kind = spec.kind,
        );
        let (owner, types) = (owner.to_owned(), spec.types);
        Ok(Static {
            owner,
            name: upper,
            code,
            types,
        })
    }

    /// Where the Settings dialog shows the setting `id`: its page's `Page` and its group, as
    /// code, `None` for each where it shows nowhere. A page is one of [`PAGES`] and holds
    /// its rows in groups, so each comes with the other.
    fn place_in_dialog(
        &self,
        id: &str,
        table: &Table,
        line: usize,
    ) -> Result<(String, String), Error> {
        let fail = |field: &str, why: &str| self.fail(table, field, line, id, why);
        let page = match table.get("page") {
            None => None,
            Some(item) => {
                let page = PAGES.iter().find(|(page, _)| item.as_str() == Some(page));
                let pages: Vec<&str> = PAGES.iter().map(|(page, _)| *page).collect();
                let why = format!("page is one of the dialog's: {}", pages.join(", "));
                Some(page.ok_or_else(|| fail("page", &why))?.1)
            }
        };
        let group =
            match table.get("group") {
                None => None,
                Some(item) => Some(item.as_str().filter(|group| is_snake(group)).ok_or_else(
                    || {
                        fail(
                            "group",
                            "group is snake_case: a to z, 0 to 9 and _, a letter first",
                        )
                    },
                )?),
            };
        match (page, group) {
            (Some(page), Some(group)) => Ok((
                format!("Some({BASE}::Page::{page})"),
                format!("Some({group:?})"),
            )),
            (None, None) => Ok(("None".to_owned(), "None".to_owned())),
            (None, Some(_)) => Err(fail("group", "a group is on a page: give its page too")),
            (Some(_), None) => Err(fail(
                "page",
                "a page holds its rows in groups: give its group too",
            )),
        }
    }

    /// The kind `table` declares, with its fields; any other field but `common` is
    /// refused. `line` is where a missing field is reported.
    fn spec(
        &self,
        id: &str,
        table: &dyn TableLike,
        line: usize,
        common: &[&str],
    ) -> Result<Spec, Error> {
        let fail = |field: &str, why: &str| self.fail(table, field, line, id, why);
        let string = |field| table.get(field).and_then(Item::as_str);
        // A number, or a whole one up to `most`.
        let number = |field: &str, most: Option<i64>| match table.get(field) {
            None => Ok(None),
            Some(item) => match (item.as_value().and_then(number), most) {
                (Some(n), None) => Ok(Some(n)),
                (Some(n), Some(most)) if n.fract() == 0.0 && (0.0..=most as f64).contains(&n) => {
                    Ok(Some(n))
                }
                _ => {
                    let what =
                        most.map_or("a number".to_owned(), |most| format!("from 0 to {most}"));
                    Err(fail(
                        field,
                        &format!("{field} is {what}, not {}", self.source(item.span())),
                    ))
                }
            },
        };
        let some = |n: Option<f64>| n.map_or("None".to_owned(), |n| format!("Some({})", float(n)));
        let (check, ty, kind, fields): (Check, String, String, &[&str]) = match string("kind") {
            Some("boolean") => (Check::Boolean, "bool".into(), "Boolean".into(), &[]),
            Some("number") => {
                let (min, max, above, step) = (
                    number("min", None)?,
                    number("max", None)?,
                    number("above", None)?,
                    number("step", None)?,
                );
                let decimals = number("decimals", Some(9))?.map(|n| n as i64);
                let factor = number("factor", None)?;
                if factor.is_some_and(|factor| factor <= 0.0) {
                    return Err(fail("factor", "factor is more than 0"));
                }
                let unit = match table.get("unit") {
                    None => None,
                    Some(item) => {
                        Some(item.as_str().filter(|unit| is_unit(unit)).ok_or_else(|| {
                            fail("unit", "unit is the id of its words: a to z, 0 to 9 and -")
                        })?)
                    }
                };
                match (min, max, above, step) {
                    (Some(min), Some(max), _, _) if min > max => {
                        return Err(fail("min", &format!("min, {min}, is over max, {max}")));
                    }
                    (Some(_), _, Some(_), _) => {
                        return Err(fail("above", "min and above are both minimums: give one"));
                    }
                    (_, Some(max), Some(above), _) if above >= max => {
                        let why = format!("above, {above}, is not under max, {max}");
                        return Err(fail("above", &why));
                    }
                    (_, _, _, Some(step)) if step <= 0.0 => {
                        return Err(fail("step", "step is more than 0"));
                    }
                    _ => {}
                }
                let (min_, max_, above_, step) = (some(min), some(max), some(above), some(step));
                let factor = some(factor);
                let kind = format!(
                    "Number({BASE}::Number {{ min: {min_}, max: {max_}, above: {above_}, step: {step}, decimals: {decimals:?}, unit: {unit:?}, factor: {factor} }})"
                );
                (
                    Check::Number { min, max, above },
                    "f32".into(),
                    kind,
                    &["min", "max", "above", "step", "decimals", "unit", "factor"],
                )
            }
            // A path that is no `Choice` stops the build where its static compiles.
            Some("choice") => match string("type") {
                Some(path) if !path.is_empty() => (
                    Check::Choice(path.into()),
                    path.into(),
                    format!("Choice({path:?})"),
                    &["type"],
                ),
                _ => return Err(fail("type", "a choice names its Rust type: type = \"…\"")),
            },
            Some("text") => (Check::Text, "&'static str".into(), "Text".into(), &[]),
            Some("list") => {
                let of = self.of(id, table, line)?;
                let length = number("length", Some(4096))?.map(|n| n as i64);
                let ty = match length {
                    Some(n) => format!("[{}; {n}]", of.ty),
                    None => format!("&'static [{}]", of.ty),
                };
                let kind = format!("List {{ of: &{}, length: {length:?} }}", of.kind);
                (
                    Check::List {
                        of: Box::new(of),
                        length,
                    },
                    ty,
                    kind,
                    &["of", "length"],
                )
            }
            Some("table") => match string("by") {
                Some(by) if REGISTRIES.contains(&by) => {
                    let of = self.of(id, table, line)?;
                    let ty = format!("{BASE}::Table<{}>", of.ty);
                    let kind = format!("Table {{ by: {by:?}, of: &{} }}", of.kind);
                    (Check::Table(Box::new(of)), ty, kind, &["by", "of"])
                }
                _ => {
                    let by = REGISTRIES.join(", ");
                    return Err(fail("by", &format!("by names the ids that key it: {by}")));
                }
            },
            _ => {
                return Err(fail(
                    "kind",
                    "kind is boolean, number, choice, text, list or table",
                ));
            }
        };
        let known =
            |field: &str| field == "kind" || common.contains(&field) || fields.contains(&field);
        if let Some((field, _)) = table.iter().find(|(field, _)| !known(field)) {
            return Err(fail(field, &format!("unknown field {field}")));
        }
        let types = match &check {
            Check::Choice(path) => vec![path.clone()],
            Check::List { of, .. } | Check::Table(of) => of.types.clone(),
            _ => Vec::new(),
        };
        Ok(Spec {
            check,
            ty,
            kind: format!("{BASE}::Kind::{kind}"),
            types,
        })
    }

    /// The kind of a list's items or a table's values: a kind's name, or an inline table of
    /// `kind` and that kind's fields.
    fn of(&self, id: &str, table: &dyn TableLike, line: usize) -> Result<Spec, Error> {
        let why = "of is its items' kind, by name or as an inline table";
        let error = self.fail(table, "of", line, id, why);
        match table.get("of").and_then(Item::as_value) {
            Some(Value::String(kind)) => {
                let mut named = InlineTable::new();
                named.insert("kind", kind.value().as_str().into());
                self.spec(id, &named, error.line, &[])
            }
            Some(Value::InlineTable(inner)) => self.spec(id, inner, error.line, &[]),
            _ => Err(error),
        }
    }

    /// `value` as Rust code of `spec`'s values, or why it is none; `at` names the setting
    /// in a choice's message.
    fn value(&self, spec: &Spec, value: &Value, at: &str) -> Result<String, String> {
        let source = self.source(value.span());
        let not = |kind: &str| format!("{source} is not {kind}");
        match &spec.check {
            Check::Boolean => value
                .as_bool()
                .map(|b| b.to_string())
                .ok_or_else(|| not("a boolean")),
            Check::Number { min, max, above } => {
                let n = number(value).ok_or_else(|| not("a number"))?;
                match (*min, *max, *above) {
                    (Some(min), _, _) if n < min => Err(format!("{n} is under its minimum, {min}")),
                    (_, Some(max), _) if n > max => Err(format!("{n} is over its maximum, {max}")),
                    (_, _, Some(above)) if n <= above => Err(format!("{n} is not above {above}")),
                    _ => Ok(float(n)),
                }
            }
            Check::Choice(path) => {
                let id = value.as_str().ok_or_else(|| not("a choice's id"))?;
                let why = format!("{at}: {id:?} is none of {path}'s options");
                Ok(format!("{BASE}::option::<{path}>({id:?}, {why:?})"))
            }
            Check::Text => value
                .as_str()
                .map(|text| format!("{text:?}"))
                .ok_or_else(|| not("a text")),
            Check::List { of, length } => {
                let items = value.as_array().ok_or_else(|| not("a list"))?;
                if let Some(n) = *length
                    && items.len() as i64 != n
                {
                    return Err(format!("{source} has not {n} items"));
                }
                let code: Vec<String> = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        self.value(of, item, at)
                            .map_err(|why| format!("item {}: {why}", i + 1))
                    })
                    .collect::<Result<_, _>>()?;
                let code = code.join(", ");
                Ok(if length.is_some() {
                    format!("[{code}]")
                } else {
                    format!("&[{code}]")
                })
            }
            Check::Table(of) => {
                let entries = value
                    .as_inline_table()
                    .ok_or_else(|| not("an inline table"))?;
                let code: Vec<String> = entries
                    .iter()
                    .map(|(key, item)| match self.value(of, item, at) {
                        Ok(code) => Ok(format!("({key:?}, {code})")),
                        Err(why) => Err(format!("{key}: {why}")),
                    })
                    .collect::<Result<_, _>>()?;
                Ok(format!("&[{}]", code.join(", ")))
            }
        }
    }

    /// The comment right above `table`'s header as doc comments, without its `#`, then a
    /// blank one; nothing without a comment.
    fn notes(&self, table: &Table) -> String {
        let prefix = table.decor().prefix().and_then(|p| p.span());
        let lines = prefix
            .map_or("", |span| &self.text[span])
            .lines()
            .rev()
            .map(str::trim);
        let mut notes: Vec<&str> = lines.take_while(|line| line.starts_with('#')).collect();
        notes.reverse();
        let mut code = String::new();
        for note in &notes {
            let note = note.trim_start_matches('#');
            match escape(note.strip_prefix(' ').unwrap_or(note)).trim_end() {
                "" => code.push_str("///\n"),
                note => _ = writeln!(code, "/// {note}"),
            }
        }
        if !notes.is_empty() {
            code.push_str("///\n");
        }
        code
    }
}

/// A finite number that an `f32` holds, whole or not.
fn number(value: &Value) -> Option<f64> {
    let n = value
        .as_float()
        .or_else(|| value.as_integer().map(|i| i as f64))?;
    (n.is_finite() && n.abs() <= f64::from(f32::MAX)).then_some(n)
}

/// `a` to `z`, `0` to `9` and `_`, a letter first.
fn is_snake(id: &str) -> bool {
    id.starts_with(|c: char| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A unit's id, as its words' message names it (`unit-<unit>`): `a` to `z`, `0` to `9` and
/// `-`, a letter first.
fn is_unit(unit: &str) -> bool {
    unit.starts_with(|c: char| c.is_ascii_lowercase())
        && unit
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_owner(owner: &str) -> bool {
    match owner.strip_prefix("module:") {
        Some(module) => is_snake(module),
        None => matches!(owner, "app" | "engine" | "export"),
    }
}

/// A number as an `f32` literal: `1.0`, never `1`.
fn float(n: f64) -> String {
    format!("{n:?}")
}

/// A note as a doc comment's Markdown: brackets and angle brackets outside code taken as
/// themselves, not as links or HTML.
fn escape(note: &str) -> String {
    let mut code = false;
    let mut escaped = String::with_capacity(note.len());
    for c in note.chars() {
        code ^= c == '`';
        if !code && matches!(c, '[' | ']' | '<' | '>') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

impl Schema {
    /// The code of `owner`'s settings, for its build script to write.
    pub fn generate(&self, owner: &str) -> String {
        assert!(
            is_owner(owner),
            "no owner {owner}: one is app, engine, export or module:<id>"
        );
        let mut code = format!(
            "// The settings `{owner}` owns, generated by apertrust-base's settings generator\n\
             // (decision 43) from {}. Never edit it: change the schema.\n",
            self.path
        );
        let mut types: Vec<&str> = Vec::new();
        let mut names = String::new();
        for setting in self
            .settings
            .iter()
            .filter(|setting| setting.owner == owner)
        {
            code.push('\n');
            code.push_str(&setting.code);
            types.extend(setting.types.iter().map(String::as_str));
            let _ = write!(names, "\n    &{},", setting.name);
        }
        let end = if names.is_empty() { "" } else { "\n" };
        let _ = writeln!(
            code,
            "\n/// Every setting `{owner}` owns, in the schema's order: the registry its store reads \
             and writes\n/// (`Store::open`).\n\
             pub static {REGISTRY}: &[&dyn {BASE}::Declared] = &[{names}{end}];"
        );
        types.sort();
        types.dedup();
        for path in types {
            let _ = write!(
                code,
                "\nconst _: () = assert!(\n    {BASE}::distinct::<{path}>(),\n    \
                 \"{path}: two of its options share an id\"\n);\n"
            );
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The errors a schema gives.
    fn refused(schema: &str) -> Vec<String> {
        match parse(schema, "test.toml") {
            Ok(_) => panic!("accepted:\n{schema}"),
            Err(errors) => errors.iter().map(Error::to_string).collect(),
        }
    }

    fn accepted(schema: &str) -> Schema {
        parse(schema, "test.toml").unwrap_or_else(|errors| {
            let errors: Vec<String> = errors.iter().map(Error::to_string).collect();
            panic!("{}", errors.join("\n"))
        })
    }

    /// Each case's one error: `(kind, fields, error)`, the setting's header at line 2 and
    /// its fields from line 5.
    fn each_refused(cases: &[(&str, &str, &str)]) {
        for (kind, fields, error) in cases {
            let schema = format!(
                "# A note.\n[scale]\nowner = \"app\"\nkind = \"{kind}\"\n{fields}applies = \"at-once\"\n"
            );
            assert_eq!(refused(&schema), [*error], "{schema}");
        }
    }

    /// Each setting's id, in the order the generated code has them.
    fn ids(code: &str) -> Vec<&str> {
        let ids = code
            .lines()
            .filter_map(|line| line.strip_prefix("    id: \""));
        ids.map(|id| id.trim_end_matches("\",")).collect()
    }

    #[test]
    fn the_schema_is_read_and_its_ids_are_unique() {
        let schema = parse(SCHEMA, SCHEMA_PATH).unwrap_or_else(|errors| panic!("{errors:?}"));
        let owned = |owner: &str| schema.settings.iter().filter(|s| s.owner == owner).count();
        assert_eq!(
            owned("app") + owned("engine") + owned("export"),
            schema.settings.len()
        );
        let (app, engine) = (schema.generate("app"), schema.generate("engine"));
        let export = schema.generate("export");
        let mut every = [ids(&app), ids(&engine), ids(&export)].concat();
        assert_eq!(
            (every.first(), every.len()),
            (Some(&"theme"), schema.settings.len())
        );
        every.sort();
        every.dedup();
        assert_eq!(every.len(), schema.settings.len());
        // The engine's are its own, and none of the app's.
        assert_eq!(ids(&engine), ["engine.adapter", "engine.memory_budget"]);
        assert!(!app.contains("ENGINE_"), "{app}");
        // The export's are under `export.`, and the others' never (decision 56).
        assert!(ids(&export).iter().all(|id| id.starts_with("export.")));
        assert!(export.contains("Owner::Export,"), "{export}");
        assert!(!app.contains("EXPORT_") && !engine.contains("EXPORT_"));
    }

    #[test]
    fn a_setting_becomes_a_typed_static_that_names_its_line() {
        let schema = accepted(
            "# Settings.\n\n\
             # The interface's size, as `[scale]` keeps it.\n# Its <unit> is none.\n\
             [scale]\nowner = \"app\"\nkind = \"number\"\nmin = 0.5\nmax = 3\nstep = 0.1\n\
             decimals = 2\ndefault = 1\napplies = \"at-once\"\n\n\
             [theme]\nowner = \"app\"\nkind = \"choice\"\ntype = \"apertrust_ui::Builtin\"\n\
             default = \"dark\"\napplies = \"at-once\"\n\n\
             [window.position]\nowner = \"app\"\nkind = \"list\"\n\
             of = { kind = \"number\", decimals = 0 }\nlength = 2\noptional = true\n\
             applies = \"at-once\"\n\n\
             [side_panel.width]\nowner = \"app\"\nkind = \"number\"\ndefault = \"computed\"\n\
             applies = \"after-restart\"\n\n\
             [keys]\nowner = \"app\"\nkind = \"table\"\nby = \"actions\"\n\
             of = { kind = \"list\", of = \"text\" }\ndefault = { undo = [\"Ctrl+Z\"] }\n\
             applies = \"at-once\"\n\n\
             [engine.memory_budget]\nowner = \"engine\"\nkind = \"number\"\nmin = 256\n\
             default = 2048\napplies = \"next-photo\"\n",
        );
        let code = schema.generate("app");
        let s = "::apertrust_base::settings";
        #[rustfmt::skip]
        let expect = [
            format!("/// The interface's size, as `[scale]` keeps it.\n/// Its \\<unit\\> is none.\n///\n/// `[scale]`, declared at line 5 of `test.toml`.\npub static SCALE: {s}::Setting<f32> = "),
            format!("kind: {s}::Kind::Number({s}::Number {{ min: Some(0.5), max: Some(3.0), above: None, step: Some(0.1), decimals: Some(2), unit: None, factor: None }}),\n    default: {s}::DefaultValue::Value(1.0),\n    applies: {s}::Applies::AtOnce,\n    page: None,\n    group: None,\n    place: 0,\n    line: 5,"),
            format!("Value({s}::option::<apertrust_ui::Builtin>(\"dark\", \"test.toml:15: [theme]: \\\"dark\\\" is none of apertrust_ui::Builtin's options\"))"),
            format!("pub static WINDOW_POSITION: {s}::Setting<Option<[f32; 2]>> = "),
            format!("default: {s}::DefaultValue::Value(None),"),
            format!("default: {s}::DefaultValue::Computed(computed::side_panel_width),"),
            format!("applies: {s}::Applies::AfterRestart,"),
            format!("pub static KEYS: {s}::Setting<{s}::Table<&'static [&'static str]>> = "),
            "Value(&[(\"undo\", &[\"Ctrl+Z\"])])".to_owned(),
            format!("kind: {s}::Kind::Table {{ by: \"actions\", of: &{s}::Kind::List {{ of: &{s}::Kind::Text, length: None }} }},"),
            format!("const _: () = assert!(\n    {s}::distinct::<apertrust_ui::Builtin>(),"),
            format!("pub static ALL: &[&dyn {s}::Declared] = &[\n    &SCALE,\n    &THEME,\n    &WINDOW_POSITION,\n    &SIDE_PANEL_WIDTH,\n    &KEYS,\n];\n"),
        ];
        for part in expect {
            assert!(code.contains(&part), "{part}\nin:\n{code}");
        }
        assert!(!code.contains("MEMORY_BUDGET"), "{code}");
        let engine = schema.generate("engine");
        assert!(
            engine.contains("place: 5,")
                && !engine.contains("SCALE")
                && engine.contains("= &[\n    &ENGINE_MEMORY_BUDGET,\n];"),
            "{engine}"
        );
    }

    #[test]
    #[rustfmt::skip]
    fn a_number_above_a_bound_is_more_than_it() {
        let schema = accepted(
            "[window.size]\nowner = \"app\"\nkind = \"list\"\n\
             of = { kind = \"number\", decimals = 0, above = 0 }\nlength = 2\n\
             default = [1600, 1000]\napplies = \"at-once\"\n",
        );
        let s = "::apertrust_base::settings";
        let code = schema.generate("app");
        let kind = format!("of: &{s}::Kind::Number({s}::Number {{ min: None, max: None, above: Some(0.0), step: None, decimals: Some(0), unit: None, factor: None }})");
        assert!(code.contains(&kind), "{code}");
        each_refused(&[
            ("number", "above = 0\ndefault = 0\n", "6: [scale]: default: 0 is not above 0"),
            ("number", "min = 0\nabove = 0\ndefault = 1\n", "6: [scale]: min and above are both minimums: give one"),
            ("number", "max = 1\nabove = 1\ndefault = 1\n", "6: [scale]: above, 1, is not under max, 1"),
        ]);
    }

    #[test]
    #[rustfmt::skip]
    fn a_setting_shown_in_the_dialog_names_its_page_and_group_and_its_unit() {
        let schema = accepted(
            "[engine.memory_budget]\nowner = \"engine\"\nkind = \"number\"\nmin = 256\n\
             decimals = 0\nunit = \"mib\"\nfactor = 1\ndefault = 2048\napplies = \"after-restart\"\n\
             page = \"processing\"\ngroup = \"gpu\"\n",
        );
        let s = "::apertrust_base::settings";
        let code = schema.generate("engine");
        let expect = [
            "decimals: Some(0), unit: Some(\"mib\"), factor: Some(1.0) }),".to_owned(),
            format!("applies: {s}::Applies::AfterRestart,\n    page: Some({s}::Page::Processing),\n    group: Some(\"gpu\"),\n    place: 0,"),
        ];
        for part in expect {
            assert!(code.contains(&part), "{part}\nin:\n{code}");
        }
    }

    /// A page the dialog does not have fails the build, and so does a group without its
    /// page or a page without its group, and a unit or a factor that is none.
    #[test]
    #[rustfmt::skip]
    fn a_page_the_dialog_does_not_have_or_a_group_off_a_page_is_refused() {
        each_refused(&[
            ("boolean", "default = true\npage = \"darkroom\"\ngroup = \"x\"\n", "6: [scale]: page is one of the dialog's: general, import, library, editing, processing, storage, shortcuts, presets, ai"),
            ("boolean", "default = true\ngroup = \"interface\"\n", "6: [scale]: a group is on a page: give its page too"),
            ("boolean", "default = true\npage = \"general\"\n", "6: [scale]: a page holds its rows in groups: give its group too"),
            ("boolean", "default = true\npage = \"general\"\ngroup = \"Inter face\"\n", "7: [scale]: group is snake_case: a to z, 0 to 9 and _, a letter first"),
            ("number", "unit = \"MiB\"\ndefault = 1\n", "5: [scale]: unit is the id of its words: a to z, 0 to 9 and -"),
            ("number", "factor = 0\ndefault = 1\n", "5: [scale]: factor is more than 0"),
            ("boolean", "unit = \"mib\"\ndefault = true\n", "5: [scale]: unknown field unit"),
        ]);
    }

    #[test]
    fn the_registry_s_name_is_no_setting_s() {
        let all =
            "[all]\nowner = \"app\"\nkind = \"boolean\"\ndefault = true\napplies = \"at-once\"\n";
        assert_eq!(
            refused(all),
            ["1: [all]: ALL names the list of every setting: choose another id"]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn an_unknown_field_or_a_wrong_kind_is_refused_at_its_line() {
        each_refused(&[
            ("number", "mni = 0.5\ndefault = 1\n", "5: [scale]: unknown field mni"),
            ("boolean", "min = 0\ndefault = true\n", "5: [scale]: unknown field min"),
            ("list", "of = { kind = \"text\", min = 1 }\noptional = true\n", "5: [scale]: unknown field min"),
            ("colour", "", "4: [scale]: kind is boolean, number, choice, text, list or table"),
            ("number", "min = \"low\"\n", "5: [scale]: min is a number, not \"low\""),
            ("number", "decimals = 1.5\n", "5: [scale]: decimals is from 0 to 9, not 1.5"),
            ("boolean", "default = \"yes\"\n", "5: [scale]: default: \"yes\" is not a boolean"),
            ("choice", "default = \"dark\"\n", "2: [scale]: a choice names its Rust type: type = \"…\""),
            ("table", "by = \"photos\"\nof = \"text\"\n", "5: [scale]: by names the ids that key it: actions, sections, modules, groups"),
            ("list", "of = 3\n", "5: [scale]: of is its items' kind, by name or as an inline table"),
            ("list", "of = \"choice\"\n", "5: [scale]: a choice names its Rust type: type = \"…\""),
        ]);
    }

    #[test]
    #[rustfmt::skip]
    fn a_default_out_of_its_range_is_refused() {
        each_refused(&[
            ("number", "min = 0.5\nmax = 3.0\ndefault = 3.5\n", "7: [scale]: default: 3.5 is over its maximum, 3"),
            ("number", "min = 256\ndefault = 128\n", "6: [scale]: default: 128 is under its minimum, 256"),
            ("number", "default = 1e39\n", "5: [scale]: default: 1e39 is not a number"),
            ("number", "min = 3\nmax = 1\ndefault = 2\n", "5: [scale]: min, 3, is over max, 1"),
            ("list", "of = { kind = \"number\", min = 1 }\nlength = 2\ndefault = [1600, 0]\n", "7: [scale]: default: item 2: 0 is under its minimum, 1"),
            ("list", "of = \"number\"\nlength = 2\ndefault = [1600]\n", "7: [scale]: default: [1600] has not 2 items"),
            ("table", "by = \"actions\"\nof = \"boolean\"\ndefault = { undo = 1 }\n", "7: [scale]: default: undo: 1 is not a boolean"),
        ]);
    }

    #[test]
    #[rustfmt::skip]
    fn what_every_setting_has_is_checked() {
        each_refused(&[
            ("text", "", "2: [scale]: has no default: give a value, \"computed\" or optional = true"),
            ("text", "default = \"\"\noptional = true\n", "5: [scale]: an optional setting has no default"),
            ("text", "optional = 1\n", "5: [scale]: optional is true or false"),
        ]);
        let me = "[s]\nowner = \"me\"\nkind = \"text\"\noptional = true\napplies = \"at-once\"\n";
        assert_eq!(
            refused(me),
            ["2: [s]: owner is app, engine, export or module:<id>"]
        );
        let later = "[s]\nowner = \"app\"\nkind = \"text\"\noptional = true\napplies = \"later\"\n";
        assert_eq!(refused(later), ["5: [s]: applies is at-once, next-photo or after-restart"]);
    }

    /// The build refuses a choice's default that is none of its type's options where its
    /// static is evaluated (`settings::option`); here, that the code asks for it by its id.
    #[test]
    fn a_choice_s_default_is_checked_against_its_type_s_ids_as_it_builds() {
        let schema = "[scale]\nowner = \"app\"\nkind = \"choice\"\ntype = \"apertrust_ui::Builtin\"\n\
                      default = \"solarized\"\napplies = \"at-once\"\n";
        let code = accepted(schema).generate("app");
        let call = "option::<apertrust_ui::Builtin>(\"solarized\", \"test.toml:1: [scale]: \
                    \\\"solarized\\\" is none of apertrust_ui::Builtin's options\")";
        assert!(code.contains(call), "{code}");
    }

    #[test]
    fn a_duplicate_id_or_an_id_out_of_place_is_refused() {
        let one =
            "[scale]\nowner = \"app\"\nkind = \"boolean\"\ndefault = true\napplies = \"at-once\"\n";
        // toml_edit's own words, which its features change: the line and what it found.
        let duplicate = refused(&format!("{one}\n{one}"));
        assert!(
            duplicate.len() == 1
                && duplicate[0].starts_with("7: ")
                && duplicate[0].contains("duplicate key"),
            "{duplicate:?}"
        );
        let errors = refused(
            "[window]\nsize = { owner = \"app\", kind = \"boolean\", applies = \"at-once\" }\n\
             top = 3\nmaximized.owner = \"app\"\n[Window2]\nowner = \"app\"\n[empty]\n",
        );
        let out = "not a setting; each setting is a table under its own header";
        #[rustfmt::skip]
        assert_eq!(errors, [
            format!("2: window.size: {out}, [window.size]"),
            format!("3: window.top: {out}, [window.top]"),
            format!("4: window.maximized: {out}, [window.maximized]"),
            "5: [Window2]: each part of an id is snake_case: a to z, 0 to 9 and _, a letter first".to_owned(),
            format!("7: empty: {out}, [empty]"),
        ]);
    }

    #[test]
    fn the_settings_keep_the_file_s_order_their_lines_and_their_notes() {
        let text = |id: &str, default: &str| {
            format!(
                "[{id}]\nowner = \"app\"\nkind = \"text\"\ndefault = {default}\napplies = \"at-once\"\n"
            )
        };
        // A value over three lines first: the lines after it count them.
        let lines = text("b", "\"\"\"\nover\nlines\"\"\"");
        let schema = accepted(&format!(
            "{lines}\n# About a.c.\n#\n#   indented\n{}{}",
            text("a.c", "\"\""),
            text("a.a", "\"\"")
        ));
        let code = schema.generate("app");
        assert_eq!(ids(&code), ["b", "a.c", "a.a"]);
        #[rustfmt::skip]
        let expect = [
            "\n/// `[b]`, declared at line 1 of `test.toml`.\npub static B:",
            "\n/// About a.c.\n///\n///   indented\n///\n/// `[a.c]`, declared at line 12 of `test.toml`.\n",
            "\n/// `[a.a]`, declared at line 17 of `test.toml`.\npub static A_A:",
        ];
        for part in expect {
            assert!(code.contains(part), "{part}\nin:\n{code}");
        }
        let later = lines.replace("at-once", "later");
        assert_eq!(
            refused(&later),
            ["7: [b]: applies is at-once, next-photo or after-restart"]
        );
    }
}
