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

//! Settings as the schema declares them (architecture decision 43; `docs/settings.md`).
//!
//! Every setting is declared once, in `data/settings.schema.toml` (decision 45), a TOML
//! table whose header is the setting's id, the dotted path of its key in `settings.toml`.
//! Each crate that owns settings generates its own from the schema in its build script,
//! with the generator, `settings::codegen`, which only the `codegen` feature builds and
//! only build-dependencies turn on, as typed statics, the settings registry:
//!
//! ```ignore
//! // build.rs
//! apertrust_base::settings::codegen::build("app");
//!
//! // src/settings.rs
//! pub mod registry {
//!     include!(concat!(env!("OUT_DIR"), "/settings.rs"));
//!
//!     /// The defaults the schema computes, each under its setting's name.
//!     mod computed {
//!         pub fn locale() -> apertrust_ui::Locale { /* the system's */ }
//!     }
//! }
//!
//! let scale: f32 = registry::SCALE.default();
//! ```
//!
//! The [`Store`] reads `settings.toml` for a registry, each owner's generated `ALL`, and
//! hands out each setting's value typed by its static, owned for text, lists and tables;
//! it writes what changes on a thread of its own, merged into the file as it is then, with
//! the file's rules, which [`Store`] lists. It reaches the file through [`Storage`],
//! [`FileStorage`] on the desktop, so the browser build can keep settings in its own:
//!
//! ```ignore
//! let mut store = Store::open(Box::new(FileStorage::new(path)), registry::ALL);
//! let theme: Builtin = store.get(&registry::THEME);
//! store.set(&registry::KEYS, keys); // a BTreeMap<String, Vec<String>>
//! store.poll(Instant::now());       // after each frame: written once still for a second
//! ```
//!
//! A program that reads settings and keeps none, as the CLI, opens them with
//! [`Store::read_only`], which borrows the storage and so never writes it. A program that
//! reads several owners' settings opens one store on their registries together, as the
//! app reads its own and the engine's.
//!
//! `--conf KEY=VALUE`, darktable's, sets a value for one run: a program reads its
//! [`Overrides`] against the same registry before anything else, an unknown key or a value
//! that cannot be read stopping its start, and its store holds them over the file, never
//! writing them:
//!
//! ```ignore
//! let overrides = Overrides::read(&conf, registry::ALL)?; // ["scale=1.5"]
//! let store = Store::open(storage, registry::ALL).with_overrides(&overrides);
//! assert!(store.overridden(&registry::SCALE));
//! ```
//!
//! A choice names its Rust type in the schema, which implements [`Choice`]: the type gives
//! the options, so none is listed twice, and a default that is none of them fails the
//! build where the static is generated:
//!
//! ```compile_fail
//! use apertrust_base::settings::{Choice, option};
//!
//! #[derive(Clone, Copy)]
//! enum Theme { Dark, Light }
//!
//! impl Choice for Theme {
//!     const OPTIONS: &'static [(Theme, &'static str)] =
//!         &[(Theme::Dark, "dark"), (Theme::Light, "light")];
//! }
//!
//! // "solarized" is none of Theme's options: the constant cannot be evaluated.
//! static DEFAULT: Theme = option::<Theme>("solarized", "no such theme");
//! ```

#[cfg(any(feature = "codegen", test))]
pub mod codegen;
mod overrides;
mod storage;
mod store;
mod stored;

use std::ops::RangeInclusive;

pub use overrides::{CONF_FLAG, OverrideError, Overrides};
pub use storage::{FileStorage, Storage};
pub use store::{DELAY, MIGRATIONS, Migration, Store, VERSION};
pub use stored::{Declared, Stored};

/// One setting as the schema declares it, typed by its value, `T`.
///
/// `T` is the kind's: `bool` for a boolean, `f32` for a number, the named type for a
/// choice, `&'static str` for text, an array for a list of a set length and a slice
/// otherwise, a [`Table`] for a table; and `Option` of it for an optional setting, which
/// has no value until its owner keeps one.
#[derive(Clone, Copy, Debug)]
pub struct Setting<T: 'static> {
    /// The dotted path of its key in `settings.toml`, permanent: `side_panel.width`.
    pub id: &'static str,
    pub owner: Owner,
    pub kind: Kind,
    pub default: DefaultValue<T>,
    pub applies: Applies,
    /// The Settings dialog's page that shows it, or None for what its owner keeps by
    /// itself and never shows, as the window's place.
    pub page: Option<Page>,
    /// The titled group of its page it sits in, by id (`settings-group-<group>` its title);
    /// None exactly when it has no page.
    pub group: Option<&'static str>,
    /// Its place among every owner's settings, in the schema's order, which the Settings
    /// dialog follows.
    pub place: u32,
    /// The line of its header in the schema.
    pub line: u32,
}

