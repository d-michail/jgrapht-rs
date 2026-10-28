// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

//! GraphML import and export using a streaming XML reader.
//!
//! GraphML is an XML-based graph exchange format. See the
//! [GraphML Primer](http://graphml.graphdrawing.org/primer/graphml-primer.html)
//! for a description of the format.

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{
    Attribute, AttributeMap, AttributeType, BaseExporter, EventDrivenImporter, GraphExporter,
    GraphImporter, ImportEvent, IntegerIdProvider, IoError, Result,
};
use indexmap::IndexMap;
use jgrapht::{DefaultEdge, Graph, GraphMut};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{BufReader, Read, Write};

#[derive(Debug, Clone)]
struct Key {
    target: String,
    name: String,
    kind: AttributeType,
    default: Option<String>,
}

fn xml_attr(
    reader: &Reader<BufReader<impl Read>>,
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>> {
    for attribute in element.attributes() {
        let attribute = attribute
            .map_err(|error| IoError::Import(format!("invalid XML attribute: {error}")))?;
        if attribute.key.as_ref() == name {
            return Ok(Some(
                attribute
                    .decoded_and_normalized_value(
                        quick_xml::XmlVersion::Implicit1_0,
                        reader.decoder(),
                    )
                    .map_err(|error| {
                        IoError::Import(format!("invalid XML attribute value: {error}"))
                    })?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

fn graphml_type(value: &str) -> AttributeType {
    match value {
        "boolean" => AttributeType::Boolean,
        "int" => AttributeType::Int,
        "long" => AttributeType::Long,
        "float" => AttributeType::Float,
        "double" => AttributeType::Double,
        "string" => AttributeType::String,
        _ => AttributeType::Unknown,
    }
}

#[derive(Debug)]
struct ParsedNode {
    id: String,
    attributes: AttributeMap,
}

#[derive(Debug)]
struct ParsedEdge {
    id: Option<String>,
    source: String,
    target: String,
    attributes: AttributeMap,
}

#[derive(Debug)]
struct ParsedDocument {
    graph_attributes: AttributeMap,
    nodes: Vec<ParsedNode>,
    edges: Vec<ParsedEdge>,
}

fn element_attributes(
    reader: &Reader<BufReader<impl Read>>,
    element: &BytesStart<'_>,
    excluded: &[&[u8]],
) -> Result<AttributeMap> {
    let mut result = AttributeMap::new();
    for item in element.attributes() {
        let item =
            item.map_err(|error| IoError::Import(format!("invalid XML attribute: {error}")))?;
        if excluded.contains(&item.key.local_name().as_ref()) {
            continue;
        }
        let name = String::from_utf8_lossy(item.key.local_name().as_ref()).into_owned();
        let value = item
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|error| IoError::Import(format!("invalid XML attribute value: {error}")))?;
        result.insert(name, Attribute::from(value.into_owned()));
    }
    Ok(result)
}

fn parse<R: Read>(input: R) -> Result<ParsedDocument> {
    let mut reader = Reader::from_reader(BufReader::new(input));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut keys = HashMap::<String, Key>::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_stack = Vec::<ParsedNode>::new();
    let mut edge_stack = Vec::<ParsedEdge>::new();
    let mut graph_attributes = AttributeMap::new();
    let mut root_seen = false;
    let mut graph_seen = false;
    let mut key_id: Option<String> = None;
    let mut data_key: Option<String> = None;
    let mut text = String::new();
    let mut in_default = false;
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| IoError::Import(format!("GraphML XML error: {error}")))?
        {
            Event::Start(element) => match element.local_name().as_ref() {
                b"graphml" => root_seen = true,
                b"graph" => graph_seen = true,
                b"key" => {
                    let id = xml_attr(&reader, &element, b"id")?
                        .ok_or_else(|| IoError::Import("GraphML key is missing id".into()))?;
                    let target =
                        xml_attr(&reader, &element, b"for")?.unwrap_or_else(|| "all".into());
                    let name =
                        xml_attr(&reader, &element, b"attr.name")?.unwrap_or_else(|| id.clone());
                    let kind = graphml_type(
                        xml_attr(&reader, &element, b"attr.type")?
                            .as_deref()
                            .unwrap_or("string"),
                    );
                    keys.insert(
                        id.clone(),
                        Key {
                            target,
                            name,
                            kind,
                            default: None,
                        },
                    );
                    key_id = Some(id);
                }
                b"default" => {
                    in_default = true;
                    text.clear();
                }
                b"node" => node_stack.push(ParsedNode {
                    id: xml_attr(&reader, &element, b"id")?
                        .ok_or_else(|| IoError::Import("GraphML node is missing id".into()))?,
                    attributes: element_attributes(&reader, &element, &[b"id"])?,
                }),
                b"edge" => edge_stack.push(ParsedEdge {
                    id: xml_attr(&reader, &element, b"id")?,
                    source: xml_attr(&reader, &element, b"source")?
                        .ok_or_else(|| IoError::Import("GraphML edge is missing source".into()))?,
                    target: xml_attr(&reader, &element, b"target")?
                        .ok_or_else(|| IoError::Import("GraphML edge is missing target".into()))?,
                    attributes: element_attributes(
                        &reader,
                        &element,
                        &[b"id", b"source", b"target"],
                    )?,
                }),
                b"hyperedge" | b"port" => {
                    return Err(IoError::Import("unsupported GraphML element".into()));
                }
                b"data" => {
                    data_key = xml_attr(&reader, &element, b"key")?;
                    text.clear();
                }
                _ => {}
            },
            Event::Empty(element) => match element.local_name().as_ref() {
                b"graphml" => root_seen = true,
                b"graph" => graph_seen = true,
                b"key" => {
                    let id = xml_attr(&reader, &element, b"id")?
                        .ok_or_else(|| IoError::Import("GraphML key is missing id".into()))?;
                    let target =
                        xml_attr(&reader, &element, b"for")?.unwrap_or_else(|| "all".into());
                    let name =
                        xml_attr(&reader, &element, b"attr.name")?.unwrap_or_else(|| id.clone());
                    let kind = graphml_type(
                        xml_attr(&reader, &element, b"attr.type")?
                            .as_deref()
                            .unwrap_or("string"),
                    );
                    keys.insert(
                        id,
                        Key {
                            target,
                            name,
                            kind,
                            default: None,
                        },
                    );
                }
                b"node" => nodes.push(ParsedNode {
                    id: xml_attr(&reader, &element, b"id")?
                        .ok_or_else(|| IoError::Import("GraphML node is missing id".into()))?,
                    attributes: element_attributes(&reader, &element, &[b"id"])?,
                }),
                b"edge" => edges.push(ParsedEdge {
                    id: xml_attr(&reader, &element, b"id")?,
                    source: xml_attr(&reader, &element, b"source")?
                        .ok_or_else(|| IoError::Import("GraphML edge is missing source".into()))?,
                    target: xml_attr(&reader, &element, b"target")?
                        .ok_or_else(|| IoError::Import("GraphML edge is missing target".into()))?,
                    attributes: element_attributes(
                        &reader,
                        &element,
                        &[b"id", b"source", b"target"],
                    )?,
                }),
                b"hyperedge" | b"port" => {
                    return Err(IoError::Import("unsupported GraphML element".into()));
                }
                _ => {}
            },
            Event::Text(value) => {
                if in_default || data_key.is_some() {
                    text.push_str(&value.decode().map_err(|error| {
                        IoError::Import(format!("invalid GraphML text: {error}"))
                    })?);
                }
            }
            Event::End(element) => match element.local_name().as_ref() {
                b"default" => {
                    if let Some(id) = &key_id
                        && let Some(key) = keys.get_mut(id)
                    {
                        key.default = Some(text.clone());
                    }
                    in_default = false;
                }
                b"key" => key_id = None,
                b"data" => {
                    if let Some(id) = data_key.take() {
                        let key = keys.get(&id).cloned().unwrap_or(Key {
                            target: "all".into(),
                            name: id,
                            kind: AttributeType::String,
                            default: None,
                        });
                        let attribute = Attribute::new(text.clone(), key.kind);
                        if let Some(edge) = edge_stack.last_mut() {
                            edge.attributes.insert(key.name, attribute);
                        } else if let Some(node) = node_stack.last_mut() {
                            node.attributes.insert(key.name, attribute);
                        } else {
                            graph_attributes.insert(key.name, attribute);
                        }
                    }
                }
                b"node" => {
                    if let Some(value) = node_stack.pop() {
                        nodes.push(value);
                    }
                }
                b"edge" => {
                    if let Some(value) = edge_stack.pop() {
                        edges.push(value);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            Event::DocType(_) => {
                return Err(IoError::Import(
                    "GraphML document types are not supported".into(),
                ));
            }
            _ => {}
        }
        buffer.clear();
    }
    if !root_seen || !graph_seen {
        return Err(IoError::Import("input is not a GraphML graph".into()));
    }
    for node in &mut nodes {
        for key in keys
            .values()
            .filter(|key| matches!(key.target.as_str(), "node" | "all"))
        {
            if let Some(default) = &key.default {
                node.attributes
                    .entry(key.name.clone())
                    .or_insert_with(|| Attribute::new(default, key.kind));
            }
        }
    }
    for edge in &mut edges {
        for key in keys
            .values()
            .filter(|key| matches!(key.target.as_str(), "edge" | "all"))
        {
            if let Some(default) = &key.default {
                edge.attributes
                    .entry(key.name.clone())
                    .or_insert_with(|| Attribute::new(default, key.kind));
            }
        }
    }
    Ok(ParsedDocument {
        graph_attributes,
        nodes,
        edges,
    })
}

type VertexFactory<V> =
    Box<dyn FnMut(&str, &AttributeMap) -> std::result::Result<V, crate::CallbackError>>;
type EdgeFactory<E> = Box<dyn FnMut(&AttributeMap) -> E>;
type GraphAttributeConsumer = Box<dyn FnMut(&str, &Attribute) -> Result<()>>;
type VertexAttributeConsumer<V> = Box<dyn FnMut(&V, &str, &Attribute) -> Result<()>>;
type EdgeAttributeConsumer<E> = Box<dyn FnMut(&E, &str, &Attribute) -> Result<()>>;

/// Imports a graph from a GraphML data source.
///
/// The destination graph must support the features present in the input, such
/// as self-loops or parallel edges. The `edgedefault` XML attribute is not used
/// to change the destination graph's directionality; that is determined by the
/// graph implementation supplied to [`GraphImporter::import_graph`].
///
/// Graph, vertex, and edge attributes are exposed through the configured
/// consumers. Vertex identifiers are additionally reported as an `ID` vertex
/// attribute. When the destination is weighted, an edge attribute named
/// `weight` is used by default as its weight.
pub struct GraphMlImporter<V, E = DefaultEdge> {
    vertex_factory: Option<VertexFactory<V>>,
    edge_factory: Option<EdgeFactory<E>>,
    edge_weight_attribute: String,
    schema_validation: bool,
    graph_attribute_consumer: Option<GraphAttributeConsumer>,
    vertex_attribute_consumer: Option<VertexAttributeConsumer<V>>,
    edge_attribute_consumer: Option<EdgeAttributeConsumer<E>>,
}

impl<V, E> GraphMlImporter<V, E> {
    /// Creates an importer with the default edge-weight attribute name.
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
            edge_factory: None,
            edge_weight_attribute: "weight".into(),
            schema_validation: true,
            graph_attribute_consumer: None,
            vertex_attribute_consumer: None,
            edge_attribute_consumer: None,
        }
    }
    /// Sets a factory which creates vertices from their GraphML identifiers.
    ///
    /// Without a custom factory, vertices are obtained from the destination
    /// graph's vertex supplier.
    pub fn set_vertex_factory(&mut self, mut factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(move |id, _| Ok(factory(id))));
    }

    /// Like [`set_vertex_factory`](Self::set_vertex_factory) for a factory that can fail, for
    /// example when it parses the identifier. Its error stops the import and is returned as
    /// [`IoError::Callback`] with the factory's error as the source.
    pub fn try_set_vertex_factory(
        &mut self,
        mut factory: impl FnMut(&str) -> std::result::Result<V, crate::CallbackError> + 'static,
    ) {
        self.vertex_factory = Some(Box::new(move |id, _| factory(id)));
    }
    /// Sets a factory which creates vertices from their identifiers and attributes.
    ///
    /// The factory is called once for every new vertex encountered in the
    /// input. Without a custom factory, vertices are obtained from the
    /// destination graph's vertex supplier.
    pub fn set_vertex_with_attributes_factory(
        &mut self,
        mut factory: impl FnMut(&str, &AttributeMap) -> V + 'static,
    ) {
        self.vertex_factory = Some(Box::new(move |id, attributes| Ok(factory(id, attributes))));
    }
    /// Sets the GraphML attribute name from which edge weights are read.
    pub fn set_edge_weight_attribute_name(&mut self, name: impl Into<String>) {
        self.edge_weight_attribute = name.into();
    }
    /// Sets a factory which creates edges from their GraphML attributes.
    ///
    /// Without a custom factory, edges are obtained from the destination
    /// graph's edge supplier.
    pub fn set_edge_with_attributes_factory(
        &mut self,
        factory: impl FnMut(&AttributeMap) -> E + 'static,
    ) {
        self.edge_factory = Some(Box::new(factory));
    }
    /// Sets the schema-validation option.
    ///
    /// The streaming Rust parser currently records this compatibility option
    /// but does not perform XML Schema validation.
    pub fn set_schema_validation(&mut self, enabled: bool) {
        self.schema_validation = enabled;
    }
    /// Returns the configured schema-validation option.
    pub const fn schema_validation(&self) -> bool {
        self.schema_validation
    }
    /// Sets the consumer notified for every graph attribute.
    pub fn set_graph_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&str, &Attribute) -> Result<()> + 'static,
    ) {
        self.graph_attribute_consumer = Some(Box::new(consumer));
    }
    /// Sets the consumer notified for every vertex attribute.
    ///
    /// The consumer also receives the GraphML vertex identifier under the
    /// attribute name `ID`.
    pub fn set_vertex_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&V, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.vertex_attribute_consumer = Some(Box::new(consumer));
    }
    /// Sets the consumer notified for every edge attribute.
    ///
    /// When present, the GraphML edge identifier is also reported under the
    /// attribute name `ID`.
    pub fn set_edge_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&E, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.edge_attribute_consumer = Some(Box::new(consumer));
    }
}

