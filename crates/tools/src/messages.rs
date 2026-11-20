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

//! A tool's words, as message ids that follow from its ids (decision 28, `docs/gui.md`,
//! Tool panels from definitions). Hyphens separate the parts, ids are snake_case, and a
//! fixed word comes before each id, so an option or a tab called `label` or `title`
//! cannot collide with another message. The messages are in
//! `crates/ui/locales/<locale>/tools.ftl`. A utility may declare words of its own for its
//! titles and settings, as Export's `export-*` in `ui.ftl` (decision 93).

use std::collections::BTreeSet;

use crate::definition::{
    ControlKind, GroupKind, ProcessingDefinition, Reset, Row, SettingRow, Slider, Unit, Word,
};
use crate::utility::UtilityDefinition;

/// The tool's title: `tool-<tool>-title`.
pub fn title(tool: &str) -> String {
    format!("tool-{tool}-title")
}

/// What the tool does and when to reach for it, a sentence or two, first in its header's
/// tooltip: `tool-<tool>-description`. A utility's, which has no definition, is the same
/// under its id ([`crate::utility::UTILITIES`]).
pub fn description(tool: &str) -> String {
    format!("tool-{tool}-description")
}

/// One of darktable's input, process and output lines ([`crate::about::Fact`]):
/// `tool-fact-` and darktable's words, lower case, each run of anything but letters and
/// digits a hyphen, so "linear, RGB, scene-referred" is
/// `tool-fact-linear-rgb-scene-referred`.
pub fn fact(darktable: &str) -> String {
    let mut id = String::from("tool-fact");
    let mut gap = true;
    for c in darktable.chars() {
        if c.is_ascii_alphanumeric() {
            if gap {
                id.push('-');
            }
            id.push(c.to_ascii_lowercase());
            gap = false;
        } else {
            gap = true;
        }
    }
    id
}

/// Auto's tooltip, which names its target: `tool-<tool>-auto`.
pub fn auto(tool: &str) -> String {
    format!("tool-{tool}-auto")
}

/// A tab's title: `tool-<tool>-tab-<tab>`.
pub fn tab(tool: &str, tab: &str) -> String {
    format!("tool-{tool}-tab-{tab}")
}

/// A plain group's title, or a row of pickers' label: `tool-<tool>-group-<group>`.
pub fn group(tool: &str, group: &str) -> String {
    format!("tool-{tool}-group-{group}")
}

/// A collapsible group's title: `tool-<tool>-collapsible-<group>`.
pub fn collapsible(tool: &str, group: &str) -> String {
    format!("tool-{tool}-collapsible-{group}")
}

/// The title of the group `group` of the kind `kind` (decision 100): a plain group's
/// ([`group`]) or a collapsible one's ([`collapsible`]).
pub fn group_title(tool: &str, group: &str, kind: GroupKind) -> String {
    match kind {
        GroupKind::Plain => self::group(tool, group),
        GroupKind::Collapsible => collapsible(tool, group),
    }
}

/// A control's short label: `tool-<tool>-control-<control>-label`.
pub fn label(tool: &str, control: &str) -> String {
    format!("tool-{tool}-control-{control}-label")
}

/// A control's full name, which screen readers say:
/// `tool-<tool>-control-<control>-full`.
pub fn full(tool: &str, control: &str) -> String {
    format!("tool-{tool}-control-{control}-full")
}

/// A control's promise, its tooltip: `tool-<tool>-control-<control>-promise`.
pub fn promise(tool: &str, control: &str) -> String {
    format!("tool-{tool}-control-{control}-promise")
}

/// A control's label's value of the photo's ([`crate::ProcessingTool::label_value`],
/// decision 88), drawn after the label, `$value` the value with its unit:
/// `tool-<tool>-control-<control>-label-value`. The definition does not imply it: the
/// tool's code gives it, so the tests ask each tool which controls have one.
pub fn label_value(tool: &str, control: &str) -> String {
    format!("tool-{tool}-control-{control}-label-value")
}

/// A choice's option: `tool-<tool>-control-<control>-option-<option>`.
pub fn option(tool: &str, control: &str, option: &str) -> String {
    format!("tool-{tool}-control-{control}-option-{option}")
}

