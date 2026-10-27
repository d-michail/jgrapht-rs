//! graph6, sparse6, and digraph6 compact graph encodings.

use crate::{EventDrivenImporter, GraphExporter, GraphImporter, ImportEvent, IoError, Result};
use jgrapht::{Graph, GraphMut};
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Graph6Format {
    #[default]
    Graph6,
    Sparse6,
    Digraph6,
}

struct BitReader {
    values: Vec<u8>,
    bit: usize,
}

impl BitReader {
    fn new(bytes: &[u8]) -> Result<Self> {
        if let Some(byte) = bytes.iter().find(|byte| !(63..=126).contains(*byte)) {
            return Err(IoError::Import(format!(
                "graph6 input contains illegal byte {byte}"
            )));
        }
        Ok(Self {
            values: bytes.iter().map(|byte| byte - 63).collect(),
            bit: 0,
        })
    }

    fn remaining(&self) -> usize {
        self.values.len() * 6 - self.bit
    }

    fn read(&mut self, count: usize) -> Result<u64> {
        if self.remaining() < count {
            return Err(IoError::Import("graph6 input is truncated".into()));
        }
        let mut result = 0u64;
        for _ in 0..count {
            let byte = self.bit / 6;
            let offset = self.bit % 6;
            result = (result << 1) | u64::from((self.values[byte] >> (5 - offset)) & 1);
            self.bit += 1;
        }
        Ok(result)
    }

    fn vertex_count(&mut self) -> Result<usize> {
        let first = self.read(6)?;
        let count = if first < 63 {
            first
        } else {
            let second = self.read(6)?;
            if second < 63 {
                let tail = self.read(12)?;
                let value = (second << 12) | tail;
                if value < 63 {
                    return Err(IoError::Import("non-canonical graph6 vertex count".into()));
                }
                value
            } else {
                let value = self.read(36)?;
                if value < 258_048 {
                    return Err(IoError::Import("non-canonical graph6 vertex count".into()));
                }
                value
            }
        };
        usize::try_from(count)
            .map_err(|_| IoError::Import("graph6 vertex count exceeds this platform".into()))
    }
}

#[derive(Default)]
struct BitWriter {
    output: Vec<u8>,
    value: u8,
    used: u8,
}

impl BitWriter {
    fn bit(&mut self, set: bool) {
        if set {
            self.value |= 1 << (5 - self.used);
        }
        self.used += 1;
        if self.used == 6 {
            self.flush();
        }
    }

    fn integer(&mut self, value: u64, bits: usize) {
        for shift in (0..bits).rev() {
            self.bit(value & (1u64 << shift) != 0);
        }
    }

    fn flush(&mut self) {
        if self.used > 0 {
            self.output.push(self.value + 63);
            self.value = 0;
            self.used = 0;
        }
    }

    fn vertex_count(&mut self, count: usize) -> Result<()> {
        let count = count as u64;
        if count <= 62 {
            self.integer(count, 6);
        } else if count <= 258_047 {
            self.integer(63, 6);
            self.integer(count, 18);
        } else if count < (1u64 << 36) {
            self.integer(63, 6);
            self.integer(63, 6);
            self.integer(count, 36);
        } else {
            return Err(IoError::Export(
                "graph6 supports fewer than 2^36 vertices".into(),
            ));
        }
        Ok(())
    }
}

pub struct Graph6EventDrivenImporter;

impl Graph6EventDrivenImporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Graph6EventDrivenImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDrivenImporter for Graph6EventDrivenImporter {
    type Vertex = usize;
    type Edge = (usize, usize);

    fn import_events<R, F>(&mut self, mut input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let line = text.lines().next().unwrap_or_default();
        if line.is_empty() {
            return Err(IoError::Import("graph6 input is empty".into()));
        }
        let (format, encoded) = if let Some(rest) = line.strip_prefix(">>sparse6<<:") {
            (Graph6Format::Sparse6, rest)
        } else if let Some(rest) = line.strip_prefix(">>graph6<<") {
            (Graph6Format::Graph6, rest)
        } else if let Some(rest) = line.strip_prefix(':') {
            (Graph6Format::Sparse6, rest)
        } else if let Some(rest) = line.strip_prefix('&') {
            (Graph6Format::Digraph6, rest)
        } else {
            (Graph6Format::Graph6, line)
        };
        let mut bits = BitReader::new(encoded.as_bytes())?;
        let count = bits.vertex_count()?;
        emit(ImportEvent::Start)?;
        emit(ImportEvent::VertexCount(count))?;
        for vertex in 0..count {
            emit(ImportEvent::Vertex(vertex))?;
        }
        match format {
            Graph6Format::Graph6 => {
                let needed = count.saturating_mul(count.saturating_sub(1)) / 2;
                if bits.remaining() < needed {
                    return Err(IoError::Import("graph6 adjacency data is truncated".into()));
                }
                for high in 0..count {
                    for low in 0..high {
                        if bits.read(1)? == 1 {
                            emit(ImportEvent::Edge((high, low)))?;
                        }
                    }
                }
            }
            Graph6Format::Digraph6 => {
                let needed = count.saturating_mul(count);
                if bits.remaining() < needed {
                    return Err(IoError::Import(
                        "digraph6 adjacency data is truncated".into(),
                    ));
                }
                for source in 0..count {
                    for target in 0..count {
                        if bits.read(1)? == 1 {
                            emit(ImportEvent::Edge((source, target)))?;
                        }
                    }
                }
            }
            Graph6Format::Sparse6 => {
                let width = if count <= 1 {
                    0
                } else {
                    usize::BITS as usize - (count - 1).leading_zeros() as usize
                };
                let mut vertex = 0usize;
                while bits.remaining() > width {
                    let advance = bits.read(1)? == 1;
                    let value = bits.read(width)? as usize;
                    if advance {
                        vertex += 1;
                    }
                    if vertex >= count {
                        break;
                    }
                    if value > vertex {
                        vertex = value;
                    } else {
                        emit(ImportEvent::Edge((value, vertex)))?;
                    }
                }
            }
        }
        emit(ImportEvent::End)
    }
}

