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

use super::*;

/// A version 1 file with one module entry, with these attributes.
fn version_1(attributes: &str) -> String {
    format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="{RDF_NAMESPACE}">
        <rdf:Description rdf:about="" xmlns:apertrust="{NAMESPACE}" xmlns:c="{CONTROL_NAMESPACE}" apertrust:version="1">
        <apertrust:modules><rdf:Seq><rdf:li>
        <rdf:Description {attributes}/>
        </rdf:li></rdf:Seq></apertrust:modules></rdf:Description></rdf:RDF></x:xmpmeta>"#
    )
}

fn module(id: &str) -> Entry {
    Entry::Module {
        module: id.to_owned(),
        version: Some("2".to_owned()),
        kept: vec!["a".to_owned(), "b".to_owned()],
        auto: vec!["c".to_owned()],
        controls: vec![
            ("a".to_owned(), "0.3".to_owned()),
            ("curve".to_owned(), "0.25,0.2 0.75,0.8".to_owned()),
            ("name".to_owned(), "R&D \"x\" <y>".to_owned()),
        ],
    }
}

fn operation(id: &str) -> Entry {
    Entry::Operation {
        operation: id.to_owned(),
        schema: Some(2),
        params: vec![("amount".to_owned(), "0.3".to_owned())],
    }
}

#[test]
fn a_document_reads_back_as_written() {
    let mut document = Document::new("0.4.0+12~g1a2b3c4");
    document.entries = vec![module("look"), module("lens"), operation("fake")];
    let xml = document.to_xml();
    for written in [
        r#"apertrust:version="1""#,
        r#"x:xmptk="Apertrust 0.4.0+12~g1a2b3c4""#,
        r#"apertrust:module="look""#,
        r#"apertrust:moduleVersion="2""#,
        r#"apertrust:kept="a b""#,
        r#"apertrust:auto="c""#,
        r#"control:curve="0.25,0.2 0.75,0.8""#,
        r#"control:name="R&amp;D &quot;x&quot; &lt;y&gt;""#,
        r#"apertrust:schema="2""#,
        r#"param:amount="0.3""#,
    ] {
        assert!(xml.contains(written), "{written} in {xml}");
    }
    assert_eq!(Document::parse(&xml).unwrap(), document);
}

