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

//! The pipeline order (decision 22): no two operations share a slot, the numbers rise
//! with the stages so the number alone gives the order, and every built-in operation sits
//! where `crates/operations/order.md`'s table puts it, on an unedited photo or not as it says.

use apertrust_operations::{KERNELS, OPERATIONS};

/// One row of `crates/operations/order.md`'s table: the operation's id, its slot as a manifest
/// prints it (`working space 28.5`), and whether an unedited photo runs it.
struct Row {
    id: String,
    slot: String,
    unedited: bool,
}

/// The rows of `crates/operations/order.md`'s table.
fn table() -> Vec<Row> {
    // A Windows checkout may end its lines with CRLF.
    let doc = include_str!("../order.md").replace("\r\n", "\n");
    doc.lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        // The header and its rule.
        .skip(2)
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let id = cells[1].split_whitespace().next().unwrap_or_default();
            Row {
                id: id.to_owned(),
                slot: cells[2].to_owned(),
                unedited: cells[3] == "yes",
            }
        })
        .collect()
}

#[test]
fn every_operation_sits_where_the_order_table_puts_it() {
    let rows = table();
    assert!(rows.len() >= OPERATIONS.len(), "{} rows", rows.len());
    for operation in OPERATIONS {
        let manifest = operation.manifest();
        let row = rows
            .iter()
            .find(|row| row.id == manifest.id)
            .unwrap_or_else(|| panic!("{} is not in order.md's table", manifest.id));
        assert_eq!(manifest.slot.to_string(), row.slot, "{}", manifest.id);
        assert_eq!(manifest.default_enabled, row.unedited, "{}", manifest.id);
    }
    // The table's own numbers rise, each once, so its order is the numbers' order.
    let numbers: Vec<f32> = rows
        .iter()
        .map(|row| {
            let number = row.slot.rsplit(' ').next().unwrap_or_default();
            number
                .parse()
                .unwrap_or_else(|_| panic!("{}: {}", row.id, row.slot))
        })
        .collect();
    assert!(
        numbers.windows(2).all(|pair| pair[0] < pair[1]),
        "{numbers:?}"
    );
}

#[test]
fn slots_are_unique_and_their_numbers_rise_with_the_stages() {
    for pair in OPERATIONS.windows(2) {
        let (a, b) = (pair[0].manifest(), pair[1].manifest());
        assert!(
            a.slot.pipeline_order(&b.slot).is_lt() && a.slot.order < b.slot.order,
            "{} ({}) comes before {} ({})",
            a.id,
            a.slot,
            b.id,
            b.slot
        );
    }
}

#[test]
fn the_kernels_are_the_operations_in_their_order() {
    let kernels: Vec<&str> = KERNELS.iter().map(|kernel| kernel.name).collect();
    let operations: Vec<&str> = OPERATIONS
        .iter()
        .map(|operation| operation.kernel().name)
        .collect();
    assert_eq!(kernels, operations);
}