impl<T: Copy> Setting<T> {
    /// Its default: the schema's value, or what its owner computes.
    pub fn default(&self) -> T {
        match self.default {
            DefaultValue::Value(value) => value,
            DefaultValue::Computed(compute) => compute(),
        }
    }
}

impl Setting<f32> {
    /// The values a number may take, from its minimum to its maximum; an end the schema
    /// leaves open is the largest `f32` there. Its `above`, if it has one, is not in it
    /// ([`Number::range`]).
    pub const fn range(&self) -> RangeInclusive<f32> {
        match self.kind {
            Kind::Number(number) => number.range(),
            _ => f32::MIN..=f32::MAX,
        }
    }
}

/// A table setting's value as the schema gives it: each key, an id of the registry that
/// keys the table, with its value, in the schema's order.
pub type Table<V> = &'static [(&'static str, V)];

/// A setting's default.
#[derive(Clone, Copy, Debug)]
pub enum DefaultValue<T: 'static> {
    /// A value of its kind, the schema's; None for an optional setting.
    Value(T),
    /// Computed in Rust by its owner, by the function the schema's `default = "computed"`
    /// names after the setting: `computed::locale` for `locale`, the language that follows
    /// the system's.
    Computed(fn() -> T),
}

/// Who reads a setting, and so whose build script generates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// The desktop app alone.
    App,
    /// Every process that runs the engine: the app, the CLI, later the server.
    Engine,
    /// The Export dialog's last settings (decision 56), generated by `apertrust-export`:
    /// the app keeps them as darktable keeps its conf, and the CLI's `export` reads them
    /// as darktable-cli reads darktablerc.
    Export,
    /// A module's tool setting, by the module's id (decision 29), kept by the client and
    /// sent with the command that runs the tool.
    Module(&'static str),
}

/// A setting's kind, with what the kind holds. Whole numbers and folders come with the
/// first setting that needs one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Boolean,
    Number(Number),
    /// One of a type's options, by the type's path in the schema (`apertrust_ui::Builtin`).
    Choice(&'static str),
    Text,
    /// A list of values of one kind, of a set length or any.
    List {
        of: &'static Kind,
        length: Option<u32>,
    },
    /// Values of one kind keyed by another registry's ids: `actions`, `sections`,
    /// `modules` or `groups`.
    Table {
        by: &'static str,
        of: &'static Kind,
    },
}

/// A number's range, step and decimals.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Number {
    pub min: Option<f32>,
    pub max: Option<f32>,
    /// What it must be more than, an exclusive minimum, as a size's parts are more than 0:
    /// a value that is not cannot be read, where one under `min` is read as `min`.
    pub above: Option<f32>,
    /// What one step of a control changes it by.
    pub step: Option<f32>,
    /// How many decimals it is kept with; all of them when None.
    pub decimals: Option<u32>,
    /// The unit it is shown in, by the id of its words (`unit-<unit>`, as `mib`); None for
    /// a number with no unit.
    pub unit: Option<&'static str>,
    /// What it is multiplied by where it is shown, as darktable's slider factor: 100 shows
    /// a scale of 1.1 as 110 %. None shows it as it is kept.
    pub factor: Option<f32>,
}

impl Number {
    /// From its minimum to its maximum, an open end the largest `f32` there. It leaves out
    /// `above`, which no range holds: a number with one has its lower end open.
    pub const fn range(&self) -> RangeInclusive<f32> {
        let min = match self.min {
            Some(min) => min,
            None => f32::MIN,
        };
        let max = match self.max {
            Some(max) => max,
            None => f32::MAX,
        };
        min..=max
    }
}

/// The Settings dialog's pages, in its list's order (`docs/gui.md`, The Settings dialog):
/// a setting's `page` in the schema is one of their ids, and the generator refuses any
/// other. A page that holds nothing yet is listed dimmed and cannot be chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    General,
    Import,
    Library,
    Editing,
    Processing,
    Storage,
    Shortcuts,
    Presets,
    Ai,
}

impl Page {
    pub const ALL: [Page; 9] = [
        Page::General,
        Page::Import,
        Page::Library,
        Page::Editing,
        Page::Processing,
        Page::Storage,
        Page::Shortcuts,
        Page::Presets,
        Page::Ai,
    ];

    /// Its id in the schema and in its words, `settings-page-<id>`.
    pub const fn id(self) -> &'static str {
        match self {
            Page::General => "general",
            Page::Import => "import",
            Page::Library => "library",
            Page::Editing => "editing",
            Page::Processing => "processing",
            Page::Storage => "storage",
            Page::Shortcuts => "shortcuts",
            Page::Presets => "presets",
            Page::Ai => "ai",
        }
    }
}

