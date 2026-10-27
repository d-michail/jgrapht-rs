//! Graphviz DOT import and export.

use crate::{
    AttributeMap, AttributeType, BaseExporter, GraphExporter, GraphImporter, IntegerIdProvider,
    IoError, Result,
};
use indexmap::IndexSet;
use jgrapht::{Graph, GraphMut};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Id(String),
    Arrow(bool),
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
            byte if byte.is_ascii_whitespace() => i += 1,
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
                tokens.push(Token::Arrow(bytes[i + 1] == b'>'));
                i += 2;
            }
            b'"' => {
                i += 1;
                let mut value = String::new();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                        value.push('"');
                        i += 2;
                    } else {
                        value.push(bytes[i] as char);
                        i += 1;
                    }
                }
                if i == bytes.len() {
                    return Err(IoError::Import("unterminated DOT string".into()));
                }
                i += 1;
                tokens.push(Token::Id(value));
            }
            b'<' => {
                let start = i;
                let mut depth = 0isize;
                while i < bytes.len() {
                    if bytes[i] == b'<' {
                        depth += 1;
                    } else if bytes[i] == b'>' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    i += 1;
                }
                if depth != 0 {
                    return Err(IoError::Import("unterminated DOT HTML identifier".into()));
                }
                tokens.push(Token::Id(
                    String::from_utf8_lossy(&bytes[start..i]).into_owned(),
                ));
            }
            _ => {
                let start = i;
                while i < bytes.len() {
                    let edge_operator =
                        bytes[i] == b'-' && matches!(bytes.get(i + 1), Some(b'-' | b'>'));
                    if bytes[i].is_ascii_whitespace()
                        || b"{}[]=:;,".contains(&bytes[i])
                        || edge_operator
                    {
                        break;
                    }
                    i += 1;
                }
                if start == i {
                    return Err(IoError::Import(format!("unexpected DOT byte {}", bytes[i])));
                }
                tokens.push(Token::Id(
                    String::from_utf8_lossy(&bytes[start..i]).into_owned(),
                ));
            }
        }
    }
    Ok(tokens)
}

#[derive(Debug)]
struct Parsed {
    directed: bool,
    strict: bool,
    nodes: IndexSet<String>,
    edges: Vec<(String, String)>,
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    directed: bool,
    strict: bool,
    nodes: IndexSet<String>,
    edges: Vec<(String, String)>,
}

impl Parser {
    fn parse(input: &str) -> Result<Parsed> {
        let mut parser = Self {
            tokens: lex(input)?,
            position: 0,
            directed: false,
            strict: false,
            nodes: IndexSet::new(),
            edges: Vec::new(),
        };
        if parser.keyword("strict") {
            parser.strict = true;
        }
        if parser.keyword("digraph") {
            parser.directed = true;
        } else if !parser.keyword("graph") {
            return Err(IoError::Import(
                "DOT input must start with graph or digraph".into(),
            ));
        }
        if matches!(parser.peek(), Some(Token::Id(_))) {
            parser.position += 1;
        }
        parser.expect(Token::LBrace)?;
        parser.block()?;
        if parser.position != parser.tokens.len() {
            return Err(IoError::Import("trailing DOT input".into()));
        }
        Ok(Parsed {
            directed: parser.directed,
            strict: parser.strict,
            nodes: parser.nodes,
            edges: parser.edges,
        })
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
    fn keyword(&mut self, keyword: &str) -> bool {
        if matches!(self.peek(), Some(Token::Id(value)) if value.eq_ignore_ascii_case(keyword)) {
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
            Some(Token::Id(value)) => {
                self.position += 1;
                Ok(value)
            }
            _ => Err(IoError::Import(format!(
                "DOT token {}: expected identifier",
                self.position
            ))),
        }
    }
    fn attributes(&mut self) -> Result<()> {
        while self.peek() == Some(&Token::LBracket) {
            self.position += 1;
            let mut depth = 1;
            while depth > 0 {
                match self.tokens.get(self.position) {
                    Some(Token::LBracket) => depth += 1,
                    Some(Token::RBracket) => depth -= 1,
                    None => return Err(IoError::Import("unterminated DOT attribute list".into())),
                    _ => {}
                }
                self.position += 1;
            }
        }
        Ok(())
    }
    fn endpoint(&mut self) -> Result<Vec<String>> {
        if self.keyword("subgraph") && matches!(self.peek(), Some(Token::Id(_))) {
            self.position += 1;
        }
        if self.peek() == Some(&Token::LBrace) {
            self.position += 1;
            let before = self.nodes.len();
            self.block()?;
            return Ok(self.nodes.iter().skip(before).cloned().collect());
        }
        let id = self.id()?;
        while self.peek() == Some(&Token::Colon) {
            self.position += 1;
            self.id()?;
        }
        self.nodes.insert(id.clone());
        Ok(vec![id])
    }
    fn block(&mut self) -> Result<()> {
        while self.peek() != Some(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(IoError::Import("unterminated DOT graph".into()));
            }
            if self.peek() == Some(&Token::Separator) {
                self.position += 1;
                continue;
            }
            if matches!(self.peek(), Some(Token::Id(value)) if ["graph", "node", "edge"].iter().any(|keyword| value.eq_ignore_ascii_case(keyword)))
                && self.tokens.get(self.position + 1) == Some(&Token::LBracket)
            {
                self.position += 1;
                self.attributes()?;
                continue;
            }
            let mut left = self.endpoint()?;
            if self.peek() == Some(&Token::Equal) {
                self.position += 1;
                self.id()?;
                continue;
            }
            while let Some(Token::Arrow(directed)) = self.peek().cloned() {
                if directed != self.directed {
                    return Err(IoError::Import(
                        "DOT edge operator does not match graph type".into(),
                    ));
                }
                self.position += 1;
                let right = self.endpoint()?;
                for source in &left {
                    for target in &right {
                        self.edges.push((source.clone(), target.clone()));
                    }
                }
                left = right;
            }
            self.attributes()?;
            if self.peek() == Some(&Token::Separator) {
                self.position += 1;
            }
        }
        self.position += 1;
        Ok(())
    }
}

