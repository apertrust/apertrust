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

//! Output colour profile (`docs/modules/output-colour-profile.md`): darktable's
//! output color profile as the user edits with it, in the first pass (decision 2):
//! darktable's page, one control, Export profile, listing darktable's export profiles in
//! the order its `dt_colorspaces_init` numbers them. It has no tabs, groups or presets,
//! and works on the background only.
//!
//! It drives `colorout`, which every photo runs, and owns both its parameters. Its
//! default is the operation's, darktable's sRGB (web-safe), on every photo, so an
//! untouched module sets nothing. darktable's output intent shows only while its
//! preference forcing LittleCMS 2 for export is on, off by default, so it is not shown
//! (rule 12 of `docs/modules/README.md`): the module fixes it at perceptual.
//!
//! The operation ports sRGB, linear Rec709 and linear Rec2020, but the viewport draws
//! colorout's output as sRGB, so an edit can choose only sRGB until the display path
//! converts from the edit's profile (decision 26): the two linear profiles are refused
//! as not ported, as the nine the operation does not port are, and the control, whose
//! only value is its default, is unavailable, its box drawn disabled at sRGB. A disabled
//! box does not open; once the control is available, its list shows every profile still
//! refused drawn disabled.
//! darktable also lists the ICC files in its `color/out` folders, which the operation
//! does not read: the definition holds darktable's built-in profiles alone
//! (`docs/modules/output-colour-profile.md`, Gaps).

use apertrust_color::PROFILE_TYPES;
use apertrust_plugin::{ImageContext, Value};

use crate::definition::{ChoiceStyle, Control, ControlKind, Definition, Row, Tab};
use crate::{Base, Controls, Module, ModuleValues, Owned, Setting};

pub struct OutputColourProfile;

/// The operation it drives.
const COLOROUT: &str = "colorout";

/// Export profile's control, the operation's parameter, darktable's `type`.
const TYPE: &str = "type";

