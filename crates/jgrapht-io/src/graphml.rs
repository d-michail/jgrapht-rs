//! GraphML import and export using a streaming XML reader.

use crate::exporter::format_f64;
use crate::{
    Attribute, AttributeMap, AttributeType, BaseExporter, GraphExporter, GraphImporter,
    IntegerIdProvider, IoError, Result,
};
use indexmap::IndexMap;
use jgrapht::{Graph, GraphMut};
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
    source: String,
    target: String,
    attributes: AttributeMap,
}

fn parse<R: Read>(input: R) -> Result<(Vec<ParsedNode>, Vec<ParsedEdge>)> {
    let mut reader = Reader::from_reader(BufReader::new(input));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut keys = HashMap::<String, Key>::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node: Option<ParsedNode> = None;
    let mut edge: Option<ParsedEdge> = None;
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
                b"node" => {
                    node = Some(ParsedNode {
                        id: xml_attr(&reader, &element, b"id")?
                            .ok_or_else(|| IoError::Import("GraphML node is missing id".into()))?,
                        attributes: IndexMap::new(),
                    })
                }
                b"edge" => {
                    edge = Some(ParsedEdge {
                        source: xml_attr(&reader, &element, b"source")?.ok_or_else(|| {
                            IoError::Import("GraphML edge is missing source".into())
                        })?,
                        target: xml_attr(&reader, &element, b"target")?.ok_or_else(|| {
                            IoError::Import("GraphML edge is missing target".into())
                        })?,
                        attributes: IndexMap::new(),
                    })
                }
                b"data" => {
                    data_key = xml_attr(&reader, &element, b"key")?;
                    text.clear();
                }
                _ => {}
            },
            Event::Empty(element) => match element.local_name().as_ref() {
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
                    attributes: IndexMap::new(),
                }),
                b"edge" => edges.push(ParsedEdge {
                    source: xml_attr(&reader, &element, b"source")?
                        .ok_or_else(|| IoError::Import("GraphML edge is missing source".into()))?,
                    target: xml_attr(&reader, &element, b"target")?
                        .ok_or_else(|| IoError::Import("GraphML edge is missing target".into()))?,
                    attributes: IndexMap::new(),
                }),
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
                        if let Some(node) = node.as_mut() {
                            node.attributes.insert(key.name, attribute);
                        } else if let Some(edge) = edge.as_mut() {
                            edge.attributes.insert(key.name, attribute);
                        }
                    }
                }
                b"node" => {
                    if let Some(mut value) = node.take() {
                        for key in keys
                            .values()
                            .filter(|key| matches!(key.target.as_str(), "node" | "all"))
                        {
                            if let Some(default) = &key.default {
                                value
                                    .attributes
                                    .entry(key.name.clone())
                                    .or_insert_with(|| Attribute::new(default, key.kind));
                            }
                        }
                        nodes.push(value);
                    }
                }
                b"edge" => {
                    if let Some(mut value) = edge.take() {
                        for key in keys
                            .values()
                            .filter(|key| matches!(key.target.as_str(), "edge" | "all"))
                        {
                            if let Some(default) = &key.default {
                                value
                                    .attributes
                                    .entry(key.name.clone())
                                    .or_insert_with(|| Attribute::new(default, key.kind));
                            }
                        }
                        edges.push(value);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok((nodes, edges))
}

type VertexFactory<V> = Box<dyn FnMut(&str, &AttributeMap) -> V>;

pub struct GraphMlImporter<V> {
    vertex_factory: Option<VertexFactory<V>>,
    edge_weight_attribute: String,
}

impl<V> GraphMlImporter<V> {
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
            edge_weight_attribute: "weight".into(),
        }
    }
    pub fn set_vertex_factory(&mut self, mut factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(move |id, _| factory(id)));
    }
    pub fn set_vertex_with_attributes_factory(
        &mut self,
        factory: impl FnMut(&str, &AttributeMap) -> V + 'static,
    ) {
        self.vertex_factory = Some(Box::new(factory));
    }
    pub fn set_edge_weight_attribute_name(&mut self, name: impl Into<String>) {
        self.edge_weight_attribute = name.into();
    }
}

impl<V> Default for GraphMlImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, G> GraphImporter<G> for GraphMlImporter<V>
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
                    "duplicate GraphML node id {:?}",
                    node.id
                )));
            }
            let vertex = if let Some(factory) = self.vertex_factory.as_mut() {
                let vertex = factory(&node.id, &node.attributes);
                if !graph.add_vertex(vertex.clone()) {
                    return Err(IoError::Import(
                        "GraphML vertex factory produced duplicate".into(),
                    ));
                }
                vertex
            } else {
                graph.add_new_vertex()?
            };
            vertices.insert(node.id, vertex);
        }
        for edge in edges {
            let from = vertices.get(&edge.source).ok_or_else(|| {
                IoError::Import(format!("GraphML source {:?} does not exist", edge.source))
            })?;
            let to = vertices.get(&edge.target).ok_or_else(|| {
                IoError::Import(format!("GraphML target {:?} does not exist", edge.target))
            })?;
            let inserted = graph
                .add_edge(from, to)?
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

pub struct GraphMlExporter<V, E> {
    base: BaseExporter<V, E>,
    export_edge_weights: bool,
}

impl<V, E> GraphMlExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self {
            base: BaseExporter::new(move |vertex| ids.id(vertex)),
            export_edge_weights: true,
        }
    }
    pub fn set_export_edge_weights(&mut self, enabled: bool) {
        self.export_edge_weights = enabled;
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
            "<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">"
        )?;
        if self.export_edge_weights {
            writeln!(
                output,
                "  <key id=\"edge_weight\" for=\"edge\" attr.name=\"weight\" attr.type=\"double\"/>"
            )?;
        }
        writeln!(
            output,
            "  <graph id=\"G\" edgedefault=\"{}\">",
            if graph.graph_type().directed {
                "directed"
            } else {
                "undirected"
            }
        )?;
        let mut ids = IndexMap::new();
        for vertex in graph.vertices() {
            let id = self.base.vertex_id(vertex)?;
            ids.insert(vertex.clone(), id.clone());
            writeln!(output, "    <node id=\"{}\"/>", escape_xml(&id))?;
        }
        for (index, edge) in graph.edges().enumerate() {
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
            if self.export_edge_weights {
                writeln!(
                    output,
                    "    <edge id=\"e{}\" source=\"{}\" target=\"{}\"><data key=\"edge_weight\">{}</data></edge>",
                    index + 1,
                    escape_xml(&ids[source]),
                    escape_xml(&ids[target]),
                    format_f64(graph.edge_weight(edge).unwrap_or(1.0))
                )?;
            } else {
                writeln!(
                    output,
                    "    <edge id=\"e{}\" source=\"{}\" target=\"{}\"/>",
                    index + 1,
                    escape_xml(&ids[source]),
                    escape_xml(&ids[target])
                )?;
            }
        }
        writeln!(output, "  </graph>\n</graphml>")?;
        Ok(())
    }
}
