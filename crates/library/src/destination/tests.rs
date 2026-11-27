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

//! Import's destination: each Subfolder choice for a fixed import time and fixed capture
//! times, the job code's two names, the places and the `~`, and the dates each pattern
//! reads.

use std::time::{Duration, UNIX_EPOCH};

use super::*;

/// The import's time in the tests: 15 November 2026, 18:30.
const NOW: DateTime = DateTime {
    year: 2026,
    month: 11,
    day: 15,
    hour: 18,
    minute: 30,
    second: 0,
    microsecond: 0,
};

fn day(year: i32, month: u8, day: u8) -> DateTime {
    DateTime {
        year,
        month,
        day,
        hour: 9,
        minute: 41,
        ..DateTime::ORIGIN
    }
}

fn places() -> Places {
    Places {
        home: Some(PathBuf::from("/home/ana")),
        pictures: Some(PathBuf::from("/home/ana/Pictures")),
        desktop: None,
        user_name: "ana".to_owned(),
    }
}

fn settings(base: &str, subfolder: &str) -> ImportSettings {
    ImportSettings {
        base_folder: base.to_owned(),
        default_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
        subfolder: subfolder.to_owned(),
        job_code: "Wedding".to_owned(),
        places: places(),
    }
}

fn shot(name: &str, taken: DateTime) -> Shot {
    Shot {
        name: name.to_owned(),
        taken: Some(taken),
        maker: "Canon".to_owned(),
        model: "EOS R5".to_owned(),
    }
}

/// Each Subfolder choice of the Import tool, and `$(JOBCODE)` beside `$(IMPORT_NAME)`, for
/// two photos taken on two days: their folders, one for the import or one for each day.
#[test]
fn each_subfolder_choice_gives_its_folders() {
    let pictures = "/home/ana/Pictures/Apertrust";
    let cases: &[(&str, [&str; 2])] = &[
        (
            "$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)",
            ["20261115_Wedding", "20261115_Wedding"],
        ),
        (
            "$(YEAR)$(MONTH)$(DAY)_$(JOBCODE)",
            ["20261115_Wedding", "20261115_Wedding"],
        ),
        (
            "$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)",
            ["20261114_Wedding", "20261115_Wedding"],
        ),
        ("$(IMPORT_NAME)", ["Wedding", "Wedding"]),
        ("", ["", ""]),
        ("$(EXIF.YEAR)/$(EXIF.MONTH)", ["2026/11", "2026/11"]),
        ("/$(MODEL)", ["EOS R5", "EOS R5"]),
    ];
    let first = shot("IMG_0001.CR3", day(2026, 11, 14));
    let second = shot("IMG_0002.CR3", day(2026, 11, 15));
    for (subfolder, expected) in cases {
        let patterns = Patterns::parse(&settings("$(PICTURES_FOLDER)/Apertrust", subfolder))
            .unwrap_or_else(|errors| panic!("{subfolder}: {errors:?}"));
        let folders = [&first, &second].map(|shot| patterns.destination(NOW, shot).unwrap());
        let expected = expected.map(|sub| Path::new(pictures).join(sub));
        assert_eq!(folders, expected, "{subfolder}");
        assert_eq!(patterns.folder(NOW, &first), Path::new(pictures));
    }
}

/// Folder's own variables, a `~` that is the home folder, and an absolute folder as typed.
#[test]
fn folder_expands_from_the_places_and_the_home_folder() {
    let shot = shot("IMG_0001.CR3", day(2026, 11, 14));
    let cases = [
        (
            "$(PICTURES_FOLDER)/Apertrust",
            "/home/ana/Pictures/Apertrust",
        ),
        ("$(HOME)/Photos/$(YEAR)", "/home/ana/Photos/2026"),
        ("~/Photos", "/home/ana/Photos"),
        ("~", "/home/ana"),
        ("/Volumes/NAS/Photos", "/Volumes/NAS/Photos"),
        ("~ana/Photos", "~ana/Photos"),
    ];
    for (folder, expected) in cases {
        let patterns = Patterns::parse(&settings(folder, "")).unwrap();
        assert_eq!(
            patterns.destination(NOW, &shot),
            Ok(PathBuf::from(expected)),
            "{folder}"
        );
    }
}

/// Where the platform has no Pictures folder, `$(PICTURES_FOLDER)` is the home's
/// `Pictures`, as darktable takes it, never the volume's root.
#[test]
fn the_pictures_folder_falls_back_to_the_home_s() {
    let mut settings = settings("$(PICTURES_FOLDER)/Apertrust", "");
    settings.places.pictures = None;
    let patterns = Patterns::parse(&settings).unwrap();
    let shot = shot("IMG_0001.CR3", day(2026, 11, 14));
    let expected = Path::new("/home/ana").join("Pictures").join("Apertrust");
    assert_eq!(patterns.destination(NOW, &shot), Ok(expected));
    settings.places.home = None;
    assert_eq!(pictures(&settings.places), None);
}

