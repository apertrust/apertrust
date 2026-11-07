// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2025  darktable developers (ported from src/common/variables.c and src/gui/gtkentry.c)
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

//! Every name darktable's `_get_base_value` knows, in its order, with what each
//! expands to ([`Key`]), Apertrust's two of D7, and darktable's list of them with its
//! descriptions (`_default_path_compl_list` in `src/gui/gtkentry.c`).

use std::fmt;

/// What a variable expands to: one for each of `_get_base_value`'s branches, a name and
/// its legacy aliases sharing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    // The export's time, darktable's `time`: the local time as the export starts.
    YearShort,
    Year,
    MonthShort,
    MonthLong,
    Month,
    Day,
    HourAmPm,
    Hour,
    Minute,
    Second,
    Msec,
    Date,
    // The photo's EXIF time.
    ExifDateRegional,
    ExifTimeRegional,
    ExifYearShort,
    ExifYear,
    ExifMonthShort,
    ExifMonthLong,
    ExifMonth,
    ExifDay,
    ExifHourAmPm,
    ExifHour,
    ExifMinute,
    ExifSecond,
    ExifMsec,
    ExifDate,
    // The photo's shooting data.
    ExifIso,
    NewLine,
    ExposureBias,
    ExposureProgram,
    Exposure,
    Aperture,
    CropFactor,
    FocalLengthEquivalent,
    FocalLength,
    FocusDistance,
    FlashIcon,
    Flash,
    Metering,
    WhiteBalance,
    Longitude,
    Latitude,
    Elevation,
    LocationIcon,
    Location,
    Maker,
    Model,
    Lens,
    // The library's.
    ImageIdNext,
    ImageId,
    ImageExif,
    VersionName,
    VersionIfMulti,
    Version,
    Jobcode,
    // The file's.
    RollName,
    FileFolder,
    ImageFilename,
    FileName,
    FileExtension,
    Sequence,
    // The person's.
    Username,
    Home,
    Pictures,
    Desktop,
    // The library's.
    Stars,
    RatingIcons,
    LabelsIcons,
    Labels,
    // darktable's own.
    OpenCl,
    WorkspaceLabel,
    // The sizes.
    MaxWidth,
    SensorWidth,
    RawWidth,
    CropWidth,
    ExportWidth,
    MaxHeight,
    SensorHeight,
    RawHeight,
    CropHeight,
    ExportHeight,
    // The library's.
    Category,
    TagsHierarchy,
    Tags,
    SidecarTxt,
    // The program's.
    ProgramVersion,
    ProgramName,
    // darktable's default metadata fields (`data.meta_data`).
    Title,
    Description,
    Creator,
    Publisher,
    Rights,
    Notes,
    ImageIdMetadata,
    PreservedFileName,
}

/// The parameters a variable takes after its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Takes {
    Nothing,
    /// `SEQUENCE`'s digits as one digit, `$(SEQUENCE4)`, or `[n,m]`, digits and start.
    Sequence,
    /// `[n]`, `_get_var_parameter`'s: ROLL.NAME's levels, an image id's digits.
    Number,
    /// `CATEGORY`'s level as one digit and its category in parentheses,
    /// `$(CATEGORY0(places))`, or `[n,category]`.
    Category,
}

impl Key {
    pub(crate) fn takes(self) -> Takes {
        match self {
            Key::Sequence => Takes::Sequence,
            Key::RollName | Key::ImageId | Key::ImageIdNext => Takes::Number,
            Key::Category => Takes::Category,
            _ => Takes::Nothing,
        }
    }

    /// Why Apertrust has no value for it, D7: `None` where it has one.
    pub(crate) fn not_here(self) -> Option<NotHere> {
        Some(match self {
            Key::NewLine | Key::LabelsIcons | Key::SidecarTxt => NotHere::ImageInformation,
            Key::ImageIdNext | Key::ImageId | Key::ImageIdMetadata => NotHere::Library,
            Key::VersionName | Key::VersionIfMulti | Key::Version => NotHere::Duplicates,
            Key::Stars | Key::RatingIcons => NotHere::Ratings,
            Key::Labels => NotHere::Labels,
            Key::Category | Key::TagsHierarchy | Key::Tags => NotHere::Tags,
            Key::Title | Key::Publisher | Key::Notes | Key::PreservedFileName => NotHere::Metadata,
            Key::OpenCl => NotHere::OpenCl,
            Key::WorkspaceLabel => NotHere::Workspaces,
            Key::CropFactor | Key::FocalLengthEquivalent => NotHere::CropFactor,
            _ => return None,
        })
    }
}

