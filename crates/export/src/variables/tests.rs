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

//! Every name darktable knows, and Apertrust's two, expanded for one photo.

use super::names::NAMES;
use super::*;

/// A Nikon photo of an export's seventh, with every value set.
fn photo() -> Values {
    let time = |year, month, day, hour, minute, second, microsecond| DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        microsecond,
    };
    Values {
        jobcode: "export".to_owned(),
        sequence: 7,
        now: time(2026, 11, 5, 14, 3, 9, 250_417),
        max: Size {
            width: 2048,
            height: 2048,
        },
        upscale: false,
        version: "0.4.0+12~gabcdef0".to_owned(),
        user_name: "ana".to_owned(),
        home: "/home/ana".to_owned(),
        pictures: "/home/ana/Pictures".to_owned(),
        desktop: Some("/home/ana/Desktop".to_owned()),
        path: "/home/ana/Photos/2017/Garden/DSC_0042.NEF".to_owned(),
        maker: "Nikon".to_owned(),
        model: "D5100".to_owned(),
        lens: "AF-S DX Zoom-Nikkor 18-70mm f/3.5-4.5G IF-ED".to_owned(),
        taken: Some(time(2017, 7, 20, 10, 52, 48, 500_000)),
        exposure: 1.0 / 50.0,
        aperture: 8.0,
        focal_length: 18.0,
        focus_distance: 0.4,
        iso: 500.0,
        exposure_bias: Some(-1.0 / 3.0),
        exposure_program: Some(1),
        metering_mode: Some(5),
        white_balance: Some(0),
        flash: Some(16),
        latitude: Some(47.958509),
        longitude: Some(-11.846224),
        elevation: Some(602.35),
        creator: Some("Ansel".to_owned()),
        rights: Some("CC BY".to_owned()),
        description: None,
        sensor: Size {
            width: 4992,
            height: 3280,
        },
        raw: Size {
            width: 4928,
            height: 3264,
        },
        crop: Size {
            width: 4928,
            height: 3264,
        },
    }
}

