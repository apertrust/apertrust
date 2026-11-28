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

//! The Metadata tool's declaration (decision 110; `docs/gui.md`, The Metadata tool):
//! darktable's image information and metadata editor in one utility, its groups after the
//! standards they follow, each a collapsible group whose title's tooltip names its
//! standard ([`UtilityDefinition::tips`]).
//!
//! - **Basic, Camera, Exposure, Lens and GPS** hold read-only facts ([`Row::Fact`]), each
//!   shown or hidden at first as darktable's image information and the design have them;
//!   Basic's Rating and Colour labels and GPS's Show on Map are the tool's own widgets;
//!   Basic's Copy, the open copy's name, is a field, and Preserved file name a fact.
//! - **Content, Creator, Rights, Location and Status** hold IPTC Core's fields, Dublin
//!   Core's publisher and Apertrust's notes ([`Row::Field`]), each editing a field by its
//!   property, as `apertrust_library::fields::FIELDS` gives it. Status shows no row at
//!   first, so it is not drawn until ＋ shows one. Maker notes is not declared until a row
//!   of its is.
//! - **＋** shows or hides any row ([`UtilityDefinition::rows_shown`]); which rows show and
//!   which groups are open are the app's settings, never an edit's.

use std::sync::LazyLock;

use crate::{
    Cut, Fact, FactEnd, FieldRow, Group, GroupKind, Reset, Row, Tab, ToolDefinition, Widget,
};

use super::UtilityDefinition;

/// A fact `id`, shown at first or not, cut at its end.
const fn fact(id: &'static str, shown: bool) -> Row {
    Row::Fact(Fact {
        id,
        shown,
        cut: Cut::End,
        end: None,
    })
}

/// A fact `id`, shown at first or not, cut in its middle.
const fn middle(id: &'static str, shown: bool) -> Row {
    Row::Fact(Fact {
        id,
        shown,
        cut: Cut::Middle,
        end: None,
    })
}

/// A collapsible group `id` holding `rows`.
const fn group(id: &'static str, rows: &'static [Row]) -> Row {
    Row::Group(Group {
        id,
        kind: GroupKind::Collapsible,
        rows,
    })
}

/// A row of the tool's own widget `widget`.
const fn own(widget: Widget) -> Row {
    Row::Custom {
        widget,
        controls: &[],
    }
}

/// Basic: what the file and the library know.
const BASIC: &[Row] = &[
    middle("file_name", true),
    Row::Fact(Fact {
        id: "folder",
        shown: true,
        cut: Cut::Middle,
        end: Some(FactEnd::ShowInFolder),
    }),
    fact("type", true),
    fact("size_on_disk", true),
    fact("dimensions", true),
    own(Widget::MetadataRating),
    own(Widget::MetadataLabels),
    fact("edited", true),
    // The open copy's name, drawn only where the photo has several copies.
    field("copy", "apertrust:name", true),
    middle("preserved_file_name", false),
];

/// Camera: EXIF's IFD0, and two of the Exif IFD's.
const CAMERA: &[Row] = &[
    fact("make", true),
    fact("model", true),
    fact("serial_number", false),
    fact("owner_name", false),
    fact("software", false),
    fact("date_time", false),
    fact("orientation", false),
    fact("resolution", false),
    fact("artist", false),
    fact("copyright", false),
    fact("image_description", false),
];

/// Exposure: the Exif IFD.
const EXPOSURE: &[Row] = &[
    fact("date_taken", true),
    fact("date_digitised", false),
    fact("exposure_time", true),
    fact("aperture", true),
    fact("iso", true),
    fact("exposure_bias", true),
    fact("exposure_program", true),
    fact("exposure_mode", false),
    fact("metering_mode", true),
    fact("flash", true),
    fact("white_balance", false),
    fact("scene_type", false),
    fact("subject_distance", false),
    fact("digital_zoom", false),
    fact("contrast", false),
    fact("saturation", false),
    fact("sharpness", false),
    fact("brightness", false),
    fact("colour_space", false),
    middle("unique_image_id", false),
];

/// Lens: the Exif IFD's lens tags.
const LENS: &[Row] = &[
    fact("lens", true),
    fact("lens_make", false),
    fact("lens_serial_number", false),
    fact("lens_specification", false),
    fact("focal_length", true),
    fact("focal_length_35mm", true),
    fact("crop_factor", false),
    fact("max_aperture", false),
];

/// GPS: the GPS IFD, then Show on Map.
const GPS: &[Row] = &[
    fact("latitude", true),
    fact("longitude", true),
    fact("altitude", true),
    fact("direction", false),
    fact("gps_time", false),
    fact("speed", false),
    fact("accuracy", false),
    fact("satellites", false),
    fact("map_datum", false),
    own(Widget::MetadataMap),
];

