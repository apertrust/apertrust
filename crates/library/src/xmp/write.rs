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

//! Writing a packet in the canonical compact form, which never varies for a tree, so a
//! packet written, read and written again is the same bytes.
//!
//! The layout is exiv2's compact one: the declarations and the simple, unqualified
//! properties are attributes of the `rdf:Description`, one to a line, and everything else
//! an element, indented one space a level. A simple property that comes after one that
//! cannot be an attribute is an element too, so the order of the properties is kept. A
//! struct is a nested `rdf:Description`, an array an `rdf:Bag`, `rdf:Seq` or `rdf:Alt`
//! of `rdf:li`, and a node with qualifiers other than `xml:lang` an `rdf:Description` with
//! its `rdf:value`.

use std::collections::{HashMap, HashSet};

use quick_xml::escape::{escape, partial_escape};

use super::model::{META, Name, Node, Property, RDF, Value, XML, Xmp};

/// The `id` XMP gives every packet's wrapper.
const PACKET_ID: &str = "W5M0MpCehiHzreSzNTczkc9d";

/// The prefix a name is written with, by namespace and the prefix it carries; and the
/// declarations the packet needs, in order.
struct Prefixes {
    declarations: Vec<(String, String)>,
    by_name: HashMap<(String, String), String>,
}

impl Prefixes {
    fn new(xmp: &Xmp) -> Self {
        let mut prefixes = Prefixes {
            declarations: Vec::new(),
            by_name: HashMap::new(),
        };
        for declaration in &xmp.namespaces {
            if !reserved(&declaration.prefix) && !prefixes.taken(&declaration.prefix) {
                prefixes
                    .declarations
                    .push((declaration.prefix.clone(), declaration.uri.clone()));
            }
        }
        for property in &xmp.properties {
            prefixes.property(property);
        }
        prefixes
    }

    fn taken(&self, prefix: &str) -> bool {
        self.declarations.iter().any(|(p, _)| p == prefix)
    }

    /// Gives `name` its prefix: the one it carries if that is free for its namespace,
    /// else a numbered one.
    fn bind(&mut self, name: &Name) {
        if name.namespace == RDF || name.namespace == XML {
            return;
        }
        let key = (name.namespace.clone(), name.prefix.clone());
        if self.by_name.contains_key(&key) {
            return;
        }
        let wanted = if name.prefix.is_empty() || reserved(&name.prefix) {
            "ns"
        } else {
            name.prefix.as_str()
        };
        let declared = self
            .declarations
            .iter()
            .any(|(p, uri)| p == wanted && *uri == name.namespace);
        let mut prefix = wanted.to_owned();
        let mut number = 0;
        while !declared && self.taken(&prefix) {
            number += 1;
            prefix = format!("{wanted}{number}");
        }
        if !declared {
            self.declarations
                .push((prefix.clone(), name.namespace.clone()));
        }
        self.by_name.insert(key, prefix);
    }

    fn property(&mut self, property: &Property) {
        self.bind(&property.name);
        self.node(&property.node);
    }

    fn node(&mut self, node: &Node) {
        for qualifier in &node.qualifiers {
            self.bind(&qualifier.name);
        }
        match &node.value {
            Value::Simple(_) | Value::Resource(_) => {}
            Value::Struct(fields) => fields.iter().for_each(|f| self.property(f)),
            Value::Array(_, items) => items.iter().for_each(|i| self.node(i)),
        }
    }

    /// The name as written, `prefix:local`.
    fn name(&self, name: &Name) -> String {
        if name.namespace == RDF {
            return format!("rdf:{}", name.local);
        }
        if name.namespace == XML {
            return format!("xml:{}", name.local);
        }
        let key = (name.namespace.clone(), name.prefix.clone());
        let prefix = self.by_name.get(&key).map_or("", String::as_str);
        format!("{prefix}:{}", name.local)
    }
}

fn reserved(prefix: &str) -> bool {
    matches!(prefix, "rdf" | "x" | "xml" | "xmlns")
}

/// Whether XML 1.0 cannot hold the character at all, not even as a reference.
fn unrepresentable(c: char) -> bool {
    matches!(
        c,
        '\0'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}'
    )
}