/// Why a segmented choice's option is drawn disabled on a photo that does not list it
/// ([`crate::ProcessingTool::options`], decision 88), its tooltip:
/// `tool-<tool>-control-<control>-option-<option>-unavailable`, as Exposure's "Automatic
/// needs a raw photo's histogram". The definition does not imply it: which photos list an
/// option is the tool's code, so the tests ask each tool which options a photo leaves out.
/// A list leaves such an option out, and needs none.
pub fn option_unavailable(tool: &str, control: &str, option: &str) -> String {
    format!("tool-{tool}-control-{control}-option-{option}-unavailable")
}

/// A row's own words, where darktable labels its control otherwise in the mode the row
/// shows for ([`crate::Slider::label`]): its short label, full name and promise,
/// `tool-<tool>-control-<control>-as-<words>-label`, `-full` and `-promise`.
pub fn row_words(tool: &str, control: &str, words: &str) -> [String; 3] {
    ["label", "full", "promise"]
        .map(|part| format!("tool-{tool}-control-{control}-as-{words}-{part}"))
}

/// A tool's setting's short label, full name and promise ([`crate::Row::Setting`]):
/// `tool-<tool>-setting-<setting>-label`, `-full` and `-promise`.
pub fn setting(tool: &str, setting: &str) -> [String; 3] {
    ["label", "full", "promise"].map(|part| format!("tool-{tool}-setting-{setting}-{part}"))
}

/// A tool's setting's option: `tool-<tool>-setting-<setting>-option-<option>`.
pub fn setting_option(tool: &str, setting: &str, option: &str) -> String {
    format!("tool-{tool}-setting-{setting}-option-{option}")
}

/// A setting row's label, full name and promise, as message ids: the words it declares
/// ([`SettingRow::words`]), a full name it leaves out being its label's and a promise it
/// leaves out none; else those its id implies ([`setting`]).
pub fn setting_row(tool: &str, row: &SettingRow) -> [Option<String>; 3] {
    match row.words {
        Some(words) => [
            Some(words.label),
            Some(words.full.unwrap_or(words.label)),
            words.promise,
        ]
        .map(|id| id.map(str::to_owned)),
        None => setting(tool, row.id).map(Some),
    }
}

/// A setting row's options' words, each a message id or a name: those it declares,
/// else the messages its options' ids imply ([`setting_option`]); none for a number and a
/// toggle, and a boolean's No and Yes where it is drawn as their choice
/// ([`SettingRow::no_or_yes`]).
pub fn setting_row_options(tool: &str, row: &SettingRow) -> Vec<Word<String>> {
    if let Some(words) = row.words {
        return words
            .options
            .iter()
            .map(|word| match word {
                Word::Message(id) => Word::Message((*id).to_owned()),
                Word::Name(name) => Word::Name((*name).to_owned()),
            })
            .collect();
    }
    match row.kind {
        ControlKind::Choice { options, .. } => options
            .iter()
            .map(|option| Word::Message(setting_option(tool, row.id, option)))
            .collect(),
        _ => Vec::new(),
    }
}

/// What an option of a setting a picker's words follow ([`crate::Picker::words_by`]) does,
/// its tooltip in the list: `tool-<tool>-setting-<setting>-option-<option>-promise`, as
/// Exposure's Picker target's Middle grey says "The picked area becomes a normal mid-tone
/// (L 50 %)" (decision 88).
pub fn setting_option_promise(tool: &str, setting: &str, option: &str) -> String {
    format!("tool-{tool}-setting-{setting}-option-{option}-promise")
}

/// darktable's guides widget's words ([`crate::Row::Guides`]), every tool's: show
/// guides' label, full name and promise, then the global settings button's name and
/// promise.
pub const GUIDES: [&str; 5] = [
    "guides-show-label",
    "guides-show-full",
    "guides-show-promise",
    "guides-settings-full",
    "guides-settings-promise",
];

