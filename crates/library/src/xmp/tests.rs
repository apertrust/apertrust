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

const EX: &str = "https://example.org/ns/1.0/";
const XMP: &str = "http://ns.adobe.com/xap/1.0/";
const DC: &str = "http://purl.org/dc/elements/1.1/";

fn name(prefix: &str, namespace: &str, local: &str) -> Name {
    Name::new(prefix, namespace, local)
}

fn simple(prefix: &str, namespace: &str, local: &str, text: &str) -> Property {
    Property::simple(name(prefix, namespace, local), text)
}

fn fixture(file: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xmp")
        .join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// What every `form-*.xmp` fixture says, however it writes it.
fn the_forms_say() -> Vec<Property> {
    let ex = |local: &str, node: Node| Property::new(name("ex", EX, local), node);
    let field = |local: &str, text: &str| simple("ex", EX, local, text);
    let mut note = Node::simple("kept");
    note.qualifiers.push(Qualifier {
        name: name("ex", EX, "since"),
        value: "2020".to_owned(),
    });
    let step = |num: &str, op: &str| Node::structure(vec![field("num", num), field("op", op)]);
    vec![
        simple("xmp", XMP, "Rating", "3"),
        Property::new(
            name("dc", DC, "title"),
            Node::array(
                ArrayKind::Alt,
                vec![
                    Node::with_lang("Harbour", DEFAULT_LANGUAGE),
                    Node::with_lang("Hafen", "de"),
                ],
            ),
        ),
        Property::new(
            name("dc", DC, "subject"),
            Node::array(
                ArrayKind::Bag,
                vec![Node::simple("harbour"), Node::simple("a & b")],
            ),
        ),
        ex(
            "place",
            Node::structure(vec![field("city", "Oslo"), field("country", "NO")]),
        ),
        ex(
            "link",
            Node {
                value: Value::Resource("https://example.org/a".to_owned()),
                qualifiers: Vec::new(),
            },
        ),
        ex("note", note),
        ex(
            "history",
            Node::array(
                ArrayKind::Seq,
                vec![step("0", "exposure"), step("1", "flip")],
            ),
        ),
    ]
}

fn read(xml: &str) -> Xmp {
    Xmp::parse(xml).unwrap_or_else(|e| panic!("{e} in {xml}"))
}

// -- Reading ------------------------------------------------------------------------------

#[test]
fn every_form_reads_to_the_same_tree() {
    for file in [
        "form-attributes.xmp",
        "form-elements.xmp",
        "form-bare-rdf.xmp",
        "form-packet.xmp",
    ] {
        let xmp = Xmp::parse(&fixture(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(xmp.properties, the_forms_say(), "{file}");
        assert_eq!(
            xmp.namespaces,
            [
                Namespace::new("xmp", XMP),
                Namespace::new("dc", DC),
                Namespace::new("ex", EX)
            ],
            "{file}"
        );
        assert_eq!(xmp.about, "", "{file}");
        // The wrapper `x:xmpmeta` names the toolkit; a bare `rdf:RDF` has none.
        let toolkit = (file != "form-bare-rdf.xmp").then(|| "forms".to_owned());
        assert_eq!(xmp.toolkit, toolkit, "{file}");
    }
}

#[test]
fn a_language_alternative_keeps_its_languages_and_x_default() {
    let xmp = read(&fixture("form-attributes.xmp"));
    let title = xmp.get(DC, "title").unwrap();
    let Value::Array(ArrayKind::Alt, items) = &title.value else {
        panic!("{title:?}");
    };
    let languages: Vec<_> = items.iter().map(|i| i.lang().unwrap()).collect();
    assert_eq!(languages, [DEFAULT_LANGUAGE, "de"]);
    assert_eq!(items[1].as_str(), Some("Hafen"));
}

#[test]
fn several_descriptions_merge_in_order_and_the_first_about_wins() {
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:ex="{EX}">
        <rdf:Description rdf:about="a" ex:one="1"/>
        <rdf:Description rdf:about="b" ex:two="2"><ex:three>3</ex:three></rdf:Description>
        </rdf:RDF>"#
    );
    let xmp = read(&xml);
    assert_eq!(xmp.about, "a");
    let locals: Vec<_> = xmp
        .properties
        .iter()
        .map(|p| p.name.local.as_str())
        .collect();
    assert_eq!(locals, ["one", "two", "three"]);
}

#[test]
fn text_is_unescaped_whatever_the_form() {
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:ex="{EX}"><rdf:Description
        ex:a="1 &lt; 2 &amp;&#x20;&#65; &quot;q&quot; &apos;s&apos;&#xA;next"><ex:b>x &gt; y<![CDATA[ <&> ]]></ex:b>
        <ex:c>  padded  </ex:c><ex:d/></rdf:Description></rdf:RDF>"#
    );
    let xmp = read(&xml);
    let text = |local| xmp.get(EX, local).unwrap().as_str().unwrap().to_owned();
    assert_eq!(text("a"), "1 < 2 & A \"q\" 's'\nnext");
    assert_eq!(text("b"), "x > y <&> ");
    assert_eq!(text("c"), "  padded  ");
    assert_eq!(text("d"), "");
}

#[test]
fn a_byte_order_mark_and_other_encodings_are_read() {
    let xml = fixture("form-attributes.xmp");
    let expected = read(&xml);
    let mut utf8 = vec![0xEF, 0xBB, 0xBF];
    utf8.extend_from_slice(xml.as_bytes());
    assert_eq!(Xmp::parse_bytes(&utf8).unwrap(), expected);
    let units: Vec<u16> = xml.encode_utf16().collect();
    let mut le = vec![0xFF, 0xFE];
    le.extend(units.iter().flat_map(|u| u.to_le_bytes()));
    assert_eq!(Xmp::parse_bytes(&le).unwrap(), expected);
    let mut be = vec![0xFE, 0xFF];
    be.extend(units.iter().flat_map(|u| u.to_be_bytes()));
    assert_eq!(Xmp::parse_bytes(&be).unwrap(), expected);
    assert!(Xmp::parse_bytes(&[b'<', 0xFF, b'>']).is_err());
    // No byte order mark but the `xpacket`'s own: found by the first `<?` in two bytes.
    let packet = fixture("form-packet.xmp");
    let packet = packet.trim_start_matches('\u{feff}');
    let expected = read(packet);
    let le: Vec<u8> = packet.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = packet.encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_eq!(&le[..4], b"<\0?\0");
    assert_eq!(&be[..4], b"\0<\0?");
    assert_eq!(Xmp::parse_bytes(&le).unwrap(), expected);
    assert_eq!(Xmp::parse_bytes(&be).unwrap(), expected);
}

#[test]
fn an_empty_description_is_an_empty_packet() {
    let xmp = read(&format!(
        r#"<x:xmpmeta xmlns:x="{META}"><rdf:RDF xmlns:rdf="{RDF}"><rdf:Description rdf:about=""/></rdf:RDF></x:xmpmeta>"#
    ));
    assert!(xmp.properties.is_empty());
    assert_eq!(xmp.toolkit, None);
    // A wrapper with no RDF in it says nothing, which is not an error.
    let bare = read(&format!("<x:xmpmeta xmlns:x=\"{META}\"/>"));
    assert!(bare.properties.is_empty());
}

// -- Writing ------------------------------------------------------------------------------

/// Every `.xmp` in the fixture folder: darktable's files and the hand-written forms.
fn every_fixture() -> Vec<(String, String)> {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/xmp");
    let mut found: Vec<_> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "xmp"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).unwrap())
        })
        .collect();
    found.sort();
    found
}

