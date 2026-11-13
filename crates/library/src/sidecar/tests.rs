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
use crate::clock::{Clock, MachineId};

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
        enabled: (id == "look").then(|| "true".to_owned()),
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
        r#"apertrust:enabled="true""#,
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
    // A module off writes no `enabled`.
    assert_eq!(xml.matches("apertrust:enabled").count(), 1, "{xml}");
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
    assert_eq!(read.version, 1);
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
            enabled: None,
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
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
    let read = Document::parse(&xml).unwrap();
    assert_eq!(read.version, 3);
    assert_eq!(read.build.as_deref(), Some("0.9.0"));
    assert_eq!(read.newer(), Some(Newer::Version(3)));
    let view_only = ViewOnly {
        build: read.build,
        reason: Reason::Newer(Newer::Version(3)),
    };
    assert_eq!(
        view_only.to_string(),
        "the edit Apertrust 0.9.0 wrote is version 3, newer than this build's 2: open for viewing only"
    );
    let view_only = ViewOnly {
        build: None,
        reason: Reason::Newer(Newer::Module("lens".to_owned())),
    };
    assert_eq!(
        view_only.to_string(),
        "the edit a newer Apertrust wrote holds the module lens, which this build does not have: open for viewing only"
    );
    // Version 2, this build's, is not newer, whatever layout a document reads.
    let xml = Document::new("0.9.0")
        .to_xml()
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="2""#);
    assert_eq!(Document::parse(&xml).unwrap().newer(), None);
    let view_only = ViewOnly {
        build: Some("0.9.0".to_owned()),
        reason: Reason::Unreadable(Problem::End {
            copy: "7c0e9a14".to_owned(),
            end: "a1".to_owned(),
        }),
    };
    assert_eq!(
        view_only.to_string(),
        "the edit Apertrust 0.9.0 wrote holds history this build cannot read (copy 7c0e9a14's end a1 names none of its steps): open for viewing only"
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

// -- Version 2 ----------------------------------------------------------------------------

/// 2026-03-14T14:02:11.120Z.
const PI_DAY: u64 = 1_773_496_931_120;

fn clock(when: u64, counter: u32, machine: &str) -> Clock {
    Clock {
        when,
        counter,
        machine: machine.parse().unwrap(),
    }
}

fn step(id: &str, clock: Clock, note: Option<Note>, entries: Vec<Entry>) -> Step {
    Step {
        id: id.to_owned(),
        clock: Some(clock),
        source: "you".to_owned(),
        note,
        batch: None,
        entries,
    }
}

/// A file with one virtual copy of three steps, the end on the second: an assistant's
/// words with a batch, a module and an operation in one step, and a message with its
/// arguments after the end.
fn edits() -> Edits {
    let mut edits = Edits::new("0.5.0+3~g1a2b3c4");
    edits.photo = Some("DSCF0001.RAF".to_owned());
    edits.fingerprint = Some(fingerprint(3, b"raw", b"raw"));
    let mut assistant = step(
        "00000000000000b2",
        clock(PI_DAY + 1_000, 0, "5e1f22c9"),
        Some(Note::Text(
            "Warmer, as asked: \"a bit\" & <more>".to_owned(),
        )),
        vec![operation("fake"), module("look")],
    );
    assistant.source = "assistant".to_owned();
    assistant.batch = Some("0f1e2d3c4b5a6978".to_owned());
    let message = step(
        "00000000000000c3",
        clock(PI_DAY + 2_000, 1, "0000beef"),
        Some(Note::Message {
            id: "history-applied".to_owned(),
            args: vec![
                ("photo".to_owned(), "IMG_2041".to_owned()),
                ("count".to_owned(), "3".to_owned()),
            ],
        }),
        vec![Entry::Module {
            module: "look".to_owned(),
            version: Some("2".to_owned()),
            kept: Vec::new(),
            auto: Vec::new(),
            enabled: None,
            controls: Vec::new(),
        }],
    );
    edits.copies.push(VirtualCopy {
        id: "7c0e9a14".to_owned(),
        number: "0".to_owned(),
        name: "Warm".to_owned(),
        end: "00000000000000b2".to_owned(),
        // In the order of their machines, as they are written.
        clocks: vec![
            clock(PI_DAY + 2_000, 1, "0000beef"),
            clock(PI_DAY + 1_000, 0, "5e1f22c9"),
        ],
        history: vec![
            step(
                "00000000000000a1",
                clock(PI_DAY, 0, "5e1f22c9"),
                None,
                vec![module("lens")],
            ),
            assistant,
            message,
        ],
    });
    edits
}

#[test]
fn version_2_reads_back_as_written() {
    let edits = edits();
    let xml = edits.to_xml();
    for written in [
        r#"apertrust:version="2""#,
        r#"x:xmptk="Apertrust 0.5.0+3~g1a2b3c4""#,
        r#"xmlns:arg="https://apertrust.org/ns/xmp/arg/""#,
        r#"apertrust:photo="DSCF0001.RAF""#,
        r#"apertrust:fingerprint="3:"#,
        r#"apertrust:id="7c0e9a14""#,
        r#"apertrust:number="0""#,
        r#"apertrust:name="Warm""#,
        r#"apertrust:end="00000000000000b2""#,
        "<rdf:Bag>",
        r#"apertrust:when="2026-03-14T14:02:11.120Z""#,
        r#"apertrust:when="2026-03-14T14:02:13.120Z""#,
        r#"apertrust:machine="0000beef""#,
        r#"apertrust:source="assistant""#,
        r#"apertrust:note="Warmer, as asked: &quot;a bit&quot; &amp; &lt;more&gt;""#,
        r#"apertrust:batch="0f1e2d3c4b5a6978""#,
        r#"apertrust:noteMessage="history-applied""#,
        r#"arg:photo="IMG_2041""#,
        r#"arg:count="3""#,
        r#"apertrust:module="look""#,
        r#"param:amount="0.3""#,
    ] {
        assert!(xml.contains(written), "{written} in {xml}");
    }
    // A step's entries keep its order, here an operation's before a module's.
    let at = |text: &str| xml.find(text).unwrap_or_else(|| panic!("{text} in {xml}"));
    assert!(at(r#"apertrust:operation="fake""#) < at(r#"apertrust:module="look""#));
    assert_eq!(xml.matches("<apertrust:entries>").count(), 3, "{xml}");
    // The arguments are a struct of their own, after the step's own fields.
    assert!(at(r#"apertrust:batch="0f1e"#) < at("<apertrust:noteArgs>"));
    assert!(at("<apertrust:noteArgs>") < at(r#"arg:photo="IMG_2041""#));
    assert_eq!(xml.matches("<apertrust:noteArgs>").count(), 1, "{xml}");
    let read = Edits::parse(&xml, 0).unwrap();
    assert_eq!(read, edits);
    assert!(read.problems.is_empty());
    assert_eq!(read.to_xml(), xml);
}

/// The clock vector is written in the order of its machines, so two machines writing the
/// same copy write the same bytes.
#[test]
fn the_clock_vector_is_written_in_the_order_of_its_machines() {
    let mut reordered = edits();
    reordered.copies[0].clocks.reverse();
    assert_ne!(reordered, edits());
    let xml = reordered.to_xml();
    assert_eq!(xml, edits().to_xml());
    let at = |text: &str| xml.find(text).unwrap();
    assert!(at(r#"apertrust:machine="0000beef""#) < at(r#"apertrust:machine="5e1f22c9""#));
}

#[test]
fn a_newer_version_is_newer() {
    let mut edits = edits();
    assert_eq!(edits.newer(), None);
    edits.version = COPIES_VERSION + 1;
    assert_eq!(edits.newer(), Some(Newer::Version(COPIES_VERSION + 1)));
}

#[test]
fn an_empty_version_2_file_reads_back_empty() {
    let mut edits = Edits::new("test");
    let xml = edits.to_xml();
    assert!(!xml.contains("apertrust:photo"), "{xml}");
    assert_eq!(Edits::parse(&xml, 0).unwrap(), edits);
    // A copy at Original, with no step, no clock and no name.
    edits.copies.push(VirtualCopy {
        id: "7c0e9a14".to_owned(),
        number: "0".to_owned(),
        name: String::new(),
        end: String::new(),
        clocks: Vec::new(),
        history: Vec::new(),
    });
    let xml = edits.to_xml();
    assert!(xml.contains(r#"apertrust:end="""#), "{xml}");
    assert_eq!(Edits::parse(&xml, 0).unwrap(), edits);
}

/// A version 1 file reads as a step of each entry, modules first, each in id order, sharing
/// a batch: the same ids on every read, from its bytes, the first the hash's first 64 bits
/// and each next one more, the batch its next 64; their clock the file's modified time on
/// no known machine; the end the last. One entry is one step in no batch.
#[test]
fn version_1_reads_as_a_step_of_each_module() {
    let mut document = Document::new("0.4.0");
    document.entries = vec![operation("fake"), module("look"), module("lens")];
    let xml = document.to_xml();
    let read = Edits::parse(&xml, PI_DAY).unwrap();
    assert_eq!(read, Edits::parse(&xml, PI_DAY).unwrap());
    assert_eq!((read.version, read.build.as_deref()), (1, Some("0.4.0")));
    assert_eq!((&read.photo, &read.fingerprint), (&None, &None));
    let [copy] = read.copies.as_slice() else {
        panic!("one copy, not {:?}", read.copies);
    };
    let hash = crate::index::hash(xml.as_bytes());
    let bits = |at: usize| u64::from_be_bytes(hash[at..at + 8].try_into().unwrap());
    let id = |index: u64| format!("{:016x}", bits(0).wrapping_add(index));
    let made = clock(PI_DAY, 0, "00000000");
    assert_eq!(made.machine, MachineId::UNKNOWN);
    let batched = |index: u64, entry: Entry| Step {
        batch: Some(format!("{:016x}", bits(8))),
        ..step(&id(index), made, None, vec![entry])
    };
    assert_eq!(
        *copy,
        VirtualCopy {
            id: FIRST_COPY.to_owned(),
            number: "0".to_owned(),
            name: String::new(),
            end: id(2),
            clocks: vec![made],
            history: vec![
                batched(0, module("lens")),
                batched(1, module("look")),
                batched(2, operation("fake")),
            ],
        }
    );
    // One entry: one step, in no batch.
    let mut one = Document::new("0.4.0");
    one.entries = vec![module("look")];
    let xml_one = one.to_xml();
    let hash_one = crate::index::hash(xml_one.as_bytes());
    let id_one: String = hash_one[..8].iter().map(|b| format!("{b:02x}")).collect();
    let read_one = Edits::parse(&xml_one, PI_DAY).unwrap();
    assert_eq!(
        read_one.copies[0].history,
        [step(&id_one, made, None, vec![module("look")])]
    );
    assert_eq!(read_one.copies[0].end, id_one);
    // Another file gives other steps; it is written as version 2.
    let other = xml.replace("0.3", "0.4");
    assert_ne!(Edits::parse(&other, PI_DAY).unwrap().copies[0].end, id(2));
    let written = read.to_xml();
    assert!(written.contains(r#"apertrust:version="2""#), "{written}");
    let again = Edits::parse(&written, 0).unwrap();
    assert_eq!(
        (again.version, &again.copies),
        (COPIES_VERSION, &read.copies)
    );
}

#[test]
fn a_version_1_or_0_file_holding_nothing_reads_as_no_step() {
    let empty = Document::new("test").to_xml();
    let version_0 = empty.replace(r#"apertrust:version="1""#, r#"apertrust:version="0""#);
    for xml in [empty, version_0] {
        let read = Edits::parse(&xml, PI_DAY).unwrap();
        let [copy] = read.copies.as_slice() else {
            panic!("one copy, not {:?}", read.copies);
        };
        assert_eq!(copy.id, FIRST_COPY);
        assert_eq!(copy.end, "");
        assert!(copy.history.is_empty() && copy.clocks.is_empty());
    }
}

#[test]
fn a_newer_version_reads_as_version_2_lays_it_out() {
    let xml = edits()
        .to_xml()
        .replace(r#"apertrust:version="2""#, r#"apertrust:version="3""#);
    let read = Edits::parse(&xml, 0).unwrap();
    assert_eq!(read.version, 3);
    assert_eq!(read.copies, edits().copies);
    assert_eq!(read.to_xml(), xml);
}

/// A step's clock that cannot be read keeps the step, with no clock, and is a problem;
/// so is a clock of the vector, which is left out. The rest of the file reads.
#[test]
fn a_clock_that_cannot_be_read_is_a_problem_and_the_rest_reads() {
    let xml = edits().to_xml();
    // The first step's time, the only one at 14:02:11.
    let damaged = xml.replacen(
        r#"apertrust:when="2026-03-14T14:02:11.120Z""#,
        r#"apertrust:when="14 March 2026""#,
        1,
    );
    assert_ne!(damaged, xml);
    let read = Edits::parse(&damaged, 0).unwrap();
    assert_eq!(
        read.problems,
        [Problem::StepClock {
            copy: "7c0e9a14".to_owned(),
            step: "00000000000000a1".to_owned(),
            error: crate::clock::ClockError::When("14 March 2026".to_owned()),
        }]
    );
    let mut expected = edits();
    expected.copies[0].history[0].clock = None;
    assert_eq!(read.copies, expected.copies);
    // Written, it holds no clock for that step, and reads back the same, the step with no
    // clock and one problem.
    let written = read.to_xml();
    assert!(!written.contains("14 March"), "{written}");
    let again = Edits::parse(&written, 0).unwrap();
    assert_eq!(again.copies[0].history[0].clock, None);
    assert!(
        matches!(
            again.problems.as_slice(),
            [Problem::StepClock { step, .. }] if step == "00000000000000a1"
        ),
        "{:?}",
        again.problems
    );
    assert_eq!(again.to_xml(), written);
    // The vector's first clock, the only one with counter 1, missing its counter.
    let missing = xml.replacen(r#" apertrust:counter="1""#, "", 1);
    assert_ne!(missing, xml);
    let read = Edits::parse(&missing, 0).unwrap();
    assert!(
        matches!(
            read.problems.as_slice(),
            [Problem::VectorClock { copy, .. }] if copy == "7c0e9a14"
        ),
        "{:?}",
        read.problems
    );
    assert_eq!(read.copies[0].clocks, edits().copies[0].clocks[1..]);
    assert_eq!(read.copies[0].history, edits().copies[0].history);
}

/// A note in words beside a message, or a message's arguments without one, is a problem:
/// the message is read, and arguments without one are left out.
#[test]
fn a_malformed_note_is_a_problem_and_the_step_reads() {
    let xml = edits().to_xml();
    let message = r#"apertrust:noteMessage="history-applied""#;
    let problem = |copy: &str, step: &str| (copy.to_owned(), step.to_owned());
    let both = xml.replace(message, &format!(r#"apertrust:note="Words" {message}"#));
    let read = Edits::parse(&both, 0).unwrap();
    let Problem::TwoNotes { copy, step } = &read.problems[0] else {
        panic!("{:?}", read.problems);
    };
    assert_eq!(
        (copy.clone(), step.clone()),
        problem("7c0e9a14", "00000000000000c3")
    );
    assert_eq!(read.copies, edits().copies);
    let alone = xml.replace(message, "");
    let read = Edits::parse(&alone, 0).unwrap();
    let Problem::ArgumentsWithoutMessage { copy, step } = &read.problems[0] else {
        panic!("{:?}", read.problems);
    };
    assert_eq!(
        (copy.clone(), step.clone()),
        problem("7c0e9a14", "00000000000000c3")
    );
    assert_eq!(read.copies[0].history[2].note, None);
    assert_eq!(read.problems.len(), 1);
}

/// What `apertrust:noteArgs` holds beside text arguments, which a save would drop, is a
/// problem; the text arguments are read.
#[test]
fn note_arguments_that_cannot_be_read_are_a_problem() {
    let xml = edits().to_xml();
    let start = xml.find("<apertrust:noteArgs>").unwrap();
    let end = xml.find("</apertrust:noteArgs>").unwrap() + "</apertrust:noteArgs>".len();
    let with = |args: &str| format!("{}{args}{}", &xml[..start], &xml[end..]);
    let message = |read: &Edits| read.copies[0].history[2].note.clone();
    let cases = [
        // A field of another namespace beside the arguments.
        (
            with(
                r#"<apertrust:noteArgs><rdf:Description arg:photo="IMG_2041" arg:count="3" apertrust:extra="x"/></apertrust:noteArgs>"#,
            ),
            edits().copies[0].history[2].note.clone(),
        ),
        // An argument that is a struct, not text.
        (
            with(
                r#"<apertrust:noteArgs><rdf:Description arg:photo="IMG_2041"><arg:count><rdf:Description arg:x="1"/></arg:count></rdf:Description></apertrust:noteArgs>"#,
            ),
            Some(Note::Message {
                id: "history-applied".to_owned(),
                args: vec![("photo".to_owned(), "IMG_2041".to_owned())],
            }),
        ),
        // Text in place of the struct.
        (
            with("<apertrust:noteArgs>photo=IMG_2041</apertrust:noteArgs>"),
            Some(Note::Message {
                id: "history-applied".to_owned(),
                args: Vec::new(),
            }),
        ),
    ];
    for (odd, note) in cases {
        let read = Edits::parse(&odd, 0).unwrap();
        assert!(
            matches!(
                read.problems.as_slice(),
                [Problem::NoteArguments { copy, step }]
                    if copy == "7c0e9a14" && step == "00000000000000c3"
            ),
            "{:?} in {odd}",
            read.problems
        );
        assert_eq!(message(&read), note, "{odd}");
    }
    // The arguments as written are no problem.
    assert!(Edits::parse(&xml, 0).unwrap().problems.is_empty());
}

/// A note or a message that is no plain text, which a save would drop or change, is a
/// problem, and is read as absent.
#[test]
fn a_note_that_is_not_plain_text_is_a_problem() {
    let xml = edits().to_xml();
    // The step's own fields, then `element` in place of its attribute `attribute`.
    let replaced = |step: &str, attribute: &str, element: &str| {
        let without = xml.replacen(attribute, "", 1);
        assert_ne!(without, xml, "{attribute}");
        let at = without.find(&format!(r#"apertrust:id="{step}""#)).unwrap();
        let entries = at + without[at..].find("<apertrust:").unwrap();
        format!("{}{element}{}", &without[..entries], &without[entries..])
    };
    let words = r#" apertrust:note="Warmer, as asked: &quot;a bit&quot; &amp; &lt;more&gt;""#;
    let message = r#" apertrust:noteMessage="history-applied""#;
    let cases = [
        // A note in words as an array, as a struct, as a resource and with a language.
        (
            "00000000000000b2",
            words,
            "<apertrust:note><rdf:Bag><rdf:li>Warmer</rdf:li></rdf:Bag></apertrust:note>",
            "note",
        ),
        (
            "00000000000000b2",
            words,
            r#"<apertrust:note rdf:parseType="Resource"><apertrust:x>1</apertrust:x></apertrust:note>"#,
            "note",
        ),
        (
            "00000000000000b2",
            words,
            r#"<apertrust:note rdf:resource="https://example.org/note"/>"#,
            "note",
        ),
        (
            "00000000000000b2",
            words,
            r#"<apertrust:note xml:lang="en">Warmer</apertrust:note>"#,
            "note",
        ),
        // A message as an array.
        (
            "00000000000000c3",
            message,
            "<apertrust:noteMessage><rdf:Seq><rdf:li>history-applied</rdf:li></rdf:Seq></apertrust:noteMessage>",
            "noteMessage",
        ),
    ];
    for (step, attribute, element, field) in cases {
        let odd = replaced(step, attribute, element);
        let read = Edits::parse(&odd, 0).unwrap_or_else(|e| panic!("{e}: {odd}"));
        let unreadable = Problem::UnreadableNote {
            copy: "7c0e9a14".to_owned(),
            step: step.to_owned(),
            field: field.to_owned(),
        };
        assert!(
            read.problems.contains(&unreadable),
            "{:?} in {odd}",
            read.problems
        );
        let read_step = read.copies[0]
            .history
            .iter()
            .find(|s| s.id == step)
            .unwrap();
        assert_eq!(read_step.note, None, "{element}");
    }
}

/// What the engine reads as numbers and names, a step's id and batch, its source and the
/// copy's end, is a problem where it is not as Apertrust writes it: the text is kept as
/// read, and written back as it was.
#[test]
fn an_id_batch_source_or_end_that_cannot_be_read_is_a_problem() {
    let xml = edits().to_xml();
    let copy = || "7c0e9a14".to_owned();
    let cases = [
        (
            r#"apertrust:id="00000000000000a1""#,
            r#"apertrust:id="00000000000000A1""#,
            Problem::StepId {
                copy: copy(),
                step: "00000000000000A1".to_owned(),
            },
        ),
        (
            r#"apertrust:batch="0f1e2d3c4b5a6978""#,
            r#"apertrust:batch="0f1e""#,
            Problem::Batch {
                copy: copy(),
                step: "00000000000000b2".to_owned(),
                batch: "0f1e".to_owned(),
            },
        ),
        (
            r#"apertrust:source="assistant""#,
            r#"apertrust:source="robot""#,
            Problem::Source {
                copy: copy(),
                step: "00000000000000b2".to_owned(),
                name: "robot".to_owned(),
            },
        ),
        (
            r#"apertrust:end="00000000000000b2""#,
            r#"apertrust:end="00000000000000ff""#,
            Problem::End {
                copy: copy(),
                end: "00000000000000ff".to_owned(),
            },
        ),
    ];
    for (written, damaged, problem) in cases {
        let changed = xml.replacen(written, damaged, 1);
        assert_ne!(changed, xml, "{damaged}");
        let read = Edits::parse(&changed, 0).unwrap();
        assert_eq!(read.problems, [problem], "{damaged}");
        assert_eq!(read.to_xml(), changed, "{damaged}");
    }
    // An end naming no step takes every step as applied.
    let read = Edits::parse(&end_on_no_step(&xml), 0).unwrap();
    assert_eq!(read.copies[0].applied(), 3);
}

/// `xml` with its copy's end on no step.
fn end_on_no_step(xml: &str) -> String {
    xml.replacen(
        r#"apertrust:end="00000000000000b2""#,
        r#"apertrust:end="00000000000000ff""#,
        1,
    )
}

/// A copy is edited where some module's or operation's last entry up to the end is not
/// empty, as the index folds a log as text; the steps after the end count for nothing.
#[test]
fn a_copy_is_edited_where_a_last_entry_up_to_the_end_is_not_empty() {
    let mut edits = edits();
    assert_eq!(edits.copies[0].applied(), 2);
    assert!(edits.edited());
    // At Original, nothing applies.
    edits.copies[0].end.clear();
    assert_eq!(edits.copies[0].applied(), 0);
    assert!(!edits.edited());
    // Each entry of the steps up to the end emptied: the last of each is empty.
    let mut edits = self::edits();
    for step in &mut edits.copies[0].history[..2] {
        for entry in &mut step.entries {
            match entry {
                Entry::Module {
                    kept,
                    auto,
                    enabled,
                    controls,
                    ..
                } => {
                    kept.clear();
                    auto.clear();
                    controls.clear();
                    *enabled = None;
                }
                Entry::Operation { params, .. } => params.clear(),
            }
            assert!(entry.is_empty());
        }
    }
    assert!(!edits.edited());
    // A later entry of a module reset over an earlier one's values.
    let mut edits = self::edits();
    let reset = edits.copies[0].history[2].clone();
    edits.copies[0].history.insert(2, reset);
    edits.copies[0].history[2].id = "00000000000000d4".to_owned();
    edits.copies[0].end = "00000000000000d4".to_owned();
    edits.copies[0].history[0].entries.clear();
    edits.copies[0].history[1]
        .entries
        .retain(|e| matches!(e, Entry::Module { .. }));
    assert!(!edits.edited());
}

/// What other tools write beside an edit, as ExifTool's `dc:subject` and `xmp:Rating`, is
/// kept as read and written back after Apertrust's own properties, byte for byte from then
/// on, with no problem; a field of Apertrust's own this build does not know, or any field
/// in a copy or a step it does not write, is a problem a save would otherwise drop.
#[test]
fn other_tools_properties_are_kept_and_unknown_fields_are_problems() {
    let xml = edits().to_xml();
    let tagged = xml
        .replacen(
            r#"xmlns:arg="https://apertrust.org/ns/xmp/arg/""#,
            r#"xmlns:arg="https://apertrust.org/ns/xmp/arg/"
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmp:Rating="3""#,
            1,
        )
        .replacen(
            "   <apertrust:copies>",
            "   <dc:subject>\n    <rdf:Bag>\n     <rdf:li>sky</rdf:li>\n     <rdf:li>sea</rdf:li>\n    </rdf:Bag>\n   </dc:subject>\n   <apertrust:copies>",
            1,
        );
    assert_ne!(tagged, xml);
    let read = Edits::parse(&tagged, 0).unwrap();
    assert!(read.problems.is_empty(), "{:?}", read.problems);
    let names: Vec<_> = (read.other.iter()).map(|p| p.name.local.as_str()).collect();
    assert_eq!(names, ["Rating", "subject"]);
    assert_eq!(read.copies, edits().copies);
    let written = read.to_xml();
    // After Apertrust's own, each simple one as an element, as the XMP layer writes one after
    // an element.
    for kept in [
        "<xmp:Rating>3</xmp:Rating>",
        "<rdf:li>sky</rdf:li>",
        "<rdf:li>sea</rdf:li>",
    ] {
        assert!(written.contains(kept), "{kept} in {written}");
    }
    let again = Edits::parse(&written, 0).unwrap();
    assert_eq!(again.other, read.other);
    assert_eq!(again.to_xml(), written);
    // Apertrust's own unknown at the top, in a copy and in a step; another tool's in a
    // step.
    let first = r#"apertrust:id="00000000000000a1""#;
    let cases = [
        (
            xml.replacen(
                r#"apertrust:version="2""#,
                r#"apertrust:version="2" apertrust:groups="a""#,
                1,
            ),
            "the file",
            "apertrust:groups",
        ),
        (
            xml.replacen(
                r#"apertrust:number="0""#,
                r#"apertrust:number="0" apertrust:snapshot="1""#,
                1,
            ),
            "copy 7c0e9a14",
            "apertrust:snapshot",
        ),
        (
            xml.replacen(first, &format!(r#"{first} apertrust:rating="2""#), 1),
            "step 00000000000000a1 of copy 7c0e9a14",
            "apertrust:rating",
        ),
        (
            xml.replacen(
                first,
                &format!(r#"{first} xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="x""#),
                1,
            ),
            "step 00000000000000a1 of copy 7c0e9a14",
            "dc:title",
        ),
    ];
    for (damaged, place, field) in cases {
        assert_ne!(damaged, xml, "{field}");
        let read = Edits::parse(&damaged, 0).unwrap();
        let expected = Problem::UnknownField {
            place: place.to_owned(),
            field: field.to_owned(),
        };
        assert_eq!(read.problems, [expected], "{field}");
    }
}

/// In a step's entries, a field a newer build may write that this one would drop is a
/// problem, by the kind of entry: one of Apertrust's own it does not write, another
/// namespace's, an operation's control or a module's parameter, an item that is no entry,
/// and an entry naming both a module and an operation.
#[test]
fn an_entry_s_unknown_field_is_a_problem() {
    let xml = edits().to_xml();
    let lens = r#"apertrust:module="lens""#;
    let place = "step 00000000000000a1 of copy 7c0e9a14";
    let cases = [
        (
            xml.replacen(lens, &format!(r#"{lens} apertrust:layer="1""#), 1),
            "apertrust:layer",
        ),
        (
            xml.replacen(
                lens,
                &format!(r#"{lens} xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="x""#),
                1,
            ),
            "dc:title",
        ),
        (
            xml.replacen(
                "<apertrust:entries>\n            <rdf:Seq>",
                "<apertrust:entries>\n            <rdf:Seq>\n             <rdf:li>a word</rdf:li>",
                1,
            ),
            "an item that is no entry",
        ),
    ];
    for (damaged, field) in cases {
        assert_ne!(damaged, xml, "{field}");
        let read = Edits::parse(&damaged, 0).unwrap();
        let expected = Problem::UnknownField {
            place: place.to_owned(),
            field: field.to_owned(),
        };
        assert_eq!(read.problems, [expected], "{field}");
    }
    // By the kind of entry: an operation's control, a module's parameter or schema, a
    // module named by nothing, and an entry naming both.
    let fake = r#"apertrust:operation="fake""#;
    let cases = [
        (
            xml.replacen(fake, &format!(r#"{fake} control:x="1""#), 1),
            "step 00000000000000b2 of copy 7c0e9a14",
            "control:x",
        ),
        (
            xml.replacen(lens, &format!(r#"{lens} param:y="1""#), 1),
            place,
            "param:y",
        ),
        (
            xml.replacen(lens, &format!(r#"{lens} apertrust:schema="1""#), 1),
            place,
            "apertrust:schema",
        ),
        (
            xml.replacen(lens, r#"apertrust:module="""#, 1),
            place,
            "an item that is no entry",
        ),
        (
            xml.replacen(lens, &format!(r#"{lens} {fake}"#), 1),
            place,
            "an entry naming both a module and an operation",
        ),
    ];
    for (damaged, at, field) in cases {
        assert_ne!(damaged, xml, "{field}");
        let read = Edits::parse(&damaged, 0).unwrap();
        let expected = Problem::UnknownField {
            place: at.to_owned(),
            field: field.to_owned(),
        };
        assert_eq!(read.problems, [expected], "{field}");
    }
}

/// A step's id twice in its copy, a step's clock past its copy's vector, and `entries` that
/// is no `rdf:Seq`, each a problem, as a save would lose or misplace what the file holds.
#[test]
fn a_duplicate_step_a_clock_past_the_vector_and_entries_no_seq_are_problems() {
    let xml = edits().to_xml();
    let copy = "7c0e9a14".to_owned();
    let twice = xml.replacen(
        r#"apertrust:id="00000000000000a1""#,
        r#"apertrust:id="00000000000000b2""#,
        1,
    );
    let read = Edits::parse(&twice, 0).unwrap();
    let duplicate = Problem::DuplicateStep {
        copy: copy.clone(),
        step: "00000000000000b2".to_owned(),
    };
    assert!(read.problems.contains(&duplicate), "{:?}", read.problems);
    // The vector's clock of machine 5e1f22c9, the later of its steps', moved before them.
    let mut past = edits();
    past.copies[0].clocks[1] = clock(PI_DAY - 1_000, 0, "5e1f22c9");
    let read = Edits::parse(&past.to_xml(), 0).unwrap();
    let steps: Vec<_> = (read.problems.iter())
        .map(|problem| match problem {
            Problem::ClockPastVector { step, .. } => step.as_str(),
            other => panic!("{other}"),
        })
        .collect();
    assert_eq!(steps, ["00000000000000a1", "00000000000000b2"]);
    // A machine the vector lacks.
    let mut lacking = edits();
    lacking.copies[0].clocks.remove(0);
    let read = Edits::parse(&lacking.to_xml(), 0).unwrap();
    assert!(
        matches!(&read.problems[..], [Problem::ClockPastVector { step, .. }] if step == "00000000000000c3"),
        "{:?}",
        read.problems
    );
    // Entries as text: no entry read, and a problem.
    let at = xml.find(r#"apertrust:id="00000000000000a1""#).unwrap();
    let start = at + xml[at..].find("<apertrust:entries>").unwrap();
    let end = start + xml[start..].find("</apertrust:entries>").unwrap();
    let text = format!(
        "{}<apertrust:entries>lens</apertrust:entries>{}",
        &xml[..start],
        &xml[end + "</apertrust:entries>".len()..]
    );
    let read = Edits::parse(&text, 0).unwrap();
    assert_eq!(
        read.problems,
        [Problem::UnknownField {
            place: format!("step 00000000000000a1 of copy {copy}"),
            field: "apertrust:entries".to_owned(),
        }]
    );
}

#[test]
fn an_edit_s_file_is_read_with_its_modified_time() {
    let directory = tempfile::tempdir().unwrap();
    let path = path_for(&directory.path().join("IMG.ARW"));
    let mut document = Document::new("test");
    document.entries = vec![module("look")];
    write_xml(&path, &document.to_xml()).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let millis = modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let read = read_edits(&path).unwrap();
    assert_eq!(
        u128::from(read.copies[0].history[0].clock.unwrap().when),
        millis.min(u128::from(crate::clock::MAX_WHEN))
    );
}

#[test]
fn the_fingerprint_is_the_size_and_a_hash_of_the_start_and_end() {
    // A file shorter than the span is hashed whole as both.
    assert_eq!(
        fingerprint(3, b"raw", b"raw"),
        format!("3:{}", blake3::hash(b"rawraw").to_hex())
    );
    let directory = tempfile::tempdir().unwrap();
    let span = usize::try_from(FINGERPRINT_SPAN).unwrap();
    let photo = directory.path().join("IMG.ARW");
    for size in [0, 1, span - 1, span, span + 1, 2 * span, 3 * span + 7] {
        let bytes: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        std::fs::write(&photo, &bytes).unwrap();
        let start = &bytes[..size.min(span)];
        let end = &bytes[size - size.min(span)..];
        assert_eq!(
            read_fingerprint(&photo).unwrap(),
            fingerprint(size as u64, start, end),
            "{size}"
        );
    }
    // The middle of a larger file is not read; its first and last bytes are.
    let mut bytes = vec![7_u8; 3 * span];
    std::fs::write(&photo, &bytes).unwrap();
    let before = read_fingerprint(&photo).unwrap();
    let size = format!("{}:", 3 * span);
    assert!(before.starts_with(&size), "{before}");
    assert_eq!(before.len(), size.len() + 64);
    bytes[span + 10] = 8;
    std::fs::write(&photo, &bytes).unwrap();
    assert_eq!(read_fingerprint(&photo).unwrap(), before);
    for at in [0, 3 * span - 1] {
        let mut changed = bytes.clone();
        changed[at] = 9;
        std::fs::write(&photo, &changed).unwrap();
        assert_ne!(read_fingerprint(&photo).unwrap(), before, "{at}");
    }
}