type VertexFactory<V> = Box<dyn FnMut(&str) -> V>;

pub struct DotImporter<V> {
    vertex_factory: Option<VertexFactory<V>>,
}
impl<V> DotImporter<V> {
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
        }
    }
    pub fn set_vertex_factory(&mut self, factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(factory));
    }
}
impl<V> Default for DotImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, G> GraphImporter<G> for DotImporter<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone,
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
        if parsed.strict && graph.graph_type().multiple_edges { /* strict input remains valid in a multigraph target */
        }
        let mut vertices = HashMap::new();
        for id in parsed.nodes {
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(&id);
                if !graph.add_vertex(vertex.clone()) {
                    return Err(IoError::Import(
                        "DOT vertex factory produced duplicate".into(),
                    ));
                }
                vertex
            } else {
                graph.add_new_vertex()?
            };
            vertices.insert(id, vertex);
        }
        for (source, target) in parsed.edges {
            let from = &vertices[&source];
            let to = &vertices[&target];
            if graph.add_edge(from, to)?.is_none() && !parsed.strict {
                return Err(IoError::Import("graph rejected DOT edge".into()));
            }
        }
        Ok(())
    }
}

pub fn is_valid_id(value: &str) -> bool {
    if value.starts_with('"') && value.ends_with('"')
        || value.starts_with('<') && value.ends_with('>')
    {
        return true;
    }
    if value.parse::<f64>().is_ok() {
        return true;
    }
    let mut chars = value.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn render_attributes(attributes: AttributeMap) -> String {
    if attributes.is_empty() {
        return String::new();
    }
    let mut output = String::from(" [ ");
    for (name, value) in attributes {
        output.push_str(&name);
        output.push('=');
        match value.attribute_type() {
            AttributeType::Html => output.push_str(&format!("<{}>", value.value())),
            AttributeType::Identifier => output.push_str(value.value()),
            _ => output.push_str(&format!("\"{}\"", value.value().replace('"', "\\\""))),
        }
        output.push(' ');
    }
    output.push(']');
    output
}

pub struct DotExporter<V, E> {
    base: BaseExporter<V, E>,
}
impl<V, E> DotExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |vertex| ids.id(vertex)),
        }
    }
    pub fn with_vertex_id_provider(provider: impl FnMut(&V) -> String + 'static) -> Self {
        Self {
            base: BaseExporter::new(provider),
        }
    }
    pub fn set_graph_id_provider(&mut self, provider: impl FnMut() -> String + 'static) {
        self.base.set_graph_id_provider(provider);
    }
    pub fn set_graph_attribute_provider(&mut self, provider: impl Fn() -> AttributeMap + 'static) {
        self.base.set_graph_attribute_provider(provider);
    }
    pub fn set_vertex_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.base.set_vertex_attribute_provider(provider);
    }
    pub fn set_edge_attribute_provider(&mut self, provider: impl Fn(&E) -> AttributeMap + 'static) {
        self.base.set_edge_attribute_provider(provider);
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
        if let Some(attributes) = self.base.graph_attributes() {
            for (name, value) in attributes {
                writeln!(output, "  {name}={};", value.value())?;
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
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
            writeln!(
                output,
                "  {}{}{}{};",
                ids[source],
                connector,
                ids[target],
                self.base
                    .edge_attributes(edge)
                    .map(render_attributes)
                    .unwrap_or_default()
            )?;
        }
        writeln!(output, "}}")?;
        Ok(())
    }
}
