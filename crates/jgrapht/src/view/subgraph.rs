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

use super::forward_graph;
use crate::graph::GraphExt;
use crate::graph::IncidenceGraph;
use crate::graph::{DirectedGraph, Graph, GraphMut, UndirectedGraph};
use crate::{GraphError, GraphType, Result};
use indexmap::IndexSet;

/// Subgraph that *hides* the vertices and edges selected by two predicates (JGraphT's
/// `MaskSubgraph`: `true` means masked). An edge is hidden if it is masked or either endpoint is.
/// The view is unmodifiable; only edge weights can be written through to the base.
pub struct MaskSubgraph<G, FV, FE> {
    pub(super) graph: G,
    vertex_mask: FV,
    edge_mask: FE,
}

impl<G: Graph, FV: Fn(&G::Vertex) -> bool, FE: Fn(&G::Edge) -> bool> MaskSubgraph<G, FV, FE> {
    /// Creates a new instance.
    pub fn new(graph: G, vertex_mask: FV, edge_mask: FE) -> Self {
        MaskSubgraph {
            graph,
            vertex_mask,
            edge_mask,
        }
    }
    pub(super) fn vertex_ok(&self, v: &G::Vertex) -> bool {
        self.graph.contains_vertex(v) && !(self.vertex_mask)(v)
    }
    pub(super) fn edge_ok(&self, e: &G::Edge) -> bool {
        match (self.graph.edge_source(e), self.graph.edge_target(e)) {
            (Some(s), Some(t)) => {
                !(self.edge_mask)(e) && !(self.vertex_mask)(s) && !(self.vertex_mask)(t)
            }
            _ => false,
        }
    }
}

impl<G: Graph, FV: Fn(&G::Vertex) -> bool, FE: Fn(&G::Edge) -> bool> Graph
    for MaskSubgraph<G, FV, FE>
{
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.graph.graph_type().as_unmodifiable()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.graph.vertices().filter(|v| !(self.vertex_mask)(v))
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.graph.edges().filter(|e| self.edge_ok(e))
    }
    fn vertex_count(&self) -> usize {
        self.vertices().count()
    }
    fn edge_count(&self) -> usize {
        self.edges().count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.vertex_ok(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.edge_ok(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.edge_ok(e).then(|| self.graph.edge_source(e)).flatten()
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.edge_ok(e).then(|| self.graph.edge_target(e)).flatten()
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.edge_ok(e).then(|| self.graph.edge_weight(e)).flatten()
    }
    fn is_edge_directed(&self, e: &G::Edge) -> bool {
        self.graph.is_edge_directed(e)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        let ok = self.vertex_ok(u) && self.vertex_ok(v);
        self.graph
            .edges_between(u, v)
            .filter(move |e| ok && self.edge_ok(e))
    }
}
impl<G: DirectedGraph, FV: Fn(&G::Vertex) -> bool, FE: Fn(&G::Edge) -> bool> DirectedGraph
    for MaskSubgraph<G, FV, FE>
{
}
impl<G: UndirectedGraph, FV: Fn(&G::Vertex) -> bool, FE: Fn(&G::Edge) -> bool> UndirectedGraph
    for MaskSubgraph<G, FV, FE>
{
}

fn unmodifiable<T>() -> Result<T> {
    Err(GraphError::Unmodifiable)
}

impl<G: GraphMut, FV: Fn(&G::Vertex) -> bool, FE: Fn(&G::Edge) -> bool> GraphMut
    for MaskSubgraph<G, FV, FE>
{
    fn add_vertex(&mut self, _: G::Vertex) -> bool {
        false
    }
    fn remove_vertex(&mut self, _: &G::Vertex) -> bool {
        false
    }
    fn add_edge_with(&mut self, _: &G::Vertex, _: &G::Vertex, _: G::Edge) -> Result<bool> {
        unmodifiable()
    }
    fn remove_edge(&mut self, _: &G::Edge) -> bool {
        false
    }
    fn set_edge_weight(&mut self, e: &G::Edge, w: f64) -> Result<()> {
        if !self.edge_ok(e) {
            return Err(GraphError::EdgeNotFound);
        }
        self.graph.set_edge_weight(e, w)
    }
    fn new_edge(&mut self) -> Result<G::Edge> {
        unmodifiable()
    }
    fn new_vertex(&mut self) -> Result<G::Vertex> {
        unmodifiable()
    }
}

/// Subgraph with its own vertex and edge sets, both subsets of the base (JGraphT's `AsSubgraph`).
///
/// Changes to this subgraph never touch the base; elements removed from the base are not tracked
/// (the view borrows the base, so it cannot change underneath it).
pub struct AsSubgraph<G: Graph> {
    pub(super) base: G,
    pub(super) vertices: IndexSet<G::Vertex>,
    pub(super) edges: IndexSet<G::Edge>,
}

impl<G: Graph> AsSubgraph<G> {
    /// `vertex_subset`/`edge_subset` of `None` mean "all vertices" and "all edges between the
    /// chosen vertices". Elements not in the base graph are ignored; edges need both endpoints
    /// chosen.
    pub fn new(
        base: G,
        vertex_subset: Option<Vec<G::Vertex>>,
        edge_subset: Option<Vec<G::Edge>>,
    ) -> Self {
        let vertices: IndexSet<G::Vertex> = match vertex_subset {
            None => base.vertices_cloned().collect(),
            Some(vs) => {
                let wanted: IndexSet<G::Vertex> = vs.into_iter().collect();
                base.vertices()
                    .filter(|v| wanted.contains(*v))
                    .cloned()
                    .collect()
            }
        };
        let both_in = |e: &G::Edge| match base.edge_endpoints_pair(e) {
            Some((s, t)) => vertices.contains(s) && vertices.contains(t),
            None => false,
        };
        let edges: IndexSet<G::Edge> = match edge_subset {
            None => base.edges().filter(|e| both_in(e)).cloned().collect(),
            Some(es) => {
                let wanted: IndexSet<G::Edge> = es.into_iter().collect();
                base.edges()
                    .filter(|e| wanted.contains(*e) && both_in(e))
                    .cloned()
                    .collect()
            }
        };
        AsSubgraph {
            base,
            vertices,
            edges,
        }
    }
    /// The whole base graph.
    pub fn all(base: G) -> Self {
        Self::new(base, None, None)
    }
    /// Vertex-induced subgraph.
    pub fn induced(base: G, vertices: impl IntoIterator<Item = G::Vertex>) -> Self {
        Self::new(base, Some(vertices.into_iter().collect()), None)
    }
    /// Subgraph with explicit vertex and edge sets.
    pub fn with_edges(
        base: G,
        vertices: impl IntoIterator<Item = G::Vertex>,
        edges: impl IntoIterator<Item = G::Edge>,
    ) -> Self {
        Self::new(
            base,
            Some(vertices.into_iter().collect()),
            Some(edges.into_iter().collect()),
        )
    }
    /// Returns the base.
    pub fn base(&self) -> &G {
        &self.base
    }
}

/// Endpoint lookup used while building a subgraph.
trait EdgePair: Graph {
    fn edge_endpoints_pair(&self, e: &Self::Edge) -> Option<(&Self::Vertex, &Self::Vertex)> {
        Some((self.edge_source(e)?, self.edge_target(e)?))
    }
}
impl<G: Graph> EdgePair for G {}

impl<G: Graph> Graph for AsSubgraph<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    forward_graph!(self => self.base; graph_type, edge_weight, is_edge_directed);
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.vertices.iter()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.edges.iter()
    }
    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.vertices.contains(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.edges.contains(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.edges
            .contains(e)
            .then(|| self.base.edge_source(e))
            .flatten()
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.edges
            .contains(e)
            .then(|| self.base.edge_target(e))
            .flatten()
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        let ok = self.vertices.contains(u) && self.vertices.contains(v);
        self.base
            .edges_between(u, v)
            .filter(move |e| ok && self.edges.contains(*e))
    }
}
impl<G: DirectedGraph> DirectedGraph for AsSubgraph<G> {}
impl<G: UndirectedGraph> UndirectedGraph for AsSubgraph<G> {}

