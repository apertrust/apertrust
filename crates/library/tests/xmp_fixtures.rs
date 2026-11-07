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

//! darktable's own `.xmp` files (`tests/fixtures/xmp/darktable-*.xmp`, from its integration
//! tests, README beside them) through the XMP layer (decision 50): what darktable writes is
//! read whole, and what the layer writes reads back to the same tree.

use std::path::Path;

use apertrust_library::xmp::{ArrayKind, DEFAULT_LANGUAGE, Value, Xmp};

const XMP: &str = "http://ns.adobe.com/xap/1.0/";
const DC: &str = "http://purl.org/dc/elements/1.1/";
const DARKTABLE: &str = "http://darktable.sf.net/";

fn fixtures() -> Vec<(String, String)> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/xmp");
    let mut found: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("darktable-"))
        })
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).unwrap())
        })
        .collect();
    found.sort();
    found
}

#[test]
fn darktable_s_files_are_read_whole() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 4, "the four darktable fixtures");
    for (name, text) in fixtures {
        let xmp = Xmp::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            xmp.toolkit.as_deref(),
            Some("XMP Core 4.4.0-Exiv2"),
            "{name}"
        );
        assert_eq!(xmp.about, "", "{name}");
        assert_eq!(
            xmp.get(XMP, "Rating").and_then(|n| n.as_str()),
            Some("1"),
            "{name}"
        );
        // Its history: a Seq of structs, one for each `darktable:operation` in the text,
        // each with its number counting up and its operation.
        let history = xmp.get(DARKTABLE, "history").expect(&name);
        let Value::Array(ArrayKind::Seq, items) = &history.value else {
            panic!("{name}: {history:?}");
        };
        assert_eq!(
            items.len(),
            text.matches("darktable:operation=").count(),
            "{name}"
        );
        assert!(!items.is_empty(), "{name}");
        for (index, item) in items.iter().enumerate() {
            let number = item.field(DARKTABLE, "num").and_then(|n| n.as_str());
            assert_eq!(number, Some(index.to_string().as_str()), "{name}");
            assert!(item.field(DARKTABLE, "operation").is_some(), "{name}");
            assert!(item.field(DARKTABLE, "params").is_some(), "{name}");
            assert!(item.field(DARKTABLE, "blendop_params").is_some(), "{name}");
        }
    }
}

#[test]
fn the_tags_and_rights_darktable_keeps_in_bags_and_alternatives_are_read() {
    let fixtures = fixtures();
    let read = |name: &str| {
        let text = &fixtures.iter().find(|(n, _)| n == name).unwrap().1;
        Xmp::parse(text).unwrap()
    };
    let strings = |node: Option<&apertrust_library::xmp::Node>| -> Vec<String> {
        node.and_then(|n| n.items())
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_owned())
            .collect()
    };
    let tagged = read("darktable-highlight-opposed.xmp");
    assert_eq!(strings(tagged.get(DC, "subject")), ["Folk", "toto"]);
    assert_eq!(
        strings(tagged.get("http://ns.adobe.com/lightroom/1.0/", "hierarchicalSubject")),
        ["Folk", "toto"]
    );
    let rights = read("darktable-exposure.xmp");
    let rights = rights.get(DC, "rights").unwrap();
    assert!(matches!(rights.value, Value::Array(ArrayKind::Alt, _)));
    let item = &rights.items().unwrap()[0];
    assert_eq!(item.lang(), Some(DEFAULT_LANGUAGE));
    // Escaped newlines are newlines, and the accents are the accents.
    assert_eq!(
        item.as_str(),
        Some(
            "Creative Commons \nPaternité\nPartage des conditions initiales à l'identique (CC-BY-SA)"
        )
    );
    assert_eq!(
        read("darktable-exposure.xmp")
            .get(DC, "publisher")
            .map(|n| strings(Some(n))),
        Some(vec!["pascal@obry.net".to_owned()])
    );
}

#[test]
fn darktable_s_files_written_back_read_to_the_same_tree() {
    for (name, text) in fixtures() {
        let xmp = Xmp::parse(&text).unwrap();
        let written = xmp.to_xml();
        let again = Xmp::parse(&written).unwrap_or_else(|e| panic!("{name}: {e}\n{written}"));
        assert_eq!(again, xmp, "{name}");
        assert_eq!(again.to_xml(), written, "{name}");
        // Nothing darktable wrote is lost: every property keeps its place and prefix.
        let names = |x: &Xmp| -> Vec<String> {
            x.properties
                .iter()
                .map(|p| format!("{}:{}", p.name.prefix, p.name.local))
                .collect()
        };
        assert_eq!(names(&again), names(&xmp), "{name}");
    }
}

#[test]
fn a_property_set_leaves_the_rest_as_darktable_wrote_it() {
    // How a caller owns a few properties and keeps the others: set what is ours.
    let (_, text) = &fixtures()[0];
    let mut xmp = Xmp::parse(text).unwrap();
    let before = xmp.clone();
    xmp.set(apertrust_library::xmp::Property::simple(
        apertrust_library::xmp::Name::new("xmp", XMP, "Rating"),
        "4",
    ));
    assert_eq!(xmp.get(XMP, "Rating").unwrap().as_str(), Some("4"));
    let mut restored = xmp.clone();
    restored.set(
        before
            .properties
            .iter()
            .find(|p| p.name.is(XMP, "Rating"))
            .unwrap()
            .clone(),
    );
    assert_eq!(restored, before);
    xmp.remove(DARKTABLE, "history");
    assert!(xmp.get(DARKTABLE, "history").is_none());
    assert_eq!(xmp.properties.len(), before.properties.len() - 1);
}
