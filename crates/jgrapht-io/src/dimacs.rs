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

//! DIMACS challenge graph formats.
//!
//! Several DIMACS implementation challenges defined closely related graph
//! formats. All use a `p` problem header followed by `e` or `a` edge records;
//! this module also accepts an optional floating-point edge weight.

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{
    BaseExporter, EventDrivenImporter, GraphExporter, GraphImporter, ImportEvent,
    IntegerIdProvider, IoError, Result,
};
use jgrapht::{Graph, GraphMut};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]

/// A supported DIMACS challenge dialect.
pub enum DimacsFormat {
    /// The shortest-path challenge format (`p sp`, with `a` arc records).
    ShortestPath,
    /// The maximum-clique challenge format (`p edge`, with `e` edge records).
    #[default]
    MaxClique,
    /// The coloring challenge format (`p col`, with `e` edge records).
    Coloring,
}

impl DimacsFormat {
    /// Returns the problem token written in the `p` header.
    pub const fn problem(self) -> &'static str {
        match self {
            Self::ShortestPath => "sp",
            Self::MaxClique => "edge",
            Self::Coloring => "col",
        }
    }

    /// Returns the record token written before each edge or arc.
    pub const fn edge_descriptor(self) -> &'static str {
        match self {
            Self::ShortestPath => "a",
            Self::MaxClique | Self::Coloring => "e",
        }
    }
}

/// Parsed DIMACS edge `(source, target, optional weight)`.
pub type DimacsEdge = (usize, usize, Option<f64>);

#[derive(Debug, Clone)]
/// Imports DIMACS records as counts and edge events.
///
/// The importer emits the declared vertex and edge counts, followed by one
/// [`DimacsEdge`] event for each `e` or `a` record. Optional DIMACS descriptors
/// are ignored. By default, identifiers encountered in edge records are
/// renumbered consecutively from zero.
pub struct DimacsEventDrivenImporter {
    zero_based_numbering: bool,
    renumber_vertices: bool,
}

impl Default for DimacsEventDrivenImporter {
    fn default() -> Self {
        Self {
            zero_based_numbering: true,
            renumber_vertices: true,
        }
    }
}

impl DimacsEventDrivenImporter {
    /// Creates an importer with consecutive zero-based renumbering enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether emitted identifiers use zero-based numbering.
    ///
    /// DIMACS files conventionally number vertices from one. When renumbering
    /// is enabled, this controls whether newly assigned identifiers begin at
    /// zero or one. When renumbering is disabled, enabling this subtracts one
    /// from the file identifiers.
    pub fn zero_based_numbering(mut self, enabled: bool) -> Self {
        self.zero_based_numbering = enabled;
        self
    }

    /// Sets whether vertices are renumbered in first-encounter order.
    ///
    /// Disabling this option preserves the file identifiers, subject to the
    /// configured zero-based-numbering conversion.
    pub fn renumber_vertices(mut self, enabled: bool) -> Self {
        self.renumber_vertices = enabled;
        self
    }
}

impl EventDrivenImporter for DimacsEventDrivenImporter {
    type Vertex = usize;
    type Edge = DimacsEdge;

