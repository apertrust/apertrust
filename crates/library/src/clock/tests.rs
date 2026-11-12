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

use super::*;

/// 2026-03-14T14:02:11.120Z.
const PI_DAY: u64 = 1_773_496_931_120;

fn machine(id: u32) -> MachineId {
    MachineId(id)
}

fn clock(when: u64, counter: u32, id: u32) -> Clock {
    Clock {
        when,
        counter,
        machine: machine(id),
    }
}

#[test]
fn the_first_clock_is_the_wall_clock() {
    assert_eq!(Clock::at(PI_DAY, None, machine(7)), clock(PI_DAY, 0, 7));
}

#[test]
fn a_later_wall_clock_starts_the_counter_again() {
    let last = clock(PI_DAY, 5, 7);
    assert_eq!(
        Clock::at(PI_DAY + 1, Some(&last), machine(7)),
        clock(PI_DAY + 1, 0, 7)
    );
}

#[test]
fn the_counter_rises_while_the_time_stays() {
    let mut last = Clock::at(PI_DAY, None, machine(7));
    for counter in 1..=3 {
        last = Clock::at(PI_DAY, Some(&last), machine(7));
        assert_eq!(last, clock(PI_DAY, counter, 7));
    }
}

#[test]
fn a_wall_clock_set_back_never_goes_back() {
    let mut last = Clock::at(PI_DAY, None, machine(7));
    // An hour back, then creeping forward, still behind the last clock.
    for wall in [PI_DAY - 3_600_000, PI_DAY - 3_599_999, PI_DAY - 1] {
        let next = Clock::at(wall, Some(&last), machine(7));
        assert!(next > last, "{next:?} after {last:?}");
        assert_eq!(next.when, PI_DAY);
        last = next;
    }
    assert_eq!(last.counter, 3);
    // The wall clock catches up and passes it.
    assert_eq!(
        Clock::at(PI_DAY + 1, Some(&last), machine(7)),
        clock(PI_DAY + 1, 0, 7)
    );
}

#[test]
fn a_later_clock_read_from_another_machine_moves_this_one_past_it() {
    let read = clock(PI_DAY + 60_000, 4, 9);
    let next = Clock::at(PI_DAY, Some(&read), machine(7));
    assert_eq!(next, clock(PI_DAY + 60_000, 5, 7));
    assert!(next > read);
}

#[test]
fn a_full_counter_moves_the_time_on() {
    let last = clock(PI_DAY, u32::MAX, 7);
    let next = Clock::at(PI_DAY, Some(&last), machine(7));
    assert_eq!(next, clock(PI_DAY + 1, 0, 7));
    assert!(next > last);
}

#[test]
fn now_follows_the_last_clock() {
    let last = Clock::now(None, machine(7));
    let next = Clock::now(Some(&last), machine(7));
    assert!(next > last);
    // A last clock far ahead of the wall clock keeps its time.
    let ahead = clock(MAX_WHEN, 0, 9);
    assert_eq!(Clock::now(Some(&ahead), machine(7)), clock(MAX_WHEN, 1, 7));
}

#[test]
fn clocks_order_by_time_then_counter_then_machine() {
    let mut clocks = vec![
        clock(PI_DAY + 1, 0, 1),
        clock(PI_DAY, 1, 1),
        clock(PI_DAY, 0, 2),
        clock(PI_DAY, 0, 1),
        clock(PI_DAY - 1, 9, 9),
    ];
    clocks.sort();
    assert_eq!(
        clocks,
        [
            clock(PI_DAY - 1, 9, 9),
            clock(PI_DAY, 0, 1),
            clock(PI_DAY, 0, 2),
            clock(PI_DAY, 1, 1),
            clock(PI_DAY + 1, 0, 1),
        ]
    );
    // The machine id's text orders as its number does.
    assert!(machine(0x0000_00ff).to_string() < machine(0x0000_0100).to_string());
}

#[test]
fn the_fields_are_an_xmp_date_with_milliseconds_a_counter_and_eight_hex_digits() {
    let fields = clock(PI_DAY, 12, 0xa1b2_c3d4).fields();
    assert_eq!(
        fields,
        [
            (WHEN, "2026-03-14T14:02:11.120Z".to_owned()),
            (COUNTER, "12".to_owned()),
            (MACHINE, "a1b2c3d4".to_owned()),
        ]
    );
}

#[test]
fn the_fields_round_trip() {
    for original in [
        clock(PI_DAY, 0, 0x5e1f_22c9),
        clock(PI_DAY + 5, u32::MAX, 0xffff_ffff),
        clock(0, 0, 0),
        clock(1, 1, 1),
        clock(MAX_WHEN, 3, 0x0000_0001),
    ] {
        let [(_, when), (_, counter), (_, machine)] = original.fields();
        assert_eq!(Clock::from_fields(&when, &counter, &machine), Ok(original));
        assert_eq!(format_when(parse_when(&when).unwrap()), when);
    }
}