/// The text without the characters XML 1.0 cannot hold; the rest of the control
/// characters are kept by the escaping. The value of `what` is changed by this, which is
/// logged.
fn clean(what: &str, text: &str) -> String {
    if !text.chars().any(unrepresentable) {
        return text.to_owned();
    }
    tracing::warn!("{what}: characters XML 1.0 cannot hold were dropped from its value on writing");
    text.chars().filter(|&c| !unrepresentable(c)).collect()
}

/// An attribute's value, escaped and with the characters an attribute would normalise as
/// numeric references; quick-xml's escaping writes a carriage return as `&#13;` itself.
fn attribute_value(what: &str, text: &str) -> String {
    escape(clean(what, text))
        .replace('\n', "&#xA;")
        .replace('\t', "&#x9;")
}

fn element_text(what: &str, text: &str) -> String {
    partial_escape(clean(what, text)).into_owned()
}

struct Writer<'a> {
    out: String,
    prefixes: &'a Prefixes,
}

impl Writer<'_> {
    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push(' ');
        }
    }

    fn attribute(&self, name: &Name, value: &str) -> String {
        let name = self.prefixes.name(name);
        format!("{name}=\"{}\"", attribute_value(&name, value))
    }

    /// An `rdf:Description` at `depth` with these properties, and `declarations` and
    /// `extra` attributes ahead of them.
    fn description(
        &mut self,
        depth: usize,
        about: Option<&str>,
        declarations: &[String],
        fields: &[Property],
        extra: &[String],
    ) {
        self.indent(depth);
        self.out.push_str("<rdf:Description");
        if let Some(about) = about {
            self.out.push_str(&format!(
                " rdf:about=\"{}\"",
                attribute_value("rdf:about", about)
            ));
        }
        // The properties that lead as attributes: an attribute cannot repeat in an element,
        // so a property whose name an earlier one has already taken (two merged
        // descriptions that both say `a:x`) starts the elements, and the order is kept.
        let mut taken: HashSet<(&str, &str)> = HashSet::new();
        let as_attribute = fields
            .iter()
            .take_while(|f| {
                let key = (f.name.namespace.as_str(), f.name.local.as_str());
                is_attribute(f) && taken.insert(key)
            })
            .count();
        let attributes: Vec<String> = declarations
            .iter()
            .cloned()
            .chain(fields[..as_attribute].iter().map(|f| {
                let text = f.node.as_str().unwrap_or_default();
                self.attribute(&f.name, text)
            }))
            .chain(extra.iter().cloned())
            .collect();
        for attribute in attributes {
            self.out.push('\n');
            self.indent(depth + 2);
            self.out.push_str(&attribute);
        }
        let children = &fields[as_attribute..];
        if children.is_empty() {
            self.out.push_str("/>\n");
            return;
        }
        self.out.push_str(">\n");
        for child in children {
            self.node(depth + 1, &self.prefixes.name(&child.name), &child.node);
        }
        self.indent(depth);
        self.out.push_str("</rdf:Description>\n");
    }

    /// A property or an item: `<tag>` with the node.
    fn node(&mut self, depth: usize, tag: &str, node: &Node) {
        let simple = matches!(node.value, Value::Simple(_) | Value::Resource(_));
        let others: Vec<&_> = node
            .qualifiers
            .iter()
            .filter(|q| !(simple && q.name.is(XML, "lang")))
            .collect();
        if others.is_empty() {
            self.plain(depth, tag, node);
            return;
        }
        // Qualified: a description whose `rdf:value` is the node, its qualifiers attributes.
        // A language stays with the text it qualifies.
        let value = Node {
            value: node.value.clone(),
            qualifiers: node
                .qualifiers
                .iter()
                .filter(|q| simple && q.name.is(XML, "lang"))
                .cloned()
                .collect(),
        };
        // The qualifiers follow `rdf:value` as properties of the description, so that one
        // that repeats a name, as the reader allows, becomes an element and the order is
        // kept. The language of a struct or an array stays an attribute, as `xml:lang` is
        // no element name.
        let mut fields = vec![Property::new(Name::rdf_value(), value)];
        let mut extra = Vec::new();
        for qualifier in others {
            if qualifier.name.is(XML, "lang") {
                extra.push(self.attribute(&qualifier.name, &qualifier.value));
            } else {
                fields.push(Property::simple(qualifier.name.clone(), &qualifier.value));
            }
        }
        self.indent(depth);
        self.out.push_str(&format!("<{tag}>\n"));
        self.description(depth + 1, None, &[], &fields, &extra);
        self.indent(depth);
        self.out.push_str(&format!("</{tag}>\n"));
    }

    /// The node with no qualifier but a language.
    fn plain(&mut self, depth: usize, tag: &str, node: &Node) {
        self.indent(depth);
        let lang = node
            .lang()
            .map(|lang| format!(" xml:lang=\"{}\"", attribute_value(tag, lang)))
            .unwrap_or_default();
        match &node.value {
            Value::Simple(text) if text.is_empty() => {
                self.out.push_str(&format!("<{tag}{lang}/>\n"));
            }
            Value::Simple(text) => {
                self.out.push_str(&format!(
                    "<{tag}{lang}>{}</{tag}>\n",
                    element_text(tag, text)
                ));
            }
            Value::Resource(uri) => {
                self.out.push_str(&format!(
                    "<{tag}{lang} rdf:resource=\"{}\"/>\n",
                    attribute_value(tag, uri)
                ));
            }
            Value::Struct(fields) => {
                self.out.push_str(&format!("<{tag}>\n"));
                self.description(depth + 1, None, &[], fields, &[]);
                self.indent(depth);
                self.out.push_str(&format!("</{tag}>\n"));
            }
            Value::Array(kind, items) => {
                let container = format!("rdf:{}", kind.local());
                self.out.push_str(&format!("<{tag}>\n"));
                self.indent(depth + 1);
                self.out.push_str(&format!("<{container}>\n"));
                for item in items {
                    self.node(depth + 2, "rdf:li", item);
                }
                self.indent(depth + 1);
                self.out.push_str(&format!("</{container}>\n"));
                self.indent(depth);
                self.out.push_str(&format!("</{tag}>\n"));
            }
        }
    }
}

