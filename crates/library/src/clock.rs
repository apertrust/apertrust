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

//! Hybrid logical clocks and the machine id (decisions 47 and 52): what orders the steps,
//! snapshots and entries several computers write, and what tells their machines apart.
//!
//! A [`Clock`] is UTC milliseconds, a counter and the [`MachineId`]. [`Clock::now`] never
//! goes back, even when the wall clock does: where the last clock is not behind the wall
//! clock, it keeps the last clock's time and raises the counter. A machine that reads a
//! later clock moves its own past it by passing [`Clock::observe`]'s result as the last,
//! which ignores another machine's clock more than [`MAX_DRIFT`] past this machine's wall
//! clock, so one sidecar dated far ahead never pins every reader's clock.
//!
//! Clocks order by time, then counter, then machine, so any two differ and every machine
//! orders them alike.
//!
//! In a sidecar a clock is three fields of Apertrust's namespace ([`Clock::fields`],
//! [`Clock::from_fields`]): [`WHEN`], an XMP Date in UTC with milliseconds
//! (`2026-03-14T14:02:11.120Z`), so other tools read it as a date; [`COUNTER`], a whole
//! number; and [`MACHINE`], eight hex digits.

use std::fmt;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat};
use thiserror::Error;

#[cfg(test)]
mod tests;

/// A clock's time, `apertrust:when`.
pub const WHEN: &str = "when";
/// A clock's counter, `apertrust:counter`.
pub const COUNTER: &str = "counter";
/// A clock's machine id, `apertrust:machine`.
pub const MACHINE: &str = "machine";

/// The latest time an XMP Date holds, 9999-12-31T23:59:59.999Z, in milliseconds.
pub const MAX_WHEN: u64 = 253_402_300_799_999;

/// How far past this machine's wall clock a clock it reads may be and still move its own
/// clock on ([`Clock::observe`]): 5 minutes, Kerberos's default clock skew. Computers that
/// set their clocks from the network agree within seconds; a clock further ahead is wrong,
/// as from a computer whose date or time zone is set wrongly, and is ignored.
pub const MAX_DRIFT: u64 = 5 * 60 * 1000;

/// The context of the machine id's hash, BLAKE3's key derivation, so it matches no other
/// hash of the same inputs. Changing it changes every machine's id.
const MACHINE_CONTEXT: &str = "apertrust.org 2026 machine id";

/// A hybrid logical clock: UTC milliseconds, a counter and the machine that made it
/// (decision 47). Ordered by `when`, then `counter`, then `machine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Clock {
    /// Milliseconds since 1970-01-01T00:00:00Z, at most [`MAX_WHEN`].
    pub when: u64,
    /// Raised for each clock made while `when` stays the same, 0 when it moves on.
    pub counter: u32,
    pub machine: MachineId,
}

impl Clock {
    /// The next clock on `machine` after `last`, the latest clock it has made or read, at
    /// the wall clock's time now.
    pub fn now(last: Option<&Clock>, machine: MachineId) -> Clock {
        Clock::at(wall_clock(), last, machine)
    }

    /// The next clock on `machine` after `last`, with the wall clock at `wall`
    /// milliseconds: `wall` with counter 0 where it is past `last`'s time, else `last`'s
    /// time with its counter raised by one, so a wall clock set back never orders a new
    /// clock before an old one. A counter at its maximum moves the time on one millisecond.
    ///
    /// The time never passes [`MAX_WHEN`], the latest an XMP Date holds, so every clock
    /// reads back as it was written: a wall clock past it counts as `MAX_WHEN`, and at
    /// `MAX_WHEN` only the counter rises, staying at its maximum once there.
    pub fn at(wall: u64, last: Option<&Clock>, machine: MachineId) -> Clock {
        let wall = wall.min(MAX_WHEN);
        match last {
            Some(last) if last.when >= wall => match last.counter.checked_add(1) {
                Some(counter) => Clock {
                    when: last.when,
                    counter,
                    machine,
                },
                None if last.when < MAX_WHEN => Clock {
                    when: last.when + 1,
                    counter: 0,
                    machine,
                },
                None => Clock {
                    when: last.when,
                    counter: u32::MAX,
                    machine,
                },
            },
            _ => Clock {
                when: wall,
                counter: 0,
                machine,
            },
        }
    }