/// Why a variable darktable has expands to nothing in Apertrust (D7): the export goes on,
/// and the dialog says so in a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotHere {
    /// darktable expands it only in its image information and overlays (job code
    /// "infos"), and to nothing in an export: `NL`, `LABELS.ICONS`, `SIDECAR_TXT`.
    ImageInformation,
    /// A library's image ids, which Apertrust has not yet.
    Library,
    /// Duplicates and their version names, which Apertrust has not yet.
    Duplicates,
    /// Star ratings, which Apertrust has not yet.
    Ratings,
    /// Colour labels, which Apertrust has not yet.
    Labels,
    /// Tags, which Apertrust has not yet.
    Tags,
    /// A metadata field the raw's EXIF does not seed, while Apertrust has no metadata.
    Metadata,
    /// Whether OpenCL runs: Apertrust has no OpenCL.
    OpenCl,
    /// darktable's workspaces, which Apertrust has not.
    Workspaces,
    /// The crop factor, from FocalLengthIn35mmFilm or the focal plane's resolution,
    /// which the raw decoder does not read.
    CropFactor,
}

impl fmt::Display for NotHere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NotHere::ImageInformation => "darktable shows it only in its image information",
            NotHere::Library => "Apertrust has no image ids yet",
            NotHere::Duplicates => "Apertrust has no duplicates yet",
            NotHere::Ratings => "Apertrust has no ratings yet",
            NotHere::Labels => "Apertrust has no colour labels yet",
            NotHere::Tags => "Apertrust has no tags yet",
            NotHere::Metadata => "Apertrust has no metadata yet",
            NotHere::OpenCl => "Apertrust does not use OpenCL",
            NotHere::Workspaces => "Apertrust has no workspaces",
            NotHere::CropFactor => "Apertrust cannot read the crop factor from raws yet",
        })
    }
}

