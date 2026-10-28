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

//! Graph Modelling Language (GML) import and export.
//!
//! GML is a hierarchical text format for graphs and their attributes. Nested
//! attribute structures which do not have a native [`AttributeType`] are
//! retained as textual values.

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{
    Attribute, AttributeMap, AttributeType, BaseExporter, GraphExporter, GraphImporter,
    IntegerIdProvider, IoError, Result,
};
use indexmap::IndexMap;
use jgrapht::{DefaultEdge, Graph, GraphMut};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;
use std::io::{Read, Write};
use std::marker::PhantomData;

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Value {
    Text(String),
    Number(String),
    List(Vec<(String, Value)>),
}

struct Parser<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    fn skip(&mut self) {
        loop {
            while self
                .input
                .get(self.position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.position += 1;
            }
            if self.input.get(self.position) == Some(&b'#') {
                while self
                    .input
                    .get(self.position)
                    .is_some_and(|byte| *byte != b'\n')
                {
                    self.position += 1;
                }
            } else {
                break;
            }
        }
    }

    fn identifier(&mut self) -> Result<String> {
        self.skip();
        let start = self.position;
        while self
            .input
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte >= 128)
        {
            self.position += 1;
        }
        if start == self.position {
            return Err(IoError::Import(format!(
                "GML byte {start}: expected identifier"
            )));
        }
        Ok(String::from_utf8_lossy(&self.input[start..self.position]).into_owned())
    }

    fn quoted(&mut self) -> Result<String> {
        self.position += 1;
        let mut output = String::new();
        while let Some(byte) = self.input.get(self.position).copied() {
            self.position += 1;
            match byte {
                b'"' => return Ok(output),
                b'\\' if self.input.get(self.position) == Some(&b'"') => {
                    self.position += 1;
                    output.push('"');
                }
                _ => output.push(byte as char),
            }
        }
        Err(IoError::Import("unterminated GML string".into()))
    }

    fn value(&mut self) -> Result<Value> {
        self.skip();
        match self.input.get(self.position) {
            Some(b'"') => self.quoted().map(Value::Text),
            Some(b'[') => {
                self.position += 1;
                let mut pairs = Vec::new();
                loop {
                    self.skip();
                    if self.input.get(self.position) == Some(&b']') {
                        self.position += 1;
                        break;
                    }
                    if self.position >= self.input.len() {
                        return Err(IoError::Import("unterminated GML list".into()));
                    }
                    let key = self.identifier()?;
                    pairs.push((key, self.value()?));
                }
                Ok(Value::List(pairs))
            }
            Some(_) => {
                let start = self.position;
                while self
                    .input
                    .get(self.position)
                    .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'.'))
                {
                    self.position += 1;
                }
                if start == self.position {
                    return Err(IoError::Import(format!("GML byte {start}: expected value")));
                }
                Ok(Value::Number(
                    String::from_utf8_lossy(&self.input[start..self.position]).into_owned(),
                ))
            }
            None => Err(IoError::Import("unexpected end of GML input".into())),
        }
    }

    fn parse(mut self) -> Result<Vec<(String, Value)>> {
        let mut pairs = Vec::new();
        loop {
            self.skip();
            if self.position == self.input.len() {
                break;
            }
            let key = self.identifier()?;
            pairs.push((key, self.value()?));
        }
        Ok(pairs)
    }
}

fn number<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    pairs.iter().find_map(|(name, value)| {
        (name == key)
            .then_some(value)
            .and_then(|value| match value {
                Value::Number(value) => Some(value.as_str()),
                _ => None,
            })
    })
}

