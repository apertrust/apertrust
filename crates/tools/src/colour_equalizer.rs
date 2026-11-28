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

//! Colour equalizer (`docs/tools/colour-equalizer.md`): darktable's color equalizer as
//! the user edits with it, in the first pass (decision 2): darktable's page with its
//! sliders shown, its controls in `gui_init`'s order under its labels.
//!
//! - **Hue, Saturation and Brightness:** each tab the eight nodes of its curve, red to
//!   magenta; Node placement under them on every tab.
//! - **Options,** collapsible, under every tab: White level, Hue curve, Use guided filter,
//!   and the guided filter's Hue analysis radius, Saturation threshold, Contrast and
//!   Effect radius, shown only while it is in use.
//!
//! Each control is one `colorequal` parameter, shown as darktable's slider shows it: the
//! saturation and brightness nodes as 100 times the gain less 100, so 1 is 0 %; the
//! saturation threshold as a percentage; the rest as they are. It is switched (decision
//! 32), as darktable leaves color equalizer off on a new photo and no value of its is
//! an identity: off, it renders nothing; any write of a control turns it on, with
//! darktable's defaults for the others, until its reset turns it off.
//!
//! White level's picker is in the definition, as darktable's page has it; the renderer
//! draws its quad once what it runs is built. Left out until the renderer can draw them:
//! darktable's curve graph, a second view of the nodes, with Node placement's picker,
//! which only marks a hue on the graph; the nodes' painted tracks; and the show-mask quads
//! of Saturation threshold and Effect radius, the display's (`docs/tools/README.md`, rule 12).

use apertrust_plugin::{ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    Control, ControlKind, End, Group, GroupKind, Picker, PickerKind, ProcessingDefinition, Reset,
    Row, Slider, Space, Tab, ToolDefinition, Unit,
};
use crate::{Base, Owned, ProcessingTool, Setting, ToolValues};

pub struct ColourEqualizer;

/// The operation it drives.
const COLOR_EQUAL: &str = "colorequal";

/// The guided filter's toggle, whose rows show only while it is on.
pub const USE_FILTER: &str = "use_filter";

/// A number control's parameter, and what the control shows of it: darktable's
/// `parameter × factor + offset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    pub control: &'static str,
    pub param: &'static str,
    pub factor: f32,
    pub offset: f32,
}

impl Parameter {
    /// The parameter for a value the control shows: darktable's
    /// `(value − offset) / factor`, in f32.
    pub fn param(&self, shown: f32) -> f32 {
        (shown - self.offset) / self.factor
    }

    /// What the control shows for a parameter: darktable's `parameter × factor + offset`.
    pub fn shown(&self, param: f32) -> f32 {
        param * self.factor + self.offset
    }
}

/// A parameter shown as it is.
const fn plain(control: &'static str) -> Parameter {
    Parameter {
        control,
        param: control,
        factor: 1.0,
        offset: 0.0,
    }
}

/// A saturation or brightness node's gain, shown as a percentage less 100: darktable's
/// default 1 is 0 %.
const fn gain(control: &'static str) -> Parameter {
    Parameter {
        control,
        param: control,
        factor: 100.0,
        offset: -100.0,
    }
}

/// Every number control's parameter, in the page's order: the nodes tab by tab, then
/// Node placement and the options, White level and Hue curve first. The controls are darktable's parameter
/// names.
pub static PARAMETERS: [Parameter; 31] = [
    plain("hue_red"),
    plain("hue_orange"),
    plain("hue_yellow"),
    plain("hue_green"),
    plain("hue_cyan"),
    plain("hue_blue"),
    plain("hue_lavender"),
    plain("hue_magenta"),
    gain("sat_red"),
    gain("sat_orange"),
    gain("sat_yellow"),
    gain("sat_green"),
    gain("sat_cyan"),
    gain("sat_blue"),
    gain("sat_lavender"),
    gain("sat_magenta"),
    gain("bright_red"),
    gain("bright_orange"),
    gain("bright_yellow"),
    gain("bright_green"),
    gain("bright_cyan"),
    gain("bright_blue"),
    gain("bright_lavender"),
    gain("bright_magenta"),
    plain("hue_shift"),
    plain("white_level"),
    plain("smoothing_hue"),
    plain("chroma_size"),
    Parameter {
        control: "threshold",
        param: "threshold",
        factor: 100.0,
        offset: 0.0,
    },
    plain("contrast"),
    plain("param_size"),
];

