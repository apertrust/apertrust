// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/colorin.c)
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

//! Input colour profile (`docs/tools/input-colour-profile.md`): darktable's input color
//! profile as the user edits with it, in the first pass (decision 2): darktable's page,
//! three choices in its `gui_init`'s order, Input profile, Working profile and Gamut
//! clipping, each listing darktable's options in its order. It has no tabs, groups or
//! presets, and works on the background only.
//!
//! It drives `colorin`, which every photo runs, and owns all its parameters. Its defaults
//! are the operation's for the photo, darktable's `reload_defaults`: a DNG's embedded
//! matrix where the image context has one, the standard matrix on another colour raw,
//! linear Rec2020 otherwise; linear Rec2020 to work in, and no gamut clipping. So an
//! untouched tool sets nothing. darktable shows no widget for the intent or blue mapping,
//! so the tool fixes them at the operation's, perceptual and off (rule 12 of
//! `docs/tools/README.md`).
//!
//! Input profile lists the photo's own matrix first, as darktable's
//! `update_profile_list` does: the embedded matrix on a DNG whose image context holds its
//! own, the standard matrix on another raw with one; then darktable's built-in input
//! profiles. Of those the operation ports linear Rec709 and linear Rec2020; the rest are
//! refused as not ported and drawn disabled. Working profile ports one option, its
//! default, so it is unavailable, its box drawn disabled at it. Gamut clipping ports all
//! five of darktable's options.
//! darktable also lists matrices of its own camera tables, a file's embedded ICC profile
//! and the ICC files in its `color/in` and `color/out` folders, which Apertrust does not
//! have: the definition leaves them out (`docs/tools/input-colour-profile.md`, Gaps).

use apertrust_color::PROFILE_TYPES;
use apertrust_plugin::{ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    ChoiceStyle, Control, ControlKind, ProcessingDefinition, Reset, Row, Tab, ToolDefinition,
};
use crate::{Base, Controls, Owned, ProcessingTool, Setting, ToolValues};

pub struct InputColourProfile;

/// The operation it drives.
const COLORIN: &str = "colorin";

/// Input profile's control, the operation's parameter, darktable's `type`.
const TYPE: &str = "type";

/// Working profile's control, darktable's `type_work`.
const TYPE_WORK: &str = "type_work";

/// Gamut clipping's control, darktable's `normalize`.
const NORMALIZE: &str = "normalize";

/// darktable's input profiles, in its list's order: the photo's own matrix, the embedded
/// matrix of a DNG or the standard matrix from the camera table, as `update_profile_list`
/// adds them; then the built-in profiles in the order `dt_colorspaces_init` gives them
/// their `in_pos`: sRGB, Adobe RGB (compatible), linear Rec709 RGB, Rec709 RGB, linear
/// Rec2020 RGB, PQ and HLG Rec2020 RGB, PQ and HLG P3 RGB, Display P3 RGB, linear ProPhoto
/// RGB, linear XYZ, Lab, linear infrared BGR and BRG (for testing). Each id is the
/// operation's choice of that profile, `apertrust_color::PROFILE_TYPES`.
pub const INPUT_PROFILES: [&str; 17] = [
    "embedded_matrix",
    "standard_matrix",
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
    "xyz",
    "lab",
    "infrared",
    "brg",
];

/// The input profiles the operation does not port: every built-in one but linear Rec709
/// and linear Rec2020.
const INPUT_UNPORTED: [&str; 13] = [
    "srgb",
    "adobe_rgb",
    "rec709",
    "pq_rec2020",
    "hlg_rec2020",
    "pq_p3",
    "hlg_p3",
    "display_p3",
    "prophoto_rgb",
    "xyz",
    "lab",
    "infrared",
    "brg",
];

/// darktable's working profiles, in the order `dt_colorspaces_init` gives them their
/// `work_pos`: sRGB (web-safe), Adobe RGB (compatible), linear Rec709 RGB, Rec709 RGB,
/// linear Rec2020 RGB, PQ and HLG Rec2020 RGB, PQ and HLG P3 RGB, Display P3 RGB and
/// linear ProPhoto RGB, by the operation's names.
pub const WORKING_PROFILES: [&str; 11] = [
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
];