/// Parse, write, parse: the same tree, and writing it again the same bytes.
fn round_trips(label: &str, xml: &str) {
    let xmp = Xmp::parse(xml).unwrap_or_else(|e| panic!("{label}: {e}"));
    let written = xmp.to_xml();
    let again = Xmp::parse(&written).unwrap_or_else(|e| panic!("{label}: {e}\n{written}"));
    assert_eq!(again, xmp, "{label}");
    assert_eq!(again.to_xml(), written, "{label}");
}

#[test]
fn what_is_written_reads_back_to_the_same_tree_and_writes_the_same_bytes() {
    let fixtures = every_fixture();
    assert!(fixtures.len() >= 8, "{} fixtures", fixtures.len());
    for (name, text) in &fixtures {
        round_trips(name, text);
    }
}

#[test]
fn a_property_two_descriptions_or_an_attribute_and_an_element_both_say_is_written_readably() {
    // Two merged descriptions, and one description that says it as attribute and element:
    // an attribute cannot repeat, so the later one is written as an element, in its place.
    for (label, body) in [
        (
            "merged",
            r#"<rdf:Description a:x="1" a:y="y"/><rdf:Description a:x="2" a:z="z"/>"#,
        ),
        (
            "attribute and element",
            r#"<rdf:Description a:x="1" a:y="y"><a:x>2</a:x><a:z>z</a:z></rdf:Description>"#,
        ),
    ] {
        let xml = format!(r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:a="urn:a">{body}</rdf:RDF>"#);
        let xmp = read(&xml);
        let order: Vec<_> = xmp
            .properties
            .iter()
            .map(|p| (p.name.local.as_str(), p.node.as_str().unwrap()))
            .collect();
        assert_eq!(
            order,
            [("x", "1"), ("y", "y"), ("x", "2"), ("z", "z")],
            "{label}"
        );
        let written = xmp.to_xml();
        assert_eq!(written.matches("a:x=").count(), 1, "{label}: {written}");
        assert!(written.contains("<a:x>2</a:x>"), "{label}: {written}");
        round_trips(label, &xml);
    }
    // The same inside a struct.
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:a="urn:a"><rdf:Description><a:s rdf:parseType="Resource" a:x="1"><a:x>2</a:x></a:s></rdf:Description></rdf:RDF>"#
    );
    let written = read(&xml).to_xml();
    assert_eq!(written.matches("a:x=").count(), 1, "{written}");
    round_trips("struct", &xml);
}

