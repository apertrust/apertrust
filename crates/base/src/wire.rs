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

//! Serde helpers for the session protocol's types (decision 108), which more than one
//! crate's types need, each used as a field's `#[serde(with = ...)]` or
//! `#[serde(deserialize_with = ...)]`:
//!
//! - [`id()`], with [`id::option`], [`id::vec`] and [`id::pairs`]: a `&'static str` id
//!   read from text by interning it, each distinct text leaked once into a process-wide
//!   set of at most [`MAX_IDS`], each at most [`MAX_ID_LEN`] bytes, so a field keeps its
//!   type and a peer cannot grow the set without bound;
//! - [`io_error`]: an `io::Error` as its kind and its words;
//! - [`error_kind`]: an `io::ErrorKind` by its name;
//! - [`nan_as_null`]: an `f64` that is NaN by design as `null`, which JSON writes for NaN
//!   but refuses to read into an `f64`;
//! - [`system_time`]: a `SystemTime` as signed seconds and nanoseconds from 1970, before it
//!   too;
//! - [`finite()`]: a number a command carries, refused where it reads as infinite.

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

/// A `&'static str` id, as a field of a type that derives `Deserialize` is written where it
/// is the id alone or an `Option` of it, with `deserialize_with` [`id()`] or [`id::option`].
/// It is the same type: the name only keeps serde's derive from taking the field as borrowed
/// from the input, as it takes every field written `&str` or `Option<&str>`, which would
/// let the type deserialize from `'static` input alone. A `Vec` of ids needs no alias.
pub type Id = &'static str;

/// The most distinct ids [`id()`] interns in a process.
pub const MAX_IDS: usize = 4096;

/// The longest id [`id()`] interns, in bytes.
pub const MAX_ID_LEN: usize = 256;

/// Every id interned so far, each leaked once.
static IDS: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());

/// A `&'static str` id read from text: the entry already interned for the same text, or the
/// text leaked once and interned. An error past [`MAX_IDS`] distinct ids or for one longer
/// than [`MAX_ID_LEN`] bytes. In-process clients never deserialize, so they leak nothing.
pub fn id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<&'static str, D::Error> {
    // A `String`, not a `&str`, so text with escapes reads too.
    let text = String::deserialize(deserializer)?;
    intern(text).map_err(D::Error::custom)
}

/// The shapes of id field beside a single one, for `deserialize_with`.
pub mod id {
    use serde::{Deserialize, Deserializer};

    use super::intern;
    use serde::de::Error as _;

    /// An `Option<&'static str>` id, the field marked `default` too, so a missing one
    /// reads as `None`, as a plain `Option` does.
    pub fn option<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<&'static str>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(intern)
            .transpose()
            .map_err(D::Error::custom)
    }

    /// A `Vec<&'static str>` of ids.
    pub fn vec<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<&'static str>, D::Error> {
        Vec::<String>::deserialize(deserializer)?
            .into_iter()
            .map(intern)
            .collect::<Result<_, _>>()
            .map_err(D::Error::custom)
    }

    /// A `Vec<(&'static str, T)>`, each pair an id and its value.
    pub fn pairs<'de, D, T>(deserializer: D) -> Result<Vec<(&'static str, T)>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Vec::<(String, T)>::deserialize(deserializer)?
            .into_iter()
            .map(|(text, value)| intern(text).map(|id| (id, value)))
            .collect::<Result<_, _>>()
            .map_err(D::Error::custom)
    }
}

/// `text` interned in the process's set.
fn intern(text: String) -> Result<&'static str, String> {
    let mut ids = IDS.lock().unwrap_or_else(PoisonError::into_inner);
    intern_in(&mut ids, text, MAX_IDS)
}

/// `text` interned in `set`, which holds at most `max` ids.
fn intern_in(
    set: &mut BTreeSet<&'static str>,
    text: String,
    max: usize,
) -> Result<&'static str, String> {
    if text.len() > MAX_ID_LEN {
        return Err(format!("an id longer than {MAX_ID_LEN} bytes"));
    }
    if let Some(id) = set.get(text.as_str()) {
        return Ok(id);
    }
    if set.len() >= max {
        return Err("too many distinct ids".to_owned());
    }
    let id: &'static str = Box::leak(text.into_boxed_str());
    set.insert(id);
    Ok(id)
}

/// An `io::Error` as `{"kind": <its ErrorKind>, "message": <its Display>}`, read back as
/// `io::Error::new(kind, message)`: its kind and its words survive, and the error read back
/// writes the same again.
pub mod io_error {
    use std::io;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Wire {
        #[serde(with = "super::error_kind")]
        kind: io::ErrorKind,
        message: String,
    }

