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

//! The session protocol as data (decision 108): every command and event, but a frame and
//! the tests' own commands, read back from JSON as it was written, in decision 59's shape,
//! tagged by `type`. No GPU: every sample is built by hand.
//!
//! Each enum of commands or events has a function that matches every variant with no
//! wildcard, giving its index: a new variant does not compile until it has an index, and
//! the round trip fails until it has a sample.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use apertrust_base::cache::{self, Limit, Sizes, Usage};
use apertrust_base::jobs::{Argument, Failure as JobFailure, Progress};
use apertrust_export::disk::{Action, Blocked, Preflight};
use apertrust_export::settings::OnConflict;
use apertrust_export::size::Request;
use apertrust_export::variables::{NotHere, PatternError, Unavailable};
use apertrust_export::{Format, jpeg, png};
use apertrust_library::clock::ClockError;
use apertrust_library::ops::{EditState, RawThumbnails, Source, Thumbnail};
use apertrust_library::sidecar::{Newer, Problem, Reason, SidecarError};
use apertrust_library::summary::{Facts, Summed};
use apertrust_library::walk::WalkedFolder;
use apertrust_plugin::{NotPorted, ParamError, RAW_HISTOGRAM_BINS, RawHistogram};
use apertrust_raw::DateTime;
use apertrust_tools::pick::{LastPick, Patch, PickWarnings, Readout, Short};
use apertrust_tools::{ControlError, DefinitionError, PickerKind, Unit};
use serde::de::DeserializeOwned;

use super::*;
use crate::export_preview::{
    ActiveSize, Fate, FolderCheck, OutputSize, PathProblem, PhotoName, SizeProblem,
};
use crate::scopes::{Histogram, Totals, Vectorscope, Waveform};
use crate::view::View;

/// A value read from its JSON, for the types whose fields are private: a job's id and a
/// machine id.
fn wire<T: DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).unwrap()
}

fn photo() -> PathBuf {
    PathBuf::from("/photos/a.cr3")
}

fn other() -> PathBuf {
    PathBuf::from("/photos/b.nef")
}

fn job() -> JobId {
    wire("7")
}

/// A step id above 2^53, which a JSON number would round.
const STEP: StepId = StepId(0xfedc_ba98_7654_3210);

const BATCH: BatchId = BatchId(0x8000_0000_0000_0001);

fn values() -> ToolValues {
    let mut values = ToolValues::new();
    values.set("exposure", ControlValue::Number(0.5));
    values.set_mark("exposure", Some(Mark::Kept));
    values.set_enabled(true);
    values
}

fn view_only() -> ViewOnly {
    ViewOnly {
        build: Some("0.5.0+3~g1a2b3c4".to_owned()),
        reason: Reason::Newer(Newer::ToolVersion {
            tool: "exposure".to_owned(),
            version: "3".to_owned(),
            has: 2,
        }),
    }
}

fn export_settings(format: Format) -> ExportSettings {
    ExportSettings {
        path: "$(FILE_FOLDER)/darktable_exported/$(FILE_NAME)".to_owned(),
        on_conflict: OnConflict::OverwriteIfChanged,
        format,
        size: Request::default(),
    }
}

fn places() -> Places {
    Places {
        home: Some(PathBuf::from("/home/me")),
        pictures: Some(PathBuf::from("/home/me/Pictures")),
        desktop: None,
        user_name: "Me".to_owned(),
    }
}

