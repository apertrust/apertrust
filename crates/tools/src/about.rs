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

//! What a tool is for, as its header's tooltip says it (`docs/gui.md`, The header's
//! tooltip): its own sentence (`tool-<tool>-description`), then darktable's facts about
//! its module, [`Facts`], which
//! [`ProcessingDefinition::facts`](crate::ProcessingDefinition::facts) holds. Two of the
//! tooltip's rows say those facts in plain words through one mapping, so every tool says
//! them alike: its [`Kind`], from darktable's purpose, and what it [`WorksOn`], from
//! darktable's input and output. The other three are darktable's input, process and output
//! word for word, each a [`Fact`] whose message is darktable's string and its own
//! translations.
//!
//! A utility, a section that edits nothing (Scopes, History), has no definition: its
//! words and what it measures are in [`crate::utility::UTILITIES`].

use crate::messages;

/// darktable's `description()` of the module a tool is, at the reference release, past
/// its first line, which the tool's own sentence replaces: its purpose, then its input,
/// process and output lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub purpose: Purpose,
    pub input: Fact,
    pub process: Fact,
    pub output: Fact,
}

impl Facts {
    /// The tool's kind, from darktable's purpose.
    pub fn kind(&self) -> Kind {
        self.purpose.kind()
    }

    /// What the tool works on, from darktable's input and output: the camera's raw data
    /// for a raw input, else the light as captured, the finished picture, or from the one
    /// to the other, as their scene- or display-referred say. `None` for facts that say
    /// neither, which no built tool's do (the built-in tools' tests).
    pub fn works_on(&self) -> Option<WorksOn> {
        if self.input.says("raw") {
            return Some(WorksOn::Raw);
        }
        let scene = |fact: Fact| fact.says("scene-referred");
        let display = |fact: Fact| fact.says("display-referred");
        match (self.input, self.output) {
            (input, output) if scene(input) && scene(output) => Some(WorksOn::Scene),
            (input, output) if scene(input) && display(output) => Some(WorksOn::SceneToDisplay),
            (input, output) if display(input) && display(output) => Some(WorksOn::Display),
            _ => None,
        }
    }
}

/// darktable's purpose line, one of the words its modules' descriptions use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Corrective,
    Creative,
    CorrectiveAndCreative,
    CorrectiveOrCreative,
    /// A module every photo runs, as darktable's raw black/white point and demosaic.
    Mandatory,
}

impl Purpose {
    /// As darktable writes it.
    pub fn darktable(self) -> &'static str {
        match self {
            Purpose::Corrective => "corrective",
            Purpose::Creative => "creative",
            Purpose::CorrectiveAndCreative => "corrective and creative",
            Purpose::CorrectiveOrCreative => "corrective or creative",
            Purpose::Mandatory => "mandatory",
        }
    }

    /// The kind it says, its "and" and "or" kept.
    pub fn kind(self) -> Kind {
        match self {
            Purpose::Corrective => Kind::Fixes,
            Purpose::Creative => Kind::Creates,
            Purpose::CorrectiveAndCreative => Kind::FixesAndShapes,
            Purpose::CorrectiveOrCreative => Kind::FixesOrCreates,
            Purpose::Mandatory => Kind::Needed,
        }
    }
}

/// The tooltip's Kind row: what a tool is for, in plain words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// darktable's corrective: "Fixes a problem".
    Fixes,
    /// creative: "Creates a look".
    Creates,
    /// corrective and creative: "Fixes a problem and shapes the look".
    FixesAndShapes,
    /// corrective or creative: "Fixes a problem or creates a look".
    FixesOrCreates,
    /// mandatory: "Needed by every photo".
    Needed,
    /// A utility's: "Edits nothing of its own".
    EditsNothing,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Fixes,
        Kind::Creates,
        Kind::FixesAndShapes,
        Kind::FixesOrCreates,
        Kind::Needed,
        Kind::EditsNothing,
    ];

    /// Its words: `tool-kind-<kind>`.
    pub fn message(self) -> &'static str {
        match self {
            Kind::Fixes => "tool-kind-corrective",
            Kind::Creates => "tool-kind-creative",
            Kind::FixesAndShapes => "tool-kind-corrective-and-creative",
            Kind::FixesOrCreates => "tool-kind-corrective-or-creative",
            Kind::Needed => "tool-kind-mandatory",
            Kind::EditsNothing => "tool-kind-utility",
        }
    }
}

