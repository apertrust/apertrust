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

//! What each tool is for, as its header's tooltip says it (`docs/gui.md`, The header's
//! tooltip): every built tool's facts are darktable's `description()` at the reference
//! release, word for word; every tool and utility has its sentence in every language; and
//! darktable's lines read as darktable writes them, in its own Ukrainian and Polish.

use apertrust_tools::TOOLS;
use apertrust_tools::about::{Fact, Kind, WorksOn};
use apertrust_tools::messages;
use apertrust_tools::utility::UTILITIES;
use apertrust_ui::{Locale, Translator};

/// darktable's `description()` of each built tool's module at the reference release,
/// past its first line: the tool's id, the module's source in `src/iop/`, then its
/// purpose, input, process and output, as `dt_iop_set_description` is given them.
const DARKTABLE: &[(&str, &str, [&str; 4])] = &[
    (
        "rawprepare",
        "rawprepare.c",
        [
            "mandatory",
            "linear, raw, scene-referred",
            "linear, raw",
            "linear, raw, scene-referred",
        ],
    ),
    (
        "highlights",
        "highlights.c",
        [
            "corrective",
            "linear, raw, scene-referred",
            "reconstruction, raw",
            "linear, raw, scene-referred",
        ],
    ),
    (
        "demosaic",
        "demosaic.c",
        [
            "mandatory",
            "linear, raw, scene-referred",
            "linear, raw",
            "linear, RGB, scene-referred",
        ],
    ),
    (
        "agx",
        "agx.c",
        [
            "corrective and creative",
            "linear, RGB, scene-referred",
            "non-linear, RGB",
            "linear, RGB, display-referred",
        ],
    ),
    (
        "colorout",
        "colorout.c",
        [
            "mandatory",
            "linear or non-linear, Lab, display-referred",
            "defined by profile",
            "non-linear, RGB or Lab, display-referred",
        ],
    ),
    (
        "temperature",
        "temperature.c",
        [
            "corrective",
            "linear, raw, scene-referred",
            "linear, raw",
            "linear, raw, scene-referred",
        ],
    ),
    (
        "flip",
        "flip.c",
        [
            "corrective",
            "linear, RGB, scene-referred",
            "geometric, RGB",
            "linear, RGB, scene-referred",
        ],
    ),
    (
        "exposure",
        "exposure.c",
        [
            "corrective and creative",
            "linear, RGB, scene-referred",
            "linear, RGB",
            "linear, RGB, scene-referred",
        ],
    ),
    (
        "channelmixerrgb",
        "channelmixerrgb.c",
        [
            "corrective or creative",
            "linear, RGB, scene-referred",
            "linear, RGB or XYZ",
            "linear, RGB, scene-referred",
        ],
    ),
    (
        "toneequal",
        "toneequal.c",
        [
            "corrective and creative",
            "linear, RGB, scene-referred",
            "quasi-linear, RGB",
            "quasi-linear, RGB, scene-referred",
        ],
    ),
    (
        "colorbalancergb",
        "colorbalancergb.c",
        [
            "corrective or creative",
            "linear, RGB, scene-referred",
            "non-linear, RGB",
            "non-linear, RGB, scene-referred",
        ],
    ),
    // The Luma curve becomes RGB curve's linked curve, and drives `rgbcurve`.
    (
        "tone_curve",
        "rgbcurve.c",
        [
            "corrective and creative",
            "linear, RGB, display-referred",
            "non-linear, RGB",
            "linear, RGB, display-referred",
        ],
    ),
    (
        "hazeremoval",
        "hazeremoval.c",
        [
            "corrective",
            "linear, RGB, scene-referred",
            "frequential, RGB",
            "linear, RGB, scene-referred",
        ],
    ),
    (
        "primaries",
        "primaries.c",
        [
            "corrective or creative",
            "linear, RGB, scene-referred",
            "linear, RGB",
            "linear, RGB, scene-referred",
        ],
    ),
    (
        "grain",
        "grain.c",
        [
            "creative",
            "non-linear, Lab, display-referred",
            "non-linear, Lab",
            "non-linear, Lab, display-referred",
        ],
    ),
    (
        "soften",
        "soften.c",
        [
            "creative",
            "linear, RGB, display-referred",
            "linear, RGB",
            "linear, RGB, display-referred",
        ],
    ),
    (
        "vignette",
        "vignette.c",
        [
            "creative",
            "non-linear, RGB, display-referred",
            "non-linear, RGB",
            "non-linear, RGB, display-referred",
        ],
    ),
];

