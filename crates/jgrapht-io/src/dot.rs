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

//! Graphviz DOT import and export.

use crate::{
    Attribute, AttributeMap, AttributeType, BaseExporter, GraphExporter, GraphImporter,
    IntegerIdProvider, IoError, Result,
};
use indexmap::{IndexMap, IndexSet};
use jgrapht::{DefaultEdge, Graph, GraphMut};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Id(String),
    Arrow,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Equal,
    Colon,
    Separator,
}

fn lex(input: &str) -> Result<Vec<Token>> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b if b.is_ascii_whitespace() => i += 1,
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                    i += 1;
                }
                if i + 1 >= bytes.len() {
                    return Err(IoError::Import("unterminated DOT comment".into()));
                }
                i += 2;
            }
            b'{' => {
                tokens.push(Token::LBrace);
                i += 1;
            }
            b'}' => {
                tokens.push(Token::RBrace);
                i += 1;
            }
            b'[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            b']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            b'=' => {
                tokens.push(Token::Equal);
                i += 1;
            }
            b':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            b';' | b',' => {
                tokens.push(Token::Separator);
                i += 1;
            }
            b'-' if matches!(bytes.get(i + 1), Some(b'-' | b'>')) => {
                tokens.push(Token::Arrow);
                i += 2;
            }
            b'"' => {
                i += 1;
                let mut value = Vec::new();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'"') {
                        value.push(b'"');
                        i += 2;
                    } else {
                        value.push(bytes[i]);
                        i += 1;
                    }
                }
                if i == bytes.len() {
                    return Err(IoError::Import("unterminated DOT string".into()));
                }
                i += 1;
                tokens.push(Token::Id(String::from_utf8(value).map_err(|e| {
                    IoError::Import(format!("invalid UTF-8 in DOT string: {e}"))
                })?));
            }
            b'<' => {
                let start = i;
                let mut depth = 0isize;
                let mut quoted = false;
                while i < bytes.len() {
                    match bytes[i] {
                        b'"' => quoted = !quoted,
                        b'<' if !quoted => depth += 1,
                        b'>' if !quoted => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                if depth != 0 || quoted {
                    return Err(IoError::Import("unterminated DOT HTML identifier".into()));
                }
                let raw = std::str::from_utf8(&bytes[start..i])
                    .map_err(|e| IoError::Import(format!("invalid DOT UTF-8: {e}")))?;
                tokens.push(Token::Id(raw[1..raw.len() - 1].to_owned()));
            }
            _ => {
                let start = i;
                while i < bytes.len() {
                    let edge = bytes[i] == b'-' && matches!(bytes.get(i + 1), Some(b'-' | b'>'));
                    if bytes[i].is_ascii_whitespace() || b"{}[]=:;,<>\"".contains(&bytes[i]) || edge
                    {
                        break;
                    }
                    i += 1;
                }
                if start == i {
                    return Err(IoError::Import(format!("unexpected DOT byte {}", bytes[i])));
                }
                let value = std::str::from_utf8(&bytes[start..i])
                    .map_err(|e| IoError::Import(format!("invalid DOT UTF-8: {e}")))?;
                if !is_valid_unquoted_id(value) {
                    return Err(IoError::Import(format!("invalid DOT identifier {value:?}")));
                }
                tokens.push(Token::Id(value.to_owned()));
            }
        }
    }
    Ok(tokens)
}

#[derive(Debug, Clone, Default)]
struct Scope {
    graph: AttributeMap,
    node: AttributeMap,
    edge: AttributeMap,
}

#[derive(Debug)]
struct Parsed {
    directed: bool,
    strict: bool,
    graph_id: Option<String>,
    graph_attributes: AttributeMap,
    nodes: IndexMap<String, AttributeMap>,
    edges: Vec<(String, String, AttributeMap)>,
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    directed: bool,
    strict: bool,
    graph_id: Option<String>,
    graph_attributes: AttributeMap,
    nodes: IndexMap<String, AttributeMap>,
    edges: Vec<(String, String, AttributeMap)>,
}