impl<V, E> Default for GraphMlImporter<V, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphImporter<G> for GraphMlImporter<V, E>
where
    G: GraphMut<Vertex = V, Edge = E>,
    V: Clone,
    E: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let document = parse(input)?;
        if let Some(consumer) = self.graph_attribute_consumer.as_mut() {
            for (name, value) in &document.graph_attributes {
                consumer(name, value)?;
            }
        }
        let mut vertices = HashMap::new();
        for node in document.nodes {
            if vertices.contains_key(&node.id) {
                return Err(IoError::Import(format!(
                    "duplicate GraphML node id {:?}",
                    node.id
                )));
            }
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(&node.id, &node.attributes).map_err(IoError::Callback)?;
                if !graph.add_vertex(vertex.clone()) {
                    return Err(IoError::Import(
                        "GraphML vertex factory produced duplicate".into(),
                    ));
                }
                vertex
            } else {
                graph.add_new_vertex()?
            };
            if let Some(consumer) = self.vertex_attribute_consumer.as_mut() {
                consumer(&vertex, "ID", &Attribute::from(node.id.clone()))?;
                for (name, value) in &node.attributes {
                    consumer(&vertex, name, value)?;
                }
            }
            vertices.insert(node.id, vertex);
        }
        for edge in document.edges {
            for id in [&edge.source, &edge.target] {
                if !vertices.contains_key(id) {
                    let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                        factory(id, &AttributeMap::new()).map_err(IoError::Callback)?
                    } else {
                        graph.new_vertex()?
                    };
                    if !graph.add_vertex(vertex.clone()) {
                        return Err(IoError::Import(
                            "GraphML vertex factory produced duplicate".into(),
                        ));
                    }
                    if let Some(consumer) = self.vertex_attribute_consumer.as_mut() {
                        consumer(&vertex, "ID", &Attribute::from(id.clone()))?;
                    }
                    vertices.insert(id.clone(), vertex);
                }
            }
            let (from, to) = (&vertices[&edge.source], &vertices[&edge.target]);
            let inserted = if let Some(factory) = self.edge_factory.as_mut() {
                let value = factory(&edge.attributes);
                graph
                    .add_edge_with(from, to, value.clone())?
                    .then_some(value)
            } else {
                graph.add_edge(from, to)?
            }
            .ok_or_else(|| IoError::Import("graph rejected GraphML edge".into()))?;
            if graph.graph_type().weighted
                && let Some(weight) = edge.attributes.get(&self.edge_weight_attribute)
            {
                graph.set_edge_weight(
                    &inserted,
                    weight
                        .value()
                        .parse()
                        .map_err(|_| IoError::Import("invalid GraphML edge weight".into()))?,
                )?;
            }
            if let Some(consumer) = self.edge_attribute_consumer.as_mut() {
                if let Some(id) = &edge.id {
                    consumer(&inserted, "ID", &Attribute::from(id.clone()))?;
                }
                for (name, value) in &edge.attributes {
                    consumer(&inserted, name, value)?;
                }
            }
        }
        Ok(())
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Exports a graph in GraphML format.
///
/// Vertices receive integer identifiers by default. Use
/// [`GraphMlExporter::with_vertex_id_provider`] when stable or domain-specific
/// identifiers are required. Attributes must be registered before they can be
/// emitted by an attribute provider.
pub struct GraphMlExporter<V, E> {
    base: BaseExporter<V, E>,
    export_edge_weights: bool,
    export_vertex_labels: bool,
    export_edge_labels: bool,
    vertex_label_attribute: String,
    edge_label_attribute: String,
    edge_weight_attribute: String,
    registered_attributes: IndexMap<String, RegisteredAttribute>,
    next_attribute: usize,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The kind of graph component to which a registered GraphML attribute applies.
pub enum AttributeCategory {
    /// A graph attribute.
    Graph,
    /// A node (vertex) attribute.
    Node,
    /// An edge attribute.
    Edge,
    /// An attribute valid for graphs, nodes, and edges.
    All,
}
impl AttributeCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::Node => "node",
            Self::Edge => "edge",
            Self::All => "all",
        }
    }
}
#[derive(Debug, Clone)]
struct RegisteredAttribute {
    key: String,
    category: AttributeCategory,
    kind: AttributeType,
    default: Option<String>,
}

