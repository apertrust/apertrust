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

//! A module's words, as message ids that follow from its ids (decision 28, `docs/gui.md`,
//! Module panels from definitions). Hyphens separate the parts, ids are snake_case, and a
//! fixed word comes before each id, so an option or a tab called `label` or `title`
//! cannot collide with another message. The messages are in
//! `crates/ui/locales/<locale>/modules.ftl`.

use std::collections::BTreeSet;

use crate::definition::{ControlKind, Definition, Row, Slider, ToolSetting, Unit};

/// The module's title: `module-<module>-title`.
pub fn title(module: &str) -> String {
    format!("module-{module}-title")
}

/// Auto's tooltip, which names its target: `module-<module>-auto`.
pub fn auto(module: &str) -> String {
    format!("module-{module}-auto")
}

/// A tab's title: `module-<module>-tab-<tab>`.
pub fn tab(module: &str, tab: &str) -> String {
    format!("module-{module}-tab-{tab}")
}

/// A group label: `module-<module>-group-<group>`.
pub fn group(module: &str, group: &str) -> String {
    format!("module-{module}-group-{group}")
}

/// A collapsible group's title: `module-<module>-collapsible-<group>`.
pub fn collapsible(module: &str, group: &str) -> String {
    format!("module-{module}-collapsible-{group}")
}

/// A control's short label: `module-<module>-control-<control>-label`.
pub fn label(module: &str, control: &str) -> String {
    format!("module-{module}-control-{control}-label")
}

/// A control's full name, which screen readers say:
/// `module-<module>-control-<control>-full`.
pub fn full(module: &str, control: &str) -> String {
    format!("module-{module}-control-{control}-full")
}

/// A control's promise, its tooltip: `module-<module>-control-<control>-promise`.
pub fn promise(module: &str, control: &str) -> String {
    format!("module-{module}-control-{control}-promise")
}

/// A choice's option: `module-<module>-control-<control>-option-<option>`.
pub fn option(module: &str, control: &str, option: &str) -> String {
    format!("module-{module}-control-{control}-option-{option}")
}

/// A row's own words, where darktable labels its control otherwise in the mode the row
/// shows for ([`crate::Slider::label`]): its short label, full name and promise,
/// `module-<module>-control-<control>-as-<words>-label`, `-full` and `-promise`.
pub fn row_words(module: &str, control: &str, words: &str) -> [String; 3] {
    ["label", "full", "promise"]
        .map(|part| format!("module-{module}-control-{control}-as-{words}-{part}"))
}

/// A tool's setting's short label, full name and promise ([`crate::Row::ToolSetting`]):
/// `module-<module>-setting-<setting>-label`, `-full` and `-promise`.
pub fn setting(module: &str, setting: &str) -> [String; 3] {
    ["label", "full", "promise"].map(|part| format!("module-{module}-setting-{setting}-{part}"))
}

/// A tool's setting's option: `module-<module>-setting-<setting>-option-<option>`.
pub fn setting_option(module: &str, setting: &str, option: &str) -> String {
    format!("module-{module}-setting-{setting}-option-{option}")
}

/// darktable's guides widget's words ([`crate::Row::Guides`]), every module's: show
/// guides' label, full name and promise, then the global settings button's name and
/// promise.
pub const GUIDES: [&str; 5] = [
    "guides-show-label",
    "guides-show-full",
    "guides-show-promise",
    "guides-settings-full",
    "guides-settings-promise",
];

/// An action's short label, full name and promise ([`crate::Row::Action`]):
/// `module-<module>-action-<action>-label`, `-full` and `-promise`.
pub fn action(module: &str, action: &str) -> [String; 3] {
    ["label", "full", "promise"].map(|part| format!("module-{module}-action-{action}-{part}"))
}

/// A preset row's short label, full name and promise:
/// `module-<module>-preset-<preset>-label`, `-full` and `-promise`.
pub fn preset(module: &str, preset: &str) -> [String; 3] {
    ["label", "full", "promise"].map(|part| format!("module-{module}-preset-{preset}-{part}"))
}