pub struct Graph6Importer<V> {
    vertex_factory: Option<Box<dyn FnMut(usize) -> V>>,
}

impl<V> Graph6Importer<V> {
    pub fn new() -> Self {
        Self {
            vertex_factory: None,
        }
    }

    pub fn set_vertex_factory(&mut self, factory: impl FnMut(usize) -> V + 'static) {
        self.vertex_factory = Some(Box::new(factory));
    }
}

impl<V> Default for Graph6Importer<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, G> GraphImporter<G> for Graph6Importer<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let mut parser = Graph6EventDrivenImporter::new();
        let factory = &mut self.vertex_factory;
        let mut vertices = Vec::new();
        parser.import_events(input, |event| {
            match event {
                ImportEvent::Vertex(id) => {
                    let vertex = if let Some(factory) = factory.as_mut() {
                        let vertex = factory(id);
                        if !graph.add_vertex(vertex.clone()) {
                            return Err(IoError::Import(format!(
                                "vertex factory produced a duplicate for graph6 id {id}"
                            )));
                        }
                        vertex
                    } else {
                        graph.add_new_vertex()?
                    };
                    vertices.push(vertex);
                }
                ImportEvent::Edge((source, target)) => {
                    let from = vertices.get(source).ok_or_else(|| {
                        IoError::Import(format!("graph6 node {source} does not exist"))
                    })?;
                    let to = vertices.get(target).ok_or_else(|| {
                        IoError::Import(format!("graph6 node {target} does not exist"))
                    })?;
                    if graph.add_edge(from, to)?.is_none() {
                        return Err(IoError::Import(format!(
                            "graph rejected graph6 edge {source} -> {target}"
                        )));
                    }
                }
                _ => {}
            }
            Ok(())
        })
    }
}

pub struct Graph6Exporter {
    format: Graph6Format,
}

impl Graph6Exporter {
    pub fn new(format: Graph6Format) -> Self {
        Self { format }
    }

    pub const fn format(&self) -> Graph6Format {
        self.format
    }

    pub fn set_format(&mut self, format: Graph6Format) {
        self.format = format;
    }
}

impl Default for Graph6Exporter {
    fn default() -> Self {
        Self::new(Graph6Format::default())
    }
}

impl<G: Graph> GraphExporter<G> for Graph6Exporter {
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        if self.format == Graph6Format::Digraph6 {
            if !graph.graph_type().directed {
                return Err(IoError::Export("digraph6 requires a directed graph".into()));
            }
        } else if !graph.graph_type().is_undirected() {
            return Err(IoError::Export(
                "graph6 and sparse6 require an undirected graph".into(),
            ));
        }
        if self.format == Graph6Format::Graph6 && !graph.graph_type().is_simple() {
            return Err(IoError::Export(
                "graph6 cannot represent loops or multiple edges".into(),
            ));
        }
        let vertices: Vec<_> = graph.vertices().cloned().collect();
        let mut bits = BitWriter::default();
        match self.format {
            Graph6Format::Sparse6 => output.write_all(b":")?,
            Graph6Format::Digraph6 => output.write_all(b"&")?,
            Graph6Format::Graph6 => {}
        }
        bits.vertex_count(vertices.len())?;
        match self.format {
            Graph6Format::Graph6 => {
                for high in 0..vertices.len() {
                    for low in 0..high {
                        bits.bit(graph.contains_edge_between(&vertices[high], &vertices[low]));
                    }
                }
            }
            Graph6Format::Digraph6 => {
                for source in &vertices {
                    for target in &vertices {
                        bits.bit(graph.contains_edge_between(source, target));
                    }
                }
            }
            Graph6Format::Sparse6 => {
                let width = if vertices.len() <= 1 {
                    0
                } else {
                    usize::BITS as usize - (vertices.len() - 1).leading_zeros() as usize
                };
                let mut edges = Vec::new();
                for high in 0..vertices.len() {
                    for low in 0..=high {
                        edges.extend(
                            graph
                                .edges_between(&vertices[low], &vertices[high])
                                .map(|_| (low, high)),
                        );
                    }
                }
                let mut current = 0usize;
                for (low, high) in edges {
                    if high > current + 1 {
                        bits.bit(true);
                        bits.integer(high as u64, width);
                        current = high;
                    } else if high == current + 1 {
                        bits.bit(true);
                        bits.integer(low as u64, width);
                        current += 1;
                    } else {
                        bits.bit(false);
                        bits.integer(low as u64, width);
                    }
                }
                while bits.used != 0 {
                    bits.bit(true);
                }
            }
        }
        bits.flush();
        output.write_all(&bits.output)?;
        Ok(())
    }
}
