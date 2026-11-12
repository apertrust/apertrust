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

//! Demosaic (`docs/modules/demosaic.md`): darktable's demosaic as the user edits
//! with it, in the first pass (decision 2): darktable's page, its controls in
//! `gui_init`'s order under its labels. The method; then the rows darktable shows only
//! for some methods, as its `gui_changed` does: dual threshold for the dual methods, edge
//! threshold for PPG, LMMSE refine for LMMSE, colour smoothing, match greens for a Bayer
//! sensor's, and capture sharpen; then, while capture sharpen is on, its controls in a
//! collapsible group.
//!
//! It drives `demosaic`, which every raw runs, and owns every parameter of it. Its
//! defaults are the operation's for the photo (`docs/modules/README.md`, rule 4):
//! darktable's `reload_defaults`, RCD for a Bayer sensor and Markesteijn 1-pass for an
//! X-Trans one, VNG4 for four colours and Monochrome for a raw with no colour filters,
//! neither of which the engine renders. Only RCD and Markesteijn 1-pass are ported, and
//! colour smoothing and match greens only while disabled, so every other option is
//! listed, drawn disabled and refused;
//! dual threshold, edge threshold, LMMSE refine and capture sharpening's controls are not
//! ported at all (`Control::unported`), so they take only their defaults, and capture
//! sharpen, off, never shows its group.
//!
//! The method lists the sensor's methods, as darktable's four lists do (decision 30):
//! a Bayer sensor's nine, an X-Trans sensor's seven, a Bayer sensor of four colours'
//! three and a monochrome sensor's one, which, its only option, leaves the method
//! unavailable there. The definition holds all seventeen, so an edit means the same on
//! every photo, and one from another sensor's photo keeps its method, shown last and
//! disabled, rendered as the photo's own, as darktable's `commit_params` falls back.

use apertrust_plugin::{ImageContext, Value};

use crate::definition::{ChoiceStyle, Control, ControlKind, Definition, Row, Slider, Tab, Unit};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct Demosaic;

/// The operation it drives.
const DEMOSAIC: &str = "demosaic";

/// The method's control, the operation's parameter.
const METHOD: &str = "demosaicing_method";

/// darktable's demosaicing methods, in the order its lists give them: a Bayer sensor's
/// nine, an X-Trans sensor's seven, then a monochrome sensor's one; a Bayer4 sensor's
/// are VNG4 and the two Bayer passthroughs. Each id is the operation's choice of that
/// name, X-Trans's passthroughs apart from Bayer's.
pub const METHODS: [&str; 17] = [
    "ppg",
    "amaze",
    "vng4",
    "rcd",
    "lmmse",
    "rcd_dual",
    "amaze_dual",
    "passthrough_monochrome",
    "photosite_color",
    "vng",
    "markesteijn",
    "markesteijn_3",
    "fdc",
    "markesteijn_3_dual",
    "passthrough_monochrome_xtrans",
    "photosite_color_xtrans",
    "monochrome",
];

/// The methods not ported: all but RCD and Markesteijn 1-pass.
const UNPORTED_METHODS: [&str; 15] = [
    "ppg",
    "amaze",
    "vng4",
    "lmmse",
    "rcd_dual",
    "amaze_dual",
    "passthrough_monochrome",
    "photosite_color",
    "vng",
    "markesteijn_3",
    "fdc",
    "markesteijn_3_dual",
    "passthrough_monochrome_xtrans",
    "photosite_color_xtrans",
    "monochrome",
];

/// The operation's choices for the method, in its order, darktable's values'
/// (`apertrust_op_demosaic::METHODS`): the binding writes a method by its name, and a
/// test holds this to the operation's schema.
const OPERATION_METHODS: [&str; 17] = [
    "ppg",
    "amaze",
    "vng4",
    "passthrough_monochrome",
    "photosite_color",
    "rcd",
    "lmmse",
    "monochrome",
    "vng",
    "markesteijn",
    "markesteijn_3",
    "passthrough_monochrome_xtrans",
    "fdc",
    "photosite_color_xtrans",
    "rcd_dual",
    "amaze_dual",
    "markesteijn_3_dual",
];

/// The methods dual threshold shows for: darktable's dual ones (`isdual`).
const DUAL: [&str; 3] = ["rcd_dual", "amaze_dual", "markesteijn_3_dual"];