/// darktable's export profiles, in the order `dt_colorspaces_init` gives them their
/// `out_pos`: sRGB (web-safe), Adobe RGB (compatible), linear Rec709 RGB, Rec709 RGB,
/// linear Rec2020 RGB, PQ and HLG Rec2020 RGB, PQ and HLG P3 RGB, Display P3 RGB, linear
/// ProPhoto RGB and BRG (for testing). darktable lists linear XYZ and Lab only while its
/// `allow_lab_output` preference is on, off by default, so they are left out (rule 12).
/// Each id is the operation's choice of that profile, `apertrust_color::PROFILE_TYPES`.
pub const PROFILES: [&str; 12] = [
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

/// The profiles the operation ports but an edit cannot choose yet: the viewport draws
/// colorout's output as sRGB until the display path converts from the edit's profile
/// (decision 26).
pub const HELD_BACK: [&str; 2] = ["lin_rec709", "lin_rec2020"];

/// Every profile an edit cannot choose: [`HELD_BACK`]'s two, and the nine the operation
/// does not port.
const UNPORTED: [&str; 11] = [
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

/// darktable's one control, its `$DEFAULT` sRGB, the operation's.
static CONTROLS: [Control; 1] = [Control {
    id: TYPE,
    kind: ControlKind::Choice {
        options: &PROFILES,
        default: 0,
        unported: &UNPORTED,
    },
    unit: None,
    unported: false,
}];

static DEFINITION: Definition = Definition {
    id: COLOROUT,
    version: 1,
    layers: false,
    auto: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[Row::Choice {
            control: TYPE,
            style: ChoiceStyle::List,
        }],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// Every parameter of `colorout`: the module is darktable's and owns them both, the
/// output intent last, fixed ([`OutputColourProfile::fixed`]).
static OWNS: [Owned; 2] = [Owned::new(COLOROUT, TYPE), Owned::new(COLOROUT, "intent")];

impl Module for OutputColourProfile {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The output intent, which darktable shows only while its preference forcing
    /// LittleCMS 2 for export is on, off by default (rule 12): perceptual, the operation's,
    /// which a matrix profile ignores.
    fn fixed(&self) -> &'static [Owned] {
        &OWNS[1..]
    }

    /// Export profile changes nothing on any photo: sRGB, its default, is the only
    /// profile an edit can choose until the display path converts from the edit's profile
    /// (decision 26). It does not depend on the module's values or the photo.
    fn available(&self, _control: &str, _: &Controls, _image: &ImageContext) -> bool {
        false
    }

    /// The profile, written by the operation's name for it, once off sRGB; the output
    /// intent stays the operation's.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let chosen = values.choice(TYPE).and_then(|i| PROFILES.get(i as usize));
        chosen
            .and_then(|name| PROFILE_TYPES.iter().position(|p| p == name))
            .map(|index| Setting::new(COLOROUT, TYPE, Value::Choice(index as u32)))
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::colorout as operation;
    use apertrust_plugin::{ImageDesc, Kind, Operation, ParamValues};

    use super::*;
    use crate::{ControlError, ControlValue, check};

    /// Every kind of image the module meets: a colour raw and an image that is not a raw.
    fn images() -> [ImageContext; 2] {
        [crate::testing::image(), ImageContext::full(8, 8)]
    }

    fn chosen(name: &str) -> ControlValue {
        let index = PROFILES.iter().position(|&p| p == name).unwrap();
        ControlValue::Choice(index as u32)
    }

    /// The binding for these values, off the defaults, as the session binds them, with
    /// no check that an edit could hold them.
    fn bind(set: &[(&str, ControlValue)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&OutputColourProfile.defaults(image));
        OutputColourProfile.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&OutputColourProfile], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&OutputColourProfile, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    /// darktable's export profiles as `dt_colorspaces_init` lists them with its
    /// preferences at their defaults, by darktable's names, `src/common/colorspaces.c`.
    #[test]
    fn its_options_are_darktable_s_export_profiles_in_its_order() {
        let darktable = [
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
            ("BRG (for testing)", "brg"),
        ];
        let ids: Vec<&str> = darktable.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, PROFILES);
        // Each one of the operation's choices; Lab and linear XYZ, darktable's only under
        // `allow_lab_output`, none of them.
        for id in PROFILES {
            assert!(operation::PROFILES.contains(&id), "{id}");
        }
        assert!(!PROFILES.contains(&"lab") && !PROFILES.contains(&"xyz"));
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        assert_eq!(shown, [TYPE]);
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_and_fixes_the_intent() {
        let schema = operation::Colorout.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        assert_eq!(
            OutputColourProfile.fixed(),
            [Owned::new(COLOROUT, "intent")]
        );
        let Kind::Choice { choices, .. } = schema.param(TYPE).unwrap().kind else {
            panic!("the profile is a choice");
        };
        assert_eq!(choices, PROFILE_TYPES);
    }

    /// darktable's `$DEFAULT`s, the operation's on every photo: sRGB and perceptual.
    #[test]
    fn its_default_is_the_operation_s_srgb() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            let Some(Value::Choice(theirs)) = base.get(COLOROUT, TYPE) else {
                panic!("the profile is a choice");
            };
            let ours = OutputColourProfile.defaults(&image).choice(TYPE);
            assert_eq!(PROFILES[ours as usize], PROFILE_TYPES[theirs as usize]);
            assert_eq!(PROFILES[ours as usize], "srgb");
            assert_eq!(base.get(COLOROUT, "intent"), Some(Value::Choice(0)));
        }
    }

    #[test]
    fn at_its_default_it_sets_nothing() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            let none = OutputColourProfile.bind(&ModuleValues::new(), &image, &base);
            assert!(none.is_empty());
            assert!(bind(&[(TYPE, chosen("srgb"))], &image).is_empty());
        }
    }

    /// Row O2: every profile, the held-back ones included, written as the operation's
    /// choice of that name, so a profile joins the edit's choices by leaving `UNPORTED`.
    #[test]
    fn o2_each_profile_writes_the_operation_s_choice_by_its_name() {
        for image in images() {
            for name in PROFILES {
                let written: Vec<&str> = bind(&[(TYPE, chosen(name))], &image)
                    .iter()
                    .map(|setting| {
                        assert_eq!((setting.operation, setting.param), (COLOROUT, TYPE));
                        match setting.value {
                            Value::Choice(i) => PROFILE_TYPES[i as usize],
                            other => panic!("{other:?}"),
                        }
                    })
                    .collect();
                let expected: &[&str] = if name == "srgb" { &[] } else { &[name] };
                assert_eq!(written, expected);
            }
        }
    }

    /// Row O3: sRGB is the only profile an edit can choose. The two held back the
    /// operation prepares; the nine others it does not port.
    #[test]
    fn o3_every_profile_but_srgb_is_refused() {
        let control = DEFINITION.control(TYPE).unwrap();
        let accepted: Vec<&str> = PROFILES
            .into_iter()
            .filter(|&p| control.check(&chosen(p)).is_ok())
            .collect();
        assert_eq!(accepted, ["srgb"]);
        for option in UNPORTED {
            assert_eq!(
                control.check(&chosen(option)),
                Err(ControlError::Unported {
                    control: TYPE,
                    option,
                })
            );
        }
        let image = ImageContext::full(4, 4);
        let prepares = |name: &str| {
            let mut values = ParamValues::new();
            let index = PROFILE_TYPES.iter().position(|&p| p == name).unwrap();
            values.set(TYPE, Value::Choice(index as u32));
            let params = operation::Colorout.schema().resolve(&values).unwrap();
            operation::Colorout
                .prepare(&params, &image, &ImageDesc::rgba(4, 4))
                .is_ok()
        };
        let ported: Vec<&str> = PROFILES.into_iter().filter(|&p| prepares(p)).collect();
        assert_eq!(ported, ["srgb", "lin_rec709", "lin_rec2020"]);
        let held: Vec<&str> = UNPORTED.into_iter().filter(|&p| prepares(p)).collect();
        assert_eq!(held, HELD_BACK);
    }

    #[test]
    fn it_is_unavailable_on_every_photo() {
        for image in images() {
            let values = OutputColourProfile.defaults(&image);
            assert!(!OutputColourProfile.available(TYPE, &values, &image));
        }
    }
}
