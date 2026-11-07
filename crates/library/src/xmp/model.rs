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

//! The XMP data model: what a packet says, with no XML in it.
//!
//! An [`Xmp`] holds properties in the order the packet has them. A [`Property`] is a name
//! and a [`Node`]; a node is a [`Value`] and its qualifiers. A value is simple text, a
//! resource, a struct (more properties) or an array (more nodes). The same node is read from
//! every way RDF/XML can write it, so two packets that say the same thing give equal trees.

/// The namespace of RDF, whose `Description`, `Seq`, `li` and `value` give the packet its
/// structure.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// The namespace of the `xml:` attributes, `xml:lang` the one XMP uses.
pub const XML: &str = "http://www.w3.org/XML/1998/namespace";
/// The namespace of XMP's wrapper, whose `x:xmpmeta` holds the RDF and `x:xmptk` names the
/// toolkit that wrote it.
pub const META: &str = "adobe:ns:meta/";
/// The language of an alternative that applies when no other does.
pub const DEFAULT_LANGUAGE: &str = "x-default";

/// A qualified name: the namespace's URI, the prefix the packet gave it and the local
/// name. Two names are the same property when [`Name::is`] says so; the prefix is kept so
/// that a packet is written back with the prefixes its author chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub namespace: String,
    pub prefix: String,
    pub local: String,
}

impl Name {
    pub fn new(prefix: &str, namespace: &str, local: &str) -> Self {
        Name {
            namespace: namespace.to_owned(),
            prefix: prefix.to_owned(),
            local: local.to_owned(),
        }
    }

    /// `xml:lang`, the qualifier of a language alternative.
    pub fn lang() -> Self {
        Name::new("xml", XML, "lang")
    }

    /// `rdf:value`, the property that holds a qualified node's own value.
    pub(super) fn rdf_value() -> Self {
        Name::new("rdf", RDF, "value")
    }

    /// Whether this is the property `local` of the namespace `namespace`, whatever prefix
    /// it has.
    pub fn is(&self, namespace: &str, local: &str) -> bool {
        self.namespace == namespace && self.local == local
    }
}

/// A qualifier: a name and simple text that say something about a value, as `xml:lang`
/// does about a text. The first qualifier of a node is its `xml:lang` where it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qualifier {
    pub name: Name,
    pub value: String,
}

/// The three kinds of array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayKind {
    /// Unordered.
    Bag,
    /// Ordered.
    Seq,
    /// Alternatives, one of which applies, each often with its `xml:lang`.
    Alt,
}

impl ArrayKind {
    /// The local name of the RDF container element.
    pub(super) fn local(self) -> &'static str {
        match self {
            ArrayKind::Bag => "Bag",
            ArrayKind::Seq => "Seq",
            ArrayKind::Alt => "Alt",
        }
    }
}

/// What a node holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Text, as the packet has it after unescaping.
    Simple(String),
    /// A URI given as `rdf:resource`, which has no text.
    Resource(String),
    /// Properties, in the packet's order.
    Struct(Vec<Property>),
    Array(ArrayKind, Vec<Node>),
}

/// A value and its qualifiers: a property's content, or an array's item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub value: Value,
    pub qualifiers: Vec<Qualifier>,
}

impl Node {
    pub fn simple(text: &str) -> Self {
        Node {
            value: Value::Simple(text.to_owned()),
            qualifiers: Vec::new(),
        }
    }

    /// A text with its `xml:lang`, an item of a language alternative.
    pub fn with_lang(text: &str, lang: &str) -> Self {
        Node {
            value: Value::Simple(text.to_owned()),
            qualifiers: vec![Qualifier {
                name: Name::lang(),
                value: lang.to_owned(),
            }],
        }
    }

    pub fn structure(fields: Vec<Property>) -> Self {
        Node {
            value: Value::Struct(fields),
            qualifiers: Vec::new(),
        }
    }

    pub fn array(kind: ArrayKind, items: Vec<Node>) -> Self {
        Node {
            value: Value::Array(kind, items),
            qualifiers: Vec::new(),
        }
    }

    /// The text of a simple value, or the URI of a resource.
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Simple(text) | Value::Resource(text) => Some(text),
            _ => None,
        }
    }

    /// The `xml:lang` qualifier.
    pub fn lang(&self) -> Option<&str> {
        self.qualifiers
            .iter()
            .find(|q| q.name.is(XML, "lang"))
            .map(|q| q.value.as_str())
    }

    /// The fields of a struct.
    pub fn fields(&self) -> Option<&[Property]> {
        match &self.value {
            Value::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    /// The items of an array of any kind.
    pub fn items(&self) -> Option<&[Node]> {
        match &self.value {
            Value::Array(_, items) => Some(items),
            _ => None,
        }
    }

    /// The field of a struct with this name.
    pub fn field(&self, namespace: &str, local: &str) -> Option<&Node> {
        find(self.fields()?, namespace, local)
    }
}

/// A named node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: Name,
    pub node: Node,
}

impl Property {
    pub fn new(name: Name, node: Node) -> Self {
        Property { name, node }
    }

    /// A simple, unqualified property.
    pub fn simple(name: Name, text: &str) -> Self {
        Property::new(name, Node::simple(text))
    }
}

/// A namespace declaration: the prefix a packet binds to a URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    pub prefix: String,
    pub uri: String,
}

impl Namespace {
    pub fn new(prefix: &str, uri: &str) -> Self {
        Namespace {
            prefix: prefix.to_owned(),
            uri: uri.to_owned(),
        }
    }
}

/// A packet: the properties of its `rdf:Description`s merged in order, the prefixes it
/// declared and the toolkit that wrote it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Xmp {
    /// `x:xmptk` of `x:xmpmeta`, the toolkit that wrote the packet.
    pub toolkit: Option<String>,
    /// `rdf:about` of the first description that has one; empty for a sidecar.
    pub about: String,
    /// The prefixes the packet declared, in its order, which a write declares again, with
    /// those it needs for names it was given without a declaration. `rdf`, `x` and `xml`
    /// are never listed.
    pub namespaces: Vec<Namespace>,
    /// Every top-level property, unknown ones included, in the packet's order.
    pub properties: Vec<Property>,
}

impl Xmp {
    /// The first top-level property with this name.
    pub fn get(&self, namespace: &str, local: &str) -> Option<&Node> {
        find(&self.properties, namespace, local)
    }

    /// Sets a top-level property, replacing the first with its name where there is one and
    /// otherwise adding it last; the properties around it keep their places.
    pub fn set(&mut self, property: Property) {
        let name = &property.name;
        match self
            .properties
            .iter_mut()
            .find(|p| p.name.is(&name.namespace, &name.local))
        {
            Some(existing) => existing.node = property.node,
            None => self.properties.push(property),
        }
    }

    /// Removes every top-level property with this name.
    pub fn remove(&mut self, namespace: &str, local: &str) {
        self.properties.retain(|p| !p.name.is(namespace, local));
    }
}

fn find<'a>(properties: &'a [Property], namespace: &str, local: &str) -> Option<&'a Node> {
    properties
        .iter()
        .find(|p| p.name.is(namespace, local))
        .map(|p| &p.node)
}