impl Parser {
    fn parse(input: &str) -> Result<Parsed> {
        let mut p = Self {
            tokens: lex(input)?,
            position: 0,
            directed: false,
            strict: false,
            graph_id: None,
            graph_attributes: AttributeMap::new(),
            nodes: IndexMap::new(),
            edges: Vec::new(),
        };
        if p.keyword("strict") {
            p.strict = true;
        }
        if p.keyword("digraph") {
            p.directed = true;
        } else if !p.keyword("graph") {
            return Err(IoError::Import(
                "DOT input must start with graph or digraph".into(),
            ));
        }
        if matches!(p.peek(), Some(Token::Id(_))) {
            p.graph_id = Some(p.id()?);
        }
        p.expect(Token::LBrace)?;
        p.block(Scope::default(), true)?;
        if p.position != p.tokens.len() {
            return Err(IoError::Import("trailing DOT input".into()));
        }
        Ok(Parsed {
            directed: p.directed,
            strict: p.strict,
            graph_id: p.graph_id,
            graph_attributes: p.graph_attributes,
            nodes: p.nodes,
            edges: p.edges,
        })
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
    fn keyword(&mut self, word: &str) -> bool {
        if matches!(self.peek(), Some(Token::Id(v)) if v.eq_ignore_ascii_case(word)) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, token: Token) -> Result<()> {
        if self.peek() == Some(&token) {
            self.position += 1;
            Ok(())
        } else {
            Err(IoError::Import(format!(
                "DOT token {}: expected {token:?}",
                self.position
            )))
        }
    }
    fn id(&mut self) -> Result<String> {
        match self.tokens.get(self.position).cloned() {
            Some(Token::Id(v)) => {
                self.position += 1;
                Ok(v)
            }
            _ => Err(IoError::Import(format!(
                "DOT token {}: expected identifier",
                self.position
            ))),
        }
    }
    fn attributes(&mut self) -> Result<AttributeMap> {
        let mut attrs = AttributeMap::new();
        while self.peek() == Some(&Token::LBracket) {
            self.position += 1;
            while self.peek() != Some(&Token::RBracket) {
                if self.peek().is_none() {
                    return Err(IoError::Import("unterminated DOT attribute list".into()));
                }
                if self.peek() == Some(&Token::Separator) {
                    self.position += 1;
                    continue;
                }
                let key = self.id()?;
                self.expect(Token::Equal)?;
                attrs.insert(key, Attribute::from(self.id()?));
                if self.peek() == Some(&Token::Separator) {
                    self.position += 1;
                }
            }
            self.position += 1;
        }
        Ok(attrs)
    }
    fn add_node(&mut self, id: String, defaults: &AttributeMap) {
        self.nodes.entry(id).or_insert_with(|| defaults.clone());
    }
    fn endpoint(&mut self, scope: &Scope) -> Result<IndexSet<String>> {
        if self.keyword("subgraph") && matches!(self.peek(), Some(Token::Id(_))) {
            self.position += 1;
        }
        if self.peek() == Some(&Token::LBrace) {
            self.position += 1;
            return self.block(scope.clone(), false);
        }
        let id = self.id()?;
        while self.peek() == Some(&Token::Colon) {
            self.position += 1;
            self.id()?;
        }
        self.add_node(id.clone(), &scope.node);
        Ok(IndexSet::from([id]))
    }
    fn block(&mut self, mut scope: Scope, top: bool) -> Result<IndexSet<String>> {
        let mut members = IndexSet::new();
        while self.peek() != Some(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(IoError::Import("unterminated DOT graph".into()));
            }
            if self.peek() == Some(&Token::Separator) {
                self.position += 1;
                continue;
            }
            let kind = match self.peek() {
                Some(Token::Id(v)) if v.eq_ignore_ascii_case("graph") => Some(0),
                Some(Token::Id(v)) if v.eq_ignore_ascii_case("node") => Some(1),
                Some(Token::Id(v)) if v.eq_ignore_ascii_case("edge") => Some(2),
                _ => None,
            };
            if let Some(kind) = kind
                && self.tokens.get(self.position + 1) == Some(&Token::LBracket)
            {
                self.position += 1;
                let attrs = self.attributes()?;
                match kind {
                    0 => {
                        scope.graph.extend(attrs.clone());
                        if top {
                            self.graph_attributes.extend(attrs);
                        }
                    }
                    1 => scope.node.extend(attrs),
                    _ => scope.edge.extend(attrs),
                }
                continue;
            }
            let left = self.endpoint(&scope)?;
            members.extend(left.iter().cloned());
            if self.peek() == Some(&Token::Equal) {
                if left.len() != 1 {
                    return Err(IoError::Import("invalid DOT graph attribute".into()));
                }
                self.position += 1;
                let value = self.id()?;
                let key = left.first().expect("single endpoint").clone();
                self.nodes.shift_remove(&key);
                members.shift_remove(&key);
                scope
                    .graph
                    .insert(key.clone(), Attribute::from(value.clone()));
                if top {
                    self.graph_attributes.insert(key, Attribute::from(value));
                }
                continue;
            }
            let mut groups = Vec::new();
            let mut previous = left;
            while self.peek() == Some(&Token::Arrow) {
                self.position += 1;
                let right = self.endpoint(&scope)?;
                members.extend(right.iter().cloned());
                groups.push((previous, right.clone()));
                previous = right;
            }
            let attrs = self.attributes()?;
            if groups.is_empty() {
                for id in previous {
                    if let Some(a) = self.nodes.get_mut(&id) {
                        a.extend(attrs.clone());
                    }
                }
            } else {
                let mut all = scope.edge.clone();
                all.extend(attrs);
                for (sources, targets) in groups {
                    for s in &sources {
                        for t in &targets {
                            self.edges.push((s.clone(), t.clone(), all.clone()));
                        }
                    }
                }
            }
            if self.peek() == Some(&Token::Separator) {
                self.position += 1;
            }
        }
        self.position += 1;
        Ok(members)
    }
}