impl<G: IncidenceGraph> GraphMut for AsSubgraph<G> {
    fn add_vertex(&mut self, v: G::Vertex) -> bool {
        // vertices outside the base graph are silently refused
        self.base.contains_vertex(&v) && self.vertices.insert(v)
    }
    fn remove_vertex(&mut self, v: &G::Vertex) -> bool {
        if self.vertices.contains(v) {
            let incident: Vec<G::Edge> = self.edges_of(v).cloned().collect();
            for e in incident {
                self.edges.shift_remove(&e);
            }
        }
        self.vertices.shift_remove(v)
    }
    fn add_edge_with(&mut self, u: &G::Vertex, v: &G::Vertex, e: G::Edge) -> Result<bool> {
        if !self.base.contains_edge(&e) {
            return Err(GraphError::InvalidArgument(
                "no such edge in base graph".into(),
            ));
        }
        if !self.vertices.contains(u) || !self.vertices.contains(v) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.edges.insert(e))
    }
    fn add_edge(&mut self, u: &G::Vertex, v: &G::Vertex) -> Result<Option<G::Edge>> {
        if !self.vertices.contains(u) || !self.vertices.contains(v) {
            return Err(GraphError::VertexNotFound);
        }
        let candidates: Vec<G::Edge> = self.base.edges_between(u, v).cloned().collect();
        if candidates.is_empty() {
            return Err(GraphError::InvalidArgument(
                "no such edge in base graph".into(),
            ));
        }
        Ok(candidates
            .into_iter()
            .find(|e| self.edges.insert(e.clone())))
    }
    fn remove_edge(&mut self, e: &G::Edge) -> bool {
        self.edges.shift_remove(e)
    }
    fn set_edge_weight(&mut self, _: &G::Edge, _: f64) -> Result<()> {
        Err(GraphError::Unmodifiable)
    }
    fn new_edge(&mut self) -> Result<G::Edge> {
        Err(GraphError::NoEdgeFactory)
    }
    fn new_vertex(&mut self) -> Result<G::Vertex> {
        Err(GraphError::NoVertexFactory)
    }
}