#[test]
fn a_qualifier_the_node_has_twice_is_written_readably() {
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:a="urn:a" xmlns:b="urn:b"><rdf:Description><a:t rdf:parseType="Resource"><rdf:value>v</rdf:value><b:q>w</b:q><b:q>z</b:q></a:t></rdf:Description></rdf:RDF>"#
    );
    let xmp = read(&xml);
    let node = xmp.get("urn:a", "t").unwrap();
    assert_eq!(node.as_str(), Some("v"));
    let qualifiers: Vec<_> = node.qualifiers.iter().map(|q| q.value.as_str()).collect();
    assert_eq!(qualifiers, ["w", "z"]);
    let written = xmp.to_xml();
    assert_eq!(written.matches("b:q=").count(), 1, "{written}");
    assert!(written.contains("<b:q>z</b:q>"), "{written}");
    round_trips("twice", &xml);
    // With a language on the value, and on a struct, whose qualifiers repeat too.
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:a="urn:a" xmlns:b="urn:b"><rdf:Description><a:t rdf:parseType="Resource"><rdf:value xml:lang="fr">v</rdf:value><b:q>w</b:q><b:q>z</b:q></a:t><a:s rdf:parseType="Resource" xml:lang="de"><rdf:value rdf:parseType="Resource"><a:k>1</a:k></rdf:value><b:q>w</b:q><b:q>z</b:q></a:s></rdf:Description></rdf:RDF>"#
    );
    round_trips("twice with languages", &xml);
}

