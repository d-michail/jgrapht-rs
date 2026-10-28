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

//! JGraphT's JSON graph interchange format.
//!
//! A graph is represented by a root object containing a `nodes` array and an
//! `edges` array. Node objects have an `id`; edge objects have `source` and
//! `target` identifiers. Additional members become typed attributes.

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{
    Attribute, AttributeMap, AttributeType, BaseExporter, EventDrivenImporter, GraphExporter,
    GraphImporter, ImportEvent, IntegerIdProvider, IoError, Result,
};
use indexmap::IndexMap;
use jgrapht::{Graph, GraphMut};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write};

#[derive(Debug, Clone)]
enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}

impl Value {
    fn compact(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.clone(),
            Self::String(value) => quote(value),
            Self::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Self::compact)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Object(values) => format!(
                "{{{}}}",
                values
                    .iter()
                    .map(|(key, value)| format!("{}:{}", quote(key), value.compact()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }

    fn identifier(&self) -> Result<String> {
        match self {
            Self::String(value) => Ok(value.clone()),
            Self::Number(value) if !value.contains(['.', 'e', 'E']) => value
                .parse::<i64>()
                .map(|value| value.to_string())
                .map_err(|_| IoError::Import("JSON identifier is outside the i64 range".into())),
            _ => Err(IoError::Import(
                "JSON identifier must be a string or integer".into(),
            )),
        }
    }

    fn attribute(&self) -> Attribute {
        match self {
            Self::Null => Attribute::null(),
            Self::Bool(value) => Attribute::from(*value),
            Self::String(value) => Attribute::from(value.clone()),
            Self::Number(value) => {
                if value.parse::<i32>().is_ok() {
                    Attribute::new(value, AttributeType::Int)
                } else if value.parse::<i64>().is_ok() {
                    Attribute::new(value, AttributeType::Long)
                } else {
                    Attribute::new(value, AttributeType::Double)
                }
            }
            Self::Array(_) | Self::Object(_) => {
                Attribute::new(self.compact(), AttributeType::Unknown)
            }
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            position: 0,
        }
    }

    fn parse(mut self) -> Result<Value> {
        let value = self.value()?;
        self.whitespace();
        if self.position != self.bytes.len() {
            return self.error("unexpected trailing input");
        }
        Ok(value)
    }

    fn error<T>(&self, message: &str) -> Result<T> {
        Err(IoError::Import(format!(
            "JSON byte {}: {message}",
            self.position
        )))
    }

    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> Result<()> {
        self.whitespace();
        if self.bytes.get(self.position) == Some(&expected) {
            self.position += 1;
            Ok(())
        } else {
            self.error(&format!("expected {:?}", expected as char))
        }
    }

    fn value(&mut self) -> Result<Value> {
        self.whitespace();
        match self.bytes.get(self.position).copied() {
            Some(b'"') => self.string().map(Value::String),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number().map(Value::Number),
            Some(_) => self.error("expected a JSON value"),
            None => self.error("unexpected end of input"),
        }
    }

    fn literal(&mut self, text: &[u8], value: Value) -> Result<Value> {
        if self.bytes.get(self.position..self.position + text.len()) == Some(text) {
            self.position += text.len();
            Ok(value)
        } else {
            self.error("invalid literal")
        }
    }

    fn string(&mut self) -> Result<String> {
        self.consume(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.bytes.get(self.position).copied() {
            self.position += 1;
            match byte {
                b'"' => return Ok(output),
                b'\\' => {
                    let escape = *self
                        .bytes
                        .get(self.position)
                        .ok_or_else(|| IoError::Import("JSON string ends after escape".into()))?;
                    self.position += 1;
                    match escape {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let code = self.hex4()?;
                            if (0xd800..=0xdbff).contains(&code) {
                                if self.bytes.get(self.position..self.position + 2) != Some(b"\\u")
                                {
                                    return self.error("high surrogate without low surrogate");
                                }
                                self.position += 2;
                                let low = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return self.error("invalid low surrogate");
                                }
                                let scalar = 0x10000
                                    + ((u32::from(code) - 0xd800) << 10)
                                    + (u32::from(low) - 0xdc00);
                                output.push(char::from_u32(scalar).expect("valid surrogate pair"));
                            } else {
                                output.push(char::from_u32(u32::from(code)).ok_or_else(|| {
                                    IoError::Import("invalid JSON Unicode escape".into())
                                })?);
                            }
                        }
                        _ => return self.error("invalid string escape"),
                    }
                }
                0..=31 => return self.error("control character in string"),
                _ => {
                    let start = self.position - 1;
                    let text = std::str::from_utf8(&self.bytes[start..])
                        .map_err(|_| IoError::Import("JSON is not valid UTF-8".into()))?;
                    let ch = text.chars().next().expect("non-empty UTF-8");
                    output.push(ch);
                    self.position = start + ch.len_utf8();
                }
            }
        }
        self.error("unterminated string")
    }

    fn hex4(&mut self) -> Result<u16> {
        let bytes = self
            .bytes
            .get(self.position..self.position + 4)
            .ok_or_else(|| IoError::Import("short JSON Unicode escape".into()))?;
        let text = std::str::from_utf8(bytes).expect("ASCII hex bytes");
        let value = u16::from_str_radix(text, 16)
            .map_err(|_| IoError::Import("invalid JSON Unicode escape".into()))?;
        self.position += 4;
        Ok(value)
    }

    fn number(&mut self) -> Result<String> {
        let start = self.position;
        if self.bytes.get(self.position) == Some(&b'-') {
            self.position += 1;
        }
        match self.bytes.get(self.position) {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => {
                self.position += 1;
                while self
                    .bytes
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
            }
            _ => return self.error("invalid number"),
        }
        if self.bytes.get(self.position) == Some(&b'.') {
            self.position += 1;
            let digits = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
            if digits == self.position {
                return self.error("fraction requires digits");
            }
        }
        if matches!(self.bytes.get(self.position), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.bytes.get(self.position), Some(b'+' | b'-')) {
                self.position += 1;
            }
            let digits = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
            if digits == self.position {
                return self.error("exponent requires digits");
            }
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.position])
            .expect("number is ASCII")
            .to_owned())
    }

    fn array(&mut self) -> Result<Value> {
        self.consume(b'[')?;
        let mut values = Vec::new();
        self.whitespace();
        if self.bytes.get(self.position) == Some(&b']') {
            self.position += 1;
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            match self.bytes.get(self.position) {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(Value::Array(values));
                }
                _ => return self.error("expected ',' or ']'"),
            }
        }
    }

    fn object(&mut self) -> Result<Value> {
        self.consume(b'{')?;
        let mut values = IndexMap::new();
        self.whitespace();
        if self.bytes.get(self.position) == Some(&b'}') {
            self.position += 1;
            return Ok(Value::Object(values));
        }
        loop {
            self.whitespace();
            if self.bytes.get(self.position) != Some(&b'"') {
                return self.error("object key must be a string");
            }
            let key = self.string()?;
            self.consume(b':')?;
            values.insert(key, self.value()?);
            self.whitespace();
            match self.bytes.get(self.position) {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Value::Object(values));
                }
                _ => return self.error("expected ',' or '}'"),
            }
        }
    }
}

fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch < '\u{0020}' => output.push_str(&format!("\\u{:04x}", ch as u32)),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

fn object_attributes(object: &IndexMap<String, Value>, forbidden: &[&str]) -> AttributeMap {
    object
        .iter()
        .filter(|(key, _)| !forbidden.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.attribute()))
        .collect()
}

/// An imported JSON edge: `(source_id, target_id, weight)`.
pub type JsonEdge = (String, String, f64);

/// Imports a JSON graph as vertices and edges with attribute maps.
///
/// Node identifiers may be strings or integers. Nodes without an `id` receive
/// generated singleton identifiers. Arrays and objects nested inside node or
/// edge attributes are retained as compact JSON strings. Edges without a
/// numeric `weight` attribute receive weight `1.0`.
pub struct JsonEventDrivenImporter {
    vertices_collection: String,
    edges_collection: String,
}

impl Default for JsonEventDrivenImporter {
    fn default() -> Self {
        Self {
            vertices_collection: "nodes".into(),
            edges_collection: "edges".into(),
        }
    }
}

impl JsonEventDrivenImporter {
    /// Creates an importer using `nodes` and `edges` as collection names.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the root-object member containing the vertex array.
    pub fn set_vertices_collection_name(&mut self, name: impl Into<String>) {
        self.vertices_collection = name.into();
    }