impl<V, E> GraphMlExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    /// Creates an exporter which assigns consecutive integer vertex identifiers.
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |vertex| ids.id(vertex)),
            export_edge_weights: false,
            export_vertex_labels: false,
            export_edge_labels: false,
            vertex_label_attribute: "VertexLabel".into(),
            edge_label_attribute: "EdgeLabel".into(),
            edge_weight_attribute: "weight".into(),
            registered_attributes: IndexMap::new(),
            next_attribute: 0,
        }
    }
    /// Creates an exporter using `provider` to generate vertex identifiers.
    pub fn with_vertex_id_provider(provider: impl FnMut(&V) -> String + 'static) -> Self {
        let mut result = Self::new();
        result.base.set_vertex_id_provider(provider);
        result
    }
    /// Sets the provider used to generate optional edge identifiers.
    pub fn set_edge_id_provider(&mut self, provider: impl FnMut(&E) -> String + 'static) {
        self.base.set_edge_id_provider(provider);
    }
    /// Sets the provider of graph attributes.
    pub fn set_graph_attribute_provider(&mut self, provider: impl Fn() -> AttributeMap + 'static) {
        self.base.set_graph_attribute_provider(provider);
    }
    /// Sets the provider of vertex attributes.
    pub fn set_vertex_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.base.set_vertex_attribute_provider(provider);
    }
    /// Sets the provider of edge attributes.
    pub fn set_edge_attribute_provider(&mut self, provider: impl Fn(&E) -> AttributeMap + 'static) {
        self.base.set_edge_attribute_provider(provider);
    }
    /// Sets whether edge weights are exported.
    pub fn set_export_edge_weights(&mut self, enabled: bool) {
        self.export_edge_weights = enabled;
    }
    /// Sets whether vertex labels are exported.
    ///
    /// Labels are read from the vertex attribute provider using the configured
    /// vertex-label attribute name. If no such attribute is supplied, the
    /// vertex identifier is used.
    pub fn set_export_vertex_labels(&mut self, enabled: bool) {
        self.export_vertex_labels = enabled;
    }
    /// Sets whether edge labels are exported.
    ///
    /// Labels are read from the edge attribute provider using the configured
    /// edge-label attribute name.
    pub fn set_export_edge_labels(&mut self, enabled: bool) {
        self.export_edge_labels = enabled;
    }
    /// Sets the attribute name used for vertex labels.
    ///
    /// Returns an error if the name is already registered as a regular
    /// GraphML attribute.
    pub fn set_vertex_label_attribute_name(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        self.ensure_not_registered(&name)?;
        self.vertex_label_attribute = name;
        Ok(())
    }
    /// Sets the attribute name used for edge labels.
    ///
    /// Returns an error if the name is already registered as a regular
    /// GraphML attribute.
    pub fn set_edge_label_attribute_name(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        self.ensure_not_registered(&name)?;
        self.edge_label_attribute = name;
        Ok(())
    }
    /// Sets the attribute name used for edge weights.
    ///
    /// Returns an error if the name is already registered as a regular
    /// GraphML attribute.
    pub fn set_edge_weight_attribute_name(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        self.ensure_not_registered(&name)?;
        self.edge_weight_attribute = name;
        Ok(())
    }
    fn ensure_not_registered(&self, name: &str) -> Result<()> {
        if self.registered_attributes.contains_key(name) {
            Err(IoError::Export(format!(
                "GraphML attribute {name:?} is already registered"
            )))
        } else {
            Ok(())
        }
    }
    fn reserved(&self, name: &str) -> bool {
        name == self.vertex_label_attribute
            || name == self.edge_label_attribute
            || name == self.edge_weight_attribute
    }
    /// Registers an attribute for export.
    ///
    /// `category` controls where the attribute is valid, `kind` is written as
    /// its GraphML `attr.type`, and `default` suppresses values equal to that
    /// default. Registering a reserved label or weight name returns an error.
    pub fn register_attribute(
        &mut self,
        name: impl Into<String>,
        category: AttributeCategory,
        kind: AttributeType,
        default: Option<String>,
    ) -> Result<()> {
        let name = name.into();
        if self.reserved(&name) {
            return Err(IoError::Export(format!(
                "reserved GraphML attribute {name:?}"
            )));
        }
        let key = format!("key{}", self.next_attribute);
        self.next_attribute += 1;
        self.registered_attributes.insert(
            name,
            RegisteredAttribute {
                key,
                category,
                kind,
                default,
            },
        );
        Ok(())
    }
    /// Unregisters an exported attribute.
    ///
    /// Removing a name which was not registered is a no-op. Reserved label and
    /// weight names cannot be unregistered.
    pub fn unregister_attribute(&mut self, name: &str) -> Result<()> {
        if self.reserved(name) {
            return Err(IoError::Export(format!(
                "reserved GraphML attribute {name:?}"
            )));
        }
        self.registered_attributes.shift_remove(name);
        Ok(())
    }
}
impl<V, E> Default for GraphMlExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for GraphMlExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        writeln!(output, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(
            output,
            "<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\" xsi:schemaLocation=\"http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
        )?;
        if self.export_vertex_labels {
            write_key(
                &mut output,
                "vertex_label_key",
                "node",
                &self.vertex_label_attribute,
                AttributeType::String,
                None,
            )?;
        }
        if self.export_edge_labels {
            write_key(
                &mut output,
                "edge_label_key",
                "edge",
                &self.edge_label_attribute,
                AttributeType::String,
                None,
            )?;
        }
        if self.export_edge_weights {
            write_key(
                &mut output,
                "edge_weight_key",
                "edge",
                &self.edge_weight_attribute,
                AttributeType::Double,
                Some("1.0"),
            )?;
        }
        for (name, details) in &self.registered_attributes {
            write_key(
                &mut output,
                &details.key,
                details.category.as_str(),
                name,
                details.kind,
                details.default.as_deref(),
            )?;
        }
        writeln!(
            output,
            "  <graph edgedefault=\"{}\">",
            if graph.graph_type().directed {
                "directed"
            } else {
                "undirected"
            }
        )?;
        let graph_attributes = self.base.graph_attributes().unwrap_or_default();
        let mut graph_data = Vec::new();
        collect_registered(
            &mut graph_data,
            &graph_attributes,
            &self.registered_attributes,
            AttributeCategory::Graph,
        );
        for (key, value) in graph_data {
            writeln!(
                output,
                "    <data key=\"{}\">{}</data>",
                escape_xml(&key),
                escape_xml(&value)
            )?;
        }
        let mut ids = IndexMap::new();
        for vertex in graph.vertices() {
            let id = self.base.vertex_id(vertex)?;
            ids.insert(vertex.clone(), id.clone());
            let attrs = self.base.vertex_attributes(vertex).unwrap_or_default();
            let mut data = Vec::new();
            if self.export_vertex_labels {
                data.push((
                    "vertex_label_key".to_owned(),
                    attrs
                        .get(&self.vertex_label_attribute)
                        .map_or_else(|| id.to_string(), |a| a.value().to_owned()),
                ));
            }
            collect_registered(
                &mut data,
                &attrs,
                &self.registered_attributes,
                AttributeCategory::Node,
            );
            write_component(
                &mut output,
                "node",
                &format!("id=\"{}\"", escape_xml(&id)),
                &data,
            )?;
        }
        for edge in graph.edges() {
            let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
            let attrs = self.base.edge_attributes(edge).unwrap_or_default();
            let mut data = Vec::new();
            if self.export_edge_labels {
                data.push((
                    "edge_label_key".to_owned(),
                    attrs
                        .get(&self.edge_label_attribute)
                        .map_or_else(String::new, |a| a.value().to_owned()),
                ));
            }
            if self.export_edge_weights {
                let weight = crate::exporter::edge_weight(graph, edge)?;
                if weight != 1.0 {
                    data.push(("edge_weight_key".to_owned(), format_f64(weight)));
                }
            }
            collect_registered(
                &mut data,
                &attrs,
                &self.registered_attributes,
                AttributeCategory::Edge,
            );
            let edge_id = self
                .base
                .edge_id(edge)
                .map(|id| format!("id=\"{}\" ", escape_xml(&id)))
                .unwrap_or_default();
            write_component(
                &mut output,
                "edge",
                &format!(
                    "{edge_id}source=\"{}\" target=\"{}\"",
                    escape_xml(
                        ids.get(source)
                            .ok_or_else(crate::exporter::missing_endpoint)?
                    ),
                    escape_xml(
                        ids.get(target)
                            .ok_or_else(crate::exporter::missing_endpoint)?
                    )
                ),
                &data,
            )?;
        }
        writeln!(output, "  </graph>\n</graphml>")?;
        Ok(())
    }
}