/// A number control. `step` is darktable's: what its slider steps by over its soft
/// range, `dt_bauhaus_slider_get_step`, which a key steps by here.
const fn number(
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
        unported: false,
    }
}

/// A hue node, an offset of −180° to 180°, darktable's default 0.
const fn hue(id: &'static str) -> Control {
    number(id, (-180.0, 180.0), 1.0, 0.0, Some(Unit::Degrees))
}

/// A saturation or brightness node, −100 % to +100 % (darktable's gain of 0 to 2),
/// darktable's default 0 % (1).
const fn node_gain(id: &'static str) -> Control {
    number(id, (-100.0, 100.0), 1.0, 0.0, Some(Unit::Percent))
}

/// darktable's hard ranges are the controls', in the units its sliders show; its soft
/// ranges are the rows' (`Slider::soft`).
static CONTROLS: [Control; 32] = [
    hue("hue_red"),
    hue("hue_orange"),
    hue("hue_yellow"),
    hue("hue_green"),
    hue("hue_cyan"),
    hue("hue_blue"),
    hue("hue_lavender"),
    hue("hue_magenta"),
    node_gain("sat_red"),
    node_gain("sat_orange"),
    node_gain("sat_yellow"),
    node_gain("sat_green"),
    node_gain("sat_cyan"),
    node_gain("sat_blue"),
    node_gain("sat_lavender"),
    node_gain("sat_magenta"),
    node_gain("bright_red"),
    node_gain("bright_orange"),
    node_gain("bright_yellow"),
    node_gain("bright_green"),
    node_gain("bright_cyan"),
    node_gain("bright_blue"),
    node_gain("bright_lavender"),
    node_gain("bright_magenta"),
    number("hue_shift", (-23.0, 23.0), 0.1, 0.0, Some(Unit::Degrees)),
    number("white_level", (-2.0, 16.0), 0.01, 1.0, Some(Unit::Ev)),
    number("smoothing_hue", (0.05, 2.0), 0.01, 1.0, None),
    Control {
        id: USE_FILTER,
        kind: ControlKind::Bool { default: true },
        unit: None,
        unported: false,
    },
    number("chroma_size", (1.0, 10.0), 0.1, 1.5, Some(Unit::Pixels)),
    number("threshold", (0.0, 30.0), 0.1, 10.0, Some(Unit::Percent)),
    number("contrast", (-1.0, 1.0), 0.01, 0.0, None),
    number("param_size", (1.0, 128.0), 1.0, 1.0, Some(Unit::Pixels)),
];

/// A slider row as darktable's: two decimals, filled, its track over the whole range.
const fn slider(control: &'static str) -> Row {
    Row::Slider(Slider::new(control))
}

/// A tab's eight node sliders, `<prefix>_red` to `<prefix>_magenta` in darktable's order.
macro_rules! nodes {
    ($prefix:literal) => {
        &[
            slider(concat!($prefix, "_red")),
            slider(concat!($prefix, "_orange")),
            slider(concat!($prefix, "_yellow")),
            slider(concat!($prefix, "_green")),
            slider(concat!($prefix, "_cyan")),
            slider(concat!($prefix, "_blue")),
            slider(concat!($prefix, "_lavender")),
            slider(concat!($prefix, "_magenta")),
        ]
    };
}

/// A slider row showing `decimals` decimals.
const fn decimals(control: &'static str, decimals: u8) -> Row {
    Row::Slider(Slider::new(control).decimals(decimals))
}

/// darktable's picker on White level (`DT_COLOR_PICKER_AREA`): the area's brightest
/// luminance, in EV, its `log2f` of the maximum's Y (`color_picker_apply`).
static PICKERS: [Picker; 1] = [Picker {
    id: "white_level",
    operation: COLOR_EQUAL,
    kind: PickerKind::Area,
    denoise: false,
    output: false,
    space: Space::Operation,
    keep: false,
    writes: &["white_level"],
    settings: &[],
    words_by: None,
}];

