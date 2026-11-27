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
//! the user edits with it, in the first pass (decision 2): darktable's page in its
//! default layout, each curve's nodes on its graph, its controls in `gui_init`'s order
//! under its labels.
//!
//! - **Hue, Saturation and Brightness:** each tab darktable's graph of its curve's eight
//!   nodes, red to magenta (`Widget::HueCurve`); Node placement under it on every tab.
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
//! The graph is the nodes' only row (decision 31): its curve the operation's, from
//! `apertrust-color`, its colours darktable's. White level's picker is in the definition,
//! as darktable's page has it; the renderer draws its quad once what it runs is built.
//! Left out until the renderer can draw them: Node placement's picker, which only marks a
//! hue on the graph, and the show-mask quads of Saturation threshold and Effect radius,
//! the display's (`docs/tools/README.md`, rule 12).

use std::f32::consts::PI;
use std::sync::OnceLock;

use apertrust_color::colour_equalizer::{
    Gradients, NODES, SLIDER_BRIGHTNESS, lookup_gamut, periodic_rbf, ucs_hue,
};
use apertrust_plugin::{ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    Control, ControlKind, End, Group, GroupKind, Picker, PickerKind, ProcessingDefinition, Reset,
    Row, Slider, Space, Tab, ToolDefinition, Unit, Widget,
};
use crate::values::{ControlValue, Controls};
use crate::{Base, CurvePaint, Owned, ProcessingTool, Setting, ToolValues};

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

/// Each curve's nodes, red to magenta, as its graph shows them.
const HUES: [&str; NODES] = [
    "hue_red",
    "hue_orange",
    "hue_yellow",
    "hue_green",
    "hue_cyan",
    "hue_blue",
    "hue_lavender",
    "hue_magenta",
];
const SATURATIONS: [&str; NODES] = [
    "sat_red",
    "sat_orange",
    "sat_yellow",
    "sat_green",
    "sat_cyan",
    "sat_blue",
    "sat_lavender",
    "sat_magenta",
];
const BRIGHTNESSES: [&str; NODES] = [
    "bright_red",
    "bright_orange",
    "bright_yellow",
    "bright_green",
    "bright_cyan",
    "bright_blue",
    "bright_lavender",
    "bright_magenta",
];

/// One of the three curves, darktable's channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    Hue,
    Saturation,
    Brightness,
}

impl Channel {
    /// The curve whose first node is `control`.
    fn of(control: &str) -> Option<Self> {
        match control {
            "hue_red" => Some(Channel::Hue),
            "sat_red" => Some(Channel::Saturation),
            "bright_red" => Some(Channel::Brightness),
            _ => None,
        }
    }

    fn nodes(self) -> &'static [&'static str; NODES] {
        match self {
            Channel::Hue => &HUES,
            Channel::Saturation => &SATURATIONS,
            Channel::Brightness => &BRIGHTNESSES,
        }
    }
}

/// How many colours the graph's strip and each row of its background hold across, and its
/// background's rows.
const PAINT_COLUMNS: usize = 73;
const PAINT_ROWS: usize = 17;

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