fn write_key<W: Write>(
    output: &mut W,
    id: &str,
    category: &str,
    name: &str,
    kind: AttributeType,
    default: Option<&str>,
) -> Result<()> {
    let attrs = format!(
        "id=\"{}\" for=\"{}\" attr.name=\"{}\" attr.type=\"{}\"",
        escape_xml(id),
        category,
        escape_xml(name),
        kind
    );
    if let Some(default) = default {
        writeln!(
            output,
            "  <key {attrs}><default>{}</default></key>",
            escape_xml(default)
        )?;
    } else {
        writeln!(output, "  <key {attrs}/>")?;
    }
    Ok(())
}

fn collect_registered(
    data: &mut Vec<(String, String)>,
    attrs: &AttributeMap,
    registered: &IndexMap<String, RegisteredAttribute>,
    category: AttributeCategory,
) {
    for (name, details) in registered {
        if !matches!(details.category, AttributeCategory::All) && details.category != category {
            continue;
        }
        if let Some(value) = attrs.get(name)
            && details.default.as_deref() != Some(value.value())
        {
            data.push((details.key.clone(), value.value().to_owned()));
        }
    }
}

fn write_component<W: Write>(
    output: &mut W,
    element: &str,
    attributes: &str,
    data: &[(String, String)],
) -> Result<()> {
    if data.is_empty() {
        writeln!(output, "    <{element} {attributes}/>")?;
    } else {
        writeln!(output, "    <{element} {attributes}>")?;
        for (key, value) in data {
            writeln!(
                output,
                "      <data key=\"{}\">{}</data>",
                escape_xml(key),
                escape_xml(value)
            )?;
        }
        writeln!(output, "    </{element}>")?;
    }
    Ok(())
}