/// darktable's own translations of its input, process and output lines, its `msgstr` in
/// `po/uk.po` and `po/pl.po` at the reference release.
const TRANSLATED: &[(&str, &str, &str)] = &[
    (
        "linear, raw, scene-referred",
        "лінійний простір, raw, на основі сцен",
        "liniowe, raw, scenocentryczny",
    ),
    ("linear, raw", "лінійна, raw", "liniowe, raw"),
    (
        "reconstruction, raw",
        "відновлювальна, raw",
        "rekonstrukcja, raw",
    ),
    (
        "linear, RGB, scene-referred",
        "лінійний простір, RGB, на основі сцен",
        "liniowe, RGB, scenocentryczny",
    ),
    (
        "linear, RGB, display-referred",
        "лінійний простір, RGB, на основі відображення",
        "liniowe, RGB, ekranocentryczny",
    ),
    ("linear, RGB", "лінійна, RGB", "liniowe, RGB"),
    (
        "linear, RGB or XYZ",
        "лінійна, RGB або XYZ",
        "liniowe, RGB lub XYZ",
    ),
    (
        "quasi-linear, RGB",
        "квазілінійна, RGB",
        "quasi-liniowe, RGB",
    ),
    (
        "quasi-linear, RGB, scene-referred",
        "квазілінійний простір, RGB, на основі сцен",
        "quasi-liniowe, RGB, scenocentryczny",
    ),
    ("non-linear, RGB", "нелінійна, RGB", "nieliniowe, RGB"),
    (
        "non-linear, RGB, scene-referred",
        "нелінійний простір, RGB, на основі сцен",
        "nieliniowe, RGB, scenocentryczny",
    ),
    (
        "non-linear, RGB, display-referred",
        "нелінійний простір, RGB, на основі відображення",
        "nieliniowe, RGB, ekranocentryczny",
    ),
    ("non-linear, Lab", "нелінійна, Lab", "nieliniowe, Lab"),
    (
        "non-linear, Lab, display-referred",
        "нелінійний простір, Lab, на основі відображення",
        "nieliniowe, Lab, ekranocentryczny",
    ),
    (
        "linear or non-linear, Lab, display-referred",
        "лінійний або нелінійний простір, Lab, на основі відображення",
        "liniowe lub nieliniowe, Lab, ekranocentryczny",
    ),
    (
        "non-linear, RGB or Lab, display-referred",
        "нелінійний простір, RGB або Lab, на основі відображення",
        "nieliniowe, RGB lub Lab, ekranocentryczny",
    ),
    ("geometric, RGB", "геометрична, RGB", "geometryczne, RGB"),
    (
        "frequential, RGB",
        "частотна, RGB",
        "częstotliwościowy, RGB",
    ),
    (
        "defined by profile",
        "Визначається профілем",
        "zdefiniowany przez profil",
    ),
];

#[test]
fn every_built_tool_s_facts_are_darktable_s_word_for_word() {
    let ids: Vec<&str> = TOOLS.iter().map(|tool| tool.definition().tool.id).collect();
    let listed: Vec<&str> = DARKTABLE.iter().map(|(id, _, _)| *id).collect();
    let mut sorted = (ids.clone(), listed.clone());
    sorted.0.sort_unstable();
    sorted.1.sort_unstable();
    assert_eq!(sorted.0, sorted.1, "every tool, and only those, is listed");
    for tool in TOOLS {
        let definition = tool.definition();
        let (_, source, [purpose, input, process, output]) = DARKTABLE
            .iter()
            .find(|(id, _, _)| *id == definition.tool.id)
            .expect("listed");
        let facts = definition
            .facts
            .unwrap_or_else(|| panic!("{} has no facts", definition.tool.id));
        assert_eq!(
            [
                facts.purpose.darktable(),
                facts.input.darktable(),
                facts.process.darktable(),
                facts.output.darktable(),
            ],
            [*purpose, *input, *process, *output],
            "{} against src/iop/{source}",
            definition.tool.id
        );
        assert!(
            facts.works_on().is_some(),
            "{} works on nothing the tooltip can say",
            definition.tool.id
        );
    }
}