/// At least one value of every command a client sends, and of every variant of the
/// commands it carries.
pub(super) fn every_command() -> Vec<Command> {
    let tool = || "exposure".to_owned();
    let set = Command::SetControl {
        tool: tool(),
        control: "exposure".to_owned(),
        value: ControlValue::Number(0.5),
        done: true,
    };
    let reset_control = Command::ResetControl {
        tool: tool(),
        control: "exposure".to_owned(),
    };
    let reset_tool = Command::ResetTool { tool: tool() };
    let set_tool = Command::SetTool {
        tool: tool(),
        values: values(),
    };
    let keep = Command::Keep {
        tool: tool(),
        control: "black".to_owned(),
        kept: false,
    };
    let apply_preset = Command::ApplyPreset {
        tool: "white-balance".to_owned(),
        preset: "mode".to_owned(),
        option: "camera".to_owned(),
    };
    let set_param = Command::Set {
        operation: "rawprepare".to_owned(),
        param: "flat_field".to_owned(),
        value: Value::Choice(1),
    };
    let tool_picker = PickerRef::Tool {
        tool: "exposure",
        picker: "exposure",
    };
    let keys = PickKeys {
        command: true,
        shift: false,
    };
    let settings = vec![("target", ControlValue::Number(-0.5))];
    vec![
        Command::Open(photo()),
        Command::Close,
        Command::Resize {
            width: 1200,
            height: 800,
        },
        Command::View(View {
            scale: ViewScale::Factor(2.0),
            centre: [0.25, 0.75],
        }),
        set.clone(),
        reset_control.clone(),
        reset_tool.clone(),
        set_tool.clone(),
        Command::CancelGesture { tool: tool() },
        keep.clone(),
        apply_preset.clone(),
        set_param.clone(),
        Command::Reset,
        Command::Undo,
        Command::Redo,
        Command::Rate {
            photos: vec![photo(), other()],
            change: RatingChange::Label { label: 2, on: true },
        },
        Command::SetMetadata {
            photos: vec![photo(), other()],
            values: Vec::new(),
            fields: vec![
                ("dc:title".to_owned(), Some("Morning market".to_owned())),
                ("photoshop:City".to_owned(), None),
            ],
        },
        Command::RenameCopy {
            name: "Black and white".to_owned(),
        },
        Command::GoTo { step: Some(STEP) },
        Command::Compress { up_to: None },
        Command::Truncate {
            after: Some(StepId(1)),
        },
        Command::DiscardHistory,
        Command::Step {
            source: StepSource::Assistant,
            note: Some(Note::Text("Brighter".to_owned())),
            commands: vec![
                set,
                reset_control,
                reset_tool,
                set_tool,
                keep,
                apply_preset,
                set_param,
                Command::Reset,
            ],
        },
        Command::ApplyStyle {
            name: "Warm".to_owned(),
            file: "warm.apstyle".to_owned(),
            mode: StyleMode::Replace,
            tools: vec![(tool(), values())],
            photos: vec![photo()],
        },
        Command::ClearStyle {
            name: "Warm".to_owned(),
            file: "builtin:warm".to_owned(),
        },
        Command::UndoBatch { batch: BATCH },
        Command::Hide {
            tool: tool(),
            hidden: true,
        },
        Command::RetrySave,
        Command::Scopes(Some(Graphs {
            histogram: true,
            waveform: false,
            vectorscope: true,
        })),
        Command::Scopes(None),
        Command::Picker(PickerCommand::On {
            picker: PickerRef::ColourPicker,
            area: true,
            keys: PickKeys::default(),
            settings: Vec::new(),
            channel: None,
        }),
        Command::Picker(PickerCommand::On {
            picker: tool_picker,
            area: false,
            keys,
            settings: settings.clone(),
            channel: Some("luminance"),
        }),
        Command::Picker(PickerCommand::Point {
            at: [0.5, 0.25],
            done: false,
            keys,
        }),
        Command::Picker(PickerCommand::Area {
            from: [0.125, 0.25],
            to: [0.75, 0.875],
            done: true,
            keys,
        }),
        Command::Picker(PickerCommand::Switch),
        Command::Picker(PickerCommand::Reset),
        Command::Picker(PickerCommand::Settings {
            tool: "exposure",
            settings: settings.clone(),
        }),
        Command::Picker(PickerCommand::Channel { channel: None }),
        Command::Picker(PickerCommand::Off),
        Command::Picker(PickerCommand::Once {
            picker: tool_picker,
            region: Region::Point([0.5, 0.5]),
            keys,
            settings: settings.clone(),
            channel: Some("luminance"),
        }),
        Command::Picker(PickerCommand::Once {
            picker: PickerRef::ColourPicker,
            region: Region::Area([0.0, 0.0, 1.0, 1.0]),
            keys,
            settings,
            channel: None,
        }),
        Command::CancelJob { job: job() },
        Command::RetryJob { job: job() },
        Command::Export {
            photos: vec![photo(), other()],
            settings: export_settings(Format::Jpeg(jpeg::Options::default())),
            places: places(),
        },
        Command::PreviewExport {
            photos: vec![photo()],
            settings: export_settings(Format::Png(png::Options::default())),
            places: Places::default(),
        },
        Command::Summarise {
            photos: vec![photo(), other()],
        },
        Command::Library(LibraryCommand::Open(Source::Folder(PathBuf::from(
            "/photos",
        )))),
        Command::Library(LibraryCommand::Open(Source::Photos(vec![photo(), other()]))),
        Command::Library(LibraryCommand::Close),
        Command::Library(LibraryCommand::Thumbnails {
            photos: vec!["a.cr3".to_owned()],
            size: 256,
        }),
        Command::Library(LibraryCommand::PhotoThumbnails {
            paths: vec![photo()],
            size: 512,
        }),
        Command::Library(LibraryCommand::Subfolders {
            folder: PathBuf::from("/photos/2026"),
        }),
        Command::Library(LibraryCommand::RawThumbnails(RawThumbnails::P720)),
        Command::Selection(SelectionCommand::Set(vec![photo(), other()])),
        Command::Selection(SelectionCommand::Add(photo())),
        Command::Selection(SelectionCommand::Remove(other())),
        Command::Selection(SelectionCommand::Range {
            to: other(),
            order: vec![photo(), other()],
        }),
        Command::Selection(SelectionCommand::All),
        Command::Selection(SelectionCommand::Clear),
        Command::Selection(SelectionCommand::Invert),
        Command::Selection(SelectionCommand::Unedited),
        Command::Selection(SelectionCommand::Previous),
        Command::Cache(CacheCommand::Limit(Limit::Bytes(1 << 30))),
        Command::Cache(CacheCommand::Limit(Limit::Unlimited)),
        Command::Cache(CacheCommand::Sizes),
        Command::Cache(CacheCommand::Clear {
            kind: cache::THUMBNAILS,
        }),
    ]
}

/// How many variants [`command_variant`] counts.
const COMMAND_VARIANTS: usize = 38;

/// Each command's variant, `None` for the tests' own. A new variant takes the next index
/// and raises the count.
fn command_variant(command: &Command) -> Option<usize> {
    Some(match command {
        Command::Open(_) => 0,
        Command::Close => 1,
        Command::Resize { .. } => 2,
        Command::View(_) => 3,
        Command::SetControl { .. } => 4,
        Command::ResetControl { .. } => 5,
        Command::ResetTool { .. } => 6,
        Command::SetTool { .. } => 7,
        Command::CancelGesture { .. } => 8,
        Command::Keep { .. } => 9,
        Command::ApplyPreset { .. } => 10,
        Command::Set { .. } => 11,
        Command::Reset => 12,
        Command::Undo => 13,
        Command::Redo => 14,
        Command::Rate { .. } => 15,
        Command::GoTo { .. } => 16,
        Command::Compress { .. } => 17,
        Command::Truncate { .. } => 18,
        Command::DiscardHistory => 19,
        Command::Step { .. } => 20,
        Command::ApplyStyle { .. } => 21,
        Command::ClearStyle { .. } => 22,
        Command::UndoBatch { .. } => 23,
        Command::Hide { .. } => 24,
        Command::RetrySave => 25,
        Command::Scopes(_) => 26,
        Command::Picker(_) => 27,
        Command::CancelJob { .. } => 28,
        Command::RetryJob { .. } => 29,
        Command::Export { .. } => 30,
        Command::PreviewExport { .. } => 31,
        Command::Summarise { .. } => 32,
        Command::Library(_) => 33,
        Command::Selection(_) => 34,
        Command::Cache(_) => 35,
        Command::SetMetadata { .. } => 36,
        Command::RenameCopy { .. } => 37,
        Command::Synthetic(_)
        | Command::HoldExport(_)
        | Command::HoldPreview(_)
        | Command::HoldSummary(_)
        | Command::HoldFolder(_)
        | Command::HoldWalk(_)
        | Command::HoldThumbnails(_)
        | Command::HoldDecode(_)
        | Command::HoldChunks(_)
        | Command::HoldClear(_) => return None,
    })
}

const PICKER_VARIANTS: usize = 9;

/// Each picker command's variant; a new one takes the next index and raises the count.
fn picker_variant(command: &PickerCommand) -> usize {
    match command {
        PickerCommand::On { .. } => 0,
        PickerCommand::Point { .. } => 1,
        PickerCommand::Area { .. } => 2,
        PickerCommand::Switch => 3,
        PickerCommand::Reset => 4,
        PickerCommand::Settings { .. } => 5,
        PickerCommand::Channel { .. } => 6,
        PickerCommand::Off => 7,
        PickerCommand::Once { .. } => 8,
    }
}

const LIBRARY_COMMAND_VARIANTS: usize = 6;

/// Each library command's variant; a new one takes the next index and raises the count.
fn library_command_variant(command: &LibraryCommand) -> usize {
    match command {
        LibraryCommand::Open(_) => 0,
        LibraryCommand::Close => 1,
        LibraryCommand::Thumbnails { .. } => 2,
        LibraryCommand::PhotoThumbnails { .. } => 3,
        LibraryCommand::Subfolders { .. } => 4,
        LibraryCommand::RawThumbnails(_) => 5,
    }
}