/// An event-driven GraphML edge: `(source_id, target_id, weight)`.
pub type GraphMlEdge = (String, String, Option<f64>);

/// Imports GraphML as a stream of vertices, edges, and attributes.
///
/// Vertex values are the identifiers read from the file. Edges contain the
/// source identifier, target identifier, and optional weight. Attribute events
/// retain their declared GraphML types.
pub struct GraphMlEventDrivenImporter {
    edge_weight_attribute: String,
    schema_validation: bool,
}
impl GraphMlEventDrivenImporter {
    /// Creates an event-driven importer with the default edge-weight attribute name.
    pub fn new() -> Self {
        Self {
            edge_weight_attribute: "weight".into(),
            schema_validation: true,
        }
    }
    /// Sets the GraphML attribute name from which edge weights are read.
    pub fn set_edge_weight_attribute_name(&mut self, name: impl Into<String>) {
        self.edge_weight_attribute = name.into();
    }
    /// Sets the schema-validation option.
    ///
    /// The streaming Rust parser currently records this compatibility option
    /// but does not perform XML Schema validation.
    pub fn set_schema_validation(&mut self, enabled: bool) {
        self.schema_validation = enabled;
    }
    /// Returns the configured schema-validation option.
    pub const fn schema_validation(&self) -> bool {
        self.schema_validation
    }
}
impl Default for GraphMlEventDrivenImporter {
    fn default() -> Self {
        Self::new()
    }
}
impl EventDrivenImporter for GraphMlEventDrivenImporter {
    type Vertex = String;
    type Edge = GraphMlEdge;
    fn import_events<R, F>(&mut self, input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        emit(ImportEvent::Start)?;
        let document = parse(input)?;
        for (name, value) in document.graph_attributes {
            emit(ImportEvent::GraphAttribute(name, value))?;
        }
        for node in document.nodes {
            emit(ImportEvent::Vertex(node.id.clone()))?;
            emit(ImportEvent::VertexAttribute(
                node.id.clone(),
                "ID".into(),
                Attribute::from(node.id.clone()),
            ))?;
            for (name, value) in node.attributes {
                emit(ImportEvent::VertexAttribute(node.id.clone(), name, value))?;
            }
        }
        for edge in document.edges {
            let weight = edge
                .attributes
                .get(&self.edge_weight_attribute)
                .and_then(|a| a.value().parse().ok());
            let value = (edge.source, edge.target, weight);
            emit(ImportEvent::Edge(value.clone()))?;
            if let Some(id) = edge.id {
                emit(ImportEvent::EdgeAttribute(
                    value.clone(),
                    "ID".into(),
                    Attribute::from(id),
                ))?;
            }
            for (name, attribute) in edge.attributes {
                emit(ImportEvent::EdgeAttribute(value.clone(), name, attribute))?;
            }
        }
        emit(ImportEvent::End)
    }
}