static DEFINITION: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: "colorequal",
        tabs: &[
            Tab {
                id: Some("hue"),
                rows: nodes!("hue"),
            },
            Tab {
                id: Some("saturation"),
                rows: nodes!("sat"),
            },
            Tab {
                id: Some("brightness"),
                rows: nodes!("bright"),
            },
        ],
        // darktable's curve graph goes over the tabs, once a widget draws it.
        before: &[],
        after: &[
            decimals("hue_shift", 0),
            Row::Group(Group {
                id: "options",
                kind: GroupKind::Collapsible,
                rows: &[
                    Row::Slider(
                        Slider::new("white_level")
                            .soft(-2.0, 2.0)
                            .end(End::Picker("white_level")),
                    ),
                    // darktable shows it among its options on the Hue tab alone.
                    slider("smoothing_hue"),
                    Row::Toggle {
                        control: USE_FILTER,
                        end: &[],
                    },
                    Row::When {
                        control: USE_FILTER,
                        is: &["true"],
                        rows: &[
                            decimals("chroma_size", 1),
                            decimals("threshold", 1),
                            decimals("contrast", 3),
                            decimals("param_size", 1),
                        ],
                    },
                ],
            }),
        ],
        reset: Reset::Tool,
        presets: true,
        open_at_first: false,
        menus: &[],
        toolbar: &[],
        cursors: &[],
        lives: crate::Lives::ToolsPanel,
    },
    version: 1,
    layers: true,
    auto: false,
    switched: true,
    controls: &CONTROLS,
    preset_rows: &[],
    views: &[],
    pickers: &PICKERS,
    // darktable's `description()` in src/iop/colorequal.c at the reference release.
    facts: Some(Facts {
        purpose: Purpose::CorrectiveAndCreative,
        input: Fact::LinearRgbSceneReferred,
        process: Fact::QuasiLinearRgb,
        output: Fact::QuasiLinearRgbSceneReferred,
    }),
    // src/iop/colorequal.c's flags: `IOP_FLAGS_INCLUDE_IN_STYLES`, not `_UNSAFE_COPY`.
    copy_safe: true,
    in_styles: true,
    publishes: &[],
};

/// Every parameter of `colorequal`: the tool is darktable's and owns them all.
static OWNS: [Owned; 32] = owns();

const fn owns() -> [Owned; 32] {
    let mut owns = [Owned::new(COLOR_EQUAL, USE_FILTER); 32];
    let mut i = 0;
    while i < PARAMETERS.len() {
        owns[i] = Owned::new(COLOR_EQUAL, PARAMETERS[i].param);
        i += 1;
    }
    owns
}

impl ProcessingTool for ColourEqualizer {
    fn definition(&self) -> &'static ProcessingDefinition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default sets its parameter, as darktable's slider does with
    /// a typed value; the guided filter's toggle sets `use_filter`.
    fn bind(&self, values: &ToolValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        let mut settings: Vec<Setting> = PARAMETERS
            .iter()
            .filter_map(|p| {
                let shown = values.number(p.control)?;
                Some(Setting::new(
                    COLOR_EQUAL,
                    p.param,
                    Value::Float(p.param(shown)),
                ))
            })
            .collect();
        if let Some(on) = values.flag(USE_FILTER) {
            settings.push(Setting::new(COLOR_EQUAL, USE_FILTER, Value::Bool(on)));
        }
        settings
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::colorequal::ColorEqual;
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::check;
    use crate::testing::{DarktablePicker, DarktableSlider};
    use crate::values::ControlValue;

    fn bind(set: &[(&str, ControlValue)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ToolValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&ColourEqualizer.defaults(&image));
        ColourEqualizer.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&ColourEqualizer], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&ColourEqualizer, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_each_with_one_control() {
        let schema = ColorEqual.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned.len(), params.len());
        for param in &params {
            assert!(owned.contains(param), "{param}");
        }
        // Each number control's default and ends are its parameter's, shown as its slider
        // shows them, within an f32 step; the binding never writes a default.
        for p in &PARAMETERS {
            let control = DEFINITION.control(p.control).unwrap();
            let ControlKind::Number {
                min, max, default, ..
            } = control.kind
            else {
                panic!("{} is a number", p.control);
            };
            let param = schema.param(p.param).unwrap();
            let Kind::Float {
                min: low,
                max: high,
                default: untouched,
            } = param.kind
            else {
                panic!("{} is a float", p.param);
            };
            for (shown, darktable) in [(min, low), (max, high), (default, untouched)] {
                let bound = p.param(shown);
                assert!(
                    (bound - darktable).abs() <= f32::EPSILON * darktable.abs().max(1.0),
                    "{}: {bound} for {darktable}",
                    p.control
                );
            }
        }
        let Kind::Bool { default } = schema.param(USE_FILTER).unwrap().kind else {
            panic!("the guided filter is a flag");
        };
        assert_eq!(
            DEFINITION.control(USE_FILTER).unwrap().kind,
            ControlKind::Bool { default }
        );
    }

    /// A slider darktable shows as it is, with `digits` decimals.
    const fn shown(id: &'static str, hard: (f32, f32), digits: u8) -> DarktableSlider {
        DarktableSlider::new(id, hard, digits)
    }