/// A field `id` editing `field`, by its property, shown at first or not.
const fn field(id: &'static str, field: &'static str, shown: bool) -> Row {
    Row::Field(FieldRow { id, field, shown })
}

/// Content: IPTC Core's Content panel, Title among it, and Apertrust's notes last.
const CONTENT: &[Row] = &[
    field("title", "dc:title", true),
    field("headline", "photoshop:Headline", false),
    field("description", "dc:description", true),
    field("description_writer", "photoshop:CaptionWriter", false),
    field("notes", "apertrust:notes", true),
];

/// Creator: IPTC Core's Contact panel, the creator's contact details.
const CREATOR: &[Row] = &[
    field("creator", "dc:creator", true),
    field("job_title", "photoshop:AuthorsPosition", false),
    field("email", "Iptc4xmpCore:CiEmailWork", false),
    field("website", "Iptc4xmpCore:CiUrlWork", false),
    field("phone", "Iptc4xmpCore:CiTelWork", false),
    field("address", "Iptc4xmpCore:CiAdrExtadr", false),
    field("contact_city", "Iptc4xmpCore:CiAdrCity", false),
    field("contact_region", "Iptc4xmpCore:CiAdrRegion", false),
    field("postal_code", "Iptc4xmpCore:CiAdrPcode", false),
    field("contact_country", "Iptc4xmpCore:CiAdrCtry", false),
];

/// Rights: IPTC Core's rights, and Dublin Core's publisher, which darktable keeps.
const RIGHTS: &[Row] = &[
    field("copyright_notice", "dc:rights", true),
    field("usage_terms", "xmpRights:UsageTerms", false),
    field("copyright_status", "xmpRights:Marked", false),
    field("web_statement", "xmpRights:WebStatement", false),
    field("credit_line", "photoshop:Credit", false),
    field("source", "photoshop:Source", false),
    field("publisher", "dc:publisher", true),
];

/// Location: IPTC Core's location shown.
const LOCATION: &[Row] = &[
    field("sublocation", "Iptc4xmpCore:Location", true),
    field("city", "photoshop:City", true),
    field("state", "photoshop:State", false),
    field("country", "photoshop:Country", true),
    field("country_code", "Iptc4xmpCore:CountryCode", false),
];

/// Status: IPTC Core's Status panel, none of it shown at first, so not drawn until ＋ shows
/// a row.
const STATUS: &[Row] = &[
    field("instructions", "photoshop:Instructions", false),
    field("job_id", "photoshop:TransmissionReference", false),
    field("date_created", "photoshop:DateCreated", false),
    field("genre", "Iptc4xmpCore:IntellectualGenre", false),
    field("scene_code", "Iptc4xmpCore:Scene", false),
    field("subject_code", "Iptc4xmpCore:SubjectCode", false),
];

/// The Metadata tool (decision 110), in the Photos tab between Collections and Styles,
/// closed at first, with presets and ＋ and no ↺.
pub const METADATA: UtilityDefinition = UtilityDefinition {
    tool: ToolDefinition {
        id: "metadata",
        tabs: &[Tab {
            id: None,
            rows: &[
                group("basic", BASIC),
                group("camera", CAMERA),
                group("exposure", EXPOSURE),
                group("lens", LENS),
                group("gps", GPS),
                // IPTC Core's groups, of fields.
                group("content", CONTENT),
                group("creator", CREATOR),
                group("rights", RIGHTS),
                group("location", LOCATION),
                group("status", STATUS),
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::None,
        presets: true,
        open_at_first: false,
        menus: &[],
        toolbar: &[],
        cursors: &[],
        lives: crate::Lives::ToolsPanel,
    },
    measures: None,
    titles: &[],
    open: &["basic", "exposure", "content"],
    rows_shown: true,
    tips: &[
        ("basic", "tool-metadata-collapsible-basic-tip"),
        ("camera", "tool-metadata-collapsible-camera-tip"),
        ("exposure", "tool-metadata-collapsible-exposure-tip"),
        ("lens", "tool-metadata-collapsible-lens-tip"),
        ("gps", "tool-metadata-collapsible-gps-tip"),
        ("content", "tool-metadata-collapsible-content-tip"),
        ("creator", "tool-metadata-collapsible-creator-tip"),
        ("rights", "tool-metadata-collapsible-rights-tip"),
        ("location", "tool-metadata-collapsible-location-tip"),
        ("status", "tool-metadata-collapsible-status-tip"),
    ],
};

/// The id of the row a widget of the tool's own is, which ＋ shows or hides as a fact's:
/// Rating and Colour labels; none for Show on Map, which is no row of its own.
pub fn widget_row(widget: Widget) -> Option<&'static str> {
    match widget {
        Widget::MetadataRating => Some("rating"),
        Widget::MetadataLabels => Some("colour_labels"),
        _ => None,
    }
}

/// One of the tool's groups as ＋ and its settings list it: its id and its rows in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub group: &'static str,
    pub rows: Vec<ListedRow>,
}

/// One row of a group as ＋ lists it: its id, and whether it shows at first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListedRow {
    pub id: &'static str,
    pub shown: bool,
}

