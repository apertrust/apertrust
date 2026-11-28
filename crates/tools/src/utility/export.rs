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

//! Export's declaration (decisions 90 and 93; `docs/gui.md`, The Export tool): darktable's
//! export module row for row, its groups under darktable's labels in sentence case.
//!
//! - **Storage options** and **Format options**, labels over their rows, always shown;
//!   **Global options**, a collapsible group open at first.
//! - **Its settings are setting rows** naming the `export.*` settings by their keys in full,
//!   with Export's own words (`export-*`), the app binding them to `apertrust-export`'s
//!   settings, on which this crate never depends. Each choice lists darktable's options in
//!   darktable's order; those not built yet the app draws disabled, saying why, and
//!   JPEG's and PNG's rows show only for their format. Allow upscaling, High quality
//!   resampling and Store masks are darktable's choices of No and Yes.
//! - **Store masks, Intent and Style** have no setting yet: drawn disabled at darktable's
//!   default.
//! - **Its size line in pixels,** `[w] × [h] px`, is a line of fields (decision 107) over
//!   `export.size.width` and `export.size.height`.
//! - **Its custom rows** are Target storage, the path line, the print and scale lines, the
//!   groups to come and the tool's end, the Summary with the Export button.

use crate::{
    ControlKind, Field, Fields, Group, GroupKind, Reset, Row, Separator, SettingRow, SettingWords,
    Tab, ToolDefinition, Track, Unit, Widget, Word,
};

use super::{UtilityDefinition, item};

/// A setting row of Export's: `kind` kept under `key` where it is kept, with `words`.
const fn setting(
    id: &'static str,
    key: Option<&'static str>,
    kind: ControlKind,
    words: &'static SettingWords,
) -> Row {
    Row::Setting(SettingRow {
        id,
        kind,
        decimals: 0,
        track: Track::Plain,
        key,
        words: Some(words),
        end: &[],
    })
}

/// A choice of `options` at `default`.
const fn choice(options: &'static [&'static str], default: u32) -> ControlKind {
    ControlKind::Choice {
        options,
        default,
        unported: &[],
    }
}

/// A whole number from `min` to `max`, at `default`.
const fn whole(min: f32, max: f32, default: f32) -> ControlKind {
    ControlKind::Number {
        min,
        max,
        step: 1.0,
        default,
    }
}

/// `label`'s words, its promise `promise` where it has one, with `options`.
const fn words(
    label: &'static str,
    promise: Option<&'static str>,
    options: &'static [Word],
) -> SettingWords {
    SettingWords {
        label,
        full: None,
        promise,
        options,
    }
}

/// darktable's choice of No and Yes (decision 90).
const NO_YES: &[Word] = &[
    Word::Message("export-option-no"),
    Word::Message("export-option-yes"),
];

const ON_CONFLICT: SettingWords = words(
    "export-on-conflict",
    None,
    &[
        Word::Message("export-conflict-unique"),
        Word::Message("export-conflict-overwrite"),
        Word::Message("export-conflict-changed"),
        Word::Message("export-conflict-skip"),
    ],
);