    /// A node's gain: `_init_sliders`' "%" format, a factor of 100, its offset of −100
    /// and two digits.
    const fn gain(id: &'static str) -> DarktableSlider {
        DarktableSlider::new(id, (0.0, 2.0), 2).shown_as(100.0, -100.0)
    }

    const HUE: (f32, f32) = (-180.0, 180.0);

    /// darktable's color equalizer sliders (`src/iop/colorequal.c`, the params,
    /// `gui_init` and `_init_sliders`), in its parameters' units:
    /// `docs/tools/colour-equalizer.md`'s slider table.
    const DARKTABLE: [DarktableSlider; 31] = [
        shown("hue_red", HUE, 2),
        shown("hue_orange", HUE, 2),
        shown("hue_yellow", HUE, 2),
        shown("hue_green", HUE, 2),
        shown("hue_cyan", HUE, 2),
        shown("hue_blue", HUE, 2),
        shown("hue_lavender", HUE, 2),
        shown("hue_magenta", HUE, 2),
        gain("sat_red"),
        gain("sat_orange"),
        gain("sat_yellow"),
        gain("sat_green"),
        gain("sat_cyan"),
        gain("sat_blue"),
        gain("sat_lavender"),
        gain("sat_magenta"),
        gain("bright_red"),
        gain("bright_orange"),
        gain("bright_yellow"),
        gain("bright_green"),
        gain("bright_cyan"),
        gain("bright_blue"),
        gain("bright_lavender"),
        gain("bright_magenta"),
        // `set_digits(0)` after the "°" format.
        shown("hue_shift", (-23.0, 23.0), 0),
        shown("white_level", (-2.0, 16.0), 2).soft(-2.0, 2.0),
        shown("smoothing_hue", (0.05, 2.0), 2),
        shown("chroma_size", (1.0, 10.0), 1),
        // `set_digits(3)`, then the "%" format's factor of 100 takes two of them.
        shown("threshold", (0.0, 0.3), 1).shown_as(100.0, 0.0),
        shown("contrast", (-1.0, 1.0), 3),
        shown("param_size", (1.0, 128.0), 1),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences = crate::testing::slider_differences(
            &ColourEqualizer,
            &DEFINITION.defaults(),
            &DARKTABLE,
        );
        assert!(differences.is_empty(), "{differences:#?}");
        // The factor and offset each shows are the ones its binding goes back by.
        for p in &PARAMETERS {
            let darktable = DARKTABLE.iter().find(|s| s.id == p.control).unwrap();
            assert_eq!(
                (darktable.factor, darktable.offset),
                (p.factor, p.offset),
                "{}",
                p.control
            );
        }
    }

    /// darktable's picker on White level (`colorequal.c`, `gui_init` and
    /// `color_picker_apply`), an area writing its value.
    #[test]
    fn its_pickers_are_darktable_s() {
        let darktable = [DarktablePicker {
            id: "white_level",
            slider: "white_level",
            kind: PickerKind::Area,
            denoise: false,
            output: false,
            writes: &["white_level"],
            settings: &[],
        }];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            ColourEqualizer
                .bind(&ToolValues::new(), &image, &base)
                .is_empty()
        );
        let defaults: Vec<(&str, ControlValue)> =
            CONTROLS.iter().map(|c| (c.id, c.default())).collect();
        assert!(bind(&defaults).is_empty());
    }

    #[test]
    fn a_node_s_percentage_sets_its_gain_and_the_rest_bind_as_shown() {
        let number = ControlValue::Number;
        let settings = bind(&[
            ("hue_green", number(-12.5)),
            ("sat_blue", number(25.0)),
            ("bright_red", number(-40.0)),
            ("threshold", number(5.0)),
            ("white_level", number(-1.5)),
            (USE_FILTER, ControlValue::Bool(false)),
        ]);
        let value = |param: &str| {
            settings
                .iter()
                .find(|setting| setting.param == param)
                .map(|setting| setting.value)
        };
        assert_eq!(value("hue_green"), Some(Value::Float(-12.5)));
        assert_eq!(value("sat_blue"), Some(Value::Float(1.25)));
        assert_eq!(value("bright_red"), Some(Value::Float(0.6)));
        assert_eq!(value("threshold"), Some(Value::Float(0.05)));
        assert_eq!(value("white_level"), Some(Value::Float(-1.5)));
        assert_eq!(value(USE_FILTER), Some(Value::Bool(false)));
        assert_eq!(settings.len(), 6);
    }
}
