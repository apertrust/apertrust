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

//! The settings store (decision 43; `docs/gui.md`, Settings and workspace): [`Store`], its
//! writer thread, and the file's version and migrations.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet, VecDeque, btree_map};
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use toml::{Table, Value};

use super::stored::unreadable;
use super::{Declared, DefaultValue, Kind, Overrides, Setting, Storage, Stored};

/// The version this build writes, and the newest it changes: the first, 1, and one more
/// for each migration.
pub const VERSION: i64 = 1 + MIGRATIONS.len() as i64;

/// How long the settings stand still before they are written.
pub const DELAY: Duration = Duration::from_secs(1);

/// A change to what `settings.toml` holds, a key that changes meaning or moves, from one
/// version to the next, made on the file's table before it is read and before a write
/// merges into it. Adding a setting needs none: a missing key is its default.
pub struct Migration {
    /// The version it migrates from, to the next.
    pub from: i64,
    pub migrate: fn(&mut Table),
}

/// Every migration, in order from version 1: none yet.
pub const MIGRATIONS: &[Migration] = &[];

/// Migrates `table`, of `version`, through the `migrations` from that version on.
fn migrate(table: &mut Table, version: i64, migrations: &[Migration]) {
    for migration in migrations.iter().filter(|m| m.from >= version) {
        (migration.migrate)(table);
        tracing::debug!(
            target: "settings",
            "migrated version {} to {}",
            migration.from,
            migration.from + 1
        );
    }
}

/// The settings of a registry, read once from where they are stored and written back on a
/// thread of its own once they have stood still for [`DELAY`], and on exit.
///
/// [`get`](Store::get) and [`set`](Store::set) take a setting's static, so their values are
/// typed by it, owned for text, lists and tables: `store.get(&SCALE)` is an `f32`,
/// `store.get(&KEYS)` a `BTreeMap<String, Vec<String>>`.
///
/// **Reading** happens once, as the store opens, for the settings of the registry it is
/// given (an owner's generated `ALL`). A missing key is its default. A value that cannot be
/// read gets its default, with a warning, and stays in the file until that setting is
/// changed; a number out of range is read as the nearer end, with a warning; a table's
/// entry that cannot be read is left out the same way. Keys no setting reads stay too, and
/// `-d settings` names them. A file that is not TOML, or not text, is set aside as
/// `settings.toml.bak` and the defaults are used. A newer version's file is read for what
/// this build knows and never overwritten; one that cannot be read at all, as when its
/// permissions forbid it, gives the defaults and is never overwritten either, nor is a
/// read-only one, which is read, with one warning. An older version's is migrated before
/// it is read ([`MIGRATIONS`]). First, what writes left as their process ended is removed
/// ([`Storage::tidy`]).
///
/// **Writing** happens on a thread of its own, as the engine's edits are written (decision
/// 28), so a slow disk never delays a frame, once the settings have stood still for
/// [`DELAY`] ([`Store::poll`]) and on exit ([`Store::flush`], or the store's drop). It reads
/// the file again and changes only the settings that changed since the last write, a
/// table's entries one by one, so a value this build cannot read, a key it does not know
/// and what another running Apertrust or a person changed meanwhile all stay; two that
/// change one setting leave the later's. A file damaged since the last write is set aside
/// and written as that write left it. [`FileStorage`](super::FileStorage) writes a
/// temporary file of its own with the file's permissions, syncs it and renames it into
/// place, following a symbolic link. A write that fails is tried again with the next
/// change and on exit. Nothing is
/// written while the settings are as the file has them, so opening and closing leaves the
/// file as it was.
///
/// **A store that never writes**, [`Store::read_only`], reads by the same rules but
/// changes nothing where the settings are stored, for a program that reads settings and
/// keeps none, as the CLI (decision 43).
///
/// **Values for one run**, `--conf KEY=VALUE` ([`Overrides`]), are held over the file once
/// the store opens ([`Store::with_overrides`]): each wins over the file and the default,
/// is never written, and is reported as overridden, so the Settings dialog draws its row
/// disabled. A set or a reset of one changes nothing.
pub struct Store {
    /// Each setting of the registry, by id.
    entries: BTreeMap<&'static str, Entry>,
    /// Where the settings are stored, for the log; None for nowhere.
    location: Option<String>,
    /// What reading could not use, and why.
    warnings: Vec<String>,
    /// Whether a value changed since the last [`poll`](Store::poll).
    dirty: bool,
    /// Since when the values have changed unwritten.
    changed: Option<Instant>,
    /// None where nothing may be written: a newer version's file, one that cannot be read,
    /// or no storage at all.
    writer: Option<Writer>,
}

/// One setting in the store.
struct Entry {
    setting: &'static dyn Declared,
    /// Its value, as its static's `Owned`; None while nothing is kept, so it is its
    /// default: none in the file, one that cannot be read, or an optional setting's None.
    value: Option<Box<dyn Any + Send>>,
    /// Its value as the file holds it since the last write, or as read; None for none.
    written: Option<Value>,
    /// Its value for this run alone, `--conf`'s, which wins over `value` and is never
    /// written; None for none.
    overridden: Option<Box<dyn Any + Send>>,
}