/// A picker's patches' words ([`crate::Row::Patches`]), every tool's, darktable's: the
/// input's heading and tooltip, the target's heading, the L readout's tooltip, and the
/// readout, `$value` its L in the language's number format, or none before a pick. The
/// target patch's tooltip is its first setting's promise.
pub const PATCHES: [&str; 6] = [
    "picker-patch-input",
    "picker-patch-input-tip",
    "picker-patch-target",
    "picker-patch-lch-tip",
    "picker-patch-lightness",
    "picker-patch-none",
];

/// A reference row's words ([`crate::Row::Reference`], decision 88), every tool's: its
/// value, `$lightness` its L in the language's numbers; the photo it was measured on,
/// `$photo` its name, or the open photo; none set; while its picker measures;
/// the tooltip of a photo no longer there; the ⋯ button's name; and its menu's Clear
/// Reference and Set Reference from an Area.
pub const REFERENCE: [&str; 9] = [
    "reference-value",
    "reference-photo",
    "reference-this-photo",
    "reference-none",
    "reference-measuring",
    "reference-gone",
    "reference-more",
    "reference-clear",
    "reference-set",
];

/// [`picker_option`]'s words for an option that shows a reference row
/// ([`crate::Row::Reference`]) where the reference's photo is not known, as one set
/// through `--conf` (decision 88): `tool-<tool>-picker-<picker>-<part>-<option>-no-photo`,
/// as Exposure's "Match an area to L 62 %".
pub fn picker_option_no_photo(tool: &str, picker: &str, part: &str, option: &str) -> String {
    format!("tool-{tool}-picker-{picker}-{part}-{option}-no-photo")
}

/// [`picker_option`]'s words for an option that shows a reference row where the reference
/// was measured on the photo that is open (decision 88):
/// `tool-<tool>-picker-<picker>-<part>-<option>-this-photo`, as Exposure's "Match an area
/// to this photo (L 62 %)".
pub fn picker_option_this_photo(tool: &str, picker: &str, part: &str, option: &str) -> String {
    format!("tool-{tool}-picker-{picker}-{part}-{option}-this-photo")
}

/// The options of the setting `picker`'s words follow under which a reference row shows
/// (a [`crate::Row::WhenSetting`] of that setting holding a [`crate::Row::Reference`]).
pub fn reference_options(
    definition: &ProcessingDefinition,
    picker: &crate::Picker,
) -> Vec<&'static str> {
    let Some(setting) = picker.words_by else {
        return Vec::new();
    };
    let holds = |rows: &[Row]| rows.iter().any(|row| matches!(row, Row::Reference { .. }));
    definition
        .rows()
        .filter_map(|row| match row {
            Row::WhenSetting {
                setting: id,
                is,
                rows,
            } if *id == setting && holds(rows) => Some(*is),
            _ => None,
        })
        .flatten()
        .copied()
        .collect()
}

/// A last pick row's words ([`crate::Row::LastPick`], decision 88), every tool's: its
/// label; its value, `$before` and `$after` the area's L and `$change` the change of the
/// control with its unit, in the language's numbers; its tooltip; the clipped warning and
/// its tooltip; and the short warning and its tooltip, `$control` the control's label,
/// `$value` the end it is held at with its unit, `$reached` and `$target` the Ls.
pub const LAST_PICK: [&str; 7] = [
    "last-pick-label",
    "last-pick-value",
    "last-pick-promise",
    "pick-warning-clipped",
    "pick-warning-clipped-tip",
    "pick-warning-short",
    "pick-warning-short-tip",
];

/// What a short pick's tooltip adds where the pick moved another control too, the
/// picker's own: `tool-<tool>-picker-<picker>-short-black`, as Exposure's "Black level was
/// moved to keep black below white".
pub fn short_black(tool: &str, picker: &str) -> String {
    format!("tool-{tool}-picker-{picker}-short-black")
}

/// A picker's words over the photo where they are its own and follow no setting: the
/// viewer bar's name of it, `tool-<tool>-picker-<picker>-hint`, as the reference row's
/// picker's "Drag over the area to keep as the reference" (decision 88).
pub fn picker_hint(tool: &str, picker: &str) -> String {
    format!("tool-{tool}-picker-{picker}-hint")
}