#[test]
fn modules_are_written_before_operations_each_in_its_order() {
    let mut document = Document::new("test");
    document.entries = vec![operation("b"), module("y"), operation("a"), module("x")];
    let xml = document.to_xml();
    let at = |text: &str| xml.find(text).unwrap_or_else(|| panic!("{text} in {xml}"));
    assert!(at(r#"apertrust:module="y""#) < at(r#"apertrust:module="x""#));
    assert!(at(r#"apertrust:module="x""#) < at(r#"apertrust:operation="b""#));
    assert!(at(r#"apertrust:operation="b""#) < at(r#"apertrust:operation="a""#));
}

#[test]
fn the_build_is_recorded_escaped_and_read_back() {
    let xml = Document::new("0.4.0+12~g1a2b3c4").to_xml();
    assert!(
        xml.contains(r#"x:xmptk="Apertrust 0.4.0+12~g1a2b3c4""#),
        "{xml}"
    );
    let read = Document::parse(&xml).unwrap();
    assert_eq!(read.build.as_deref(), Some("0.4.0+12~g1a2b3c4"));
    // A packager's version is written as given, escaped.
    let xml = Document::new("1.0 \"R&D\"").to_xml();
    let read = Document::parse(&xml).unwrap();
    assert_eq!(read.build.as_deref(), Some("1.0 \"R&D\""));
    // A file that names no Apertrust build, as one another tool rewrote, or an empty
    // one.
    let read = Document::parse(&version_1("")).unwrap();
    assert_eq!(read.build, None);
    let xml = Document::new(" ").to_xml();
    assert!(xml.contains(r#"x:xmptk="Apertrust  ""#), "{xml}");
    assert_eq!(Document::parse(&xml).unwrap().build, None);
}

#[test]
fn an_empty_document_has_no_entries() {
    let xml = Document::new("test").to_xml();
    assert!(!xml.contains("apertrust:operation="), "{xml}");
    assert!(!xml.contains("apertrust:module="), "{xml}");
    let read = Document::parse(&xml).unwrap();
    assert!(read.entries.is_empty());
    assert_eq!(read.version, VERSION);
    assert_eq!(read.newer(), None);
}

#[test]
fn entries_are_read_as_text_in_the_file_s_order() {
    let xml = version_1(
        r#"apertrust:module="look" apertrust:moduleVersion="2.0" apertrust:kept="strength  grain" apertrust:auto="" c:strength="1.5" c:style="medium""#,
    );
    let read = Document::parse(&xml).unwrap();
    assert_eq!(
        read.entries,
        [Entry::Module {
            module: "look".to_owned(),
            version: Some("2.0".to_owned()),
            kept: vec!["strength".to_owned(), "grain".to_owned()],
            auto: Vec::new(),
            controls: vec![
                ("strength".to_owned(), "1.5".to_owned()),
                ("style".to_owned(), "medium".to_owned()),
            ],
        }]
    );
    // A schema that is no number is none; a description with neither a module nor an
    // operation is no entry.
    let xml = xml.replace(
        "<rdf:li>",
        r#"<rdf:li><rdf:Description apertrust:operation="fake" apertrust:schema="x"/></rdf:li><rdf:li><rdf:Description rdf:about=""/></rdf:li><rdf:li>"#,
    );
    let read = Document::parse(&xml).unwrap();
    assert_eq!(read.entries.len(), 2);
    assert_eq!(
        read.entries[0],
        Entry::Operation {
            operation: "fake".to_owned(),
            schema: None,
            params: Vec::new(),
        }
    );
}

/// A newer file version is read as this version lays it out, and told newer.
#[test]
fn a_newer_file_version_is_newer() {
    let xml = Document::new("0.9.0")
        .to_xml()
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="2""#);
    let read = Document::parse(&xml).unwrap();
    assert_eq!(read.version, 2);
    assert_eq!(read.build.as_deref(), Some("0.9.0"));
    assert_eq!(read.newer(), Some(Newer::Version(2)));
    let view_only = ViewOnly {
        build: read.build,
        newer: Newer::Version(2),
    };
    assert_eq!(
        view_only.to_string(),
        "the edit Apertrust 0.9.0 wrote is version 2, newer than this build's 1: open for viewing only"
    );
    let view_only = ViewOnly {
        build: None,
        newer: Newer::Module("lens".to_owned()),
    };
    assert_eq!(
        view_only.to_string(),
        "the edit a newer Apertrust wrote holds the module lens, which this build does not have: open for viewing only"
    );
    // Version 0 still reads, as an older file.
    let xml = Document::new("0.1.0")
        .to_xml()
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="0""#);
    let read = Document::parse(&xml).unwrap();
    assert_eq!((read.version, read.newer()), (0, None));
}

#[test]
fn a_version_that_is_no_number_or_a_foreign_file_is_refused() {
    let unreadable =
        version_1("").replace(r#"apertrust:version="1""#, r#"apertrust:version="2.0""#);
    assert!(matches!(
        Document::parse(&unreadable),
        Err(SidecarError::Version(version)) if version == "2.0"
    ));
    let darktable = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about=""/></rdf:RDF></x:xmpmeta>"#;
    assert!(matches!(
        Document::parse(darktable),
        Err(SidecarError::NotAnEdit)
    ));
    assert!(matches!(
        Document::parse("<x:xmpmeta"),
        Err(SidecarError::Xml(_))
    ));
}

/// Apertrust's layout under another namespace is no Apertrust edit: only `apertrust.org`'s
/// are read (decision 48).
#[test]
fn an_edit_in_another_namespace_is_refused() {
    let other = version_1(r#"apertrust:module="look" c:a="0.3""#).replace(
        "https://apertrust.org/ns/xmp/",
        "https://example.org/other/",
    );
    assert!(!other.contains("apertrust.org"), "{other}");
    assert!(matches!(
        Document::parse(&other),
        Err(SidecarError::NotAnEdit)
    ));
}

#[test]
fn a_sidecar_sits_beside_the_photo_and_is_replaced_whole() {
    assert_eq!(
        path_for(Path::new("/photos/IMG_2041.ARW")),
        Path::new("/photos/IMG_2041.ARW.apxmp")
    );
    assert_eq!(
        file_name(Path::new("/photos/IMG.ARW.apxmp")),
        "IMG.ARW.apxmp"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = path_for(&directory.path().join("IMG.ARW"));
    let mut document = Document::new("test");
    document.entries = vec![module("look"), operation("fake")];
    write_xml(&path, &document.to_xml()).unwrap();
    document.entries.pop();
    write_xml(&path, &document.to_xml()).unwrap();
    assert_eq!(read(&path).unwrap(), document);
    let names: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["IMG.ARW.apxmp"]);
    assert!(matches!(
        read(&directory.path().join("missing.apxmp")),
        Err(SidecarError::Io(_))
    ));
}