/// The page the Settings dialog last showed (`settings_window.page`) is kept by its id.
impl Choice for Page {
    const OPTIONS: &'static [(Page, &'static str)] = &[
        (Page::General, Page::General.id()),
        (Page::Import, Page::Import.id()),
        (Page::Library, Page::Library.id()),
        (Page::Editing, Page::Editing.id()),
        (Page::Processing, Page::Processing.id()),
        (Page::Storage, Page::Storage.id()),
        (Page::Shortcuts, Page::Shortcuts.id()),
        (Page::Presets, Page::Presets.id()),
        (Page::Ai, Page::Ai.id()),
    ];
}

/// When a change to a setting takes effect, which the Settings dialog says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applies {
    AtOnce,
    NextPhoto,
    AfterRestart,
}

/// A type whose values a choice setting takes: each value with its stable id, the one
/// `settings.toml` keeps, every value once, in the order a list of them shows.
///
/// The ids must differ: the code generated for a choice checks them as it builds.
pub trait Choice: Copy + 'static {
    const OPTIONS: &'static [(Self, &'static str)];

    /// The option `id` names, as the store reads it from `settings.toml`: the one whose id
    /// it is exactly. A type whose ids ignore case, as the language's BCP 47 tags do,
    /// reads them so.
    fn from_id(id: &str) -> Option<Self> {
        let mut options = Self::OPTIONS.iter();
        options
            .find(|(_, option)| *option == id)
            .map(|(value, _)| *value)
    }
}

/// The option of `T` whose id is `id`; for a choice's default in generated code, where a
/// default that is none of them stops the build, saying `why`.
pub const fn option<T: Choice>(id: &str, why: &str) -> T {
    let mut i = 0;
    while i < T::OPTIONS.len() {
        if same(T::OPTIONS[i].1, id) {
            return T::OPTIONS[i].0;
        }
        i += 1;
    }
    panic!("{}", why)
}

/// Whether every option of `T` has an id of its own.
pub const fn distinct<T: Choice>() -> bool {
    let options = T::OPTIONS;
    let mut i = 0;
    while i < options.len() {
        let mut j = i + 1;
        while j < options.len() {
            if same(options[i].1, options[j].1) {
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

/// Whether two ids are one; `str`'s own comparison is not `const`.
const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
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

    /// Two options under one id.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Twice {
        One,
        Other,
    }

    impl Choice for Twice {
        const OPTIONS: &'static [(Twice, &'static str)] =
            &[(Twice::One, "same"), (Twice::Other, "same")];
    }

    #[test]
    fn a_choice_s_default_is_one_of_its_options_by_id() {
        const LIGHT: Theme = option::<Theme>("light", "no such theme");
        assert_eq!(LIGHT, Theme::Light);
    }

    /// At run time; in a static, as generated code has it, the same stops the build (the
    /// module's `compile_fail` example).
    #[test]
    #[should_panic(expected = "settings.schema.toml:3: [theme]: solarized is no option")]
    fn a_choice_s_default_that_is_none_of_its_options_is_refused() {
        option::<Theme>(
            "solarized",
            "settings.schema.toml:3: [theme]: solarized is no option",
        );
    }

    #[test]
    fn options_sharing_an_id_are_found() {
        assert!(distinct::<Theme>());
        assert!(!distinct::<Twice>());
        assert!(distinct::<Page>());
    }

    #[test]
    fn the_pages_are_listed_once_each_by_their_ids() {
        let ids: Vec<&str> = Page::ALL.iter().map(|page| page.id()).collect();
        let options: Vec<&str> = Page::OPTIONS.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, options);
        assert_eq!(Page::from_id("ai"), Some(Page::Ai));
    }

    #[test]
    fn a_setting_gives_its_default_and_its_range() {
        static SCALE: Setting<f32> = Setting {
            id: "scale",
            owner: Owner::App,
            kind: Kind::Number(Number {
                min: Some(0.5),
                max: Some(3.0),
                above: None,
                step: Some(0.1),
                decimals: Some(2),
                unit: Some("percent"),
                factor: Some(100.0),
            }),
            default: DefaultValue::Value(1.0),
            applies: Applies::AtOnce,
            page: Some(Page::General),
            group: Some("interface"),
            place: 0,
            line: 1,
        };
        // As a constant, which the app's ranges are.
        const SCALES: RangeInclusive<f32> = SCALE.range();
        fn computed() -> Theme {
            Theme::Light
        }
        static THEME: Setting<Theme> = Setting {
            id: "theme",
            owner: Owner::App,
            kind: Kind::Choice("Theme"),
            default: DefaultValue::Computed(computed),
            applies: Applies::AtOnce,
            page: None,
            group: None,
            place: 1,
            line: 10,
        };
        assert_eq!(SCALE.default(), 1.0);
        assert_eq!(SCALES, 0.5..=3.0);
        assert_eq!(THEME.default(), Theme::Light);
    }
}