type VertexFactory<V> = Box<dyn FnMut(&str) -> V>;
type VertexAttrsFactory<V> = Box<dyn FnMut(&str, &AttributeMap) -> V>;
type EdgeAttrsFactory<E> = Box<dyn FnMut(&AttributeMap) -> E>;
type GraphAttrConsumer = Box<dyn FnMut(&str, &Attribute) -> Result<()>>;
type VertexAttrConsumer<V> = Box<dyn FnMut(&V, &str, &Attribute) -> Result<()>>;
type EdgeAttrConsumer<E> = Box<dyn FnMut(&E, &str, &Attribute) -> Result<()>>;

pub struct DotImporter<V, E = DefaultEdge> {
    vertex_factory: Option<VertexFactory<V>>,
    vertex_attrs_factory: Option<VertexAttrsFactory<V>>,
    edge_attrs_factory: Option<EdgeAttrsFactory<E>>,
    graph_attr_consumer: Option<GraphAttrConsumer>,
    vertex_attr_consumer: Option<VertexAttrConsumer<V>>,
    edge_attr_consumer: Option<EdgeAttrConsumer<E>>,
}
impl<V, E> DotImporter<V, E> {
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
            vertex_attrs_factory: None,
            edge_attrs_factory: None,
            graph_attr_consumer: None,
            vertex_attr_consumer: None,
            edge_attr_consumer: None,
        }
    }
    pub fn set_vertex_factory(&mut self, f: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(f));
        self.vertex_attrs_factory = None;
    }
    pub fn set_vertex_with_attributes_factory(
        &mut self,
        f: impl FnMut(&str, &AttributeMap) -> V + 'static,
    ) {
        self.vertex_attrs_factory = Some(Box::new(f));
        self.vertex_factory = None;
    }
    pub fn set_edge_with_attributes_factory(
        &mut self,
        f: impl FnMut(&AttributeMap) -> E + 'static,
    ) {
        self.edge_attrs_factory = Some(Box::new(f));
    }
    pub fn set_graph_attribute_consumer(
        &mut self,
        f: impl FnMut(&str, &Attribute) -> Result<()> + 'static,
    ) {
        self.graph_attr_consumer = Some(Box::new(f));
    }
    pub fn set_vertex_attribute_consumer(
        &mut self,
        f: impl FnMut(&V, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.vertex_attr_consumer = Some(Box::new(f));
    }
    pub fn set_edge_attribute_consumer(
        &mut self,
        f: impl FnMut(&E, &str, &Attribute) -> Result<()> + 'static,
    ) {
        self.edge_attr_consumer = Some(Box::new(f));
    }
}
impl<V, E> Default for DotImporter<V, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphImporter<G> for DotImporter<V, E>
where
    G: GraphMut<Vertex = V, Edge = E>,
    V: Clone,
    E: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, mut input: R) -> Result<()> {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let parsed = Parser::parse(&text)?;
        if parsed.directed != graph.graph_type().directed {
            return Err(IoError::Import(
                "DOT graph direction does not match target graph".into(),
            ));
        }
        if let Some(consumer) = self.graph_attr_consumer.as_mut() {
            if let Some(id) = &parsed.graph_id {
                consumer("ID", &Attribute::from(id.clone()))?;
            }
            for (k, v) in &parsed.graph_attributes {
                consumer(k, v)?;
            }
        }
        let mut vertices = HashMap::new();
        for (id, attrs) in &parsed.nodes {
            let vertex = if let Some(f) = self.vertex_attrs_factory.as_mut() {
                f(id, attrs)
            } else if let Some(f) = self.vertex_factory.as_mut() {
                f(id)
            } else {
                graph.new_vertex()?
            };
            if !graph.add_vertex(vertex.clone()) {
                return Err(IoError::Import(
                    "DOT vertex factory produced duplicate".into(),
                ));
            }
            if let Some(c) = self.vertex_attr_consumer.as_mut() {
                c(&vertex, "ID", &Attribute::from(id.clone()))?;
                for (k, v) in attrs {
                    c(&vertex, k, v)?;
                }
            }
            vertices.insert(id.clone(), vertex);
        }
        let mut seen = HashSet::new();
        for (source, target, attrs) in parsed.edges {
            let key = if parsed.directed || source <= target {
                (source.clone(), target.clone())
            } else {
                (target.clone(), source.clone())
            };
            if parsed.strict && !seen.insert(key) {
                continue;
            }
            let (from, to) = (&vertices[&source], &vertices[&target]);
            let edge = if let Some(f) = self.edge_attrs_factory.as_mut() {
                let e = f(&attrs);
                graph.add_edge_with(from, to, e.clone())?.then_some(e)
            } else {
                graph.add_edge(from, to)?
            }
            .ok_or_else(|| IoError::Import("graph rejected DOT edge".into()))?;
            if let Some(c) = self.edge_attr_consumer.as_mut() {
                for (k, v) in &attrs {
                    c(&edge, k, v)?;
                }
            }
        }
        Ok(())
    }
}

