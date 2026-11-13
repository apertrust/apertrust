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

//! A module's definition (decision 28, `docs/gui.md`, Module panels from definitions):
//! static data the app draws a panel from and MCP describes. Controls, what an edit
//! stores, are kept apart from rows, how a tab shows them, so validation, edits and MCP
//! read the controls without walking the panel.
//!
//! The rows are darktable's page as darktable has it (decision 29): its tabs,
//! its collapsible sections as [`Row::Group`]s, the rows it shows only in some modes as
//! [`Row::When`], the rows it draws outside its notebook in [`Definition::before`] and
//! [`Definition::after`], every option of a choice, those not ported marked, its own
//! settings for a tool, which no edit stores, as [`Row::ToolSetting`], and its buttons,
//! actions no edit stores either, as [`Row::Action`]. Which rows show
//! and which options can be chosen is the panel's, from the values, so the edit and the
//! render never depend on it.
//!
//! Two kinds of row store nothing (decision 2): a preset row, whose options each write
//! several controls ([`PresetRow`]), and a lossless view's coordinate, a second view of
//! some stored controls ([`View`]), shown by a [`Row::Slider`] that names it.
//!
//! A module's colour pickers (decision 79) are declared apart from the rows too
//! ([`Picker`]), and each shown by one row: a slider's quad ([`Quad::Picker`]) or a row of
//! picker buttons ([`Row::Pickers`]).

use std::collections::BTreeSet;

use crate::values::{ControlValue, Controls};

/// A module, as data. Its words are message ids that follow from its ids
/// ([`crate::messages`]), so it holds no text.
#[derive(Debug)]
pub struct Definition {
    /// snake_case ASCII and permanent: every saved edit, the CLI and scripts name it.
    pub id: &'static str,
    /// From 1, raised whenever a control's meaning or the binding changes, with a
    /// migration ([`crate::Module::migrate`]).
    pub version: u32,
    /// Whether the module works on a masked layer.
    pub layers: bool,
    /// Whether the module has an Auto. The header draws no button for it.
    pub auto: bool,
    /// Whether the module is switched (decision 32): one darktable leaves off on a new
    /// photo, off until a command writes one of its controls, then on with darktable's
    /// defaults for the rest until its reset turns it off. It owns no parameter of an
    /// operation every photo runs ([`crate::check::accept`]), and its defaults bind its
    /// operations' own ([`crate::check::switched_defaults`]).
    pub switched: bool,
    pub controls: &'static [Control],
    /// One untitled tab draws no tab strip.
    pub tabs: &'static [Tab],
    /// Rows darktable draws outside its notebook, shown whichever tab is open: these
    /// before the tab's rows, as the colour equalizer's node placement.
    pub before: &'static [Row],
    /// And these after them, as the tone equalizer's mask display.
    pub after: &'static [Row],
    /// The preset rows its [`Row::Preset`]s draw (decision 2): options that each write
    /// several of its controls, as White balance's Mode. Nothing is stored under them.
    pub presets: &'static [PresetRow],
    /// Its lossless views (decision 2): second views of some of its controls, as White
    /// balance shows its light as temperature and tint, and as hue and chroma. Nothing is
    /// stored under them.
    pub views: &'static [View],
    /// Its colour pickers (decision 79), each darktable's `dt_color_picker_new` call, shown
    /// by a slider's quad ([`Quad::Picker`]) or a row of picker buttons ([`Row::Pickers`]).
    /// Nothing is stored under them: the session measures the photo and the module's
    /// [`crate::Module::pick`] writes its controls.
    pub pickers: &'static [Picker],
}

/// One of the module's colour pickers (decision 79; `docs/pickers.md`, Declaring
/// pickers): darktable's `dt_color_picker_new` call, its flags and colour space, and what
/// its pick may write and read.
///
/// Its words come with its button and the central bar (`docs/pickers.md`, The picker
/// button, On the photo), in the controls' and actions' scheme:
/// `module-<module>-picker-<id>-label`, its name; `-full`, what screen readers say;
/// `-promise`, its tooltip; and, for a point picker, `-what`, the central bar's sentence
/// ([`crate::messages::picker`]). [`crate::messages::of`] implies none of them: the app
/// draws a module's pickers only once its pick is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picker {
    /// snake_case ASCII and permanent, unique among the module's pickers: clients name it
    /// with the module's id. Pickers are a namespace of their own, as a picker on a
    /// slider's quad takes its control's id (Exposure's `exposure`).
    pub id: &'static str,
    /// The operation whose input, and with `output` output, it reads: one the module owns
    /// a parameter of ([`crate::check::accept`]).
    pub operation: &'static str,
    /// darktable's `DT_COLOR_PICKER_POINT`, `_AREA` or `_POINT_AREA`.
    pub kind: PickerKind,
    /// darktable's `DT_COLOR_PICKER_DENOISE`: a B-spline blur before the statistics, never
    /// on a mosaic.
    pub denoise: bool,
    /// darktable's `DT_COLOR_PICKER_IO`: the operation's output read as well as its input.
    pub output: bool,
    /// darktable's `picker_cst`: the space each pixel is read in.
    pub space: Space,
    /// darktable's "keep-active": it stays on when its module changes, as RGB curve's
    /// show colour does.
    pub keep: bool,
    /// The controls its pick may write, each once; none for a picker that only shows.
    pub writes: &'static [&'static str],
    /// The tool settings its pick reads ([`Row::ToolSetting`]), each once.
    pub tools: &'static [&'static str],
}

/// Whether a picker takes a point, an area or either: darktable's
/// `DT_COLOR_PICKER_POINT`, `_AREA` and `_POINT_AREA`. One with both turns on as an area
/// with a right-click or Ctrl-click, else as a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Point,
    Area,
    PointArea,
}

/// The space a picker reads each pixel in, darktable's `picker_cst`
/// (`dt_color_picker_helper`). In a space with a hue the fourth channel is the third moved
/// half a turn, so a hue round 0 has a minimum and a maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// The stage's own values: the mosaic in the raw stage, per CFA colour, camera RGB,
    /// the working space.
    Operation,
    /// LCh, for a Lab stage (`dt_Lab_2_LCH`).
    Lch,
    /// HSL, for an RGB stage (`dt_RGB_2_HSL`).
    Hsl,
    /// JzCzhz, through the working profile's matrix.
    Jzczhz,
}

/// Options that each write several of the module's controls, as White balance's Mode
/// writes its light (decision 2). Nothing is stored under it: the module gives each
/// option's values for a photo ([`crate::Module::preset`]) and says which option the
/// values hold ([`crate::Module::matching`]), and the session applies one as one change.
#[derive(Debug)]
pub struct PresetRow {
    /// snake_case ASCII and permanent, as a control's, and unique among the module's
    /// controls and preset rows.
    pub id: &'static str,
    /// Option ids, snake_case, in the row's order.
    pub options: &'static [&'static str],
    /// The controls every option writes.
    pub controls: &'static [&'static str],
}

/// A lossless view of some of the module's controls, every one a number (decision 2):
/// coordinates that are an exact, invertible change of coordinates of them, or fewer
/// coordinates than controls where the module's model drops a scale of them, as White
/// balance's temperature and tint drop its three coefficients' (decision 27), `unview`
/// then the inverse up to that scale. Nothing is stored under it: the module gives the
/// coordinates for the controls' values ([`crate::Module::view`]) and the values for
/// coordinates ([`crate::Module::unview`]), and the session writes the controls when a
/// coordinate is set, holding the view's other coordinates through the gesture.
#[derive(Debug)]
pub struct View {
    /// snake_case ASCII and permanent, unique among the module's views.
    pub id: &'static str,
    /// The stored controls it covers, each a number of the module, each once.
    pub controls: &'static [&'static str],
    /// As many as the controls it covers, or fewer for a model of them, at least one, in
    /// the order [`crate::Module::view`] gives them.
    pub coordinates: &'static [Coordinate],
}

/// One coordinate of a [`View`], addressed like a control: its id names it in a module
/// command, and its words are a control's ([`crate::messages`]).
#[derive(Debug)]
pub struct Coordinate {
    /// snake_case ASCII and permanent, unique among the module's controls, preset rows
    /// and coordinates.
    pub id: &'static str,
    /// What a command may set it to, `min..=max`. The controls' values may give it one
    /// outside, which a slider shows at its end.
    pub min: f32,
    pub max: f32,
    /// What a key or a scroll step moves it by.
    pub step: f32,
    pub unit: Option<Unit>,
}

impl Coordinate {
    /// `value` as this coordinate's number, if it is a number in its range.
    pub fn check(&self, value: &ControlValue) -> Result<f32, ControlError> {
        let control = self.id;
        match *value {
            // Also refuses NaN, which no range contains.
            ControlValue::Number(number) if (self.min..=self.max).contains(&number) => Ok(number),
            ControlValue::Number(number) => Err(ControlError::OutOfRange {
                control,
                value: f64::from(number),
                min: f64::from(self.min),
                max: f64::from(self.max),
            }),
            _ => Err(ControlError::WrongKind {
                control,
                expected: "a number",
            }),
        }
    }
}

/// What an edit stores for a module: one value of one kind.
#[derive(Debug)]
pub struct Control {
    /// snake_case ASCII and permanent, as the module's.
    pub id: &'static str,
    pub kind: ControlKind,
    /// A number's unit; `None` for a plain number and every other kind.
    pub unit: Option<Unit>,
    /// Whether its parameter is not ported (decision 29). darktable shows it, so it is a
    /// control, but it takes only its default, as a choice takes no option not ported:
    /// any other value is refused, so no edit stores a value the render ignores and
    /// nothing needs migrating once it is ported. The session reports it unavailable, so
    /// a panel draws it disabled.
    pub unported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlKind {
    /// A number in `min..=max`. `step` is what a key or a scroll step moves it by; a
    /// command may set any value in the range.
    Number {
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    },
    /// One of `options`, snake_case ids, by index: every option darktable lists
    /// (decision 29). Those its operation does not port are `unported`, by id: the panel
    /// draws them disabled and a value naming one is refused, so the default is never
    /// one.
    Choice {
        options: &'static [&'static str],
        default: u32,
        unported: &'static [&'static str],
    },
    Bool {
        default: bool,
    },
    /// A curve's points between its ends, (0, 0) and (1, 1), which the curve holds and
    /// the control does not store: up to [`MAX_POINTS`] of them, each coordinate from 0 to
    /// 1, x increasing from left to right by at least [`MIN_GAP`], from the ends too. None
    /// is the straight curve.
    Points {
        default: &'static [[f32; 2]],
    },
}

/// The most points a curve takes between its ends: darktable's RGB curve's 20 nodes
/// (`DT_IOP_RGBCURVE_MAXNODES`), less the two ends.
pub const MAX_POINTS: usize = 18;

/// How close in x two of a curve's points may come: darktable's RGB curve keeps its nodes
/// this far apart (`DT_IOP_RGBCURVE_MIN_X_DISTANCE`), and so does the design system's
/// curve, measuring in f32 as [`ControlKind::Points`] does.
pub const MIN_GAP: f32 = 0.0025;

/// A number's unit, from the closed list the translator has messages for: `unit-<id>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Ev,
    Percent,
    Degrees,
    Kelvin,
    Pixels,
    /// A film's speed, as darktable's grain shows its coarseness.
    Iso,
}

impl Unit {
    pub const ALL: [Unit; 6] = [
        Unit::Ev,
        Unit::Percent,
        Unit::Degrees,
        Unit::Kelvin,
        Unit::Pixels,
        Unit::Iso,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Unit::Ev => "ev",
            Unit::Percent => "percent",
            Unit::Degrees => "degrees",
            Unit::Kelvin => "kelvin",
            Unit::Pixels => "pixels",
            Unit::Iso => "iso",
        }
    }
}

/// One tab of the panel. A titled tab with no rows has nothing behind it yet: the panel
/// draws it disabled, as an action that cannot run is, saying it is to come.
#[derive(Debug)]
pub struct Tab {
    /// The tab's id, for its title; `None` for a module's only tab, which draws no tab
    /// strip.
    pub id: Option<&'static str>,
    pub rows: &'static [Row],
}