/// Every group of the tool in order, those with no rows yet among them, each with its
/// rows: its facts and its Rating and Colour labels ([`widget_row`]), built from
/// [`METADATA`].
pub static ROWS: LazyLock<Vec<Listed>> = LazyLock::new(|| {
    let page = METADATA.tool.tabs[0].rows;
    page.iter()
        .filter_map(|row| match row {
            Row::Group(group) => Some(Listed {
                group: group.id,
                rows: group.rows.iter().filter_map(listed_row).collect(),
            }),
            _ => None,
        })
        .collect()
});

/// The row `row` is for ＋, where it is one.
fn listed_row(row: &Row) -> Option<ListedRow> {
    match row {
        Row::Fact(fact) => Some(ListedRow {
            id: fact.id,
            shown: fact.shown,
        }),
        Row::Field(field) => Some(ListedRow {
            id: field.id,
            shown: field.shown,
        }),
        Row::Custom { widget, .. } => widget_row(*widget).map(|id| ListedRow { id, shown: true }),
        _ => None,
    }
}

/// The group holding the row `id`, where it is one of the tool's.
pub fn group_of(id: &str) -> Option<&'static str> {
    ROWS.iter()
        .find(|listed| listed.rows.iter().any(|row| row.id == id))
        .map(|listed| listed.group)
}

/// The field row `id`, where it is one of the tool's: its field by its property.
pub fn field_row(id: &str) -> Option<&'static FieldRow> {
    let page = METADATA.tool.tabs[0].rows;
    let groups = page.iter().filter_map(|row| match row {
        Row::Group(group) => Some(group.rows),
        _ => None,
    });
    groups.flatten().find_map(|row| match row {
        Row::Field(field) if field.id == id => Some(field),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It holds together as a utility, its ＋, tips and facts with their words in British
    /// English, as every utility's.
    #[test]
    fn the_declaration_holds_together_with_its_words() {
        assert_eq!(METADATA.check(), Ok(()));
        let known: std::collections::BTreeSet<String> =
            apertrust_ui::i18n::message_ids().into_iter().collect();
        let wanted = crate::messages::of_utility(&METADATA);
        assert!(wanted.contains(&"tool-metadata-fact-aperture-tag".to_owned()));
        let missing: Vec<&String> = wanted.iter().filter(|id| !known.contains(*id)).collect();
        assert!(missing.is_empty(), "no message for {missing:?}");
    }

    /// The groups in the standards' order, each row once among them all.
    #[test]
    fn the_groups_and_rows_follow_the_declaration() {
        let groups: Vec<&str> = ROWS.iter().map(|listed| listed.group).collect();
        let order = [
            "basic", "camera", "exposure", "lens", "gps", "content", "creator", "rights",
            "location", "status",
        ];
        assert_eq!(groups, order);
        let ids: Vec<&str> = ROWS
            .iter()
            .flat_map(|l| l.rows.iter().map(|r| r.id))
            .collect();
        let mut once = ids.clone();
        once.sort_unstable();
        once.dedup();
        assert_eq!(once.len(), ids.len());
        // Basic's nine, the copy's name among them; Exposure's eight shown at first.
        let shown = |group: &str| {
            let listed = ROWS.iter().find(|l| l.group == group).unwrap();
            listed.rows.iter().filter(|row| row.shown).count()
        };
        assert_eq!(shown("basic"), 9);
        assert_eq!(shown("exposure"), 8);
        assert_eq!(shown("camera"), 2);
        assert_eq!(shown("lens"), 3);
        assert_eq!(shown("gps"), 3);
        assert_eq!(group_of("aperture"), Some("exposure"));
        assert_eq!(group_of("rating"), Some("basic"));
        assert_eq!(group_of("title"), Some("content"));
        assert_eq!(shown("content"), 3);
        assert_eq!(shown("status"), 0);
        assert_eq!(group_of("city"), Some("location"));
    }
}