/// The tooltip's Works on row, or a utility's Measures: the camera's raw data, the light as
/// captured (darktable's scene-referred), the finished picture (display-referred), or from
/// the one to the other, as a tone mapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorksOn {
    Raw,
    Scene,
    SceneToDisplay,
    Display,
}

impl WorksOn {
    pub const ALL: [WorksOn; 4] = [
        WorksOn::Raw,
        WorksOn::Scene,
        WorksOn::SceneToDisplay,
        WorksOn::Display,
    ];

    /// Its words: `tool-works-<place>`.
    pub fn message(self) -> &'static str {
        match self {
            WorksOn::Raw => "tool-works-raw",
            WorksOn::Scene => "tool-works-scene",
            WorksOn::SceneToDisplay => "tool-works-scene-to-display",
            WorksOn::Display => "tool-works-display",
        }
    }
}

/// One of darktable's input, process or output lines, each a string darktable translates
/// whole, so each is a message of its own, `tool-fact-<words>` ([`Fact::message`]), its
/// English darktable's and its Ukrainian and Polish darktable's own translations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fact {
    LinearRawSceneReferred,
    LinearRaw,
    ReconstructionRaw,
    LinearRgbSceneReferred,
    LinearRgbDisplayReferred,
    LinearRgb,
    LinearRgbOrXyz,
    QuasiLinearRgb,
    QuasiLinearRgbSceneReferred,
    NonLinearRgb,
    NonLinearRgbSceneReferred,
    NonLinearRgbDisplayReferred,
    NonLinearLab,
    NonLinearLabDisplayReferred,
    LinearOrNonLinearLabDisplayReferred,
    LinearOrNonLinearRgbSceneReferred,
    NonLinearRgbOrLabDisplayReferred,
    GeometricRgb,
    FrequentialRgb,
    DefinedByProfile,
}

impl Fact {
    pub const ALL: [Fact; 20] = [
        Fact::LinearRawSceneReferred,
        Fact::LinearRaw,
        Fact::ReconstructionRaw,
        Fact::LinearRgbSceneReferred,
        Fact::LinearRgbDisplayReferred,
        Fact::LinearRgb,
        Fact::LinearRgbOrXyz,
        Fact::QuasiLinearRgb,
        Fact::QuasiLinearRgbSceneReferred,
        Fact::NonLinearRgb,
        Fact::NonLinearRgbSceneReferred,
        Fact::NonLinearRgbDisplayReferred,
        Fact::NonLinearLab,
        Fact::NonLinearLabDisplayReferred,
        Fact::LinearOrNonLinearLabDisplayReferred,
        Fact::LinearOrNonLinearRgbSceneReferred,
        Fact::NonLinearRgbOrLabDisplayReferred,
        Fact::GeometricRgb,
        Fact::FrequentialRgb,
        Fact::DefinedByProfile,
    ];