const SELECTION_VARIANTS: usize = 9;

/// Each selection command's variant; a new one takes the next index and raises the count.
fn selection_variant(command: &SelectionCommand) -> usize {
    match command {
        SelectionCommand::Set(_) => 0,
        SelectionCommand::Add(_) => 1,
        SelectionCommand::Remove(_) => 2,
        SelectionCommand::Range { .. } => 3,
        SelectionCommand::All => 4,
        SelectionCommand::Clear => 5,
        SelectionCommand::Invert => 6,
        SelectionCommand::Unedited => 7,
        SelectionCommand::Previous => 8,
    }
}

const CACHE_COMMAND_VARIANTS: usize = 3;

/// Each cache command's variant; a new one takes the next index and raises the count.
fn cache_command_variant(command: &CacheCommand) -> usize {
    match command {
        CacheCommand::Limit(_) => 0,
        CacheCommand::Sizes => 1,
        CacheCommand::Clear { .. } => 2,
    }
}

fn session_error(error: SessionError) -> Event {
    Event::Failed(error)
}

/// At least one value of every event but a frame, and of every variant of the events and
/// errors it carries.
fn every_event() -> Vec<Event> {
    let mut image = apertrust_tools::testing::image();
    let mut counts = vec![0; RAW_HISTOGRAM_BINS];
    counts[1024] = 6912;
    image.raw_histogram = RawHistogram::new(counts);
    let exif = apertrust_raw::Exif {
        taken: Some(DateTime {
            year: 2026,
            month: 11,
            day: 26,
            hour: 9,
            minute: 30,
            second: 15,
            millisecond: 250,
            offset_minutes: Some(60),
        }),
        exposure_time: Some(0.25),
        ..Default::default()
    };
    let thumbnail = Thumbnail {
        width: 2,
        height: 1,
        pixels: Arc::from([1, 2, 3, 255, 4, 5, 6, 255].as_slice()),
    };
    let stats = Stats {
        mean: [0.5, 0.25, 0.125, 1.0],
        min: [0.0; 4],
        max: [1.0; 4],
    };
    let patch = Patch {
        srgb: [0.5, 0.5, 0.5],
        lch: [50.0, 0.0, 0.0],
    };
    let message = Message::new("job-export")
        .with("count", 3)
        .with("folder", Argument::Path(PathBuf::from("/photos")))
        .with(
            "size",
            Argument::Decimals {
                value: 1.5,
                at_least: 1,
            },
        )
        .with("name", "a.cr3");
    let rating = Rating {
        stars: 3,
        rejected: false,
        labels: 0b101,
    };
    let folder = PathBuf::from("/photos");
    let shot = Arc::new(apertrust_library::ops::Shot {
        exif: exif.clone(),
        iso: Some(400.0),
        size: [96, 72],
        sensor: [100, 76],
        format: "CR3".to_owned(),
        make: "Canon".to_owned(),
        alias: "EOS R5".to_owned(),
        file_size: 1 << 20,
    });
    let title = apertrust_library::fields::Fields {
        values: vec![("dc:title".to_owned(), FieldValue::Text("Tram".to_owned()))],
        name: String::new(),
        copies: 1,
    };
    let summary = apertrust_library::summary::build(&[
        Summed {
            name: "a.cr3",
            folder: &folder,
            size: 1 << 20,
            edited: true,
            facts: Facts::Read(&shot),
            fields: &title,
        },
        Summed {
            name: "b.nef",
            folder: Path::new("/photos/2026"),
            size: 2 << 20,
            edited: false,
            facts: Facts::Missing,
            fields: &Default::default(),
        },
    ]);
    let tool_error = ToolError::Definition(DefinitionError::UnknownControl("brightness"));
    let mut events = vec![
        Event::OpenPreview(OpenPreview {
            open: 3,
            picture: Some(thumbnail.clone()),
            source: PreviewSource::Embedded,
        }),
        Event::Opened(Photo {
            path: photo(),
            open: 3,
            camera: "Canon EOS R5".to_owned(),
            width: 96,
            height: 72,
            view_only: Some(view_only()),
            exposure_time: Some(0.25),
            aperture: Some(2.5),
            focal_length: None,
            exif: Box::new(exif),
            format: "CR3".to_owned(),
            file_size: 1 << 20,
            sensor: [100, 76],
            image: Box::new(image),
        }),
        Event::Values(Values {
            tools: vec![ToolState {
                tool: "exposure",
                enabled: true,
                edited: true,
                hidden: false,
                controls: vec![
                    ControlState {
                        control: "exposure",
                        value: ControlValue::Number(0.5),
                        default: ControlValue::Number(0.0),
                        mark: Some(Mark::Kept),
                        available: true,
                        options: Vec::new(),
                        range: Some(Range {
                            min: -18.0,
                            max: 18.0,
                            soft: (-3.0, 4.0),
                            step: 0.125,
                        }),
                        visible: true,
                        label_value: Some(LabelValue {
                            value: -0.5,
                            unit: Unit::Ev,
                            format: ValueFormat::Thirds,
                        }),
                    },
                    ControlState {
                        control: "mode",
                        value: ControlValue::Choice(1),
                        default: ControlValue::Choice(0),
                        mark: Some(Mark::Auto),
                        available: false,
                        options: vec!["manual", "automatic"],
                        range: None,
                        visible: false,
                        label_value: None,
                    },
                    ControlState {
                        control: "compensate",
                        value: ControlValue::Bool(true),
                        default: ControlValue::Bool(false),
                        mark: None,
                        available: true,
                        options: Vec::new(),
                        range: None,
                        visible: true,
                        label_value: Some(LabelValue {
                            value: 2.0,
                            unit: Unit::Percent,
                            format: ValueFormat::Decimals(1),
                        }),
                    },
                    ControlState {
                        control: "curve",
                        value: ControlValue::Points(vec![[0.0, 0.0], [0.5, 0.625], [1.0, 1.0]]),
                        default: ControlValue::Points(Vec::new()),
                        mark: None,
                        available: true,
                        options: Vec::new(),
                        range: None,
                        visible: true,
                        label_value: None,
                    },
                ],
                presets: vec![PresetState {
                    preset: "mode",
                    matching: Some("camera"),
                    available: vec!["camera", "custom"],
                }],
                views: vec![CoordinateState {
                    view: "temperature",
                    coordinate: "tint",
                    value: 1.25,
                    default: 1.0,
                    mark: None,
                    available: false,
                }],
                computed: vec![ComputedState {
                    id: "deflicker",
                    value: Some(0.75),
                }],
                readouts: vec![
                    ReadoutState {
                        id: "cct",
                        readout: Some(Readout {
                            text: "readout-cct",
                            value: Some(5000.0),
                            swatch: Some([1.0, 0.875, 0.75]),
                        }),
                    },
                    ReadoutState {
                        id: "colour",
                        readout: None,
                    },
                ],
                actions: vec![ActionState {
                    id: "auto",
                    available: true,
                }],
                warnings: vec![WarningState {
                    id: "clipped",
                    on: false,
                }],
            }],
            settings: vec![Setting {
                operation: "rawprepare",
                param: "flat_field",
                value: Value::Int(3),
            }],
            edited: true,
            through: 9,
            extent: [96, 72],
            live: true,
            styles: vec![StyleState {
                name: "Warm".to_owned(),
                file: "warm.apstyle".to_owned(),
                tools: vec!["exposure".to_owned()],
            }],
        }),
        Event::Scopes(Reading {
            through: 9,
            width: 96,
            height: 72,
            totals: Totals {
                pixels: 6912,
                shadows: 12,
                highlights: 3,
            },
            histogram: Some(Histogram {
                red: vec![1, 2],
                green: vec![3, 4],
                blue: vec![5, 6],
                luminance: vec![7, 8],
            }),
            waveform: Some(Waveform {
                columns: 1,
                levels: 2,
                red: vec![1, 2],
                green: vec![3, 4],
                blue: vec![5, 6],
                luminance: vec![7, 8],
            }),
            vectorscope: Some(Box::new(Vectorscope {
                size: 1,
                counts: vec![4],
                colours: Arc::from([[1, 2, 3]].as_slice()),
            })),
            // No timestamps on this device.
            gpu_ms: f64::NAN,
            latency_ms: 1.5,
        }),
        Event::Picker {
            through: 10,
            reading: Some(PickerReading {
                through: 10,
                picker: PickerRef::Tool {
                    tool: "exposure",
                    picker: "exposure",
                },
                region: Region::Area([0.125, 0.25, 0.75, 0.875]),
                tool: Some(ToolReading {
                    input: Some(stats),
                    output: None,
                    picked: Some(Shown {
                        patch: Some(patch),
                        settings: vec![("target", ControlValue::Number(-0.5))],
                        last: Some(LastPick {
                            before: 0.0,
                            after: 0.5,
                            exposure: 0.5,
                            patches: [patch, patch],
                        }),
                        warnings: PickWarnings {
                            clipped: true,
                            short: Some(Short {
                                exposure: 4.0,
                                reached: -1.5,
                                target: -0.5,
                                black: false,
                            }),
                        },
                    }),
                }),
                frame: Some(FrameReading {
                    rgb: [[0.5; 3]; 3],
                    lab: [[50.0, 0.0, 0.0]; 3],
                }),
            }),
        },
        Event::Picker {
            through: 11,
            reading: None,
        },
        Event::Saved {
            path: PathBuf::from("/photos/a.cr3.apxmp"),
            through: 9,
        },
        Event::Job {
            event: JobEvent::Added {
                job: job(),
                kind: "export",
                queue: Some(Queue::UserExport),
                title: message.clone(),
                cancellable: true,
                destination: Some(Destination::Folder(PathBuf::from("/photos/out"))),
                detail: Some(Message::new("job-export-detail")),
                progress: Progress {
                    done: 0,
                    total: Some(3),
                },
                retries: Some(wire("6")),
                quiet: false,
            },
            command: Some(12),
        },
        Event::Job {
            event: JobEvent::Progress {
                job: job(),
                done: 1,
                total: None,
            },
            command: None,
        },
        Event::Job {
            event: JobEvent::Title {
                job: job(),
                title: message.clone(),
            },
            command: None,
        },
        Event::Job {
            event: JobEvent::Detail {
                job: job(),
                detail: None,
            },
            command: None,
        },
        Event::Job {
            event: JobEvent::Destination {
                job: job(),
                destination: Destination::File(PathBuf::from("/photos/out/a.jpg")),
            },
            command: None,
        },
        Event::Job {
            event: JobEvent::State {
                job: job(),
                state: State::Running,
            },
            command: None,
        },
        Event::Job {
            event: JobEvent::Ended {
                job: job(),
                state: State::Finished,
                outcome: Some(message),
                failure: Some(JobFailure {
                    cause: "disk-full".to_owned(),
                    item: Some(photo()),
                    file: Some(PathBuf::from("/photos/out/a.jpg")),
                    reason: "No space left on device".to_owned(),
                }),
                at: UNIX_EPOCH + Duration::from_millis(1_795_000_000_123),
                offers: vec![Offer::Retry { items: 2 }],
            },
            command: None,
        },
        Event::Export {
            job: job(),
            status: ExportStatus::Started {
                index: 0,
                source: photo(),
            },
        },
        Event::Export {
            job: job(),
            status: ExportStatus::Done {
                index: 0,
                result: PhotoResult::Written {
                    path: PathBuf::from("/photos/out/a.jpg"),
                    width: 96,
                    height: 72,
                    tiles: 1,
                    newer: false,
                },
            },
        },
        Event::Export {
            job: job(),
            status: ExportStatus::Done {
                index: 1,
                result: PhotoResult::Skipped {
                    path: PathBuf::from("/photos/out/b.jpg"),
                    why: Why::NotChanged,
                },
            },
        },
        Event::Export {
            job: job(),
            status: ExportStatus::Done {
                index: 2,
                result: PhotoResult::Failed(Failure {
                    cause: Cause::DiskFull,
                    file: None,
                    message: "No space left on device".to_owned(),
                }),
            },
        },
        Event::Export {
            job: job(),
            status: ExportStatus::Finished(Summary {
                written: 1,
                skipped: 1,
                failed: 1,
                not_reached: 0,
                cancelled: false,
            }),
        },
        Event::ExportPreview {
            through: 13,
            preview: Box::new(ExportPreview {
                problem: Some(PathProblem::Pattern(PatternError::Unknown {
                    name: "FILE_NAM".to_owned(),
                    at: 2..10,
                    suggestion: Some("FILE_NAME"),
                })),
                sequence_added: true,
                not_here: vec![Unavailable {
                    name: "LABELS",
                    why: NotHere::Labels,
                }],
                names: vec![
                    PhotoName {
                        source: photo(),
                        exact: false,
                        fate: Fate::File {
                            file: PathBuf::from("/photos/out/a.jpg"),
                            exists: true,
                            action: Action::Skip(Why::Exists),
                        },
                    },
                    PhotoName {
                        source: other(),
                        exact: true,
                        fate: Fate::NoName,
                    },
                ],
                size: Some(ActiveSize {
                    source: photo(),
                    size: Ok(OutputSize {
                        extent: [96, 72],
                        width: 48,
                        height: 36,
                        scale: 0.5,
                    }),
                }),
                folders: vec![
                    FolderCheck {
                        folder: PathBuf::from("/photos/out"),
                        preflight: Preflight::Blocked {
                            at: PathBuf::from("/photos"),
                            why: Blocked::Unreadable(io::ErrorKind::PermissionDenied),
                        },
                    },
                    FolderCheck {
                        folder: PathBuf::from("/photos/new/out"),
                        preflight: Preflight::Creates {
                            from: PathBuf::from("/photos/new"),
                        },
                    },
                ],
            }),
        },
        Event::ExportPreview {
            through: 14,
            preview: Box::new(ExportPreview {
                problem: Some(PathProblem::Empty),
                sequence_added: false,
                not_here: Vec::new(),
                names: Vec::new(),
                size: Some(ActiveSize {
                    source: photo(),
                    size: Err(SizeProblem::TooLarge {
                        format: "JPEG",
                        largest: 65535,
                        width: 70000,
                        height: 100,
                    }),
                }),
                folders: Vec::new(),
            }),
        },
        Event::Summary {
            through: 15,
            summary: Arc::new(summary),
        },
        Event::Library(LibraryEvent::Opened {
            open: 4,
            name: "photos".to_owned(),
            folder: folder.clone(),
        }),
        Event::Library(LibraryEvent::Subfolders {
            open: 4,
            names: vec!["2026".to_owned()],
        }),
        Event::Library(LibraryEvent::SubfoldersOf {
            open: 4,
            folder: PathBuf::from("/photos/2026"),
            names: Vec::new(),
            unreadable: Some("permission denied".to_owned()),
        }),
        Event::Library(LibraryEvent::Listed {
            open: 4,
            photos: vec![
                apertrust_library::ops::Photo {
                    name: "a.cr3".to_owned(),
                    size: 1 << 20,
                    modified: Some(UNIX_EPOCH + Duration::from_secs(1_795_000_000)),
                    edit: EditState::Edited,
                    rating,
                    fields: Arc::new(apertrust_library::fields::Fields {
                        values: vec![(
                            "dc:title".to_owned(),
                            FieldValue::LangAlt(vec![("x-default".to_owned(), "Tram".to_owned())]),
                        )],
                        name: "Black and white".to_owned(),
                        copies: 2,
                    }),
                },
                apertrust_library::ops::Photo {
                    name: "b.nef".to_owned(),
                    size: 2 << 20,
                    // A camera with its clock unset: before 1970.
                    modified: Some(UNIX_EPOCH - Duration::from_millis(1_250)),
                    edit: EditState::ViewOnly(Newer::Field(Problem::Field {
                        copy: "0".to_owned(),
                        field: "rating".to_owned(),
                        what: "a number",
                    })),
                    rating: Rating::default(),
                    fields: Default::default(),
                },
                apertrust_library::ops::Photo {
                    name: "c.arw".to_owned(),
                    size: 3,
                    modified: None,
                    edit: EditState::Unreadable("line 2: not XMP".to_owned()),
                    rating: Rating::default(),
                    fields: Default::default(),
                },
                apertrust_library::ops::Photo {
                    name: "d.dng".to_owned(),
                    size: 4,
                    modified: None,
                    edit: EditState::None,
                    rating: Rating::default(),
                    fields: Default::default(),
                },
            ],
            done: false,
        }),
        Event::Library(LibraryEvent::Failed {
            open: 4,
            reason: "permission denied".to_owned(),
        }),
        Event::Library(LibraryEvent::Taken {
            open: 4,
            times: vec![None],
        }),
        Event::Library(LibraryEvent::Closed { open: 5 }),
        Event::Library(LibraryEvent::Thumbnail {
            open: 4,
            photo: "a.cr3".to_owned(),
            size: 256,
            image: Some(thumbnail.clone()),
        }),
        Event::Library(LibraryEvent::PhotoThumbnail {
            open: 4,
            path: photo(),
            size: 256,
            image: None,
        }),
        Event::Library(LibraryEvent::Walked {
            open: 4,
            folders: vec![WalkedFolder {
                folder: PathBuf::from("/photos/2026"),
                raws: 12,
                subfolders: vec!["11".to_owned()],
                unreadable: None,
            }],
            done: true,
        }),
        Event::Cache(CacheEvent::Sizes {
            sizes: Sizes::default(),
            limited: Usage {
                entries: 2,
                bytes: 10,
            },
            limit: Limit::Bytes(1 << 30),
        }),
        Event::Cache(CacheEvent::NoCache),
        Event::History(History {
            through: 9,
            steps: vec![Arc::new(HistoryStep {
                id: STEP,
                clock: Clock {
                    when: 1_795_000_000_123,
                    counter: 2,
                    machine: wire("305419896"),
                },
                source: StepSource::Style,
                note: Some(Note::Message {
                    id: "history-style".to_owned(),
                    args: vec![("name".to_owned(), "Warm".to_owned())],
                }),
                batch: Some(BATCH),
                changes: vec![ToolChange {
                    tool: Cow::Borrowed("exposure"),
                    enabled: Some([false, true]),
                    controls: vec![ControlChange {
                        control: "exposure",
                        before: ControlValue::Number(0.0),
                        after: ControlValue::Number(0.5),
                        mark: [None, Some(Mark::Kept)],
                    }],
                }],
            })],
            end: 1,
            undo: Some(Move::Steps {
                first: STEP,
                count: 1,
                photos: 1,
            }),
            redo: Some(Move::Field {
                change: RatingChange::Stars(4),
                photos: 3,
            }),
        }),
        Event::Selection(Selection {
            photos: vec![photo(), other()],
            active: Some(photo()),
            previous: true,
            through: 16,
        }),
        Event::View(ViewState {
            through: 16,
            view: View {
                scale: ViewScale::Fill,
                centre: [0.5, 0.5],
            },
            placement: Some(Placement {
                shown: 0.25,
                render: 0.5,
                size: [96, 72],
                visible: [0.0, 0.0, 96.0, 72.0],
                centre: [0.5, 0.5],
            }),
        }),
        Event::Batch(BatchEvent::Changed {
            batch: BATCH,
            photo: other(),
            edit: EditState::Edited,
        }),
        Event::Batch(BatchEvent::Ended {
            command: 17,
            batch: BATCH,
            action: BatchAction::TakenBack,
            what: Batched::Rating(RatingChange::Rejected(true)),
            named: 7,
            changed: vec![photo()],
            skipped: vec![
                (other(), Skip::NotOnDisk),
                (other(), Skip::Unreadable("line 1".to_owned())),
                (other(), Skip::ViewOnly(view_only())),
                (
                    other(),
                    Skip::Refused("the tool exposure has no control x".to_owned()),
                ),
                (other(), Skip::KeptChanging),
                (other(), Skip::Failed("permission denied".to_owned())),
                (other(), Skip::ChangedSince),
            ],
            cancelled: true,
        }),
        Event::Batch(BatchEvent::Ended {
            command: 18,
            batch: BATCH,
            action: BatchAction::Applied,
            what: Batched::Style("Warm".to_owned()),
            named: 1,
            changed: Vec::new(),
            skipped: Vec::new(),
            cancelled: false,
        }),
        Event::Rating(RatingState {
            through: 19,
            photo: photo(),
            rating,
        }),
        Event::Rated {
            photo: other(),
            rating,
        },
        Event::Metadata(MetadataState {
            through: 20,
            photo: photo(),
            fields: vec![
                (
                    "dc:title".to_owned(),
                    Some(FieldValue::LangAlt(vec![(
                        "x-default".to_owned(),
                        "Morning market".to_owned(),
                    )])),
                ),
                ("photoshop:City".to_owned(), None),
            ],
            copy: CopyState {
                name: "Black and white".to_owned(),
                number: 2,
                count: 3,
            },
            preserved_file_name: Some("IMG_0001.CR2".to_owned()),
            darktable: false,
        }),
    ];
    events.extend(
        [
            SessionError::Open {
                open: 3,
                error: Box::new(SessionError::NotAMosaic),
            },
            SessionError::Refused {
                command: 12,
                error: Box::new(SessionError::Edit(EditError::UnknownTool(
                    "brightness".to_owned(),
                ))),
            },
            SessionError::Decode(apertrust_raw::Error::Read {
                path: photo(),
                source: io::ErrorKind::NotFound.into(),
            }),
            SessionError::Decode(apertrust_raw::Error::Decode("no decoder".to_owned())),
            SessionError::NotAMosaic,
            SessionError::Sidecar(SidecarError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            ))),
            SessionError::Edit(EditError::Control {
                tool: "exposure",
                source: ControlError::OutOfRange {
                    control: "exposure",
                    value: 20.0,
                    min: -18.0,
                    max: 18.0,
                },
            }),
            SessionError::ViewOnly(ViewOnly {
                build: None,
                reason: Reason::Unreadable(Problem::StepClock {
                    copy: "0".to_owned(),
                    step: "1".to_owned(),
                    error: ClockError::When("yesterday".to_owned()),
                }),
            }),
            SessionError::NotLive,
            SessionError::InStep {
                index: 2,
                error: Box::new(SessionError::NotInStep),
            },
            SessionError::NotInStep,
            SessionError::DefaultSource,
            SessionError::EmptyStep,
            SessionError::UnknownStep { step: STEP },
            SessionError::Tool {
                tool: "exposure",
                source: tool_error,
            },
            SessionError::Save {
                path: PathBuf::from("/photos/a.cr3.apxmp"),
                through: 9,
                source: io::ErrorKind::PermissionDenied.into(),
            },
            SessionError::NotSaved {
                path: PathBuf::from("/photos/a.cr3.apxmp"),
                through: 9,
                error: Box::new(SessionError::StillChanging),
            },
            SessionError::MoveNotKept {
                path: PathBuf::from("/photos/a.cr3.apxmp"),
            },
            SessionError::StillChanging,
            SessionError::Plan(PlanError::NotPorted(NotPorted {
                operation: "demosaic",
                what: "X-Trans",
            })),
            SessionError::Plan(PlanError::Param {
                operation: "exposure",
                source: ParamError::WrongKind {
                    name: "exposure",
                    expected: "a number",
                },
            }),
            SessionError::Compute(ComputeError::Poll(wgpu::PollError::WrongSubmissionIndex(
                1, 2,
            ))),
            SessionError::Compute(ComputeError::Poll(wgpu::PollError::Timeout)),
            SessionError::Compute(ComputeError::Map(wgpu::BufferAsyncError)),
            SessionError::Gpu("out of memory".to_owned()),
            SessionError::Scopes("mapping failed".to_owned()),
            SessionError::Zoom("demosaic refuses the region".to_owned()),
            SessionError::UnknownPicker,
            SessionError::UnknownSetting("target"),
            SessionError::NoPicker,
            SessionError::OneKind {
                takes: PickerKind::PointArea,
            },
            SessionError::Hidden,
            SessionError::NotReady,
            SessionError::Pickers("mapping failed".to_owned()),
            SessionError::NotInCollection { photo: other() },
            SessionError::NoPreviousSelection,
            SessionError::UnknownStyle {
                name: "Warm".to_owned(),
            },
            SessionError::UnknownBatch { batch: BATCH },
            SessionError::NotCancellable { job: job() },
            SessionError::NotRetryable { job: job() },
            SessionError::CacheKept {
                kind: cache::LIBRARIES,
            },
            SessionError::CacheClearing {
                kind: cache::THUMBNAILS,
            },
            SessionError::NoCache,
            SessionError::CacheWalk {
                command: 20,
                source: io::Error::other("the share went away"),
            },
            SessionError::Panicked("index out of bounds".to_owned()),
            SessionError::Stopped,
            SessionError::NoPhoto,
            SessionError::UnknownField {
                field: "dc:subject".to_owned(),
            },
            SessionError::FieldValue {
                field: "photoshop:DateCreated".to_owned(),
                reason: apertrust_library::fields::TypedError::NotADate,
            },
        ]
        .into_iter()
        .map(session_error),
    );
    events
}