fn attribute(value: &Value) -> Attribute {
    match value {
        Value::Text(value) => Attribute::from(value.clone()),
        Value::Number(value) if value.parse::<i32>().is_ok() => {
            Attribute::new(value, AttributeType::Int)
        }
        Value::Number(value) if value.parse::<i64>().is_ok() => {
            Attribute::new(value, AttributeType::Long)
        }
        Value::Number(value) => Attribute::new(value, AttributeType::Double),
        Value::List(_) => Attribute::new(format_value(value), AttributeType::Unknown),
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Text(value) => format!("\"{value}\""),
        Value::Number(value) => value.clone(),
        Value::List(pairs) => format!(
            "[ {} ]",
            pairs
                .iter()
                .map(|(key, value)| format!("{key} {}", format_value(value)))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    }
}

fn attributes(pairs: &[(String, Value)], forbidden: &[&str]) -> AttributeMap {
    pairs
        .iter()
        .filter(|(key, _)| !forbidden.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), attribute(value)))
        .collect()
}

type VertexAttributeConsumer<V> = Box<dyn FnMut(&V, &str, &Attribute) -> Result<()>>;
type EdgeAttributeConsumer<E> = Box<dyn FnMut(&E, &str, &Attribute) -> Result<()>>;

/// A fallible vertex factory, called with the vertex number of the input.
type VertexFactory<V, Id> = Box<dyn FnMut(Id) -> std::result::Result<V, crate::CallbackError>>;

/// Imports a graph from Graph Modelling Language (GML).
///
/// The destination graph must support the features present in the input, such
/// as loops and parallel edges. Numeric `weight` attributes are applied when
/// the destination is weighted. Other node and edge attributes are reported to
/// the configured consumers; nested structures are represented as strings.
/// Node identifiers are also reported as an `ID` vertex attribute.
pub struct GmlImporter<V, E = DefaultEdge> {
    vertex_factory: Option<VertexFactory<V, i64>>,
    vertex_attribute_consumer: Option<VertexAttributeConsumer<V>>,
    edge_attribute_consumer: Option<EdgeAttributeConsumer<E>>,
    edge: PhantomData<E>,
}

impl<V, E> GmlImporter<V, E> {
    /// Creates an importer which uses the destination graph's suppliers.
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
            vertex_attribute_consumer: None,
            edge_attribute_consumer: None,
            edge: PhantomData,
        }
    }

    /// Sets a factory which creates vertices from numeric GML node identifiers.
    ///
    /// The importer also invokes the factory for vertices referenced by edges
    /// but not explicitly declared by a `node` block.
    pub fn set_vertex_factory(&mut self, mut factory: impl FnMut(i64) -> V + 'static) {
        self.vertex_factory = Some(Box::new(move |id| Ok(factory(id))));
    }

    /// Like [`set_vertex_factory`](Self::set_vertex_factory) for a factory that can fail, for
    /// example when it parses the identifier. Its error stops the import and is returned as
    /// [`IoError::Callback`] with the factory's error as the source.
    ///
    /// # Errors
    ///
    /// Returns an [`IoError`] if reading or writing fails, if the input is malformed, or if a user callback fails.
    pub fn try_set_vertex_factory(
        &mut self,
        factory: impl FnMut(i64) -> std::result::Result<V, crate::CallbackError> + 'static,
    ) {
        self.vertex_factory = Some(Box::new(factory));
    }

    /// Sets the consumer notified for every vertex attribute.
    ///
    /// The consumer also receives each numeric node identifier under the
    /// attribute name `ID`.
    ///
    /// # Errors
    ///
    /// Returns an [`IoError`] if reading or writing fails, if the input is malformed, or if a user callback fails.
    pub fn set_vertex_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&V, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.vertex_attribute_consumer = Some(Box::new(consumer));
    }

    /// Sets the consumer notified for every edge attribute.
    ///
    /// # Errors
    ///
    /// Returns an [`IoError`] if reading or writing fails, if the input is malformed, or if a user callback fails.
    pub fn set_edge_attribute_consumer(
        &mut self,
        consumer: impl FnMut(&E, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.edge_attribute_consumer = Some(Box::new(consumer));
    }
}

