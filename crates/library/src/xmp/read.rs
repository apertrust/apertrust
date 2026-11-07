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

//! Reading a packet: quick-xml's events into an element tree, with every name resolved to
//! its namespace and every error given its line (worked out once, at the end), then the tree's RDF/XML into the model.
//!
//! The RDF read is the subset XMP uses: `rdf:Description` for the properties and for
//! structs, `rdf:Bag`, `rdf:Seq` and `rdf:Alt` with `rdf:li` for arrays, `rdf:parseType=
//! "Resource"`, `rdf:resource`, `rdf:value` with qualifiers, and simple values as attributes
//! or as text. What RDF has beyond that, typed nodes, `parseType="Literal"` and
//! `"Collection"`, reification and the rest, is refused with the line it is on.

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{PrefixDeclaration, ResolveResult};
use quick_xml::{NsReader, XmlVersion};

use super::model::{
    ArrayKind, META, Name, Namespace, Node, Property, Qualifier, RDF, Value, XML, Xmp,
};
use super::{Error, line_of};

/// How deep elements may nest. A packet is a few levels deep; this stops one built to
/// overflow the stack.
const MAX_DEPTH: usize = 100;

const VERSION: XmlVersion = XmlVersion::Implicit1_0;

/// An element with its names resolved.
struct Element {
    name: Name,
    /// The attributes that are no namespace declaration, in order.
    attrs: Vec<(Name, String)>,
    /// The declarations on this element, in order.
    declarations: Vec<Namespace>,
    children: Vec<Child>,
    offset: usize,
}

enum Child {
    Element(Element),
    Text(String),
}

impl Element {
    fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|child| match child {
            Child::Element(element) => Some(element),
            Child::Text(_) => None,
        })
    }

    /// The element's text, its pieces joined.
    fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|child| match child {
                Child::Text(text) => Some(text.as_str()),
                Child::Element(_) => None,
            })
            .collect()
    }

    /// Fails unless the element holds no text but whitespace between its elements.
    fn require_blank(&self) -> Result<(), Fail> {
        if self.text().trim().is_empty() {
            Ok(())
        } else {
            Err(Fail::at(
                self.offset,
                format!("text next to the elements of <{}>", self.name.local),
            ))
        }
    }
}

/// Reads a packet from XML text.
pub(super) fn parse(xml: &str) -> Result<Xmp, Error> {
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    tree(xml)
        .and_then(|root| interpret(&root))
        .map_err(|fail| Error::at(line_of(xml, fail.offset), fail.message))
}

/// A failure while reading, at a byte offset of the text. The line is worked out once, when
/// the read is over, so reading stays linear.
struct Fail {
    offset: usize,
    message: String,
}

impl Fail {
    fn at(offset: usize, message: impl Into<String>) -> Self {
        Fail {
            offset,
            message: message.into(),
        }
    }
}

// -- The element tree --------------------------------------------------------------------

/// A resolved namespace, owned.
enum Resolved {
    Bound(String),
    Unbound,
    Unknown(String),
}