    fn import_events<R, F>(&mut self, input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        let reader = BufReader::new(input);
        let mut lines = reader.lines().enumerate();
        emit(ImportEvent::Start)?;

        let mut header = None;
        for (index, line) in lines.by_ref() {
            let line = line?;
            let text = line.trim();
            if text.is_empty() || text.starts_with('c') || text.starts_with('%') {
                continue;
            }
            let columns: Vec<_> = text.split_whitespace().collect();
            if columns.first() != Some(&"p") || columns.len() < 4 {
                return Err(IoError::Import(format!(
                    "DIMACS line {}: failed to read problem header",
                    index + 1
                )));
            }
            let vertices = columns[2].parse::<isize>().map_err(|_| {
                IoError::Import(format!(
                    "DIMACS line {}: invalid number of vertices",
                    index + 1
                ))
            })?;
            let edges = columns[3].parse::<usize>().map_err(|_| {
                IoError::Import(format!(
                    "DIMACS line {}: invalid number of edges",
                    index + 1
                ))
            })?;
            if vertices < 0 {
                return Err(IoError::Import(format!(
                    "DIMACS line {}: negative number of vertices",
                    index + 1
                )));
            }
            header = Some((vertices as usize, edges));
            break;
        }
        let (vertex_count, edge_count) =
            header.ok_or_else(|| IoError::Import("DIMACS problem header is missing".into()))?;
        emit(ImportEvent::VertexCount(vertex_count))?;
        emit(ImportEvent::EdgeCount(edge_count))?;

        let mut vertex_map = HashMap::<usize, usize>::new();
        let mut next_id = usize::from(!self.zero_based_numbering);
        let mut parsed_edges = 0usize;
        for (index, line) in lines {
            let line = line?;
            let text = line.trim();
            if text.is_empty() || text.starts_with('c') || text.starts_with('%') {
                continue;
            }
            let columns: Vec<_> = text.split_whitespace().collect();
            if !matches!(columns.first(), Some(&"e" | &"a")) {
                continue;
            }
            if columns.len() < 3 {
                return Err(IoError::Import(format!(
                    "DIMACS line {}: edge requires source and target",
                    index + 1
                )));
            }
            let source = columns[1].parse::<usize>().map_err(|_| {
                IoError::Import(format!("DIMACS line {}: invalid edge source", index + 1))
            })?;
            let target = columns[2].parse::<usize>().map_err(|_| {
                IoError::Import(format!("DIMACS line {}: invalid edge target", index + 1))
            })?;
            let map_vertex =
                |id: usize, vertex_map: &mut HashMap<usize, usize>, next_id: &mut usize| {
                    if self.renumber_vertices {
                        *vertex_map.entry(id).or_insert_with(|| {
                            let mapped = *next_id;
                            *next_id += 1;
                            mapped
                        })
                    } else if self.zero_based_numbering {
                        id.saturating_sub(1)
                    } else {
                        id
                    }
                };
            let source = map_vertex(source, &mut vertex_map, &mut next_id);
            let target = map_vertex(target, &mut vertex_map, &mut next_id);
            let weight = columns.get(3).and_then(|value| value.parse().ok());
            emit(ImportEvent::Edge((source, target, weight)))?;
            parsed_edges += 1;
        }
        if parsed_edges != edge_count {
            return Err(IoError::Import(format!(
                "DIMACS header declares {edge_count} edges but {parsed_edges} were read"
            )));
        }
        emit(ImportEvent::End)
    }
}

/// A fallible vertex factory, called with the vertex number of the input.
type VertexFactory<V, Id> = Box<dyn FnMut(Id) -> std::result::Result<V, crate::CallbackError>>;

/// Imports a DIMACS graph into a mutable graph.
///
/// Vertices are created for the one-based identifier range declared by the
/// problem header. The destination graph must support all loops and parallel
/// edges present in the input. For a weighted destination, omitted weights are
/// replaced with the configured default weight.
pub struct DimacsImporter<V> {
    default_weight: f64,
    vertex_factory: Option<VertexFactory<V, usize>>,
}

impl<V> Default for DimacsImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> DimacsImporter<V> {
    /// Creates an importer whose default edge weight is `1.0`.
    pub fn new() -> Self {
        Self::with_default_weight(1.0)
    }

    /// Creates an importer using `default_weight` for unweighted edge records.
    pub fn with_default_weight(default_weight: f64) -> Self {
        Self {
            default_weight,
            vertex_factory: None,
        }
    }

    /// Sets a factory which creates vertices from one-based DIMACS identifiers.
    ///
    /// Without a custom factory, vertices are obtained from the destination
    /// graph's vertex supplier.
    pub fn set_vertex_factory(&mut self, mut factory: impl FnMut(usize) -> V + 'static) {
        self.vertex_factory = Some(Box::new(move |id| Ok(factory(id))));
    }

    /// Like [`set_vertex_factory`](Self::set_vertex_factory) for a factory that can fail, for
    /// example when it parses the identifier. Its error stops the import and is returned as
    /// [`IoError::Callback`] with the factory's error as the source.
    pub fn try_set_vertex_factory(
        &mut self,
        factory: impl FnMut(usize) -> std::result::Result<V, crate::CallbackError> + 'static,
    ) {
        self.vertex_factory = Some(Box::new(factory));
    }
}