/// How a tab shows one control, a label between them, rows darktable groups or shows only
/// in some modes, a preset row, a lossless view's coordinate, one of darktable's settings
/// for a tool, or a row of picker buttons. Each control is in exactly one row, or in one
/// row per mode where darktable labels it differently by mode ([`Slider::label`]) or shows
/// it under conditions one [`Row::When`] cannot join, as highlight reconstruction's Noise
/// level, or only in the rows of the views covering it; each coordinate is in one row at
/// most, and each picker in exactly one.
#[derive(Debug, Clone)]
pub enum Row {
    Slider(Slider),
    /// A choice control, as a list or as segments. Only a list draws an option disabled,
    /// so a choice with unported options is a list.
    Choice {
        control: &'static str,
        style: ChoiceStyle,
    },
    /// A boolean control.
    Toggle {
        control: &'static str,
    },
    /// A label over the rows that follow, by its id: darktable's section label.
    Label {
        id: &'static str,
    },
    /// darktable's collapsible section, a collapsible group (decision 29): its rows
    /// under its title, closed at first, its open state the app's. It holds controls,
    /// never another group. One with no rows has nothing behind it yet, as AgX's Show
    /// curve until a read-only graph row exists: the panel draws it disabled, closed, as a
    /// titled tab with no rows is, saying it is to come.
    Group {
        id: &'static str,
        rows: &'static [Row],
    },
    /// Rows darktable shows only while another control of the module, a choice or a
    /// toggle, holds one of the values `is` names as that control writes them
    /// ([`Control::text`]): option ids, or `true` or `false`. Otherwise they are hidden,
    /// which a control that merely cannot act never is (decision 30). The control is
    /// shown wherever they are, or on another tab outside any group or mode, as the tone
    /// equalizer's filter on its Masking tab is for a row of its Advanced tab.
    When {
        control: &'static str,
        is: &'static [&'static str],
        rows: &'static [Row],
    },
    /// A widget from the design system's closed registry, with the controls it edits. The
    /// equalizer, over controls that have rows of their own, is a second view of them, the
    /// tone equalizer's graph of its bands (decision 2), and stores nothing of its own;
    /// no other widget may be a second row, until one needs it.
    Custom {
        widget: Widget,
        controls: &'static [&'static str],
    },
    /// A preset row of [`Definition::presets`], by its id. It shows no control of its
    /// own: each of its options writes several. It is a list: segments cannot draw an
    /// option the photo cannot have, nor Custom, yet, so [`Definition::check`] refuses
    /// them.
    Preset {
        preset: &'static str,
        style: ChoiceStyle,
    },
    /// One of darktable's own settings for a tool of the module, which darktable keeps in
    /// its darktablerc and never in an edit: the colour checker's chart, an area mode
    /// (decision 29; `docs/modules/README.md`, rule 12). It is no control: an app setting
    /// that comes with its tool, never stored with the photo. Until the tool comes the
    /// panel draws it disabled at darktable's default, so the page reads as darktable's and
    /// shows what is missing.
    ToolSetting(ToolSetting),
    /// One of darktable's buttons, by its id: an action that writes the module's controls
    /// and is never stored (`docs/modules/README.md`, rule 10), as AgX's Auto tune levels
    /// picks both exposures from the photo and its Set from above copies one set of
    /// controls to another. It is no control. Until its tool comes the panel draws it
    /// disabled, so the page reads as darktable's and shows what is missing.
    Action {
        id: &'static str,
    },
    /// darktable's picker buttons packed in a box, one line: a label at its start, if any,
    /// by its id, and the buttons of pickers of [`Definition::pickers`] at its end, as
    /// AgX's Auto tune levels and RGB levels' black, grey and white. It is no control.
    /// The label is a group label's: its words are [`crate::messages::group`]'s, and its
    /// id is unique among the module's group labels.
    Pickers {
        label: Option<&'static str>,
        pickers: &'static [&'static str],
    },
    /// One control's row of darktable's buttons, its label the control's, double-clicked
    /// to reset it, as darktable's reset label, with the buttons inline after it: actions
    /// that write the control from its value ([`crate::Module::action`]), never stored
    /// themselves, as orientation's rotate and flip. A click is one change, a
    /// `SetControl` of each control the action writes; the control shows only here.
    Buttons {
        control: &'static str,
        buttons: &'static [Button],
    },
    /// darktable's guides widget, which it adds to a module with
    /// `IOP_FLAGS_GUIDES_WIDGET` (`dt_guides_init_module_widget`): its show guides
    /// toggle, the overlay while the module has focus, kept in darktablerc and never in
    /// an edit, and the global guide settings button at its end. Its words are shared
    /// ([`crate::messages::GUIDES`]). Until the module's guides come the panel draws it
    /// disabled, off.
    Guides,
}

/// One button of a [`Row::Buttons`]: its action's id, whose words are an action's
/// (`module-<module>-action-<id>-label`, `-full` and `-promise`), and its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Button {
    pub id: &'static str,
    pub icon: ButtonIcon,
}

/// The design system's icons a button row can name, a closed registry as [`Widget`] is:
/// darktable's own shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonIcon {
    /// darktable's circular arrow turning anticlockwise (`dtgtk_cairo_paint_refresh`).
    RotateLeft,
    /// Its mirror image, clockwise.
    RotateRight,
    /// darktable's two triangles mirrored across a vertical line (`dtgtk_cairo_paint_flip`).
    FlipHorizontal,
    /// The same across a horizontal line.
    FlipVertical,
}

impl Row {
    /// The controls the row shows itself, or the coordinate a view's row shows, ids being
    /// unique together; a group's and a mode's are their rows', and a preset row and an
    /// action show none.
    pub fn controls(&self) -> &[&'static str] {
        match self {
            Row::Slider(slider) => std::slice::from_ref(&slider.control),
            Row::Choice { control, .. }
            | Row::Toggle { control }
            | Row::Buttons { control, .. } => std::slice::from_ref(control),
            Row::Label { .. }
            | Row::Group { .. }
            | Row::When { .. }
            | Row::ToolSetting(_)
            | Row::Preset { .. }
            | Row::Action { .. }
            | Row::Pickers { .. }
            | Row::Guides => &[],
            Row::Custom { controls, .. } => controls,
        }
    }

    /// The pickers the row shows: a slider's quad's, or a row of picker buttons'.
    pub fn pickers(&self) -> &[&'static str] {
        match self {
            Row::Slider(Slider {
                quad: Some(Quad::Picker(picker)),
                ..
            }) => std::slice::from_ref(picker),
            Row::Pickers { pickers, .. } => pickers,
            _ => &[],
        }
    }

    /// The rows inside a group or a mode's rows; none for any other row.
    pub fn rows(&self) -> &'static [Row] {
        match self {
            Row::Group { rows, .. } | Row::When { rows, .. } => rows,
            _ => &[],
        }
    }
}

/// A number control, or a view's coordinate, as a slider row
/// (`apertrust_ui::components::SliderRow`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slider {
    /// A number control's id, or a coordinate's.
    pub control: &'static str,
    /// Decimals the value shows.
    pub decimals: u8,
    /// darktable's soft range (`dt_bauhaus_slider_set_soft_range`), `(min, max)` in the
    /// control's units, inside its range: what the track spans at rest. The control's
    /// range stays what a value may hold, so a typed value, a script or a preset goes
    /// beyond it, and the track widens to show it, as darktable's does. `None` where
    /// darktable sets none: the track spans the whole range.
    pub soft: Option<(f32, f32)>,
    pub track: Track,
    /// Whether the track fills from the default to the value.
    pub fill: bool,
    /// The action at the end of the row, if any.
    pub quad: Option<Quad>,
    /// The row's own words, by an id, where darktable labels the control otherwise in
    /// the mode the row shows for; `None` for the control's own.
    pub label: Option<&'static str>,
}

impl Slider {
    /// Two decimals, the track over the whole range, plain, with its fill, no quad, the
    /// control's own words.
    pub const fn new(control: &'static str) -> Self {
        Self {
            control,
            decimals: 2,
            soft: None,
            track: Track::Plain,
            fill: true,
            quad: None,
            label: None,
        }
    }

    /// Words of the row's own, `words` a snake_case id: one row per mode where darktable
    /// shows one parameter under two labels, as local contrast's `sigma_r` is Contrast
    /// with the bilateral grid and Highlights with the local Laplacian filter, each row
    /// in a [`Row::When`] of its mode ([`crate::messages::row_words`]).
    pub const fn label(mut self, words: &'static str) -> Self {
        self.label = Some(words);
        self
    }

    pub const fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// darktable's soft range, in the control's units ([`Self::soft`]).
    pub const fn soft(mut self, min: f32, max: f32) -> Self {
        self.soft = Some((min, max));
        self
    }

    pub const fn track(mut self, track: Track) -> Self {
        self.track = track;
        self
    }

    pub const fn no_fill(mut self) -> Self {
        self.fill = false;
        self
    }

    pub const fn quad(mut self, quad: Quad) -> Self {
        self.quad = Some(quad);
        self
    }
}

/// A number control's range for the module's values (decision 30), as darktable's slider
/// has them: what a value may hold, `min..=max`; what the track spans at rest, `soft`,
/// inside it; and what a key moves, `step`. Most are the definition's
/// ([`Definition::range`]); where darktable's `gui_changed` moves a slider's range with
/// other controls, as AgX's pivot spans black to white relative exposure, the module gives
/// it ([`crate::Module::range`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub soft: (f32, f32),
    pub step: f32,
}

impl Range {
    /// Whether `value` lies in `min..=max`.
    pub fn contains(&self, value: f32) -> bool {
        (self.min..=self.max).contains(&value)
    }

    /// `value` at the range's nearer end, if it lies outside it.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }

    /// Whether it holds together inside `within`, the definition's range: finite, `min`
    /// below `max`, inside `within`'s `min..=max`, its track in order inside its own, and
    /// its step above 0.
    pub fn holds_inside(&self, within: &Range) -> bool {
        let (low, high) = self.soft;
        [self.min, self.max, low, high, self.step]
            .iter()
            .all(|v| v.is_finite())
            && self.min < self.max
            && within.min <= self.min
            && self.max <= within.max
            && self.min <= low
            && low < high
            && high <= self.max
            && self.step > 0.0
    }
}

/// darktable's step for a slider given none (`dt_bauhaus_slider_get_step`), in the units
/// it shows: from `min` to `max`, the span at rest in the parameter's units, and `factor`,
/// what it shows a parameter by. 1 in the parameter's units where the span's top reaches
/// 100, else a hundredth of it in the units shown, taken to the power of ten at or just
/// below it (within a tenth of a decade above), times 5 where it lies more than half a
/// decade above that power: 0.04 EV gives 0.05, 0.2 % gives 0.1 %. In f32, as darktable
/// computes it.
pub fn slider_step(min: f32, max: f32, factor: f32) -> f32 {
    let top = (max - min).min(min.abs().max(max.abs()));
    let factor = factor.abs();
    if top >= 100.0 {
        return factor;
    }
    let log10step = (top * factor / 100.0).log10();
    let digits = (log10step + 0.1).floor();
    let step = 10f32.powf(digits);
    if log10step - digits > 0.5 {
        step * 5.0
    } else {
        step
    }
}

/// One of darktable's own settings for a tool, as a row ([`Row::ToolSetting`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolSetting {
    /// snake_case, unique among the module's tool settings: its words follow from it
    /// ([`crate::messages::setting`]).
    pub id: &'static str,
    /// A number, a choice or a boolean, with darktable's range, options and default.
    pub kind: ControlKind,
    /// Decimals a number shows.
    pub decimals: u8,
}

impl ToolSetting {
    /// Whether the setting holds together: its id, and its kind as a control's; curve
    /// points are no setting darktable has.
    fn check(&self) -> Result<(), DefinitionError> {
        if !is_id(self.id) {
            return Err(DefinitionError::Id(self.id));
        }
        if matches!(self.kind, ControlKind::Points { .. }) {
            return Err(DefinitionError::Row(self.id));
        }
        Control {
            id: self.id,
            kind: self.kind,
            unit: None,
            unported: false,
        }
        .check_kind()
    }
}

/// A slider's track: a plain line, or a colour band where colour is the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Plain,
    Temperature,
    Tint,
    Hue,
    Saturation,
    Luminance,
}

/// What a slider row's quad does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quad {
    /// darktable's picker on a slider (`dt_color_picker_new` on a bauhaus widget): turns
    /// the picker of [`Definition::pickers`] it names on and off.
    Picker(&'static str),
    /// Draws a line on the photo, as a horizon is levelled.
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceStyle {
    /// A drop-down list.
    List,
    /// Segments, for a few short options.
    Segmented,
}

/// The design system's painted widgets a definition can name; a third-party module
/// chooses among them and ships no UI code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Widget {
    Curve,
    ColourWheels,
    /// darktable's tone equalizer graph: its controls are bands in EV, numbers 1 EV apart
    /// up to +0 EV, the darkest first; it draws the module's curve through them
    /// ([`crate::Module::curve`] of the first), and a drag reaches as far as the curve is
    /// wide ([`crate::Module::curve_width`]).
    Equalizer,
    HueCurve,
    Swatches,
}