impl Store {
    /// Reads the settings `registry` lists from `storage`, logging what it could not use,
    /// and starts the writer unless the storage must not be written.
    pub fn open(mut storage: Box<dyn Storage>, registry: &[&'static dyn Declared]) -> Store {
        let location = storage.location();
        let removed = storage.tidy();
        if removed > 0 {
            let name = file_name(&location);
            tracing::debug!(target: "settings", "{name}: removed {removed} stale temporary files");
        }
        let loaded = load(storage.as_mut(), registry);
        report(&location, &loaded, true);
        let writer = loaded.writable.then(|| Writer::start(storage, loaded.base));
        Store {
            entries: loaded.entries,
            location: Some(location),
            warnings: loaded.warnings,
            dirty: false,
            changed: None,
            writer,
        }
    }

    /// Reads the settings `registry` lists from `storage` by the same rules as
    /// [`open`](Store::open), and never writes: for a program that reads settings and
    /// keeps none, as the CLI (decision 43). It borrows the storage, so it cannot write
    /// it, set a damaged file aside or remove what other writes left; a damaged file gives
    /// the defaults and stays as it is. What is set is kept for the run alone and never
    /// written.
    pub fn read_only(storage: &dyn Storage, registry: &[&'static dyn Declared]) -> Store {
        let location = storage.location();
        let loaded = match look(storage, registry, false) {
            Ok(loaded) => loaded,
            Err(why) => {
                let why = format!("{why}; defaults used, left as it is");
                Loaded::defaults(registry, Some(why), false)
            }
        };
        report(&location, &loaded, false);
        Store {
            entries: loaded.entries,
            location: Some(location),
            warnings: loaded.warnings,
            dirty: false,
            changed: None,
            writer: None,
        }
    }

    /// A store with nowhere to keep anything, for a system that gives no folder for
    /// settings: the defaults, never written.
    pub fn nowhere(registry: &[&'static dyn Declared]) -> Store {
        Store {
            entries: entries(registry, |_| None),
            location: None,
            warnings: Vec::new(),
            dirty: false,
            changed: None,
            writer: None,
        }
    }

    /// Where the settings are stored, as the log names it; None for nowhere.
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// What reading could not use, and why, each as the log has it: the setting's id
    /// first.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Holds `overrides`, `--conf`'s values, over what the store read, for this run alone:
    /// [`get`](Store::get) and [`kept`](Store::kept) give each in place of the file's or
    /// the default, [`set`](Store::set) and [`reset`](Store::reset) of one change nothing,
    /// and nothing of them is written, so the file keeps its own values as they are. Each
    /// must be one of the store's settings: `overrides` are read against its registry.
    pub fn with_overrides(mut self, overrides: &Overrides) -> Store {
        for (id, value) in overrides.given() {
            let entry = self.entries.get_mut(id);
            let entry = entry.unwrap_or_else(|| unregistered(id));
            let read = entry.setting.read(value, &mut Vec::new());
            let read = read.unwrap_or_else(|| panic!("settings: {id}'s --conf was read before"));
            entry.overridden = Some(read);
        }
        self
    }

    /// Whether `--conf` sets `setting` for this run
    /// ([`with_overrides`](Store::with_overrides)): its value is then that one, which a set
    /// does not change, and the Settings dialog draws its row disabled.
    pub fn overridden(&self, setting: &dyn Declared) -> bool {
        let entry = self.entries.get(setting.id());
        let entry = entry.unwrap_or_else(|| unregistered(setting.id()));
        entry.overridden.is_some()
    }

    /// `setting`'s value: `--conf`'s for this run, else as kept, else its default.
    ///
    /// For a computed default, the value it is computed as now: `set(get())` then keeps
    /// it, so the setting no longer follows what it is computed from. A caller sets such a
    /// setting only on the user's action, as the app's dialog keeps the language chosen,
    /// and otherwise reads [`kept`](Store::kept).
    pub fn get<T: Stored>(&self, setting: &Setting<T>) -> T::Owned {
        self.kept(setting)
            .unwrap_or_else(|| setting.default().owned())
    }

    /// `setting`'s value as kept, or None while it is its default: none in the file, or
    /// one that cannot be read, which then stays there until the setting is set. One
    /// `--conf` gives for this run is kept for it.
    pub fn kept<T: Stored>(&self, setting: &Setting<T>) -> Option<T::Owned> {
        let entry = self.entry(setting);
        let value = entry.overridden.as_deref().or(entry.value.as_deref())?;
        Some(owned(setting, value).clone())
    }

    /// Keeps `value` for `setting`, written once the settings have stood still.
    ///
    /// A set that leaves the value as it is changes nothing: one equal to the value kept,
    /// or, while none is, to the schema's default. One back to the schema's default keeps
    /// nothing where the file holds none, so a change undone before it is written leaves
    /// the file as it is; where the file holds one, the default is written.
    ///
    /// A computed default is no value: it follows what it is computed from (the system's
    /// language, the theme), so a set keeps what it is given even when it is the computed
    /// one, and [`reset`](Store::reset) goes back to following it.
    ///
    /// A set checks no range: the caller's value is kept as it is, as a control within
    /// the setting's range gives it. Only what is read is clamped.
    ///
    /// A set of a setting `--conf` gives is refused: its value stays `--conf`'s for the
    /// run, as darktable keeps its override's (`_conf_set_if_not_overridden`), and
    /// the file keeps its own, where darktable would write the value set for later runs.
    pub fn set<T: Stored>(&mut self, setting: &Setting<T>, value: T::Owned) {
        let entry = self.entry_mut(setting);
        if entry.overridden.is_some() {
            return;
        }
        let unchanged = match (entry.value.as_deref(), setting.default) {
            (Some(kept), _) => *owned(setting, kept) == value,
            (None, DefaultValue::Value(default)) => default.owned() == value,
            (None, DefaultValue::Computed(_)) => false,
        };
        if unchanged {
            return;
        }
        // An optional setting's None keeps nothing, nor does the schema's default where the
        // file holds none: changed and changed back, the file stays as it is.
        let default = matches!(setting.default, DefaultValue::Value(d) if d.owned() == value);
        let none = T::write(&setting.kind, &value).is_none() || default && entry.written.is_none();
        entry.value = (!none).then(|| Box::new(value) as Box<dyn Any + Send>);
        self.dirty = true;
    }

    /// Keeps nothing for `setting`, so it is its default again, and leaves the file
    /// without it once written. One that is its default already changes nothing, and so a
    /// value the file holds that cannot be read, which is not kept, stays there. A reset of
    /// a setting `--conf` gives is refused, as a set is.
    pub fn reset<T: Stored>(&mut self, setting: &Setting<T>) {
        let entry = self.entry_mut(setting);
        if entry.overridden.is_none() && entry.value.take().is_some() {
            self.dirty = true;
        }
    }

    /// `setting`'s value as [`get`](Store::get) gives it, as `settings.toml` keeps it,
    /// for a client that shows any setting without its type, as the Settings dialog draws
    /// its rows; None for an optional setting's none.
    pub fn value(&self, setting: &dyn Declared) -> Option<Value> {
        setting.value_in(self)
    }

    /// Sets `setting` to `value`, read by its kind as `settings.toml`'s is, then kept as
    /// [`set`](Store::set) keeps it. Returns false, changing nothing, for a value its kind
    /// cannot read.
    pub fn set_value(&mut self, setting: &dyn Declared, value: &Value) -> bool {
        setting.set_in(self, value)
    }

    /// Resets `setting` as [`reset`](Store::reset) does.
    pub fn reset_value(&mut self, setting: &dyn Declared) {
        setting.reset_in(self);
    }

    /// Looks at the settings at `now`: hands what changed to the writer once they have
    /// stood still for [`DELAY`], and otherwise says how long until they will have, for a
    /// look then. A client calls it after each frame's sets.
    pub fn poll(&mut self, now: Instant) -> Option<Duration> {
        if std::mem::take(&mut self.dirty) {
            self.changed = Some(now);
        }
        let still = now.saturating_duration_since(self.changed?);
        if still < DELAY {
            return Some(DELAY - still);
        }
        self.take();
        None
    }

    /// On exit: hands over what changed since the last write, waits for the writer to
    /// finish, a failed write tried once more, and ends it. Nothing is written after.
    pub fn flush(&mut self) {
        self.take();
        self.writer = None;
    }

    /// Hands the writer what changed since the last write, if anything did.
    fn take(&mut self) {
        self.changed = None;
        let mut job = Job::new();
        // `--conf`'s values are never written, and the file keeps its own.
        let entries = self.entries.iter_mut();
        for (id, entry) in entries.filter(|(_, entry)| entry.overridden.is_none()) {
            let now = entry
                .value
                .as_deref()
                .and_then(|value| entry.setting.write(value));
            if now != entry.written {
                let mut path = id.split('.').map(str::to_owned).collect();
                let kind = entry.setting.kind();
                diff(
                    kind,
                    &mut path,
                    entry.written.as_ref(),
                    now.as_ref(),
                    &mut job,
                );
                entry.written = now;
            }
        }
        if let Some(writer) = &self.writer
            && !job.is_empty()
        {
            writer.send(job);
        }
    }

    /// `setting`'s entry, which must be of its type: a static of another under its id
    /// would otherwise keep what no writing can read, and so drop the setting from the
    /// file.
    fn entry<T: Stored>(&self, setting: &Setting<T>) -> &Entry {
        let entry = self.entries.get(setting.id);
        let entry = entry.unwrap_or_else(|| unregistered(setting.id));
        typed(setting, entry);
        entry
    }

    fn entry_mut<T: Stored>(&mut self, setting: &Setting<T>) -> &mut Entry {
        let entry = self.entries.get_mut(setting.id);
        let entry = entry.unwrap_or_else(|| unregistered(setting.id));
        typed(setting, entry);
        entry
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        self.flush();
    }
}

fn unregistered(id: &str) -> ! {
    panic!("settings: {id} is in no registry this store was opened with")
}

/// Checks that `entry`'s values are `setting`'s type.
fn typed<T: Stored>(setting: &Setting<T>, entry: &Entry) {
    if entry.setting.value_type() != TypeId::of::<T::Owned>() {
        panic!("settings: {} holds a value of another type", setting.id);
    }
}

/// A value of the store as `setting`'s `Owned`.
fn owned<'a, T: Stored>(setting: &Setting<T>, value: &'a (dyn Any + Send)) -> &'a T::Owned {
    value
        .downcast_ref()
        .unwrap_or_else(|| panic!("settings: {} holds a value of another type", setting.id))
}

/// The store's entries for `registry`, each with the value `read` gives it.
fn entries(
    registry: &[&'static dyn Declared],
    mut read: impl FnMut(&'static dyn Declared) -> Option<Box<dyn Any + Send>>,
) -> BTreeMap<&'static str, Entry> {
    let mut entries = BTreeMap::new();
    for &setting in registry {
        let value = read(setting);
        let written = value.as_deref().and_then(|value| setting.write(value));
        let entry = Entry {
            setting,
            value,
            written,
            overridden: None,
        };
        entries.insert(setting.id(), entry);
    }
    entries
}

/// The file's name in a storage's location, for a debug line.
fn file_name(location: &str) -> std::borrow::Cow<'_, str> {
    let path = Path::new(location);
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
}

/// What changed in the settings since the last write: each value's path in the file, with
/// the value it had then and has now, None for none.
type Job = BTreeMap<Vec<String>, Change>;

#[derive(Clone, Debug, PartialEq)]
struct Change {
    before: Option<Value>,
    after: Option<Value>,
}

/// Puts in `job` the changes of a setting at `path`, of `kind`, from `before` to `after`:
/// a table's entries one by one, so its other entries, another Apertrust's and those this
/// build cannot read, stay; any other value whole.
fn diff(
    kind: &Kind,
    path: &mut Vec<String>,
    before: Option<&Value>,
    after: Option<&Value>,
    job: &mut Job,
) {
    let none = Table::new();
    let old = before.map_or(Some(&none), Value::as_table);
    let new = after.map_or(Some(&none), Value::as_table);
    if let (Kind::Table { of, .. }, Some(old), Some(new)) = (kind, old, new) {
        let keys: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
        for key in keys {
            path.push(key.clone());
            diff(of, path, old.get(key), new.get(key), job);
            path.pop();
        }
    } else if before != after {
        let change = Change {
            before: before.cloned(),
            after: after.cloned(),
        };
        job.insert(path.clone(), change);
    }
}

/// `newer`'s changes after `older`'s: everything either changed, but what changed and
/// changed back since the last write, which then leaves the file as it is.
fn join(older: Option<Job>, newer: Job) -> Job {
    let Some(mut job) = older else {
        return newer;
    };
    for (path, change) in newer {
        match job.entry(path) {
            btree_map::Entry::Vacant(entry) => _ = entry.insert(change),
            btree_map::Entry::Occupied(mut entry) => {
                entry.get_mut().after = change.after;
                if entry.get().before == entry.get().after {
                    entry.remove();
                }
            }
        }
    }
    job
}

/// What reading the storage gave.
struct Loaded {
    entries: BTreeMap<&'static str, Entry>,
    /// The file's table as read, for a file damaged since.
    base: Table,
    warnings: Vec<String>,
    unknown: Vec<String>,
    /// Whether anything was stored.
    found: bool,
    /// Whether the storage may be written: not when it holds a newer version, nor when it
    /// could not be read.
    writable: bool,
}

impl Loaded {
    fn defaults(
        registry: &[&'static dyn Declared],
        warning: Option<String>,
        writable: bool,
    ) -> Loaded {
        Loaded {
            entries: entries(registry, |_| None),
            base: Table::new(),
            warnings: warning.into_iter().collect(),
            unknown: Vec::new(),
            found: false,
            writable,
        }
    }
}

/// Logs what reading `location` gave, under `-d settings`, and each warning, which shows
/// without it: the whole path in the first line and in every warning, the file's name in
/// the domain's other lines. Keys no setting reads are named only by a store that `writes`,
/// which keeps them: one that never writes may read one owner's settings of several, as
/// the CLI reads the engine's in a file it shares with the app.
fn report(location: &str, loaded: &Loaded, writes: bool) {
    let never = if writes { "" } else { ", never written" };
    if loaded.found {
        tracing::debug!(target: "settings", "read {location}{never}");
    } else if loaded.warnings.is_empty() {
        tracing::debug!(target: "settings", "none yet, defaults: {location}{never}");
    }
    for warning in &loaded.warnings {
        tracing::warn!(target: "settings", "{location}: {warning}");
    }
    if writes && !loaded.unknown.is_empty() {
        tracing::debug!(
            target: "settings",
            "{}: unknown to this build, kept: {}",
            file_name(location),
            loaded.unknown.join(", ")
        );
    }
}

/// Reads the storage for a store that writes it, setting a damaged file aside.
fn load(storage: &mut dyn Storage, registry: &[&'static dyn Declared]) -> Loaded {
    match look(storage, registry, true) {
        Ok(loaded) => loaded,
        Err(why) => damaged(storage, registry, why),
    }
}

/// Reads what `storage` holds, changing nothing: its settings, or why it is damaged (not
/// text, or not TOML). `writes` says whether the store will write it, which a newer version
/// and, for a store that writes, a read-only file forbid.
fn look(
    storage: &dyn Storage,
    registry: &[&'static dyn Declared],
    writes: bool,
) -> Result<Loaded, String> {
    let text = match storage.read() {
        Ok(None) => return Ok(Loaded::defaults(registry, None, writes)),
        Ok(Some(text)) => text,
        Err(e) if e.kind() == io::ErrorKind::InvalidData => return Err(format!("not text ({e})")),
        Err(e) => {
            let not = if writes { ", file not overwritten" } else { "" };
            let why = format!("cannot read: {e}; defaults used{not}");
            return Ok(Loaded::defaults(registry, Some(why), false));
        }
    };
    let base: Table = match text.parse() {
        Ok(table) => table,
        Err(e) => {
            let e: toml::de::Error = e;
            let at = e.to_string().lines().next().unwrap_or_default().to_owned();
            return Err(format!("not TOML: {at}: {}", e.message()));
        }
    };
    let (version, warning) = version(&base);
    let mut warnings: Vec<String> = warning.into_iter().collect();
    let mut table = base.clone();
    migrate(&mut table, version, MIGRATIONS);
    let entries = read(&table, registry, &mut warnings);
    // Asked only of a storage the store will write.
    let read_only = writes && storage.read_only();
    if version > VERSION {
        warnings.push(format!(
            "version {version} is newer than this build's {VERSION}: read, never overwritten"
        ));
    } else if read_only {
        warnings.push("read-only: read, never overwritten".to_owned());
    }
    let writable = writes && version <= VERSION && !read_only;
    Ok(Loaded {
        entries,
        unknown: unknown(&table, registry),
        base,
        warnings,
        found: true,
        writable,
    })
}

/// Sets a damaged file aside, so that the defaults can be written in its place, unless it
/// is read-only: then it stays, never overwritten.
fn damaged(storage: &mut dyn Storage, registry: &[&'static dyn Declared], why: String) -> Loaded {
    if storage.read_only() {
        let why = format!("{why}; defaults used, read-only: never overwritten");
        return Loaded::defaults(registry, Some(why), false);
    }
    match storage.set_aside() {
        Ok(aside) => {
            let why = format!("{why}; kept as {aside}, defaults used");
            Loaded::defaults(registry, Some(why), true)
        }
        Err(e) => {
            let why = format!("{why}; defaults used, not overwritten: set aside failed: {e}");
            Loaded::defaults(registry, Some(why), false)
        }
    }
}

/// A file's version, the file's own and not a setting, and a warning when it has none
/// that can be read, as this build's then.
fn version(table: &Table) -> (i64, Option<String>) {
    match table.get("version") {
        Some(Value::Integer(version)) if *version >= 1 => (*version, None),
        Some(other) => {
            let why = format!("version {other} is no version; read as version {VERSION}");
            (VERSION, Some(why))
        }
        None => {
            let why = format!("version missing; read as version {VERSION}");
            (VERSION, Some(why))
        }
    }
}

/// Reads each setting of `registry` from `table`, in the registry's order, noting what
/// cannot be read.
fn read(
    table: &Table,
    registry: &[&'static dyn Declared],
    warnings: &mut Vec<String>,
) -> BTreeMap<&'static str, Entry> {
    // The tables on settings' paths that are something else, each warned of once.
    let mut not_tables = BTreeSet::new();
    entries(registry, |setting| {
        let value = find(table, setting.id(), &mut not_tables, warnings)?;
        let read = setting.read(value, warnings);
        if read.is_none() {
            warnings.push(unreadable(setting.id(), value));
        }
        read
    })
}

/// The value at the dotted `id` in `table`, if there is one; a table on its path that is
/// something else, a value that cannot be read for every setting under it, is warned of
/// once, by `not_tables`.
fn find<'a>(
    table: &'a Table,
    id: &str,
    not_tables: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) -> Option<&'a Value> {
    let (parents, key) = match id.rsplit_once('.') {
        Some((parents, key)) => (Some(parents), key),
        None => (None, id),
    };
    let mut within = table;
    let mut at = String::new();
    for part in parents.into_iter().flat_map(|parents| parents.split('.')) {
        if !at.is_empty() {
            at.push('.');
        }
        at.push_str(part);
        let value = within.get(part)?;
        within = match value.as_table() {
            Some(inner) => inner,
            None => {
                if not_tables.insert(at.clone()) {
                    warnings.push(unreadable(&at, value));
                }
                return None;
            }
        };
    }
    within.get(key)
}

/// The keys of `table` that no setting of `registry` reads, dotted: the shallowest first,
/// each level in alphabetical order, as `toml`'s tables keep keys. A table's own entries,
/// as `keys`' actions, are the setting's, never unknown.
fn unknown(table: &Table, registry: &[&'static dyn Declared]) -> Vec<String> {
    let ids: BTreeSet<&str> = registry.iter().map(|setting| setting.id()).collect();
    let holds_one = |prefix: &str| {
        ids.iter().any(|id| {
            id.strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('.'))
        })
    };
    let mut unknown = Vec::new();
    let mut tables = VecDeque::from([(String::new(), table)]);
    while let Some((prefix, table)) = tables.pop_front() {
        for (key, value) in table {
            let id = match prefix.as_str() {
                "" => key.clone(),
                prefix => format!("{prefix}.{key}"),
            };
            if id == "version" || ids.contains(id.as_str()) {
                continue;
            }
            match value.as_table() {
                Some(inner) if holds_one(&id) => tables.push_back((id, inner)),
                // Something else where settings' table is: warned of as they are read.
                _ if holds_one(&id) => {}
                _ => unknown.push(id),
            }
        }
    }
    unknown
}

/// The file's text, from its table with `version` first, written by hand, where a person
/// looks for it.
fn text(table: &Table) -> String {
    let mut table = table.clone();
    table.remove("version");
    let text = toml::to_string(&table).expect("a table of plain values is TOML");
    format!(
        "# Apertrust's settings. Apertrust writes here only what changes, without comments;\n\
         # keys it does not know stay.\n\nversion = {VERSION}\n{text}"
    )
}

/// Sets each of `job`'s values in `table`, or removes it, with the tables its removal
/// leaves empty.
fn merge(table: &mut Table, job: &Job) {
    for (path, change) in job {
        let Some((key, parents)) = path.split_last() else {
            continue;
        };
        match &change.after {
            Some(value) => {
                let within = parents
                    .iter()
                    .fold(&mut *table, |table, key| within(table, key));
                within.insert(key.clone(), value.clone());
            }
            None => remove(table, path),
        }
    }
}

/// The table `key` of `table`, made one if it is missing or something else.
fn within<'a>(table: &'a mut Table, key: &str) -> &'a mut Table {
    let entry = table
        .entry(key.to_owned())
        .or_insert_with(|| Table::new().into());
    if !entry.is_table() {
        *entry = Table::new().into();
    }
    entry.as_table_mut().expect("made a table above")
}

/// Removes the value at `path`, and each table on it that is left empty.
fn remove(table: &mut Table, path: &[String]) {
    match path {
        [] => {}
        [key] => _ = table.remove(key),
        [key, rest @ ..] => {
            if let Some(inner) = table.get_mut(key).and_then(Value::as_table_mut) {
                remove(inner, rest);
                if inner.is_empty() {
                    table.remove(key);
                }
            }
        }
    }
}

/// The thread that writes, and the channel to it.
struct Writer {
    jobs: Option<mpsc::Sender<Job>>,
    thread: Option<JoinHandle<()>>,
}

/// How a write went.
enum Wrote {
    /// The file now holds this table.
    Written(Table),
    /// Try again with the next changes, and on exit.
    Failed,
    /// The storage now holds a newer version, or a damaged file that could not be set
    /// aside: write nothing more.
    Stop,
}

impl Writer {
    /// Starts writing to `storage`; `base` is the file as it was read, and then as last
    /// written, for a file damaged since, so a damage keeps this run's earlier writes.
    fn start(mut storage: Box<dyn Storage>, mut base: Table) -> Writer {
        let (jobs, received) = mpsc::channel::<Job>();
        let thread = std::thread::Builder::new()
            .name("settings writer".to_owned())
            .spawn(move || {
                let mut pending: Option<Job> = None;
                let mut open = true;
                while open {
                    // A job that failed stays, joined with the next; once the channel
                    // closes, it is tried once more.
                    match received.recv() {
                        Ok(job) => pending = Some(join(pending.take(), job)),
                        Err(mpsc::RecvError) => open = false,
                    }
                    for job in received.try_iter() {
                        pending = Some(join(pending.take(), job));
                    }
                    let Some(job) = pending.take() else {
                        continue;
                    };
                    match write(storage.as_mut(), &base, &job) {
                        Wrote::Written(table) => base = table,
                        Wrote::Failed => pending = Some(job),
                        Wrote::Stop => return,
                    }
                }
            })
            .expect("the settings writer starts");
        Writer {
            jobs: Some(jobs),
            thread: Some(thread),
        }
    }