impl<V, G> GraphImporter<G> for DimacsImporter<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let mut parser = DimacsEventDrivenImporter::new()
            .renumber_vertices(false)
            .zero_based_numbering(false);
        let mut vertices = Vec::new();
        let default_weight = self.default_weight;
        let factory = &mut self.vertex_factory;
        parser.import_events(input, |event| {
            match event {
                ImportEvent::VertexCount(count) => {
                    for id in 1..=count {
                        let vertex = if let Some(factory) = factory.as_mut() {
                            let vertex = factory(id).map_err(IoError::Callback)?;
                            if !graph.add_vertex(vertex.clone()) {
                                return Err(IoError::Import(format!(
                                    "vertex factory produced duplicate vertex for DIMACS id {id}"
                                )));
                            }
                            vertex
                        } else {
                            graph.add_new_vertex()?
                        };
                        vertices.push(vertex);
                    }
                }
                ImportEvent::Edge((source, target, weight)) => {
                    let from = vertices.get(source.wrapping_sub(1)).ok_or_else(|| {
                        IoError::Import(format!("DIMACS node {source} does not exist"))
                    })?;
                    let to = vertices.get(target.wrapping_sub(1)).ok_or_else(|| {
                        IoError::Import(format!("DIMACS node {target} does not exist"))
                    })?;
                    let edge = graph.add_edge(from, to)?.ok_or_else(|| {
                        IoError::Import(format!("graph rejected DIMACS edge {source} -> {target}"))
                    })?;
                    if graph.graph_type().weighted {
                        graph.set_edge_weight(&edge, weight.unwrap_or(default_weight))?;
                    }
                }
                _ => {}
            }
            Ok(())
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Options which modify DIMACS export behavior.
pub enum DimacsParameter {
    /// Append each graph edge's weight to its DIMACS record.
    ExportEdgeWeights,
}

/// Exports a graph in one of the supported DIMACS challenge formats.
///
/// The maximum-clique dialect is used by default, and vertices receive
/// sequential integer identifiers unless a provider is supplied.
pub struct DimacsExporter<V, E> {
    base: BaseExporter<V, E>,
    format: DimacsFormat,
    export_edge_weights: bool,
}

impl<V, E> DimacsExporter<V, E>
where
    V: Clone + Eq + std::hash::Hash + 'static,
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
            format: DimacsFormat::default(),
            export_edge_weights: false,
        }
    }

    /// Returns the output DIMACS dialect.
    pub const fn format(&self) -> DimacsFormat {
        self.format
    }

    /// Sets the output DIMACS dialect.
    pub fn set_format(&mut self, format: DimacsFormat) {
        self.format = format;
    }

    /// Returns whether an export option is enabled.
    pub const fn is_parameter(&self, parameter: DimacsParameter) -> bool {
        match parameter {
            DimacsParameter::ExportEdgeWeights => self.export_edge_weights,
        }
    }

    /// Enables or disables an export option.
    pub fn set_parameter(&mut self, parameter: DimacsParameter, enabled: bool) {
        match parameter {
            DimacsParameter::ExportEdgeWeights => self.export_edge_weights = enabled,
        }
    }
}

impl<V, E> Default for DimacsExporter<V, E>
where
    V: Clone + Eq + std::hash::Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for DimacsExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + std::hash::Hash + 'static,
    E: Clone + Eq + std::hash::Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        writeln!(output, "c")?;
        writeln!(output, "c SOURCE: Generated using the JGraphT library")?;
        writeln!(output, "c")?;
        writeln!(
            output,
            "p {} {} {}",
            self.format.problem(),
            graph.vertex_count(),
            graph.edge_count()
        )?;
        for edge in graph.edges() {
            let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
            let source = self.base.vertex_id(source)?;
            let target = self.base.vertex_id(target)?;
            write!(
                output,
                "{} {source} {target}",
                self.format.edge_descriptor()
            )?;
            if self.export_edge_weights {
                let weight = graph
                    .edge_weight(edge)
                    .ok_or_else(|| IoError::Export("edge has no weight".into()))?;
                write!(output, " {}", format_f64(weight))?;
            }
            writeln!(output)?;
        }
        Ok(())
    }
}