/// A preset row's option: `module-<module>-preset-<preset>-option-<option>`.
pub fn preset_option(module: &str, preset: &str, option: &str) -> String {
    format!("module-{module}-preset-{preset}-option-{option}")
}

/// What a preset row shows while the values hold none of its options, the same in every
/// module: `preset-custom`, in `ui.ftl`.
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

/// Every message the definition implies, each once: its title, Auto's tooltip if it has
/// Auto, each titled tab, each group label, a row of pickers' label among them, each
/// collapsible group's title, the words of each row that has its own, each tool's
/// setting's words and options, each action's words, each control's label, full name and
/// promise, each option, each view's coordinate's label, full name and promise, as a
/// control's, each preset row's label, full name, promise and options, each unit it uses,
/// and Custom if it has a preset row. A picker's own words come with its button
/// ([`crate::Picker`]).
pub fn of(definition: &Definition) -> Vec<String> {
    let module = definition.id;
    let mut ids = vec![title(module)];
    if definition.auto {
        ids.push(auto(module));
    }
    ids.extend(
        definition
            .tabs
            .iter()
            .filter_map(|t| t.id)
            .map(|t| tab(module, t)),
    );
    for row in definition.rows() {
        match row {
            Row::Label { id }
            | Row::Pickers {
                label: Some(id), ..
            } => ids.push(group(module, id)),
            Row::Group { id, .. } => ids.push(collapsible(module, id)),
            Row::Slider(Slider {
                control,
                label: Some(words),
                ..
            }) => ids.extend(row_words(module, control, words)),
            Row::ToolSetting(ToolSetting { id, kind, .. }) => {
                ids.extend(setting(module, id));
                if let ControlKind::Choice { options, .. } = kind {
                    ids.extend(options.iter().map(|o| setting_option(module, id, o)));
                }
            }
            Row::Action { id } => ids.extend(action(module, id)),
            Row::Buttons { buttons, .. } => {
                for button in *buttons {
                    ids.extend(action(module, button.id));
                }
            }
            Row::Guides => ids.extend(GUIDES.map(str::to_owned)),
            _ => {}
        }
    }
    for control in definition.controls {
        let id = control.id;
        ids.extend([label(module, id), full(module, id), promise(module, id)]);
        if let ControlKind::Choice { options, .. } = control.kind {
            ids.extend(options.iter().map(|o| option(module, id, o)));
        }
    }
    // A view's coordinate is addressed like a control, and worded like one.
    let coordinates = || definition.views.iter().flat_map(|view| view.coordinates);
    for coordinate in coordinates() {
        let id = coordinate.id;
        ids.extend([label(module, id), full(module, id), promise(module, id)]);
    }
    for row in definition.presets {
        ids.extend(preset(module, row.id));
        ids.extend(row.options.iter().map(|o| preset_option(module, row.id, o)));
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
    if !definition.presets.is_empty() {
        ids.push(PRESET_CUSTOM.to_owned());
    }
    // A control's rows in two modes may share their words.
    let mut seen = BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Module;
    use crate::definition::{ChoiceStyle, Control, Tab};
    use crate::testing::Sample;

    #[test]
    fn a_definition_implies_its_messages_in_the_scheme() {
        assert_eq!(
            of(Sample.definition()),
            [
                "module-sample-title",
                "module-sample-tab-grain",
                "module-sample-tab-shape",
                "module-sample-group-grain",
                "module-sample-group-measure",
                "module-sample-collapsible-midtones",
                "module-sample-control-brightness-label",
                "module-sample-control-brightness-full",
                "module-sample-control-brightness-promise",
                "module-sample-control-grain-label",
                "module-sample-control-grain-full",
                "module-sample-control-grain-promise",
                "module-sample-control-size-label",
                "module-sample-control-size-full",
                "module-sample-control-size-promise",
                "module-sample-control-size-option-fine",
                "module-sample-control-size-option-medium",
                "module-sample-control-size-option-coarse",
                "module-sample-control-method-label",
                "module-sample-control-method-full",
                "module-sample-control-method-promise",
                "module-sample-control-method-option-even",
                "module-sample-control-method-option-biased",
                "module-sample-control-method-option-film",
                "module-sample-control-method-option-camera",
                "module-sample-control-bias-label",
                "module-sample-control-bias-full",
                "module-sample-control-bias-promise",
                "module-sample-control-coarseness-label",
                "module-sample-control-coarseness-full",
                "module-sample-control-coarseness-promise",
                "module-sample-control-sum-label",
                "module-sample-control-sum-full",
                "module-sample-control-sum-promise",
                "module-sample-control-difference-label",
                "module-sample-control-difference-full",
                "module-sample-control-difference-promise",
                "module-sample-preset-look-label",
                "module-sample-preset-look-full",
                "module-sample-preset-look-promise",
                "module-sample-preset-look-option-untouched",
                "module-sample-preset-look-option-grainy",
                "module-sample-preset-look-option-camera",
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
        static LOCAL_CONTRAST: Definition = Definition {
            id: "bilat",
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
        LOCAL_CONTRAST.check().unwrap();
        let ids = of(&LOCAL_CONTRAST);
        let words = row_words("bilat", "sigma_r", "highlights");
        assert_eq!(
            words,
            [
                "module-bilat-control-sigma_r-as-highlights-label",
                "module-bilat-control-sigma_r-as-highlights-full",
                "module-bilat-control-sigma_r-as-highlights-promise",
            ]
        );
        assert_eq!(&ids[1..4], words);
        // The control's own words stay, for the CLI and MCP.
        assert!(ids.contains(&label("bilat", "sigma_r")));
        assert_eq!(ids.len(), 1 + 3 + 3 + 3 + 3);
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
        static CHECKER: Definition = Definition {
            id: "mixer",
            version: 1,
            layers: false,
            auto: false,
            switched: false,
            controls: CONTROLS,
            tabs: &[Tab {
                id: None,
                rows: &[Row::Slider(Slider::new("amount"))],
            }],
            before: &[],
            after: &[Row::ToolSetting(ToolSetting {
                id: "chart",
                kind: ControlKind::Choice {
                    options: &["classic", "passport"],
                    default: 0,
                    unported: &[],
                },
                decimals: 0,
            })],
            presets: &[],
            views: &[],
            pickers: &[],
        };
        CHECKER.check().unwrap();
        assert_eq!(
            of(&CHECKER),
            [
                "module-mixer-title",
                "module-mixer-setting-chart-label",
                "module-mixer-setting-chart-full",
                "module-mixer-setting-chart-promise",
                "module-mixer-setting-chart-option-classic",
                "module-mixer-setting-chart-option-passport",
                "module-mixer-control-amount-label",
                "module-mixer-control-amount-full",
                "module-mixer-control-amount-promise",
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
        static LEVELS: Definition = Definition {
            id: "levels",
            version: 1,
            layers: false,
            auto: false,
            switched: false,
            controls: CONTROLS,
            tabs: &[Tab {
                id: None,
                rows: &[
                    Row::Slider(Slider::new("white")),
                    Row::Action { id: "auto_tune" },
                ],
            }],
            before: &[],
            after: &[],
            presets: &[],
            views: &[],
            pickers: &[],
        };
        LEVELS.check().unwrap();
        assert_eq!(
            of(&LEVELS),
            [
                "module-levels-title",
                "module-levels-action-auto_tune-label",
                "module-levels-action-auto_tune-full",
                "module-levels-action-auto_tune-promise",
                "module-levels-control-white-label",
                "module-levels-control-white-full",
                "module-levels-control-white-promise",
            ]
        );
    }
}