fn is_valid_unquoted_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = usize::from(bytes[0] == b'-');
    if start == bytes.len() {
        return false;
    }
    let number = &bytes[start..];
    let numeric = if number[0] == b'.' {
        number.len() > 1 && number[1..].iter().all(u8::is_ascii_digit)
    } else if number[0].is_ascii_digit() {
        let mut dot = false;
        number.iter().all(|b| {
            if *b == b'.' && !dot {
                dot = true;
                true
            } else {
                b.is_ascii_digit()
            }
        })
    } else {
        false
    };
    if numeric {
        return true;
    }
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric())
}
pub fn is_valid_id(value: &str) -> bool {
    (value.len() >= 2 && value.starts_with('"') && value.ends_with('"'))
        || (value.len() >= 2 && value.starts_with('<') && value.ends_with('>'))
        || is_valid_unquoted_id(value)
}
fn render_attributes(attrs: AttributeMap) -> String {
    if attrs.is_empty() {
        return String::new();
    }
    let mut out = String::from(" [ ");
    for (name, value) in attrs {
        out.push_str(&name);
        out.push('=');
        match value.attribute_type() {
            AttributeType::Html => out.push_str(&format!("<{}>", value.value())),
            AttributeType::Identifier => out.push_str(value.value()),
            _ => out.push_str(&format!("\"{}\"", value.value().replace('"', "\\\""))),
        }
        out.push(' ');
    }
    out.push(']');
    out
}

