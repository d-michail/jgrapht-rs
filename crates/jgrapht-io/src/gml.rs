//! Graph Modelling Language (GML) import and export.

use crate::exporter::format_f64;
use crate::{BaseExporter, GraphExporter, GraphImporter, IntegerIdProvider, IoError, Result};
use indexmap::IndexMap;
use jgrapht::{Graph, GraphMut};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write};

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

pub struct GmlImporter<V> {
    vertex_factory: Option<Box<dyn FnMut(i64) -> V>>,
}

impl<V> GmlImporter<V> {
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
        }
    }

    pub fn set_vertex_factory(&mut self, factory: impl FnMut(i64) -> V + 'static) {
        self.vertex_factory = Some(Box::new(factory));
    }
}

impl<V> Default for GmlImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, G> GraphImporter<G> for GmlImporter<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, mut input: R) -> Result<()> {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let document = Parser::new(&text).parse()?;
        let graph_data = document
            .iter()
            .find_map(|(key, value)| {
                (key == "graph")
                    .then_some(value)
                    .and_then(|value| match value {
                        Value::List(value) => Some(value),
                        _ => None,
                    })
            })
            .ok_or_else(|| IoError::Import("GML document has no graph list".into()))?;
        let mut vertices = HashMap::new();
        for (_, value) in graph_data.iter().filter(|(key, _)| key == "node") {
            let Value::List(node) = value else { continue };
            let id = number(node, "id")
                .ok_or_else(|| IoError::Import("GML node is missing id".into()))?
                .parse::<i64>()
                .map_err(|_| IoError::Import("invalid GML node id".into()))?;
            if vertices.contains_key(&id) {
                return Err(IoError::Import(format!("duplicate GML node id {id}")));
            }
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(id);
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
            let source = number(edge, "source")
                .ok_or_else(|| IoError::Import("GML edge is missing source".into()))?
                .parse::<i64>()
                .map_err(|_| IoError::Import("invalid GML edge source".into()))?;
            let target = number(edge, "target")
                .ok_or_else(|| IoError::Import("GML edge is missing target".into()))?
                .parse::<i64>()
                .map_err(|_| IoError::Import("invalid GML edge target".into()))?;
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
        }
        Ok(())
    }
}

pub struct GmlExporter<V, E> {
    base: BaseExporter<V, E>,
    export_weights: bool,
}

impl<V, E> GmlExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |vertex| ids.id(vertex)),
            export_weights: false,
        }
    }

    pub fn set_export_edge_weights(&mut self, enabled: bool) {
        self.export_weights = enabled;
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
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
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
            let id = self.base.vertex_id(vertex)?;
            ids.insert(vertex.clone(), id.clone());
            writeln!(output, "\tnode\n\t[\n\t\tid {id}\n\t]")?;
        }
        for (index, edge) in graph.edges().enumerate() {
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
            writeln!(
                output,
                "\tedge\n\t[\n\t\tid {}\n\t\tsource {}\n\t\ttarget {}",
                index + 1,
                ids[source],
                ids[target]
            )?;
            if self.export_weights {
                writeln!(
                    output,
                    "\t\tweight {}",
                    format_f64(graph.edge_weight(edge).unwrap_or(1.0))
                )?;
            }
            writeln!(output, "\t]")?;
        }
        writeln!(output, "]")?;
        Ok(())
    }
}