/// Whether a property can be an attribute: simple text with no qualifier.
fn is_attribute(property: &Property) -> bool {
    matches!(property.node.value, Value::Simple(_)) && property.node.qualifiers.is_empty()
}

/// The packet's XML from `<x:xmpmeta>` to `</x:xmpmeta>`, a line each, ending with a
/// newline.
fn body(xmp: &Xmp) -> String {
    let prefixes = Prefixes::new(xmp);
    let mut out = format!("<x:xmpmeta xmlns:x=\"{META}\"");
    if let Some(toolkit) = &xmp.toolkit {
        out.push_str(&format!(
            " x:xmptk=\"{}\"",
            attribute_value("x:xmptk", toolkit)
        ));
    }
    out.push_str(&format!(">\n <rdf:RDF xmlns:rdf=\"{RDF}\">\n"));
    let declarations: Vec<String> = prefixes
        .declarations
        .iter()
        .map(|(prefix, uri)| format!("xmlns:{prefix}=\"{}\"", attribute_value(prefix, uri)))
        .collect();
    let mut writer = Writer {
        out,
        prefixes: &prefixes,
    };
    writer.description(2, Some(&xmp.about), &declarations, &xmp.properties, &[]);
    writer.out.push_str(" </rdf:RDF>\n</x:xmpmeta>\n");
    writer.out
}

/// The packet as a sidecar file holds it: an XML declaration and the packet, no wrapper.
pub(super) fn to_xml(xmp: &Xmp) -> String {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body(xmp))
}

/// The packet as it is embedded in a file: inside the `xpacket` wrapper, with `padding`
/// bytes of whitespace for the next edit to grow into.
pub(super) fn to_packet(xmp: &Xmp, padding: usize) -> String {
    let mut out = format!(
        "<?xpacket begin=\"\u{feff}\" id=\"{PACKET_ID}\"?>\n{}",
        body(xmp)
    );
    let mut left = padding;
    while left > 0 {
        let line = left.min(100);
        out.push_str(&" ".repeat(line.saturating_sub(1)));
        out.push('\n');
        left -= line;
    }
    out.push_str("<?xpacket end=\"w\"?>");
    out
}