/// How many variants [`event_variant`] counts.
const EVENT_VARIANTS: usize = 20;

/// Each event's variant, `None` for a frame. A new variant takes the next index and raises
/// the count.
fn event_variant(event: &Event) -> Option<usize> {
    Some(match event {
        Event::OpenPreview(_) => 0,
        Event::Opened(_) => 1,
        Event::Values(_) => 2,
        Event::Frame(_) => return None,
        Event::Scopes(_) => 3,
        Event::Picker { .. } => 4,
        Event::Saved { .. } => 5,
        Event::Job { .. } => 6,
        Event::Export { .. } => 7,
        Event::ExportPreview { .. } => 8,
        Event::Summary { .. } => 9,
        Event::Library(_) => 10,
        Event::Cache(_) => 11,
        Event::History(_) => 12,
        Event::Selection(_) => 13,
        Event::View(_) => 14,
        Event::Batch(_) => 15,
        Event::Rating(_) => 16,
        Event::Rated { .. } => 17,
        Event::Failed(_) => 18,
        Event::Metadata(_) => 19,
    })
}

const LIBRARY_EVENT_VARIANTS: usize = 10;

/// Each library event's variant; a new one takes the next index and raises the count.
fn library_event_variant(event: &LibraryEvent) -> usize {
    match event {
        LibraryEvent::Opened { .. } => 0,
        LibraryEvent::Subfolders { .. } => 1,
        LibraryEvent::SubfoldersOf { .. } => 2,
        LibraryEvent::Listed { .. } => 3,
        LibraryEvent::Failed { .. } => 4,
        LibraryEvent::Taken { .. } => 5,
        LibraryEvent::Closed { .. } => 6,
        LibraryEvent::Thumbnail { .. } => 7,
        LibraryEvent::PhotoThumbnail { .. } => 8,
        LibraryEvent::Walked { .. } => 9,
    }
}