/// darktable's graph of a curve's nodes, `controls`, red to magenta (decision 31): the
/// nodes' only row, as darktable's default layout shows them.
const fn graph(controls: &'static [&'static str]) -> Row {
    Row::Custom {
        widget: Widget::HueCurve,
        controls,
    }
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
                rows: &[graph(&HUES)],
            },
            Tab {
                id: Some("saturation"),
                rows: &[graph(&SATURATIONS)],
            },
            Tab {
                id: Some("brightness"),
                rows: &[graph(&BRIGHTNESSES)],
            },
        ],
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

    /// The curve darktable's graph draws through a tab's nodes (`_iop_colorequalizer_draw`):
    /// its periodic RBF, with Hue curve's smoothing on the Hue tab and clipped at a gain of
    /// 0 on the others, as the operation renders it, at a hue in degrees from the red
    /// node, in the nodes' units.
    fn curve(&self, control: &str, values: &Controls) -> Option<Box<dyn Fn(f32) -> f32>> {
        let channel = Channel::of(control)?;
        let parameter = |control: &str| {
            let p = PARAMETERS.iter().find(|p| p.control == control)?;
            Some(p.param(value_of(values, control)))
        };
        let mut nodes = [0.0; NODES];
        for (node, &control) in nodes.iter_mut().zip(channel.nodes()) {
            *node = parameter(control)?;
        }
        let (table, shown): (Vec<f32>, fn(f32) -> f32) = match channel {
            Channel::Hue => (
                periodic_rbf(
                    &nodes.map(apertrust_color::deg_to_rad),
                    PI / parameter("smoothing_hue")?,
                    0.0,
                    false,
                ),
                f32::to_degrees,
            ),
            Channel::Saturation | Channel::Brightness => {
                (periodic_rbf(&nodes, PI, 0.0, true), |gain| {
                    gain * 100.0 - 100.0
                })
            }
        };
        Some(Box::new(move |degrees| {
            shown(lookup_gamut(&table, ucs_hue(degrees)))
        }))
    }

    /// Node placement, which moves the nodes along the hue, over 360°.
    fn curve_offset(&self, control: &str, values: &Controls) -> Option<f32> {
        Channel::of(control).map(|_| value_of(values, "hue_shift") / 360.0)
    }

    /// darktable's colours for a tab's graph (`_init_graph_backgrounds`, and the gradient
    /// under it): the strip each hue at the graph's saturation; on Hue each hue turned by
    /// the height, as a node there turns it, on Saturation from grey to full, on Brightness
    /// from black to 1.25 times the sliders' brightness. Each column is the strip's hue,
    /// where darktable's runs backwards in a buffer it fills in the wrong channel order.
    fn curve_paint(&self, control: &str, _values: &Controls) -> Option<CurvePaint> {
        static PAINT: OnceLock<[CurvePaint; 3]> = OnceLock::new();
        let channel = Channel::of(control)?;
        let paint = PAINT.get_or_init(|| {
            let gradients = Gradients::srgb();
            [Channel::Hue, Channel::Saturation, Channel::Brightness]
                .map(|channel| paint(&gradients, channel))
        });
        Some(paint[channel as usize].clone())
    }
}

/// A number control's value in `values`: the control's default where they hold none.
fn value_of(values: &Controls, control: &str) -> f32 {
    match values.get(control) {
        Some(ControlValue::Number(value)) => *value,
        _ => match CONTROLS.iter().find(|c| c.id == control).map(|c| c.kind) {
            Some(ControlKind::Number { default, .. }) => default,
            _ => 0.0,
        },
    }
}