/// A Subfolder that expands to anything but folder names, a `..`, a root or, on Windows,
/// a drive, is refused with what it expanded to, as it would lead out of Folder or replace
/// it; nested names, and a separator it starts with, stay under Folder.
#[test]
fn a_subfolder_leading_out_of_the_folder_is_refused() {
    let mut shot = shot("IMG_0001.CR3", day(2026, 11, 14));
    let cases = [
        ("..", Err("..")),
        ("../Other", Err("../Other")),
        ("Wedding/../../Other", Err("Wedding/../../Other")),
        ("./Wedding", Err("./Wedding")),
        ("$(MODEL)", Err("..")),
        ("Wedding/Day 1", Ok("Wedding/Day 1")),
        ("//Wedding", Ok("Wedding")),
    ];
    shot.model = "..".to_owned();
    for (subfolder, expected) in cases {
        let patterns = Patterns::parse(&settings("/Photos", subfolder)).unwrap();
        let expected = expected
            .map(|sub| Path::new("/Photos").join(sub))
            .map_err(str::to_owned);
        assert_eq!(patterns.destination(NOW, &shot), expected, "{subfolder}");
    }
    let patterns = Patterns::parse(&settings("/Photos", "D:/Other")).unwrap();
    let drive = patterns.destination(NOW, &shot);
    if cfg!(windows) {
        assert_eq!(drive, Err("D:/Other".to_owned()));
    } else {
        assert_eq!(drive, Ok(PathBuf::from("/Photos/D:/Other")));
    }
}

/// An unknown variable in either row is refused, each with its row and the nearest name,
/// Folder's first.
#[test]
fn a_pattern_that_cannot_be_read_names_its_row() {
    let errors = Patterns::parse(&settings("$(HOME)/x", "$(YEAR)$(MONT)")).unwrap_err();
    let [
        (
            Row::Subfolder,
            PatternError::Unknown {
                name, suggestion, ..
            },
        ),
    ] = errors.as_slice()
    else {
        panic!("{errors:?}");
    };
    assert_eq!((name.as_str(), *suggestion), ("MONT", Some("MONTH")));
    let errors = Patterns::parse(&settings("$(HOEM)", "$(YEAR")).unwrap_err();
    let rows: Vec<Row> = errors.iter().map(|(row, _)| *row).collect();
    assert_eq!(rows, [Row::Folder, Row::Subfolder]);
}

/// Which pattern reads today's date and which the day taken, for the rows' tooltips.
#[test]
fn each_pattern_says_which_dates_it_reads() {
    let uses =
        |base: &str, subfolder: &str| Patterns::parse(&settings(base, subfolder)).unwrap().uses();
    let today = Uses {
        today: true,
        ..Uses::default()
    };
    let taken = Uses {
        taken: true,
        ..Uses::default()
    };
    let none = Uses::default();
    assert_eq!(
        uses(
            "$(PICTURES_FOLDER)/Apertrust",
            "$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)"
        ),
        (none, today)
    );
    assert_eq!(
        uses(
            "$(HOME)/$(YEAR)",
            "$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)"
        ),
        (today, taken)
    );
    assert_eq!(uses("/Photos", ""), (none, none));
    let (_, photo) = uses("/Photos", "$(MODEL)");
    assert!(photo.photo && !photo.taken && !photo.today);
    // Told from the variables, never from what they expand to: a year's last two digits,
    // a part of the model, a variable nested in another's operation.
    let cases = [
        ("$(EXIF.YEAR.SHORT)", taken),
        (
            "$(MODEL:1)",
            Uses {
                photo: true,
                ..none
            },
        ),
        ("$(IMPORT_NAME-$(EXIF.MONTH))", taken),
        ("$(IMPORT_NAME/x/$(YEAR))", today),
        ("$(IMPORT_NAME)", none),
    ];
    for (subfolder, expected) in cases {
        assert_eq!(uses("/Photos", subfolder), (none, expected), "{subfolder}");
    }
}

/// A photo with no EXIF time takes its file's modified time on the local clock, as
/// darktable's import does, and the import's time where it has neither.
#[test]
fn a_photo_without_an_exif_time_takes_its_file_s_time() {
    let exif = apertrust_raw::DateTime {
        year: 2026,
        month: 11,
        day: 14,
        hour: 9,
        minute: 41,
        second: 2,
        millisecond: 120,
        offset_minutes: None,
    };
    let taken = taken(Some(exif), Some(UNIX_EPOCH), NOW);
    assert_eq!(
        (taken.year, taken.day, taken.microsecond),
        (2026, 14, 120_000)
    );
    let modified = UNIX_EPOCH + Duration::from_secs(1_763_200_000);
    assert_eq!(super::taken(None, Some(modified), NOW), local(modified));
    assert_eq!(super::taken(None, None, NOW), NOW);
}

#[test]
fn the_deepest_folder_there_is_found_without_making_any() {
    let root = tempfile::tempdir().unwrap();
    let wanted = root.path().join("a").join("b");
    assert_eq!(deepest_existing(&wanted), Some(root.path()));
    assert!(!root.path().join("a").exists());
    assert_eq!(deepest_existing(root.path()), Some(root.path()));
}
