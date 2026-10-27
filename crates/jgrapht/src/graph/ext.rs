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

//! Convenience methods derived from the core traits (JGraphT's `Graphs` utility class).

use super::{DirectedGraph, Graph, GraphMut, IncidenceGraph, IncomingGraph, OutgoingGraph};
use crate::Result;

pub trait GraphExt: Graph {
    /// `(source, target)` of an edge.
    fn edge_endpoints(&self, e: &Self::Edge) -> Option<(&Self::Vertex, &Self::Vertex)> {
        Some((self.edge_source(e)?, self.edge_target(e)?))
    }

    /// The endpoint of `e` that is not `v` (`v` itself for a self-loop).
    fn opposite_vertex(&self, e: &Self::Edge, v: &Self::Vertex) -> Option<&Self::Vertex> {
        let (s, t) = self.edge_endpoints(e)?;
        if s == v {
            Some(t)
        } else if t == v {
            Some(s)
        } else {
            None
        }
    }

    /// Adjacent vertices, ignoring direction. Parallel edges yield repeats.
    fn neighbors_of<'a>(&'a self, v: &'a Self::Vertex) -> impl Iterator<Item = &'a Self::Vertex>
    where
        Self: IncidenceGraph,
    {
        self.edges_of(v)
            .filter_map(move |e| self.opposite_vertex(e, v))
    }

    /// Edges leaving `v` with the vertex they lead to. Directed graphs follow edge direction,
    /// undirected graphs yield every incident edge.
    fn outgoing_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = (&'a Self::Edge, &'a Self::Vertex)>
    where
        Self: OutgoingGraph,
    {
        self.out_edges_of(v).filter_map(move |e| {
            let (s, t) = self.edge_endpoints(e)?;
            if self.is_edge_directed(e) {
                (s == v).then_some((e, t))
            } else {
                Some((e, if s == v { t } else { s }))
            }
        })
    }

    /// Edges entering `v` with the vertex they come from (all incident edges if undirected).
    fn incoming_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = (&'a Self::Edge, &'a Self::Vertex)>
    where
        Self: IncomingGraph,
    {
        self.in_edges_of(v).filter_map(move |e| {
            let (s, t) = self.edge_endpoints(e)?;
            if self.is_edge_directed(e) {
                (t == v).then_some((e, s))
            } else {
                Some((e, if s == v { t } else { s }))
            }
        })
    }

    /// Vertices reachable over one edge; repeats for parallel edges.
    fn out_neighbors_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Vertex>
    where
        Self: OutgoingGraph,
    {
        self.outgoing_of(v).map(|(_, n)| n)
    }

    /// Neighbours as a list (with repeats for parallel edges), `Graphs.neighborListOf`.
    fn neighbor_list_of(&self, v: &Self::Vertex) -> Vec<Self::Vertex>
    where
        Self: IncidenceGraph,
    {
        self.neighbors_of(v).cloned().collect()
    }

    /// Distinct neighbours, `Graphs.neighborSetOf`.
    fn neighbor_set_of(&self, v: &Self::Vertex) -> std::collections::HashSet<Self::Vertex>
    where
        Self: IncidenceGraph,
    {
        self.neighbors_of(v).cloned().collect()
    }

    /// Whether `v` is an endpoint of `e` (`Graphs.testIncidence`).
    fn test_incidence(&self, e: &Self::Edge, v: &Self::Vertex) -> bool {
        self.edge_endpoints(e)
            .is_some_and(|(s, t)| s == v || t == v)
    }

    /// Weight of an edge that the graph itself yielded (from `edges()` or an incidence query).
    ///
    /// This is [`Graph::edge_weight`] under the [`Graph`] integrity contract: such an edge has a
    /// weight (`1.0` in an unweighted graph), so the lookup cannot fail.
    ///
    /// # Panics
    ///
    /// If `e` is not an edge of the graph, which a well-formed graph never yields.
    fn weight_of(&self, e: &Self::Edge) -> f64 {
        self.edge_weight(e)
            .expect("Graph integrity: every edge yielded by the graph has a weight")
    }

    /// Sum of all edge weights.
    fn total_weight(&self) -> f64 {
        self.edges().filter_map(|e| self.edge_weight(e)).sum()
    }

    fn is_empty(&self) -> bool {
        self.vertex_count() == 0
    }
}
impl<G: Graph + ?Sized> GraphExt for G {}