const JOB_EVENT_VARIANTS: usize = 7;

/// Each job event's variant; a new one takes the next index and raises the count.
fn job_event_variant(event: &JobEvent) -> usize {
    match event {
        JobEvent::Added { .. } => 0,
        JobEvent::Progress { .. } => 1,
        JobEvent::Title { .. } => 2,
        JobEvent::Detail { .. } => 3,
        JobEvent::Destination { .. } => 4,
        JobEvent::State { .. } => 5,
        JobEvent::Ended { .. } => 6,
    }
}

const EXPORT_STATUS_VARIANTS: usize = 3;

/// Each export status's variant; a new one takes the next index and raises the count.
fn export_status_variant(status: &ExportStatus) -> usize {
    match status {
        ExportStatus::Started { .. } => 0,
        ExportStatus::Done { .. } => 1,
        ExportStatus::Finished(_) => 2,
    }
}

const PHOTO_RESULT_VARIANTS: usize = 3;

/// Each exported photo's result's variant; a new one takes the next index and raises the
/// count.
fn photo_result_variant(result: &PhotoResult) -> usize {
    match result {
        PhotoResult::Written { .. } => 0,
        PhotoResult::Skipped { .. } => 1,
        PhotoResult::Failed(_) => 2,
    }
}

const CACHE_EVENT_VARIANTS: usize = 2;