/// The kind and what each works on, in plain words, for the tools `docs/gui.md`'s
/// "The header's tooltip" shows.
#[test]
fn the_plain_words_are_the_designed_ones_for_the_tools_shown() {
    let expected = [
        ("exposure", Kind::FixesAndShapes, WorksOn::Scene),
        ("temperature", Kind::Fixes, WorksOn::Raw),
        ("colorbalancergb", Kind::FixesOrCreates, WorksOn::Scene),
        ("agx", Kind::FixesAndShapes, WorksOn::SceneToDisplay),
        ("toneequal", Kind::FixesAndShapes, WorksOn::Scene),
        ("channelmixerrgb", Kind::FixesOrCreates, WorksOn::Scene),
        ("highlights", Kind::Fixes, WorksOn::Raw),
        ("vignette", Kind::Creates, WorksOn::Display),
        ("grain", Kind::Creates, WorksOn::Display),
    ];
    for (id, kind, place) in expected {
        let tool = TOOLS
            .iter()
            .find(|tool| tool.definition().tool.id == id)
            .expect("built");
        let facts = tool.definition().facts.expect("darktable's");
        assert_eq!(
            (facts.kind(), facts.works_on()),
            (kind, Some(place)),
            "{id}"
        );
    }
}

/// The utilities whose headers say nothing yet, their sentences a design question of their
/// own: Browse and Export, which joined [`UTILITIES`] with no tooltip, as each is drawn
/// (decision 93).
const SAYS_NOTHING: [&str; 2] = ["browse", "export"];

#[test]
fn every_tool_and_utility_says_what_it_is_for_in_every_language() {
    let ids = TOOLS
        .iter()
        .map(|tool| tool.definition().tool.id)
        .chain(UTILITIES.iter().map(|utility| utility.tool.id))
        .filter(|id| !SAYS_NOTHING.contains(id));
    let mut wanted: Vec<String> = ids.map(messages::description).collect();
    wanted.extend(Kind::ALL.map(|kind| kind.message().to_owned()));
    wanted.extend(WorksOn::ALL.map(|place| place.message().to_owned()));
    wanted.extend(Fact::ALL.map(Fact::message));
    for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
        let words = Translator::new(locale);
        let missing: Vec<&String> = wanted.iter().filter(|id| !words.has(id)).collect();
        assert!(missing.is_empty(), "{}: no {missing:?}", locale.id());
    }
}

#[test]
fn darktable_s_lines_read_as_darktable_writes_and_translates_them() {
    assert_eq!(TRANSLATED.len(), Fact::ALL.len());
    let english = Translator::new(Locale::EnGb);
    let american = Translator::new(Locale::EnUs);
    let ukrainian = Translator::new(Locale::Uk);
    let polish = Translator::new(Locale::Pl);
    for fact in Fact::ALL {
        let id = fact.message();
        let (_, uk, pl) = TRANSLATED
            .iter()
            .find(|(en, _, _)| *en == fact.darktable())
            .unwrap_or_else(|| panic!("{} has no translations listed", fact.darktable()));
        assert_eq!(english.text(&id), fact.darktable());
        assert_eq!(american.text(&id), fact.darktable());
        assert_eq!(ukrainian.text(&id), *uk);
        assert_eq!(polish.text(&id), *pl);
    }
}

/// A sentence or two, in the style `docs/gui.md`'s "The header's tooltip" gives: no darktable words, and a line's worth
/// at most twice over.
#[test]
fn each_description_is_a_sentence_or_two() {
    let english = Translator::new(Locale::EnGb);
    let ids = TOOLS
        .iter()
        .map(|tool| tool.definition().tool.id)
        .chain(UTILITIES.iter().map(|utility| utility.tool.id))
        .filter(|id| !SAYS_NOTHING.contains(id));
    for id in ids {
        let text = english.text(&messages::description(id));
        let sentences = text.matches(". ").count() + usize::from(text.ends_with('.'));
        assert!((1..=2).contains(&sentences), "{id}: {text}");
        for word in ["darktable", "scene-referred", "display-referred", "module"] {
            assert!(!text.contains(word), "{id} says {word}: {text}");
        }
    }
}