/// Every name, in `_get_base_value`'s order: legacy aliases beside their names, darktable's
/// image information's own (`NL`, `LABELS.ICONS`, `SIDECAR_TXT`), the default metadata
/// fields after them, then darktable's legacy aliases of those (`_legacy_aliases`) and
/// Apertrust's two (D7). darktable takes the first that the text starts with; no name
/// here starts with one before it, so that is the longest.
pub(crate) const NAMES: &[(&str, Key)] = &[
    ("YEAR.SHORT", Key::YearShort),
    ("SHORT_YEAR", Key::YearShort),
    ("DATE.SHORT_YEAR", Key::YearShort),
    ("YEAR", Key::Year),
    ("DATE.LONG_YEAR", Key::Year),
    ("MONTH.SHORT", Key::MonthShort),
    ("DATE.SHORT_MONTH", Key::MonthShort),
    ("MONTH.LONG", Key::MonthLong),
    ("DATE.LONG_MONTH", Key::MonthLong),
    ("MONTH", Key::Month),
    ("DATE.MONTH", Key::Month),
    ("DAY", Key::Day),
    ("DATE.DAY", Key::Day),
    ("HOUR.AMPM", Key::HourAmPm),
    ("DATE.HOUR_AMPM", Key::HourAmPm),
    ("HOUR", Key::Hour),
    ("DATE.HOUR", Key::Hour),
    ("MINUTE", Key::Minute),
    ("DATE.MINUTE", Key::Minute),
    ("SECOND", Key::Second),
    ("DATE.SECOND", Key::Second),
    ("MSEC", Key::Msec),
    ("DATE", Key::Date),
    ("EXIF.DATE.REGIONAL", Key::ExifDateRegional),
    ("EXIF.TIME.REGIONAL", Key::ExifTimeRegional),
    ("EXIF.YEAR.SHORT", Key::ExifYearShort),
    ("EXIF.DATE.SHORT_YEAR", Key::ExifYearShort),
    ("EXIF.YEAR", Key::ExifYear),
    ("EXIF_YEAR", Key::ExifYear),
    ("EXIF.DATE.LONG_YEAR", Key::ExifYear),
    ("EXIF.MONTH.SHORT", Key::ExifMonthShort),
    ("EXIF.DATE.SHORT_MONTH", Key::ExifMonthShort),
    ("EXIF.MONTH.LONG", Key::ExifMonthLong),
    ("EXIF.DATE.LONG_MONTH", Key::ExifMonthLong),
    ("EXIF.MONTH", Key::ExifMonth),
    ("EXIF_MONTH", Key::ExifMonth),
    ("EXIF.DATE.MONTH", Key::ExifMonth),
    ("EXIF.DAY", Key::ExifDay),
    ("EXIF_DAY", Key::ExifDay),
    ("EXIF.DATE.DAY", Key::ExifDay),
    ("EXIF.HOUR.AMPM", Key::ExifHourAmPm),
    ("EXIF.DATE.HOUR_AMPM", Key::ExifHourAmPm),
    ("EXIF.HOUR", Key::ExifHour),
    ("EXIF_HOUR", Key::ExifHour),
    ("EXIF.DATE.HOUR", Key::ExifHour),
    ("EXIF.MINUTE", Key::ExifMinute),
    ("EXIF_MINUTE", Key::ExifMinute),
    ("EXIF.DATE.MINUTE", Key::ExifMinute),
    ("EXIF.SECOND", Key::ExifSecond),
    ("EXIF_SECOND", Key::ExifSecond),
    ("EXIF.DATE.SECOND", Key::ExifSecond),
    ("EXIF.MSEC", Key::ExifMsec),
    ("EXIF_MSEC", Key::ExifMsec),
    ("EXIF.DATE", Key::ExifDate),
    ("EXIF.ISO", Key::ExifIso),
    ("EXIF_ISO", Key::ExifIso),
    ("NL", Key::NewLine),
    ("EXIF.EXPOSURE.BIAS", Key::ExposureBias),
    ("EXIF_EXPOSURE_BIAS", Key::ExposureBias),
    ("EXIF.EXPOSURE.PROGRAM", Key::ExposureProgram),
    ("EXIF.EXPOSURE", Key::Exposure),
    ("EXIF_EXPOSURE", Key::Exposure),
    ("EXIF.APERTURE", Key::Aperture),
    ("EXIF_APERTURE", Key::Aperture),
    ("EXIF.CROP_FACTOR", Key::CropFactor),
    ("EXIF.FOCAL.LENGTH.EQUIV", Key::FocalLengthEquivalent),
    ("EXIF.FOCAL.LENGTH", Key::FocalLength),
    ("EXIF_FOCAL_LENGTH", Key::FocalLength),
    ("EXIF.FOCUS.DISTANCE", Key::FocusDistance),
    ("EXIF_FOCUS_DISTANCE", Key::FocusDistance),
    ("EXIF.FLASH.ICON", Key::FlashIcon),
    ("EXIF.FLASH", Key::Flash),
    ("EXIF.METERING", Key::Metering),
    ("EXIF.WHITEBALANCE", Key::WhiteBalance),
    ("LONGITUDE", Key::Longitude),
    ("GPS.LONGITUDE", Key::Longitude),
    ("LATITUDE", Key::Latitude),
    ("GPS.LATITUDE", Key::Latitude),
    ("ELEVATION", Key::Elevation),
    ("GPS.ELEVATION", Key::Elevation),
    ("GPS.LOCATION.ICON", Key::LocationIcon),
    ("GPS.LOCATION", Key::Location),
    ("EXIF.MAKER", Key::Maker),
    ("MAKER", Key::Maker),
    ("EXIF.MODEL", Key::Model),
    ("MODEL", Key::Model),
    ("EXIF.LENS", Key::Lens),
    ("LENS", Key::Lens),
    ("IMAGE.ID.NEXT", Key::ImageIdNext),
    ("ID", Key::ImageId),
    ("IMAGE.ID", Key::ImageId),
    ("IMAGE.EXIF", Key::ImageExif),
    ("VERSION.NAME", Key::VersionName),
    ("VERSION_NAME", Key::VersionName),
    ("VERSION.IF_MULTI", Key::VersionIfMulti),
    ("VERSION_IF_MULTI", Key::VersionIfMulti),
    ("VERSION", Key::Version),
    ("JOBCODE", Key::Jobcode),
    ("ROLL.NAME", Key::RollName),
    ("ROLL_NAME", Key::RollName),
    ("FILE.DIRECTORY", Key::FileFolder),
    ("FILE_DIRECTORY", Key::FileFolder),
    ("FILE.FOLDER", Key::FileFolder),
    ("FILE_FOLDER", Key::FileFolder),
    ("IMAGE.FILENAME", Key::ImageFilename),
    ("FILE.NAME", Key::FileName),
    ("FILE_NAME", Key::FileName),
    ("IMAGE.BASENAME", Key::FileName),
    ("FILE.EXTENSION", Key::FileExtension),
    ("FILE_EXTENSION", Key::FileExtension),
    ("SEQUENCE", Key::Sequence),
    ("USERNAME", Key::Username),
    ("FOLDER.HOME", Key::Home),
    ("HOME_FOLDER", Key::Home),
    ("HOME", Key::Home),
    ("FOLDER.PICTURES", Key::Pictures),
    ("PICTURES_FOLDER", Key::Pictures),
    ("FOLDER.DESKTOP", Key::Desktop),
    ("DESKTOP_FOLDER", Key::Desktop),
    ("DESKTOP", Key::Desktop),
    ("STARS", Key::Stars),
    ("RATING.ICONS", Key::RatingIcons),
    ("RATING_ICONS", Key::RatingIcons),
    ("Xmp.xmp.Rating", Key::RatingIcons),
    ("LABELS.ICONS", Key::LabelsIcons),
    ("LABELS_ICONS", Key::LabelsIcons),
    ("LABELS.COLORICONS", Key::LabelsIcons),
    ("LABELS_COLORICONS", Key::LabelsIcons),
    ("LABELS", Key::Labels),
    ("OPENCL.ACTIVATED", Key::OpenCl),
    ("OPENCL_ACTIVATED", Key::OpenCl),
    ("WORKSPACE.LABEL", Key::WorkspaceLabel),
    ("WIDTH.MAX", Key::MaxWidth),
    ("MAX_WIDTH", Key::MaxWidth),
    ("WIDTH.SENSOR", Key::SensorWidth),
    ("SENSOR_WIDTH", Key::SensorWidth),
    ("WIDTH.RAW", Key::RawWidth),
    ("RAW_WIDTH", Key::RawWidth),
    ("WIDTH.CROP", Key::CropWidth),
    ("CROP_WIDTH", Key::CropWidth),
    ("WIDTH.EXPORT", Key::ExportWidth),
    ("EXPORT_WIDTH", Key::ExportWidth),
    ("HEIGHT.MAX", Key::MaxHeight),
    ("MAX_HEIGHT", Key::MaxHeight),
    ("HEIGHT.SENSOR", Key::SensorHeight),
    ("SENSOR_HEIGHT", Key::SensorHeight),
    ("HEIGHT.RAW", Key::RawHeight),
    ("RAW_HEIGHT", Key::RawHeight),
    ("HEIGHT.CROP", Key::CropHeight),
    ("CROP_HEIGHT", Key::CropHeight),
    ("HEIGHT.EXPORT", Key::ExportHeight),
    ("EXPORT_HEIGHT", Key::ExportHeight),
    ("CATEGORY", Key::Category),
    ("IMAGE.TAGS.HIERARCHY", Key::TagsHierarchy),
    ("TAGS", Key::Tags),
    ("IMAGE.TAGS", Key::Tags),
    ("SIDECAR_TXT", Key::SidecarTxt),
    ("DARKTABLE.VERSION", Key::ProgramVersion),
    ("DARKTABLE_VERSION", Key::ProgramVersion),
    ("DARKTABLE.NAME", Key::ProgramName),
    ("DARKTABLE_NAME", Key::ProgramName),
    // darktable's default metadata fields, in their display order (`dt_metadata_init`).
    ("Xmp.dc.title", Key::Title),
    ("Xmp.dc.description", Key::Description),
    ("Xmp.dc.creator", Key::Creator),
    ("Xmp.dc.publisher", Key::Publisher),
    ("Xmp.dc.rights", Key::Rights),
    ("Xmp.acdsee.notes", Key::Notes),
    ("Xmp.darktable.version_name", Key::VersionName),
    ("Xmp.darktable.image_id", Key::ImageIdMetadata),
    ("Xmp.xmpMM.PreservedFileName", Key::PreservedFileName),
    // `_legacy_aliases`: darktable replaces `$(TITLE)` and the others whole before it
    // expands; here they are names of their own.
    ("TITLE", Key::Title),
    ("DESCRIPTION", Key::Description),
    ("PUBLISHER", Key::Publisher),
    ("CREATOR", Key::Creator),
    ("RIGHTS", Key::Rights),
    // D7.
    ("APERTRUST.VERSION", Key::ProgramVersion),
    ("APERTRUST.NAME", Key::ProgramName),
];