/// Each cache event's variant; a new one takes the next index and raises the count.
fn cache_event_variant(event: &CacheEvent) -> usize {
    match event {
        CacheEvent::Sizes { .. } => 0,
        CacheEvent::NoCache => 1,
    }
}

const BATCH_EVENT_VARIANTS: usize = 2;

/// Each batch event's variant; a new one takes the next index and raises the count.
fn batch_event_variant(event: &BatchEvent) -> usize {
    match event {
        BatchEvent::Changed { .. } => 0,
        BatchEvent::Ended { .. } => 1,
    }
}

/// How many variants [`session_error_variant`] counts.
const SESSION_ERROR_VARIANTS: usize = 45;

/// Each session error's variant; a new one takes the next index and raises the count.
fn session_error_variant(error: &SessionError) -> usize {
    match error {
        SessionError::Open { .. } => 0,
        SessionError::Refused { .. } => 1,
        SessionError::Decode(_) => 2,
        SessionError::NotAMosaic => 3,
        SessionError::Sidecar(_) => 4,
        SessionError::Edit(_) => 5,
        SessionError::ViewOnly(_) => 6,
        SessionError::NotLive => 7,
        SessionError::InStep { .. } => 8,
        SessionError::NotInStep => 9,
        SessionError::DefaultSource => 10,
        SessionError::EmptyStep => 11,
        SessionError::UnknownStep { .. } => 12,
        SessionError::Tool { .. } => 13,
        SessionError::Save { .. } => 14,
        SessionError::NotSaved { .. } => 15,
        SessionError::MoveNotKept { .. } => 16,
        SessionError::StillChanging => 17,
        SessionError::Plan(_) => 18,
        SessionError::Compute(_) => 19,
        SessionError::Gpu(_) => 20,
        SessionError::Scopes(_) => 21,
        SessionError::Zoom(_) => 22,
        SessionError::UnknownPicker => 23,
        SessionError::UnknownSetting(_) => 24,
        SessionError::NoPicker => 25,
        SessionError::OneKind { .. } => 26,
        SessionError::Hidden => 27,
        SessionError::NotReady => 28,
        SessionError::Pickers(_) => 29,
        SessionError::NotInCollection { .. } => 30,
        SessionError::NoPreviousSelection => 31,
        SessionError::UnknownStyle { .. } => 32,
        SessionError::UnknownBatch { .. } => 33,
        SessionError::NotCancellable { .. } => 34,
        SessionError::NotRetryable { .. } => 35,
        SessionError::CacheKept { .. } => 36,
        SessionError::CacheClearing { .. } => 37,
        SessionError::NoCache => 38,
        SessionError::CacheWalk { .. } => 39,
        SessionError::Panicked(_) => 40,
        SessionError::Stopped => 41,
        SessionError::NoPhoto => 42,
        SessionError::UnknownField { .. } => 43,
        SessionError::FieldValue { .. } => 44,
    }
}