/// `count` simple properties, the first half as attributes and the rest as elements.
fn many_properties(count: usize) -> String {
    let mut xml = format!(r#"<rdf:RDF xmlns:rdf="{RDF}" xmlns:a="urn:a"><rdf:Description"#);
    for index in 0..count / 2 {
        xml.push_str(&format!(" a:p{index}=\"{index}\""));
    }
    xml.push_str(">\n");
    for index in count / 2..count {
        xml.push_str(&format!("<a:p{index}>{index}</a:p{index}>\n"));
    }
    xml.push_str("</rdf:Description></rdf:RDF>");
    xml
}

#[test]
fn reading_and_writing_scale_linearly() {
    // Time 10,000 properties and 40,000: linear work takes 4 times as long, quadratic 16.
    // The best of a few runs each, and a limit between the two with room for noise.
    let best = |count: usize| {
        let xml = many_properties(count);
        (0..3)
            .map(|_| {
                let start = std::time::Instant::now();
                let xmp = read(&xml);
                assert_eq!(xmp.properties.len(), count);
                let written = xmp.to_xml();
                assert_eq!(read(&written).properties.len(), count);
                start.elapsed()
            })
            .min()
            .unwrap()
    };
    let (small, large) = (best(10_000), best(40_000));
    assert!(
        large < small * 10,
        "10,000 properties took {small:?}, 40,000 took {large:?}"
    );
}

#[test]
fn an_error_deep_in_a_large_packet_names_its_line() {
    let xml = many_properties(2_000);
    let lines = xml.matches('\n').count();
    let broken = xml.replace(
        "</rdf:Description>",
        "<a:late>1</a:other></rdf:Description>",
    );
    let error = Xmp::parse(&broken).expect_err("mismatched");
    assert_eq!(error.line(), lines + 1, "{error}");
}

#[test]
fn the_canonical_form_is_compact() {
    let xmp = read(&fixture("form-elements.xmp"));
    let expected = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="forms">
 <rdf:RDF xmlns:rdf="{RDF}">
  <rdf:Description rdf:about=""
    xmlns:xmp="{XMP}"
    xmlns:dc="{DC}"
    xmlns:ex="{EX}"
    xmp:Rating="3">
   <dc:title>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">Harbour</rdf:li>
     <rdf:li xml:lang="de">Hafen</rdf:li>
    </rdf:Alt>
   </dc:title>
   <dc:subject>
    <rdf:Bag>
     <rdf:li>harbour</rdf:li>
     <rdf:li>a &amp; b</rdf:li>
    </rdf:Bag>
   </dc:subject>
   <ex:place>
    <rdf:Description
      ex:city="Oslo"
      ex:country="NO"/>
   </ex:place>
   <ex:link rdf:resource="https://example.org/a"/>
   <ex:note>
    <rdf:Description
      rdf:value="kept"
      ex:since="2020"/>
   </ex:note>
   <ex:history>
    <rdf:Seq>
     <rdf:li>
      <rdf:Description
        ex:num="0"
        ex:op="exposure"/>
     </rdf:li>
     <rdf:li>
      <rdf:Description
        ex:num="1"
        ex:op="flip"/>
     </rdf:li>
    </rdf:Seq>
   </ex:history>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#
    );
    assert_eq!(xmp.to_xml(), expected);
}

#[test]
fn a_simple_property_after_an_element_stays_after_it() {
    let xmp = Xmp {
        properties: vec![
            simple("ex", EX, "first", "1"),
            Property::new(
                name("ex", EX, "list"),
                Node::array(ArrayKind::Bag, vec![Node::simple("x")]),
            ),
            simple("ex", EX, "last", "2"),
        ],
        ..Xmp::default()
    };
    let written = xmp.to_xml();
    assert!(written.contains("<ex:last>2</ex:last>"), "{written}");
    assert!(written.contains("ex:first=\"1\""), "{written}");
    assert_eq!(read(&written).properties, xmp.properties);
}

#[test]
fn qualifiers_of_every_node_survive_a_round_trip() {
    let q = |local: &str, value: &str| Qualifier {
        name: name("ex", EX, local),
        value: value.to_owned(),
    };
    let lang = || Qualifier {
        name: Name::lang(),
        value: "fr".to_owned(),
    };
    let mut text = Node::simple("bonjour");
    text.qualifiers = vec![lang(), q("source", "me"), q("year", "2020")];
    let mut only_other = Node::simple("x");
    only_other.qualifiers = vec![q("a", "1")];
    let mut array = Node::array(ArrayKind::Seq, vec![Node::simple("one"), text.clone()]);
    array.qualifiers = vec![q("origin", "import")];
    let mut structure = Node::structure(vec![simple("ex", EX, "k", "v")]);
    structure.qualifiers = vec![lang(), q("rev", "2")];
    let xmp = Xmp {
        properties: vec![
            Property::new(name("ex", EX, "text"), text),
            Property::new(name("ex", EX, "other"), only_other),
            Property::new(name("ex", EX, "array"), array),
            Property::new(name("ex", EX, "struct"), structure),
        ],
        ..Xmp::default()
    };
    let written = xmp.to_xml();
    assert_eq!(read(&written).properties, xmp.properties, "{written}");
    assert_eq!(read(&written).to_xml(), written);
}