/// The methods colour smoothing shows for: every method but the passthroughs, the dual
/// ones and a monochrome sensor's.
const SMOOTHED: [&str; 9] = [
    "ppg",
    "amaze",
    "vng4",
    "rcd",
    "lmmse",
    "vng",
    "markesteijn",
    "markesteijn_3",
    "fdc",
];

/// The methods match greens shows for: a Bayer sensor's, but its passthroughs.
const BAYER: [&str; 7] = [
    "ppg",
    "amaze",
    "vng4",
    "rcd",
    "lmmse",
    "rcd_dual",
    "amaze_dual",
];

/// The methods capture sharpen shows for: every method but the passthroughs
/// (`capture_support`).
const SHARPENED: [&str; 13] = [
    "ppg",
    "amaze",
    "vng4",
    "rcd",
    "lmmse",
    "rcd_dual",
    "amaze_dual",
    "vng",
    "markesteijn",
    "markesteijn_3",
    "fdc",
    "markesteijn_3_dual",
    "monochrome",
];

/// Colour smoothing's options, the operation's choices: disabled, then once to five
/// times.
const SMOOTHING: [&str; 6] = ["off", "once", "twice", "three", "four", "five"];

/// Match greens' options, the operation's choices: disabled, local average, full average,
/// full and local average.
const GREENS: [&str; 4] = ["no", "local", "full", "both"];

/// LMMSE refine's options, each the operation's choice at its place, whose `3x_median`,
/// `refine_medians` and `2x_refine_medians` start with a digit, which an id cannot.
const REFINE: [&str; 5] = [
    "basic",
    "median",
    "median_3x",
    "refine_medians",
    "refine_2x_medians",
];

/// A control not ported, taking only its default: a number, with darktable's hard range
/// and the step its slider takes over it, since `gui_init` sets no soft range.
const fn unported(
    id: &'static str,
    (min, max): (f32, f32),
    step: f32,
    default: f32,
    unit: Option<Unit>,
) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min,
            max,
            step,
            default,
        },
        unit,
        unported: true,
    }
}

/// darktable's parameters, in `gui_init`'s order, with their ranges and defaults; contrast
/// sensitivity at the operation's 0.4, where darktable's `reload_defaults` lowers it for
/// sensors above 12 bits and low ISOs, and sharp centre in percent, as darktable's "%"
/// shows it.
static CONTROLS: [Control; 12] = [
    Control {
        id: METHOD,
        // The photo's (`defaults`): RCD, or Markesteijn 1-pass for an X-Trans sensor.
        kind: ControlKind::Choice {
            options: &METHODS,
            default: 3,
            unported: &UNPORTED_METHODS,
        },
        unit: None,
        unported: false,
    },
    unported("dual_thrs", (0.0, 1.0), 0.01, 0.2, None),
    unported("median_thrs", (0.0, 1.0), 0.01, 0.0, None),
    Control {
        id: "lmmse_refine",
        kind: ControlKind::Choice {
            options: &REFINE,
            default: 1,
            unported: &[],
        },
        unit: None,
        unported: true,
    },
    Control {
        id: "color_smoothing",
        kind: ControlKind::Choice {
            options: &SMOOTHING,
            default: 0,
            unported: &["once", "twice", "three", "four", "five"],
        },
        unit: None,
        unported: false,
    },
    Control {
        id: "green_eq",
        kind: ControlKind::Choice {
            options: &GREENS,
            default: 0,
            unported: &["local", "full", "both"],
        },
        unit: None,
        unported: false,
    },
    Control {
        id: "cs_enabled",
        kind: ControlKind::Bool { default: false },
        unit: None,
        unported: true,
    },
    unported("cs_iter", (1.0, 25.0), 1.0, 8.0, None),
    unported("cs_radius", (0.0, 1.5), 0.01, 0.0, Some(Unit::Pixels)),
    unported("cs_thrs", (0.0, 1.0), 0.01, 0.4, None),
    unported("cs_boost", (0.0, 1.5), 0.01, 0.0, Some(Unit::Pixels)),
    unported("cs_center", (0.0, 100.0), 1.0, 0.0, Some(Unit::Percent)),
];