/// Compatibility name for the simple GraphML graph importer.
pub type SimpleGraphMlImporter<V, E = DefaultEdge> = GraphMlImporter<V, E>;
/// Compatibility name for the simple event-driven GraphML importer.
pub type SimpleGraphMlEventDrivenImporter = GraphMlEventDrivenImporter;

/// Imports a GraphML document as an event stream containing only an edge list.
///
/// Vertices are numbered from zero in the order in which their identifiers are
/// first encountered by an edge. Isolated vertices and all attribute events
/// are omitted; an edge's configured weight attribute is retained.
pub struct SimpleGraphMlEdgeListImporter {
    edge_weight_attribute: String,
    schema_validation: bool,
}
impl SimpleGraphMlEdgeListImporter {
    /// Creates an edge-list importer with the default edge-weight attribute name.
    pub fn new() -> Self {
        Self {
            edge_weight_attribute: "weight".into(),
            schema_validation: true,
        }
    }
    /// Sets the GraphML attribute name from which edge weights are read.
    pub fn set_edge_weight_attribute_name(&mut self, name: impl Into<String>) {
        self.edge_weight_attribute = name.into();
    }
    /// Sets the schema-validation option.
    ///
    /// The streaming Rust parser currently records this compatibility option
    /// but does not perform XML Schema validation.
    pub fn set_schema_validation(&mut self, enabled: bool) {
        self.schema_validation = enabled;
    }
}
impl Default for SimpleGraphMlEdgeListImporter {
    fn default() -> Self {
        Self::new()
    }
}
impl EventDrivenImporter for SimpleGraphMlEdgeListImporter {
    type Vertex = usize;
    type Edge = (usize, usize, Option<f64>);
    fn import_events<R, F>(&mut self, input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        emit(ImportEvent::Start)?;
        let document = parse(input)?;
        let mut ids = HashMap::<String, usize>::new();
        for edge in document.edges {
            let next = ids.len();
            let source = *ids.entry(edge.source).or_insert(next);
            let next = ids.len();
            let target = *ids.entry(edge.target).or_insert(next);
            let weight = edge
                .attributes
                .get(&self.edge_weight_attribute)
                .and_then(|a| a.value().parse().ok());
            emit(ImportEvent::Edge((source, target, weight)))?;
        }
        emit(ImportEvent::End)
    }
}