    fn send(&self, job: Job) {
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(job);
        }
    }
}

impl Drop for Writer {
    /// Writes what it still has, then ends.
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!(target: "settings", "settings writer panicked");
        }
    }
}

/// Writes `job`'s changes into what `storage` holds now, so what another Apertrust or a
/// person changed since the last write stays. `base` stands in for a file damaged since
/// the last write, or since it was read.
fn write(storage: &mut dyn Storage, base: &Table, job: &Job) -> Wrote {
    // The whole path in a warning, which shows without -d; the file's name in a debug
    // line, after the one that read it.
    let location = storage.location();
    if storage.read_only() {
        tracing::warn!(target: "settings", "{location}: read-only since read, no longer written");
        return Wrote::Stop;
    }
    let now = match storage.read() {
        Ok(None) => Ok(Table::new()),
        Ok(Some(text)) => match text.parse::<Table>() {
            Ok(table) => match version(&table) {
                (version, _) if version > VERSION => {
                    tracing::warn!(
                        target: "settings",
                        "{location}: version {version} from a newer Apertrust, no longer written"
                    );
                    return Wrote::Stop;
                }
                _ => Ok(table),
            },
            Err(e) => Err(e.message().to_owned()),
        },
        Err(e) if e.kind() == io::ErrorKind::InvalidData => Err(e.to_string()),
        Err(e) => {
            tracing::warn!(target: "settings", "cannot read {location} to write it: {e}; retried on the next change");
            return Wrote::Failed;
        }
    };
    let mut table = match now {
        Ok(now) => now,
        Err(why) => match storage.set_aside() {
            Ok(aside) => {
                tracing::warn!(target: "settings", "{location} damaged since read: {why}; kept as {aside}");
                base.clone()
            }
            Err(e) => {
                tracing::warn!(target: "settings", "{location} damaged since read: {why}; not written, set aside failed: {e}");
                return Wrote::Stop;
            }
        },
    };
    let (version, _) = version(&table);
    migrate(&mut table, version, MIGRATIONS);
    merge(&mut table, job);
    let start = Instant::now();
    match storage.write(&text(&table)) {
        Ok(()) => {
            tracing::debug!(
                target: "settings",
                "wrote {} in {:.1} ms",
                file_name(&location),
                start.elapsed().as_secs_f64() * 1e3
            );
            Wrote::Written(table)
        }
        Err(e) => {
            tracing::warn!(target: "settings", "cannot write {location}: {e}; retried on the next change");
            Wrote::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::settings::{self, Applies, Choice, FileStorage, Number, OverrideError, Owner};

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Theme {
        Dark,
        Light,
        HighContrast,
    }

    impl Choice for Theme {
        const OPTIONS: &'static [(Theme, &'static str)] = &[
            (Theme::Dark, "dark"),
            (Theme::Light, "light"),
            (Theme::HighContrast, "high-contrast"),
        ];
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Channel {
        Red,
        Green,
        Blue,
    }

    impl Choice for Channel {
        const OPTIONS: &'static [(Channel, &'static str)] = &[
            (Channel::Red, "red"),
            (Channel::Green, "green"),
            (Channel::Blue, "blue"),
        ];
    }

    const fn setting<T>(id: &'static str, kind: Kind, default: DefaultValue<T>) -> Setting<T> {
        Setting {
            id,
            owner: Owner::App,
            kind,
            default,
            applies: Applies::AtOnce,
            page: None,
            group: None,
            requires: None,
            place: 0,
            line: 0,
        }
    }

    const fn number(
        min: Option<f32>,
        max: Option<f32>,
        above: Option<f32>,
        decimals: Option<u32>,
    ) -> Kind {
        Kind::Number(Number {
            min,
            max,
            above,
            step: None,
            decimals,
            unit: None,
            factor: None,
        })
    }

    const WHOLE: Kind = number(None, None, None, Some(0));
    const WHOLE_ABOVE_0: Kind = number(None, None, Some(0.0), Some(0));

    // A registry of every kind, as the app's is: generated code holds such statics.
    use DefaultValue::{Computed, Value as Is};
    static THEME: Setting<Theme> = setting("theme", Kind::Choice("Theme"), Is(Theme::Dark));
    /// Computed, as the language is.
    static NAME: Setting<&'static str> = setting("name", Kind::Text, Computed(|| "system"));
    static SCALE: Setting<f32> = setting(
        "scale",
        number(Some(0.5), Some(3.0), None, Some(2)),
        Is(1.0),
    );
    static SIZE: Setting<[f32; 2]> = setting(
        "window.size",
        Kind::List {
            of: &WHOLE_ABOVE_0,
            length: Some(2),
        },
        Is([1600.0, 1000.0]),
    );
    static POSITION: Setting<Option<[f32; 2]>> = setting(
        "window.position",
        Kind::List {
            of: &WHOLE,
            length: Some(2),
        },
        Is(None),
    );
    static FACTOR: Setting<Option<f32>> = setting(
        "window.scale_factor",
        number(None, None, Some(0.0), None),
        Is(None),
    );
    static MAXIMIZED: Setting<bool> = setting("window.maximized", Kind::Boolean, Is(false));
    /// Computed, as the side panel's width is.
    static WIDTH: Setting<f32> = setting(
        "side_panel.width",
        number(Some(240.0), Some(600.0), None, Some(0)),
        Computed(|| 320.0),
    );
    static TAB: Setting<&'static str> = setting("edit.tab", Kind::Text, Is("adjust"));
    static SECTIONS: Setting<settings::Table<bool>> = setting(
        "edit.sections",
        Kind::Table {
            by: "sections",
            of: &Kind::Boolean,
        },
        Is(&[]),
    );
    static GROUPS: Setting<settings::Table<settings::Table<bool>>> = setting(
        "edit.groups",
        Kind::Table {
            by: "modules",
            of: &Kind::Table {
                by: "groups",
                of: &Kind::Boolean,
            },
        },
        Is(&[]),
    );
    static CHANNELS: Setting<&'static [Channel]> = setting(
        "scopes.channels",
        Kind::List {
            of: &Kind::Choice("Channel"),
            length: None,
        },
        Is(&[Channel::Red, Channel::Green, Channel::Blue]),
    );
    static KEYS: Setting<settings::Table<&'static [&'static str]>> = setting(
        "keys",
        Kind::Table {
            by: "actions",
            of: &Kind::List {
                of: &Kind::Text,
                length: None,
            },
        },
        Is(&[]),
    );
    static ALL: &[&dyn Declared] = &[
        &THEME, &NAME, &SCALE, &SIZE, &POSITION, &FACTOR, &MAXIMIZED, &WIDTH, &TAB, &SECTIONS,
        &GROUPS, &CHANNELS, &KEYS,
    ];

    /// A folder of a test's own, empty, removed when the test ends: never the user's.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Folder {
            let path = std::env::temp_dir().join(format!(
                "apertrust-base-settings-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temporary folder");
            Folder(path)
        }

        /// A folder whose settings file holds `contents`.
        fn holding(name: &str, contents: impl AsRef<[u8]>) -> Folder {
            let folder = Folder::new(name);
            std::fs::write(folder.settings(), contents).expect("written");
            folder
        }

        fn settings(&self) -> PathBuf {
            self.0.join("settings.toml")
        }

        fn storage(&self) -> Box<dyn Storage> {
            Box::new(FileStorage::new(self.settings()))
        }

        fn open(&self) -> Store {
            Store::open(self.storage(), ALL)
        }

        /// The names of the files in it, sorted.
        fn files(&self) -> Vec<String> {
            let names = std::fs::read_dir(&self.0).expect("the folder").map(|e| {
                let name = e.expect("an entry").file_name();
                name.to_string_lossy().into_owned()
            });
            let mut names: Vec<String> = names.collect();
            names.sort();
            names
        }

        fn text(&self) -> String {
            std::fs::read_to_string(self.settings()).expect("the settings file")
        }

        fn table(&self) -> Table {
            self.text().parse().expect("TOML")
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Settings in memory: their writes counted, the first `failures` of them failing, and
    /// with `unreadable`, never read.
    #[derive(Clone, Default)]
    struct Memory {
        text: Arc<Mutex<Option<String>>>,
        failures: Arc<AtomicUsize>,
        writes: Arc<AtomicUsize>,
        unreadable: bool,
    }

    impl Memory {
        fn failing(failures: usize) -> Memory {
            Memory {
                failures: Arc::new(AtomicUsize::new(failures)),
                ..Memory::default()
            }
        }

        fn open(&self) -> Store {
            Store::open(Box::new(self.clone()), ALL)
        }

        fn writes(&self) -> usize {
            self.writes.load(Ordering::SeqCst)
        }
    }

    impl Storage for Memory {
        fn location(&self) -> String {
            "memory".to_owned()
        }

        fn read(&self) -> io::Result<Option<String>> {
            if self.unreadable {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(self.text.lock().expect("unpoisoned").clone())
        }

        fn write(&mut self, text: &str) -> io::Result<()> {
            let fail = self
                .failures
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok();
            if fail {
                return Err(io::Error::other("the disk is full"));
            }
            *self.text.lock().expect("unpoisoned") = Some(text.to_owned());
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn set_aside(&mut self) -> io::Result<String> {
            Err(io::Error::other("memory has no aside"))
        }
    }

    /// Each warning's setting, its first word.
    fn names(store: &Store) -> Vec<&str> {
        let names = store.warnings().iter();
        names
            .map(|w| w.split(' ').next().unwrap_or_default())
            .collect()
    }

    /// Every setting off its default.
    fn everything(store: &mut Store) {
        store.set(&THEME, Theme::HighContrast);
        store.set(&NAME, "mine".to_owned());
        store.set(&SCALE, 1.3);
        store.set(&SIZE, [1280.0, 800.0]);
        store.set(&POSITION, Some([-80.0, 50.0]));
        store.set(&FACTOR, Some(2.0));
        store.set(&MAXIMIZED, true);
        store.set(&WIDTH, 480.0);
        store.set(&TAB, "optics".to_owned());
        let sections = [("exposure", false), ("tone_curve", true)];
        store.set(&SECTIONS, sections.map(|(k, v)| (k.to_owned(), v)).into());
        let groups = BTreeMap::from([("mapping".to_owned(), true)]);
        store.set(&GROUPS, BTreeMap::from([("exposure".to_owned(), groups)]));
        store.set(&CHANNELS, vec![Channel::Blue, Channel::Red]);
        let keys = [("undo", vec!["Ctrl+K".to_owned()]), ("quit", Vec::new())];
        store.set(&KEYS, keys.map(|(k, v)| (k.to_owned(), v)).into());
    }

    /// Sets in `to` what `from` keeps of `setting`.
    fn copy<T: Stored>(from: &Store, to: &mut Store, setting: &Setting<T>) {
        if let Some(value) = from.kept(setting) {
            to.set(setting, value);
        }
    }

    #[test]
    fn a_missing_key_is_its_default_and_a_set_to_it_changes_nothing() {
        let folder = Folder::new("missing");
        let mut store = folder.open();
        assert!(store.warnings().is_empty());
        assert_eq!(store.get(&THEME), Theme::Dark);
        assert_eq!(store.get(&NAME), "system");
        assert_eq!(store.get(&SIZE), [1600.0, 1000.0]);
        assert_eq!(store.get(&POSITION), None);
        assert_eq!(
            store.get(&CHANNELS),
            [Channel::Red, Channel::Green, Channel::Blue]
        );
        assert_eq!(store.get(&KEYS), BTreeMap::new());
        assert_eq!(store.kept(&SCALE), None);
        store.set(&THEME, Theme::Dark);
        store.set(&SIZE, [1600.0, 1000.0]);
        store.set(&POSITION, None);
        store.set(&SECTIONS, BTreeMap::new());
        store.reset(&WIDTH);
        assert!(!store.dirty);
        // Changed and changed back: nothing kept, so nothing to write.
        store.set(&THEME, Theme::Light);
        store.set(&THEME, Theme::Dark);
        assert_eq!(store.kept(&THEME), None);
        store.flush();
        assert!(folder.files().is_empty(), "{:?}", folder.files());
    }

    #[test]
    fn values_are_typed_by_their_statics_and_owned() {
        let folder = Folder::new("typed");
        let mut store = folder.open();
        everything(&mut store);
        assert_eq!(store.get(&TAB), "optics");
        assert_eq!(store.get(&CHANNELS), [Channel::Blue, Channel::Red]);
        assert_eq!(store.get(&KEYS)["undo"], ["Ctrl+K"]);
        assert_eq!(store.get(&POSITION), Some([-80.0, 50.0]));
        // A computed default is no value: a set keeps the computed one, a reset follows it.
        store.set(&WIDTH, 320.0);
        assert_eq!(store.kept(&WIDTH), Some(320.0));
        store.reset(&NAME);
        assert_eq!(
            (store.kept(&NAME), store.get(&NAME)),
            (None, "system".to_owned())
        );
        // An optional setting's None keeps nothing.
        store.set(&FACTOR, None);
        assert_eq!(store.kept(&FACTOR), None);
        store.flush();
        let read = folder.open();
        assert_eq!(read.get(&WIDTH), 320.0);
        assert!(read.get(&GROUPS)["exposure"]["mapping"]);
        assert_eq!((read.kept(&NAME), read.get(&FACTOR)), (None, None));
    }

    /// What the Settings dialog reads and writes without the statics' types: each value as
    /// the file keeps it, set by the kind's reading and reset as a typed reset does.
    #[test]
    fn any_setting_is_read_set_and_reset_as_the_file_keeps_it() {
        let mut store = Store::nowhere(ALL);
        let (theme, scale): (&dyn Declared, &dyn Declared) = (&THEME, &SCALE);
        assert_eq!(store.value(theme), Some(Value::String("dark".into())));
        assert_eq!(theme.default_value(), Some(Value::String("dark".into())));
        assert_eq!(theme.options(), ["dark", "light", "high-contrast"]);
        assert!(scale.options().is_empty());
        assert!(store.set_value(theme, &Value::String("light".into())));
        assert_eq!(store.get(&THEME), Theme::Light);
        // A value its kind cannot read changes nothing.
        assert!(!store.set_value(theme, &Value::String("solarized".into())));
        assert!(!store.set_value(scale, &Value::Boolean(true)));
        assert_eq!(store.get(&THEME), Theme::Light);
        assert!(store.set_value(scale, &Value::Integer(2)));
        assert_eq!(store.value(scale), Some(Value::Float(2.0)));
        store.reset_value(theme);
        assert_eq!(store.kept(&THEME), None);
        // An optional setting's none is none.
        assert_eq!(store.value(&POSITION), None);
        assert_eq!(POSITION.default_value(), None);
        // A computed default is what it computes now.
        assert_eq!(NAME.default_value(), Some(Value::String("system".into())));
    }

    #[test]
    #[should_panic(expected = "settings: elsewhere is in no registry this store was opened with")]
    fn a_setting_of_no_registry_is_refused() {
        static ELSEWHERE: Setting<bool> = setting("elsewhere", Kind::Boolean, Is(false));
        Store::nowhere(ALL).get(&ELSEWHERE);
    }

    /// A file of every kind, as this build writes it, in the order of `toml`'s tables: read
    /// back, it gives each value with no warning, and written afresh, the same bytes.
    #[test]
    fn every_kind_reads_and_writes_back_byte_for_byte() {
        let folder = Folder::new("every-kind");
        let mut store = folder.open();
        everything(&mut store);
        store.flush();
        let expected = "\
# Apertrust's settings. Apertrust writes here only what changes, without comments;
# keys it does not know stay.

version = 1
name = \"mine\"
scale = 1.3
theme = \"high-contrast\"

[edit]
tab = \"optics\"

[edit.groups.exposure]
mapping = true

[edit.sections]
exposure = false
tone_curve = true

[keys]
quit = []
undo = [\"Ctrl+K\"]

[scopes]
channels = [\"blue\", \"red\"]

[side_panel]
width = 480

[window]
maximized = true
position = [-80, 50]
scale_factor = 2.0
size = [1280, 800]
";
        assert_eq!(folder.text(), expected);
        let read = folder.open();
        assert!(read.warnings().is_empty(), "{:?}", read.warnings());
        let again = Folder::new("every-kind-again");
        let mut copied = again.open();
        copy(&read, &mut copied, &THEME);
        copy(&read, &mut copied, &NAME);
        copy(&read, &mut copied, &SCALE);
        copy(&read, &mut copied, &SIZE);
        copy(&read, &mut copied, &POSITION);
        copy(&read, &mut copied, &FACTOR);
        copy(&read, &mut copied, &MAXIMIZED);
        copy(&read, &mut copied, &WIDTH);
        copy(&read, &mut copied, &TAB);
        copy(&read, &mut copied, &SECTIONS);
        copy(&read, &mut copied, &GROUPS);
        copy(&read, &mut copied, &CHANNELS);
        copy(&read, &mut copied, &KEYS);
        copied.flush();
        assert_eq!(again.text(), expected);
    }

    #[test]
    fn a_value_that_cannot_be_read_gets_its_default_with_a_warning_and_stays_until_changed() {
        let folder = Folder::holding(
            "unreadable",
            "version = 1\ntheme = \"solarized\"\nscale = \"big\"\n\
             [window]\nsize = [0, 600]\nscale_factor = -1\nmaximized = 1\n\
             [edit]\nsections = { exposure = \"open\", curve = true }\n\
             groups = { sample = true, exposure = { mapping = 1, shape = true } }\n\
             [scopes]\nchannels = [\"red\", \"ultraviolet\"]\n\
             [keys]\nundo = 4\nredo = \"Ctrl+R\"\n",
        );
        let mut store = folder.open();
        assert_eq!((store.get(&THEME), store.kept(&THEME)), (Theme::Dark, None));
        assert_eq!(store.get(&SCALE), 1.0);
        assert_eq!(store.kept(&SIZE), None);
        assert_eq!(store.get(&FACTOR), None);
        assert!(!store.get(&MAXIMIZED));
        let curve = BTreeMap::from([("curve".to_owned(), true)]);
        assert_eq!(store.get(&SECTIONS), curve);
        let shape = BTreeMap::from([("shape".to_owned(), true)]);
        let groups = BTreeMap::from([("exposure".to_owned(), shape)]);
        assert_eq!(store.get(&GROUPS), groups);
        assert_eq!(store.kept(&CHANNELS), None);
        // A single key may be written without its list.
        let redo = BTreeMap::from([("redo".to_owned(), vec!["Ctrl+R".to_owned()])]);
        assert_eq!(store.get(&KEYS), redo);
        assert_eq!(
            names(&store),
            [
                "theme",
                "scale",
                "window.size",
                "window.scale_factor",
                "window.maximized",
                "edit.sections.exposure",
                "edit.groups.exposure.mapping",
                "edit.groups.sample",
                "scopes.channels",
                "keys.undo",
            ]
        );
        assert_eq!(
            store.warnings()[0],
            "theme cannot be \"solarized\"; the default is used"
        );
        // Set as read, which a client does each frame: nothing changes.
        store.set(&THEME, store.get(&THEME));
        store.set(&SECTIONS, store.get(&SECTIONS));
        store.set(&KEYS, store.get(&KEYS));
        store.set(&CHANNELS, store.get(&CHANNELS));
        assert!(!store.dirty);
        // Another change leaves them in the file.
        store.set(&TAB, "optics".to_owned());
        store.flush();
        let table = folder.table();
        assert_eq!(table["theme"].as_str(), Some("solarized"));
        assert_eq!(table["scale"].as_str(), Some("big"));
        assert_eq!(table["window"]["size"][1].as_integer(), Some(600));
        assert_eq!(table["window"]["scale_factor"].as_integer(), Some(-1));
        assert_eq!(table["edit"]["sections"]["exposure"].as_str(), Some("open"));
        assert_eq!(table["edit"]["groups"]["sample"].as_bool(), Some(true));
        assert_eq!(table["scopes"]["channels"][1].as_str(), Some("ultraviolet"));
        assert_eq!(table["keys"]["undo"].as_integer(), Some(4));
        assert_eq!(table["keys"]["redo"].as_str(), Some("Ctrl+R"));
        assert_eq!(table["edit"]["tab"].as_str(), Some("optics"));
        // Until it changes, a table's entries one by one.
        let mut store = folder.open();
        store.set(&THEME, Theme::Light);
        let sections = [("curve", true), ("exposure", true)];
        store.set(&SECTIONS, sections.map(|(k, v)| (k.to_owned(), v)).into());
        store.flush();
        let table = folder.table();
        assert_eq!(table["theme"].as_str(), Some("light"));
        assert_eq!(table["edit"]["sections"]["exposure"].as_bool(), Some(true));
        assert_eq!(table["keys"]["undo"].as_integer(), Some(4));
    }

    #[test]
    fn a_table_on_a_setting_s_path_that_is_something_else_cannot_be_read() {
        let folder = Folder::holding("not-a-table", "version = 1\nwindow = 3\n");
        let mut store = folder.open();
        assert_eq!(
            store.warnings(),
            ["window cannot be 3; the default is used"]
        );
        assert_eq!(store.get(&SIZE), [1600.0, 1000.0]);
        // Once a window setting changes, a table takes its place.
        store.set(&MAXIMIZED, true);
        store.flush();
        assert_eq!(folder.table()["window"]["maximized"].as_bool(), Some(true));
    }

    #[test]
    fn a_number_out_of_range_reads_as_the_nearer_end() {
        let folder = Folder::holding(
            "out-of-range",
            "version = 1\nscale = 9.0\n[side_panel]\nwidth = 100\n",
        );
        let mut store = folder.open();
        assert_eq!((store.get(&SCALE), store.kept(&WIDTH)), (3.0, Some(240.0)));
        assert_eq!(
            store.warnings(),
            [
                "scale 9 is out of range; 3 is used",
                "side_panel.width 100 is out of range; 240 is used"
            ]
        );
        // What it is read as is what is kept, so it stays in the file until it changes.
        store.set(&SCALE, 3.0);
        store.set(&MAXIMIZED, true);
        store.flush();
        let table = folder.table();
        assert_eq!(table["scale"].as_float(), Some(9.0));
        assert_eq!(table["side_panel"]["width"].as_integer(), Some(100));
    }

    #[test]
    fn unknown_keys_stay() {
        let text = "version = 1\ntheme = \"light\"\ndensity = \"compact\"\n\
                    [window]\nmonitor = \"DELL\"\n[workspace]\ntabs = [\"adjust\"]\n\
                    [keys]\nfuture = [\"F13\"]\n";
        let folder = Folder::holding("unknown", text);
        // A table's own entries, as an action this build lacks, are the setting's.
        let loaded = load(folder.storage().as_mut(), ALL);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.unknown, ["density", "workspace", "window.monitor"]);
        let mut store = folder.open();
        assert_eq!(store.get(&THEME), Theme::Light);
        store.set(&THEME, Theme::Dark);
        store.flush();
        let table = folder.table();
        assert_eq!(table["density"].as_str(), Some("compact"));
        assert_eq!(table["window"]["monitor"].as_str(), Some("DELL"));
        assert_eq!(table["workspace"]["tabs"][0].as_str(), Some("adjust"));
        assert_eq!(table["keys"]["future"][0].as_str(), Some("F13"));
        assert_eq!(table["theme"].as_str(), Some("dark"));
    }

    #[test]
    fn a_file_that_is_not_toml_is_set_aside_and_the_defaults_used() {
        for (name, damage) in [
            ("not-toml", b"theme = \"light\"\n[window\n".as_slice()),
            ("not-text", b"theme = \"\xff\xfe\"\n".as_slice()),
        ] {
            let folder = Folder::holding(name, damage);
            let store = folder.open();
            assert_eq!(store.get(&THEME), Theme::Dark, "{name}");
            assert_eq!(store.warnings().len(), 1, "{name}");
            let warning = &store.warnings()[0];
            assert!(warning.contains(".bak"), "{warning}");
            assert!(store.writer.is_some());
            assert_eq!(folder.files(), ["settings.toml.bak"], "{name}");
            let aside = std::fs::read(folder.0.join("settings.toml.bak")).expect("kept");
            assert_eq!(aside, damage, "{name}");
        }
        // The next change writes it alone; the damaged file stays aside.
        let folder = Folder::holding("damaged-then-changed", "[window\n");
        let mut store = folder.open();
        store.set(&THEME, Theme::Light);
        store.flush();
        assert_eq!(folder.files(), ["settings.toml", "settings.toml.bak"]);
        assert_eq!(folder.open().get(&THEME), Theme::Light);
    }

    #[test]
    fn a_file_damaged_since_it_was_read_is_set_aside_and_written_as_it_was_read() {
        let folder = Folder::holding("damaged-since", "version = 1\nscale = 2\ndensity = 1\n");
        let mut store = folder.open();
        std::fs::write(folder.settings(), "[window\n").expect("damaged");
        store.set(&THEME, Theme::Light);
        store.flush();
        assert_eq!(folder.files(), ["settings.toml", "settings.toml.bak"]);
        let table = folder.table();
        assert_eq!(table["theme"].as_str(), Some("light"));
        assert_eq!(table["density"].as_integer(), Some(1));
        assert_eq!(table["scale"].as_integer(), Some(2));
    }

    /// Waits for what the writer thread does, for at most five seconds.
    fn wait_for(done: impl Fn() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(start.elapsed() < Duration::from_secs(5), "timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn a_file_damaged_mid_session_keeps_this_session_s_earlier_writes() {
        let folder = Folder::holding("damaged-mid-session", "version = 1\ntheme = \"light\"\n");
        let mut store = folder.open();
        let start = Instant::now();
        store.set(&SCALE, 1.5);
        store.poll(start);
        store.poll(start + DELAY);
        // Damaged once the first write is in it.
        wait_for(|| std::fs::read_to_string(folder.settings()).is_ok_and(|t| t.contains("1.5")));
        std::fs::write(folder.settings(), "[window\n").expect("damaged");
        store.set(&TAB, "optics".to_owned());
        store.flush();
        assert_eq!(folder.files(), ["settings.toml", "settings.toml.bak"]);
        let table = folder.table();
        assert_eq!(table["theme"].as_str(), Some("light"));
        assert_eq!(table["scale"].as_float(), Some(1.5));
        assert_eq!(table["edit"]["tab"].as_str(), Some("optics"));
    }

    #[cfg(unix)]
    fn set_mode(folder: &Folder, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(folder.settings(), permissions).expect("its mode");
    }

    #[cfg(unix)]
    #[test]
    fn a_file_only_its_owner_may_read_stays_so() {
        use std::os::unix::fs::PermissionsExt;
        let folder = Folder::holding("private", "version = 1\n");
        set_mode(&folder, 0o600);
        let mut store = folder.open();
        store.set(&THEME, Theme::Light);
        store.flush();
        let mode = std::fs::metadata(folder.settings())
            .expect("there")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");
        assert_eq!(folder.open().get(&THEME), Theme::Light);
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_file_is_read_and_never_overwritten() {
        let text = "version = 1\ntheme = \"light\"\n";
        let folder = Folder::holding("read-only", text);
        set_mode(&folder, 0o444);
        let mut store = folder.open();
        assert_eq!(store.get(&THEME), Theme::Light);
        assert_eq!(store.warnings(), ["read-only: read, never overwritten"]);
        assert!(store.writer.is_none());
        everything(&mut store);
        store.flush();
        assert_eq!(
            (folder.text().as_str(), folder.files()),
            (text, vec!["settings.toml".to_owned()])
        );
        // Nor one made read-only since it was read.
        let folder = Folder::holding("read-only-since", text);
        let mut store = folder.open();
        set_mode(&folder, 0o444);
        store.set(&SCALE, 2.0);
        store.flush();
        assert_eq!(folder.text(), text);
        // A damaged one is neither set aside nor written.
        let folder = Folder::holding("read-only-damaged", "[window\n");
        set_mode(&folder, 0o444);
        let mut store = folder.open();
        assert_eq!(store.warnings().len(), 1);
        assert!(
            store.warnings()[0].contains("read-only"),
            "{:?}",
            store.warnings()
        );
        store.set(&SCALE, 2.0);
        store.flush();
        assert_eq!(folder.files(), ["settings.toml"]);
        assert_eq!(folder.text(), "[window\n");
    }

    /// The CLI's store (decision 43): read by the file's rules, never written, whatever is
    /// set and however the file is.
    #[test]
    fn a_store_that_never_writes_reads_by_the_same_rules_and_changes_nothing() {
        let text = "# mine\nversion = 1\ntheme = \"light\"\nscale = 9.0\ndensity = 1\n";
        let folder = Folder::holding("never-writes", text);
        // What a write that ended with its process left, which opening removes.
        let stale = folder.0.join("settings.toml.123-4.tmp");
        std::fs::write(&stale, "x").expect("written");
        let file = std::fs::File::options().write(true).open(&stale);
        let old = std::time::SystemTime::now() - Duration::from_secs(120);
        file.and_then(|f| f.set_modified(old)).expect("dated");
        let mut store = Store::read_only(folder.storage().as_ref(), ALL);
        assert_eq!((store.get(&THEME), store.get(&SCALE)), (Theme::Light, 3.0));
        assert_eq!(store.warnings(), ["scale 9 is out of range; 3 is used"]);
        // What is set is kept for the run alone.
        everything(&mut store);
        assert_eq!(store.get(&THEME), Theme::HighContrast);
        let start = Instant::now();
        store.poll(start);
        assert_eq!(store.poll(start + DELAY), None);
        store.flush();
        drop(store);
        assert_eq!(folder.text(), text);
        assert_eq!(folder.files(), ["settings.toml", "settings.toml.123-4.tmp"]);

        // A damaged file gives the defaults and stays as it is, never set aside.
        for (name, damage) in [
            (
                "never-writes-not-toml",
                b"theme = \"light\"\n[window\n".as_slice(),
            ),
            (
                "never-writes-not-text",
                b"theme = \"\xff\xfe\"\n".as_slice(),
            ),
        ] {
            let folder = Folder::holding(name, damage);
            let mut store = Store::read_only(folder.storage().as_ref(), ALL);
            assert_eq!(store.get(&THEME), Theme::Dark, "{name}");
            let warnings = store.warnings();
            assert!(
                warnings.len() == 1 && warnings[0].ends_with("defaults used, left as it is"),
                "{warnings:?}"
            );
            store.set(&THEME, Theme::Light);
            drop(store);
            assert_eq!(folder.files(), ["settings.toml"], "{name}");
            let kept = std::fs::read(folder.settings()).expect("there");
            assert_eq!(kept, damage, "{name}");
        }
        // Nothing stored: nothing written.
        let folder = Folder::new("never-writes-none");
        let mut store = Store::read_only(folder.storage().as_ref(), ALL);
        assert!(store.warnings().is_empty());
        everything(&mut store);
        drop(store);
        assert!(folder.files().is_empty(), "{:?}", folder.files());
    }

    fn conf(given: &[&str]) -> Result<Overrides, OverrideError> {
        Overrides::read(given, ALL)
    }

    /// `--conf`'s values by the setting's kind, with the file's rules: a text or a choice's
    /// id as it is, `~` included; anything else as the file writes it.
    #[test]
    fn conf_values_are_read_by_their_kind_with_the_file_s_rules() {
        let overrides = conf(&[
            "theme=high-contrast",
            "name=~/mine",
            "edit.tab=",
            "scale=9",
            "window.maximized=true",
            "window.size=[1280, 800]",
            "window.position=[-80, 50]",
            "scopes.channels=[\"blue\"]",
            "keys={ undo = [\"Ctrl+K\"], redo = 4 }",
        ])
        .expect("read");
        let store = Store::nowhere(ALL).with_overrides(&overrides);
        assert_eq!(store.get(&THEME), Theme::HighContrast);
        assert_eq!(
            (store.get(&NAME), store.get(&TAB)),
            ("~/mine".into(), "".into())
        );
        assert!(store.get(&MAXIMIZED));
        assert_eq!(store.get(&SIZE), [1280.0, 800.0]);
        assert_eq!(store.get(&POSITION), Some([-80.0, 50.0]));
        assert_eq!(store.get(&CHANNELS), [Channel::Blue]);
        let undo = BTreeMap::from([("undo".to_owned(), vec!["Ctrl+K".to_owned()])]);
        assert_eq!(store.get(&KEYS), undo);
        // Out of range, the nearer end; a table's entry that cannot be read, left out.
        assert_eq!(store.get(&SCALE), 3.0);
        assert_eq!(
            overrides.warnings().collect::<Vec<_>>(),
            [
                "keys.redo cannot be 4; the default is used",
                "scale 9 is out of range; 3 is used"
            ]
        );
        assert_eq!(overrides.value(&SCALE), Some(Value::Float(3.0)));
        assert_eq!(overrides.value(&FACTOR), None);
    }

    #[test]
    fn a_conf_of_no_setting_or_of_a_value_that_cannot_be_read_is_refused_naming_it() {
        let unknown = |key: &str| Err(OverrideError::Unknown(key.to_owned()));
        let unreadable = |key: &str, value: &str| {
            Err(OverrideError::Unreadable {
                key: key.to_owned(),
                value: value.to_owned(),
            })
        };
        assert_eq!(conf(&["density=1"]).map(drop), unknown("density"));
        // A table's entry is no setting of its own.
        assert_eq!(conf(&["keys.undo=Ctrl+K"]).map(drop), unknown("keys.undo"));
        assert_eq!(conf(&[" scale=1"]).map(drop), unknown(" scale"));
        assert_eq!(
            conf(&["scale"]).map(drop),
            Err(OverrideError::Form("scale".to_owned()))
        );
        for (key, value) in [
            ("scale", "big"),
            ("scale", ""),
            ("scale", "inf"),
            // One value alone.
            ("scale", "1\nwindow.maximized = true"),
            // A choice's id exactly.
            ("theme", "Light"),
            ("window.maximized", "yes"),
            ("window.size", "[1200, 0]"),
            ("window.size", "1200"),
            ("scopes.channels", "[\"red\", \"ultraviolet\"]"),
            ("keys", "[]"),
        ] {
            let given = format!("{key}={value}");
            assert_eq!(conf(&[&given]).map(drop), unreadable(key, value), "{given}");
        }
        // Named as typed, each.
        let errors = [conf(&["density=1"]), conf(&["scale=big"]), conf(&["scale"])];
        let errors = errors.map(|e| e.expect_err("refused").to_string());
        assert_eq!(
            errors,
            [
                "--conf density: no setting this program reads",
                "--conf scale: cannot be \"big\"",
                "--conf \"scale\" is not KEY=VALUE",
            ]
        );
        // One that cannot be read is refused even when another follows it.
        assert_eq!(
            conf(&["scale=big", "scale=2"]).map(drop),
            unreadable("scale", "big")
        );
    }

    #[test]
    fn the_last_conf_of_a_setting_counts() {
        let overrides = conf(&["scale=9", "theme=light", "scale=2.5"]).expect("read");
        let store = Store::nowhere(ALL).with_overrides(&overrides);
        assert_eq!((store.get(&SCALE), store.get(&THEME)), (2.5, Theme::Light));
        // The first's warning went with it.
        assert_eq!(overrides.warnings().count(), 0);
    }

    /// `--conf`, then the file, then the default; a flag of a setting's own, before all of
    /// them, is its owner's (the engine's `--adapter`).
    #[test]
    fn a_conf_comes_before_the_file_and_the_file_before_the_default() {
        let folder = Folder::holding(
            "conf-precedence",
            "version = 1\nscale = 2\ntheme = \"light\"\n",
        );
        let overrides = conf(&["scale=1.5", "side_panel.width=500"]).expect("read");
        let store = folder.open().with_overrides(&overrides);
        assert_eq!(store.get(&SCALE), 1.5);
        assert_eq!(store.get(&THEME), Theme::Light);
        assert_eq!(store.get(&TAB), "adjust");
        // Kept for the run, a computed default's too.
        assert_eq!(store.kept(&WIDTH), Some(500.0));
        let overridden: Vec<&str> = ALL
            .iter()
            .filter(|setting| store.overridden(**setting))
            .map(|setting| setting.id())
            .collect();
        assert_eq!(overridden, ["scale", "side_panel.width"]);
        assert!(!store.overridden(&THEME));
        drop(store);
        let read = folder.open();
        assert_eq!((read.get(&SCALE), read.kept(&WIDTH)), (2.0, None));
    }

    /// `--conf`'s values are never written: a set or a reset of one is refused, the file
    /// keeps its own, and the other settings are written as without them.
    #[test]
    fn a_conf_is_never_written_and_a_set_of_it_changes_nothing() {
        let text = "# mine\nversion = 1\nscale = 2\n";
        let folder = Folder::holding("conf-never-written", text);
        let overrides = conf(&["scale=1.5", "theme=light", "side_panel.width=500"]);
        let overrides = overrides.expect("read");
        let mut store = folder.open().with_overrides(&overrides);
        // Set and reset alone: nothing to write, so the file stays, comment and all.
        store.set(&SCALE, 1.3);
        store.set(&THEME, Theme::Dark);
        store.reset(&THEME);
        store.reset(&WIDTH);
        assert!(!store.dirty);
        assert_eq!((store.get(&SCALE), store.get(&THEME)), (1.5, Theme::Light));
        store.flush();
        drop(store);
        assert_eq!(folder.text(), text);
        // With every other setting changed: those written, these as the file had them.
        let mut store = folder.open().with_overrides(&overrides);
        everything(&mut store);
        store.flush();
        assert_eq!(store.get(&SCALE), 1.5);
        let expected = "\
# Apertrust's settings. Apertrust writes here only what changes, without comments;
# keys it does not know stay.

version = 1
name = \"mine\"
scale = 2

[edit]
tab = \"optics\"

[edit.groups.exposure]
mapping = true

[edit.sections]
exposure = false
tone_curve = true

[keys]
quit = []
undo = [\"Ctrl+K\"]

[scopes]
channels = [\"blue\", \"red\"]

[window]
maximized = true
position = [-80, 50]
scale_factor = 2.0
size = [1280, 800]
";
        assert_eq!(folder.text(), expected);
    }

    #[test]
    fn stale_temporary_files_are_removed_as_the_store_opens() {
        let folder = Folder::holding("stale", "version = 1\n");
        let [stale, fresh, mine] = [
            "settings.toml.123-4.tmp",
            "settings.toml.124-5.tmp",
            "settings.toml.mine.tmp",
        ]
        .map(|name| folder.0.join(name));
        let old = std::time::SystemTime::now() - Duration::from_secs(120);
        for path in [&stale, &fresh, &mine] {
            std::fs::write(path, "x").expect("written");
        }
        for path in [&stale, &mine] {
            let file = std::fs::File::options()
                .write(true)
                .open(path)
                .expect("there");
            file.set_modified(old).expect("dated");
        }
        // Another Apertrust's under way, and a person's own, stay.
        let _store = folder.open();
        assert_eq!(
            folder.files(),
            [
                "settings.toml",
                "settings.toml.124-5.tmp",
                "settings.toml.mine.tmp"
            ]
        );
    }

    #[test]
    fn two_storages_in_one_process_never_share_a_temporary_file() {
        let folder = Folder::new("one-process");
        let writers: Vec<_> = (0..4)
            .map(|i| {
                let mut storage = FileStorage::new(folder.settings());
                std::thread::spawn(move || {
                    (0..50).all(|n| storage.write(&format!("version = 1\n# {i}-{n}\n")).is_ok())
                })
            })
            .collect();
        for writer in writers {
            assert!(writer.join().expect("joined"), "a write failed");
        }
        assert_eq!(
            folder.files(),
            ["settings.toml"],
            "no temporary file is left"
        );
    }

    #[test]
    #[should_panic(expected = "settings: theme holds a value of another type")]
    fn a_static_of_another_type_is_refused() {
        static THEME_AS_TEXT: Setting<&'static str> = setting("theme", Kind::Text, Is("dark"));
        Store::nowhere(ALL).set(&THEME_AS_TEXT, "light".to_owned());
    }

    #[test]
    fn a_newer_version_is_read_but_never_overwritten() {
        let text =
            "version = 2\ntheme = \"light\"\nlayout = \"compact\"\n[window]\nsize = [1400, 900]\n";
        let folder = Folder::holding("newer", text);
        let mut store = folder.open();
        assert_eq!(store.get(&THEME), Theme::Light);
        assert_eq!(store.get(&SIZE), [1400.0, 900.0]);
        assert!(store.writer.is_none());
        assert!(
            store.warnings().iter().any(|w| w.contains("version 2")),
            "{:?}",
            store.warnings()
        );
        everything(&mut store);
        store.flush();
        assert_eq!(folder.text(), text);
        assert_eq!(folder.files(), ["settings.toml"]);
        // Nor one a newer Apertrust wrote since this one read it.
        let folder = Folder::holding("newer-since", "version = 1\n");
        let mut store = folder.open();
        std::fs::write(folder.settings(), text).expect("newer");
        store.set(&THEME, Theme::Dark);
        store.set(&SCALE, 2.0);
        store.flush();
        assert_eq!(folder.text(), text);
    }

    #[test]
    fn a_version_that_cannot_be_read_is_read_as_this_build_s() {
        for (name, text, warning) in [
            (
                "version-missing",
                "theme = \"light\"\n",
                "version missing; read as version 1",
            ),
            (
                "version-text",
                "version = \"one\"\ntheme = \"light\"\n",
                "version \"one\" is no version; read as version 1",
            ),
        ] {
            let folder = Folder::holding(name, text);
            let store = folder.open();
            assert_eq!(store.warnings(), [warning]);
            assert_eq!(store.get(&THEME), Theme::Light);
        }
    }

    #[test]
    fn a_file_that_cannot_be_read_is_never_overwritten() {
        let memory = Memory {
            unreadable: true,
            ..Memory::default()
        };
        let mut store = memory.open();
        assert_eq!(names(&store), ["cannot"]);
        assert!(store.warnings()[0].contains("not overwritten"));
        everything(&mut store);
        store.flush();
        assert_eq!(memory.writes(), 0);
    }

    #[test]
    fn migrations_run_on_an_older_file_before_it_is_read() {
        // Each migrates from the version before it: the first from 1.
        for (i, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(migration.from, i as i64 + 1, "migration {i}");
        }
        assert_eq!(VERSION, 1 + MIGRATIONS.len() as i64);
        assert_eq!(VERSION, 1);
        // A key that moved at version 2, and one that changed meaning at version 3.
        let moved = Migration {
            from: 1,
            migrate: |table| {
                if let Some(theme) = table.remove("colours") {
                    table.insert("theme".to_owned(), theme);
                }
            },
        };
        let halved = Migration {
            from: 2,
            migrate: |table| {
                if let Some(Value::Float(scale)) = table.get_mut("scale") {
                    *scale /= 2.0;
                }
            },
        };
        let migrations = [moved, halved];
        let table = |text: &str| text.parse::<Table>().expect("TOML");
        let mut one = table("colours = \"light\"\nscale = 3.0\n");
        migrate(&mut one, 1, &migrations);
        assert_eq!(one, table("theme = \"light\"\nscale = 1.5\n"));
        let mut two = table("colours = \"light\"\nscale = 3.0\n");
        migrate(&mut two, 2, &migrations);
        assert_eq!(two, table("colours = \"light\"\nscale = 1.5\n"));
    }

    #[test]
    fn opening_and_closing_leaves_the_file_as_it_was() {
        let text = "# mine\nversion = 1\nscale = 2\ntheme = \"light\"\n[keys]\nredo = \"Ctrl+R\"\n";
        let folder = Folder::holding("unchanged", text);
        let mut store = folder.open();
        let start = Instant::now();
        // A change, undone before it stood still for a second.
        store.set(&SIZE, [800.0, 600.0]);
        assert_eq!(store.poll(start), Some(DELAY));
        store.reset(&SIZE);
        store.set(&KEYS, store.get(&KEYS));
        store.poll(start + Duration::from_millis(16));
        store.poll(start + DELAY * 2);
        store.flush();
        assert_eq!(folder.text(), text);
    }

    #[test]
    fn nothing_is_written_per_frame_only_once_the_settings_stand_still() {
        let memory = Memory::default();
        let mut store = memory.open();
        let frame = Duration::from_millis(16);
        let start = Instant::now();
        assert_eq!(store.poll(start), None);
        // Sixty frames of a window being dragged: each frame changes the settings.
        let mut now = start;
        for i in 1..=60 {
            now = start + frame * i;
            store.set(&SIZE, [1000.0 + i as f32, 700.0]);
            assert_eq!(store.poll(now), Some(DELAY), "frame {i}");
        }
        // Then frames without a change, until a second has passed: handed over once.
        let last_change = now;
        let mut handed = Vec::new();
        for i in 1..=120 {
            now = last_change + frame * i;
            let waiting = store.changed.is_some();
            match store.poll(now) {
                Some(wait) => assert_eq!(wait, DELAY - frame * i),
                None if waiting => handed.push(now - last_change),
                None => {}
            }
        }
        assert_eq!(handed.len(), 1);
        assert!(handed[0] >= DELAY && handed[0] < DELAY + frame);
        // A change undone before the second is no change: back at its default, which the
        // file does not hold, it keeps nothing to write.
        store.set(&SCALE, 1.5);
        store.poll(now + frame);
        store.set(&SCALE, 1.0);
        assert_eq!(store.kept(&SCALE), None);
        store.poll(now + frame * 2);
        assert_eq!(store.poll(now + DELAY * 2), None);
        // A change the exit catches before its second is written then. The writer joins
        // what it has not written yet, so these are one write or two.
        store.set(&SCALE, 2.0);
        assert_eq!(store.poll(now + DELAY * 3), Some(DELAY));
        store.flush();
        assert!((1..=2).contains(&memory.writes()), "{}", memory.writes());
        let written = memory.open();
        assert_eq!(
            (written.get(&SIZE), written.get(&SCALE)),
            ([1060.0, 700.0], 2.0)
        );
    }

    #[test]
    fn a_failed_write_is_tried_again_on_exit() {
        let memory = Memory::failing(1);
        let mut store = memory.open();
        let start = Instant::now();
        store.set(&THEME, Theme::Light);
        store.poll(start);
        store.poll(start + DELAY);
        // Nothing changed since: the exit tries the failed write once more.
        store.flush();
        assert_eq!(memory.open().get(&THEME), Theme::Light);
        assert_eq!(memory.writes(), 1);
    }

    #[test]
    fn a_failed_write_is_tried_again_with_the_next_change() {
        let memory = Memory::failing(1);
        let mut store = memory.open();
        let start = Instant::now();
        store.set(&THEME, Theme::Light);
        store.poll(start);
        store.poll(start + DELAY);
        store.set(&SCALE, 1.5);
        store.poll(start + DELAY * 2);
        store.poll(start + DELAY * 3);
        store.flush();
        // The second write holds the first's change too.
        let written = memory.open();
        assert_eq!(
            (written.get(&THEME), written.get(&SCALE)),
            (Theme::Light, 1.5)
        );
    }

    #[test]
    fn two_stores_on_one_file_merge_setting_by_setting() {
        let folder = Folder::new("two");
        let (mut one, mut two) = (folder.open(), folder.open());
        let undo = BTreeMap::from([("undo".to_owned(), vec!["Ctrl+K".to_owned()])]);
        one.set(&THEME, Theme::Light);
        one.set(&SECTIONS, BTreeMap::from([("exposure".to_owned(), false)]));
        one.set(&KEYS, undo);
        one.set(&TAB, "optics".to_owned());
        one.flush();
        two.set(&SCALE, 1.5);
        two.set(&SECTIONS, BTreeMap::from([("tone_curve".to_owned(), true)]));
        two.set(&KEYS, BTreeMap::from([("quit".to_owned(), Vec::new())]));
        // Both change one setting: the later's stays.
        two.set(&TAB, "detail".to_owned());
        two.flush();
        let both = folder.open();
        assert_eq!((both.get(&THEME), both.get(&SCALE)), (Theme::Light, 1.5));
        let sections = [("exposure", false), ("tone_curve", true)];
        let sections = sections.map(|(k, v)| (k.to_owned(), v));
        assert_eq!(both.get(&SECTIONS), BTreeMap::from(sections));
        assert_eq!(both.get(&KEYS).len(), 2);
        assert_eq!(both.get(&TAB), "detail");
        assert_eq!(
            folder.files(),
            ["settings.toml"],
            "no temporary file is left"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_settings_file_that_is_a_link_stays_one() {
        let folder = Folder::new("link");
        let dotfiles = folder.0.join("dotfiles");
        std::fs::create_dir_all(&dotfiles).expect("a folder");
        std::fs::write(dotfiles.join("apertrust.toml"), "version = 1\n").expect("written");
        std::os::unix::fs::symlink("dotfiles/apertrust.toml", folder.settings()).expect("linked");
        let mut store = folder.open();
        store.set(&THEME, Theme::Light);
        store.flush();
        let meta = std::fs::symlink_metadata(folder.settings()).expect("there");
        assert!(meta.file_type().is_symlink());
        let target = std::fs::read_to_string(dotfiles.join("apertrust.toml")).expect("there");
        assert!(target.contains("theme = \"light\""), "{target}");
    }
}