/// Each name with what it expands to for [`photo`], in [`NAMES`]' order.
const EXPECTED: &[(&str, &str)] = &[
    ("YEAR.SHORT", "26"),
    ("SHORT_YEAR", "26"),
    ("DATE.SHORT_YEAR", "26"),
    ("YEAR", "2026"),
    ("DATE.LONG_YEAR", "2026"),
    ("MONTH.SHORT", "Nov"),
    ("DATE.SHORT_MONTH", "Nov"),
    ("MONTH.LONG", "November"),
    ("DATE.LONG_MONTH", "November"),
    ("MONTH", "11"),
    ("DATE.MONTH", "11"),
    ("DAY", "05"),
    ("DATE.DAY", "05"),
    ("HOUR.AMPM", "02 PM"),
    ("DATE.HOUR_AMPM", "02 PM"),
    ("HOUR", "14"),
    ("DATE.HOUR", "14"),
    ("MINUTE", "03"),
    ("DATE.MINUTE", "03"),
    ("SECOND", "09"),
    ("DATE.SECOND", "09"),
    ("MSEC", "250"),
    ("DATE", "2026:11:05 14:03:09"),
    ("EXIF.DATE.REGIONAL", "07/20/17"),
    ("EXIF.TIME.REGIONAL", "10:52:48"),
    ("EXIF.YEAR.SHORT", "17"),
    ("EXIF.DATE.SHORT_YEAR", "17"),
    ("EXIF.YEAR", "2017"),
    ("EXIF_YEAR", "2017"),
    ("EXIF.DATE.LONG_YEAR", "2017"),
    ("EXIF.MONTH.SHORT", "Jul"),
    ("EXIF.DATE.SHORT_MONTH", "Jul"),
    ("EXIF.MONTH.LONG", "July"),
    ("EXIF.DATE.LONG_MONTH", "July"),
    ("EXIF.MONTH", "07"),
    ("EXIF_MONTH", "07"),
    ("EXIF.DATE.MONTH", "07"),
    ("EXIF.DAY", "20"),
    ("EXIF_DAY", "20"),
    ("EXIF.DATE.DAY", "20"),
    ("EXIF.HOUR.AMPM", "10 AM"),
    ("EXIF.DATE.HOUR_AMPM", "10 AM"),
    ("EXIF.HOUR", "10"),
    ("EXIF_HOUR", "10"),
    ("EXIF.DATE.HOUR", "10"),
    ("EXIF.MINUTE", "52"),
    ("EXIF_MINUTE", "52"),
    ("EXIF.DATE.MINUTE", "52"),
    ("EXIF.SECOND", "48"),
    ("EXIF_SECOND", "48"),
    ("EXIF.DATE.SECOND", "48"),
    // darktable rebuilds the photo's time from whole seconds.
    ("EXIF.MSEC", "000"),
    ("EXIF_MSEC", "000"),
    ("EXIF.DATE", "2017:07:20 10:52:48"),
    ("EXIF.ISO", "500"),
    ("EXIF_ISO", "500"),
    ("NL", ""),
    ("EXIF.EXPOSURE.BIAS", "-0.33"),
    ("EXIF_EXPOSURE_BIAS", "-0.33"),
    ("EXIF.EXPOSURE.PROGRAM", "Manual"),
    ("EXIF.EXPOSURE", "1_50"),
    ("EXIF_EXPOSURE", "1_50"),
    ("EXIF.APERTURE", "8.0"),
    ("EXIF_APERTURE", "8.0"),
    ("EXIF.CROP_FACTOR", ""),
    ("EXIF.FOCAL.LENGTH.EQUIV", ""),
    ("EXIF.FOCAL.LENGTH", "18.0"),
    ("EXIF_FOCAL_LENGTH", "18.0"),
    ("EXIF.FOCUS.DISTANCE", "0.40"),
    ("EXIF_FOCUS_DISTANCE", "0.40"),
    ("EXIF.FLASH.ICON", ""),
    ("EXIF.FLASH", "no"),
    ("EXIF.METERING", "Multi-segment"),
    ("EXIF.WHITEBALANCE", "Auto"),
    ("LONGITUDE", "W011.846224"),
    ("GPS.LONGITUDE", "W011.846224"),
    ("LATITUDE", "N47.958509"),
    ("GPS.LATITUDE", "N47.958509"),
    ("ELEVATION", "602.35"),
    ("GPS.ELEVATION", "602.35"),
    ("GPS.LOCATION.ICON", "📌"),
    ("GPS.LOCATION", "N47.958509, W011.846224, 602.35"),
    ("EXIF.MAKER", "Nikon"),
    ("MAKER", "Nikon"),
    ("EXIF.MODEL", "D5100"),
    ("MODEL", "D5100"),
    ("EXIF.LENS", "AF-S DX Zoom-Nikkor 18-70mm f/3.5-4.5G IF-ED"),
    ("LENS", "AF-S DX Zoom-Nikkor 18-70mm f/3.5-4.5G IF-ED"),
    ("IMAGE.ID.NEXT", ""),
    ("ID", ""),
    ("IMAGE.ID", ""),
    ("IMAGE.EXIF", "1/50 f/8.0 18mm ISO 500"),
    ("VERSION.NAME", ""),
    ("VERSION_NAME", ""),
    ("VERSION.IF_MULTI", ""),
    ("VERSION_IF_MULTI", ""),
    ("VERSION", ""),
    ("JOBCODE", "export"),
    ("ROLL.NAME", "Garden"),
    ("ROLL_NAME", "Garden"),
    ("FILE.DIRECTORY", "/home/ana/Photos/2017/Garden"),
    ("FILE_DIRECTORY", "/home/ana/Photos/2017/Garden"),
    ("FILE.FOLDER", "/home/ana/Photos/2017/Garden"),
    ("FILE_FOLDER", "/home/ana/Photos/2017/Garden"),
    (
        "IMAGE.FILENAME",
        "/home/ana/Photos/2017/Garden/DSC_0042.NEF",
    ),
    ("FILE.NAME", "DSC_0042"),
    ("FILE_NAME", "DSC_0042"),
    ("IMAGE.BASENAME", "DSC_0042"),
    ("FILE.EXTENSION", "NEF"),
    ("FILE_EXTENSION", "NEF"),
    ("SEQUENCE", "0007"),
    ("USERNAME", "ana"),
    ("FOLDER.HOME", "/home/ana"),
    ("HOME_FOLDER", "/home/ana"),
    ("HOME", "/home/ana"),
    ("FOLDER.PICTURES", "/home/ana/Pictures"),
    ("PICTURES_FOLDER", "/home/ana/Pictures"),
    ("FOLDER.DESKTOP", "/home/ana/Desktop"),
    ("DESKTOP_FOLDER", "/home/ana/Desktop"),
    ("DESKTOP", "/home/ana/Desktop"),
    ("STARS", ""),
    ("RATING.ICONS", ""),
    ("RATING_ICONS", ""),
    ("Xmp.xmp.Rating", ""),
    ("LABELS.ICONS", ""),
    ("LABELS_ICONS", ""),
    ("LABELS.COLORICONS", ""),
    ("LABELS_COLORICONS", ""),
    ("LABELS", ""),
    ("OPENCL.ACTIVATED", ""),
    ("OPENCL_ACTIVATED", ""),
    ("WORKSPACE.LABEL", ""),
    ("WIDTH.MAX", "2048"),
    ("MAX_WIDTH", "2048"),
    ("WIDTH.SENSOR", "4992"),
    ("SENSOR_WIDTH", "4992"),
    ("WIDTH.RAW", "4928"),
    ("RAW_WIDTH", "4928"),
    ("WIDTH.CROP", "4928"),
    ("CROP_WIDTH", "4928"),
    ("WIDTH.EXPORT", "2048"),
    ("EXPORT_WIDTH", "2048"),
    ("HEIGHT.MAX", "2048"),
    ("MAX_HEIGHT", "2048"),
    ("HEIGHT.SENSOR", "3280"),
    ("SENSOR_HEIGHT", "3280"),
    ("HEIGHT.RAW", "3264"),
    ("RAW_HEIGHT", "3264"),
    ("HEIGHT.CROP", "3264"),
    ("CROP_HEIGHT", "3264"),
    // 3264 × (2048 / 4928) is 1356.47.
    ("HEIGHT.EXPORT", "1356"),
    ("EXPORT_HEIGHT", "1356"),
    ("CATEGORY", ""),
    ("IMAGE.TAGS.HIERARCHY", ""),
    ("TAGS", ""),
    ("IMAGE.TAGS", ""),
    ("SIDECAR_TXT", ""),
    ("DARKTABLE.VERSION", "0.4.0+12~gabcdef0"),
    ("DARKTABLE_VERSION", "0.4.0+12~gabcdef0"),
    ("DARKTABLE.NAME", "Apertrust"),
    ("DARKTABLE_NAME", "Apertrust"),
    ("Xmp.dc.title", ""),
    ("Xmp.dc.description", ""),
    ("Xmp.dc.creator", "Ansel"),
    ("Xmp.dc.publisher", ""),
    ("Xmp.dc.rights", "CC BY"),
    ("Xmp.acdsee.notes", ""),
    ("Xmp.darktable.version_name", ""),
    ("Xmp.darktable.image_id", ""),
    ("Xmp.xmpMM.PreservedFileName", ""),
    ("TITLE", ""),
    ("DESCRIPTION", ""),
    ("PUBLISHER", ""),
    ("CREATOR", "Ansel"),
    ("RIGHTS", "CC BY"),
    ("APERTRUST.VERSION", "0.4.0+12~gabcdef0"),
    ("APERTRUST.NAME", "Apertrust"),
];