    /// The latest clock the machine `me` keeps after reading `read`, with `last` the latest
    /// it had and its wall clock at `wall` milliseconds: the later of the two, but `last`
    /// alone where `read` is another machine's more than [`MAX_DRIFT`] past `wall`. Passed
    /// to [`Clock::at`] or [`Clock::now`] as the last, it orders the next clock after every
    /// clock read within the bound and after every clock of `me`'s own, so a wall clock set
    /// back never orders a new step before one this machine made; no other machine's file
    /// moves its clocks more than the bound past its wall clock. A clock ignored is still
    /// read and kept where it is.
    pub fn observe(last: Option<&Clock>, read: &Clock, wall: u64, me: MachineId) -> Option<Clock> {
        let honoured = read.machine == me || read.when <= wall.saturating_add(MAX_DRIFT);
        match last {
            Some(last) if !honoured || last >= read => Some(*last),
            _ if honoured => Some(*read),
            _ => None,
        }
    }

    /// Its three fields as a sidecar writes them, each a local name in Apertrust's
    /// namespace with its text: [`WHEN`], [`COUNTER`], [`MACHINE`].
    pub fn fields(&self) -> [(&'static str, String); 3] {
        [
            (WHEN, format_when(self.when)),
            (COUNTER, self.counter.to_string()),
            (MACHINE, self.machine.to_string()),
        ]
    }

    /// A clock from its three fields' text, as [`Clock::fields`] writes them.
    pub fn from_fields(when: &str, counter: &str, machine: &str) -> Result<Clock, ClockError> {
        Ok(Clock {
            when: parse_when(when)?,
            counter: parse_counter(counter)?,
            machine: machine.parse()?,
        })
    }
}

/// Milliseconds as an XMP Date in UTC with milliseconds: `2026-03-14T14:02:11.120Z`. A
/// time past [`MAX_WHEN`] is written as `MAX_WHEN`, the latest an XMP Date holds.
pub fn format_when(when: u64) -> String {
    let millis = i64::try_from(when.min(MAX_WHEN)).unwrap_or(i64::MAX);
    DateTime::from_timestamp_millis(millis)
        .unwrap_or_default()
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// An XMP Date with a time to the second and a time zone, `Z` or an offset, or RFC 3339's
/// other forms (chrono's reading, which also takes a space for the `T`), as milliseconds
/// since 1970 in UTC; digits past the millisecond are dropped. A date before 1970 or past
/// [`MAX_WHEN`], or one without a time or a zone, is refused. [`format_when`] writes only
/// the one form.
pub fn parse_when(text: &str) -> Result<u64, ClockError> {
    let error = || ClockError::When(text.to_owned());
    let date = DateTime::parse_from_rfc3339(text).map_err(|_| error())?;
    u64::try_from(date.timestamp_millis())
        .ok()
        .filter(|when| *when <= MAX_WHEN)
        .ok_or_else(error)
}

/// A counter's text: decimal digits alone.
fn parse_counter(text: &str) -> Result<u32, ClockError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ClockError::Counter(text.to_owned()));
    }
    text.parse()
        .map_err(|_| ClockError::Counter(text.to_owned()))
}

/// The wall clock in milliseconds since 1970, 0 for a clock set before it.
fn wall_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// A field of a clock that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClockError {
    #[error("not an XMP Date with a time and a time zone, from 1970 to 9999: {0:?}")]
    When(String),
    #[error("not a clock's counter: {0:?}")]
    Counter(String),
    #[error("not a machine id of eight hex digits: {0:?}")]
    Machine(String),
}

/// The machine id: eight hex digits, a hash of the system's machine id and the user
/// (decision 47), so two accounts on one computer are two machines. It is worked out
/// each time Apertrust starts and never stored in the user data folder, so a copied user
/// data folder never makes two computers one machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineId(u32);