/// A value a control cannot take.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ControlError {
    #[error("{control} takes {expected}")]
    WrongKind {
        control: &'static str,
        expected: &'static str,
    },
    #[error("{control} is {value}, outside {min} to {max}")]
    OutOfRange {
        control: &'static str,
        value: f64,
        min: f64,
        max: f64,
    },
    #[error("{control} has no option {option}")]
    NoOption {
        control: &'static str,
        option: String,
    },
    #[error(
        "{control} takes up to {MAX_POINTS} points between its ends, left to right, {MIN_GAP} apart"
    )]
    Points { control: &'static str },
    #[error("{control}: cannot read {text:?}")]
    Invalid { control: &'static str, text: String },
    #[error("{control}: {option} is not ported yet")]
    Unported {
        control: &'static str,
        option: &'static str,
    },
    #[error("{control} is not ported yet: it takes only its default")]
    NotPorted { control: &'static str },
}

/// What is wrong with a definition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DefinitionError {
    #[error("{0:?} is not a snake_case id")]
    Id(&'static str),
    #[error("a module's version starts at 1")]
    Version,
    #[error("two controls are called {0}")]
    DuplicateControl(&'static str),
    #[error("{0}: its range, step and default do not hold together")]
    Range(&'static str),
    #[error("{0}: its slider's soft range is not in order inside its range")]
    SoftRange(&'static str),
    #[error(
        "{0}: its options are missing, repeated or not snake_case, or its default is none of them"
    )]
    Options(&'static str),
    #[error("{0}: an option marked unported is none of its options, or is its default")]
    Unported(&'static str),
    #[error(
        "{0}: its default points are not up to {MAX_POINTS} between its ends, left to right, {MIN_GAP} apart"
    )]
    Points(&'static str),
    #[error("{0}: only a number has a unit")]
    Unit(&'static str),
    #[error("a module has one untitled tab, or every tab titled, each once")]
    Tabs,
    #[error("a row shows {0}, which the module lacks")]
    UnknownControl(&'static str),
    #[error(
        "{control} is in {rows} rows; each control is in exactly one, or in one per mode, no two shown at once, or only in the rows of a view covering it, besides one equalizer's at most"
    )]
    Rows { control: &'static str, rows: usize },
    #[error("{0} is in a row that cannot show its kind")]
    Row(&'static str),
    #[error("{0} has options not ported, which only a list draws disabled")]
    Segmented(&'static str),
    #[error("two group labels are called {0}")]
    DuplicateLabel(&'static str),
    #[error("two collapsible groups are called {0}")]
    DuplicateGroup(&'static str),
    #[error("the collapsible group {0} is inside another; a group holds controls, never a group")]
    GroupInGroup(&'static str),
    #[error("two tool settings are called {0}")]
    DuplicateSetting(&'static str),
    #[error("two actions are called {0}")]
    DuplicateAction(&'static str),
    #[error(
        "rows follow {0}: not a choice or a toggle of the module, no value or one it cannot hold, or among those rows"
    )]
    Condition(&'static str),
    #[error(
        "rows follow {0} where none of its rows is sure to show: on their tab, outside its group or outside its modes; on another tab, in a group or a mode"
    )]
    Hidden(&'static str),
    #[error("{0}: rows following it, with no row")]
    Empty(&'static str),
    #[error("a preset row is called {0}, as a control or another preset row is")]
    DuplicatePreset(&'static str),
    #[error("the preset row {0}: its options are missing, repeated or not snake_case")]
    PresetOptions(&'static str),
    #[error("the preset row {0} writes no control, one twice or one the module lacks")]
    PresetControls(&'static str),
    #[error("a row shows the preset row {0}, which the module lacks")]
    UnknownPreset(&'static str),
    #[error("the preset row {0} is segmented; only a list draws Custom and options disabled")]
    SegmentedPreset(&'static str),
    #[error("the preset row {preset} is in {rows} rows; each is in one at most")]
    PresetRows { preset: &'static str, rows: usize },
    #[error("two views are called {0}")]
    DuplicateView(&'static str),
    #[error(
        "the view {0} covers no control, one twice, one the module lacks or one that is no number"
    )]
    ViewControls(&'static str),
    #[error("the view {0} has no coordinate, or more than the controls it covers")]
    ViewCoordinates(&'static str),
    #[error("a coordinate is called {0}, as a control, a preset row or another coordinate is")]
    DuplicateCoordinate(&'static str),
    #[error("the coordinate {coordinate} is in {rows} rows; each is in one at most")]
    CoordinateRows {
        coordinate: &'static str,
        rows: usize,
    },
    #[error("two pickers are called {0}")]
    DuplicatePicker(&'static str),
    #[error("the picker {0} writes a control the module lacks, or one twice")]
    PickerWrites(&'static str),
    #[error("the picker {0} reads a tool setting the module lacks, or one twice")]
    PickerTools(&'static str),
    #[error("a row shows the picker {0}, which the module lacks")]
    UnknownPicker(&'static str),
    #[error("a row of pickers holds none")]
    NoPickers,
    #[error("the picker {picker} is in {rows} rows; each is in exactly one")]
    PickerRows { picker: &'static str, rows: usize },
}

/// snake_case ASCII: a lowercase letter, then lowercase letters, digits and single
/// underscores, not ending in one. Message ids join ids with hyphens, so none can
/// contain one.
pub(crate) fn is_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !id.contains("__")
        && !id.ends_with('_')
}

impl Definition {
    pub fn control(&self, id: &str) -> Option<&Control> {
        self.controls.iter().find(|control| control.id == id)
    }

    pub fn preset(&self, id: &str) -> Option<&PresetRow> {
        self.presets.iter().find(|preset| preset.id == id)
    }

    pub fn view(&self, id: &str) -> Option<&'static View> {
        self.views.iter().find(|view| view.id == id)
    }

    pub fn picker(&self, id: &str) -> Option<&'static Picker> {
        self.pickers.iter().find(|picker| picker.id == id)
    }

    /// The coordinate called `id`: its view, and its place among the view's coordinates.
    pub fn coordinate(&self, id: &str) -> Option<(&'static View, usize)> {
        self.views.iter().find_map(|view| {
            let at = view.coordinates.iter().position(|c| c.id == id)?;
            Some((view, at))
        })
    }

    /// A number control's range as the definition gives it (decision 30): its range and
    /// step, and the soft range of the first slider row showing it, else its range, so its
    /// track. `None` for a control that is no number, or that the module lacks.
    pub fn range(&self, control: &str) -> Option<Range> {
        let ControlKind::Number { min, max, step, .. } = self.control(control)?.kind else {
            return None;
        };
        let soft = self.rows().find_map(|row| match row {
            Row::Slider(slider) if slider.control == control => Some(slider.soft),
            _ => None,
        });
        Some(Range {
            min,
            max,
            soft: soft.flatten().unwrap_or((min, max)),
            step,
        })
    }

    /// Every control at the definition's default.
    pub fn defaults(&self) -> Controls {
        let mut defaults = Controls::new();
        for control in self.controls {
            defaults.set(control.id, control.default());
        }
        defaults
    }

    /// Every row in the order the panel lists them, each before the rows inside it: those
    /// before every tab's, the tabs', then those after, with the rows of groups and modes.
    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.placed().into_iter().map(|placed| placed.row)
    }

    /// Every row, as [`Self::rows`], with where it shows: its tab, its group, its modes.
    fn placed(&self) -> Vec<Placed<'_>> {
        let mut placed = Vec::new();
        let tabs = self.tabs.iter().enumerate();
        let all = std::iter::once((None, self.before))
            .chain(tabs.map(|(i, tab)| (Some(i), tab.rows)))
            .chain(std::iter::once((None, self.after)));
        for (tab, rows) in all {
            let at = Where {
                tab,
                ..Where::default()
            };
            place(rows, &at, &mut placed);
        }
        placed
    }

    /// Whether the definition holds together: ids, ranges, options, tabs, groups that
    /// hold no group, modes of a choice or a toggle with rows of their own, each
    /// where the control it follows is sure to show, or following a control that another
    /// tab shows outside any group or mode, each control in exactly one row
    /// that can show it, or in one per mode, no two of them shown at once, or in none
    /// and only in the rows of the views covering it or of a preset row writing it,
    /// besides, for a number, in one equalizer's row at most, the one widget allowed
    /// beside a control's own rows, each preset row with options,
    /// writing controls the module has, in one row at most and a list, each view
    /// covering number controls of the module, each once, with a coordinate for each or
    /// fewer,
    /// each coordinate with its range in one slider row at most, each slider's soft range
    /// in order inside its control's or coordinate's range, each tool setting once, its
    /// kind as a control's, and each action once. Control, preset row and coordinate ids
    /// are unique together. Each picker has an id of its own, writes controls of the
    /// module and reads its tool settings, each once, and is in exactly one row, a
    /// slider's quad or a row of pickers, which holds at least one.
    pub fn check(&self) -> Result<(), DefinitionError> {
        if !is_id(self.id) {
            return Err(DefinitionError::Id(self.id));
        }
        if self.version == 0 {
            return Err(DefinitionError::Version);
        }
        let mut ids = BTreeSet::new();
        for control in self.controls {
            if !is_id(control.id) {
                return Err(DefinitionError::Id(control.id));
            }
            if !ids.insert(control.id) {
                return Err(DefinitionError::DuplicateControl(control.id));
            }
            control.check_kind()?;
        }
        for preset in self.presets {
            if !is_id(preset.id) {
                return Err(DefinitionError::Id(preset.id));
            }
            // A preset row's id is in the controls' namespace (`docs/engine.md`, Preset
            // rows and lossless views).
            if !ids.insert(preset.id) {
                return Err(DefinitionError::DuplicatePreset(preset.id));
            }
            self.check_preset(preset)?;
        }
        let mut views = BTreeSet::new();
        for view in self.views {
            if !is_id(view.id) {
                return Err(DefinitionError::Id(view.id));
            }
            if !views.insert(view.id) {
                return Err(DefinitionError::DuplicateView(view.id));
            }
            self.check_view(view)?;
            for coordinate in view.coordinates {
                if !is_id(coordinate.id) {
                    return Err(DefinitionError::Id(coordinate.id));
                }
                // Addressed like a control, so in the controls' namespace too.
                if !ids.insert(coordinate.id) {
                    return Err(DefinitionError::DuplicateCoordinate(coordinate.id));
                }
                if !range_holds(coordinate.min, coordinate.max, coordinate.step) {
                    return Err(DefinitionError::Range(coordinate.id));
                }
            }
        }
        let titled = self.tabs.iter().filter(|tab| tab.id.is_some()).count();
        let tab_ids: BTreeSet<_> = self.tabs.iter().filter_map(|tab| tab.id).collect();
        let tabs_hold = match self.tabs.len() {
            0 => false,
            1 => true,
            n => titled == n && tab_ids.len() == n,
        };
        if !tabs_hold {
            return Err(DefinitionError::Tabs);
        }
        if let Some(id) = tab_ids.iter().find(|id| !is_id(id)) {
            return Err(DefinitionError::Id(id));
        }
        let placed = self.placed();
        let (mut labels, mut groups, mut settings, mut actions) = (
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        for &Placed { row, ref at } in &placed {
            // A group with no rows has nothing behind it yet; rows following a control
            // must be rows.
            if let Row::When { control, rows, .. } = row
                && rows.is_empty()
            {
                return Err(DefinitionError::Empty(control));
            }
            match row {
                // A row of pickers' label shares a group label's words, so its id is one.
                Row::Label { id }
                | Row::Pickers {
                    label: Some(id), ..
                } => {
                    if !is_id(id) {
                        return Err(DefinitionError::Id(id));
                    }
                    if !labels.insert(*id) {
                        return Err(DefinitionError::DuplicateLabel(id));
                    }
                }
                Row::Group { id, rows } => {
                    if !is_id(id) {
                        return Err(DefinitionError::Id(id));
                    }
                    if !groups.insert(*id) {
                        return Err(DefinitionError::DuplicateGroup(id));
                    }
                    if let Some(inner) = group_in(rows) {
                        return Err(DefinitionError::GroupInGroup(inner));
                    }
                }
                Row::When { control, is, rows } => {
                    self.check_mode(control, is, rows)?;
                    let shown = placed.iter().any(|its| {
                        its.row.controls().contains(control)
                            && (its.at.shows_wherever(at) || its.at.shows_on_another_tab(at))
                    });
                    if !shown {
                        return Err(DefinitionError::Hidden(control));
                    }
                }
                Row::Slider(Slider {
                    label: Some(words), ..
                }) if !is_id(words) => return Err(DefinitionError::Id(words)),
                Row::Slider(Slider {
                    control,
                    soft: Some(soft),
                    ..
                }) => {
                    // A control or a coordinate the module lacks is refused below.
                    if let Some(range) = self.number_range(control)
                        && !soft_holds(*soft, range)
                    {
                        return Err(DefinitionError::SoftRange(control));
                    }
                }
                Row::Preset { preset, .. } if self.preset(preset).is_none() => {
                    return Err(DefinitionError::UnknownPreset(preset));
                }
                Row::Preset {
                    preset,
                    style: ChoiceStyle::Segmented,
                } => return Err(DefinitionError::SegmentedPreset(preset)),
                Row::ToolSetting(setting) => {
                    if !settings.insert(setting.id) {
                        return Err(DefinitionError::DuplicateSetting(setting.id));
                    }
                    setting.check()?;
                }
                Row::Action { id } => {
                    if !is_id(id) {
                        return Err(DefinitionError::Id(id));
                    }
                    if !actions.insert(*id) {
                        return Err(DefinitionError::DuplicateAction(id));
                    }
                }
                Row::Buttons { buttons, .. } => {
                    for Button { id, .. } in *buttons {
                        if !is_id(id) {
                            return Err(DefinitionError::Id(id));
                        }
                        if !actions.insert(*id) {
                            return Err(DefinitionError::DuplicateAction(id));
                        }
                    }
                }
                _ => {}
            }
            if let Row::Pickers { pickers: [], .. } = row {
                return Err(DefinitionError::NoPickers);
            }
            if let Some(picker) = row.pickers().iter().find(|id| self.picker(id).is_none()) {
                return Err(DefinitionError::UnknownPicker(picker));
            }
            for &id in row.controls() {
                let Some(control) = self.control(id) else {
                    // A view's coordinate, which only a slider shows.
                    match (row, self.coordinate(id)) {
                        (Row::Slider(_), Some(_)) => continue,
                        (_, Some(_)) => return Err(DefinitionError::Row(id)),
                        (_, None) => return Err(DefinitionError::UnknownControl(id)),
                    }
                };
                let fits = match (row, control.kind) {
                    (Row::Slider(_), ControlKind::Number { .. })
                    | (Row::Toggle { .. }, ControlKind::Bool { .. })
                    | (
                        Row::Custom {
                            widget: Widget::Equalizer,
                            ..
                        },
                        ControlKind::Number { .. },
                    ) => true,
                    (
                        Row::Custom {
                            widget: Widget::Equalizer,
                            ..
                        },
                        _,
                    ) => false,
                    (Row::Custom { .. }, _) => true,
                    // Its buttons write it, whatever its kind.
                    (Row::Buttons { .. }, _) => true,
                    (Row::Choice { style, .. }, ControlKind::Choice { unported, .. }) => {
                        if *style == ChoiceStyle::Segmented && !unported.is_empty() {
                            return Err(DefinitionError::Segmented(id));
                        }
                        true
                    }
                    _ => false,
                };
                if !fits {
                    return Err(DefinitionError::Row(id));
                }
            }
        }
        let rows_of = |id: &str| -> Vec<&Placed> {
            placed
                .iter()
                .filter(|placed| placed.row.controls().contains(&id))
                .collect()
        };
        for control in self.controls {
            let all = rows_of(control.id);
            // The equalizer over controls with rows of their own is a second view of them,
            // the tone equalizer's graph of its bands, so only their own rows count; one at
            // most. It is the only widget allowed so, until another needs it.
            let (custom, own): (Vec<&Placed>, Vec<&Placed>) = all.iter().partition(|placed| {
                matches!(
                    placed.row,
                    Row::Custom {
                        widget: Widget::Equalizer,
                        ..
                    }
                )
            });
            if custom.len() > 1 {
                return Err(DefinitionError::Rows {
                    control: control.id,
                    rows: all.len(),
                });
            }
            let rows = if own.is_empty() { custom } else { own };
            let apart = rows
                .iter()
                .enumerate()
                .all(|(i, a)| rows[i + 1..].iter().all(|b| a.apart(b)));
            // A control in no row of its own shows through the views covering it, or
            // through a preset row that writes it, as White balance's `preset` does
            // through Settings (decision 27).
            let viewed = || {
                self.views
                    .iter()
                    .filter(|view| view.controls.contains(&control.id))
                    .flat_map(|view| view.coordinates)
                    .any(|coordinate| !rows_of(coordinate.id).is_empty())
            };
            let preset_shown = || {
                self.presets
                    .iter()
                    .filter(|preset| preset.controls.contains(&control.id))
                    .any(|preset| {
                        placed.iter().any(|placed| {
                            matches!(placed.row, Row::Preset { preset: id, .. } if *id == preset.id)
                        })
                    })
            };
            if (rows.is_empty() && !viewed() && !preset_shown()) || !apart {
                return Err(DefinitionError::Rows {
                    control: control.id,
                    rows: rows.len(),
                });
            }
        }
        for coordinate in self.views.iter().flat_map(|view| view.coordinates) {
            let rows = rows_of(coordinate.id).len();
            if rows > 1 {
                return Err(DefinitionError::CoordinateRows {
                    coordinate: coordinate.id,
                    rows,
                });
            }
        }
        for preset in self.presets {
            let rows = placed
                .iter()
                .filter(|placed| match placed.row {
                    Row::Preset { preset: id, .. } => *id == preset.id,
                    _ => false,
                })
                .count();
            if rows > 1 {
                return Err(DefinitionError::PresetRows {
                    preset: preset.id,
                    rows,
                });
            }
        }
        let mut pickers = BTreeSet::new();
        for picker in self.pickers {
            if !is_id(picker.id) {
                return Err(DefinitionError::Id(picker.id));
            }
            if !pickers.insert(picker.id) {
                return Err(DefinitionError::DuplicatePicker(picker.id));
            }
            self.check_picker(picker, &settings)?;
            // Counted where they are named, so a row naming one twice shows it twice.
            let rows = placed
                .iter()
                .flat_map(|placed| placed.row.pickers())
                .filter(|&&id| id == picker.id)
                .count();
            if rows != 1 {
                return Err(DefinitionError::PickerRows {
                    picker: picker.id,
                    rows,
                });
            }
        }
        Ok(())
    }

    /// Whether a picker's writes are controls of the module and its tools tool settings
    /// of it, `settings` being their ids, each once.
    fn check_picker(
        &self,
        picker: &Picker,
        settings: &BTreeSet<&str>,
    ) -> Result<(), DefinitionError> {
        let writes: BTreeSet<_> = picker.writes.iter().collect();
        let writes_hold = writes.len() == picker.writes.len()
            && picker.writes.iter().all(|id| self.control(id).is_some());
        if !writes_hold {
            return Err(DefinitionError::PickerWrites(picker.id));
        }
        let tools: BTreeSet<_> = picker.tools.iter().collect();
        let tools_hold = tools.len() == picker.tools.len()
            && picker.tools.iter().all(|id| settings.contains(id));
        if !tools_hold {
            return Err(DefinitionError::PickerTools(picker.id));
        }
        Ok(())
    }

    /// The range of a number control or a coordinate called `id`.
    fn number_range(&self, id: &str) -> Option<(f32, f32)> {
        match self.control(id).map(|control| control.kind) {
            Some(ControlKind::Number { min, max, .. }) => Some((min, max)),
            Some(_) => None,
            None => self.coordinate(id).map(|(view, at)| {
                let coordinate = &view.coordinates[at];
                (coordinate.min, coordinate.max)
            }),
        }
    }

    /// Whether a preset row holds together: options that are ids, each once, and controls
    /// of the module, each once.
    fn check_preset(&self, preset: &PresetRow) -> Result<(), DefinitionError> {
        let options: BTreeSet<_> = preset.options.iter().collect();
        let options_hold = !preset.options.is_empty()
            && options.len() == preset.options.len()
            && preset.options.iter().all(|option| is_id(option));
        if !options_hold {
            return Err(DefinitionError::PresetOptions(preset.id));
        }
        let controls: BTreeSet<_> = preset.controls.iter().collect();
        let controls_hold = !preset.controls.is_empty()
            && controls.len() == preset.controls.len()
            && preset.controls.iter().all(|id| self.control(id).is_some());
        if !controls_hold {
            return Err(DefinitionError::PresetControls(preset.id));
        }
        Ok(())
    }

    /// Whether a view holds together: number controls of the module, each once, and a
    /// coordinate for each, so it can be an invertible change of their coordinates, or
    /// fewer, where the module's model drops a scale of them, as White balance's
    /// temperature and tint drop the coefficients' (decision 27).
    fn check_view(&self, view: &View) -> Result<(), DefinitionError> {
        let controls: BTreeSet<_> = view.controls.iter().collect();
        let numbers = view.controls.iter().all(|id| {
            self.control(id)
                .is_some_and(|control| matches!(control.kind, ControlKind::Number { .. }))
        });
        if view.controls.is_empty() || controls.len() != view.controls.len() || !numbers {
            return Err(DefinitionError::ViewControls(view.id));
        }
        if view.coordinates.is_empty() || view.coordinates.len() > view.controls.len() {
            return Err(DefinitionError::ViewCoordinates(view.id));
        }
        Ok(())
    }

    /// Whether rows can follow `control` for the values `is` names: a choice or a toggle
    /// of the module, values it holds, and none of the rows its own.
    fn check_mode(
        &self,
        control: &'static str,
        is: &[&str],
        rows: &[Row],
    ) -> Result<(), DefinitionError> {
        let values: &[&str] = match self.control(control).map(|found| found.kind) {
            Some(ControlKind::Choice { options, .. }) => options,
            Some(ControlKind::Bool { .. }) => &["false", "true"],
            _ => return Err(DefinitionError::Condition(control)),
        };
        let holds = !is.is_empty() && is.iter().all(|value| values.contains(value));
        if !holds || shows(rows, control) {
            return Err(DefinitionError::Condition(control));
        }
        Ok(())
    }
}

/// A row, and where its definition puts it.
struct Placed<'d> {
    row: &'d Row,
    at: Where,
}

/// Where a row shows: its tab, `None` for the rows before and after every tab's; its
/// collapsible group; each mode it shows in, as the control it follows and the values it
/// shows for, outermost first.
#[derive(Clone, Default)]
struct Where {
    tab: Option<usize>,
    group: Option<&'static str>,
    modes: Vec<(&'static str, &'static [&'static str])>,
}

impl Placed<'_> {
    /// Whether the two rows are never shown at once: one follows a control for values
    /// the other follows it for none of.
    fn apart(&self, other: &Placed) -> bool {
        self.at.modes.iter().any(|(control, is)| {
            other
                .at
                .modes
                .iter()
                .any(|(other, its)| control == other && is.iter().all(|v| !its.contains(v)))
        })
    }
}

impl Where {
    /// Whether a row here shows whenever one at `other` does: on every tab or on its tab,
    /// outside any group or in its group, and in each of these modes because `other` is
    /// in it too, for some of its values only.
    fn shows_wherever(&self, other: &Where) -> bool {
        let tab = self.tab.is_none() || self.tab == other.tab;
        let group = self.group.is_none() || self.group == other.group;
        let modes = self.modes.iter().all(|(control, is)| {
            other
                .modes
                .iter()
                .any(|(other, its)| control == other && its.iter().all(|v| is.contains(v)))
        });
        tab && group && modes
    }

    /// Whether a row here is on a tab other than `other`'s and shows whenever its own tab
    /// is open, outside any group or mode: a control that rows on another tab may follow,
    /// as darktable's tone equalizer shows its contrast compensation, on the Advanced tab,
    /// only for some of the filters chosen on the Masking tab.
    fn shows_on_another_tab(&self, other: &Where) -> bool {
        self.tab.is_some() && self.tab != other.tab && self.group.is_none() && self.modes.is_empty()
    }
}

/// Places `rows` and the rows inside them, at `at` and inside the groups and modes they
/// are in.
fn place<'d>(rows: &'d [Row], at: &Where, placed: &mut Vec<Placed<'d>>) {
    for row in rows {
        placed.push(Placed {
            row,
            at: at.clone(),
        });
        let mut inside = at.clone();
        match row {
            Row::Group { id, .. } => inside.group = Some(id),
            Row::When { control, is, .. } => inside.modes.push((control, is)),
            _ => {}
        }
        place(row.rows(), &inside, placed);
    }
}

/// The first group among `rows` or inside them.
fn group_in(rows: &[Row]) -> Option<&'static str> {
    rows.iter().find_map(|row| match row {
        Row::Group { id, .. } => Some(*id),
        _ => group_in(row.rows()),
    })
}

/// Whether `rows`, or the rows inside them, show `control`.
fn shows(rows: &[Row], control: &str) -> bool {
    rows.iter()
        .any(|row| row.controls().contains(&control) || shows(row.rows(), control))
}

/// Whether a number's range holds together: finite ends in order, and a step that fits.
fn range_holds(min: f32, max: f32, step: f32) -> bool {
    min.is_finite() && max.is_finite() && min < max && step > 0.0 && step <= max - min
}

/// Whether a slider's soft range holds together: finite ends in order, inside the range,
/// as darktable's `dt_bauhaus_slider_set_soft_range` keeps it.
fn soft_holds((low, high): (f32, f32), (min, max): (f32, f32)) -> bool {
    low.is_finite() && high.is_finite() && min <= low && low < high && high <= max
}

/// Whether `points` lie between a curve's ends: up to [`MAX_POINTS`], each coordinate
/// from 0 to 1, x increasing by at least [`MIN_GAP`] from the end at 0, through the
/// points, to the end at 1, measured in f32 as the curve widget measures it.
fn points_hold(points: &[[f32; 2]]) -> bool {
    let xs: Vec<f32> = std::iter::once(0.0)
        .chain(points.iter().map(|point| point[0]))
        .chain(std::iter::once(1.0))
        .collect();
    points.len() <= MAX_POINTS
        && points
            .iter()
            .flatten()
            .all(|coordinate| (0.0..=1.0).contains(coordinate))
        && xs.windows(2).all(|pair| pair[1] - pair[0] >= MIN_GAP)
}

impl Control {
    pub fn default(&self) -> ControlValue {
        match self.kind {
            ControlKind::Number { default, .. } => ControlValue::Number(default),
            ControlKind::Choice { default, .. } => ControlValue::Choice(default),
            ControlKind::Bool { default } => ControlValue::Bool(default),
            ControlKind::Points { default } => ControlValue::Points(default.to_vec()),
        }
    }

    /// Whether the kind holds together, for [`Definition::check`].
    fn check_kind(&self) -> Result<(), DefinitionError> {
        let id = self.id;
        match self.kind {
            ControlKind::Number {
                min,
                max,
                step,
                default,
            } => {
                if !range_holds(min, max, step) || !(min..=max).contains(&default) {
                    return Err(DefinitionError::Range(id));
                }
            }
            ControlKind::Choice {
                options,
                default,
                unported,
            } => {
                let unique: BTreeSet<_> = options.iter().collect();
                let holds = !options.is_empty()
                    && unique.len() == options.len()
                    && options.iter().all(|option| is_id(option))
                    && (default as usize) < options.len();
                if !holds {
                    return Err(DefinitionError::Options(id));
                }
                let marked = unported.iter().all(|option| options.contains(option))
                    && !unported.contains(&options[default as usize]);
                if !marked {
                    return Err(DefinitionError::Unported(id));
                }
            }
            ControlKind::Bool { .. } => {}
            ControlKind::Points { default } => {
                if !points_hold(default) {
                    return Err(DefinitionError::Points(id));
                }
            }
        }
        if self.unit.is_some() && !matches!(self.kind, ControlKind::Number { .. }) {
            return Err(DefinitionError::Unit(id));
        }
        Ok(())
    }

    /// Whether `value` has this control's kind and lies in its range: for a choice, one
    /// of its options that is ported; for a control that is not ported, its default.
    pub fn check(&self, value: &ControlValue) -> Result<(), ControlError> {
        self.check_kind_of(value)?;
        if self.unported && *value != self.default() {
            return Err(ControlError::NotPorted { control: self.id });
        }
        Ok(())
    }

    /// Whether `value` has this control's kind and lies in its range: for a choice, one
    /// of its options that is ported.
    fn check_kind_of(&self, value: &ControlValue) -> Result<(), ControlError> {
        let control = self.id;
        match (self.kind, value) {
            (ControlKind::Number { min, max, .. }, ControlValue::Number(value)) => {
                // Also refuses NaN, which no range contains.
                if (min..=max).contains(value) {
                    Ok(())
                } else {
                    Err(ControlError::OutOfRange {
                        control,
                        value: f64::from(*value),
                        min: f64::from(min),
                        max: f64::from(max),
                    })
                }
            }
            (
                ControlKind::Choice {
                    options, unported, ..
                },
                ControlValue::Choice(index),
            ) => match options.get(*index as usize) {
                Some(option) if unported.contains(option) => {
                    Err(ControlError::Unported { control, option })
                }
                Some(_) => Ok(()),
                None => Err(ControlError::NoOption {
                    control,
                    option: index.to_string(),
                }),
            },
            (ControlKind::Bool { .. }, ControlValue::Bool(_)) => Ok(()),
            (ControlKind::Points { .. }, ControlValue::Points(points)) => {
                if points_hold(points) {
                    Ok(())
                } else {
                    Err(ControlError::Points { control })
                }
            }
            (kind, _) => Err(ControlError::WrongKind {
                control,
                expected: match kind {
                    ControlKind::Number { .. } => "a number",
                    ControlKind::Choice { .. } => "one of its options",
                    ControlKind::Bool { .. } => "true or false",
                    ControlKind::Points { .. } => "points",
                },
            }),
        }
    }

    /// Reads a value from text, as edits and the CLI write it: a number, an option by id
    /// or index, `true` or `false`, or points as `x,y` pairs separated by spaces. The
    /// result still needs [`Self::check`].
    pub fn parse(&self, text: &str) -> Result<ControlValue, ControlError> {
        let control = self.id;
        let invalid = || ControlError::Invalid {
            control,
            text: text.to_owned(),
        };
        let text = text.trim();
        match self.kind {
            ControlKind::Number { .. } => text
                .parse()
                .map(ControlValue::Number)
                .map_err(|_| invalid()),
            ControlKind::Choice { options, .. } => options
                .iter()
                .position(|option| option.eq_ignore_ascii_case(text))
                .map(|index| index as u32)
                .or_else(|| text.parse().ok())
                .map(ControlValue::Choice)
                .ok_or_else(|| ControlError::NoOption {
                    control,
                    option: text.to_owned(),
                }),
            ControlKind::Bool { .. } => text.parse().map(ControlValue::Bool).map_err(|_| invalid()),
            ControlKind::Points { .. } => text
                .split_whitespace()
                .map(|pair| {
                    let (x, y) = pair.split_once(',')?;
                    Some([x.parse().ok()?, y.parse().ok()?])
                })
                .collect::<Option<Vec<_>>>()
                .map(ControlValue::Points)
                .ok_or_else(invalid),
        }
    }

    /// The value as text that [`Self::parse`] reads back exactly: a number in its
    /// shortest exact form, an option by its id, `true` or `false`, points as `x,y`
    /// pairs.
    pub fn text(&self, value: &ControlValue) -> String {
        match (self.kind, value) {
            (ControlKind::Choice { options, .. }, ControlValue::Choice(index)) => options
                .get(*index as usize)
                .map_or_else(|| index.to_string(), |option| (*option).to_owned()),
            (_, ControlValue::Number(value)) => value.to_string(),
            (_, ControlValue::Choice(index)) => index.to_string(),
            (_, ControlValue::Bool(value)) => value.to_string(),
            (_, ControlValue::Points(points)) => points
                .iter()
                .map(|[x, y]| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// Whether `value` is one of `values`, as this control writes them ([`Self::text`]):
    /// whether the rows of a [`Row::When`] following this control show.
    pub fn is_one_of(&self, value: &ControlValue, values: &[&str]) -> bool {
        values.contains(&self.text(value).as_str())
    }

    /// A value read from a saved edit, made usable as decision 21 asks: a number or a
    /// point's coordinate outside its range is brought to its nearer end. An option the
    /// control lacks or does not port, a value of the wrong kind, NaN and points out of
    /// order stay refused.
    pub fn clamp(&self, value: ControlValue) -> Result<ControlValue, ControlError> {
        let value = match (self.kind, value) {
            (ControlKind::Number { min, max, .. }, ControlValue::Number(v)) if !v.is_nan() => {
                ControlValue::Number(v.clamp(min, max))
            }
            (ControlKind::Points { .. }, ControlValue::Points(points)) => ControlValue::Points(
                points
                    .into_iter()
                    .map(|point| point.map(|v| if v.is_nan() { v } else { v.clamp(0.0, 1.0) }))
                    .collect(),
            ),
            (_, value) => value,
        };
        self.check(&value).map(|()| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CONTROLS: &[Control] = &[
        Control {
            id: "exposure",
            kind: ControlKind::Number {
                min: -3.0,
                max: 4.0,
                step: 0.01,
                default: 0.0,
            },
            unit: Some(Unit::Ev),
            unported: false,
        },
        Control {
            id: "mode",
            kind: ControlKind::Choice {
                options: &["soft", "hard"],
                default: 0,
                unported: &[],
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "preserve",
            kind: ControlKind::Bool { default: true },
            unit: None,
            unported: false,
        },
        Control {
            id: "curve",
            kind: ControlKind::Points { default: &[] },
            unit: None,
            unported: false,
        },
    ];

    static ROWS: &[Row] = &[
        Row::Slider(Slider::new("exposure")),
        Row::Label { id: "look" },
        Row::Choice {
            control: "mode",
            style: ChoiceStyle::Segmented,
        },
        Row::Toggle {
            control: "preserve",
        },
        Row::Custom {
            widget: Widget::Curve,
            controls: &["curve"],
        },
    ];

    static DEFINITION: Definition = Definition {
        id: "look",
        version: 1,
        layers: true,
        auto: false,
        switched: false,
        controls: CONTROLS,
        tabs: &[Tab {
            id: None,
            rows: ROWS,
        }],
        before: &[],
        after: &[],
        presets: &[],
        views: &[],
        pickers: &[],
    };

    fn control(id: &str) -> &'static Control {
        DEFINITION.control(id).unwrap()
    }

    /// A number from 0 to 1, at 0.
    const fn number(id: &'static str) -> Control {
        Control {
            id,
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        }
    }

    /// A page as darktable draws one, as local contrast and demosaic do: a mode whose
    /// third option is not ported, a parameter under another label in each mode, a
    /// collapsible group with rows shown while a toggle is on, and rows before and after
    /// every tab's.
    static PAGE_CONTROLS: &[Control] = &[
        number("amount"),
        Control {
            id: "mode",
            kind: ControlKind::Choice {
                options: &["bilateral", "laplacian", "neural"],
                default: 0,
                unported: &["neural"],
            },
            unit: None,
            unported: false,
        },
        number("detail"),
        number("contrast"),
        Control {
            id: "smooth",
            kind: ControlKind::Bool { default: false },
            unit: None,
            unported: false,
        },
        number("radius"),
        Control {
            id: "mask",
            kind: ControlKind::Bool { default: false },
            unit: None,
            unported: false,
        },
    ];

    const MODE: Row = Row::Choice {
        control: "mode",
        style: ChoiceStyle::List,
    };

    /// `contrast`'s row, and its row under its other label.
    const CONTRAST: &[Row] = &[Row::Slider(Slider::new("contrast"))];
    const HIGHLIGHTS: &[Row] = &[Row::Slider(Slider::new("contrast").label("highlights"))];

    static PER_MODE: &[Row] = &[
        Row::When {
            control: "mode",
            is: &["bilateral"],
            rows: CONTRAST,
        },
        Row::When {
            control: "mode",
            is: &["laplacian", "neural"],
            rows: HIGHLIGHTS,
        },
    ];

    const FINE_TUNING: Row = Row::Group {
        id: "fine_tuning",
        rows: &[
            Row::Toggle { control: "smooth" },
            Row::When {
                control: "smooth",
                is: &["true"],
                rows: &[Row::Slider(Slider::new("radius"))],
            },
        ],
    };

    static PAGE_TABS: &[Tab] = &[
        Tab {
            id: Some("main"),
            rows: &[
                MODE,
                Row::Slider(Slider::new("detail")),
                Row::When {
                    control: "mode",
                    is: &["bilateral"],
                    rows: CONTRAST,
                },
                Row::When {
                    control: "mode",
                    is: &["laplacian", "neural"],
                    rows: HIGHLIGHTS,
                },
            ],
        },
        Tab {
            id: Some("fine"),
            rows: &[FINE_TUNING],
        },
    ];

    static PAGE: Definition = Definition {
        id: "page",
        version: 1,
        layers: true,
        auto: false,
        switched: false,
        controls: PAGE_CONTROLS,
        tabs: PAGE_TABS,
        before: &[Row::Slider(Slider::new("amount"))],
        after: &[Row::Toggle { control: "mask" }],
        presets: &[],
        views: &[],
        pickers: &[],
    };

    /// [`PAGE`] with its first tab's rows in place of its own.
    fn page(rows: &'static [Row]) -> Definition {
        Definition {
            tabs: Box::leak(Box::new([
                Tab {
                    id: Some("main"),
                    rows,
                },
                Tab {
                    id: Some("fine"),
                    rows: &[FINE_TUNING],
                },
            ])),
            ..PAGE
        }
    }

    /// `rows` with the mode and the rows `contrast` needs around them.
    fn with_mode(rows: &[Row]) -> &'static [Row] {
        let mut all = vec![MODE.clone(), Row::Slider(Slider::new("detail"))];
        all.extend(rows.iter().cloned());
        Box::leak(all.into_boxed_slice())
    }

    #[test]
    fn a_definition_holds_together() {
        DEFINITION.check().unwrap();
        assert_eq!(
            DEFINITION.defaults().get("curve"),
            Some(&ControlValue::Points(Vec::new()))
        );
    }

    #[test]
    fn a_page_as_darktable_draws_it_holds_together() {
        PAGE.check().unwrap();
        // Before every tab's rows, the tabs', then after them, each row before those it
        // holds.
        let shown: Vec<Vec<&str>> = PAGE.rows().map(|row| row.controls().to_vec()).collect();
        assert_eq!(
            shown,
            [
                vec!["amount"],
                vec!["mode"],
                vec!["detail"],
                vec![],
                vec!["contrast"],
                vec![],
                vec!["contrast"],
                vec![],
                vec!["smooth"],
                vec![],
                vec!["radius"],
                vec!["mask"],
            ]
        );
        assert_eq!(page(with_mode(PER_MODE)).check(), Ok(()));
    }

    #[test]
    fn a_control_in_two_rows_is_one_per_mode_never_two_at_once() {
        let twice = |rows: &[Row]| page(with_mode(rows)).check();
        let rows = |n| {
            Err(DefinitionError::Rows {
                control: "contrast",
                rows: n,
            })
        };
        let laplacian = Row::When {
            control: "mode",
            is: &["laplacian", "bilateral"],
            rows: HIGHLIGHTS,
        };
        // Both shown with the bilateral grid.
        assert_eq!(twice(&[PER_MODE[0].clone(), laplacian]), rows(2));
        // One shown in every mode.
        assert_eq!(
            twice(&[PER_MODE[0].clone(), Row::Slider(Slider::new("contrast"))]),
            rows(2)
        );
        // Apart on another control than the mode: shown together when both hold.
        let mask = Row::When {
            control: "mask",
            is: &["true"],
            rows: CONTRAST,
        };
        assert_eq!(twice(&[PER_MODE[0].clone(), mask]), rows(2));
        // Under every tab and in one.
        const UNDER: &[Row] = &[
            Row::Toggle { control: "mask" },
            Row::Slider(Slider::new("detail")),
        ];
        let under = Definition {
            after: UNDER,
            ..PAGE
        };
        assert_eq!(
            under.check(),
            Err(DefinitionError::Rows {
                control: "detail",
                rows: 2
            })
        );
    }

    #[test]
    fn rows_follow_only_a_choice_or_a_toggle_for_values_it_holds() {
        let follow = |control, is| {
            let rows: &'static [Row] = Box::leak(Box::new([Row::When {
                control,
                is,
                rows: CONTRAST,
            }]));
            page(with_mode(rows)).check()
        };
        assert_eq!(
            follow("mode", &["neural"]),
            Ok(()),
            "an unported mode's rows"
        );
        for (control, is) in [
            ("missing", &["true"][..]),
            ("detail", &["0"]),
            ("mode", &["guided"]),
            ("mode", &[]),
            ("smooth", &["on"]),
        ] {
            assert_eq!(
                follow(control, is),
                Err(DefinitionError::Condition(control)),
                "{control} {is:?}"
            );
        }
        // Rows that hide the control they follow.
        const ITSELF: &[Row] = &[MODE, Row::Slider(Slider::new("contrast"))];
        let itself = Row::When {
            control: "mode",
            is: &["bilateral"],
            rows: ITSELF,
        };
        let rows: &'static [Row] =
            Box::leak(Box::new([Row::Slider(Slider::new("detail")), itself]));
        assert_eq!(page(rows).check(), Err(DefinitionError::Condition("mode")));
    }

    #[test]
    fn rows_follow_a_control_on_another_tab_shown_outside_any_group_or_mode() {
        // Contrast on the Fine tab, following the Main tab's mode, as darktable's tone
        // equalizer's contrast compensation follows its filter.
        let two_tabs = |main: &'static [Row], fine: &'static [Row], after: &'static [Row]| {
            Definition {
                tabs: Box::leak(Box::new([
                    Tab {
                        id: Some("main"),
                        rows: main,
                    },
                    Tab {
                        id: Some("fine"),
                        rows: fine,
                    },
                ])),
                after,
                ..PAGE
            }
            .check()
        };
        const FOLLOWING_MODE: &[Row] = &[
            FINE_TUNING,
            Row::When {
                control: "mode",
                is: &["bilateral"],
                rows: CONTRAST,
            },
        ];
        const MASK: &[Row] = &[Row::Toggle { control: "mask" }];
        assert_eq!(two_tabs(with_mode(&[]), FOLLOWING_MODE, MASK), Ok(()));
        // The mask's toggle in a mode of the Main tab: rows on the Fine tab cannot follow
        // it, as it may be hidden there.
        let main = with_mode(&[
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: MASK,
            },
            Row::When {
                control: "mode",
                is: &["bilateral"],
                rows: CONTRAST,
            },
        ]);
        const FOLLOWING_MASK: &[Row] = &[
            FINE_TUNING,
            Row::When {
                control: "mask",
                is: &["true"],
                rows: &[Row::Label { id: "masked" }],
            },
        ];
        assert_eq!(
            two_tabs(main, FOLLOWING_MASK, &[]),
            Err(DefinitionError::Hidden("mask"))
        );
    }

    #[test]
    fn rows_follow_a_control_only_where_one_of_its_rows_is_sure_to_show() {
        // The smooth toggle is in the Fine tab's group: rows on the Main tab, or in that
        // tab outside the group, would show while it is out of sight.
        assert_eq!(
            page(with_mode(&[Row::When {
                control: "smooth",
                is: &["true"],
                rows: CONTRAST,
            }]))
            .check(),
            Err(DefinitionError::Hidden("smooth"))
        );
        const OUTSIDE: &[Row] = &[
            FINE_TUNING,
            Row::When {
                control: "smooth",
                is: &["true"],
                rows: CONTRAST,
            },
        ];
        let outside = Definition {
            tabs: Box::leak(Box::new([
                Tab {
                    id: Some("main"),
                    rows: with_mode(&[]),
                },
                Tab {
                    id: Some("fine"),
                    rows: OUTSIDE,
                },
            ])),
            ..PAGE
        };
        assert_eq!(outside.check(), Err(DefinitionError::Hidden("smooth")));
        // The mask's toggle shown only with the local Laplacian filter: rows that follow
        // it must be in that mode too, for those values or fewer.
        const IN_MODE: &[Row] = &[Row::Toggle { control: "mask" }];
        let follow_mask = |is: &'static [&'static str]| {
            let following: &'static [Row] = Box::leak(Box::new([Row::When {
                control: "mask",
                is: &["true"],
                rows: CONTRAST,
            }]));
            let rows = [
                Row::When {
                    control: "mode",
                    is: &["laplacian"],
                    rows: IN_MODE,
                },
                Row::When {
                    control: "mode",
                    is,
                    rows: following,
                },
            ];
            Definition {
                after: &[],
                ..page(with_mode(&rows))
            }
            .check()
        };
        assert_eq!(follow_mask(&["laplacian"]), Ok(()));
        assert_eq!(
            follow_mask(&["laplacian", "bilateral"]),
            Err(DefinitionError::Hidden("mask"))
        );
        assert_eq!(
            follow_mask(&["bilateral"]),
            Err(DefinitionError::Hidden("mask"))
        );
    }

    #[test]
    fn a_mode_holds_rows_and_a_group_may_have_none_yet() {
        // A group with nothing behind it yet, as AgX's Show curve before its graph.
        const EMPTY_GROUP: &[Row] = &[
            Row::Slider(Slider::new("contrast")),
            Row::Group {
                id: "empty",
                rows: &[],
            },
        ];
        assert_eq!(page(with_mode(EMPTY_GROUP)).check(), Ok(()));
        const EMPTY_MODE: &[Row] = &[
            Row::Slider(Slider::new("contrast")),
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: &[],
            },
        ];
        assert_eq!(
            page(with_mode(EMPTY_MODE)).check(),
            Err(DefinitionError::Empty("mode"))
        );
    }

    #[test]
    fn a_group_holds_no_group_and_groups_and_labels_have_ids_of_their_own() {
        let inner = Row::Group {
            id: "inner",
            rows: CONTRAST,
        };
        let outer = Row::Group {
            id: "outer",
            rows: Box::leak(Box::new([Row::When {
                control: "mode",
                is: &["bilateral"],
                rows: Box::leak(Box::new([inner])),
            }])),
        };
        assert_eq!(
            page(with_mode(&[outer])).check(),
            Err(DefinitionError::GroupInGroup("inner"))
        );
        let again = Row::Group {
            id: "fine_tuning",
            rows: CONTRAST,
        };
        assert_eq!(
            page(with_mode(&[again])).check(),
            Err(DefinitionError::DuplicateGroup("fine_tuning"))
        );
        // A label may share a group's id: their messages differ.
        let labels = [
            Row::Label { id: "fine_tuning" },
            Row::Slider(Slider::new("contrast")),
        ];
        assert_eq!(page(with_mode(&labels)).check(), Ok(()));
        let twice = [
            Row::Label { id: "look" },
            Row::Slider(Slider::new("contrast")),
            Row::Label { id: "look" },
        ];
        assert_eq!(
            page(with_mode(&twice)).check(),
            Err(DefinitionError::DuplicateLabel("look"))
        );
        let words = [Row::Slider(Slider::new("contrast").label("High lights"))];
        assert_eq!(
            page(with_mode(&words)).check(),
            Err(DefinitionError::Id("High lights"))
        );
    }

    #[test]
    fn a_slider_s_soft_range_lies_in_order_inside_its_range() {
        // `contrast` holds 0 to 1.
        let soft = |min, max| {
            let rows: &'static [Row] = Box::leak(Box::new([Row::Slider(
                Slider::new("contrast").soft(min, max),
            )]));
            page(with_mode(rows)).check()
        };
        assert_eq!(soft(0.0, 0.5), Ok(()));
        assert_eq!(soft(0.0, 1.0), Ok(()), "the whole range");
        for (min, max) in [
            (0.5, 0.5),
            (0.5, 0.2),
            (-0.1, 0.5),
            (0.5, 1.5),
            (0.0, f32::NAN),
        ] {
            assert_eq!(
                soft(min, max),
                Err(DefinitionError::SoftRange("contrast")),
                "{min} to {max}"
            );
        }
        // Its default may lie outside it, as darktable's may.
        assert_eq!(soft(0.2, 0.5), Ok(()));
        // A coordinate's slider is held to the coordinate's range, −1 to 2.
        let total = |min, max| Row::Slider(Slider::new("total").soft(min, max));
        assert_eq!(viewed(tone(), &[AMOUNT, total(-0.5, 1.5)]), Ok(()));
        assert_eq!(
            viewed(tone(), &[AMOUNT, total(0.0, 3.0)]),
            Err(DefinitionError::SoftRange("total"))
        );
    }

    /// A tool's setting, darktable's chart for its colour checker.
    const CHART: ToolSetting = ToolSetting {
        id: "chart",
        kind: ControlKind::Choice {
            options: &["classic", "passport"],
            default: 0,
            unported: &[],
        },
        decimals: 0,
    };

    #[test]
    fn a_tool_s_settings_are_rows_of_no_control_each_once_and_whole() {
        // In a group of their own, as darktable's colour checker: a group of settings
        // alone holds, and the settings show no control.
        let checker = Row::Group {
            id: "checker",
            rows: Box::leak(Box::new([
                Row::ToolSetting(CHART),
                Row::ToolSetting(ToolSetting {
                    id: "patch_scale",
                    kind: ControlKind::Number {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                        default: 0.5,
                    },
                    decimals: 3,
                }),
            ])),
        };
        let rows = [Row::Slider(Slider::new("contrast")), checker];
        assert_eq!(page(with_mode(&rows)).check(), Ok(()));
        assert!(Row::ToolSetting(CHART).controls().is_empty());
        let twice = [
            Row::Slider(Slider::new("contrast")),
            Row::ToolSetting(CHART),
            Row::ToolSetting(CHART),
        ];
        assert_eq!(
            page(with_mode(&twice)).check(),
            Err(DefinitionError::DuplicateSetting("chart"))
        );
        let setting = |setting| {
            let rows = [
                Row::Slider(Slider::new("contrast")),
                Row::ToolSetting(setting),
            ];
            page(with_mode(&rows)).check()
        };
        let named = |id| ToolSetting { id, ..CHART };
        assert_eq!(setting(named("Chart")), Err(DefinitionError::Id("Chart")));
        let default_outside = ToolSetting {
            kind: ControlKind::Choice {
                options: &["classic"],
                default: 1,
                unported: &[],
            },
            ..CHART
        };
        assert_eq!(
            setting(default_outside),
            Err(DefinitionError::Options("chart"))
        );
        let points = ToolSetting {
            kind: ControlKind::Points { default: &[] },
            ..CHART
        };
        assert_eq!(setting(points), Err(DefinitionError::Row("chart")));
    }

    #[test]
    fn an_action_is_a_row_of_no_control_each_once() {
        // In a mode, as AgX's Set from above shows only while adjustments are on.
        let rows = [
            Row::Slider(Slider::new("contrast")),
            Row::Action { id: "auto_tune" },
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: &[Row::Action {
                    id: "set_from_above",
                }],
            },
        ];
        assert_eq!(page(with_mode(&rows)).check(), Ok(()));
        assert!(Row::Action { id: "auto_tune" }.controls().is_empty());
        // A row of buttons shows its control, and its buttons' ids are actions'.
        let button = |id| Button {
            id,
            icon: ButtonIcon::RotateLeft,
        };
        let buttons = Row::Buttons {
            control: "contrast",
            buttons: Box::leak(Box::new([button("auto_tune")])),
        };
        assert_eq!(buttons.controls(), ["contrast"]);
        let clash = [buttons, Row::Action { id: "auto_tune" }];
        assert_eq!(
            page(with_mode(&clash)).check(),
            Err(DefinitionError::DuplicateAction("auto_tune"))
        );
        let twice = [
            Row::Slider(Slider::new("contrast")),
            Row::Action { id: "auto_tune" },
            Row::Action { id: "auto_tune" },
        ];
        assert_eq!(
            page(with_mode(&twice)).check(),
            Err(DefinitionError::DuplicateAction("auto_tune"))
        );
        let named = [
            Row::Slider(Slider::new("contrast")),
            Row::Action { id: "Auto tune" },
        ];
        assert_eq!(
            page(with_mode(&named)).check(),
            Err(DefinitionError::Id("Auto tune"))
        );
    }

    /// A picker writing `contrast` and `detail` and reading the chart, as Exposure's
    /// writes its exposure and black and reads its area mode and target.
    const LEVEL: Picker = Picker {
        id: "level",
        operation: "bilat",
        kind: PickerKind::Area,
        denoise: false,
        output: false,
        space: Space::Operation,
        keep: false,
        writes: &["contrast", "detail"],
        tools: &["chart"],
    };

    /// [`PAGE`] with `pickers`, the chart's setting and `rows` on its first tab.
    fn picking(pickers: &[Picker], rows: &[Row]) -> Result<(), DefinitionError> {
        let mut all = vec![Row::ToolSetting(CHART)];
        all.extend(rows.iter().cloned());
        Definition {
            pickers: Box::leak(pickers.into()),
            ..page(with_mode(&all))
        }
        .check()
    }

    #[test]
    fn a_picker_is_in_exactly_one_row_a_quad_or_a_row_of_pickers() {
        const CONTRAST: Row = Row::Slider(Slider::new("contrast"));
        const QUAD: Row = Row::Slider(Slider::new("contrast").quad(Quad::Picker("level")));
        assert_eq!(QUAD.pickers(), ["level"]);
        assert_eq!(picking(&[LEVEL], &[QUAD]), Ok(()));
        let buttons = |label, pickers: &[&'static str]| Row::Pickers {
            label,
            pickers: Box::leak(pickers.into()),
        };
        // A row of pickers shows no control, with a label or none.
        let row = buttons(Some("auto_tune"), &["level"]);
        assert!(row.controls().is_empty());
        assert_eq!(row.pickers(), ["level"]);
        assert_eq!(picking(&[LEVEL], &[CONTRAST, row.clone()]), Ok(()));
        let unlabelled = buttons(None, &["level", "spot"]);
        let spot = Picker {
            id: "spot",
            ..LEVEL
        };
        assert_eq!(picking(&[LEVEL, spot], &[CONTRAST, unlabelled]), Ok(()));
        // Pickers are a namespace of their own: one takes its quad's control's id.
        let contrast = Picker {
            id: "contrast",
            ..LEVEL
        };
        let own = Row::Slider(Slider::new("contrast").quad(Quad::Picker("contrast")));
        assert_eq!(picking(&[contrast], &[own]), Ok(()));
        let rows = |rows| {
            Err(DefinitionError::PickerRows {
                picker: "level",
                rows,
            })
        };
        assert_eq!(picking(&[LEVEL], &[CONTRAST]), rows(0));
        assert_eq!(picking(&[LEVEL], &[QUAD, row.clone()]), rows(2));
        let twice = buttons(None, &["level", "level"]);
        assert_eq!(picking(&[LEVEL], &[CONTRAST, twice]), rows(2));
        assert_eq!(
            picking(&[], &[QUAD]),
            Err(DefinitionError::UnknownPicker("level"))
        );
        assert_eq!(
            picking(&[LEVEL], &[QUAD, buttons(None, &[])]),
            Err(DefinitionError::NoPickers)
        );
        assert_eq!(
            picking(&[LEVEL], &[CONTRAST, buttons(Some("Auto"), &["level"])]),
            Err(DefinitionError::Id("Auto"))
        );
        // Its label is a group label, sharing its words, so never one's id twice.
        let label = Row::Label { id: "auto_tune" };
        assert_eq!(
            picking(&[LEVEL], &[CONTRAST, label, row]),
            Err(DefinitionError::DuplicateLabel("auto_tune"))
        );
    }

    #[test]
    fn a_picker_writes_the_module_s_controls_and_reads_its_tool_settings_each_once() {
        let quad = |picker| Row::Slider(Slider::new("contrast").quad(Quad::Picker(picker)));
        let one = |picker: Picker| picking(&[picker], &[quad(picker.id)]);
        // One that only shows writes nothing and reads nothing.
        let shows = Picker {
            writes: &[],
            tools: &[],
            ..LEVEL
        };
        assert_eq!(one(shows), Ok(()));
        let named = Picker {
            id: "Level",
            ..LEVEL
        };
        assert_eq!(one(named), Err(DefinitionError::Id("Level")));
        assert_eq!(
            picking(&[LEVEL, LEVEL], &[quad("level")]),
            Err(DefinitionError::DuplicatePicker("level"))
        );
        let writes = |writes| one(Picker { writes, ..LEVEL });
        let wrong = Err(DefinitionError::PickerWrites("level"));
        assert_eq!(writes(&["gamma"]), wrong);
        assert_eq!(writes(&["contrast", "contrast"]), wrong);
        let tools = |tools| one(Picker { tools, ..LEVEL });
        let wrong = Err(DefinitionError::PickerTools("level"));
        // A tool setting is a row's, never a control.
        assert_eq!(tools(&["mode"]), wrong);
        assert_eq!(tools(&["chart", "chart"]), wrong);
    }

    #[test]
    fn a_preset_row_writes_controls_the_module_has_and_is_in_one_row_at_most() {
        const AMOUNT: Row = Row::Slider(Slider::new("amount"));
        const LOOK: Row = Row::Preset {
            preset: "look",
            style: ChoiceStyle::List,
        };
        let with = |presets: &'static [PresetRow], before: &'static [Row]| {
            Definition {
                before,
                presets,
                ..PAGE
            }
            .check()
        };
        let look = |options: &'static [&'static str], controls: &'static [&'static str]| {
            let preset = PresetRow {
                id: "look",
                options,
                controls,
            };
            with(Box::leak(Box::new([preset])), &[LOOK, AMOUNT])
        };
        assert_eq!(
            look(&["soft", "crisp"], &["amount", "detail", "mode"]),
            Ok(())
        );
        // In no row, as darktable's presets in the presets menu will be (Phase 3).
        let presets = &[PresetRow {
            id: "look",
            options: &["soft"],
            controls: &["amount"],
        }];
        assert_eq!(with(presets, &[AMOUNT]), Ok(()));
        // A control in no row of its own shows through a preset row that writes it, once
        // the row is placed, as White balance's `preset` shows through Settings.
        assert_eq!(with(presets, &[LOOK]), Ok(()));
        assert_eq!(
            with(presets, &[]),
            Err(DefinitionError::Rows {
                control: "amount",
                rows: 0
            })
        );
        assert_eq!(
            with(presets, &[LOOK, AMOUNT, LOOK]),
            Err(DefinitionError::PresetRows {
                preset: "look",
                rows: 2
            })
        );
        assert_eq!(
            with(&[], &[LOOK, AMOUNT]),
            Err(DefinitionError::UnknownPreset("look"))
        );
        // A list: segments draw neither Custom nor an option the photo cannot have.
        let segmented = Row::Preset {
            preset: "look",
            style: ChoiceStyle::Segmented,
        };
        assert_eq!(
            with(presets, Box::leak(Box::new([segmented, AMOUNT]))),
            Err(DefinitionError::SegmentedPreset("look"))
        );
        // Its controls are the module's, each once.
        for controls in [&[][..], &["amount", "missing"], &["amount", "amount"]] {
            assert_eq!(
                look(&["soft"], controls),
                Err(DefinitionError::PresetControls("look")),
                "{controls:?}"
            );
        }
        for options in [&[][..], &["soft", "soft"], &["Soft"]] {
            assert_eq!(
                look(options, &["amount"]),
                Err(DefinitionError::PresetOptions("look")),
                "{options:?}"
            );
        }
        // Its id is unique together with the controls'.
        let named = |id| PresetRow {
            id,
            options: &["soft"],
            controls: &["amount"],
        };
        let twice = Box::leak(Box::new([named("look"), named("look")]));
        assert_eq!(
            with(twice, &[AMOUNT]),
            Err(DefinitionError::DuplicatePreset("look"))
        );
        let detail = Box::leak(Box::new([named("detail")]));
        assert_eq!(
            with(detail, &[AMOUNT]),
            Err(DefinitionError::DuplicatePreset("detail"))
        );
        let words = Box::leak(Box::new([named("Look")]));
        assert_eq!(with(words, &[AMOUNT]), Err(DefinitionError::Id("Look")));
    }

    /// A coordinate from −1 to 2.
    const fn coordinate(id: &'static str) -> Coordinate {
        Coordinate {
            id,
            min: -1.0,
            max: 2.0,
            step: 0.01,
            unit: None,
        }
    }

    /// A view of detail and contrast, as their total and their balance.
    static TONE: View = View {
        id: "tone",
        controls: &["detail", "contrast"],
        coordinates: &[coordinate("total"), coordinate("balance")],
    };

    const AMOUNT: Row = Row::Slider(Slider::new("amount"));
    const TOTAL: Row = Row::Slider(Slider::new("total"));
    const BALANCE: Row = Row::Slider(Slider::new("balance"));

    /// [`PAGE`] with these views and these rows before every tab's.
    fn viewed(views: Vec<View>, before: &[Row]) -> Result<(), DefinitionError> {
        Definition {
            views: Box::leak(views.into_boxed_slice()),
            before: Box::leak(before.to_vec().into_boxed_slice()),
            ..PAGE
        }
        .check()
    }

    fn tone() -> Vec<View> {
        vec![View { ..TONE }]
    }

    #[test]
    fn a_view_covers_number_controls_each_once_with_a_coordinate_for_each_or_fewer() {
        assert_eq!(viewed(tone(), &[AMOUNT, TOTAL, BALANCE]), Ok(()));
        // Fewer coordinates than controls, a model of them that drops a scale, as White
        // balance's temperature and tint of three coefficients (decision 27); none, never.
        let model = View {
            controls: &["detail", "contrast", "amount"],
            ..TONE
        };
        assert_eq!(viewed(vec![model], &[AMOUNT, TOTAL, BALANCE]), Ok(()));
        let none = View {
            coordinates: &[],
            ..TONE
        };
        assert_eq!(
            viewed(vec![none], &[AMOUNT]),
            Err(DefinitionError::ViewCoordinates("tone"))
        );
        for controls in [
            &[][..],
            &["detail", "detail"],
            &["detail", "mode"],
            &["detail", "missing"],
        ] {
            let view = View { controls, ..TONE };
            assert_eq!(
                viewed(vec![view], &[AMOUNT]),
                Err(DefinitionError::ViewControls("tone")),
                "{controls:?}"
            );
        }
        let one = View {
            controls: &["detail"],
            ..TONE
        };
        assert_eq!(
            viewed(vec![one], &[AMOUNT]),
            Err(DefinitionError::ViewCoordinates("tone"))
        );
        // Views have ids of their own.
        let named = |id| View { id, ..TONE };
        assert_eq!(
            viewed(vec![named("Tone")], &[AMOUNT]),
            Err(DefinitionError::Id("Tone"))
        );
        let other = View {
            coordinates: Box::leak(Box::new([coordinate("lift"), coordinate("gain")])),
            ..TONE
        };
        assert_eq!(
            viewed(vec![named("tone"), other], &[AMOUNT]),
            Err(DefinitionError::DuplicateView("tone"))
        );
    }

    #[test]
    fn a_coordinate_has_an_id_of_its_own_a_range_and_one_slider_row_at_most() {
        // In no row, as a view a panel does not draw yet.
        assert_eq!(viewed(tone(), &[AMOUNT]), Ok(()));
        assert_eq!(
            viewed(tone(), &[AMOUNT, TOTAL, BALANCE, TOTAL]),
            Err(DefinitionError::CoordinateRows {
                coordinate: "total",
                rows: 2
            })
        );
        let toggle = Row::Toggle { control: "total" };
        assert_eq!(
            viewed(tone(), &[AMOUNT, toggle]),
            Err(DefinitionError::Row("total"))
        );
        // Unique among the controls, the preset rows and the other coordinates.
        let with = |coordinates: [Coordinate; 2]| {
            Definition {
                views: Box::leak(Box::new([View {
                    coordinates: Box::leak(Box::new(coordinates)),
                    ..TONE
                }])),
                presets: &[PresetRow {
                    id: "look",
                    options: &["soft"],
                    controls: &["amount"],
                }],
                ..PAGE
            }
            .check()
        };
        for (coordinates, twice) in [
            ([coordinate("detail"), coordinate("balance")], "detail"),
            ([coordinate("look"), coordinate("balance")], "look"),
            ([coordinate("total"), coordinate("total")], "total"),
        ] {
            assert_eq!(
                with(coordinates),
                Err(DefinitionError::DuplicateCoordinate(twice))
            );
        }
        assert_eq!(
            with([coordinate("Total"), coordinate("balance")]),
            Err(DefinitionError::Id("Total"))
        );
        let backwards = Coordinate {
            min: 2.0,
            max: -1.0,
            ..coordinate("total")
        };
        assert_eq!(
            with([backwards, coordinate("balance")]),
            Err(DefinitionError::Range("total"))
        );
    }

    #[test]
    fn a_control_in_no_row_of_its_own_shows_through_a_view_covering_it() {
        let percent = || View {
            id: "percent",
            controls: &["amount"],
            coordinates: Box::leak(Box::new([coordinate("amount_percent")])),
        };
        let shown = Row::Slider(Slider::new("amount_percent"));
        assert_eq!(viewed(vec![percent()], &[shown]), Ok(()));
        // Its view's coordinate in no row, or only a view not covering it in one: it shows
        // nowhere.
        let nowhere = Err(DefinitionError::Rows {
            control: "amount",
            rows: 0,
        });
        assert_eq!(viewed(vec![percent()], &[]), nowhere);
        assert_eq!(viewed(tone(), &[TOTAL]), nowhere);
    }

    #[test]
    fn two_views_may_cover_the_same_controls() {
        // As White balance shows its light, x and y, as temperature and tint and as hue
        // and chroma.
        let lift = || View {
            id: "lift",
            coordinates: Box::leak(Box::new([coordinate("lift"), coordinate("gain")])),
            ..TONE
        };
        let rows = [
            AMOUNT,
            TOTAL,
            BALANCE,
            Row::Slider(Slider::new("lift")),
            Row::Slider(Slider::new("gain")),
        ];
        assert_eq!(viewed(vec![View { ..TONE }, lift()], &rows), Ok(()));
        // A control in no row of its own, covered by two views, one of them in no row.
        let percent = |id, named| View {
            id,
            controls: &["amount"],
            coordinates: Box::leak(Box::new([coordinate(named)])),
        };
        let views = vec![percent("hidden", "fraction"), percent("shown", "percent")];
        let shown = Row::Slider(Slider::new("percent"));
        assert_eq!(viewed(views, &[shown]), Ok(()));
    }

    #[test]
    fn a_coordinate_takes_numbers_in_its_range() {
        let total = coordinate("total");
        assert_eq!(total.check(&ControlValue::Number(2.0)), Ok(2.0));
        assert!(matches!(
            total.check(&ControlValue::Number(2.01)),
            Err(ControlError::OutOfRange {
                control: "total",
                ..
            })
        ));
        assert!(total.check(&ControlValue::Number(f32::NAN)).is_err());
        assert!(matches!(
            total.check(&ControlValue::Bool(true)),
            Err(ControlError::WrongKind {
                control: "total",
                ..
            })
        ));
    }

    #[test]
    fn an_unported_option_is_never_the_default_and_never_segments() {
        let mode = |default, unported| {
            let controls: Vec<Control> = PAGE_CONTROLS
                .iter()
                .map(|control| match control.kind {
                    ControlKind::Choice { options, .. } => Control {
                        kind: ControlKind::Choice {
                            options,
                            default,
                            unported,
                        },
                        ..*control
                    },
                    _ => Control { ..*control },
                })
                .collect();
            Definition {
                controls: Box::leak(controls.into_boxed_slice()),
                ..PAGE
            }
            .check()
        };
        assert_eq!(mode(1, &["neural"]), Ok(()));
        assert_eq!(mode(2, &["neural"]), Err(DefinitionError::Unported("mode")));
        assert_eq!(mode(0, &["guided"]), Err(DefinitionError::Unported("mode")));
        let segments = [Row::Choice {
            control: "mode",
            style: ChoiceStyle::Segmented,
        }];
        let rows: Vec<Row> = segments
            .into_iter()
            .chain(with_mode(PER_MODE)[1..].iter().cloned())
            .collect();
        assert_eq!(
            page(Box::leak(rows.into_boxed_slice())).check(),
            Err(DefinitionError::Segmented("mode"))
        );
    }

    #[test]
    fn an_unported_option_is_refused_as_a_value() {
        let mode = PAGE.control("mode").unwrap();
        assert_eq!(mode.check(&ControlValue::Choice(1)), Ok(()));
        let unported = ControlError::Unported {
            control: "mode",
            option: "neural",
        };
        assert_eq!(mode.check(&ControlValue::Choice(2)), Err(unported.clone()));
        // Whether typed, as the CLI takes it, or read from an edit.
        let typed = mode.parse("neural").unwrap();
        assert_eq!(mode.check(&typed), Err(unported.clone()));
        assert_eq!(mode.clamp(ControlValue::Choice(2)), Err(unported));
    }

    #[test]
    fn a_control_not_ported_takes_only_its_default() {
        let unported = |kind| Control {
            id: "hold",
            kind,
            unit: None,
            unported: true,
        };
        let refused = ControlError::NotPorted { control: "hold" };
        let not_ported = Err(refused.clone());
        let level = unported(ControlKind::Number {
            min: -18.0,
            max: 18.0,
            step: 0.1,
            default: -4.0,
        });
        assert_eq!(level.check(&ControlValue::Number(-4.0)), Ok(()));
        assert_eq!(level.check(&ControlValue::Number(-3.9)), not_ported);
        // Whether typed, as the CLI takes it, or read from an edit, clamped first.
        assert_eq!(level.check(&level.parse("0").unwrap()), not_ported);
        assert_eq!(level.clamp(ControlValue::Number(40.0)), Err(refused));
        // Its kind still comes first.
        assert!(matches!(
            level.check(&ControlValue::Bool(true)),
            Err(ControlError::WrongKind { .. })
        ));
        let toggle = unported(ControlKind::Bool { default: false });
        assert_eq!(toggle.check(&ControlValue::Bool(false)), Ok(()));
        assert_eq!(toggle.check(&ControlValue::Bool(true)), not_ported);
    }

    #[test]
    fn a_mode_s_rows_show_for_the_values_it_names() {
        let mode = PAGE.control("mode").unwrap();
        let laplacian = ControlValue::Choice(1);
        assert!(mode.is_one_of(&laplacian, &["laplacian", "neural"]));
        assert!(!mode.is_one_of(&laplacian, &["bilateral"]));
        let smooth = PAGE.control("smooth").unwrap();
        assert!(smooth.is_one_of(&ControlValue::Bool(true), &["true"]));
        assert!(!smooth.is_one_of(&ControlValue::Bool(false), &["true"]));
    }

    #[test]
    fn a_control_in_no_row_or_two_or_the_wrong_one_is_refused() {
        static TWICE: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Slider(Slider::new("exposure")),
        ];
        static WRONG: &[Row] = &[Row::Toggle {
            control: "exposure",
        }];
        let with = |rows: &'static [Row]| Definition {
            id: "look",
            version: 1,
            layers: true,
            auto: false,
            switched: false,
            controls: &CONTROLS[..1],
            tabs: Box::leak(Box::new([Tab { id: None, rows }])),
            before: &[],
            after: &[],
            presets: &[],
            views: &[],
            pickers: &[],
        };
        assert_eq!(
            with(TWICE).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 2
            })
        );
        assert_eq!(
            with(&[]).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 0
            })
        );
        assert_eq!(with(WRONG).check(), Err(DefinitionError::Row("exposure")));
        assert_eq!(with(&ROWS[..1]).check(), Ok(()), "one control, one slider");
    }

    #[test]
    fn only_the_equalizer_is_a_second_view_of_number_controls() {
        // As the tone equalizer's graph over its bands' sliders: one equalizer besides
        // a control's own row, never two; or the widget alone; and only over numbers.
        static VIEWED: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["exposure"],
            },
        ];
        static ALONE: &[Row] = &[Row::Custom {
            widget: Widget::Equalizer,
            controls: &["exposure"],
        }];
        static TWICE: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["exposure"],
            },
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["exposure"],
            },
        ];
        static OF_A_CHOICE: &[Row] = &[
            Row::Choice {
                control: "mode",
                style: ChoiceStyle::List,
            },
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["mode"],
            },
        ];
        let with = |controls: &'static [Control], rows: &'static [Row]| Definition {
            id: "look",
            version: 1,
            layers: true,
            auto: false,
            switched: false,
            controls,
            tabs: Box::leak(Box::new([Tab { id: None, rows }])),
            before: &[],
            after: &[],
            presets: &[],
            views: &[],
            pickers: &[],
        };
        assert_eq!(with(&CONTROLS[..1], VIEWED).check(), Ok(()));
        assert_eq!(with(&CONTROLS[..1], ALONE).check(), Ok(()));
        assert_eq!(
            with(&CONTROLS[..1], TWICE).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 3
            })
        );
        assert_eq!(
            with(&CONTROLS[1..2], OF_A_CHOICE).check(),
            Err(DefinitionError::Row("mode"))
        );
        // No other widget is a second row: a curve beside its control's own row is two.
        static CURVE_TOO: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::Curve,
                controls: &["exposure"],
            },
        ];
        assert_eq!(
            with(&CONTROLS[..1], CURVE_TOO).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 2
            })
        );
    }

    #[test]
    fn ids_are_snake_case() {
        for id in ["exposure", "white_balance", "zone_3"] {
            assert!(is_id(id), "{id}");
        }
        for id in ["", "Exposure", "white-balance", "_x", "x_", "a__b", "3d"] {
            assert!(!is_id(id), "{id}");
        }
    }

    #[test]
    fn values_out_of_range_or_of_another_kind_are_refused() {
        let exposure = control("exposure");
        assert!(exposure.check(&ControlValue::Number(4.0)).is_ok());
        assert!(matches!(
            exposure.check(&ControlValue::Number(4.01)),
            Err(ControlError::OutOfRange { .. })
        ));
        assert!(exposure.check(&ControlValue::Number(f32::NAN)).is_err());
        assert!(matches!(
            exposure.check(&ControlValue::Bool(true)),
            Err(ControlError::WrongKind { .. })
        ));
        assert!(control("mode").check(&ControlValue::Choice(2)).is_err());
        let curve = control("curve");
        let full: Vec<[f32; 2]> = (1..=MAX_POINTS).map(|k| [k as f32 * 0.05, 0.5]).collect();
        let mut over = full.clone();
        over.push([0.97, 0.5]);
        // The ends are the curve's: a point at either, or closer to one than the gap.
        for points in [
            vec![[0.0, 0.0]],
            vec![[1.0, 1.0]],
            vec![[0.002, 0.5]],
            vec![[0.998, 0.5]],
            vec![[0.5, 0.0], [0.4, 1.0]],
            vec![[0.5, 1.5]],
            vec![[0.5, 0.5], [0.502, 0.5]],
            over,
        ] {
            assert!(
                curve.check(&ControlValue::Points(points.clone())).is_err(),
                "{points:?}"
            );
        }
        for points in [Vec::new(), vec![[0.0025, 0.0]], vec![[0.5, 0.5]], full] {
            assert!(
                curve.check(&ControlValue::Points(points.clone())).is_ok(),
                "{points:?}"
            );
        }
    }

    #[test]
    fn text_reads_back_exactly() {
        for (id, value) in [
            ("exposure", ControlValue::Number(0.1 + 0.2)),
            ("exposure", ControlValue::Number(-2.999_999_8)),
            ("mode", ControlValue::Choice(1)),
            ("preserve", ControlValue::Bool(false)),
            (
                "curve",
                ControlValue::Points(vec![[0.25, 0.2], [0.75, 0.8]]),
            ),
            ("curve", ControlValue::Points(Vec::new())),
        ] {
            let control = control(id);
            let text = control.text(&value);
            assert_eq!(control.parse(&text), Ok(value), "{id}: {text}");
        }
        assert_eq!(control("mode").text(&ControlValue::Choice(1)), "hard");
        assert_eq!(control("mode").parse("1"), Ok(ControlValue::Choice(1)));
        assert_eq!(
            control("curve").text(&ControlValue::Points(vec![[0.0, 0.0], [0.5, 0.25]])),
            "0,0 0.5,0.25"
        );
        assert!(control("curve").parse("0,0 0.5").is_err());
        assert!(control("mode").parse("medium").is_err());
    }

    #[test]
    fn a_saved_value_out_of_range_is_clamped() {
        let exposure = control("exposure");
        assert_eq!(
            exposure.clamp(ControlValue::Number(9.0)),
            Ok(ControlValue::Number(4.0))
        );
        assert!(exposure.clamp(ControlValue::Number(f32::NAN)).is_err());
        assert_eq!(
            control("curve").clamp(ControlValue::Points(vec![[0.25, -0.1], [0.75, 1.2]])),
            Ok(ControlValue::Points(vec![[0.25, 0.0], [0.75, 1.0]]))
        );
        // A point brought onto an end is still refused.
        assert!(
            control("curve")
                .clamp(ControlValue::Points(vec![[-0.1, 0.0]]))
                .is_err()
        );
        assert!(control("mode").clamp(ControlValue::Choice(5)).is_err());
    }
}
