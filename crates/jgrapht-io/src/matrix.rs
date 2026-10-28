// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Sparse adjacency and Laplacian matrix exporters.
//!
//! Each nonzero matrix entry is written on its own line as
//! `row_id column_id value`, suitable for ingestion by matrix-processing
//! software.

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{BaseExporter, GraphExporter, IntegerIdProvider, IoError, Result};
use indexmap::{IndexMap, IndexSet};
use jgrapht::GraphExt;
use jgrapht::IncidenceGraph;
use std::hash::Hash;
use std::io::Write;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// A sparse matrix representation supported by [`MatrixExporter`].
pub enum MatrixFormat {
    /// The adjacency matrix.
    ///
    /// Each value is the number of edges between the corresponding vertices.
    /// Undirected matrices are symmetric, and a loop contributes two to its
    /// diagonal entry.
    #[default]
    SparseAdjacency,
    /// The combinatorial Laplacian matrix of an undirected graph.
    SparseLaplacian,
    /// The normalized Laplacian matrix of an undirected graph.
    SparseNormalizedLaplacian,
}

/// Exports a graph as a sparse plain-text matrix.
///
/// Vertices receive sequential integer identifiers by default. Adjacency
/// matrices support directed and undirected graphs; both Laplacian formats
/// require an undirected graph.
pub struct MatrixExporter<V, E> {
    base: BaseExporter<V, E>,
    format: MatrixFormat,
}

impl<V, E> MatrixExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    /// Creates an exporter for `format` with sequential integer vertex identifiers.
    pub fn new(format: MatrixFormat) -> Self {
        let mut ids = IntegerIdProvider::new();
        Self::with_vertex_id_provider(format, move |vertex| ids.id(vertex))
    }

    /// Creates an exporter for `format` using the supplied vertex identifier provider.
    pub fn with_vertex_id_provider(
        format: MatrixFormat,
        provider: impl FnMut(&V) -> String + 'static,
    ) -> Self {
        Self {
            base: BaseExporter::new(provider),
            format,
        }
    }

    /// Chooses the matrix that is written: adjacency, Laplacian or normalized Laplacian.
    pub fn set_format(&mut self, format: MatrixFormat) {
        self.format = format;
    }

    /// The matrix that is written.
    pub fn format(&self) -> MatrixFormat {
        self.format
    }
}

impl<V, E> Default for MatrixExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new(MatrixFormat::default())
    }
}

impl<V, E, G> GraphExporter<G> for MatrixExporter<V, E>
where
    G: IncidenceGraph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        if self.format != MatrixFormat::SparseAdjacency && !graph.graph_type().is_undirected() {
            return Err(IoError::Export(
                "Laplacian formats require an undirected graph".into(),
            ));
        }
        let vertices: Vec<_> = graph.vertices_cloned().collect();
        let mut ids = IndexMap::new();
        for vertex in &vertices {
            ids.insert(vertex.clone(), self.base.vertex_id(vertex)?);
        }
        let id = |vertex: &V| {
            ids.get(vertex).ok_or_else(|| {
                IoError::Export("an edge endpoint is not a vertex of the graph".into())
            })
        };
        match self.format {
            MatrixFormat::SparseAdjacency => {
                for from in &vertices {
                    let edges: Vec<_> = if graph.graph_type().directed {
                        graph.out_edges_of(from).collect()
                    } else {
                        graph.edges_of(from).collect()
                    };
                    let mut counts = IndexMap::<V, usize>::new();
                    for edge in edges {
                        let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
                        let to = if source == from { target } else { source };
                        let amount = if to == from { 2 } else { 1 };
                        *counts.entry(to.clone()).or_default() += amount;
                    }
                    for (to, count) in counts {
                        writeln!(output, "{} {} {count}", ids[from], id(&to)?)?;
                    }
                }
            }
            MatrixFormat::SparseLaplacian => {
                for from in &vertices {
                    let mut neighbors = Vec::new();
                    for edge in graph.edges_of(from) {
                        let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
                        neighbors.push(if source == from { target } else { source });
                    }
                    writeln!(output, "{} {} {}", ids[from], ids[from], neighbors.len())?;
                    for to in neighbors {
                        writeln!(output, "{} {} -1", ids[from], id(to)?)?;
                    }
                }
            }
            MatrixFormat::SparseNormalizedLaplacian => {
                for from in &vertices {
                    let mut neighbors = IndexSet::new();
                    for edge in graph.edges_of(from) {
                        let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
                        neighbors.insert(if source == from {
                            target.clone()
                        } else {
                            source.clone()
                        });
                    }
                    if neighbors.is_empty() {
                        writeln!(output, "{} {} 0", ids[from], ids[from])?;
                    } else {
                        writeln!(output, "{} {} 1", ids[from], ids[from])?;
                        for to in neighbors {
                            let value = -1.0
                                / ((graph.degree_of(from) * graph.degree_of(&to)) as f64).sqrt();
                            writeln!(output, "{} {} {}", ids[from], id(&to)?, format_f64(value))?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