impl Resolved {
    fn of(result: ResolveResult<'_>) -> Self {
        match result {
            ResolveResult::Bound(namespace) => Resolved::Bound(lossy(namespace.as_ref())),
            ResolveResult::Unbound => Resolved::Unbound,
            ResolveResult::Unknown(prefix) => Resolved::Unknown(lossy(&prefix)),
        }
    }
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// quick-xml's events as a tree: the root element.
fn tree(xml: &str) -> Result<Element, Fail> {
    let mut reader = NsReader::from_str(xml);
    let mut open: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    loop {
        let start = reader.buffer_position() as usize;
        let (resolved, event) = match reader.read_resolved_event() {
            Ok((resolved, event)) => (Resolved::of(resolved), event),
            Err(error) => {
                let at = match reader.error_position() {
                    0 => reader.buffer_position(),
                    at => at,
                };
                return Err(Fail::at(at as usize, error.to_string()));
            }
        };
        let offset = start;
        match event {
            Event::Start(tag) => {
                if open.len() >= MAX_DEPTH {
                    return Err(Fail::at(offset, "elements nested too deeply"));
                }
                open.push(element(&reader, resolved, &tag, offset)?);
            }
            Event::Empty(tag) => {
                let element = element(&reader, resolved, &tag, offset)?;
                attach(&mut open, &mut root, element, offset)?;
            }
            Event::End(_) => {
                let element = open
                    .pop()
                    .ok_or_else(|| Fail::at(offset, "an end tag with no start tag"))?;
                attach(&mut open, &mut root, element, offset)?;
            }
            Event::Text(text) => {
                let text = text
                    .xml_content(VERSION)
                    .map_err(|e| Fail::at(offset, e.to_string()))?;
                add_text(&mut open, &text, offset)?;
            }
            Event::CData(data) => {
                let text = data
                    .xml_content(VERSION)
                    .map_err(|e| Fail::at(offset, e.to_string()))?;
                add_text(&mut open, &text, offset)?;
            }
            Event::GeneralRef(reference) => {
                let text = reference_text(&reference, offset)?;
                add_text(&mut open, &text, offset)?;
            }
            Event::Eof => break,
            Event::Comment(_) | Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }
    if let Some(unclosed) = open.last() {
        return Err(Fail::at(
            unclosed.offset,
            format!("<{}> is never closed", unclosed.name.local),
        ));
    }
    root.ok_or_else(|| Fail::at(1, "no XML element"))
}

/// The text of `&name;` or `&#N;`: the five predefined entities and character references.
fn reference_text(
    reference: &quick_xml::events::BytesRef<'_>,
    offset: usize,
) -> Result<String, Fail> {
    if let Some(character) = reference
        .resolve_char_ref()
        .map_err(|e| Fail::at(offset, e.to_string()))?
    {
        return Ok(character.to_string());
    }
    let name = reference
        .decode()
        .map_err(|e| Fail::at(offset, e.to_string()))?;
    match name.as_ref() {
        "lt" => Ok("<".to_owned()),
        "gt" => Ok(">".to_owned()),
        "amp" => Ok("&".to_owned()),
        "quot" => Ok("\"".to_owned()),
        "apos" => Ok("'".to_owned()),
        other => Err(Fail::at(offset, format!("unknown entity &{other};"))),
    }
}

/// Adds text to the open element; outside the root only whitespace may be.
fn add_text(open: &mut [Element], text: &str, offset: usize) -> Result<(), Fail> {
    let Some(parent) = open.last_mut() else {
        // The text starts where the last event ended, and whitespace leads it.
        let leading: usize = text
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum();
        let offset = offset + leading;
        return if text.trim().is_empty() {
            Ok(())
        } else {
            Err(Fail::at(offset, "text outside the root element"))
        };
    };
    if let Some(Child::Text(last)) = parent.children.last_mut() {
        last.push_str(text);
    } else {
        parent.children.push(Child::Text(text.to_owned()));
    }
    Ok(())
}

/// Puts a finished element in its parent, or makes it the root.
fn attach(
    open: &mut [Element],
    root: &mut Option<Element>,
    element: Element,
    offset: usize,
) -> Result<(), Fail> {
    if let Some(parent) = open.last_mut() {
        parent.children.push(Child::Element(element));
    } else if root.is_some() {
        return Err(Fail::at(offset, "more than one root element"));
    } else {
        *root = Some(element);
    }
    Ok(())
}

/// An element from its start tag: its name and attributes resolved, its declarations kept.
fn element(
    reader: &NsReader<&[u8]>,
    resolved: Resolved,
    tag: &BytesStart<'_>,
    offset: usize,
) -> Result<Element, Fail> {
    let qname = tag.name();
    let local = lossy(tag.local_name().as_ref());
    let prefix = qname
        .prefix()
        .map(|p| lossy(p.as_ref()))
        .unwrap_or_default();
    let name = match resolved {
        Resolved::Bound(namespace) => Name {
            namespace,
            prefix,
            local,
        },
        // Only a property must have a namespace, which `property` checks.
        Resolved::Unbound => Name {
            namespace: String::new(),
            prefix,
            local,
        },
        Resolved::Unknown(prefix) => {
            return Err(Fail::at(
                offset,
                format!("the prefix {prefix}: is not declared"),
            ));
        }
    };
    let mut attrs = Vec::new();
    let mut declarations = Vec::new();
    for attribute in tag.attributes() {
        let attribute = attribute.map_err(|e| Fail::at(offset, e.to_string()))?;
        let value = attribute
            .normalized_value(VERSION)
            .map_err(|e| Fail::at(offset, e.to_string()))?
            .into_owned();
        if let Some(declaration) = attribute.key.as_namespace_binding() {
            let prefix = match declaration {
                PrefixDeclaration::Default => String::new(),
                PrefixDeclaration::Named(prefix) => lossy(prefix),
            };
            declarations.push(Namespace { prefix, uri: value });
            continue;
        }
        let (resolved, local) = reader.resolver().resolve_attribute(attribute.key);
        let local = lossy(local.as_ref());
        let prefix = attribute
            .key
            .prefix()
            .map(|p| lossy(p.as_ref()))
            .unwrap_or_default();
        match Resolved::of(resolved) {
            Resolved::Bound(namespace) => attrs.push((
                Name {
                    namespace,
                    prefix,
                    local,
                },
                value,
            )),
            Resolved::Unbound => {
                return Err(Fail::at(
                    offset,
                    format!("the attribute {local} is in no namespace"),
                ));
            }
            Resolved::Unknown(prefix) => {
                return Err(Fail::at(
                    offset,
                    format!("the prefix {prefix}: is not declared"),
                ));
            }
        }
    }
    Ok(Element {
        name,
        attrs,
        declarations,
        children: Vec::new(),
        offset,
    })
}

// -- RDF/XML into the model --------------------------------------------------------------

fn interpret(root: &Element) -> Result<Xmp, Fail> {
    let mut xmp = Xmp::default();
    let mut rdfs = Vec::new();
    if root.name.is(META, "xmpmeta") {
        declare(&mut xmp, &root.declarations);
        for (name, value) in &root.attrs {
            if name.is(META, "xmptk") {
                xmp.toolkit = Some(value.clone());
            }
        }
        root.require_blank()?;
        for child in root.elements() {
            if child.name.is(RDF, "RDF") {
                rdfs.push(child);
            } else {
                return Err(unexpected(child, "inside x:xmpmeta"));
            }
        }
    } else if root.name.is(RDF, "RDF") {
        rdfs.push(root);
    } else {
        return Err(Fail::at(
            root.offset,
            format!(
                "not XMP: the root element is <{}>, not x:xmpmeta or rdf:RDF",
                root.name.local
            ),
        ));
    }
    for rdf in rdfs {
        declare(&mut xmp, &rdf.declarations);
        rdf.require_blank()?;
        for description in rdf.elements() {
            if !description.name.is(RDF, "Description") {
                return Err(unexpected(
                    description,
                    "in rdf:RDF, where rdf:Description goes",
                ));
            }
            declare(&mut xmp, &description.declarations);
            if xmp.about.is_empty()
                && let Some((_, about)) = description.attrs.iter().find(|(n, _)| n.is(RDF, "about"))
            {
                xmp.about = about.clone();
            }
            let (fields, _) = description_fields(description)?;
            xmp.properties.extend(fields);
        }
    }
    Ok(xmp)
}

fn unexpected(element: &Element, place: &str) -> Fail {
    Fail::at(
        element.offset,
        format!("unexpected <{}> {place}", element.name.local),
    )
}

/// Records the prefixes an element declares, but for the default namespace and the ones
/// the format fixes.
fn declare(xmp: &mut Xmp, declarations: &[Namespace]) {
    for declaration in declarations {
        let fixed = [RDF, META, XML].contains(&declaration.uri.as_str());
        if !declaration.prefix.is_empty() && !fixed && !xmp.namespaces.contains(declaration) {
            xmp.namespaces.push(declaration.clone());
        }
    }
}

/// An element's attributes as RDF reads them.
struct Attrs {
    parse_type: Option<String>,
    resource: Option<String>,
    /// `xml:lang`.
    qualifiers: Vec<Qualifier>,
    /// The attributes that are properties, in order.
    fields: Vec<Property>,
}

fn attrs(element: &Element) -> Result<Attrs, Fail> {
    let mut found = Attrs {
        parse_type: None,
        resource: None,
        qualifiers: Vec::new(),
        fields: Vec::new(),
    };
    for (name, value) in &element.attrs {
        if name.namespace == RDF {
            match name.local.as_str() {
                "about" | "ID" | "nodeID" | "datatype" => {}
                "parseType" => found.parse_type = Some(value.clone()),
                "resource" => found.resource = Some(value.clone()),
                "value" => found.fields.push(Property::simple(name.clone(), value)),
                other => {
                    return Err(Fail::at(
                        element.offset,
                        format!("rdf:{other} as an attribute is not XMP"),
                    ));
                }
            }
        } else if name.namespace == XML {
            if name.local == "lang" {
                found.qualifiers.push(Qualifier {
                    name: name.clone(),
                    value: value.clone(),
                });
            }
        } else {
            found.fields.push(Property::simple(name.clone(), value));
        }
    }
    Ok(found)
}

/// The fields of a description or a struct, its property attributes then its property
/// elements, with the qualifiers its attributes give.
fn description_fields(description: &Element) -> Result<(Vec<Property>, Vec<Qualifier>), Fail> {
    let found = attrs(description)?;
    if found.parse_type.is_some() || found.resource.is_some() {
        return Err(Fail::at(
            description.offset,
            "rdf:parseType and rdf:resource do not belong on rdf:Description",
        ));
    }
    description.require_blank()?;
    let mut fields = found.fields;
    for child in description.elements() {
        fields.push(property(child)?);
    }
    Ok((fields, found.qualifiers))
}

fn property(element: &Element) -> Result<Property, Fail> {
    if element.name.namespace.is_empty() {
        return Err(Fail::at(
            element.offset,
            format!("<{}> is in no namespace", element.name.local),
        ));
    }
    if element.name.namespace == RDF && element.name.local != "value" {
        return Err(unexpected(element, "where a property goes"));
    }
    Ok(Property::new(element.name.clone(), node(element)?))
}

/// The node a property element, or an `rdf:li`, holds, in whichever form it is written.
fn node(element: &Element) -> Result<Node, Fail> {
    let found = attrs(element)?;
    let elements: Vec<&Element> = element.elements().collect();
    match found.parse_type.as_deref() {
        Some("Resource") => {
            element.require_blank()?;
            let mut fields = found.fields;
            for child in &elements {
                fields.push(property(child)?);
            }
            return qualify(fields, found.qualifiers, element.offset);
        }
        Some(other) => {
            return Err(Fail::at(
                element.offset,
                format!("rdf:parseType=\"{other}\" is not XMP"),
            ));
        }
        None => {}
    }
    if let Some(uri) = found.resource {
        if !elements.is_empty() || !element.text().trim().is_empty() {
            return Err(Fail::at(
                element.offset,
                "rdf:resource on an element that has content",
            ));
        }
        let mut qualifiers = found.qualifiers;
        for field in found.fields {
            qualifiers.push(simple_qualifier(field, element.offset)?);
        }
        return Ok(Node {
            value: Value::Resource(uri),
            qualifiers,
        });
    }
    if let Some(&child) = elements.first() {
        if elements.len() > 1 {
            return Err(Fail::at(
                elements[1].offset,
                format!("<{}> holds more than one value", element.name.local),
            ));
        }
        element.require_blank()?;
        if !found.fields.is_empty() {
            return Err(Fail::at(
                element.offset,
                "property attributes next to a value element",
            ));
        }
        return value_element(child, found.qualifiers);
    }
    if !found.fields.is_empty() {
        // `<p a:x="1" b:y="2"/>`: a struct written as attributes alone.
        element.require_blank()?;
        return qualify(found.fields, found.qualifiers, element.offset);
    }
    Ok(Node {
        value: Value::Simple(element.text()),
        qualifiers: found.qualifiers,
    })
}

/// The one element inside a property: an array or a description.
fn value_element(element: &Element, qualifiers: Vec<Qualifier>) -> Result<Node, Fail> {
    let kind = [ArrayKind::Bag, ArrayKind::Seq, ArrayKind::Alt]
        .into_iter()
        .find(|kind| element.name.is(RDF, kind.local()));
    if let Some(kind) = kind {
        element.require_blank()?;
        let mut items = Vec::new();
        for item in element.elements() {
            if !item.name.is(RDF, "li") {
                return Err(unexpected(item, "in an array, where rdf:li goes"));
            }
            items.push(node(item)?);
        }
        return Ok(Node {
            value: Value::Array(kind, items),
            qualifiers,
        });
    }
    if element.name.is(RDF, "Description") {
        let (fields, mut lang) = description_fields(element)?;
        let mut all = qualifiers;
        all.append(&mut lang);
        return qualify(fields, all, element.offset);
    }
    Err(unexpected(
        element,
        "as a value: typed nodes are not XMP, only rdf:Description, rdf:Bag, rdf:Seq and rdf:Alt",
    ))
}

/// A struct from its fields, or, where one is `rdf:value`, that value with the other
/// fields as its qualifiers. The `xml:lang` comes first.
fn qualify(
    mut fields: Vec<Property>,
    qualifiers: Vec<Qualifier>,
    offset: usize,
) -> Result<Node, Fail> {
    let Some(at) = fields.iter().position(|f| f.name.is(RDF, "value")) else {
        return Ok(Node {
            value: Value::Struct(fields),
            qualifiers,
        });
    };
    let mut node = fields.remove(at).node;
    let mut all = std::mem::take(&mut node.qualifiers);
    all.extend(qualifiers);
    for field in fields {
        all.push(simple_qualifier(field, offset)?);
    }
    all.sort_by_key(|q| !q.name.is(XML, "lang"));
    node.qualifiers = all;
    Ok(node)
}

fn simple_qualifier(field: Property, offset: usize) -> Result<Qualifier, Fail> {
    match field.node {
        Node {
            value: Value::Simple(value),
            qualifiers,
        } if qualifiers.is_empty() => Ok(Qualifier {
            name: field.name,
            value,
        }),
        _ => Err(Fail::at(
            offset,
            format!("the qualifier {} is not simple text", field.name.local),
        )),
    }
}