/// The name `text` starts with, the longest, with what it expands to.
pub(crate) fn name_at(text: &str) -> Option<(&'static str, Key)> {
    NAMES
        .iter()
        .filter(|(name, _)| text.starts_with(name))
        .max_by_key(|(name, _)| name.len())
        .copied()
}

/// The name nearest to `typed`, an unknown name, for a "did you mean" (D8): the list's
/// names first, then their aliases, ignoring case, within an edit distance that grows
/// with the name's length, 1 up to 4 characters, 2 up to 8 and 3 beyond.
pub(crate) fn nearest(typed: &str) -> Option<&'static str> {
    if typed.is_empty() {
        return None;
    }
    let length = typed.chars().count();
    let reach = length.div_ceil(4).clamp(1, 3);
    // A name whose length differs by more than the reach is beyond it, so a long typed
    // name costs nothing.
    let listed = LIST.iter().map(|listed| listed.name());
    let aliases = NAMES.iter().map(|(name, _)| *name);
    let mut candidates = listed
        .chain(aliases)
        .filter(|name| name.chars().count().abs_diff(length) <= reach)
        .peekable();
    candidates.peek()?;
    let typed: Vec<char> = typed.to_lowercase().chars().collect();
    let mut best: Option<(usize, &'static str)> = None;
    for name in candidates {
        let distance = distance(&typed, &name.to_lowercase().chars().collect::<Vec<_>>());
        if distance <= reach && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, name));
        }
    }
    best.map(|(_, name)| name)
}