/// That `indices` holds every index below `count`, `what`'s variants each with a sample.
fn every_variant(what: &str, indices: impl IntoIterator<Item = usize>, count: usize) {
    let seen: BTreeSet<usize> = indices.into_iter().collect();
    let missing: Vec<usize> = (0..count).filter(|index| !seen.contains(index)).collect();
    assert!(
        missing.is_empty(),
        "{what}: no sample of the variants numbered {missing:?}"
    );
}

/// `value` written, read back and written again: the text both times, and the value read.
fn round_trip<T: Serialize + DeserializeOwned>(value: &T) -> (String, T) {
    let json = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&json).unwrap_or_else(|error| panic!("{json}: {error}"));
    assert_eq!(serde_json::to_string(&read).unwrap(), json);
    (json, read)
}

#[test]
fn every_command_round_trips_through_json() {
    let commands = every_command();
    for command in &commands {
        let (json, read) = round_trip(command);
        assert_eq!(&read, command, "{json}");
    }

    // A step's commands are commands too.
    let all: Vec<&Command> = commands
        .iter()
        .flat_map(|command| match command {
            Command::Step { commands, .. } => commands.iter().chain([command]).collect(),
            _ => vec![command],
        })
        .collect();
    every_variant(
        "Command",
        all.iter().filter_map(|command| command_variant(command)),
        COMMAND_VARIANTS,
    );
    let carried = |pick: fn(&Command) -> Option<usize>| all.iter().filter_map(move |c| pick(c));
    every_variant(
        "PickerCommand",
        carried(|command| match command {
            Command::Picker(picker) => Some(picker_variant(picker)),
            _ => None,
        }),
        PICKER_VARIANTS,
    );
    every_variant(
        "LibraryCommand",
        carried(|command| match command {
            Command::Library(library) => Some(library_command_variant(library)),
            _ => None,
        }),
        LIBRARY_COMMAND_VARIANTS,
    );
    every_variant(
        "SelectionCommand",
        carried(|command| match command {
            Command::Selection(selection) => Some(selection_variant(selection)),
            _ => None,
        }),
        SELECTION_VARIANTS,
    );
    every_variant(
        "CacheCommand",
        carried(|command| match command {
            Command::Cache(cache) => Some(cache_command_variant(cache)),
            _ => None,
        }),
        CACHE_COMMAND_VARIANTS,
    );
}