/// Every working profile but linear Rec2020, the working space the operations after it
/// assume.
const WORKING_UNPORTED: [&str; 10] = [
    "srgb",
    "adobe_rgb",
    "lin_rec709",
    "rec709",
    "pq_rec2020",
    "hlg_rec2020",
    "pq_p3",
    "hlg_p3",
    "display_p3",
    "prophoto_rgb",
];

/// darktable's gamut clipping, `dt_iop_color_normalize_t`'s `$DESCRIPTION`s: off, sRGB,
/// Adobe RGB (compatible), linear Rec709 RGB and linear Rec2020 RGB, the operation's
/// choices.
pub const GAMUT_CLIPPING: [&str; 5] = ["off", "srgb", "adobe_rgb", "lin_rec709", "lin_rec2020"];

/// darktable's three controls. Input profile's default is the photo's
/// ([`InputColourProfile::defaults`]); the definition's is the standard matrix, a raw's.
static CONTROLS: [Control; 3] = [
    Control {
        id: TYPE,
        kind: ControlKind::Choice {
            options: &INPUT_PROFILES,
            default: 1,
            unported: &INPUT_UNPORTED,
        },
        unit: None,
        unported: false,
    },
    Control {
        id: TYPE_WORK,
        kind: ControlKind::Choice {
            options: &WORKING_PROFILES,
            default: 4,
            unported: &WORKING_UNPORTED,
        },
        unit: None,
        unported: false,
    },
    Control {
        id: NORMALIZE,
        kind: ControlKind::Choice {
            options: &GAMUT_CLIPPING,
            default: 0,
            unported: &[],
        },
        unit: None,
        unported: false,
    },
];

static DEFINITION: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: COLORIN,
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Choice {
                    control: TYPE,
                    style: ChoiceStyle::List,
                },
                Row::Choice {
                    control: TYPE_WORK,
                    style: ChoiceStyle::List,
                },
                Row::Choice {
                    control: NORMALIZE,
                    style: ChoiceStyle::List,
                },
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::Tool,
        presets: true,
        open_at_first: false,
        menus: &[],
        cursors: &[],
    },
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    preset_rows: &[],
    views: &[],
    pickers: &[],
    // darktable's `description()` in src/iop/colorin.c at the reference release.
    facts: Some(Facts {
        purpose: Purpose::Mandatory,
        input: Fact::LinearOrNonLinearRgbSceneReferred,
        process: Fact::DefinedByProfile,
        output: Fact::LinearRgbSceneReferred,
    }),
};

/// Every parameter of `colorin`, in its schema's order: the tool is darktable's and owns
/// them all, the intent and blue mapping fixed ([`InputColourProfile::fixed`]).
static OWNS: [Owned; 5] = [
    Owned::new(COLORIN, TYPE),
    Owned::new(COLORIN, "intent"),
    Owned::new(COLORIN, NORMALIZE),
    Owned::new(COLORIN, "blue_mapping"),
    Owned::new(COLORIN, TYPE_WORK),
];

/// The parameters darktable shows no widget for (rule 12).
static FIXED: [Owned; 2] = [
    Owned::new(COLORIN, "intent"),
    Owned::new(COLORIN, "blue_mapping"),
];

/// The photo's own matrix, as darktable's `reload_defaults` chooses its input profile
/// and colorin's defaults restate it: a DNG's embedded matrix where the image context
/// holds one, a linear DNG's among them, the standard matrix on another colour raw;
/// `None` for an image that is neither, whose default is linear Rec2020.
fn camera_matrix(image: &ImageContext) -> Option<&'static str> {
    let camera = image.camera.as_ref()?;
    if camera.embedded_matrix {
        return Some("embedded_matrix");
    }
    camera.cfa.as_ref()?;
    Some("standard_matrix")
}

fn index(options: &[&str], name: &str) -> u32 {
    options
        .iter()
        .position(|&o| o == name)
        .expect("an option of the list") as u32
}