#[test]
fn every_name_expands_as_darktable_writes_it() {
    let names: Vec<&str> = NAMES.iter().map(|(name, _)| *name).collect();
    let expected: Vec<&str> = EXPECTED.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, expected, "every name, in darktable's order");
    let photo = photo();
    for (name, expected) in EXPECTED {
        let pattern = Pattern::parse(&format!("$({name})")).unwrap();
        assert_eq!(pattern.expand(&photo), *expected, "$({name})");
    }
}

#[test]
fn every_name_with_no_value_here_expands_to_nothing_and_says_so() {
    let photo = photo();
    for (name, key) in NAMES {
        let pattern = Pattern::parse(&format!("$({name})")).unwrap();
        let unavailable = pattern.not_here();
        match key.not_here() {
            Some(why) => {
                assert_eq!(pattern.expand(&photo), "", "$({name})");
                assert_eq!(unavailable, [Unavailable { name, why }], "$({name})");
            }
            None => assert!(unavailable.is_empty(), "$({name})"),
        }
    }
}

#[test]
fn a_photo_without_its_data_expands_as_darktables_image_without_them() {
    // darktable's image starts from 0001-01-01 and no exposure time, as a raw with no
    // EXIF leaves it.
    let photo = Values {
        path: "/photos/IMG.CR3".to_owned(),
        ..Values::default()
    };
    for (pattern, expected) in [
        ("$(EXIF.YEAR)-$(EXIF.MONTH)-$(EXIF.DAY)", "1-01-01"),
        ("$(EXIF.DATE)", "1:01:01 00:00:00"),
        ("$(EXIF.DATE.REGIONAL)", "01/01/01"),
        ("$(EXIF.HOUR.AMPM)", "12 AM"),
        ("$(EXIF.EXPOSURE)", "1_inf"),
        (
            "$(EXIF.APERTURE) $(EXIF.FOCAL.LENGTH) $(EXIF.ISO)",
            "0.0 0.0 0",
        ),
        ("$(EXIF.EXPOSURE.BIAS)", ""),
        ("$(EXIF.FLASH)$(EXIF.FLASH.ICON)", "no info"),
        (
            "$(EXIF.EXPOSURE.PROGRAM)$(EXIF.METERING)$(EXIF.WHITEBALANCE)",
            "",
        ),
        (
            "$(GPS.LATITUDE)$(GPS.LONGITUDE)$(GPS.LOCATION)$(GPS.LOCATION.ICON)",
            "",
        ),
        ("$(GPS.ELEVATION)", "nan"),
        ("$(EXIF.FOCUS.DISTANCE)", "0.00"),
        ("$(WIDTH.EXPORT)x$(HEIGHT.EXPORT)", "0x0"),
        ("$(FOLDER.DESKTOP)", ""),
        ("$(IMAGE.EXIF)", "1/inf f/0.0 0mm ISO 0"),
    ] {
        let expanded = Pattern::parse(pattern).unwrap().expand(&photo);
        assert_eq!(expanded, expected, "{pattern}");
    }
}