/// darktable's formats in its order, Copy last; JPEG and PNG are the ones built.
const FORMATS: &[&str] = &[
    "avif", "heif", "jpeg", "jpeg2000", "jpeg_xl", "openexr", "pdf", "pfm", "png", "ppm", "tiff",
    "webp", "xcf", "copy",
];
const FORMAT: SettingWords = words(
    "export-file-format",
    None,
    &[
        Word::Name("AVIF"),
        Word::Name("HEIF"),
        Word::Name("JPEG (8-bit)"),
        Word::Name("JPEG 2000 (12-bit)"),
        Word::Name("JPEG XL"),
        Word::Name("OpenEXR"),
        Word::Name("PDF"),
        Word::Name("PFM"),
        Word::Name("PNG"),
        Word::Name("PPM (16-bit)"),
        Word::Name("TIFF"),
        Word::Name("WebP"),
        Word::Name("XCF"),
        Word::Message("export-format-copy"),
    ],
);
const QUALITY: SettingWords = words("export-jpeg-quality", None, &[]);
const SUBSAMPLE: SettingWords = words(
    "export-jpeg-subsample",
    Some("export-jpeg-subsample-tip"),
    &[
        Word::Message("export-subsample-auto"),
        Word::Name("4:4:4"),
        Word::Name("4:4:0"),
        Word::Name("4:2:2"),
        Word::Name("4:2:0"),
    ],
);
const BIT_DEPTH: SettingWords = words(
    "export-png-bit-depth",
    None,
    &[
        Word::Message("export-bits-8"),
        Word::Message("export-bits-16"),
    ],
);
const COMPRESSION: SettingWords = words("export-png-compression", None, &[]);
const SIZE_MODE: SettingWords = words(
    "export-set-size",
    None,
    &[
        Word::Message("export-mode-pixels"),
        Word::Message("export-mode-cm"),
        Word::Message("export-mode-inch"),
        Word::Message("export-mode-scale"),
    ],
);
const WIDTH: SettingWords = words("export-width", None, &[]);
const HEIGHT: SettingWords = words("export-height", None, &[]);
/// The largest width or height in pixels, 0 for no limit, as a field of the size line.
const fn side(id: &'static str, key: &'static str, words: &'static SettingWords) -> SettingRow {
    SettingRow {
        id,
        kind: whole(0.0, 100_000.0, 0.0),
        decimals: 0,
        track: Track::Plain,
        key: Some(key),
        words: Some(words),
        end: &[],
    }
}
/// The size line in pixels, `[w] × [h] px`, one line with no label at every width the
/// Tools panel takes, to screen readers "Size in pixels".
const SIZE_IN_PIXELS: Fields = Fields {
    id: "size",
    words: "export-size-pixels",
    fields: &[
        Field {
            setting: side("width", "export.size.width", &WIDTH),
            before: None,
            unit: Some(Unit::Pixels),
        },
        Field {
            setting: side("height", "export.size.height", &HEIGHT),
            before: Some(Separator::Times),
            unit: Some(Unit::Pixels),
        },
    ],
};
const UPSCALE: SettingWords = words("export-upscale", None, NO_YES);
const HIGH_QUALITY: SettingWords = words(
    "export-high-quality",
    Some("export-high-quality-tip"),
    NO_YES,
);
const MASKS: SettingWords = words("export-masks", Some("export-masks-later"), NO_YES);
/// darktable's twelve export profiles after Image settings, in its order, by the Output
/// colour profile tool's words for them; Image settings and sRGB are the ones built.
const PROFILES: &[&str] = &[
    "image_settings",
    "srgb",
    "adobe_rgb",
    "lin_rec709",
    "rec709",
    "lin_rec2020",
    "pq_rec2020",
    "hlg_rec2020",
    "pq_p3",
    "hlg_p3",
    "display_p3",
    "prophoto_rgb",
    "brg",
];
const PROFILE: SettingWords = words(
    "export-profile",
    Some("export-profile-tip"),
    &[
        Word::Message("export-profile-image"),
        Word::Message("tool-colorout-control-type-option-srgb"),
        Word::Message("tool-colorout-control-type-option-adobe_rgb"),
        Word::Message("tool-colorout-control-type-option-lin_rec709"),
        Word::Message("tool-colorout-control-type-option-rec709"),
        Word::Message("tool-colorout-control-type-option-lin_rec2020"),
        Word::Message("tool-colorout-control-type-option-pq_rec2020"),
        Word::Message("tool-colorout-control-type-option-hlg_rec2020"),
        Word::Message("tool-colorout-control-type-option-pq_p3"),
        Word::Message("tool-colorout-control-type-option-hlg_p3"),
        Word::Message("tool-colorout-control-type-option-display_p3"),
        Word::Message("tool-colorout-control-type-option-prophoto_rgb"),
        Word::Message("tool-colorout-control-type-option-brg"),
    ],
);
const INTENT: SettingWords = words(
    "export-intent",
    Some("export-intent-tip"),
    &[
        Word::Message("export-profile-image"),
        Word::Message("export-intent-perceptual"),
        Word::Message("export-intent-relative"),
        Word::Message("export-intent-saturation"),
        Word::Message("export-intent-absolute"),
    ],
);
const STYLE: SettingWords = words(
    "export-style",
    Some("export-style-later"),
    &[Word::Message("export-style-none")],
);
const NOTES: SettingWords = words("export-notes", Some("export-notes-tip"), NO_YES);

/// A custom row of Export's own widget.
const fn custom(widget: Widget) -> Row {
    Row::Custom {
        widget,
        controls: &[],
    }
}

