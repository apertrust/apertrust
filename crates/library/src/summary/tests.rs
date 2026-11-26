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

use std::path::PathBuf;
use std::time::Instant;

use apertrust_raw::Exif;

use super::*;
use crate::fields::FieldValue;

/// A photo's facts: a Ricoh GR IV's, with `change` made to its EXIF.
fn shot(change: impl FnOnce(&mut Exif)) -> Arc<Shot> {
    let mut exif = Exif {
        lens: Some("GR Lens 18.3mm F2.8".to_owned()),
        exposure_time: Some(1.0 / 500.0),
        aperture: Some(5.6),
        ..Exif::default()
    };
    change(&mut exif);
    Arc::new(Shot {
        exif,
        iso: Some(200.0),
        size: [6000, 4000],
        sensor: [6064, 4040],
        format: "DNG".to_owned(),
        make: "Ricoh".to_owned(),
        alias: "GR IV".to_owned(),
        file_size: 10,
    })
}

fn taken(day: u8, hour: u8, year: u16) -> DateTime {
    DateTime {
        year,
        month: 7,
        day,
        hour,
        minute: 10,
        second: 0,
        millisecond: 0,
        offset_minutes: Some(60),
    }
}

/// The summary of photos with `shots`, all in one folder, of 10 bytes each, unedited.
fn summary(shots: &[Facts<'_>]) -> Summary {
    let folder = PathBuf::from("/photos");
    let names: Vec<String> = (0..shots.len()).map(|n| format!("R{n:04}.DNG")).collect();
    let none = Fields::default();
    let photos: Vec<Summed<'_>> = (shots.iter().zip(&names))
        .map(|(facts, name)| Summed {
            name,
            folder: &folder,
            size: 10,
            edited: false,
            facts: *facts,
            fields: &none,
        })
        .collect();
    build(&photos)
}

/// The summary of photos holding `fields`, their facts not read yet.
fn of_fields(fields: &[Fields]) -> Summary {
    let folder = PathBuf::from("/photos");
    let photos: Vec<Summed<'_>> = (fields.iter())
        .map(|fields| Summed {
            name: "R0000.DNG",
            folder: &folder,
            size: 10,
            edited: false,
            facts: Facts::Waiting,
            fields,
        })
        .collect();
    build(&photos)
}

/// A photo's fields: its title, where it has one, and its copy's name.
fn titled(title: Option<&str>, name: &str) -> Fields {
    let text = |text: &str| FieldValue::LangAlt(vec![("x-default".to_owned(), text.to_owned())]);
    Fields {
        values: title
            .map(|title| ("dc:title".to_owned(), text(title)))
            .into_iter()
            .collect(),
        name: name.to_owned(),
        copies: 1,
    }
}

/// Each field over several photos: the same where every photo holds one value, empty
/// where none holds any, else each value with its count, most common first, at most ten,
/// how many more and how many photos hold none; the copy's name the same way.
#[test]
fn fields_are_the_same_empty_or_different_with_their_counts() {
    let same = of_fields(&[titled(Some("Alfama"), "B&W"), titled(Some("Alfama"), "B&W")]);
    let title = |text: &str| FieldValue::LangAlt(vec![("x-default".to_owned(), text.to_owned())]);
    assert_eq!(
        same.field("dc:title"),
        Some(&FieldSummary::Same(title("Alfama")))
    );
    assert_eq!(
        same.field(COPY_NAME),
        Some(&FieldSummary::Same(FieldValue::Text("B&W".to_owned())))
    );
    assert_eq!(same.field("dc:description"), Some(&FieldSummary::Empty));
    // The rating's fields are no field of the tool's.
    assert_eq!(same.field(fields::RATING), None);

    let mut photos = vec![
        titled(Some("Alfama"), ""),
        titled(Some("Belém"), ""),
        titled(Some("Alfama"), ""),
        titled(None, ""),
    ];
    let different = of_fields(&photos);
    assert_eq!(
        different.field("dc:title"),
        Some(&FieldSummary::Different {
            values: vec![(title("Alfama"), 2), (title("Belém"), 1)],
            more: 0,
            empty: 1,
        })
    );
    assert_eq!(different.field(COPY_NAME), Some(&FieldSummary::Empty));

    // Twelve titles: the ten most common, ties by the value, and two more.
    photos = (0..12)
        .map(|n| titled(Some(&format!("T{n:02}")), ""))
        .chain([titled(Some("T11"), "")])
        .collect();
    let Some(FieldSummary::Different {
        values,
        more,
        empty,
    }) = of_fields(&photos).field("dc:title").cloned()
    else {
        panic!("twelve titles differ");
    };
    let shown: Vec<(String, usize)> = (values.iter())
        .map(|(value, count)| (value.shown(), *count))
        .collect();
    assert_eq!(shown[0], ("T11".to_owned(), 2));
    assert_eq!(shown[1], ("T00".to_owned(), 1));
    assert_eq!((shown.len(), more, empty), (10, 2, 0));
}

fn keys(values: &[(FactValue, usize)]) -> Vec<(Key, usize)> {
    values.iter().map(|(v, n)| (v.key.clone(), *n)).collect()
}

#[test]
fn two_photos_the_same_share_each_value() {
    let (a, b) = (shot(|_| {}), shot(|_| {}));
    let summary = summary(&[Facts::Read(&a), Facts::Read(&b)]);
    assert_eq!(summary.photos, 2);
    let Some(FactSummary::Same(lens)) = summary.get("lens") else {
        panic!("{:?}", summary.get("lens"));
    };
    assert_eq!(lens.key, Key::Text("GR Lens 18.3mm F2.8".to_owned()));
    assert!(lens.shot.is_some());
    assert_eq!(summary.get("software"), Some(&FactSummary::None));
    // The make the camera's where the file names none.
    let Some(FactSummary::Same(make)) = summary.get("make") else {
        panic!();
    };
    assert_eq!(make.key, Key::Text("Ricoh".to_owned()));
    assert!(matches!(summary.get("folder"), Some(FactSummary::Same(_))));
    assert_eq!(
        summary.get("size_on_disk"),
        Some(&FactSummary::Total { bytes: 20 })
    );
}

#[test]
fn two_photos_that_differ_or_lack_a_value_are_different() {
    let a = shot(|_| {});
    let b = shot(|exif| exif.lens = Some("Other".to_owned()));
    let c = shot(|exif| exif.lens = None);
    let summary = summary(&[Facts::Read(&a), Facts::Read(&b)]);
    let Some(FactSummary::Different {
        values,
        more,
        missing,
    }) = summary.get("lens")
    else {
        panic!();
    };
    assert_eq!((values.len(), *more, *missing), (2, 0, 0));
    // A value all share where some have none is not shared.
    let summary = super::tests::summary(&[Facts::Read(&a), Facts::Read(&a), Facts::Read(&c)]);
    let Some(FactSummary::Different {
        values,
        more,
        missing,
    }) = summary.get("lens")
    else {
        panic!();
    };
    let lens = Key::Text("GR Lens 18.3mm F2.8".to_owned());
    assert_eq!((keys(values), *more, *missing), (vec![(lens, 2)], 0, 1));
}

#[test]
fn ten_values_are_kept_the_most_common_first_ties_by_value() {
    let shots: Vec<Arc<Shot>> = (0..14)
        .map(|n| shot(|exif| exif.serial_number = Some(format!("S{:02}", n.min(12)))))
        .collect();
    let facts: Vec<Facts<'_>> = shots.iter().map(Facts::Read).collect();
    let summary = summary(&facts);
    let Some(FactSummary::Different {
        values,
        more,
        missing,
    }) = summary.get("serial_number")
    else {
        panic!();
    };
    // S12 is on two photos; the rest on one each, in order.
    let listed = keys(values);
    assert_eq!(listed[0], (Key::Text("S12".to_owned()), 2));
    assert_eq!(listed[1], (Key::Text("S00".to_owned()), 1));
    assert_eq!(listed[9], (Key::Text("S08".to_owned()), 1));
    assert_eq!((values.len(), *more, *missing), (10, 3, 0));
}

/// The range's ends' times, by their shots.
fn ends(summary: &Summary, id: &str) -> (Option<DateTime>, Option<DateTime>, usize) {
    let Some(FactSummary::Range { low, high, missing }) = summary.get(id) else {
        panic!("{:?}", summary.get(id));
    };
    let at = |value: &FactValue| value.shot.as_ref().and_then(|shot| shot.exif.taken);
    (at(low), at(high), *missing)
}

#[test]
fn dates_taken_range_from_the_earliest_to_the_latest() {
    for (early, late) in [
        (taken(4, 8, 2026), taken(4, 9, 2026)),
        (taken(4, 8, 2026), taken(6, 8, 2026)),
        (taken(8, 8, 2025), taken(4, 8, 2026)),
    ] {
        let shots = [
            shot(|exif| exif.taken = Some(late)),
            shot(|exif| exif.taken = Some(early)),
            shot(|exif| exif.taken = None),
        ];
        let facts: Vec<Facts<'_>> = shots.iter().map(Facts::Read).collect();
        assert_eq!(
            ends(&summary(&facts), "date_taken"),
            (Some(early), Some(late), 1)
        );
    }
}

#[test]
fn exposure_times_apertures_and_isos_range_as_shown() {
    let a = shot(|exif| exif.exposure_time = Some(1.0 / 250.0));
    let b = shot(|exif| exif.exposure_time = Some(0.004));
    // 1/250 and 0.004 s are written alike: one value.
    let same = summary(&[Facts::Read(&a), Facts::Read(&b)]);
    assert!(matches!(
        same.get("exposure_time"),
        Some(FactSummary::Same(_))
    ));
    // 1/8000 and 1/8001 s are written apart: a range.
    let fast = shot(|exif| exif.exposure_time = Some(1.0 / 8000.0));
    let faster = shot(|exif| exif.exposure_time = Some(1.0 / 8001.0));
    let apart = summary(&[Facts::Read(&fast), Facts::Read(&faster)]);
    assert!(matches!(
        apart.get("exposure_time"),
        Some(FactSummary::Range { .. })
    ));
    let c = shot(|exif| {
        exif.exposure_time = Some(1.0 / 60.0);
        exif.aperture = Some(8.0);
    });
    let d = shot(|exif| exif.exposure_time = Some(1.0 / 2000.0));
    let summary = summary(&[Facts::Read(&a), Facts::Read(&c), Facts::Read(&d)]);
    let Some(FactSummary::Range { low, high, .. }) = summary.get("exposure_time") else {
        panic!();
    };
    let time = |v: &FactValue| v.shot.as_ref().and_then(|s| s.exif.exposure_time);
    assert_eq!(
        (time(low), time(high)),
        (Some(1.0 / 2000.0), Some(1.0 / 60.0))
    );
    let Some(FactSummary::Range { low, high, .. }) = summary.get("aperture") else {
        panic!();
    };
    assert_eq!(
        (low.key.clone(), high.key.clone()),
        (Key::Number(56), Key::Number(80))
    );
    assert!(matches!(summary.get("iso"), Some(FactSummary::Same(_))));
}

#[test]
fn photos_not_read_yet_wait_and_missing_ones_count_as_none() {
    let a = shot(|_| {});
    let waiting = summary(&[Facts::Read(&a), Facts::Waiting, Facts::Missing]);
    assert_eq!(
        waiting.get("lens"),
        Some(&FactSummary::Waiting { read: 2, of: 3 })
    );
    // The listing's rows do not wait.
    assert_eq!(
        waiting.get("size_on_disk"),
        Some(&FactSummary::Total { bytes: 30 })
    );
    assert!(matches!(waiting.get("edited"), Some(FactSummary::Same(_))));
    // A cloud file not on the disk is never read: it has none.
    let read = summary(&[Facts::Read(&a), Facts::Missing]);
    let Some(FactSummary::Different { missing, .. }) = read.get("lens") else {
        panic!();
    };
    assert_eq!(*missing, 1);
    assert_eq!(read.get("software"), Some(&FactSummary::None));
}

/// 10,000 photos are summarised in under 16 ms on the M4 in a release build, about 100 ms in
/// a debug one: the test fails above 50 ms in a release build, CI's machines' bound, and
/// above 500 ms in a debug one, as CI runs it.
#[test]
fn ten_thousand_photos_are_summarised_within_the_budget() {
    let shots: Vec<Arc<Shot>> = (0..10_000)
        .map(|n| {
            shot(|exif| {
                exif.taken = Some(taken((n % 28 + 1) as u8, (n % 24) as u8, 2026));
                exif.exposure_time = Some(1.0 / (60 + n % 100) as f32);
                exif.serial_number = Some(format!("S{}", n % 40));
            })
        })
        .collect();
    let facts: Vec<Facts<'_>> = shots.iter().map(Facts::Read).collect();
    let start = Instant::now();
    let summary = summary(&facts);
    let took = start.elapsed();
    assert_eq!(summary.photos, 10_000);
    let bound = if cfg!(debug_assertions) { 500 } else { 50 };
    assert!(took.as_millis() < bound, "{took:?}");
}

/// A bag's items in another order are the same value; a sequence's are not.
#[test]
fn a_bag_compares_its_items_in_any_order_and_a_sequence_in_its_own() {
    let list = |property: &str, items: [&str; 2]| Fields {
        values: vec![(
            property.to_owned(),
            FieldValue::List(items.map(str::to_owned).to_vec()),
        )],
        name: String::new(),
        copies: 1,
    };
    let bag = of_fields(&[
        list("dc:publisher", ["Agency", "Archive"]),
        list("dc:publisher", ["Archive", "Agency"]),
    ]);
    assert!(matches!(
        bag.field("dc:publisher"),
        Some(FieldSummary::Same(_))
    ));
    let seq = of_fields(&[
        list("dc:creator", ["Ana", "Rui"]),
        list("dc:creator", ["Rui", "Ana"]),
    ]);
    assert!(matches!(
        seq.field("dc:creator"),
        Some(FieldSummary::Different { .. })
    ));
}