/// Why a picker cannot start ([`crate::ProcessingTool::picker_ready`]), its tooltip while
/// drawn disabled: `tool-<tool>-picker-<picker>-unavailable`, as Exposure's "Set a
/// reference first, with the picker on the Reference row". The definition does not imply
/// it: readiness is the tool's code, so the tests ask each tool when its pickers are ready.
pub fn picker_unavailable(tool: &str, picker: &str) -> String {
    format!("tool-{tool}-picker-{picker}-unavailable")
}

/// An action's short label, full name and promise ([`crate::Row::Action`]):
/// `tool-<tool>-action-<action>-label`, `-full` and `-promise`.
pub fn action(tool: &str, action: &str) -> [String; 3] {
    ["label", "full", "promise"].map(|part| format!("tool-{tool}-action-{action}-{part}"))
}

/// A computed value's label and promise ([`crate::Row::Computed`]):
/// `tool-<tool>-computed-<id>-label` and `-promise`.
pub fn computed(tool: &str, id: &str) -> [String; 2] {
    ["label", "promise"].map(|part| format!("tool-{tool}-computed-{id}-{part}"))
}

/// A colour picker's name, full name, promise and, for one that takes a point, the
/// viewer bar's sentence ([`crate::Picker`]): `tool-<tool>-picker-<picker>-label`, `-full`,
/// `-promise` and `-what`.
pub fn picker(tool: &str, picker: &str) -> [String; 4] {
    ["label", "full", "promise", "what"].map(|part| format!("tool-{tool}-picker-{picker}-{part}"))
}

/// A picker's tooltip or its hint over the photo, `part` `promise` or `hint`, while the
/// setting its words follow holds `option` ([`crate::Picker::words_by`]):
/// `tool-<tool>-picker-<picker>-<part>-<option>`, as Exposure's "Make an area middle grey".
/// Each of the tool's number settings is a variable named by its id, as `$lightness`.
pub fn picker_option(tool: &str, picker: &str, part: &str, option: &str) -> String {
    format!("tool-{tool}-picker-{picker}-{part}-{option}")
}

/// The words of `picker` of `definition` that its button and the viewer bar use, of
/// [`picker`]'s: its promise and full name, or where its words follow a setting a promise
/// and a hint for each of the setting's options ([`picker_option`]), the promise being its
/// button's name too, the action; its name
/// where it is no slider's quad, which takes its control's label; and its sentence where
/// it takes a point, an area picker's being every picker's.
pub fn picker_words(definition: &ProcessingDefinition, picker: &crate::Picker) -> Vec<String> {
    let tool = definition.tool.id;
    let [label, full, promise, what] = self::picker(tool, picker.id);
    let on_quad = definition
        .rows()
        .any(|row| matches!(row, Row::Slider(_)) && row.pickers() == [picker.id]);
    let mut words = Vec::new();
    if !on_quad {
        words.push(label);
    }
    let options = picker
        .words_by
        .and_then(|setting| definition.choice_setting(setting));
    match options {
        Some(options) => {
            for option in options {
                for part in ["promise", "hint"] {
                    words.push(picker_option(tool, picker.id, part, option));
                }
            }
            // An option that shows a reference says so with no photo known, and with the
            // open photo, too.
            for option in reference_options(definition, picker) {
                for part in ["promise", "hint"] {
                    words.push(picker_option_no_photo(tool, picker.id, part, option));
                    words.push(picker_option_this_photo(tool, picker.id, part, option));
                }
            }
        }
        None => {
            words.push(full);
            words.push(promise);
        }
    }
    // A reference row's picker says over the photo what it will keep.
    let on_reference = (definition.rows())
        .any(|row| matches!(row, Row::Reference { picker: id, .. } if *id == picker.id));
    if on_reference {
        words.push(picker_hint(tool, picker.id));
    }
    if picker.kind != crate::PickerKind::Area {
        words.push(what);
    }
    words
}

/// A preset row's short label, full name and promise:
/// `tool-<tool>-preset-<preset>-label`, `-full` and `-promise`.
pub fn preset(tool: &str, preset: &str) -> [String; 3] {
    ["label", "full", "promise"].map(|part| format!("tool-{tool}-preset-{preset}-{part}"))
}