/// The strip and background of `channel`'s graph.
fn paint(gradients: &Gradients, channel: Channel) -> CurvePaint {
    let s = gradients.max_saturation;
    let across = |column: usize| 360.0 * column as f32 / (PAINT_COLUMNS - 1) as f32;
    let strip = (0..PAINT_COLUMNS)
        .map(|column| gradients.colour([ucs_hue(across(column)), s, SLIDER_BRIGHTNESS]))
        .collect();
    let background = (0..PAINT_ROWS)
        .flat_map(|row| {
            // The height, 1 at the top, 0 at the foot.
            let y = 1.0 - row as f32 / (PAINT_ROWS - 1) as f32;
            (0..PAINT_COLUMNS).map(move |column| {
                let hue = ucs_hue(across(column));
                let hsb = match channel {
                    Channel::Hue => [hue + (y - 0.5) * 2.0 * PI, s, SLIDER_BRIGHTNESS],
                    Channel::Saturation => [hue, s * y, SLIDER_BRIGHTNESS],
                    Channel::Brightness => [hue, s, 1.25 * SLIDER_BRIGHTNESS * y],
                };
                gradients.colour(hsb)
            })
        })
        .collect();
    CurvePaint {
        strip,
        background,
        columns: PAINT_COLUMNS,
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
        // The nodes have no slider: the graph draws each over its control's range, steps
        // it by its control's step and rounds it to a slider's default decimals.
        let (nodes, sliders) = DARKTABLE.split_at(3 * NODES);
        let differences =
            crate::testing::slider_differences(&ColourEqualizer, &DEFINITION.defaults(), sliders);
        assert!(differences.is_empty(), "{differences:#?}");
        for theirs in nodes {
            let Some(ControlKind::Number { min, max, step, .. }) =
                DEFINITION.control(theirs.id).map(|c| c.kind)
            else {
                panic!("{}: not a number control", theirs.id);
            };
            assert_eq!((min, max), theirs.range(), "{}", theirs.id);
            assert!((step - theirs.step()).abs() <= 1e-6, "{}", theirs.id);
            assert_eq!(
                Slider::new(theirs.id).decimals,
                theirs.digits,
                "{}",
                theirs.id
            );
        }
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

    /// The tool's values with `set`, the rest at their defaults.
    fn values(set: &[(&'static str, f32)]) -> Controls {
        let mut values = Controls::new();
        for (control, value) in set {
            values.set(control, ControlValue::Number(*value));
        }
        values
    }

    #[test]
    fn each_tab_s_curve_is_the_one_the_operation_renders() {
        use apertrust_operations::colorequal::{Curves, LUT_SIZE, Settings};
        let shown = values(&[
            ("hue_green", 30.0),
            ("hue_blue", -45.0),
            ("smoothing_hue", 0.5),
            ("sat_red", 40.0),
            ("sat_cyan", -100.0),
            ("bright_orange", 25.0),
            ("hue_shift", 10.0),
        ]);
        let mut settings = Settings::default();
        settings.hue[3] = 30.0;
        settings.hue[5] = -45.0;
        settings.smoothing_hue = 0.5;
        settings.saturation[0] = 1.4;
        settings.saturation[4] = 0.0;
        settings.brightness[1] = 1.25;
        settings.hue_shift = 10.0;
        let tables = Curves::new(&settings);
        // The operation's table at the UCS hue a conventional hue `degrees` from red has,
        // Node placement turning the nodes from where the graph's curve counts.
        let rendered = |table: &[f32], degrees: f32| {
            lookup_gamut(table, ucs_hue(degrees + settings.hue_shift))
        };
        assert_eq!(tables.hue.len(), LUT_SIZE);
        for degrees in (0..72).map(|k| k as f32 * 5.0) {
            let hue = ColourEqualizer.curve("hue_red", &shown).unwrap()(degrees);
            let expected = rendered(&tables.hue, degrees).to_degrees();
            assert!(
                (hue - expected).abs() < 0.1,
                "hue at {degrees}°: {hue} {expected}"
            );
            let saturation = ColourEqualizer.curve("sat_red", &shown).unwrap()(degrees);
            let expected = rendered(&tables.saturation, degrees) * 100.0 - 100.0;
            assert!(
                (saturation - expected).abs() < 0.1,
                "{degrees}°: {saturation} {expected}"
            );
            let brightness = ColourEqualizer.curve("bright_red", &shown).unwrap()(degrees);
            let expected = rendered(&tables.brightness, degrees) * 100.0 - 100.0;
            assert!(
                (brightness - expected).abs() < 0.1,
                "{degrees}°: {brightness} {expected}"
            );
        }
        // Through the nodes, at their hues, and clipped at no saturation.
        let hue = ColourEqualizer.curve("hue_red", &shown).unwrap();
        assert!((hue(135.0) - 30.0).abs() < 0.5, "{}", hue(135.0));
        let saturation = ColourEqualizer.curve("sat_red", &shown).unwrap();
        assert!((saturation(0.0) - 40.0).abs() < 1.0, "{}", saturation(0.0));
        assert!(saturation(180.0) >= -100.0);
        // Only a curve's first node names its graph.
        assert!(ColourEqualizer.curve("hue_green", &shown).is_none());
        assert_eq!(
            ColourEqualizer.curve_offset("sat_red", &shown),
            Some(10.0 / 360.0)
        );
        assert_eq!(ColourEqualizer.curve_offset("hue_shift", &shown), None);
    }

    #[test]
    fn each_tab_paints_its_strip_and_a_background_of_its_own() {
        let none = Controls::new();
        let hue = ColourEqualizer.curve_paint("hue_red", &none).unwrap();
        let saturation = ColourEqualizer.curve_paint("sat_red", &none).unwrap();
        let brightness = ColourEqualizer.curve_paint("bright_red", &none).unwrap();
        for paint in [&hue, &saturation, &brightness] {
            assert_eq!(paint.strip.len(), PAINT_COLUMNS);
            assert_eq!(paint.columns, PAINT_COLUMNS);
            assert_eq!(paint.background.len(), PAINT_COLUMNS * PAINT_ROWS);
        }
        // The same strip on every tab; at the middle of the Hue tab's background its
        // colours, no hue turned.
        assert_eq!(hue.strip, saturation.strip);
        let middle = PAINT_ROWS / 2 * PAINT_COLUMNS;
        assert_eq!(
            hue.background[middle..middle + PAINT_COLUMNS],
            hue.strip[..]
        );
        // Grey at the foot of the Saturation tab's, black at the Brightness tab's.
        let foot = (PAINT_ROWS - 1) * PAINT_COLUMNS;
        let [r, g, b] = saturation.background[foot + 10];
        assert!((r - g).abs() < 2e-3 && (g - b).abs() < 2e-3, "{r} {g} {b}");
        assert!(brightness.background[foot + 10].iter().all(|&c| c < 1e-3));
    }
}