pub trait DirectedExt: DirectedGraph {
    /// Successors reached by outgoing edges.
    fn successors_of<'a>(&'a self, v: &'a Self::Vertex) -> impl Iterator<Item = &'a Self::Vertex>
    where
        Self: OutgoingGraph,
    {
        self.out_edges_of(v)
            .filter_map(move |e| self.edge_target(e))
    }

    /// Successors as an owned list.
    fn successor_list_of(&self, v: &Self::Vertex) -> Vec<Self::Vertex>
    where
        Self: OutgoingGraph,
    {
        self.successors_of(v).cloned().collect()
    }

    /// Predecessors reached by incoming edges.
    fn predecessors_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Vertex>
    where
        Self: IncomingGraph,
    {
        self.in_edges_of(v).filter_map(move |e| self.edge_source(e))
    }

    /// Predecessors as an owned list.
    fn predecessor_list_of(&self, v: &Self::Vertex) -> Vec<Self::Vertex>
    where
        Self: IncomingGraph,
    {
        self.predecessors_of(v).cloned().collect()
    }

    /// Every vertex from which `v` is reachable (excluding `v` unless it lies on a cycle).
    fn ancestors_of(&self, v: &Self::Vertex) -> std::collections::HashSet<Self::Vertex>
    where
        Self: IncomingGraph,
    {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![v];
        while let Some(x) = stack.pop() {
            for p in self.predecessors_of(x) {
                if seen.insert(p.clone()) {
                    stack.push(p);
                }
            }
        }
        seen
    }

    /// Every vertex reachable from `v` (excluding `v` unless it lies on a cycle).
    fn descendants_of(&self, v: &Self::Vertex) -> std::collections::HashSet<Self::Vertex>
    where
        Self: OutgoingGraph,
    {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![v];
        while let Some(x) = stack.pop() {
            for p in self.successors_of(x) {
                if seen.insert(p.clone()) {
                    stack.push(p);
                }
            }
        }
        seen
    }

    /// Vertices in a topological order (sources in insertion order first); `None` if cyclic.
    fn topological_order(&self) -> Option<Vec<Self::Vertex>>
    where
        Self: Sized + IncidenceGraph,
    {
        crate::traverse::topological_sort(self).map(|v| v.into_iter().cloned().collect())
    }

    /// Whether `v` has an outgoing edge.
    fn vertex_has_successors(&self, v: &Self::Vertex) -> bool
    where
        Self: OutgoingGraph,
    {
        self.out_edges_of(v).next().is_some()
    }

    /// Whether `v` has an incoming edge.
    fn vertex_has_predecessors(&self, v: &Self::Vertex) -> bool
    where
        Self: IncomingGraph,
    {
        self.in_edges_of(v).next().is_some()
    }
}
impl<G: DirectedGraph + ?Sized> DirectedExt for G {}

pub trait GraphMutExt: GraphMut {
    /// Adds both endpoints if missing, then the edge (`Graphs.addEdgeWithVertices`).
    fn add_edge_with_vertices(
        &mut self,
        u: Self::Vertex,
        v: Self::Vertex,
    ) -> Result<Option<Self::Edge>> {
        self.add_vertex(u.clone());
        self.add_vertex(v.clone());
        self.add_edge(&u, &v)
    }

    /// Adds an edge and sets its weight.
    fn add_weighted_edge(
        &mut self,
        u: &Self::Vertex,
        v: &Self::Vertex,
        w: f64,
    ) -> Result<Option<Self::Edge>> {
        let e = self.add_edge(u, v)?;
        if let Some(e) = &e {
            self.set_edge_weight(e, w)?;
        }
        Ok(e)
    }

    fn add_vertices(&mut self, vs: impl IntoIterator<Item = Self::Vertex>) -> bool {
        vs.into_iter().fold(false, |ch, v| self.add_vertex(v) | ch)
    }

    /// Copies vertices, edges and weights of `src` into `self`, reusing edge objects.
    fn add_graph<G>(&mut self, src: &G) -> Result<bool>
    where
        G: Graph<Vertex = Self::Vertex, Edge = Self::Edge>,
    {
        let mut changed = self.add_vertices(src.vertices().cloned());
        for e in src.edges() {
            let (s, t) = src.edge_endpoints(e).expect("edge of source graph");
            if self.add_edge_with(s, t, e.clone())? {
                changed = true;
                if self.graph_type().weighted
                    && let Some(w) = src.edge_weight(e)
                {
                    self.set_edge_weight(e, w)?;
                }
            }
        }
        Ok(changed)
    }