#[test]
fn the_time_keeps_its_milliseconds_and_z() {
    assert_eq!(format_when(PI_DAY), "2026-03-14T14:02:11.120Z");
    assert_eq!(format_when(PI_DAY - 120), "2026-03-14T14:02:11.000Z");
    assert_eq!(format_when(PI_DAY - 119), "2026-03-14T14:02:11.001Z");
    assert_eq!(format_when(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(format_when(MAX_WHEN), "9999-12-31T23:59:59.999Z");
    assert_eq!(format_when(u64::MAX), "9999-12-31T23:59:59.999Z");
}

#[test]
fn other_forms_of_an_xmp_date_read() {
    // Another tool may rewrite the date: fewer or more digits, or an offset.
    assert_eq!(parse_when("2026-03-14T14:02:11.12Z"), Ok(PI_DAY));
    assert_eq!(parse_when("2026-03-14T14:02:11.120999Z"), Ok(PI_DAY));
    assert_eq!(parse_when("2026-03-14T16:02:11.120+02:00"), Ok(PI_DAY));
    assert_eq!(parse_when("2026-03-14T14:02:11Z"), Ok(PI_DAY - 120));
}

#[test]
fn what_is_not_a_clock_is_refused() {
    for when in [
        "",
        "2026-03-14",
        "2026-03-14T14:02Z",
        "2026-03-14T14:02:11.120",
        "1969-12-31T23:59:59.999Z",
        "yesterday",
    ] {
        assert_eq!(
            Clock::from_fields(when, "0", "a1b2c3d4"),
            Err(ClockError::When(when.to_owned())),
            "{when}"
        );
    }
    for counter in ["", "-1", "+1", "1.0", "4294967296", " 1"] {
        assert_eq!(
            Clock::from_fields("2026-03-14T14:02:11.120Z", counter, "a1b2c3d4"),
            Err(ClockError::Counter(counter.to_owned())),
            "{counter}"
        );
    }
    for machine in [
        "",
        "a1b2c3d",
        "a1b2c3d4e",
        "a1b2c3dg",
        "+1b2c3d4",
        "0x12345",
    ] {
        assert_eq!(
            Clock::from_fields("2026-03-14T14:02:11.120Z", "0", machine),
            Err(ClockError::Machine(machine.to_owned())),
            "{machine}"
        );
    }
}

#[test]
fn a_machine_id_reads_in_either_case_and_writes_lowercase() {
    let id: MachineId = "A1B2C3D4".parse().unwrap();
    assert_eq!(id, machine(0xa1b2_c3d4));
    assert_eq!(id.to_string(), "a1b2c3d4");
    assert_eq!(MachineId::UNKNOWN.to_string(), "00000000");
}

#[test]
fn the_machine_id_is_stable() {
    assert_eq!(MachineId::current(), MachineId::current());
    let system = "4c4c4544-0042-3510-8051-b4c04f4e3432";
    assert_eq!(
        MachineId::from_parts(system, "/home/anna"),
        MachineId::from_parts(system, "/home/anna")
    );
}

#[test]
fn the_machine_id_differs_by_user_and_by_system() {
    let system = "4c4c4544-0042-3510-8051-b4c04f4e3432";
    let anna = MachineId::from_parts(system, "/home/anna");
    assert_ne!(anna, MachineId::from_parts(system, "/home/ben"));
    assert_ne!(
        anna,
        MachineId::from_parts("4c4c4544-0042-3510-8051-b4c04f4e3433", "/home/anna")
    );
    // The system's id and the user cannot trade characters.
    assert_ne!(
        MachineId::from_parts("ab", "c"),
        MachineId::from_parts("a", "bc")
    );
}

#[test]
fn the_machine_ids_hash_never_changes() {
    // A changed hash makes every machine a new one, its clocks those of a stranger.
    assert_eq!(
        MachineId::from_parts("4c4c4544-0042-3510-8051-b4c04f4e3432", "/home/anna").to_string(),
        "c608175f"
    );
}

#[test]
fn a_system_id_that_names_no_machine_gives_a_random_id() {
    let home = "/home/anna";
    for system in [None, Some(""), Some("  \n"), Some("uninitialized")] {
        assert_eq!(
            MachineId::from_system(system, home, || 0x1234_5678),
            machine(0x1234_5678),
            "{system:?}"
        );
    }
    // Never the unknown machine's.
    assert_eq!(MachineId::from_system(None, home, || 0), machine(1));
}

#[test]
fn a_readable_system_id_is_hashed_with_the_home_folder() {
    let system = "4c4c4544-0042-3510-8051-b4c04f4e3432";
    let id = MachineId::from_system(Some(system), "/home/anna", || {
        panic!("no random id where the system's reads")
    });
    assert_eq!(id, MachineId::from_parts(system, "/home/anna"));
}

#[test]
fn the_time_stops_at_the_latest_xmp_date() {
    // A wall clock past it counts as it.
    assert_eq!(Clock::at(u64::MAX, None, machine(7)), clock(MAX_WHEN, 0, 7));
    let last = clock(MAX_WHEN, 4, 9);
    assert_eq!(
        Clock::at(u64::MAX, Some(&last), machine(7)),
        clock(MAX_WHEN, 5, 7)
    );
    // At the top, a full counter stays full, never ordered before the last clock.
    let full = clock(MAX_WHEN, u32::MAX, 7);
    let next = Clock::at(MAX_WHEN, Some(&full), machine(7));
    assert_eq!(next, full);
    let [(_, when), (_, counter), (_, id)] = next.fields();
    assert_eq!(Clock::from_fields(&when, &counter, &id), Ok(next));
}

#[test]
fn a_time_past_the_latest_xmp_date_is_refused() {
    assert_eq!(parse_when("9999-12-31T23:59:59.999Z"), Ok(MAX_WHEN));
    for when in ["9999-12-31T23:59:59.999-01:00", "9999-12-31T23:00:00-23:59"] {
        assert_eq!(
            parse_when(when),
            Err(ClockError::When(when.to_owned())),
            "{when}"
        );
    }
}