    pub fn serialize<S: Serializer>(error: &io::Error, serializer: S) -> Result<S::Ok, S::Error> {
        Wire {
            kind: error.kind(),
            message: error.to_string(),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<io::Error, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        Ok(io::Error::new(wire.kind, wire.message))
    }
}

/// An `io::ErrorKind` by its `Debug` name, `"NotFound"`, read back by matching every stable
/// kind of the pinned toolchain ([`error_kind::KINDS`]), anything else read as `Other`: `ErrorKind`
/// has no serde, and serde's remote derive cannot cover an enum of another crate's that is
/// `#[non_exhaustive]`.
pub mod error_kind {
    use std::io::{self, ErrorKind};

    use serde::{Deserialize, Deserializer, Serializer};

    /// Every stable `ErrorKind` of the toolchain `rust-toolchain.toml` pins, 1.99.1.
    pub const KINDS: &[ErrorKind] = &[
        ErrorKind::NotFound,
        ErrorKind::PermissionDenied,
        ErrorKind::ConnectionRefused,
        ErrorKind::ConnectionReset,
        ErrorKind::HostUnreachable,
        ErrorKind::NetworkUnreachable,
        ErrorKind::ConnectionAborted,
        ErrorKind::NotConnected,
        ErrorKind::AddrInUse,
        ErrorKind::AddrNotAvailable,
        ErrorKind::NetworkDown,
        ErrorKind::BrokenPipe,
        ErrorKind::AlreadyExists,
        ErrorKind::WouldBlock,
        ErrorKind::NotADirectory,
        ErrorKind::IsADirectory,
        ErrorKind::DirectoryNotEmpty,
        ErrorKind::ReadOnlyFilesystem,
        ErrorKind::StaleNetworkFileHandle,
        ErrorKind::InvalidInput,
        ErrorKind::InvalidData,
        ErrorKind::TimedOut,
        ErrorKind::WriteZero,
        ErrorKind::StorageFull,
        ErrorKind::NotSeekable,
        ErrorKind::QuotaExceeded,
        ErrorKind::FileTooLarge,
        ErrorKind::ResourceBusy,
        ErrorKind::ExecutableFileBusy,
        ErrorKind::Deadlock,
        ErrorKind::CrossesDevices,
        ErrorKind::TooManyLinks,
        ErrorKind::InvalidFilename,
        ErrorKind::ArgumentListTooLong,
        ErrorKind::Interrupted,
        ErrorKind::Unsupported,
        ErrorKind::UnexpectedEof,
        ErrorKind::OutOfMemory,
        ErrorKind::Other,
    ];

    pub fn serialize<S: Serializer>(
        kind: &io::ErrorKind,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{kind:?}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<io::ErrorKind, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(KINDS
            .iter()
            .copied()
            .find(|kind| format!("{kind:?}") == name)
            .unwrap_or(ErrorKind::Other))
    }
}

/// An `f64` that is NaN by design as `null`, and `null` as NaN: serde_json writes NaN as
/// `null`, then refuses to read `null` into an `f64`.
pub mod nan_as_null {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        (!value.is_nan()).then_some(*value).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        Ok(Option::<f64>::deserialize(deserializer)?.unwrap_or(f64::NAN))
    }
}

/// A `SystemTime` as `{"secs", "nanos"}` from the Unix epoch, `secs` signed and `nanos`
/// below a second, so a time before 1970, a file's from a camera with its clock unset, writes
/// too: serde's own refuses one. [`system_time::option`] for an `Option`, with `default`.
pub mod system_time {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::de::Error as _;
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    const NANOS: u32 = 1_000_000_000;

    #[derive(Serialize, Deserialize)]
    struct Wire {
        secs: i64,
        nanos: u32,
    }

    fn to_wire(time: &SystemTime) -> Option<Wire> {
        Some(match time.duration_since(UNIX_EPOCH) {
            Ok(after) => Wire {
                secs: i64::try_from(after.as_secs()).ok()?,
                nanos: after.subsec_nanos(),
            },
            Err(before) => {
                let before = before.duration();
                let secs = -i64::try_from(before.as_secs()).ok()?;
                match before.subsec_nanos() {
                    0 => Wire { secs, nanos: 0 },
                    nanos => Wire {
                        secs: secs.checked_sub(1)?,
                        nanos: NANOS - nanos,
                    },
                }
            }
        })
    }