#[test]
fn unknown_namespaces_and_properties_are_kept_in_order_with_their_prefixes() {
    let xml = format!(
        r#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="{META}" x:xmptk="Adobe XMP Core 7.0">
 <rdf:RDF xmlns:rdf="{RDF}">
  <rdf:Description rdf:about="uuid:1"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    xmlns:xmp="{XMP}"
    crs:Exposure2012="+0.50"
    xmp:Rating="4">
   <crs:ToneCurve><rdf:Seq><rdf:li>0, 0</rdf:li><rdf:li>255, 255</rdf:li></rdf:Seq></crs:ToneCurve>
   <vendor:Blob xmlns:vendor="urn:vendor:blob"><rdf:Description vendor:id="7"><vendor:size>12</vendor:size></rdf:Description></vendor:Blob>
   <xmp:Label>Red</xmp:Label>
   <plain xmlns="urn:default">in the default namespace</plain>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#
    );
    let xmp = read(&xml);
    let order: Vec<_> = xmp
        .properties
        .iter()
        .map(|p| (p.name.prefix.as_str(), p.name.local.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("crs", "Exposure2012"),
            ("xmp", "Rating"),
            ("crs", "ToneCurve"),
            ("vendor", "Blob"),
            ("xmp", "Label"),
            ("", "plain"),
        ]
    );
    assert_eq!(xmp.toolkit.as_deref(), Some("Adobe XMP Core 7.0"));
    assert_eq!(xmp.about, "uuid:1");

    let written = xmp.to_xml();
    for kept in [
        "xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"",
        "xmlns:vendor=\"urn:vendor:blob\"",
        "crs:Exposure2012=\"+0.50\"",
        "<vendor:Blob>",
        "x:xmptk=\"Adobe XMP Core 7.0\"",
        "rdf:about=\"uuid:1\"",
    ] {
        assert!(written.contains(kept), "{kept} in {written}");
    }
    let again = read(&written);
    assert_eq!(
        again.get("urn:vendor:blob", "Blob"),
        xmp.get("urn:vendor:blob", "Blob")
    );
    assert_eq!(
        again.get("urn:default", "plain").unwrap().as_str(),
        Some("in the default namespace")
    );
    // A name with no prefix is written with one, in the same namespace.
    let order = |x: &Xmp| -> Vec<(String, String)> {
        x.properties
            .iter()
            .map(|p| (p.name.namespace.clone(), p.name.local.clone()))
            .collect()
    };
    assert_eq!(order(&again), order(&xmp));
    assert_eq!(read(&written).to_xml(), written);
}

#[test]
fn a_prefix_two_namespaces_claim_is_written_once_for_each() {
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{RDF}">
        <rdf:Description xmlns:a="urn:first" a:one="1"/>
        <rdf:Description xmlns:a="urn:second" a:two="2"/>
        </rdf:RDF>"#
    );
    let xmp = read(&xml);
    let written = xmp.to_xml();
    let again = read(&written);
    assert_eq!(again.get("urn:first", "one").unwrap().as_str(), Some("1"));
    assert_eq!(again.get("urn:second", "two").unwrap().as_str(), Some("2"));
    assert_eq!(again.properties.len(), 2);
}

#[test]
fn values_with_markup_whitespace_and_unicode_round_trip() {
    let text = "R&D <\"x\" 'y'>\n\ttab\r\nа ж ✓ 日本";
    let xmp = Xmp {
        properties: vec![
            simple("ex", EX, "attribute", text),
            Property::new(
                name("ex", EX, "element"),
                Node::array(ArrayKind::Bag, vec![Node::simple(text)]),
            ),
            simple("ex", EX, "empty", ""),
        ],
        ..Xmp::default()
    };
    let written = xmp.to_xml();
    assert!(
        written.contains("&#xA;") && written.contains("&#13;"),
        "{written}"
    );
    assert_eq!(read(&written).properties, xmp.properties);
    // A character XML 1.0 cannot hold is dropped.
    let dropped = Xmp {
        properties: vec![simple("ex", EX, "p", "a\u{1}b\u{0}c")],
        ..Xmp::default()
    };
    assert_eq!(
        read(&dropped.to_xml()).get(EX, "p").unwrap().as_str(),
        Some("abc")
    );
}

#[test]
fn a_packet_wraps_for_embedding_and_reads_back() {
    let xmp = read(&fixture("form-attributes.xmp"));
    let packet = xmp.to_packet(2048);
    assert!(
        packet.starts_with(
            "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n<x:xmpmeta"
        )
    );
    assert!(packet.ends_with("\n<?xpacket end=\"w\"?>"), "{packet}");
    assert!(!packet.contains("<?xml"));
    let body = xmp.to_xml();
    let body = body.split_once('\n').unwrap().1;
    assert!(packet.len() >= body.len() + 2048);
    assert_eq!(read(&packet), xmp);
    // No padding: the wrapper alone.
    let tight = xmp.to_packet(0);
    assert!(
        tight.ends_with("</x:xmpmeta>\n<?xpacket end=\"w\"?>"),
        "{tight}"
    );
}