    /// Adds edges `source -> t` for every target, adding missing vertices first.
    fn add_outgoing_edges(
        &mut self,
        source: &Self::Vertex,
        targets: impl IntoIterator<Item = Self::Vertex>,
    ) -> Result<()> {
        self.add_vertex(source.clone());
        for t in targets {
            self.add_vertex(t.clone());
            self.add_edge(source, &t)?;
        }
        Ok(())
    }

    /// Adds edges `s -> target` for every source, adding missing vertices first.
    fn add_incoming_edges(
        &mut self,
        target: &Self::Vertex,
        sources: impl IntoIterator<Item = Self::Vertex>,
    ) -> Result<()> {
        self.add_vertex(target.clone());
        for s in sources {
            self.add_vertex(s.clone());
            self.add_edge(&s, target)?;
        }
        Ok(())
    }

    /// Adds a reversed copy of `src` (vertices and edges `v -> u` for every `u -> v`).
    fn add_graph_reversed<G>(&mut self, src: &G) -> Result<bool>
    where
        G: Graph<Vertex = Self::Vertex, Edge = Self::Edge>,
    {
        let mut changed = self.add_vertices(src.vertices().cloned());
        for e in src.edges() {
            let (s, t) = src.edge_endpoints(e).expect("edge of source graph");
            if self.add_edge_with(t, s, e.clone())? {
                changed = true;
                if self.graph_type().weighted {
                    if let Some(w) = src.edge_weight(e) {
                        self.set_edge_weight(e, w)?;
                    }
                }
            }
        }
        Ok(changed)
    }

    /// Removes one edge between `u` and `v` and returns it (`removeEdge(u, v)`).
    fn remove_edge_between(&mut self, u: &Self::Vertex, v: &Self::Vertex) -> Option<Self::Edge> {
        let e = self.get_edge(u, v).cloned()?;
        self.remove_edge(&e);
        Some(e)
    }

    /// Removes every edge between `u` and `v` and returns them (`removeAllEdges(u, v)`);
    /// `None` if either vertex is missing.
    fn remove_all_edges(&mut self, u: &Self::Vertex, v: &Self::Vertex) -> Option<Vec<Self::Edge>> {
        if !self.contains_vertex(u) || !self.contains_vertex(v) {
            return None;
        }
        let es: Vec<_> = self.edges_between(u, v).cloned().collect();
        for e in &es {
            self.remove_edge(e);
        }
        Some(es)
    }

    fn remove_edges_between(&mut self, u: &Self::Vertex, v: &Self::Vertex) -> bool {
        let es: Vec<_> = self.edges_between(u, v).cloned().collect();
        es.iter().fold(false, |ch, e| self.remove_edge(e) | ch)
    }
}
impl<G: GraphMut + ?Sized> GraphMutExt for G {}

/// Removes `v` but reconnects every predecessor to every successor
/// (`Graphs.removeVertexAndPreserveConnectivity`). Returns `false` if `v` is absent.
pub fn remove_vertex_and_preserve_connectivity<G: GraphMut + DirectedGraph + IncidenceGraph>(
    g: &mut G,
    v: &G::Vertex,
) -> Result<bool> {
    if !g.contains_vertex(v) {
        return Ok(false);
    }
    if g.vertex_has_predecessors(v) {
        let (preds, succs) = (g.predecessor_list_of(v), g.successor_list_of(v));
        for p in &preds {
            g.add_outgoing_edges(p, succs.iter().cloned())?;
        }
    }
    g.remove_vertex(v);
    Ok(true)
}

/// Applies [`remove_vertex_and_preserve_connectivity`] to each vertex in turn.
pub fn remove_vertices_and_preserve_connectivity<G: GraphMut + DirectedGraph + IncidenceGraph>(
    g: &mut G,
    vs: impl IntoIterator<Item = G::Vertex>,
) -> Result<bool> {
    let mut any = false;
    for v in vs {
        any |= remove_vertex_and_preserve_connectivity(g, &v)?;
    }
    Ok(any)
}
