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

//! Basic GEXF 1.2 import and export support.
//!
//! GEXF (Graph Exchange XML Format) is an XML graph format used by Gephi.
//! This module supports static graphs, typed node and edge attributes, labels,
//! weights, edge types, and document metadata.

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{
    Attribute, AttributeMap, AttributeType, BaseExporter, EventDrivenImporter, GraphExporter,
    GraphImporter, ImportEvent, IntegerIdProvider, IoError, Result,
};
use indexmap::IndexMap;
use jgrapht::{DefaultEdge, Graph, GraphMut};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{BufReader, Read, Write};

fn attr<R: Read>(
    reader: &Reader<BufReader<R>>,
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>> {
    for attribute in element.attributes() {
        let attribute = attribute
            .map_err(|error| IoError::Import(format!("invalid GEXF attribute: {error}")))?;
        if attribute.key.as_ref() == name {
            return Ok(Some(
                attribute
                    .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                    .map_err(|error| {
                        IoError::Import(format!("invalid GEXF attribute value: {error}"))
                    })?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

#[derive(Debug)]
struct Node {
    id: String,
    attributes: AttributeMap,
}

#[derive(Debug)]
struct Edge {
    id: Option<String>,
    source: String,
    target: String,
    weight: Option<f64>,
    attributes: AttributeMap,
}

#[derive(Debug)]
enum Item {
    Node(Node),
    Edge(Edge),
}

#[derive(Debug, Clone)]
struct AttributeDefinition {
    name: String,
    kind: AttributeType,
}

fn gexf_type(value: &str) -> AttributeType {
    match value {
        "boolean" => AttributeType::Boolean,
        "integer" | "short" | "byte" => AttributeType::Int,
        "long" | "biginteger" => AttributeType::Long,
        "float" => AttributeType::Float,
        "double" | "bigdecimal" => AttributeType::Double,
        _ => AttributeType::String,
    }
}

fn parse<R: Read>(input: R) -> Result<Vec<Item>> {
    let mut reader = Reader::from_reader(BufReader::new(input));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut items = Vec::new();
    let mut node_definitions = HashMap::new();
    let mut edge_definitions = HashMap::new();
    let mut definition_class = None::<String>;
    let mut current_node = None::<Node>;
    let mut current_edge = None::<Edge>;
    let mut nodes_depth = 0usize;
    let mut edges_depth = 0usize;
    let mut ignored_depth = 0usize;
    let mut root_seen = false;
    let mut graph_seen = false;
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| IoError::Import(format!("GEXF XML error: {error}")))?
        {
            Event::Start(element) => {
                if ignored_depth > 0 {
                    ignored_depth += 1;
                    buffer.clear();
                    continue;
                }
                match element.local_name().as_ref() {
                    b"gexf" => root_seen = true,
                    b"graph" => graph_seen = true,
                    b"nodes" if current_node.is_some() => ignored_depth = 1,
                    b"nodes" => nodes_depth += 1,
                    b"edges" if current_node.is_some() => ignored_depth = 1,
                    b"edges" => edges_depth += 1,
                    b"attributes" => definition_class = attr(&reader, &element, b"class")?,
                    b"attribute" => add_definition(
                        &reader,
                        &element,
                        definition_class.as_deref(),
                        &mut node_definitions,
                        &mut edge_definitions,
                    )?,
                    b"node" => {
                        if nodes_depth == 0 {
                            return Err(IoError::Import("GEXF node must be inside nodes".into()));
                        }
                        current_node = Some(read_node(&reader, &element)?);
                    }
                    b"edge" => {
                        if edges_depth == 0 {
                            return Err(IoError::Import("GEXF edge must be inside edges".into()));
                        }
                        current_edge = Some(read_edge(&reader, &element)?);
                    }
                    b"attvalue" => add_attvalue(
                        &reader,
                        &element,
                        current_node.as_mut(),
                        current_edge.as_mut(),
                        &node_definitions,
                        &edge_definitions,
                    )?,
                    _ => {}
                }
            }
            Event::Empty(element) => {
                if ignored_depth > 0 {
                    buffer.clear();
                    continue;
                }
                match element.local_name().as_ref() {
                    b"node" => {
                        if nodes_depth == 0 {
                            return Err(IoError::Import("GEXF node must be inside nodes".into()));
                        }
                        items.push(Item::Node(read_node(&reader, &element)?));
                    }
                    b"edge" => {
                        if edges_depth == 0 {
                            return Err(IoError::Import("GEXF edge must be inside edges".into()));
                        }
                        items.push(Item::Edge(read_edge(&reader, &element)?));
                    }
                    b"attribute" => add_definition(
                        &reader,
                        &element,
                        definition_class.as_deref(),
                        &mut node_definitions,
                        &mut edge_definitions,
                    )?,
                    b"attvalue" => add_attvalue(
                        &reader,
                        &element,
                        current_node.as_mut(),
                        current_edge.as_mut(),
                        &node_definitions,
                        &edge_definitions,
                    )?,
                    _ => {}
                }
            }
            Event::End(element) => {
                if ignored_depth > 0 {
                    ignored_depth -= 1;
                    buffer.clear();
                    continue;
                }
                match element.local_name().as_ref() {
                    b"node" => {
                        if let Some(node) = current_node.take() {
                            items.push(Item::Node(node));
                        }
                    }
                    b"edge" => {
                        if let Some(edge) = current_edge.take() {
                            items.push(Item::Edge(edge));
                        }
                    }
                    b"nodes" => nodes_depth = nodes_depth.saturating_sub(1),
                    b"edges" => edges_depth = edges_depth.saturating_sub(1),
                    b"attributes" => definition_class = None,
                    _ => {}
                }
            }
            Event::Eof => break,
            Event::DocType(_) => {
                return Err(IoError::Import(
                    "GEXF document types are not supported".into(),
                ));
            }
            _ => {}
        }
        buffer.clear();
    }
    if !root_seen || !graph_seen {
        return Err(IoError::Import("input is not a GEXF graph".into()));
    }
    Ok(items)
}

fn read_node<R: Read>(reader: &Reader<BufReader<R>>, element: &BytesStart<'_>) -> Result<Node> {
    let id = attr(reader, element, b"id")?
        .ok_or_else(|| IoError::Import("GEXF node is missing id".into()))?;
    let mut attributes = AttributeMap::new();
    if let Some(label) = attr(reader, element, b"label")? {
        attributes.insert("label".into(), Attribute::from(label));
    }
    Ok(Node { id, attributes })
}

fn read_edge<R: Read>(reader: &Reader<BufReader<R>>, element: &BytesStart<'_>) -> Result<Edge> {
    Ok(Edge {
        id: attr(reader, element, b"id")?,
        source: attr(reader, element, b"source")?
            .ok_or_else(|| IoError::Import("GEXF edge is missing source".into()))?,
        target: attr(reader, element, b"target")?
            .ok_or_else(|| IoError::Import("GEXF edge is missing target".into()))?,
        weight: attr(reader, element, b"weight")?
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| IoError::Import("invalid GEXF edge weight".into()))?,
        attributes: AttributeMap::new(),
    })
}

fn add_definition<R: Read>(
    reader: &Reader<BufReader<R>>,
    element: &BytesStart<'_>,
    class: Option<&str>,
    nodes: &mut HashMap<String, AttributeDefinition>,
    edges: &mut HashMap<String, AttributeDefinition>,
) -> Result<()> {
    let Some(class) = class else {
        return Ok(());
    };
    let id = attr(reader, element, b"id")?
        .ok_or_else(|| IoError::Import("GEXF attribute is missing id".into()))?;
    let name = attr(reader, element, b"title")?.unwrap_or_else(|| id.clone());
    let kind = gexf_type(
        attr(reader, element, b"type")?
            .as_deref()
            .unwrap_or("string"),
    );
    let definition = AttributeDefinition { name, kind };
    if class == "node" {
        nodes.insert(id, definition);
    } else if class == "edge" {
        edges.insert(id, definition);
    }
    Ok(())
}

fn add_attvalue<R: Read>(
    reader: &Reader<BufReader<R>>,
    element: &BytesStart<'_>,
    node: Option<&mut Node>,
    edge: Option<&mut Edge>,
    node_defs: &HashMap<String, AttributeDefinition>,
    edge_defs: &HashMap<String, AttributeDefinition>,
) -> Result<()> {
    let id = attr(reader, element, b"for")?
        .ok_or_else(|| IoError::Import("GEXF attvalue is missing for".into()))?;
    let value = attr(reader, element, b"value")?
        .ok_or_else(|| IoError::Import("GEXF attvalue is missing value".into()))?;
    if let Some(node) = node
        && let Some(def) = node_defs.get(&id)
    {
        node.attributes
            .insert(def.name.clone(), Attribute::new(value, def.kind));
    } else if let Some(edge) = edge
        && let Some(def) = edge_defs.get(&id)
    {
        edge.attributes
            .insert(def.name.clone(), Attribute::new(value, def.kind));
    }
    Ok(())
}

type VertexFactory<V> = Box<dyn FnMut(&str) -> V>;
type VertexAttributeConsumer<V> = Box<dyn FnMut(&V, &str, &Attribute) -> Result<()>>;
type EdgeAttributeConsumer<E> = Box<dyn FnMut(&E, &str, &Attribute) -> Result<()>>;

/// Imports a graph from a GEXF data source.
///
/// The destination graph must support any loops and parallel edges in the
/// input. GEXF `defaultedgetype` and per-edge `type` values do not change the
/// destination graph's directionality. Edge weights are applied only when the
/// destination is weighted. Vertex identifiers are reported under `ID`.
pub struct GexfImporter<V, E = DefaultEdge> {
    vertex_factory: Option<VertexFactory<V>>,
    vertex_attribute_consumer: Option<VertexAttributeConsumer<V>>,
    edge_attribute_consumer: Option<EdgeAttributeConsumer<E>>,
    schema_validation: bool,
}

impl<V, E> GexfImporter<V, E> {
    /// Creates an importer which uses the destination graph's suppliers.
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
            vertex_attribute_consumer: None,
            edge_attribute_consumer: None,
            schema_validation: true,
        }
    }
    /// Sets a factory which creates vertices from GEXF identifiers.
    ///
    /// The factory may be called for an endpoint before its node declaration
    /// when edges precede nodes in the document.
    pub fn set_vertex_factory(&mut self, factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(factory));
    }
    /// Sets the consumer notified for every vertex attribute.
    ///
    /// Node identifiers are additionally reported under the key `ID`.
    pub fn set_vertex_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&V, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.vertex_attribute_consumer = Some(Box::new(consumer));
    }
    /// Sets the consumer notified for every edge attribute.
    ///
    /// Edge identifiers, sources, and targets are additionally reported under
    /// `ID`, `source`, and `target`, respectively.
    pub fn set_edge_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&E, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.edge_attribute_consumer = Some(Box::new(consumer));
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
impl<V, E> Default for GexfImporter<V, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphImporter<G> for GexfImporter<V, E>
where
    G: GraphMut<Vertex = V, Edge = E>,
    V: Clone,
    E: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let items = parse(input)?;
        let mut vertices = HashMap::new();
        let mut declared = std::collections::HashSet::new();
        for item in items {
            match item {
                Item::Node(node) => {
                    if !declared.insert(node.id.clone()) {
                        return Err(IoError::Import(format!(
                            "duplicate GEXF node id {:?}",
                            node.id
                        )));
                    }
                    let vertex =
                        map_vertex(graph, &mut vertices, &node.id, self.vertex_factory.as_mut())?;
                    if let Some(consumer) = self.vertex_attribute_consumer.as_mut() {
                        consumer(&vertex, "ID", &Attribute::from(node.id))?;
                        for (name, value) in &node.attributes {
                            consumer(&vertex, name, value)?;
                        }
                    }
                }
                Item::Edge(edge) => {
                    let source = map_vertex(
                        graph,
                        &mut vertices,
                        &edge.source,
                        self.vertex_factory.as_mut(),
                    )?;
                    let target = map_vertex(
                        graph,
                        &mut vertices,
                        &edge.target,
                        self.vertex_factory.as_mut(),
                    )?;
                    let inserted = graph
                        .add_edge(&source, &target)?
                        .ok_or_else(|| IoError::Import("graph rejected GEXF edge".into()))?;
                    if graph.graph_type().weighted
                        && let Some(weight) = edge.weight
                    {
                        graph.set_edge_weight(&inserted, weight)?;
                    }
                    if let Some(consumer) = self.edge_attribute_consumer.as_mut() {
                        if let Some(id) = edge.id {
                            consumer(&inserted, "ID", &Attribute::from(id))?;
                        }
                        consumer(&inserted, "source", &Attribute::from(edge.source))?;
                        consumer(&inserted, "target", &Attribute::from(edge.target))?;
                        for (name, value) in &edge.attributes {
                            consumer(&inserted, name, value)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn map_vertex<V, E, G>(
    graph: &mut G,
    vertices: &mut HashMap<String, V>,
    id: &str,
    factory: Option<&mut VertexFactory<V>>,
) -> Result<V>
where
    G: GraphMut<Vertex = V, Edge = E>,
    V: Clone,
{
    if let Some(vertex) = vertices.get(id) {
        return Ok(vertex.clone());
    }
    let vertex = if let Some(factory) = factory {
        factory(id)
    } else {
        graph.new_vertex()?
    };
    if !graph.add_vertex(vertex.clone()) {
        return Err(IoError::Import(
            "GEXF vertex factory produced duplicate".into(),
        ));
    }
    vertices.insert(id.to_owned(), vertex.clone());
    Ok(vertex)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// An event-driven GEXF edge: `(source_id, target_id, weight)`.
pub type GexfEdge = (String, String, Option<f64>);

/// Imports GEXF as a stream of vertices, edges, and attributes.
///
/// Vertices are represented by their GEXF identifiers. Edges contain source
/// and target identifiers plus an optional weight. Registered attribute
/// definitions determine the types attached to attribute events.
pub struct GexfEventDrivenImporter {
    schema_validation: bool,
}
impl GexfEventDrivenImporter {
    /// Creates an event-driven GEXF importer.
    pub fn new() -> Self {
        Self {
            schema_validation: true,
        }
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
impl Default for GexfEventDrivenImporter {
    fn default() -> Self {
        Self::new()
    }
}
impl EventDrivenImporter for GexfEventDrivenImporter {
    type Vertex = String;
    type Edge = GexfEdge;
    fn import_events<R, F>(&mut self, input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        emit(ImportEvent::Start)?;
        for item in parse(input)? {
            match item {
                Item::Node(node) => {
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
                Item::Edge(edge) => {
                    let value = (edge.source, edge.target, edge.weight);
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
            }
        }
        emit(ImportEvent::End)
    }
}

/// Compatibility name for the simple GEXF graph importer.
pub type SimpleGexfImporter<V, E = DefaultEdge> = GexfImporter<V, E>;
/// Compatibility name for the simple event-driven GEXF importer.
pub type SimpleGexfEventDrivenImporter = GexfEventDrivenImporter;

/// Exports a graph as GEXF 1.2 XML.
///
/// Vertices receive sequential integer identifiers and edges receive
/// sequential identifiers starting at zero. Metadata export is enabled by
/// default; labels, weights, and per-edge type values are opt-in. Custom node
/// and edge attributes must be registered before their provider values are
/// emitted.
pub struct GexfExporter<V, E> {
    base: BaseExporter<V, E>,
    parameters: std::collections::HashSet<GexfParameter>,
    creator: Option<String>,
    keywords: Option<String>,
    description: Option<String>,
    node_attributes: IndexMap<String, GexfAttributeDetails>,
    edge_attributes: IndexMap<String, GexfAttributeDetails>,
    next_node_attribute: usize,
    next_edge_attribute: usize,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Options which modify GEXF export behavior.
pub enum GexfParameter {
    /// Export graph edge weights.
    ExportEdgeWeights,
    /// Export edge labels from the edge attribute named `label`.
    ExportEdgeLabels,
    /// Export each edge's directed or undirected type.
    ExportEdgeTypes,
    /// Export creator, description, and keyword metadata.
    ExportMeta,
}
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The graph component to which a registered GEXF attribute applies.
pub enum GexfAttributeCategory {
    /// A node (vertex) attribute.
    Node,
    /// An edge attribute.
    Edge,
}
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An attribute type supported by GEXF.
pub enum GexfAttributeType {
    /// Boolean value.
    Boolean,
    /// Integer value.
    Integer,
    /// Long integer value.
    Long,
    /// Single-precision floating-point value.
    Float,
    /// Double-precision floating-point value.
    Double,
    /// String value.
    String,
    /// List of string values.
    ListString,
    /// URI value.
    AnyUri,
}
impl GexfAttributeType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Long => "long",
            Self::Float => "float",
            Self::Double => "double",
            Self::String => "string",
            Self::ListString => "liststring",
            Self::AnyUri => "anyURI",
        }
    }
}
#[derive(Debug, Clone)]
struct GexfAttributeDetails {
    id: String,
    kind: GexfAttributeType,
    default: Option<String>,
    options: Option<String>,
}

impl<V, E> GexfExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash + 'static,
{
    /// Creates a GEXF exporter with default integer identifier providers.
    ///
    /// Vertex identifiers start at one, edge identifiers start at zero, and
    /// metadata export is enabled with `The JGraphT Library` as the creator.
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        let mut edge_ids = IntegerIdProvider::with_start(0);
        let mut parameters = std::collections::HashSet::new();
        parameters.insert(GexfParameter::ExportMeta);
        let mut base = BaseExporter::new(move |vertex| ids.id(vertex));
        base.set_edge_id_provider(move |edge| edge_ids.id(edge));
        Self {
            base,
            parameters,
            creator: Some("The JGraphT Library".into()),
            keywords: None,
            description: None,
            node_attributes: IndexMap::new(),
            edge_attributes: IndexMap::new(),
            next_node_attribute: 0,
            next_edge_attribute: 0,
        }
    }
    /// Enables or disables exporting graph edge weights.
    ///
    /// This is a convenience wrapper for [`GexfParameter::ExportEdgeWeights`].
    pub fn set_export_edge_weights(&mut self, enabled: bool) {
        self.set_parameter(GexfParameter::ExportEdgeWeights, enabled);
    }
    /// Enables or disables an export option.
    pub fn set_parameter(&mut self, parameter: GexfParameter, enabled: bool) {
        if enabled {
            self.parameters.insert(parameter);
        } else {
            self.parameters.remove(&parameter);
        }
    }
    /// Returns whether an export option is enabled.
    pub fn parameter(&self, parameter: GexfParameter) -> bool {
        self.parameters.contains(&parameter)
    }
    /// Sets the creator written in the metadata section.
    ///
    /// The value is emitted only when [`GexfParameter::ExportMeta`] is enabled.
    pub fn set_creator(&mut self, value: impl Into<String>) {
        self.creator = Some(value.into());
    }
    /// Sets the keywords written in the metadata section.
    ///
    /// The value is emitted only when [`GexfParameter::ExportMeta`] is enabled.
    pub fn set_keywords(&mut self, value: impl Into<String>) {
        self.keywords = Some(value.into());
    }
    /// Sets the description written in the metadata section.
    ///
    /// The value is emitted only when [`GexfParameter::ExportMeta`] is enabled.
    pub fn set_description(&mut self, value: impl Into<String>) {
        self.description = Some(value.into());
    }
    /// Sets the provider of vertex attributes.
    ///
    /// Only attributes registered with [`Self::register_attribute`] are
    /// emitted.
    pub fn set_vertex_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.base.set_vertex_attribute_provider(provider);
    }
    /// Sets the provider of edge attributes.
    ///
    /// Only registered attributes are emitted, except that the `label`
    /// attribute supplies edge labels when [`GexfParameter::ExportEdgeLabels`]
    /// is enabled.
    pub fn set_edge_attribute_provider(&mut self, provider: impl Fn(&E) -> AttributeMap + 'static) {
        self.base.set_edge_attribute_provider(provider);
    }
    /// Registers a node or edge attribute for export.
    ///
    /// `default` suppresses values equal to the declared default, while
    /// `options` is written as the GEXF attribute's options declaration.
    /// Node names `id` and `label` are reserved. Edge names `id`, `type`,
    /// `label`, `source`, `target`, and `weight` are reserved. Name matching
    /// for this check is ASCII case-insensitive.
    ///
    /// Registering an existing name replaces its definition and assigns it a
    /// new numeric attribute identifier.
    pub fn register_attribute(
        &mut self,
        name: impl Into<String>,
        category: GexfAttributeCategory,
        kind: GexfAttributeType,
        default: Option<String>,
        options: Option<String>,
    ) -> Result<()> {
        let name = name.into();
        let lower = name.to_ascii_lowercase();
        let reserved = match category {
            GexfAttributeCategory::Node => matches!(lower.as_str(), "id" | "label"),
            GexfAttributeCategory::Edge => matches!(
                lower.as_str(),
                "id" | "type" | "label" | "source" | "target" | "weight"
            ),
        };
        if reserved {
            return Err(IoError::Export(format!("reserved GEXF attribute {name:?}")));
        }
        let (map, next) = match category {
            GexfAttributeCategory::Node => {
                (&mut self.node_attributes, &mut self.next_node_attribute)
            }
            GexfAttributeCategory::Edge => {
                (&mut self.edge_attributes, &mut self.next_edge_attribute)
            }
        };
        let id = next.to_string();
        *next += 1;
        map.insert(
            name,
            GexfAttributeDetails {
                id,
                kind,
                default,
                options,
            },
        );
        Ok(())
    }
}
impl<V, E> Default for GexfExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for GexfExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash + 'static,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        writeln!(output, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(
            output,
            "<gexf xmlns=\"http://www.gexf.net/1.2draft\" version=\"1.2\" xsi:schemaLocation=\"http://www.gexf.net/1.2draft http://www.gexf.net/1.2draft/gexf.xsd\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
        )?;
        if self.parameter(GexfParameter::ExportMeta) {
            writeln!(output, "  <meta>")?;
            if let Some(value) = &self.creator {
                writeln!(output, "    <creator>{}</creator>", escape(value))?;
            }
            if let Some(value) = &self.description {
                writeln!(output, "    <description>{}</description>", escape(value))?;
            }
            if let Some(value) = &self.keywords {
                writeln!(output, "    <keywords>{}</keywords>", escape(value))?;
            }
            writeln!(output, "  </meta>")?;
        }
        writeln!(
            output,
            "  <graph mode=\"static\" defaultedgetype=\"{}\">",
            if graph.graph_type().directed {
                "directed"
            } else {
                "undirected"
            }
        )?;
        write_attribute_declarations(&mut output, "node", &self.node_attributes)?;
        write_attribute_declarations(&mut output, "edge", &self.edge_attributes)?;
        writeln!(output, "    <nodes>")?;
        let mut ids = IndexMap::new();
        for vertex in graph.vertices() {
            let id = self.base.vertex_id(vertex)?;
            ids.insert(vertex.clone(), id.clone());
            let attributes = self.base.vertex_attributes(vertex).unwrap_or_default();
            write_gexf_component(
                &mut output,
                "node",
                &format!("id=\"{}\" label=\"{}\"", escape(&id), escape(&id)),
                &attributes,
                &self.node_attributes,
            )?;
        }
        writeln!(output, "    </nodes>\n    <edges>")?;
        for edge in graph.edges() {
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
            write!(
                output,
                "      <edge id=\"{}\" source=\"{}\" target=\"{}\"",
                escape(
                    &self.base.edge_id(edge).ok_or_else(|| IoError::Export(
                        "GEXF edge id provider is missing".into()
                    ))?
                ),
                escape(&ids[source]),
                escape(&ids[target])
            )?;
            if self.parameter(GexfParameter::ExportEdgeTypes) {
                write!(
                    output,
                    " type=\"{}\"",
                    if graph.graph_type().directed {
                        "directed"
                    } else {
                        "undirected"
                    }
                )?;
            }
            if self.parameter(GexfParameter::ExportEdgeWeights) {
                write!(
                    output,
                    " weight=\"{}\"",
                    format_f64(graph.edge_weight(edge).unwrap_or(1.0))
                )?;
            }
            let attributes = self.base.edge_attributes(edge).unwrap_or_default();
            if self.parameter(GexfParameter::ExportEdgeLabels)
                && let Some(label) = attributes.get("label")
            {
                write!(output, " label=\"{}\"", escape(label.value()))?;
            }
            if self.edge_attributes.iter().any(|(name, details)| {
                attributes
                    .get(name)
                    .is_some_and(|value| details.default.as_deref() != Some(value.value()))
            }) {
                writeln!(output, ">")?;
                write_attvalues(&mut output, &attributes, &self.edge_attributes, "        ")?;
                writeln!(output, "      </edge>")?;
            } else {
                writeln!(output, "/>")?;
            }
        }
        writeln!(output, "    </edges>\n  </graph>\n</gexf>")?;
        Ok(())
    }
}

fn write_attribute_declarations<W: Write>(
    output: &mut W,
    class: &str,
    attributes: &IndexMap<String, GexfAttributeDetails>,
) -> Result<()> {
    if attributes.is_empty() {
        return Ok(());
    }
    writeln!(output, "    <attributes class=\"{class}\">")?;
    for (name, details) in attributes {
        if details.default.is_none() && details.options.is_none() {
            writeln!(
                output,
                "      <attribute id=\"{}\" title=\"{}\" type=\"{}\"/>",
                details.id,
                escape(name),
                details.kind.as_str()
            )?;
        } else {
            writeln!(
                output,
                "      <attribute id=\"{}\" title=\"{}\" type=\"{}\">",
                details.id,
                escape(name),
                details.kind.as_str()
            )?;
            if let Some(value) = &details.default {
                writeln!(output, "        <default>{}</default>", escape(value))?;
            }
            if let Some(value) = &details.options {
                writeln!(output, "        <options>{}</options>", escape(value))?;
            }
            writeln!(output, "      </attribute>")?;
        }
    }
    writeln!(output, "    </attributes>")?;
    Ok(())
}

fn write_attvalues<W: Write>(
    output: &mut W,
    attributes: &AttributeMap,
    registered: &IndexMap<String, GexfAttributeDetails>,
    indent: &str,
) -> Result<()> {
    writeln!(output, "{indent}<attvalues>")?;
    for (name, details) in registered {
        if let Some(value) = attributes.get(name)
            && details.default.as_deref() != Some(value.value())
        {
            writeln!(
                output,
                "{indent}  <attvalue for=\"{}\" value=\"{}\"/>",
                details.id,
                escape(value.value())
            )?;
        }
    }
    writeln!(output, "{indent}</attvalues>")?;
    Ok(())
}

fn write_gexf_component<W: Write>(
    output: &mut W,
    element: &str,
    structural: &str,
    attributes: &AttributeMap,
    registered: &IndexMap<String, GexfAttributeDetails>,
) -> Result<()> {
    let has_values = registered.iter().any(|(name, details)| {
        attributes
            .get(name)
            .is_some_and(|value| details.default.as_deref() != Some(value.value()))
    });
    if has_values {
        writeln!(output, "      <{element} {structural}>")?;
        write_attvalues(output, attributes, registered, "        ")?;
        writeln!(output, "      </{element}>")?;
    } else {
        writeln!(output, "      <{element} {structural}/>")?;
    }
    Ok(())
}