// -- Refusals -----------------------------------------------------------------------------

fn description(inner: &str) -> String {
    format!(
        "<x:xmpmeta xmlns:x=\"{META}\">\n<rdf:RDF xmlns:rdf=\"{RDF}\" xmlns:ex=\"{EX}\">\n\
         <rdf:Description rdf:about=\"\">\n{inner}\n</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n"
    )
}

/// Parses `xml`, which must fail, and checks the line and a fragment of the message.
fn refused(xml: &str, line: usize, fragment: &str) {
    let error = Xmp::parse(xml).expect_err(xml);
    assert_eq!(error.line(), line, "{error} for {xml}");
    assert!(error.message().contains(fragment), "{error} for {xml}");
    assert!(error.to_string().starts_with(&format!("line {line}: ")));
}

#[test]
fn malformed_packets_are_refused_with_the_line() {
    // Not XML, or not whole.
    refused("<x:xmpmeta", 1, "");
    refused("", 1, "no XML element");
    refused("   \n\n", 1, "no XML element");
    refused(&description("<ex:a>1</ex:b>"), 4, "");
    refused(&description("<ex:a>1"), 5, "");
    refused(
        &format!("{}<ex:extra xmlns:ex=\"urn:x\"/>", description("")),
        8,
        "more than one root",
    );
    refused(&format!("{}stray", description("")), 8, "text outside");
    // Names that do not resolve.
    refused(
        &description("<ex:ok>1</ex:ok>\n<nope:a>1</nope:a>"),
        5,
        "nope",
    );
    refused(&description("<ex:a plain=\"1\"/>"), 4, "no namespace");
    refused(&description("<ex:a>&unknown;</ex:a>"), 4, "unknown entity");
    // Not the XMP root.
    refused("<html><body/></html>", 1, "not XMP");
    refused(
        &format!("<x:xmpmeta xmlns:x=\"{META}\"><p/></x:xmpmeta>"),
        1,
        "unexpected",
    );
    refused(
        &format!("<rdf:RDF xmlns:rdf=\"{RDF}\" xmlns:ex=\"{EX}\">\n<ex:a/>\n</rdf:RDF>"),
        2,
        "rdf:Description",
    );
    // RDF beyond XMP's subset.
    refused(
        &description("<ex:a rdf:parseType=\"Literal\"><b/></ex:a>"),
        4,
        "Literal",
    );
    refused(
        &description("<ex:a rdf:parseType=\"Collection\"/>"),
        4,
        "Collection",
    );
    refused(
        &description("<ex:a><rdf:Bag><ex:x/></rdf:Bag></ex:a>"),
        4,
        "rdf:li",
    );
    refused(&description("<ex:a><ex:typed/></ex:a>"), 4, "typed nodes");
    refused(
        &description("<ex:a><rdf:Seq/><rdf:Seq/></ex:a>"),
        4,
        "more than one",
    );
    refused(
        &description("<ex:a>text<rdf:Seq/></ex:a>"),
        4,
        "text next to",
    );
    refused(
        &description("<ex:a ex:b=\"1\"><rdf:Seq/></ex:a>"),
        4,
        "property attributes",
    );
    refused(
        &description("<ex:a rdf:resource=\"u\">x</ex:a>"),
        4,
        "rdf:resource",
    );
    refused(&description("<ex:a rdf:type=\"u\"/>"), 4, "rdf:type");
    refused(&description("<rdf:li>1</rdf:li>"), 4, "unexpected");
    refused(
        &description(
            "<ex:a><rdf:Description>\n<rdf:value>v</rdf:value>\n<ex:q><rdf:Bag/></ex:q>\n</rdf:Description></ex:a>",
        ),
        4,
        "qualifier q",
    );
}

#[test]
fn elements_nested_too_deeply_are_refused() {
    let inner = format!(
        "<ex:a>{}</ex:a>",
        "<rdf:Description><ex:a>".repeat(60) + &"</ex:a></rdf:Description>".repeat(60)
    );
    let error = Xmp::parse(&description(&inner)).expect_err("too deep");
    assert!(error.message().contains("nested too deeply"), "{error}");
}