impl ProcessingTool for InputColourProfile {
    fn definition(&self) -> &'static ProcessingDefinition {
        &DEFINITION
    }

    /// colorin's defaults for the photo: the photo's own matrix on a colour raw, linear
    /// Rec2020 otherwise. This crate cannot call colorin, so its rule is repeated here; a
    /// test holds the two to agree.
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        let profile = camera_matrix(image).unwrap_or("lin_rec2020");
        defaults.set(
            TYPE,
            crate::ControlValue::Choice(index(&INPUT_PROFILES, profile)),
        );
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The intent and blue mapping, which darktable shows no widget for (rule 12):
    /// perceptual, which a matrix profile ignores, and off.
    fn fixed(&self) -> &'static [Owned] {
        &FIXED
    }

    /// Working profile changes nothing on any photo: it ports one option, its default.
    /// Input profile always lists two linear profiles beside the photo's own, and Gamut
    /// clipping ports every option.
    fn available(&self, control: &str, _: &Controls, _image: &ImageContext) -> bool {
        control != TYPE_WORK
    }

    /// Input profile lists the photo's own matrix, then darktable's built-in profiles, as
    /// `update_profile_list` does: the embedded matrix where the image context holds a
    /// DNG's, the standard matrix on another raw with a matrix, or with colour filters,
    /// where it is the photo's default (decision 30). The other choices list every
    /// option on every photo.
    fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
        match control {
            TYPE => {
                let own = image.camera.as_ref().and_then(|camera| {
                    if camera.embedded_matrix {
                        Some("embedded_matrix")
                    } else if camera.xyz_to_camera.is_some() || camera.cfa.is_some() {
                        Some("standard_matrix")
                    } else {
                        None
                    }
                });
                own.into_iter()
                    .chain(INPUT_PROFILES[2..].iter().copied())
                    .collect()
            }
            TYPE_WORK => WORKING_PROFILES.to_vec(),
            NORMALIZE => GAMUT_CLIPPING.to_vec(),
            _ => Vec::new(),
        }
    }

    /// Each choice off the photo's default, written as the operation's choice of that
    /// name; the intent and blue mapping stay the operation's.
    fn bind(&self, values: &ToolValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let profile = |control: &'static str, options: &[&'static str], choices: &[&str]| {
            let name = values
                .choice(control)
                .and_then(|i| options.get(i as usize))?;
            let index = choices.iter().position(|c| c == name)?;
            Some(Setting::new(COLORIN, control, Value::Choice(index as u32)))
        };
        [
            profile(TYPE, &INPUT_PROFILES, PROFILE_TYPES),
            profile(NORMALIZE, &GAMUT_CLIPPING, &GAMUT_CLIPPING),
            profile(TYPE_WORK, &WORKING_PROFILES, PROFILE_TYPES),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::colorin as operation;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::{ControlError, ControlValue, check};

    /// Every kind of image the tool meets: a colour raw, a DNG with its own matrix, a
    /// linear DNG with its own, a raw with no colour filters and an image that is not a
    /// raw.
    fn images() -> [ImageContext; 5] {
        let raw = crate::testing::image();
        let mut dng = raw.clone();
        dng.camera.as_mut().unwrap().embedded_matrix = true;
        let mut linear_dng = dng.clone();
        linear_dng.camera.as_mut().unwrap().cfa = None;
        let mut monochrome = raw.clone();
        let camera = monochrome.camera.as_mut().unwrap();
        camera.cfa = None;
        camera.xyz_to_camera = None;
        [raw, dng, linear_dng, monochrome, ImageContext::full(8, 8)]
    }

    fn chosen(options: &[&str], name: &str) -> ControlValue {
        ControlValue::Choice(index(options, name))
    }

    /// The binding for these values, off the defaults, as the session binds them, with
    /// no check that an edit could hold them.
    fn bind(set: &[(&str, ControlValue)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ToolValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&InputColourProfile.defaults(image));
        InputColourProfile.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&InputColourProfile], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&InputColourProfile, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    /// darktable's lists with its preferences at their defaults, by darktable's names:
    /// the input profiles as `update_profile_list` builds them for a raw, less the
    /// matrices of darktable's own camera tables, the working profiles and gamut
    /// clipping; `src/iop/colorin.c` and `src/common/colorspaces.c`.
    #[test]
    fn its_options_are_darktable_s_in_its_order() {
        let input = [
            ("embedded matrix", "embedded_matrix"),
            ("standard color matrix", "standard_matrix"),
            ("sRGB", "srgb"),
            ("Adobe RGB (compatible)", "adobe_rgb"),
            ("linear Rec709 RGB", "lin_rec709"),
            ("Rec709 RGB", "rec709"),
            ("linear Rec2020 RGB", "lin_rec2020"),
            ("PQ Rec2020 RGB", "pq_rec2020"),
            ("HLG Rec2020 RGB", "hlg_rec2020"),
            ("PQ P3 RGB", "pq_p3"),
            ("HLG P3 RGB", "hlg_p3"),
            ("Display P3 RGB", "display_p3"),
            ("linear ProPhoto RGB", "prophoto_rgb"),
            ("linear XYZ", "xyz"),
            ("Lab", "lab"),
            ("linear infrared BGR", "infrared"),
            ("BRG (for testing)", "brg"),
        ];
        let ids: Vec<&str> = input.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, INPUT_PROFILES);
        let working = [
            ("sRGB (web-safe)", "srgb"),
            ("Adobe RGB (compatible)", "adobe_rgb"),
            ("linear Rec709 RGB", "lin_rec709"),
            ("Rec709 RGB", "rec709"),
            ("linear Rec2020 RGB", "lin_rec2020"),
            ("PQ Rec2020 RGB", "pq_rec2020"),
            ("HLG Rec2020 RGB", "hlg_rec2020"),
            ("PQ P3 RGB", "pq_p3"),
            ("HLG P3 RGB", "hlg_p3"),
            ("Display P3 RGB", "display_p3"),
            ("linear ProPhoto RGB", "prophoto_rgb"),
        ];
        let ids: Vec<&str> = working.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, WORKING_PROFILES);
        // Each one of the operation's choices.
        for id in INPUT_PROFILES.iter().chain(&WORKING_PROFILES) {
            assert!(operation::PROFILES.contains(id), "{id}");
        }
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(shown, [TYPE, TYPE_WORK, NORMALIZE]);
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_and_fixes_two() {
        let schema = operation::Colorin.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        assert_eq!(InputColourProfile.fixed(), FIXED);
        let Kind::Choice { choices, .. } = schema.param(NORMALIZE).unwrap().kind else {
            panic!("gamut clipping is a choice");
        };
        assert_eq!(choices, GAMUT_CLIPPING);
        for param in [TYPE, TYPE_WORK] {
            let Kind::Choice { choices, .. } = schema.param(param).unwrap().kind else {
                panic!("{param} is a choice");
            };
            assert_eq!(choices, PROFILE_TYPES);
        }
    }

    /// Row I5: the defaults are colorin's for each photo, its `reload_defaults`, with the
    /// intent and blue mapping the operation's.
    #[test]
    fn i5_its_defaults_are_the_operation_s_for_the_photo() {
        let expected = [
            "standard_matrix",
            "embedded_matrix",
            "embedded_matrix",
            "lin_rec2020",
            "lin_rec2020",
        ];
        for (image, expected) in images().iter().zip(expected) {
            let base = Base::new(OPERATIONS, image);
            let ours = InputColourProfile.defaults(image);
            for (control, options) in [
                (TYPE, &INPUT_PROFILES[..]),
                (TYPE_WORK, &WORKING_PROFILES[..]),
                (NORMALIZE, &GAMUT_CLIPPING[..]),
            ] {
                let Some(Value::Choice(theirs)) = base.get(COLORIN, control) else {
                    panic!("{control} is a choice");
                };
                let choices: &[&str] = if control == NORMALIZE {
                    &GAMUT_CLIPPING
                } else {
                    PROFILE_TYPES
                };
                assert_eq!(
                    options[ours.choice(control) as usize],
                    choices[theirs as usize],
                    "{control}"
                );
            }
            assert_eq!(INPUT_PROFILES[ours.choice(TYPE) as usize], expected);
            assert_eq!(base.get(COLORIN, "intent"), Some(Value::Choice(0)));
            assert_eq!(base.get(COLORIN, "blue_mapping"), Some(Value::Bool(false)));
        }
    }

    /// Row I0, on the CPU: at its defaults it sets nothing, written or not.
    #[test]
    fn i0_at_its_defaults_it_sets_nothing() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            assert!(
                InputColourProfile
                    .bind(&ToolValues::new(), &image, &base)
                    .is_empty()
            );
            let defaults = InputColourProfile.defaults(&image);
            let written: Vec<(&str, ControlValue)> = [TYPE, TYPE_WORK, NORMALIZE]
                .into_iter()
                .map(|control| (control, defaults.get(control).cloned().unwrap()))
                .collect();
            assert!(bind(&written, &image).is_empty());
        }
    }

    /// Row I2: every option of every choice, the unported ones included, written as the
    /// operation's choice of its name, and nothing at the photo's default.
    #[test]
    fn i2_each_option_writes_the_operation_s_choice_by_its_name() {
        for image in images() {
            let defaults = InputColourProfile.defaults(&image);
            for (control, options, choices) in [
                (TYPE, &INPUT_PROFILES[..], PROFILE_TYPES),
                (TYPE_WORK, &WORKING_PROFILES[..], PROFILE_TYPES),
                (NORMALIZE, &GAMUT_CLIPPING[..], &GAMUT_CLIPPING[..]),
            ] {
                for &name in options {
                    let written: Vec<&str> = bind(&[(control, chosen(options, name))], &image)
                        .iter()
                        .map(|setting| {
                            assert_eq!((setting.operation, setting.param), (COLORIN, control));
                            match setting.value {
                                Value::Choice(i) => choices[i as usize],
                                other => panic!("{other:?}"),
                            }
                        })
                        .collect();
                    let default = options[defaults.choice(control) as usize];
                    let expected: &[&str] = if name == default { &[] } else { &[name] };
                    assert_eq!(written, expected, "{control} {name}");
                }
            }
        }
    }

    /// Row I3: an edit can choose the camera's matrix and the two linear profiles as
    /// input, linear Rec2020 to work in and any gamut clipping; every other option is
    /// refused as not ported.
    #[test]
    fn i3_every_option_not_ported_is_refused() {
        for (control, options, accepted) in [
            (
                TYPE,
                &INPUT_PROFILES[..],
                &[
                    "embedded_matrix",
                    "standard_matrix",
                    "lin_rec709",
                    "lin_rec2020",
                ][..],
            ),
            (TYPE_WORK, &WORKING_PROFILES[..], &["lin_rec2020"][..]),
            (NORMALIZE, &GAMUT_CLIPPING[..], &GAMUT_CLIPPING[..]),
        ] {
            let found = DEFINITION.control(control).unwrap();
            let ok: Vec<&str> = options
                .iter()
                .copied()
                .filter(|&o| found.check(&chosen(options, o)).is_ok())
                .collect();
            assert_eq!(ok, accepted, "{control}");
            for &option in options.iter().filter(|o| !accepted.contains(o)) {
                assert_eq!(
                    found.check(&chosen(options, option)),
                    Err(ControlError::Unported { control, option })
                );
            }
        }
    }

    /// Row I4, on the CPU: Input profile lists the photo's own matrix, then darktable's
    /// built-in profiles.
    #[test]
    fn i4_input_profile_lists_the_photo_s_own_matrix_first() {
        let [raw, dng, linear_dng, monochrome, not_raw] = images();
        let builtin = &INPUT_PROFILES[2..];
        let listed = |image: &ImageContext| InputColourProfile.options(TYPE, image);
        assert_eq!(listed(&raw)[0], "standard_matrix");
        assert_eq!(&listed(&raw)[1..], builtin);
        assert_eq!(listed(&dng)[0], "embedded_matrix");
        assert_eq!(&listed(&dng)[1..], builtin);
        assert_eq!(listed(&linear_dng)[0], "embedded_matrix");
        assert_eq!(listed(&monochrome), builtin);
        assert_eq!(listed(&not_raw), builtin);
        for image in images() {
            for control in [TYPE, NORMALIZE] {
                assert!(InputColourProfile.available(control, &Controls::new(), &image));
            }
            assert!(!InputColourProfile.available(TYPE_WORK, &Controls::new(), &image));
        }
    }
}