/// darktable's collapsible section of capture sharpening's controls, shown while capture
/// sharpen is on. darktable shows sharp centre only while corner boost is off 0, which a
/// row cannot follow, since rows follow a choice or a toggle: it stays in the group,
/// which never shows while capture sharpen is not ported.
const CAPTURE: Row = Row::Group {
    id: "capture_sharpen_controls",
    rows: &[
        Row::Slider(Slider::new("cs_iter").decimals(0)),
        Row::Slider(Slider::new("cs_radius")),
        Row::Slider(Slider::new("cs_thrs")),
        Row::Slider(Slider::new("cs_boost")),
        Row::Slider(Slider::new("cs_center").decimals(0)),
    ],
};

static DEFINITION: Definition = Definition {
    id: DEMOSAIC,
    version: 1,
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Choice {
                control: METHOD,
                style: ChoiceStyle::List,
            },
            Row::When {
                control: METHOD,
                is: &DUAL,
                rows: &[Row::Slider(Slider::new("dual_thrs"))],
            },
            Row::When {
                control: METHOD,
                is: &["ppg"],
                rows: &[Row::Slider(Slider::new("median_thrs").decimals(3))],
            },
            Row::When {
                control: METHOD,
                is: &["lmmse"],
                rows: &[Row::Choice {
                    control: "lmmse_refine",
                    style: ChoiceStyle::List,
                }],
            },
            Row::When {
                control: METHOD,
                is: &SMOOTHED,
                rows: &[Row::Choice {
                    control: "color_smoothing",
                    style: ChoiceStyle::List,
                }],
            },
            Row::When {
                control: METHOD,
                is: &BAYER,
                rows: &[Row::Choice {
                    control: "green_eq",
                    style: ChoiceStyle::List,
                }],
            },
            Row::When {
                control: METHOD,
                is: &SHARPENED,
                rows: &[
                    Row::Toggle {
                        control: "cs_enabled",
                    },
                    Row::When {
                        control: "cs_enabled",
                        is: &["true"],
                        rows: &[CAPTURE],
                    },
                ],
            },
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
    pickers: &[],
};

/// Every parameter of `demosaic`: the module is darktable's and owns them all.
static OWNS: [Owned; 12] = [
    Owned::new(DEMOSAIC, "green_eq"),
    Owned::new(DEMOSAIC, "median_thrs"),
    Owned::new(DEMOSAIC, "color_smoothing"),
    Owned::new(DEMOSAIC, METHOD),
    Owned::new(DEMOSAIC, "lmmse_refine"),
    Owned::new(DEMOSAIC, "dual_thrs"),
    Owned::new(DEMOSAIC, "cs_radius"),
    Owned::new(DEMOSAIC, "cs_thrs"),
    Owned::new(DEMOSAIC, "cs_boost"),
    Owned::new(DEMOSAIC, "cs_iter"),
    Owned::new(DEMOSAIC, "cs_center"),
    Owned::new(DEMOSAIC, "cs_enabled"),
];

/// The sensors darktable's demosaic tells apart, each with a list of its methods.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sensor {
    /// A Bayer sensor of three colours, and an image that is not a raw, which darktable's
    /// page says demosaic does not apply to.
    Bayer,
    XTrans,
    /// A pattern of four colours, whatever its size, darktable's `DT_IMAGE_4BAYER`.
    Bayer4,
    /// A raw with no colour filters, taken as darktable's monochrome sraw. A linear raw
    /// has none either, and darktable gives it RCD and no demosaic; the image context
    /// cannot yet tell the two apart (`docs/ports/demosaic.md`, Open questions).
    Monochrome,
}

impl Sensor {
    fn of(image: &ImageContext) -> Sensor {
        let Some(camera) = &image.camera else {
            return Sensor::Bayer;
        };
        match &camera.cfa {
            None => Sensor::Monochrome,
            Some(cfa) if (cfa.width, cfa.height) == (6, 6) => Sensor::XTrans,
            Some(cfa) if cfa.values.iter().any(|&c| c > 2) => Sensor::Bayer4,
            Some(_) => Sensor::Bayer,
        }
    }

    /// Its methods, as darktable's combobox for it lists them (`gui_init`).
    fn methods(self) -> &'static [&'static str] {
        match self {
            Sensor::Bayer => &METHODS[..9],
            Sensor::XTrans => &METHODS[9..16],
            Sensor::Bayer4 => &["vng4", "passthrough_monochrome", "photosite_color"],
            Sensor::Monochrome => &["monochrome"],
        }
    }

    /// The method darktable's `reload_defaults` gives it, as the operation's defaults
    /// give it.
    fn default_method(self) -> &'static str {
        match self {
            Sensor::Bayer => "rcd",
            Sensor::XTrans => "markesteijn",
            Sensor::Bayer4 => "vng4",
            Sensor::Monochrome => "monochrome",
        }
    }
}

