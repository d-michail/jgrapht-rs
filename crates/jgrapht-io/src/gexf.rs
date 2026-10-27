//! Basic GEXF 1.2 import and export support.

use crate::exporter::format_f64;
use crate::{BaseExporter, GraphExporter, GraphImporter, IntegerIdProvider, IoError, Result};
use indexmap::IndexMap;
use jgrapht::{Graph, GraphMut};
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
}

#[derive(Debug)]
struct Edge {
    source: String,
    target: String,
    weight: Option<f64>,
}

fn parse<R: Read>(input: R) -> Result<(Vec<Node>, Vec<Edge>)> {
    let mut reader = Reader::from_reader(BufReader::new(input));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| IoError::Import(format!("GEXF XML error: {error}")))?
        {
            Event::Start(element) | Event::Empty(element) => match element.local_name().as_ref() {
                b"node" => nodes.push(Node {
                    id: attr(&reader, &element, b"id")?
                        .ok_or_else(|| IoError::Import("GEXF node is missing id".into()))?,
                }),
                b"edge" => edges.push(Edge {
                    source: attr(&reader, &element, b"source")?
                        .ok_or_else(|| IoError::Import("GEXF edge is missing source".into()))?,
                    target: attr(&reader, &element, b"target")?
                        .ok_or_else(|| IoError::Import("GEXF edge is missing target".into()))?,
                    weight: attr(&reader, &element, b"weight")?
                        .map(|value| value.parse())
                        .transpose()
                        .map_err(|_| IoError::Import("invalid GEXF edge weight".into()))?,
                }),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok((nodes, edges))
}

type VertexFactory<V> = Box<dyn FnMut(&str) -> V>;

pub struct GexfImporter<V> {
    vertex_factory: Option<VertexFactory<V>>,
}

impl<V> GexfImporter<V> {
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
        }
    }
    pub fn set_vertex_factory(&mut self, factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(factory));
    }
}
impl<V> Default for GexfImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, G> GraphImporter<G> for GexfImporter<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let (nodes, edges) = parse(input)?;
        let mut vertices = HashMap::new();
        for node in nodes {
            if vertices.contains_key(&node.id) {
                return Err(IoError::Import(format!(
                    "duplicate GEXF node id {:?}",
                    node.id
                )));
            }
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(&node.id);
                if !graph.add_vertex(vertex.clone()) {
                    return Err(IoError::Import(
                        "GEXF vertex factory produced duplicate".into(),
                    ));
                }
                vertex
            } else {
                graph.add_new_vertex()?
            };
            vertices.insert(node.id, vertex);
        }
        for edge in edges {
            let source = vertices.get(&edge.source).ok_or_else(|| {
                IoError::Import(format!("GEXF source {:?} does not exist", edge.source))
            })?;
            let target = vertices.get(&edge.target).ok_or_else(|| {
                IoError::Import(format!("GEXF target {:?} does not exist", edge.target))
            })?;
            let inserted = graph
                .add_edge(source, target)?
                .ok_or_else(|| IoError::Import("graph rejected GEXF edge".into()))?;
            if graph.graph_type().weighted
                && let Some(weight) = edge.weight
            {
                graph.set_edge_weight(&inserted, weight)?;
            }
        }
        Ok(())
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub struct GexfExporter<V, E> {
    base: BaseExporter<V, E>,
    export_weights: bool,
}

impl<V, E> GexfExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |vertex| ids.id(vertex)),
            export_weights: true,
        }
    }
    pub fn set_export_edge_weights(&mut self, enabled: bool) {
        self.export_weights = enabled;
    }
}
impl<V, E> Default for GexfExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for GexfExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        writeln!(output, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(
            output,
            "<gexf xmlns=\"http://www.gexf.net/1.2draft\" version=\"1.2\">"
        )?;
        writeln!(
            output,
            "  <graph mode=\"static\" defaultedgetype=\"{}\">",
            if graph.graph_type().directed {
                "directed"
            } else {
                "undirected"
            }
        )?;
        writeln!(output, "    <nodes>")?;
        let mut ids = IndexMap::new();
        for vertex in graph.vertices() {
            let id = self.base.vertex_id(vertex)?;
            ids.insert(vertex.clone(), id.clone());
            writeln!(
                output,
                "      <node id=\"{}\" label=\"{}\"/>",
                escape(&id),
                escape(&id)
            )?;
        }
        writeln!(output, "    </nodes>\n    <edges>")?;
        for (index, edge) in graph.edges().enumerate() {
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
            write!(
                output,
                "      <edge id=\"{}\" source=\"{}\" target=\"{}\"",
                index,
                escape(&ids[source]),
                escape(&ids[target])
            )?;
            if self.export_weights {
                write!(
                    output,
                    " weight=\"{}\"",
                    format_f64(graph.edge_weight(edge).unwrap_or(1.0))
                )?;
            }
            writeln!(output, "/>")?;
        }
        writeln!(output, "    </edges>\n  </graph>\n</gexf>")?;
        Ok(())
    }
}