/// A preset row's option: `tool-<tool>-preset-<preset>-option-<option>`.
pub fn preset_option(tool: &str, preset: &str, option: &str) -> String {
    format!("tool-{tool}-preset-{preset}-option-{option}")
}

/// What a preset row shows while the values hold none of its options, the same in every
/// tool: `preset-custom`, in `ui.ftl`.
pub const PRESET_CUSTOM: &str = "preset-custom";

/// A unit, from the closed list: `unit-<unit>`, in `ui.ftl`.
pub fn unit(unit: Unit) -> String {
    format!("unit-{}", unit.id())
}

/// A value with its unit, `$value` already in the language's number format:
/// `unit-<unit>-value`, in `ui.ftl` (`{ $value } EV`, `{ $value }%`). Each language puts
/// the unit where it writes it, so a panel never joins a number and a unit itself.
pub fn unit_value(unit: Unit) -> String {
    format!("unit-{}-value", unit.id())
}

/// Every message the definition implies, each once: its title, its description, Auto's
/// tooltip if it has Auto, each titled tab, each group's title, plain or collapsible, a row
/// of pickers' label among the plain ones', the words of each row that has its own, each
/// setting's words and options, with each option's promise where a picker's words follow
/// the setting, each action's words, each control's label, full name and promise, each
/// option, each view's coordinate's label, full name and promise, as a control's, each
/// preset row's label, full name, promise and options, each unit it uses, Custom if it
/// has a preset row, the patches' words if it has patches, a reference row's if it has
/// one, and each picker's words its button and the viewer bar use ([`picker_words`]),
/// then, where it has darktable's facts, its kind's and what it works on, and each of
/// darktable's lines.
pub fn of(definition: &ProcessingDefinition) -> Vec<String> {
    let tool = definition.tool.id;
    let mut ids = vec![title(tool), description(tool)];
    if definition.auto {
        ids.push(auto(tool));
    }
    ids.extend(
        definition
            .tool
            .tabs
            .iter()
            .filter_map(|t| t.id)
            .map(|t| tab(tool, t)),
    );
    for row in definition.rows() {
        match row {
            Row::Group(found) => ids.push(group_title(tool, found.id, found.kind)),
            Row::Pickers {
                label: Some(id), ..
            } => ids.push(group(tool, id)),
            Row::Slider(Slider {
                control,
                label: Some(words),
                ..
            }) => ids.extend(row_words(tool, control, words)),
            Row::Setting(SettingRow { id, kind, .. }) => {
                ids.extend(setting(tool, id));
                if let ControlKind::Choice { options, .. } = kind {
                    ids.extend(options.iter().map(|o| setting_option(tool, id, o)));
                    // What each option makes a picker do, where a picker's words follow it.
                    let words_by = definition.pickers.iter().any(|p| p.words_by == Some(id));
                    if words_by {
                        ids.extend(options.iter().map(|o| setting_option_promise(tool, id, o)));
                    }
                }
            }
            Row::Action { id } => ids.extend(action(tool, id)),
            Row::Computed(computed) => ids.extend(self::computed(tool, computed.id)),
            Row::Buttons { buttons, .. } => {
                for button in *buttons {
                    ids.extend(action(tool, button.id));
                }
            }
            Row::Guides => ids.extend(GUIDES.map(str::to_owned)),
            Row::Patches { .. } => ids.extend(PATCHES.map(str::to_owned)),
            Row::Reference { .. } => ids.extend(REFERENCE.map(str::to_owned)),
            Row::LastPick { picker } => {
                ids.extend(LAST_PICK.map(str::to_owned));
                ids.push(short_black(tool, picker));
            }
            _ => {}
        }
    }
    for picker in definition.pickers {
        ids.extend(picker_words(definition, picker));
    }
    for control in definition.controls {
        let id = control.id;
        ids.extend([label(tool, id), full(tool, id), promise(tool, id)]);
        if let ControlKind::Choice { options, .. } = control.kind {
            ids.extend(options.iter().map(|o| option(tool, id, o)));
        }
    }
    // A view's coordinate is addressed like a control, and worded like one.
    let coordinates = || definition.views.iter().flat_map(|view| view.coordinates);
    for coordinate in coordinates() {
        let id = coordinate.id;
        ids.extend([label(tool, id), full(tool, id), promise(tool, id)]);
    }
    for row in definition.preset_rows {
        ids.extend(preset(tool, row.id));
        ids.extend(row.options.iter().map(|o| preset_option(tool, row.id, o)));
    }
    for used in Unit::ALL {
        let units = definition.controls.iter().map(|c| c.unit);
        if units
            .chain(coordinates().map(|c| c.unit))
            .any(|u| u == Some(used))
        {
            ids.push(unit(used));
        }
    }
    if !definition.preset_rows.is_empty() {
        ids.push(PRESET_CUSTOM.to_owned());
    }
    if let Some(facts) = definition.facts {
        ids.push(facts.kind().message().to_owned());
        ids.extend(facts.works_on().map(|place| place.message().to_owned()));
        ids.extend([facts.input, facts.process, facts.output].map(|fact| fact.message()));
    }
    // A control's rows in two modes may share their words.
    let mut seen = BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    ids
}