    /// As darktable writes it, its `msgid`.
    pub fn darktable(self) -> &'static str {
        match self {
            Fact::LinearRawSceneReferred => "linear, raw, scene-referred",
            Fact::LinearRaw => "linear, raw",
            Fact::ReconstructionRaw => "reconstruction, raw",
            Fact::LinearRgbSceneReferred => "linear, RGB, scene-referred",
            Fact::LinearRgbDisplayReferred => "linear, RGB, display-referred",
            Fact::LinearRgb => "linear, RGB",
            Fact::LinearRgbOrXyz => "linear, RGB or XYZ",
            Fact::QuasiLinearRgb => "quasi-linear, RGB",
            Fact::QuasiLinearRgbSceneReferred => "quasi-linear, RGB, scene-referred",
            Fact::NonLinearRgb => "non-linear, RGB",
            Fact::NonLinearRgbSceneReferred => "non-linear, RGB, scene-referred",
            Fact::NonLinearRgbDisplayReferred => "non-linear, RGB, display-referred",
            Fact::NonLinearLab => "non-linear, Lab",
            Fact::NonLinearLabDisplayReferred => "non-linear, Lab, display-referred",
            Fact::LinearOrNonLinearLabDisplayReferred => {
                "linear or non-linear, Lab, display-referred"
            }
            Fact::LinearOrNonLinearRgbSceneReferred => "linear or non-linear, RGB, scene-referred",
            Fact::NonLinearRgbOrLabDisplayReferred => "non-linear, RGB or Lab, display-referred",
            Fact::GeometricRgb => "geometric, RGB",
            Fact::FrequentialRgb => "frequential, RGB",
            Fact::DefinedByProfile => "defined by profile",
        }
    }

    /// Its words: `tool-fact-` and darktable's, lower case, each run of anything but
    /// letters and digits a hyphen ([`messages::fact`]).
    pub fn message(self) -> String {
        messages::fact(self.darktable())
    }

    /// Whether one of darktable's comma-separated parts is `part`.
    fn says(self, part: &str) -> bool {
        self.darktable().split(", ").any(|p| p == part)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(purpose: Purpose, input: Fact, output: Fact) -> Facts {
        Facts {
            purpose,
            input,
            process: Fact::LinearRgb,
            output,
        }
    }

    /// The mapping table of `docs/gui.md`'s "The header's tooltip": darktable's purpose
    /// to Kind, its input and output to Works on.
    #[test]
    fn darktable_s_facts_map_to_the_plain_words_as_gui_md_s_table_does() {
        let kinds = [
            (Purpose::Corrective, Kind::Fixes),
            (Purpose::Creative, Kind::Creates),
            (Purpose::CorrectiveAndCreative, Kind::FixesAndShapes),
            (Purpose::CorrectiveOrCreative, Kind::FixesOrCreates),
            (Purpose::Mandatory, Kind::Needed),
        ];
        for (purpose, kind) in kinds {
            assert_eq!(purpose.kind(), kind, "{purpose:?}");
        }
        let places = [
            (
                Fact::LinearRawSceneReferred,
                Fact::LinearRgbSceneReferred,
                WorksOn::Raw,
            ),
            (
                Fact::LinearRawSceneReferred,
                Fact::LinearRawSceneReferred,
                WorksOn::Raw,
            ),
            (
                Fact::LinearRgbSceneReferred,
                Fact::LinearRgbSceneReferred,
                WorksOn::Scene,
            ),
            (
                Fact::LinearRgbSceneReferred,
                Fact::LinearRgbDisplayReferred,
                WorksOn::SceneToDisplay,
            ),
            (
                Fact::NonLinearLabDisplayReferred,
                Fact::NonLinearLabDisplayReferred,
                WorksOn::Display,
            ),
            (
                Fact::LinearOrNonLinearLabDisplayReferred,
                Fact::NonLinearRgbOrLabDisplayReferred,
                WorksOn::Display,
            ),
        ];
        for (input, output, place) in places {
            let facts = facts(Purpose::Corrective, input, output);
            assert_eq!(facts.works_on(), Some(place), "{input:?} to {output:?}");
        }
        // Display in, scene out: no plain words for it, as no darktable module does it.
        let back = facts(
            Purpose::Creative,
            Fact::LinearRgbDisplayReferred,
            Fact::LinearRgbSceneReferred,
        );
        assert_eq!(back.works_on(), None);
    }

    #[test]
    fn each_fact_s_message_follows_from_darktable_s_words() {
        assert_eq!(
            Fact::LinearOrNonLinearLabDisplayReferred.message(),
            "tool-fact-linear-or-non-linear-lab-display-referred"
        );
        assert_eq!(
            Fact::DefinedByProfile.message(),
            "tool-fact-defined-by-profile"
        );
        let mut ids: Vec<String> = Fact::ALL.iter().map(|f| f.message()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), Fact::ALL.len(), "two facts share a message");
    }
}