    fn from_wire(wire: Wire) -> Option<SystemTime> {
        if wire.nanos >= NANOS {
            return None;
        }
        let whole = Duration::from_secs(wire.secs.unsigned_abs());
        let at = if wire.secs >= 0 {
            UNIX_EPOCH.checked_add(whole)?
        } else {
            UNIX_EPOCH.checked_sub(whole)?
        };
        at.checked_add(Duration::from_nanos(u64::from(wire.nanos)))
    }

    pub fn serialize<S: Serializer>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error> {
        to_wire(time)
            .ok_or_else(|| S::Error::custom("a time too far from 1970"))?
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SystemTime, D::Error> {
        from_wire(Wire::deserialize(deserializer)?)
            .ok_or_else(|| D::Error::custom("not a time this system holds"))
    }

    /// An `Option<SystemTime>`, `null` for `None`.
    pub mod option {
        use std::time::SystemTime;

        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        #[derive(Serialize, Deserialize)]
        struct Time(#[serde(with = "super")] SystemTime);

        pub fn serialize<S: Serializer>(
            time: &Option<SystemTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            time.map(Time).serialize(serializer)
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<SystemTime>, D::Error> {
            Ok(Option::<Time>::deserialize(deserializer)?.map(|Time(time)| time))
        }
    }
}

/// A number a command carries that must be finite, read with `deserialize_with`: an `f32`,
/// an array of them or a curve's points. JSON has no form for one that is not, but a number
/// past `f32`'s range, `1e39`, reads as infinite, which would then write as `null`; it is
/// refused instead.
pub fn finite<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Finite,
{
    let value = T::deserialize(deserializer)?;
    if value.finite() {
        Ok(value)
    } else {
        Err(D::Error::custom("a number past the range of an f32"))
    }
}

/// What [`finite()`] reads: whether every number of it is finite.
pub trait Finite {
    fn finite(&self) -> bool;
}

impl Finite for f32 {
    fn finite(&self) -> bool {
        self.is_finite()
    }
}

impl<const N: usize> Finite for [f32; N] {
    fn finite(&self) -> bool {
        self.iter().all(|value| value.is_finite())
    }
}

impl Finite for Vec<[f32; 2]> {
    fn finite(&self) -> bool {
        self.iter().all(Finite::finite)
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use serde::Serialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct Ids {
        #[serde(deserialize_with = "id")]
        one: Id,
        #[serde(default, deserialize_with = "id::option")]
        maybe: Option<Id>,
        #[serde(deserialize_with = "id::vec")]
        many: Vec<&'static str>,
        #[serde(deserialize_with = "id::pairs")]
        pairs: Vec<(&'static str, f32)>,
    }

    #[test]
    fn an_id_seen_twice_is_one_static_str() {
        // Written as `Id`, the fields borrow nothing from the input.
        fn owned<T: serde::de::DeserializeOwned>() {}
        owned::<Ids>();
        let json = r#"{"one":"wire-test-id","maybe":"wire-test-id","many":["wire-test-id"],"pairs":[["wire-test-id",0.5]]}"#;
        let first: Ids = serde_json::from_str(json).unwrap();
        let second: Ids = serde_json::from_str(json).unwrap();
        assert!(std::ptr::eq(first.one, second.one));
        assert!(std::ptr::eq(first.one, first.maybe.unwrap()));
        assert!(std::ptr::eq(first.one, first.many[0]));
        assert!(std::ptr::eq(first.one, first.pairs[0].0));
        assert_eq!(serde_json::to_string(&first).unwrap(), json);
        // Escaped text reads too, and an optional id left out is `None`.
        let escaped: Ids =
            serde_json::from_str(r#"{"one":"wire\u002dtest-id","many":[],"pairs":[]}"#).unwrap();
        assert!(std::ptr::eq(first.one, escaped.one));
        assert_eq!(escaped.maybe, None);
    }

    #[test]
    fn interning_stops_at_its_limit() {
        let mut set = BTreeSet::new();
        let a = intern_in(&mut set, "a".to_owned(), 2).unwrap();
        intern_in(&mut set, "b".to_owned(), 2).unwrap();
        assert!(std::ptr::eq(
            a,
            intern_in(&mut set, "a".to_owned(), 2).unwrap()
        ));
        assert_eq!(
            intern_in(&mut set, "c".to_owned(), 2),
            Err("too many distinct ids".to_owned())
        );
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn an_id_longer_than_the_limit_is_refused() {
        let mut set = BTreeSet::new();
        let longest = "x".repeat(MAX_ID_LEN);
        assert_eq!(intern_in(&mut set, longest.clone(), 2).unwrap(), longest);
        assert_eq!(
            intern_in(&mut set, "y".repeat(MAX_ID_LEN + 1), 2),
            Err("an id longer than 256 bytes".to_owned())
        );
        let error = serde_json::from_str::<Ids>(&format!(
            r#"{{"one":"{}","maybe":null,"many":[],"pairs":[]}}"#,
            "z".repeat(MAX_ID_LEN + 1)
        ))
        .unwrap_err();
        assert!(error.to_string().contains("an id longer than 256 bytes"));
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Failed {
        #[serde(with = "io_error")]
        source: io::Error,
    }

    #[test]
    fn an_io_error_keeps_its_kind_and_words() {
        let failed = Failed {
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };
        let json = serde_json::to_string(&failed).unwrap();
        assert_eq!(
            json,
            r#"{"source":{"kind":"PermissionDenied","message":"permission denied"}}"#
        );
        let read: Failed = serde_json::from_str(&json).unwrap();
        assert_eq!(read.source.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(read.source.to_string(), "permission denied");
        assert_eq!(serde_json::to_string(&read).unwrap(), json);
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Kind(#[serde(with = "error_kind")] io::ErrorKind);

    #[test]
    fn every_stable_error_kind_round_trips() {
        for &kind in error_kind::KINDS {
            let json = serde_json::to_string(&Kind(kind)).unwrap();
            assert_eq!(json, format!("\"{kind:?}\""));
            assert_eq!(serde_json::from_str::<Kind>(&json).unwrap().0, kind);
        }
        let unknown: Kind = serde_json::from_str(r#""NotAKindYet""#).unwrap();
        assert_eq!(unknown.0, io::ErrorKind::Other);
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Timed(#[serde(with = "nan_as_null")] f64);

    #[test]
    fn nan_is_null_and_null_nan() {
        assert_eq!(serde_json::to_string(&Timed(f64::NAN)).unwrap(), "null");
        assert!(serde_json::from_str::<Timed>("null").unwrap().0.is_nan());
        assert_eq!(serde_json::to_string(&Timed(1.5)).unwrap(), "1.5");
        assert_eq!(serde_json::from_str::<Timed>("1.5").unwrap().0, 1.5);
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Modified {
        #[serde(with = "system_time")]
        at: std::time::SystemTime,
        #[serde(default, with = "system_time::option")]
        maybe: Option<std::time::SystemTime>,
    }

    #[test]
    fn a_time_before_1970_writes_and_reads_back() {
        use std::time::{Duration, UNIX_EPOCH};

        let times = [
            (UNIX_EPOCH, r#"{"secs":0,"nanos":0}"#),
            (
                UNIX_EPOCH + Duration::new(1_795_000_000, 123_000_000),
                r#"{"secs":1795000000,"nanos":123000000}"#,
            ),
            (
                UNIX_EPOCH - Duration::from_secs(86_400),
                r#"{"secs":-86400,"nanos":0}"#,
            ),
            // 0.25 s before 1970 is a second before it and 0.75 s on.
            (
                UNIX_EPOCH - Duration::from_millis(250),
                r#"{"secs":-1,"nanos":750000000}"#,
            ),
        ];
        for (time, json) in times {
            let modified = Modified {
                at: time,
                maybe: Some(time),
            };
            let written = serde_json::to_string(&modified).unwrap();
            assert_eq!(written, format!(r#"{{"at":{json},"maybe":{json}}}"#));
            assert_eq!(
                serde_json::from_str::<Modified>(&written).unwrap(),
                modified
            );
        }
        let none: Modified = serde_json::from_str(r#"{"at":{"secs":0,"nanos":0}}"#).unwrap();
        assert_eq!(none.maybe, None);
        assert!(
            serde_json::from_str::<Modified>(r#"{"at":{"secs":0,"nanos":1000000000}}"#).is_err()
        );
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Numbers {
        #[serde(deserialize_with = "finite")]
        one: f32,
        #[serde(deserialize_with = "finite")]
        at: [f32; 2],
        #[serde(deserialize_with = "finite")]
        points: Vec<[f32; 2]>,
    }

    #[test]
    fn a_number_past_an_f32_is_refused() {
        let read = |json: &str| serde_json::from_str::<Numbers>(json);
        let numbers = read(r#"{"one":0.5,"at":[0.25,1e38],"points":[[0,1]]}"#).unwrap();
        assert_eq!(numbers.at, [0.25, 1e38]);
        for json in [
            r#"{"one":1e39,"at":[0,0],"points":[]}"#,
            r#"{"one":0,"at":[0,-1e39],"points":[]}"#,
            r#"{"one":0,"at":[0,0],"points":[[0,0],[1e39,1]]}"#,
        ] {
            let error = read(json).unwrap_err();
            assert!(
                error.to_string().contains("past the range of an f32"),
                "{json}"
            );
        }
    }
}