    /// Sets the root-object member containing the edge array.
    pub fn set_edges_collection_name(&mut self, name: impl Into<String>) {
        self.edges_collection = name.into();
    }
}

impl EventDrivenImporter for JsonEventDrivenImporter {
    type Vertex = String;
    type Edge = JsonEdge;

    fn import_events<R, F>(&mut self, mut input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let Value::Object(root) = Parser::new(&text).parse()? else {
            return Err(IoError::Import("JSON graph root must be an object".into()));
        };
        emit(ImportEvent::Start)?;
        let nodes = root.get(&self.vertices_collection).ok_or_else(|| {
            IoError::Import(format!(
                "JSON graph has no {:?} collection",
                self.vertices_collection
            ))
        })?;
        let Value::Array(nodes) = nodes else {
            return Err(IoError::Import(
                "JSON node collection must be an array".into(),
            ));
        };
        let mut singleton = 0usize;
        for node in nodes {
            let Value::Object(node) = node else {
                return Err(IoError::Import("JSON node must be an object".into()));
            };
            let id = if let Some(id) = node.get("id") {
                id.identifier()?
            } else {
                let id = format!("Singleton_{singleton}");
                singleton += 1;
                id
            };
            emit(ImportEvent::VertexWithAttributes(
                id,
                object_attributes(node, &["id"]),
            ))?;
        }
        let edges = root.get(&self.edges_collection).ok_or_else(|| {
            IoError::Import(format!(
                "JSON graph has no {:?} collection",
                self.edges_collection
            ))
        })?;
        let edges = match edges {
            Value::Array(edges) => edges.as_slice(),
            Value::Null => &[],
            _ => {
                return Err(IoError::Import(
                    "JSON edge collection must be an array or null".into(),
                ));
            }
        };
        for edge in edges {
            let Value::Object(edge) = edge else {
                return Err(IoError::Import("JSON edge must be an object".into()));
            };
            let source = edge
                .get("source")
                .ok_or_else(|| IoError::Import("JSON edge is missing source".into()))?
                .identifier()?;
            let target = edge
                .get("target")
                .ok_or_else(|| IoError::Import("JSON edge is missing target".into()))?
                .identifier()?;
            let attributes = object_attributes(edge, &["id", "source", "target"]);
            let weight = attributes
                .get("weight")
                .filter(|value| {
                    matches!(
                        value.attribute_type(),
                        AttributeType::Int
                            | AttributeType::Long
                            | AttributeType::Float
                            | AttributeType::Double
                    )
                })
                .and_then(|value| value.value().parse().ok())
                .unwrap_or(1.0);
            emit(ImportEvent::EdgeWithAttributes(
                (source, target, weight),
                attributes,
            ))?;
        }
        emit(ImportEvent::End)
    }
}

type VertexFactory<V> =
    Box<dyn FnMut(&str, &AttributeMap) -> std::result::Result<V, crate::CallbackError>>;
type EdgeFactory<E> = Box<dyn FnMut(&AttributeMap) -> E>;

/// Imports a graph from JGraphT's JSON interchange format.
///
/// The destination graph must support the features present in the input, such
/// as loops and parallel edges. Vertices and edges are created through the
/// destination graph's suppliers unless custom factories are configured. A
/// numeric edge `weight` is applied when the destination graph is weighted.
pub struct JsonImporter<V, E> {
    parser: JsonEventDrivenImporter,
    vertex_factory: Option<VertexFactory<V>>,
    edge_factory: Option<EdgeFactory<E>>,
}

impl<V, E> Default for JsonImporter<V, E> {
    fn default() -> Self {
        Self {
            parser: JsonEventDrivenImporter::new(),
            vertex_factory: None,
            edge_factory: None,
        }
    }
}