impl<V, E> Default for GmlImporter<V, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphImporter<G> for GmlImporter<V, E>
where
    G: GraphMut<Vertex = V, Edge = E>,
    V: Clone,
    E: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, mut input: R) -> Result<()> {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let document = Parser::new(&text).parse()?;
        let Some(graph_data) = document.iter().find_map(|(key, value)| {
            (key == "graph")
                .then_some(value)
                .and_then(|value| match value {
                    Value::List(value) => Some(value),
                    _ => None,
                })
        }) else {
            return Ok(());
        };
        let mut vertices = HashMap::new();
        let mut next_singleton = graph_data
            .iter()
            .filter(|(key, _)| key == "node")
            .filter_map(|(_, value)| match value {
                Value::List(node) => number(node, "id")?.parse::<i64>().ok(),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            + 1;
        for (_, value) in graph_data.iter().filter(|(key, _)| key == "node") {
            let Value::List(node) = value else { continue };
            if node
                .iter()
                .any(|(key, value)| key == "id" && !matches!(value, Value::Number(_)))
            {
                return Err(IoError::Import("invalid type for GML node id".into()));
            }
            let id = if let Some(id) = number(node, "id") {
                id.parse::<i64>()
                    .map_err(|_| IoError::Import("invalid GML node id".into()))?
            } else {
                let id = next_singleton;
                next_singleton += 1;
                id
            };
            if vertices.contains_key(&id) {
                return Err(IoError::Import(format!("duplicate GML node id {id}")));
            }
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(id).map_err(IoError::Callback)?;
                if !graph.add_vertex(vertex.clone()) {
                    return Err(IoError::Import(
                        "GML vertex factory produced duplicate".into(),
                    ));
                }
                vertex
            } else {
                graph.add_new_vertex()?
            };
            if let Some(consumer) = self.vertex_attribute_consumer.as_mut() {
                consumer(&vertex, "ID", &Attribute::from(id))?;
                for (key, value) in attributes(node, &["id"]) {
                    consumer(&vertex, &key, &value)?;
                }
            }
            vertices.insert(id, vertex);
        }
        let referenced: Vec<i64> = graph_data
            .iter()
            .filter(|(key, _)| key == "edge")
            .filter_map(|(_, value)| match value {
                Value::List(edge) => {
                    let source = number(edge, "source")?.parse::<i64>().ok()?;
                    let target = number(edge, "target")?.parse::<i64>().ok()?;
                    Some([source, target])
                }
                _ => None,
            })
            .flatten()
            .collect();
        for id in referenced {
            if vertices.contains_key(&id) {
                continue;
            }
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(id).map_err(IoError::Callback)?;
                if !graph.add_vertex(vertex.clone()) {
                    return Err(IoError::Import(
                        "GML vertex factory produced duplicate".into(),
                    ));
                }
                vertex
            } else {
                graph.add_new_vertex()?
            };
            vertices.insert(id, vertex);
        }
        for (_, value) in graph_data.iter().filter(|(key, _)| key == "edge") {
            let Value::List(edge) = value else { continue };
            for key in ["source", "target", "weight"] {
                if edge
                    .iter()
                    .any(|(name, value)| name == key && !matches!(value, Value::Number(_)))
                {
                    return Err(IoError::Import(format!("invalid type for GML edge {key}")));
                }
            }
            let Some(source) = number(edge, "source").and_then(|value| value.parse::<i64>().ok())
            else {
                continue;
            };
            let Some(target) = number(edge, "target").and_then(|value| value.parse::<i64>().ok())
            else {
                continue;
            };
            let from = vertices
                .get(&source)
                .ok_or_else(|| IoError::Import(format!("GML node {source} does not exist")))?;
            let to = vertices
                .get(&target)
                .ok_or_else(|| IoError::Import(format!("GML node {target} does not exist")))?;
            let inserted = graph
                .add_edge(from, to)?
                .ok_or_else(|| IoError::Import("graph rejected GML edge".into()))?;
            if graph.graph_type().weighted
                && let Some(weight) = number(edge, "weight")
            {
                graph.set_edge_weight(
                    &inserted,
                    weight
                        .parse()
                        .map_err(|_| IoError::Import("invalid GML edge weight".into()))?,
                )?;
            }
            if let Some(consumer) = self.edge_attribute_consumer.as_mut() {
                for (key, value) in attributes(edge, &["id", "source", "target"]) {
                    consumer(&inserted, &key, &value)?;
                }
            }
        }
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Options which modify GML export behavior.
pub enum GmlParameter {
    /// Export edge labels from the edge attribute named `label`.
    ExportEdgeLabels,
    /// Export weights of weighted-graph edges under the key `weight`.
    ExportEdgeWeights,
    /// Export all custom edge attributes supplied by the edge provider.
    ///
    /// The keys `id`, `source`, and `target` are reserved. Special label and
    /// weight handling takes precedence when enabled.
    ExportCustomEdgeAttributes,
    /// Export vertex labels from the vertex attribute named `label`.
    ExportVertexLabels,
    /// Export all custom vertex attributes supplied by the vertex provider.
    ///
    /// The key `id` is reserved, and special label handling takes precedence
    /// when enabled.
    ExportCustomVertexAttributes,
    /// Export a `graphics` attribute block for each vertex.
    ExportCustomVertexGraphicsAttributes,
    /// Escape string values using Java string-literal rules.
    EscapeStringsAsJava,
}

type GraphicsAttributeProvider<V> = Box<dyn Fn(&V) -> AttributeMap>;

/// Exports a graph in Graph Modelling Language (GML).
///
/// Vertices receive sequential integer identifiers. Labels, weights, custom
/// attributes, and vertex graphics blocks are opt-in through [`GmlParameter`].
pub struct GmlExporter<V, E> {
    base: BaseExporter<V, E>,
    parameters: HashSet<GmlParameter>,
    vertex_graphics_attribute_provider: Option<GraphicsAttributeProvider<V>>,
}

impl<V, E> GmlExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    /// Creates an exporter with sequential integer vertex identifiers.
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |vertex| ids.id(vertex)),
            parameters: HashSet::new(),
            vertex_graphics_attribute_provider: None,
        }
    }

    /// Enables or disables edge-weight export.
    ///
    /// This is a convenience wrapper for [`GmlParameter::ExportEdgeWeights`].
    pub fn set_export_edge_weights(&mut self, enabled: bool) {
        self.set_parameter(GmlParameter::ExportEdgeWeights, enabled);
    }

    /// Enables or disables an export option.
    pub fn set_parameter(&mut self, parameter: GmlParameter, enabled: bool) {
        if enabled {
            self.parameters.insert(parameter);
        } else {
            self.parameters.remove(&parameter);
        }
    }

    /// Returns whether an export option is enabled.
    pub fn is_parameter(&self, parameter: GmlParameter) -> bool {
        self.parameters.contains(&parameter)
    }

    /// Sets the provider used to generate optional edge identifiers.
    pub fn set_edge_id_provider(&mut self, provider: impl FnMut(&E) -> String + 'static) {
        self.base.set_edge_id_provider(provider);
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

    /// Sets the provider for attributes written inside each vertex's `graphics` block.
    pub fn set_vertex_graphics_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.vertex_graphics_attribute_provider = Some(Box::new(provider));
    }
}