/// Export, the Photos tab's last tool: its ↺ its own, Default settings, and its presets
/// in its header; its page Storage options, Format options and Global options, then the
/// groups to come and its end; Export… and Export Again offered for the Toolbar.
pub const EXPORT: UtilityDefinition = UtilityDefinition {
    tool: ToolDefinition {
        id: "export",
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Group(Group {
                    id: "storage_options",
                    kind: GroupKind::Plain,
                    rows: &[
                        custom(Widget::ExportStorage),
                        custom(Widget::ExportPath),
                        setting(
                            "on_conflict",
                            Some("export.disk.on_conflict"),
                            choice(
                                &[
                                    "create_unique_filename",
                                    "overwrite",
                                    "overwrite_if_changed",
                                    "skip",
                                ],
                                0,
                            ),
                            &ON_CONFLICT,
                        ),
                    ],
                }),
                Row::Group(Group {
                    id: "format_options",
                    kind: GroupKind::Plain,
                    rows: &[
                        setting("format", Some("export.format"), choice(FORMATS, 2), &FORMAT),
                        Row::WhenSetting {
                            setting: "format",
                            is: &["jpeg"],
                            rows: &[
                                setting(
                                    "jpeg_quality",
                                    Some("export.jpeg.quality"),
                                    whole(5.0, 100.0, 95.0),
                                    &QUALITY,
                                ),
                                setting(
                                    "jpeg_subsample",
                                    Some("export.jpeg.subsample"),
                                    choice(&["auto", "yuv444", "yuv440", "yuv422", "yuv420"], 0),
                                    &SUBSAMPLE,
                                ),
                            ],
                        },
                        Row::WhenSetting {
                            setting: "format",
                            is: &["png"],
                            rows: &[
                                setting(
                                    "png_bit_depth",
                                    Some("export.png.bit_depth"),
                                    choice(&["bits_8", "bits_16"], 0),
                                    &BIT_DEPTH,
                                ),
                                setting(
                                    "png_compression",
                                    Some("export.png.compression"),
                                    whole(0.0, 9.0, 5.0),
                                    &COMPRESSION,
                                ),
                            ],
                        },
                    ],
                }),
                Row::Group(Group {
                    id: "global",
                    kind: GroupKind::Collapsible,
                    rows: &[
                        setting(
                            "size_mode",
                            Some("export.size.mode"),
                            choice(&["pixels", "cm", "inch", "scale"], 0),
                            &SIZE_MODE,
                        ),
                        Row::WhenSetting {
                            setting: "size_mode",
                            is: &["pixels"],
                            rows: &[Row::Fields(SIZE_IN_PIXELS)],
                        },
                        // The print and scale lines, which draw nothing in pixels mode but
                        // count the resolution and the scale in Global options' "N set" in
                        // every mode, as before.
                        custom(Widget::ExportSize),
                        setting(
                            "upscale",
                            Some("export.size.upscale"),
                            ControlKind::Bool { default: false },
                            &UPSCALE,
                        ),
                        setting(
                            "high_quality",
                            Some("export.size.high_quality"),
                            ControlKind::Bool { default: false },
                            &HIGH_QUALITY,
                        ),
                        setting("masks", None, ControlKind::Bool { default: false }, &MASKS),
                        setting(
                            "profile",
                            Some("export.profile"),
                            choice(PROFILES, 0),
                            &PROFILE,
                        ),
                        setting(
                            "intent",
                            None,
                            choice(
                                &[
                                    "image_settings",
                                    "perceptual",
                                    "relative",
                                    "saturation",
                                    "absolute",
                                ],
                                0,
                            ),
                            &INTENT,
                        ),
                        setting("style", None, choice(&["none"], 0), &STYLE),
                    ],
                }),
                // Drawn disabled, closed, until metadata in exports is built.
                Row::Group(Group {
                    id: "metadata",
                    kind: GroupKind::Collapsible,
                    rows: &[setting(
                        "notes",
                        Some("export.metadata.notes"),
                        ControlKind::Bool { default: false },
                        &NOTES,
                    )],
                }),
                custom(Widget::ExportLater),
                custom(Widget::ExportEnd),
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::Own,
        presets: true,
        open_at_first: false,
        menus: &[],
        // Export… and Export Again for the Toolbar (decision 92).
        toolbar: &[item("export", "export"), item("export-again", "export")],
        cursors: &[],
        lives: crate::Lives::ToolsPanel,
    },
    measures: None,
    titles: &[
        ("storage_options", "export-storage-options"),
        ("format_options", "export-format-options"),
        ("global", "export-global-options"),
        ("metadata", "export-metadata"),
    ],
    open: &["global"],
    rows_shown: false,
    tips: &[],
};