/// Levenshtein's edit distance between `a` and `b`.
fn distance(a: &[char], b: &[char]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = diagonal + usize::from(ca != cb);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(diagonal + 1);
        }
    }
    row[b.len()]
}

/// A variable of darktable's list, which its path field offers after "$(", with its
/// description (`_default_path_compl_list`), for the Export dialog's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listed {
    insert: &'static str,
    description: &'static str,
    key: Key,
}

impl Listed {
    /// What the list inserts after "$(", before its ")": darktable's `varname`, the name
    /// with its parameters' brackets, such as `SEQUENCE[4,1]` or `ROLL.NAME[]`.
    pub fn insert(&self) -> &'static str {
        self.insert
    }

    /// The name alone, such as `SEQUENCE`.
    pub fn name(&self) -> &'static str {
        self.insert
            .split_once('[')
            .map_or(self.insert, |(name, _)| name)
    }

    /// darktable's description as its list shows it, in English, such as
    /// `$(SEQUENCE[n,m]) - sequence number, n: number of digits, m: start number`.
    pub fn description(&self) -> &'static str {
        self.description
    }

    /// Why it expands to nothing in Apertrust (D7), `None` where it has a value.
    pub fn not_here(&self) -> Option<NotHere> {
        self.key.not_here()
    }
}

/// darktable's list, in its order (`_default_path_compl_list`), then the default metadata
/// fields it adds, those not internal, in their display order (`_init_completion_model`),
/// for the Export dialog's list of variables. `APERTRUST.NAME` and `APERTRUST.VERSION`
/// stand where darktable lists `DARKTABLE.NAME` and `DARKTABLE.VERSION`, which still read
/// as Apertrust's (D7).
pub fn list() -> &'static [Listed] {
    LIST
}

const fn listed(insert: &'static str, description: &'static str, key: Key) -> Listed {
    Listed {
        insert,
        description,
        key,
    }
}