impl<V, E> JsonImporter<V, E> {
    /// Creates an importer using the default collection names and graph suppliers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a factory which creates vertices from their JSON identifiers.
    pub fn set_vertex_factory(&mut self, mut factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(move |id, _| Ok(factory(id))));
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
        mut factory: impl FnMut(&str) -> std::result::Result<V, crate::CallbackError> + 'static,
    ) {
        self.vertex_factory = Some(Box::new(move |id, _| factory(id)));
    }

    /// Sets a factory which creates vertices from identifiers and attributes.
    ///
    /// The supplied map contains every member of the node object except `id`.
    pub fn set_vertex_with_attributes_factory(
        &mut self,
        mut factory: impl FnMut(&str, &AttributeMap) -> V + 'static,
    ) {
        self.vertex_factory = Some(Box::new(move |id, attributes| Ok(factory(id, attributes))));
    }

    /// Sets a factory which creates edges from their JSON attributes.
    ///
    /// The supplied map excludes the reserved members `id`, `source`, and
    /// `target`.
    pub fn set_edge_with_attributes_factory(
        &mut self,
        factory: impl FnMut(&AttributeMap) -> E + 'static,
    ) {
        self.edge_factory = Some(Box::new(factory));
    }

    /// Sets the root-object member containing the vertex array.
    pub fn set_vertices_collection_name(&mut self, name: impl Into<String>) {
        self.parser.set_vertices_collection_name(name);
    }

    /// Sets the root-object member containing the edge array.
    pub fn set_edges_collection_name(&mut self, name: impl Into<String>) {
        self.parser.set_edges_collection_name(name);
    }
}

impl<V, E, G> GraphImporter<G> for JsonImporter<V, E>
where
    G: GraphMut<Vertex = V, Edge = E>,
    V: Clone,
    E: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let vertex_factory = &mut self.vertex_factory;
        let edge_factory = &mut self.edge_factory;
        let mut vertices = HashMap::<String, V>::new();
        self.parser.import_events(input, |event| {
            match event {
                ImportEvent::VertexWithAttributes(id, attributes) => {
                    if vertices.contains_key(&id) {
                        return Err(IoError::Import(format!("duplicate JSON node id {id:?}")));
                    }
                    let vertex = if let Some(factory) = vertex_factory.as_mut() {
                        let vertex = factory(&id, &attributes).map_err(IoError::Callback)?;
                        if !graph.add_vertex(vertex.clone()) {
                            return Err(IoError::Import(format!(
                                "vertex factory produced a duplicate for JSON id {id:?}"
                            )));
                        }
                        vertex
                    } else {
                        graph.add_new_vertex()?
                    };
                    vertices.insert(id, vertex);
                }
                ImportEvent::EdgeWithAttributes((source, target, weight), attributes) => {
                    let from = vertices.get(&source).ok_or_else(|| {
                        IoError::Import(format!("JSON source node {source:?} does not exist"))
                    })?;
                    let to = vertices.get(&target).ok_or_else(|| {
                        IoError::Import(format!("JSON target node {target:?} does not exist"))
                    })?;
                    let edge = if let Some(factory) = edge_factory.as_mut() {
                        let edge = factory(&attributes);
                        if !graph.add_edge_with(from, to, edge.clone())? {
                            return Err(IoError::Import("graph rejected JSON edge".into()));
                        }
                        edge
                    } else {
                        graph
                            .add_edge(from, to)?
                            .ok_or_else(|| IoError::Import("graph rejected JSON edge".into()))?
                    };
                    if graph.graph_type().weighted {
                        graph.set_edge_weight(&edge, weight)?;
                    }
                }
                _ => {}
            }
            Ok(())
        })
    }
}

/// Exports a graph using JGraphT's JSON interchange format.
///
/// The output root contains `creator` and `version` metadata plus configurable
/// vertex and edge collections. Vertices receive sequential integer-string
/// identifiers by default. Attribute providers may emit booleans, numbers,
/// nulls, and string values; non-finite floating-point values are rejected.
pub struct JsonExporter<V, E> {
    base: BaseExporter<V, E>,
    vertices_collection: String,
    edges_collection: String,
}