impl<V, E> Default for GmlExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for GmlExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + Display + 'static,
    E: Clone + Eq + Hash + Display,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        writeln!(
            output,
            "Creator \"JGraphT GML Exporter\"\nVersion 1\ngraph\n["
        )?;
        writeln!(
            output,
            "\tlabel \"\"\n\tdirected {}",
            usize::from(graph.graph_type().directed)
        )?;
        let mut ids = IndexMap::new();
        for vertex in graph.vertices() {
            let vertex = vertex.borrow();
            let id = self.base.vertex_id(vertex)?;
            ids.insert(vertex.clone(), id.clone());
            writeln!(output, "\tnode\n\t[\n\t\tid {id}")?;
            let vertex_attributes = self.base.vertex_attributes(vertex).unwrap_or_default();
            if self.is_parameter(GmlParameter::ExportVertexLabels) {
                let label = vertex_attributes
                    .get("label")
                    .map_or_else(|| vertex.to_string(), |value| value.value().to_owned());
                writeln!(output, "\t\tlabel {}", self.quoted(&label))?;
            }
            if self.is_parameter(GmlParameter::ExportCustomVertexAttributes) {
                for (key, value) in &vertex_attributes {
                    if key == "id" {
                        return Err(IoError::Export("GML vertex key id is reserved".into()));
                    }
                    if key == "label" && self.is_parameter(GmlParameter::ExportVertexLabels) {
                        continue;
                    }
                    self.write_attribute(&mut output, "\t\t", key, value)?;
                }
            }
            if self.is_parameter(GmlParameter::ExportCustomVertexGraphicsAttributes)
                && let Some(provider) = &self.vertex_graphics_attribute_provider
            {
                writeln!(output, "\t\tgraphics\n\t\t[")?;
                for (key, value) in provider(vertex) {
                    if key == "id" {
                        return Err(IoError::Export("GML graphics key id is reserved".into()));
                    }
                    self.write_attribute(&mut output, "\t\t\t", &key, &value)?;
                }
                writeln!(output, "\t\t]")?;
            }
            writeln!(output, "\t]")?;
        }
        for edge in graph.edges() {
            let edge = edge.borrow();
            let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
            writeln!(output, "\tedge\n\t[")?;
            if let Some(id) = self.base.edge_id(edge) {
                writeln!(output, "\t\tid {id}")?;
            }
            writeln!(
                output,
                "\t\tsource {}\n\t\ttarget {}",
                ids.get(source)
                    .ok_or_else(crate::exporter::missing_endpoint)?,
                ids.get(target)
                    .ok_or_else(crate::exporter::missing_endpoint)?
            )?;
            let edge_attributes = self.base.edge_attributes(edge).unwrap_or_default();
            if self.is_parameter(GmlParameter::ExportEdgeLabels) {
                let label = edge_attributes.get("label").map_or_else(
                    || format!("({source} : {target})"),
                    |value| value.value().to_owned(),
                );
                self.write_attribute(&mut output, "\t\t", "label", &Attribute::from(label))?;
            }
            if self.is_parameter(GmlParameter::ExportEdgeWeights) && graph.graph_type().weighted {
                writeln!(
                    output,
                    "\t\tweight {}",
                    format_f64(crate::exporter::edge_weight(graph, edge)?)
                )?;
            }
            if self.is_parameter(GmlParameter::ExportCustomEdgeAttributes) {
                for (key, value) in &edge_attributes {
                    if matches!(key.as_str(), "id" | "source" | "target") {
                        return Err(IoError::Export(format!("GML edge key {key} is reserved")));
                    }
                    if (key == "label" && self.is_parameter(GmlParameter::ExportEdgeLabels))
                        || (key == "weight" && self.is_parameter(GmlParameter::ExportEdgeWeights))
                    {
                        continue;
                    }
                    self.write_attribute(&mut output, "\t\t", key, value)?;
                }
            }
            writeln!(output, "\t]")?;
        }
        writeln!(output, "]")?;
        Ok(())
    }
}

impl<V, E> GmlExporter<V, E> {
    fn quoted(&self, value: &str) -> String {
        if !self.parameters.contains(&GmlParameter::EscapeStringsAsJava) {
            return format!("\"{value}\"");
        }
        let mut escaped = String::new();
        for ch in value.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(ch),
            }
        }
        format!("\"{escaped}\"")
    }

    fn write_attribute<W: Write>(
        &self,
        mut output: W,
        prefix: &str,
        key: &str,
        attribute: &Attribute,
    ) -> Result<()> {
        match attribute.attribute_type() {
            AttributeType::Int | AttributeType::Long => {
                let value = attribute
                    .value()
                    .parse::<i64>()
                    .map_err(|_| IoError::Export("invalid integer GML attribute".into()))?;
                writeln!(output, "{prefix}{key} {value}")?;
            }
            AttributeType::Float | AttributeType::Double => {
                let value = attribute
                    .value()
                    .parse::<f64>()
                    .map_err(|_| IoError::Export("invalid numeric GML attribute".into()))?;
                writeln!(output, "{prefix}{key} {}", format_f64(value))?;
            }
            _ => writeln!(output, "{prefix}{key} {}", self.quoted(attribute.value()))?,
        }
        Ok(())
    }
}