/// A utility's own ↺'s name (`Reset::Own`): `<utility>-reset`, as Export's.
pub fn utility_reset(utility: &str) -> String {
    format!("{utility}-reset")
}

/// A utility's presets button's name, and its tooltip while it is drawn disabled, until
/// its presets are built: `<utility>-presets` and `<utility>-presets-later`, as Export's.
pub fn utility_presets(utility: &str) -> [String; 2] {
    [
        format!("{utility}-presets"),
        format!("{utility}-presets-later"),
    ]
}

/// Every message a utility's header and rows imply, each once, as [`of`] lists a
/// processing tool's: its own ↺'s name, its presets button's words, each group's title, and each setting's words and options, those it
/// declares where it does, a name an option declares needing none. Its custom rows' words
/// are its widgets' own.
pub fn of_utility(utility: &UtilityDefinition) -> Vec<String> {
    fn rows(utility: &UtilityDefinition, list: &[Row], ids: &mut Vec<String>) {
        let tool = utility.tool.id;
        for row in list {
            match row {
                Row::Group(group) => ids.push(utility.title(group.id, group.kind)),
                Row::Setting(setting) => {
                    ids.extend(setting_row(tool, setting).into_iter().flatten());
                    ids.extend(
                        setting_row_options(tool, setting).into_iter().filter_map(
                            |word| match word {
                                Word::Message(id) => Some(id),
                                Word::Name(_) => None,
                            },
                        ),
                    );
                }
                _ => {}
            }
            rows(utility, row.rows(), ids);
        }
    }
    let definition = &utility.tool;
    let mut ids = Vec::new();
    if definition.reset == Reset::Own {
        ids.push(utility_reset(definition.id));
    }
    if definition.presets {
        ids.extend(utility_presets(definition.id));
    }
    let tabs = definition.tabs.iter().map(|tab| tab.rows);
    for list in std::iter::once(definition.before)
        .chain(tabs)
        .chain(std::iter::once(definition.after))
    {
        rows(utility, list, &mut ids);
    }
    let mut seen = BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProcessingTool;
    use crate::definition::{ChoiceStyle, Control, Reset, Tab, ToolDefinition, Track};
    use crate::testing::Sample;

    #[test]
    fn a_definition_implies_its_messages_in_the_scheme() {
        assert_eq!(
            of(Sample.definition()),
            [
                "tool-sample-title",
                "tool-sample-description",
                "tool-sample-tab-grain",
                "tool-sample-tab-shape",
                "tool-sample-group-grain",
                "tool-sample-group-measure",
                "tool-sample-collapsible-midtones",
                // A quad's picker takes its slider's label, and an area's needs no
                // sentence of its own.
                "tool-sample-picker-brightness-full",
                "tool-sample-picker-brightness-promise",
                "tool-sample-picker-spread-label",
                "tool-sample-picker-spread-full",
                "tool-sample-picker-spread-promise",
                "tool-sample-picker-spread-what",
                "tool-sample-picker-show-label",
                "tool-sample-picker-show-full",
                "tool-sample-picker-show-promise",
                "tool-sample-picker-show-what",
                "tool-sample-control-brightness-label",
                "tool-sample-control-brightness-full",
                "tool-sample-control-brightness-promise",
                "tool-sample-control-grain-label",
                "tool-sample-control-grain-full",
                "tool-sample-control-grain-promise",
                "tool-sample-control-size-label",
                "tool-sample-control-size-full",
                "tool-sample-control-size-promise",
                "tool-sample-control-size-option-fine",
                "tool-sample-control-size-option-medium",
                "tool-sample-control-size-option-coarse",
                "tool-sample-control-method-label",
                "tool-sample-control-method-full",
                "tool-sample-control-method-promise",
                "tool-sample-control-method-option-even",
                "tool-sample-control-method-option-biased",
                "tool-sample-control-method-option-film",
                "tool-sample-control-method-option-camera",
                "tool-sample-control-bias-label",
                "tool-sample-control-bias-full",
                "tool-sample-control-bias-promise",
                "tool-sample-control-coarseness-label",
                "tool-sample-control-coarseness-full",
                "tool-sample-control-coarseness-promise",
                "tool-sample-control-sum-label",
                "tool-sample-control-sum-full",
                "tool-sample-control-sum-promise",
                "tool-sample-control-difference-label",
                "tool-sample-control-difference-full",
                "tool-sample-control-difference-promise",
                "tool-sample-preset-look-label",
                "tool-sample-preset-look-full",
                "tool-sample-preset-look-promise",
                "tool-sample-preset-look-option-untouched",
                "tool-sample-preset-look-option-grainy",
                "tool-sample-preset-look-option-camera",
                "unit-ev",
                "unit-percent",
                "preset-custom",
            ]
        );
    }

    #[test]
    fn a_row_with_words_of_its_own_implies_them_once() {
        static CONTROLS: &[Control] = &[
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
            Control {
                id: "sigma_r",
                kind: ControlKind::Number {
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                    default: 0.5,
                },
                unit: None,
                unported: false,
            },
        ];
        static ROWS: &[Row] = &[
            Row::Choice {
                control: "mode",
                style: ChoiceStyle::List,
            },
            Row::When {
                control: "mode",
                is: &["bilateral"],
                rows: &[Row::Slider(Slider::new("sigma_r"))],
            },
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: &[Row::Slider(Slider::new("sigma_r").label("highlights"))],
            },
            Row::When {
                control: "mode",
                is: &["neural"],
                rows: &[Row::Slider(Slider::new("sigma_r").label("highlights"))],
            },
        ];
        static LOCAL_CONTRAST: ProcessingDefinition = ProcessingDefinition {
            tool: ToolDefinition {
                id: "bilat",
                tabs: &[Tab {
                    id: None,
                    rows: ROWS,
                }],
                before: &[],
                after: &[],
                reset: Reset::Tool,
                presets: true,
                open_at_first: false,
                menus: &[],
            },
            version: 1,
            layers: true,
            auto: false,
            switched: false,
            controls: CONTROLS,
            preset_rows: &[],
            views: &[],
            pickers: &[],
            facts: None,
        };
        LOCAL_CONTRAST.check().unwrap();
        let ids = of(&LOCAL_CONTRAST);
        let words = row_words("bilat", "sigma_r", "highlights");
        assert_eq!(
            words,
            [
                "tool-bilat-control-sigma_r-as-highlights-label",
                "tool-bilat-control-sigma_r-as-highlights-full",
                "tool-bilat-control-sigma_r-as-highlights-promise",
            ]
        );
        assert_eq!(&ids[2..5], words);
        // The control's own words stay, for the CLI and MCP.
        assert!(ids.contains(&label("bilat", "sigma_r")));
        assert_eq!(ids.len(), 2 + 3 + 3 + 3 + 3);
    }

    #[test]
    fn a_tool_s_setting_implies_its_words_and_options() {
        static CONTROLS: &[Control] = &[Control {
            id: "amount",
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        }];
        static CHECKER: ProcessingDefinition = ProcessingDefinition {
            tool: ToolDefinition {
                id: "mixer",
                tabs: &[Tab {
                    id: None,
                    rows: &[Row::Slider(Slider::new("amount"))],
                }],
                before: &[],
                after: &[Row::Setting(SettingRow {
                    id: "chart",
                    kind: ControlKind::Choice {
                        options: &["classic", "passport"],
                        default: 0,
                        unported: &[],
                    },
                    decimals: 0,
                    track: Track::Plain,
                    key: None,
                    words: None,
                })],
                reset: Reset::Tool,
                presets: true,
                open_at_first: false,
                menus: &[],
            },
            version: 1,
            layers: false,
            auto: false,
            switched: false,
            controls: CONTROLS,
            preset_rows: &[],
            views: &[],
            pickers: &[],
            facts: None,
        };
        CHECKER.check().unwrap();
        assert_eq!(
            of(&CHECKER),
            [
                "tool-mixer-title",
                "tool-mixer-description",
                "tool-mixer-setting-chart-label",
                "tool-mixer-setting-chart-full",
                "tool-mixer-setting-chart-promise",
                "tool-mixer-setting-chart-option-classic",
                "tool-mixer-setting-chart-option-passport",
                "tool-mixer-control-amount-label",
                "tool-mixer-control-amount-full",
                "tool-mixer-control-amount-promise",
            ]
        );

        // A picker whose words follow the chart: a promise and a hint per option in place
        // of its promise, and a tooltip for each of the chart's options (decision 88).
        static PICKER: crate::Picker = crate::Picker {
            id: "amount",
            operation: "mixer",
            kind: crate::PickerKind::Area,
            denoise: false,
            output: false,
            space: crate::Space::Operation,
            keep: false,
            writes: &["amount"],
            settings: &["chart"],
            words_by: Some("chart"),
        };
        static QUAD: &[Tab] = &[Tab {
            id: None,
            rows: &[Row::Slider(
                Slider::new("amount").quad(crate::Quad::Picker("amount")),
            )],
        }];
        let following = ProcessingDefinition {
            tool: ToolDefinition {
                tabs: QUAD,
                ..CHECKER.tool
            },
            pickers: std::slice::from_ref(&PICKER),
            ..CHECKER
        };
        following.check().unwrap();
        assert_eq!(
            of(&following),
            [
                "tool-mixer-title",
                "tool-mixer-description",
                "tool-mixer-setting-chart-label",
                "tool-mixer-setting-chart-full",
                "tool-mixer-setting-chart-promise",
                "tool-mixer-setting-chart-option-classic",
                "tool-mixer-setting-chart-option-passport",
                "tool-mixer-setting-chart-option-classic-promise",
                "tool-mixer-setting-chart-option-passport-promise",
                "tool-mixer-picker-amount-promise-classic",
                "tool-mixer-picker-amount-hint-classic",
                "tool-mixer-picker-amount-promise-passport",
                "tool-mixer-picker-amount-hint-passport",
                "tool-mixer-control-amount-label",
                "tool-mixer-control-amount-full",
                "tool-mixer-control-amount-promise",
            ]
        );
    }

    #[test]
    fn an_action_implies_its_words() {
        static CONTROLS: &[Control] = &[Control {
            id: "white",
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.5,
            },
            unit: None,
            unported: false,
        }];
        static LEVELS: ProcessingDefinition = ProcessingDefinition {
            tool: ToolDefinition {
                id: "levels",
                tabs: &[Tab {
                    id: None,
                    rows: &[
                        Row::Slider(Slider::new("white")),
                        Row::Action { id: "auto_tune" },
                    ],
                }],
                before: &[],
                after: &[],
                reset: Reset::Tool,
                presets: true,
                open_at_first: false,
                menus: &[],
            },
            version: 1,
            layers: false,
            auto: false,
            switched: false,
            controls: CONTROLS,
            preset_rows: &[],
            views: &[],
            pickers: &[],
            facts: None,
        };
        LEVELS.check().unwrap();
        assert_eq!(
            of(&LEVELS),
            [
                "tool-levels-title",
                "tool-levels-description",
                "tool-levels-action-auto_tune-label",
                "tool-levels-action-auto_tune-full",
                "tool-levels-action-auto_tune-promise",
                "tool-levels-control-white-label",
                "tool-levels-control-white-full",
                "tool-levels-control-white-promise",
            ]
        );
    }
}