const LIST: &[Listed] = &[
    listed(
        "ROLL.NAME[]",
        "$(ROLL.NAME[n]) - roll of the input image, n: levels used",
        Key::RollName,
    ),
    listed(
        "FILE.FOLDER",
        "$(FILE.FOLDER) - folder containing the input image",
        Key::FileFolder,
    ),
    listed(
        "FILE.NAME",
        "$(FILE.NAME) - basename of the input image",
        Key::FileName,
    ),
    listed(
        "FILE.EXTENSION",
        "$(FILE.EXTENSION) - extension of the input image",
        Key::FileExtension,
    ),
    listed("VERSION", "$(VERSION) - duplicate version", Key::Version),
    listed(
        "VERSION.IF_MULTI",
        "$(VERSION.IF_MULTI) - same as $(VERSION) but null string if only one version exists",
        Key::VersionIfMulti,
    ),
    listed(
        "VERSION.NAME",
        "$(VERSION.NAME) - version name from metadata",
        Key::VersionName,
    ),
    listed("JOBCODE", "$(JOBCODE) - job code for import", Key::Jobcode),
    listed(
        "SEQUENCE[4,1]",
        "$(SEQUENCE[n,m]) - sequence number, n: number of digits, m: start number",
        Key::Sequence,
    ),
    listed(
        "WIDTH.MAX",
        "$(WIDTH.MAX) - maximum image export width",
        Key::MaxWidth,
    ),
    listed(
        "WIDTH.SENSOR",
        "$(WIDTH.SENSOR) - image sensor width",
        Key::SensorWidth,
    ),
    listed("WIDTH.RAW", "$(WIDTH.RAW) - RAW image width", Key::RawWidth),
    listed(
        "WIDTH.CROP",
        "$(WIDTH.CROP) - image width after crop",
        Key::CropWidth,
    ),
    listed(
        "WIDTH.EXPORT",
        "$(WIDTH.EXPORT) - exported image width",
        Key::ExportWidth,
    ),
    listed(
        "HEIGHT.MAX",
        "$(HEIGHT.MAX) - maximum image export height",
        Key::MaxHeight,
    ),
    listed(
        "HEIGHT.SENSOR",
        "$(HEIGHT.SENSOR) - image sensor height",
        Key::SensorHeight,
    ),
    listed(
        "HEIGHT.RAW",
        "$(HEIGHT.RAW) - RAW image height",
        Key::RawHeight,
    ),
    listed(
        "HEIGHT.CROP",
        "$(HEIGHT.CROP) - image height after crop",
        Key::CropHeight,
    ),
    listed(
        "HEIGHT.EXPORT",
        "$(HEIGHT.EXPORT) - exported image height",
        Key::ExportHeight,
    ),
    listed("YEAR", "$(YEAR) - year", Key::Year),
    listed(
        "YEAR.SHORT",
        "$(YEAR.SHORT) - year without century",
        Key::YearShort,
    ),
    listed("MONTH", "$(MONTH) - month", Key::Month),
    listed(
        "MONTH.SHORT",
        "$(MONTH.SHORT) - abbreviated month name according to the current locale",
        Key::MonthShort,
    ),
    listed(
        "MONTH.LONG",
        "$(MONTH.LONG) - full month name according to the current locale",
        Key::MonthLong,
    ),
    listed("DAY", "$(DAY) - day", Key::Day),
    listed("HOUR", "$(HOUR) - hour", Key::Hour),
    listed(
        "HOUR.AMPM",
        "$(HOUR.AMPM) - hour, 12-hour clock",
        Key::HourAmPm,
    ),
    listed("MINUTE", "$(MINUTE) - minute", Key::Minute),
    listed("SECOND", "$(SECOND) - second", Key::Second),
    listed("MSEC", "$(MSEC) - millisecond", Key::Msec),
    listed(
        "EXIF.DATE.REGIONAL",
        "$(EXIF.DATE.REGIONAL) - localized EXIF date",
        Key::ExifDateRegional,
    ),
    listed(
        "EXIF.TIME.REGIONAL",
        "$(EXIF.TIME.REGIONAL) - localized EXIF time",
        Key::ExifTimeRegional,
    ),
    listed("EXIF.YEAR", "$(EXIF.YEAR) - EXIF year", Key::ExifYear),
    listed(
        "EXIF.YEAR.SHORT",
        "$(EXIF.YEAR.SHORT) - EXIF year without century",
        Key::ExifYearShort,
    ),
    listed("EXIF.MONTH", "$(EXIF.MONTH) - EXIF month", Key::ExifMonth),
    listed(
        "EXIF.MONTH.SHORT",
        "$(EXIF.MONTH.SHORT) - abbreviated EXIF month name according to the current locale",
        Key::ExifMonthShort,
    ),
    listed(
        "EXIF.MONTH.LONG",
        "$(EXIF.MONTH.LONG) - full EXIF month name according to the current locale",
        Key::ExifMonthLong,
    ),
    listed("EXIF.DAY", "$(EXIF.DAY) - EXIF day", Key::ExifDay),
    listed("EXIF.HOUR", "$(EXIF.HOUR) - EXIF hour", Key::ExifHour),
    listed(
        "EXIF.HOUR.AMPM",
        "$(EXIF.HOUR.AMPM) - EXIF hour, 12-hour clock",
        Key::ExifHourAmPm,
    ),
    listed(
        "EXIF.MINUTE",
        "$(EXIF.MINUTE) - EXIF minute",
        Key::ExifMinute,
    ),
    listed(
        "EXIF.SECOND",
        "$(EXIF.SECOND) - EXIF second",
        Key::ExifSecond,
    ),
    listed(
        "EXIF.MSEC",
        "$(EXIF.MSEC) - EXIF millisecond",
        Key::ExifMsec,
    ),
    listed("EXIF.ISO", "$(EXIF.ISO) - ISO value", Key::ExifIso),
    listed(
        "EXIF.EXPOSURE.BIAS",
        "$(EXIF.EXPOSURE.BIAS) - EXIF exposure bias",
        Key::ExposureBias,
    ),
    listed(
        "EXIF.EXPOSURE.PROGRAM",
        "$(EXIF.EXPOSURE.PROGRAM) - EXIF exposure program",
        Key::ExposureProgram,
    ),
    listed(
        "EXIF.EXPOSURE",
        "$(EXIF.EXPOSURE) - EXIF exposure",
        Key::Exposure,
    ),
    listed(
        "EXIF.APERTURE",
        "$(EXIF.APERTURE) - EXIF aperture",
        Key::Aperture,
    ),
    listed(
        "EXIF.CROP_FACTOR",
        "$(EXIF.CROP_FACTOR) - EXIF crop factor",
        Key::CropFactor,
    ),
    listed(
        "EXIF.FOCAL.LENGTH",
        "$(EXIF.FOCAL.LENGTH) - EXIF focal length",
        Key::FocalLength,
    ),
    listed(
        "EXIF.FOCAL.LENGTH.EQUIV",
        "$(EXIF.FOCAL.LENGTH.EQUIV) - EXIF 35 mm equivalent focal length",
        Key::FocalLengthEquivalent,
    ),
    listed(
        "EXIF.FOCUS.DISTANCE",
        "$(EXIF.FOCUS.DISTANCE) - EXIF focal distance",
        Key::FocusDistance,
    ),
    listed("EXIF.MAKER", "$(EXIF.MAKER) - camera maker", Key::Maker),
    listed("EXIF.MODEL", "$(EXIF.MODEL) - camera model", Key::Model),
    listed(
        "EXIF.WHITEBALANCE",
        "$(EXIF.WHITEBALANCE) - EXIF selected white balance",
        Key::WhiteBalance,
    ),
    listed(
        "EXIF.METERING",
        "$(EXIF.METERING) - EXIF exposure metering mode",
        Key::Metering,
    ),
    listed("EXIF.LENS", "$(EXIF.LENS) - lens", Key::Lens),
    listed(
        "EXIF.FLASH.ICON",
        "$(EXIF.FLASH.ICON) - icon indicating whether flash was used",
        Key::FlashIcon,
    ),
    listed(
        "EXIF.FLASH",
        "$(EXIF.FLASH) - was flash used (yes/no/--)",
        Key::Flash,
    ),
    listed(
        "GPS.LONGITUDE",
        "$(GPS.LONGITUDE) - longitude",
        Key::Longitude,
    ),
    listed("GPS.LATITUDE", "$(GPS.LATITUDE) - latitude", Key::Latitude),
    listed(
        "GPS.ELEVATION",
        "$(GPS.ELEVATION) - elevation",
        Key::Elevation,
    ),
    listed(
        "GPS.LOCATION.ICON",
        "$(GPS.LOCATION.ICON) - icon indicating whether GPS location is known",
        Key::LocationIcon,
    ),
    listed("LONGITUDE", "$(LONGITUDE) - longitude", Key::Longitude),
    listed("LATITUDE", "$(LATITUDE) - latitude", Key::Latitude),
    listed("ELEVATION", "$(ELEVATION) - elevation", Key::Elevation),
    listed(
        "STARS",
        "$(STARS) - star rating as number (-1 for rejected)",
        Key::Stars,
    ),
    listed(
        "RATING.ICONS",
        "$(RATING.ICONS) - star/reject rating in icon form",
        Key::RatingIcons,
    ),
    listed("LABELS", "$(LABELS) - color labels as text", Key::Labels),
    listed(
        "LABELS.ICONS",
        "$(LABELS.ICONS) - color labels as icons",
        Key::LabelsIcons,
    ),
    listed(
        "IMAGE.TAGS.HIERARCHY",
        "$(IMAGE.TAGS.HIERARCHY) - tags as set in metadata settings, preserving hierarchy",
        Key::TagsHierarchy,
    ),
    listed(
        "IMAGE.TAGS",
        "$(IMAGE.TAGS) - tags as set in metadata settings, flattened",
        Key::Tags,
    ),
    listed("IMAGE.ID", "$(IMAGE.ID) - image ID", Key::ImageId),
    listed(
        "IMAGE.ID.NEXT",
        "$(IMAGE.ID.NEXT) - next image ID to be assigned on import",
        Key::ImageIdNext,
    ),
    listed("ID", "$(ID) - image ID", Key::ImageId),
    listed("USERNAME", "$(USERNAME) - login name", Key::Username),
    listed(
        "FOLDER.PICTURES",
        "$(FOLDER.PICTURES) - pictures folder",
        Key::Pictures,
    ),
    listed("FOLDER.HOME", "$(FOLDER.HOME) - home folder", Key::Home),
    listed(
        "FOLDER.DESKTOP",
        "$(FOLDER.DESKTOP) - desktop folder",
        Key::Desktop,
    ),
    listed(
        "OPENCL.ACTIVATED",
        "$(OPENCL.ACTIVATED) - whether OpenCL is activated",
        Key::OpenCl,
    ),
    listed(
        "CATEGORY[,]",
        "$(CATEGORY[n,category]) - subtag of level n in hierarchical tags",
        Key::Category,
    ),
    listed(
        "TAGS",
        "$(TAGS) - tags as set in metadata settings",
        Key::Tags,
    ),
    listed(
        "APERTRUST.NAME",
        "$(APERTRUST.NAME) - Apertrust name",
        Key::ProgramName,
    ),
    listed(
        "APERTRUST.VERSION",
        "$(APERTRUST.VERSION) - current Apertrust version",
        Key::ProgramVersion,
    ),
    listed(
        "SIDECAR_TXT",
        "$(SIDECAR_TXT) - contents of .txt sidecar file, if present",
        Key::SidecarTxt,
    ),
    listed(
        "WORKSPACE.LABEL",
        "$(WORKSPACE.LABEL) - label of the current workspace",
        Key::WorkspaceLabel,
    ),
    // `dt_gtkentry_variables_add_metadata`: "$(%s) - from metadata".
    listed(
        "Xmp.dc.title",
        "$(Xmp.dc.title) - from metadata",
        Key::Title,
    ),
    listed(
        "Xmp.dc.description",
        "$(Xmp.dc.description) - from metadata",
        Key::Description,
    ),
    listed(
        "Xmp.dc.creator",
        "$(Xmp.dc.creator) - from metadata",
        Key::Creator,
    ),
    listed(
        "Xmp.dc.publisher",
        "$(Xmp.dc.publisher) - from metadata",
        Key::Publisher,
    ),
    listed(
        "Xmp.dc.rights",
        "$(Xmp.dc.rights) - from metadata",
        Key::Rights,
    ),
    listed(
        "Xmp.acdsee.notes",
        "$(Xmp.acdsee.notes) - from metadata",
        Key::Notes,
    ),
    listed(
        "Xmp.darktable.version_name",
        "$(Xmp.darktable.version_name) - from metadata",
        Key::VersionName,
    ),
    listed(
        "Xmp.xmpMM.PreservedFileName",
        "$(Xmp.xmpMM.PreservedFileName) - from metadata",
        Key::PreservedFileName,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_name_starts_with_one_before_it() {
        // darktable takes the first name the text starts with, the longest takes the same.
        for (i, (later, _)) in NAMES.iter().enumerate() {
            for (earlier, _) in &NAMES[..i] {
                assert!(
                    !later.starts_with(earlier),
                    "{later} starts with {earlier}, before it"
                );
            }
        }
    }

    #[test]
    fn every_name_is_unique_and_of_name_characters() {
        for (i, (name, _)) in NAMES.iter().enumerate() {
            assert!(
                name.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_'),
                "{name}"
            );
            assert!(!NAMES[..i].iter().any(|(n, _)| n == name), "{name} twice");
        }
    }

    #[test]
    fn every_listed_name_is_a_name_of_its_key() {
        for listed in LIST {
            assert_eq!(name_at(listed.name()), Some((listed.name(), listed.key)));
        }
    }

    #[test]
    fn lists_darktables_variables_with_d7s_two_in_place_of_its_own() {
        assert_eq!(LIST.len(), 86 + 8);
        assert_eq!(LIST[0].insert(), "ROLL.NAME[]");
        assert!(LIST.iter().all(|l| !l.name().starts_with("DARKTABLE")));
        assert!(LIST.iter().any(|l| l.name() == "APERTRUST.NAME"));
        // The internal image id is not listed, as darktable does not list it.
        assert!(LIST.iter().all(|l| l.key != Key::ImageIdMetadata));
        for listed in LIST {
            let shown = listed.description().split(" - ").next().unwrap();
            assert!(
                shown.starts_with(&format!("$({}", listed.name())),
                "{shown}"
            );
        }
    }

    #[test]
    fn suggests_the_nearest_name() {
        assert_eq!(nearest("FILE_NAM"), Some("FILE_NAME"));
        assert_eq!(nearest("file.name"), Some("FILE.NAME"));
        assert_eq!(nearest("SEQUENSE"), Some("SEQUENCE"));
        assert_eq!(nearest("EXIF.YAER"), Some("EXIF.YEAR"));
        assert_eq!(nearest("NONEXISTANT"), None);
        assert_eq!(nearest(""), None);
    }
}