impl<V, E> JsonExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    /// Creates an exporter with sequential integer vertex identifiers.
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self::with_vertex_id_provider(move |vertex| ids.id(vertex))
    }

    /// Creates an exporter using `provider` to generate vertex identifiers.
    pub fn with_vertex_id_provider(provider: impl FnMut(&V) -> String + 'static) -> Self {
        Self {
            base: BaseExporter::new(provider),
            vertices_collection: "nodes".into(),
            edges_collection: "edges".into(),
        }
    }

    /// Sets the provider used to generate optional edge identifiers.
    pub fn set_edge_id_provider(&mut self, provider: impl FnMut(&E) -> String + 'static) {
        self.base.set_edge_id_provider(provider);
    }

    /// Sets the provider of vertex attributes.
    ///
    /// An attribute named `id` is ignored because the key is reserved.
    pub fn set_vertex_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.base.set_vertex_attribute_provider(provider);
    }

    /// Sets the provider of edge attributes.
    ///
    /// Attributes named `id`, `source`, or `target` are ignored because those
    /// keys are reserved.
    pub fn set_edge_attribute_provider(&mut self, provider: impl Fn(&E) -> AttributeMap + 'static) {
        self.base.set_edge_attribute_provider(provider);
    }

    /// Sets the root-object member used for the vertex array.
    pub fn set_vertices_collection_name(&mut self, name: impl Into<String>) {
        self.vertices_collection = name.into();
    }

    /// Sets the root-object member used for the edge array.
    pub fn set_edges_collection_name(&mut self, name: impl Into<String>) {
        self.edges_collection = name.into();
    }
}

impl<V, E> Default for JsonExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

fn write_attribute<W: Write>(mut output: W, attribute: &Attribute) -> Result<()> {
    match attribute.attribute_type() {
        AttributeType::Boolean => {
            output.write_all(if attribute.value().eq_ignore_ascii_case("true") {
                b"true"
            } else {
                b"false"
            })?;
        }
        AttributeType::Int | AttributeType::Long => {
            let value = attribute
                .value()
                .parse::<i64>()
                .map_err(|_| IoError::Export("invalid integer JSON attribute".into()))?;
            write!(output, "{value}")?;
        }
        AttributeType::Float | AttributeType::Double => {
            let value = attribute
                .value()
                .parse::<f64>()
                .map_err(|_| IoError::Export("invalid numeric JSON attribute".into()))?;
            if !value.is_finite() {
                return Err(IoError::Export(
                    "Infinity and NaN are not allowed in JSON".into(),
                ));
            }
            write!(output, "{}", format_f64(value))?;
        }
        AttributeType::Null => output.write_all(b"null")?,
        _ => write!(output, "{}", quote(attribute.value()))?,
    }
    Ok(())
}

impl<V, E, G> GraphExporter<G> for JsonExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        write!(
            output,
            "{{\"creator\":\"JGraphT JSON Exporter\",\"version\":\"1\",{}:[",
            quote(&self.vertices_collection)
        )?;
        for (index, vertex) in graph.vertices().enumerate() {
            if index > 0 {
                output.write_all(b",")?;
            }
            let id = self.base.vertex_id(vertex)?;
            write!(output, "{{\"id\":{}", quote(&id))?;
            if let Some(attributes) = self.base.vertex_attributes(vertex) {
                for (key, attribute) in attributes {
                    if key == "id" {
                        continue;
                    }
                    write!(output, ",{}:", quote(&key))?;
                    write_attribute(&mut output, &attribute)?;
                }
            }
            output.write_all(b"}")?;
        }
        write!(output, "],{}:[", quote(&self.edges_collection))?;
        for (index, edge) in graph.edges().enumerate() {
            if index > 0 {
                output.write_all(b",")?;
            }
            let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
            let source = self.base.vertex_id(source)?;
            let target = self.base.vertex_id(target)?;
            output.write_all(b"{")?;
            if let Some(id) = self.base.edge_id(edge) {
                write!(output, "\"id\":{},", quote(&id))?;
            }
            write!(
                output,
                "\"source\":{},\"target\":{}",
                quote(&source),
                quote(&target)
            )?;
            if let Some(attributes) = self.base.edge_attributes(edge) {
                for (key, attribute) in attributes {
                    if matches!(key.as_str(), "id" | "source" | "target") {
                        continue;
                    }
                    write!(output, ",{}:", quote(&key))?;
                    write_attribute(&mut output, &attribute)?;
                }
            }
            output.write_all(b"}")?;
        }
        output.write_all(b"]}")?;
        Ok(())
    }
}