/// The index of a method among [`METHODS`].
fn method(name: &str) -> u32 {
    METHODS
        .iter()
        .position(|&m| m == name)
        .expect("a method of METHODS") as u32
}

impl Module for Demosaic {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    /// The definition's, with the photo's method.
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        let default = Sensor::of(image).default_method();
        defaults.set(METHOD, ControlValue::Choice(method(default)));
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Nothing on an image that is not a raw, which darktable's page says demosaic does
    /// not apply to. The controls not ported are unavailable everywhere, as the session
    /// reports every such control, and so is the method where the photo lists one only.
    fn available(&self, _: &str, _: &Controls, image: &ImageContext) -> bool {
        image.camera.is_some()
    }

    /// The method lists the sensor's, as darktable's combobox for it does (decision 30);
    /// every other choice, every option.
    fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
        match DEFINITION.control(control).map(|control| control.kind) {
            _ if control == METHOD => Sensor::of(image).methods().to_vec(),
            Some(ControlKind::Choice { options, .. }) => options.to_vec(),
            _ => Vec::new(),
        }
    }

    /// The method, written by the operation's name for it, once off the photo's own. The
    /// other controls take only their defaults, the operation's, so it sets none of them.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let chosen = values.choice(METHOD).and_then(|i| METHODS.get(i as usize));
        chosen
            .and_then(|name| OPERATION_METHODS.iter().position(|m| m == name))
            .map(|index| Setting::new(DEMOSAIC, METHOD, Value::Choice(index as u32)))
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::demosaic as operation;
    use apertrust_plugin::{Camera, Kind, Operation, Pattern};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;

    /// A raw's context with this colour filter pattern, or none.
    fn raw(cfa: Option<Pattern<u32>>) -> ImageContext {
        let image = crate::testing::image();
        ImageContext {
            camera: Some(Camera {
                cfa,
                ..image.camera.clone().unwrap()
            }),
            ..image
        }
    }

    fn bayer() -> ImageContext {
        crate::testing::image()
    }

    fn x_trans() -> ImageContext {
        raw(Some(Pattern {
            width: 6,
            height: 6,
            values: vec![
                1, 1, 0, 1, 1, 2, 1, 1, 2, 1, 1, 0, 2, 0, 1, 0, 2, 1, 1, 1, 2, 1, 1, 0, 1, 1, 0, 1,
                1, 2, 0, 2, 1, 2, 0, 1,
            ],
        }))
    }

    /// A Bayer raw of four colours, darktable's Bayer4.
    fn bayer4() -> ImageContext {
        raw(Some(Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 3, 2],
        }))
    }

    /// Every kind of image the module meets: a Bayer raw, an X-Trans raw, two Bayer4 raws,
    /// the second's four colours in a 2×4 pattern, a raw with no colour filters and an
    /// image that is not a raw.
    fn images() -> [ImageContext; 6] {
        [
            bayer(),
            x_trans(),
            bayer4(),
            raw(Some(Pattern {
                width: 2,
                height: 4,
                values: vec![0, 1, 3, 2, 1, 0, 2, 3],
            })),
            raw(None),
            ImageContext::full(8, 8),
        ]
    }

    fn bind(set: &[(&str, ControlValue)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&Demosaic.defaults(image));
        Demosaic.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    fn chosen(name: &str) -> ControlValue {
        ControlValue::Choice(method(name))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&Demosaic], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&Demosaic, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_in_gui_init_s_order() {
        let shown: Vec<&str> = DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        let order = [
            METHOD,
            "dual_thrs",
            "median_thrs",
            "lmmse_refine",
            "color_smoothing",
            "green_eq",
            "cs_enabled",
            "cs_iter",
            "cs_radius",
            "cs_thrs",
            "cs_boost",
            "cs_center",
        ];
        assert_eq!(shown, order);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, order);
        // Capture sharpening's five in darktable's collapsible section.
        let group = DEFINITION.rows().find_map(|row| match row {
            Row::Group { id, rows } => Some((*id, rows.len())),
            _ => None,
        });
        assert_eq!(group, Some(("capture_sharpen_controls", 5)));
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_with_its_ranges_and_defaults() {
        let schema = operation::Demosaic.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        // Each control is its parameter: the same range, and the same default but for
        // the method's, the photo's, and sharp centre in percent.
        for control in &CONTROLS {
            let param = schema.param(control.id).unwrap();
            match (control.kind, param.kind) {
                (
                    ControlKind::Number {
                        min, max, default, ..
                    },
                    Kind::Float {
                        min: low,
                        max: high,
                        default: theirs,
                    },
                ) => {
                    let factor = if control.id == "cs_center" {
                        100.0
                    } else {
                        1.0
                    };
                    let ours = (min / factor, max / factor, default / factor);
                    assert_eq!(ours, (low, high, theirs), "{}", control.id);
                }
                (
                    ControlKind::Number {
                        min, max, default, ..
                    },
                    Kind::Int {
                        min: low,
                        max: high,
                        default: theirs,
                    },
                ) => assert_eq!(
                    (min, max, default),
                    (low as f32, high as f32, theirs as f32),
                    "{}",
                    control.id
                ),
                (ControlKind::Choice { options, .. }, Kind::Choice { choices, .. }) => {
                    assert_eq!(options.len(), choices.len(), "{}", control.id);
                }
                (ControlKind::Bool { default }, Kind::Bool { default: theirs }) => {
                    assert_eq!(default, theirs, "{}", control.id);
                }
                (ours, theirs) => panic!("{}: {ours:?} against {theirs:?}", control.id),
            }
        }
        // Each option the operation's choice of the same name, or at the same place.
        let choices = |param: &str| match schema.param(param).unwrap().kind {
            Kind::Choice { choices, .. } => choices,
            _ => unreachable!(),
        };
        assert_eq!(choices("color_smoothing"), SMOOTHING);
        assert_eq!(choices("green_eq"), GREENS);
        assert_eq!(choices("lmmse_refine")[..2], REFINE[..2]);
        assert_eq!(choices(METHOD), OPERATION_METHODS);
        let mut ours = METHODS.to_vec();
        ours.sort_unstable();
        let mut theirs = OPERATION_METHODS.to_vec();
        theirs.sort_unstable();
        assert_eq!(ours, theirs);
    }

    #[test]
    fn its_defaults_are_the_operation_s_for_the_photo() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            let defaults = Demosaic.defaults(&image);
            let Some(Value::Choice(theirs)) = base.get(DEMOSAIC, METHOD) else {
                panic!("the method is a choice");
            };
            assert_eq!(
                METHODS[defaults.choice(METHOD) as usize],
                OPERATION_METHODS[theirs as usize]
            );
            for control in &CONTROLS[1..] {
                let theirs = base.get(DEMOSAIC, control.id).unwrap();
                let ours = defaults.get(control.id).unwrap();
                let same = match (ours, theirs) {
                    (ControlValue::Number(v), Value::Float(w)) if control.id == "cs_center" => {
                        *v == w * 100.0
                    }
                    (ControlValue::Number(v), Value::Float(w)) => *v == w,
                    (ControlValue::Number(v), Value::Int(w)) => *v == w as f32,
                    (ControlValue::Choice(v), Value::Choice(w)) => *v == w,
                    (ControlValue::Bool(v), Value::Bool(w)) => *v == w,
                    _ => false,
                };
                assert!(same, "{}: {ours:?} against {theirs:?}", control.id);
            }
        }
        // darktable's `reload_defaults`: RCD for a Bayer sensor, Markesteijn 1-pass for an
        // X-Trans one, by darktable's values.
        let darktable = |image: &ImageContext| {
            let name = METHODS[Demosaic.defaults(image).choice(METHOD) as usize];
            operation::METHODS.iter().find(|m| m.0 == name).unwrap().1
        };
        assert_eq!(darktable(&bayer()), 5);
        assert_eq!(darktable(&x_trans()), 1025);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        for image in images() {
            let base = Base::new(OPERATIONS, &image);
            assert!(
                Demosaic
                    .bind(&ModuleValues::new(), &image, &base)
                    .is_empty()
            );
            let defaults = Demosaic.defaults(&image);
            let written: Vec<(&str, ControlValue)> = CONTROLS
                .iter()
                .map(|c| (c.id, defaults.get(c.id).unwrap().clone()))
                .collect();
            assert!(bind(&written, &image).is_empty());
        }
    }

    #[test]
    fn the_method_writes_the_operation_s_choice_by_its_name() {
        // The other sensor's method, which the operation falls back from, as darktable's
        // `commit_params` does.
        assert_eq!(
            bind(&[(METHOD, chosen("markesteijn"))], &bayer()),
            [Setting::new(DEMOSAIC, METHOD, Value::Choice(9))]
        );
        assert_eq!(
            bind(&[(METHOD, chosen("rcd"))], &x_trans()),
            [Setting::new(DEMOSAIC, METHOD, Value::Choice(5))]
        );
        // Every method by its name, whatever the photo.
        for name in METHODS {
            let settings = bind(&[(METHOD, chosen(name))], &raw(None));
            let written = settings.iter().map(|s| match s.value {
                Value::Choice(i) => OPERATION_METHODS[i as usize],
                _ => unreachable!(),
            });
            let expected: &[&str] = if name == "monochrome" { &[] } else { &[name] };
            assert_eq!(written.collect::<Vec<_>>(), expected);
        }
    }

    #[test]
    fn what_is_not_ported_takes_only_its_default() {
        let unported: Vec<&str> = CONTROLS
            .iter()
            .filter(|control| control.unported)
            .map(|control| control.id)
            .collect();
        assert_eq!(
            unported,
            [
                "dual_thrs",
                "median_thrs",
                "lmmse_refine",
                "cs_enabled",
                "cs_iter",
                "cs_radius",
                "cs_thrs",
                "cs_boost",
                "cs_center",
            ]
        );
        for control in CONTROLS.iter().filter(|control| control.unported) {
            assert_eq!(control.check(&control.default()), Ok(()));
        }
        let cs = DEFINITION.control("cs_enabled").unwrap();
        assert!(cs.check(&ControlValue::Bool(true)).is_err());
        // Of the methods, RCD and Markesteijn 1-pass; colour smoothing and match greens
        // only while disabled.
        let method = DEFINITION.control(METHOD).unwrap();
        let ported: Vec<&str> = METHODS
            .into_iter()
            .filter(|&m| method.check(&chosen(m)).is_ok())
            .collect();
        assert_eq!(ported, ["rcd", "markesteijn"]);
        for (control, options) in [("color_smoothing", 6), ("green_eq", 4)] {
            let control = DEFINITION.control(control).unwrap();
            assert!(control.check(&ControlValue::Choice(0)).is_ok());
            for i in 1..options {
                assert!(control.check(&ControlValue::Choice(i)).is_err());
            }
        }
    }

    #[test]
    fn every_control_can_act_on_a_raw_and_none_on_an_image_that_is_not_one() {
        for image in images() {
            let mut values = Demosaic.defaults(&image);
            for method in ["rcd", "markesteijn"] {
                values.set(METHOD, chosen(method));
                for control in &CONTROLS {
                    let available = Demosaic.available(control.id, &values, &image);
                    assert_eq!(available, image.camera.is_some(), "{}", control.id);
                }
            }
        }
    }

    /// D5's lists (decision 30): each sensor's methods as darktable's combobox for it
    /// lists them, in `gui_init`'s order, holding the photo's default; every method listed
    /// for some sensor; and the method a monochrome sensor's one, so unavailable there.
    #[test]
    fn d5_each_sensor_lists_darktable_s_methods() {
        let lists = |image: &ImageContext| Demosaic.options(METHOD, image);
        let bayer_methods = [
            "ppg",
            "amaze",
            "vng4",
            "rcd",
            "lmmse",
            "rcd_dual",
            "amaze_dual",
            "passthrough_monochrome",
            "photosite_color",
        ];
        assert_eq!(lists(&bayer()), bayer_methods);
        assert_eq!(lists(&ImageContext::full(8, 8)), bayer_methods);
        assert_eq!(
            lists(&x_trans()),
            [
                "vng",
                "markesteijn",
                "markesteijn_3",
                "fdc",
                "markesteijn_3_dual",
                "passthrough_monochrome_xtrans",
                "photosite_color_xtrans",
            ]
        );
        assert_eq!(
            lists(&bayer4()),
            ["vng4", "passthrough_monochrome", "photosite_color"]
        );
        assert_eq!(lists(&raw(None)), ["monochrome"]);
        let method = DEFINITION.control(METHOD).unwrap();
        for image in images() {
            let default = Demosaic.defaults(&image).get(METHOD).unwrap().clone();
            assert_eq!(check::listed(method, &lists(&image), &default), None);
            let alone = check::lists_only(method, &lists(&image), &default);
            assert_eq!(
                alone,
                image.camera.as_ref().is_some_and(|c| c.cfa.is_none())
            );
        }
        for name in METHODS {
            assert!(
                images().iter().any(|image| lists(image).contains(&name)),
                "{name}"
            );
        }
        // Every other choice lists every option.
        for control in ["lmmse_refine", "color_smoothing", "green_eq"] {
            assert_eq!(
                Demosaic.options(control, &x_trans()),
                Demosaic
                    .definition()
                    .control(control)
                    .map_or(Vec::new(), |c| {
                        match c.kind {
                            ControlKind::Choice { options, .. } => options.to_vec(),
                            _ => Vec::new(),
                        }
                    })
            );
        }
        assert!(Demosaic.options("dual_thrs", &bayer()).is_empty());
    }

    /// The controls `rows` show while the module holds `values`, as a panel shows them:
    /// a mode's rows only for its values, a group's whatever it holds.
    fn shown(rows: &[Row], values: &Controls, into: &mut Vec<&'static str>) {
        for row in rows {
            if let Row::When { control, is, rows } = row {
                let control = DEFINITION.control(control).unwrap();
                if control.is_one_of(values.get(control.id).unwrap(), is) {
                    shown(rows, values, into);
                }
                continue;
            }
            into.extend(row.controls().iter().copied());
            shown(row.rows(), values, into);
        }
    }

    #[test]
    fn d5_its_rows_follow_the_method_as_darktable_s_gui_changed_shows_them() {
        // darktable's `gui_changed` for the method's sensor: `bayer`, `xtrans` and the
        // true monochrome follow from the method; a Bayer4 sensor's VNG4 reads as a
        // Bayer one's, which Apertrust's decoder does not open.
        let bayer_methods = &METHODS[..9];
        for (i, &name) in METHODS.iter().enumerate() {
            let mono = name == "monochrome";
            let xtrans = (9..16).contains(&i);
            let passing = name.contains("passthrough") || name.contains("photosite");
            let dual = name.ends_with("_dual");
            let expected: Vec<&str> = [
                (METHOD, true),
                ("dual_thrs", dual),
                ("median_thrs", name == "ppg"),
                ("lmmse_refine", name == "lmmse"),
                ("color_smoothing", !passing && !dual && !mono),
                (
                    "green_eq",
                    !passing && !xtrans && !mono && bayer_methods.contains(&name),
                ),
                ("cs_enabled", !passing),
            ]
            .into_iter()
            .filter_map(|(control, shows)| shows.then_some(control))
            .collect();
            let mut values = Demosaic.defaults(&bayer());
            values.set(METHOD, chosen(name));
            let mut got = Vec::new();
            shown(DEFINITION.tabs[0].rows, &values, &mut got);
            assert_eq!(got, expected, "{name}");
            // Capture sharpen on shows its controls, but where the method has none.
            values.set("cs_enabled", ControlValue::Bool(true));
            let mut on = Vec::new();
            shown(DEFINITION.tabs[0].rows, &values, &mut on);
            let group = ["cs_iter", "cs_radius", "cs_thrs", "cs_boost", "cs_center"];
            let more = if passing { 0 } else { group.len() };
            assert_eq!(on.len(), got.len() + more, "{name}");
        }
    }

    /// darktable's demosaic sliders (`src/iop/demosaic.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/modules/demosaic.md`'s slider table.
    /// None has a soft range; sharp centre's "%" multiplies by 100, its range within ±10.
    const DARKTABLE: [DarktableSlider; 7] = [
        DarktableSlider::new("dual_thrs", (0.0, 1.0), 2),
        DarktableSlider::new("median_thrs", (0.0, 1.0), 3),
        DarktableSlider::whole("cs_iter", (1.0, 25.0)),
        DarktableSlider::new("cs_radius", (0.0, 1.5), 2),
        DarktableSlider::new("cs_thrs", (0.0, 1.0), 2),
        DarktableSlider::new("cs_boost", (0.0, 1.5), 2),
        DarktableSlider::new("cs_center", (0.0, 1.0), 0).shown_as(100.0, 0.0),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&Demosaic, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
    }
}