impl MachineId {
    /// The machine of a clock no machine made: version 1's edits read as one step.
    pub const UNKNOWN: MachineId = MachineId(0);

    /// This machine's id, worked out once a run: the system's machine id, read through
    /// `machine-uid` (`/var/lib/dbus/machine-id`, else `/etc/machine-id`, on Linux, the
    /// hardware UUID on macOS, the registry's `MachineGuid` on Windows), hashed with the
    /// user's home folder ([`MachineId::from_system`]). Where the system's id cannot be
    /// read, a random id for this run, with a warning.
    pub fn current() -> MachineId {
        static CURRENT: OnceLock<MachineId> = OnceLock::new();
        *CURRENT.get_or_init(|| {
            let system = match machine_uid::get() {
                Ok(id) if readable(Some(&id)).is_some() => Some(id),
                Ok(id) => {
                    tracing::warn!(
                        "the system's machine id is {id:?}: this run's machine id is random"
                    );
                    None
                }
                Err(error) => {
                    tracing::warn!(
                        "the system's machine id cannot be read ({error}): this run's machine \
                         id is random"
                    );
                    None
                }
            };
            // std's home folder, not base's: the library takes only base's `fs` part. Both
            // read `HOME`, then the account's entry, on Unix, and the profile on Windows.
            let home = std::env::home_dir().unwrap_or_default();
            MachineId::from_system(system.as_deref(), &home.to_string_lossy(), random)
        })
    }

    /// The id of the system's machine id `system` and the user's home folder `home`, by
    /// [`MachineId::from_parts`]. A system id that could not be read (`None`), is empty or
    /// is systemd's `uninitialized`, as early in a first boot, names no machine: the id is
    /// then `random`'s, never the home folder's alone, which two computers can share.
    pub fn from_system(
        system: Option<&str>,
        home: &str,
        random: impl FnOnce() -> u32,
    ) -> MachineId {
        match readable(system) {
            Some(system) => MachineId::from_parts(system, home),
            None => MachineId(random().max(1)),
        }
    }

    /// The id of the system's machine id `system` and the user `user`: the first 32 bits of
    /// their BLAKE3 hash in key derivation mode, so the system's id, which systemd asks to
    /// keep confidential, cannot be read back from it.
    pub fn from_parts(system: &str, user: &str) -> MachineId {
        let mut hasher = blake3::Hasher::new_derive_key(MACHINE_CONTEXT);
        hasher.update(&(system.len() as u64).to_le_bytes());
        hasher.update(system.as_bytes());
        hasher.update(user.as_bytes());
        let bytes = *hasher.finalize().as_bytes();
        MachineId(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

/// A system machine id that names a machine: not empty, and not systemd's
/// `uninitialized`.
fn readable(system: Option<&str>) -> Option<&str> {
    system
        .map(str::trim)
        .filter(|system| !system.is_empty() && *system != "uninitialized")
}

/// A random machine id's number, from the system's random source; where even that fails,
/// a hash of the time and the process's id.
fn random() -> u32 {
    getrandom::u32().unwrap_or_else(|error| {
        tracing::warn!("no random number from the system ({error}): the time's instead");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let mut hasher = blake3::Hasher::new_derive_key(MACHINE_CONTEXT);
        hasher.update(&nanos.to_le_bytes());
        hasher.update(&std::process::id().to_le_bytes());
        let bytes = *hasher.finalize().as_bytes();
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    })
}

impl fmt::Display for MachineId {
    /// Eight lowercase hex digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

impl std::str::FromStr for MachineId {
    type Err = ClockError;

    /// Eight hex digits, in either case.
    fn from_str(text: &str) -> Result<MachineId, ClockError> {
        if text.len() != 8 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ClockError::Machine(text.to_owned()));
        }
        u32::from_str_radix(text, 16)
            .map(MachineId)
            .map_err(|_| ClockError::Machine(text.to_owned()))
    }
}