#[test]
fn every_event_round_trips_through_json() {
    let events = every_event();
    for event in &events {
        // `Event` has no `PartialEq`: the text written again stands for it.
        round_trip(event);
    }

    every_variant(
        "Event",
        events.iter().filter_map(event_variant),
        EVENT_VARIANTS,
    );
    let carried = |pick: fn(&Event) -> Option<usize>| events.iter().filter_map(pick);
    every_variant(
        "LibraryEvent",
        carried(|event| match event {
            Event::Library(library) => Some(library_event_variant(library)),
            _ => None,
        }),
        LIBRARY_EVENT_VARIANTS,
    );
    every_variant(
        "JobEvent",
        carried(|event| match event {
            Event::Job { event, .. } => Some(job_event_variant(event)),
            _ => None,
        }),
        JOB_EVENT_VARIANTS,
    );
    every_variant(
        "ExportStatus",
        carried(|event| match event {
            Event::Export { status, .. } => Some(export_status_variant(status)),
            _ => None,
        }),
        EXPORT_STATUS_VARIANTS,
    );
    every_variant(
        "PhotoResult",
        carried(|event| match event {
            Event::Export {
                status: ExportStatus::Done { result, .. },
                ..
            } => Some(photo_result_variant(result)),
            _ => None,
        }),
        PHOTO_RESULT_VARIANTS,
    );
    every_variant(
        "CacheEvent",
        carried(|event| match event {
            Event::Cache(cache) => Some(cache_event_variant(cache)),
            _ => None,
        }),
        CACHE_EVENT_VARIANTS,
    );
    every_variant(
        "BatchEvent",
        carried(|event| match event {
            Event::Batch(batch) => Some(batch_event_variant(batch)),
            _ => None,
        }),
        BATCH_EVENT_VARIANTS,
    );
    every_variant(
        "SessionError",
        carried(|event| match event {
            Event::Failed(error) => Some(session_error_variant(error)),
            _ => None,
        }),
        SESSION_ERROR_VARIANTS,
    );
}

#[test]
fn the_json_is_tagged_by_type() {
    let json = |command: &Command| serde_json::to_string(command).unwrap();
    assert_eq!(
        json(&Command::Open(photo())),
        r#"{"type":"Open","data":"/photos/a.cr3"}"#
    );
    assert_eq!(json(&Command::Reset), r#"{"type":"Reset"}"#);
    assert_eq!(
        json(&Command::Library(LibraryCommand::Close)),
        r#"{"type":"Library","data":{"type":"Close"}}"#
    );
    assert_eq!(
        json(&Command::SetControl {
            tool: "exposure".to_owned(),
            control: "exposure".to_owned(),
            value: ControlValue::Number(0.5),
            done: true,
        }),
        r#"{"type":"SetControl","data":{"tool":"exposure","control":"exposure","value":{"type":"Number","data":0.5},"done":true}}"#
    );
    // A unit-only enum is its variant's name.
    assert_eq!(serde_json::to_string(&StepSource::You).unwrap(), r#""You""#);
    // A step id is 16 hex digits, never a number a JavaScript client would round.
    assert_eq!(
        json(&Command::GoTo { step: Some(STEP) }),
        r#"{"type":"GoTo","data":{"step":"fedcba9876543210"}}"#
    );
    // No timestamps is null.
    let scopes = every_event()
        .into_iter()
        .find(|event| matches!(event, Event::Scopes(_)))
        .unwrap();
    let scopes: serde_json::Value = serde_json::to_value(&scopes).unwrap();
    assert_eq!(scopes["data"]["gpu_ms"], serde_json::Value::Null);
}

#[test]
fn an_optional_id_left_out_reads_as_none() {
    let command: Command =
        serde_json::from_str(r#"{"type":"Picker","data":{"type":"Channel","data":{}}}"#).unwrap();
    assert_eq!(
        command,
        Command::Picker(PickerCommand::Channel { channel: None })
    );
    let command: Command = serde_json::from_str(
        r#"{"type":"Picker","data":{"type":"On","data":{"picker":{"type":"ColourPicker"},"area":true,"keys":{"command":false,"shift":false},"settings":[]}}}"#,
    )
    .unwrap();
    assert_eq!(
        command,
        Command::Picker(PickerCommand::On {
            picker: PickerRef::ColourPicker,
            area: true,
            keys: PickKeys::default(),
            settings: Vec::new(),
            channel: None,
        })
    );
    let preset: PresetState =
        serde_json::from_str(r#"{"preset":"mode","available":["camera"]}"#).unwrap();
    assert_eq!(preset.matching, None);
    let error: PatternError = serde_json::from_str(
        r#"{"type":"Unknown","data":{"name":"FILE_NAM","at":{"start":2,"end":10}}}"#,
    )
    .unwrap();
    assert!(matches!(
        error,
        PatternError::Unknown {
            suggestion: None,
            ..
        }
    ));
}

#[test]
fn a_number_past_an_f32_is_refused() {
    let read = |json: &str| serde_json::from_str::<Command>(json);
    let set = |value: &str| {
        format!(
            r#"{{"type":"SetControl","data":{{"tool":"exposure","control":"exposure","value":{value},"done":true}}}}"#
        )
    };
    assert!(read(&set(r#"{"type":"Number","data":0.5}"#)).is_ok());
    for json in [
        set(r#"{"type":"Number","data":1e39}"#),
        set(r#"{"type":"Points","data":[[0,0],[1,-1e39]]}"#),
        r#"{"type":"Set","data":{"operation":"rawprepare","param":"x","value":{"type":"Float","data":1e39}}}"#.to_owned(),
        r#"{"type":"View","data":{"scale":{"type":"Factor","data":1e39},"centre":[0.5,0.5]}}"#.to_owned(),
        r#"{"type":"View","data":{"scale":{"type":"Fit"},"centre":[1e39,0.5]}}"#.to_owned(),
        r#"{"type":"Picker","data":{"type":"Point","data":{"at":[0.5,1e39],"done":true,"keys":{"command":false,"shift":false}}}}"#.to_owned(),
    ] {
        let error = read(&json).unwrap_err();
        assert!(
            error.to_string().contains("past the range of an f32"),
            "{json}: {error}"
        );
    }
}