#[derive(Debug, Clone)]
pub struct DotSubgraph<V, E> {
    pub vertices: Vec<V>,
    pub edges: Vec<E>,
    pub graph_attributes: AttributeMap,
    pub cluster_attributes: AttributeMap,
}
impl<V, E> DotSubgraph<V, E> {
    pub fn new(vertices: impl IntoIterator<Item = V>, edges: impl IntoIterator<Item = E>) -> Self {
        Self {
            vertices: vertices.into_iter().collect(),
            edges: edges.into_iter().collect(),
            graph_attributes: AttributeMap::new(),
            cluster_attributes: AttributeMap::new(),
        }
    }
}
type SubgraphProvider<V, E> = Box<dyn Fn() -> IndexMap<String, DotSubgraph<V, E>>>;
pub struct DotExporter<V, E> {
    base: BaseExporter<V, E>,
    subgraph_provider: Option<SubgraphProvider<V, E>>,
}
impl<V, E> DotExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |v| ids.id(v)),
            subgraph_provider: None,
        }
    }
    pub fn with_vertex_id_provider(p: impl FnMut(&V) -> String + 'static) -> Self {
        Self {
            base: BaseExporter::new(p),
            subgraph_provider: None,
        }
    }
    pub fn set_graph_id_provider(&mut self, p: impl FnMut() -> String + 'static) {
        self.base.set_graph_id_provider(p);
    }
    pub fn set_graph_attribute_provider(&mut self, p: impl Fn() -> AttributeMap + 'static) {
        self.base.set_graph_attribute_provider(p);
    }
    pub fn set_vertex_attribute_provider(&mut self, p: impl Fn(&V) -> AttributeMap + 'static) {
        self.base.set_vertex_attribute_provider(p);
    }
    pub fn set_edge_attribute_provider(&mut self, p: impl Fn(&E) -> AttributeMap + 'static) {
        self.base.set_edge_attribute_provider(p);
    }
    pub fn set_subgraph_provider(
        &mut self,
        p: impl Fn() -> IndexMap<String, DotSubgraph<V, E>> + 'static,
    ) {
        self.subgraph_provider = Some(Box::new(p));
    }
}
impl<V, E> Default for DotExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for DotExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        let graph_id = self.base.graph_id().unwrap_or_else(|| "G".into());
        if !is_valid_id(&graph_id) {
            return Err(IoError::Export(format!(
                "invalid DOT graph id {graph_id:?}"
            )));
        }
        writeln!(
            output,
            "{}{} {} {{",
            if graph.graph_type().multiple_edges {
                ""
            } else {
                "strict "
            },
            if graph.graph_type().directed {
                "digraph"
            } else {
                "graph"
            },
            graph_id
        )?;
        if let Some(attrs) = self.base.graph_attributes() {
            for (k, v) in attrs {
                writeln!(output, "  {k}={};", v.value())?;
            }
        }
        let mut ids = HashMap::new();
        for vertex in graph.vertices() {
            let id = self.base.vertex_id(vertex)?;
            if !is_valid_id(&id) {
                return Err(IoError::Export(format!("invalid DOT vertex id {id:?}")));
            }
            ids.insert(vertex.clone(), id.clone());
            writeln!(
                output,
                "  {}{};",
                id,
                self.base
                    .vertex_attributes(vertex)
                    .map(render_attributes)
                    .unwrap_or_default()
            )?;
        }
        let connector = if graph.graph_type().directed {
            " -> "
        } else {
            " -- "
        };
        for edge in graph.edges() {
            write_edge(&mut output, graph, edge, &ids, connector, "  ", &self.base)?;
        }
        if let Some(provider) = &self.subgraph_provider {
            for (name, subgraph) in provider() {
                if !is_valid_id(&name) {
                    return Err(IoError::Export(format!("invalid DOT subgraph id {name:?}")));
                }
                writeln!(output, "  subgraph {name} {{")?;
                if !subgraph.cluster_attributes.is_empty() {
                    writeln!(
                        output,
                        "    {name}{};",
                        render_attributes(subgraph.cluster_attributes)
                    )?;
                }
                for (k, v) in subgraph.graph_attributes {
                    writeln!(output, "    {k}={};", v.value())?;
                }
                for vertex in &subgraph.vertices {
                    writeln!(
                        output,
                        "    {};",
                        ids.get(vertex).ok_or_else(|| IoError::Export(
                            "DOT subgraph contains an unknown vertex".into()
                        ))?
                    )?;
                }
                for edge in &subgraph.edges {
                    write_edge(
                        &mut output,
                        graph,
                        edge,
                        &ids,
                        connector,
                        "    ",
                        &self.base,
                    )?;
                }
                writeln!(output, "  }}")?;
            }
        }
        writeln!(output, "}}")?;
        Ok(())
    }
}
fn write_edge<V, E, G, W>(
    output: &mut W,
    graph: &G,
    edge: &E,
    ids: &HashMap<V, String>,
    connector: &str,
    indent: &str,
    base: &BaseExporter<V, E>,
) -> Result<()>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash,
    E: Clone + Eq + Hash,
    W: Write,
{
    let source = graph
        .edge_source(edge)
        .ok_or_else(|| IoError::Export("edge has no source".into()))?;
    let target = graph
        .edge_target(edge)
        .ok_or_else(|| IoError::Export("edge has no target".into()))?;
    writeln!(
        output,
        "{indent}{}{}{}{};",
        ids[source],
        connector,
        ids[target],
        base.edge_attributes(edge)
            .map(render_attributes)
            .unwrap_or_default()
    )?;
    Ok(())
}
