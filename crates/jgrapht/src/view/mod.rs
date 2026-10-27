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

//! Graph views: wrappers implementing the graph traits over another graph.
//!
//! A view owns its inner graph value `G`. Pass `&graph` for a read-only view, or `&mut graph` to let
//! the view write through to the base (mutable references implement the graph traits too), the way
//! JGraphT's delegating graphs do.

mod capability;
mod subgraph;
mod union;
mod weights;

pub use subgraph::{AsSubgraph, MaskSubgraph};
pub use union::{GraphUnion, WeightCombiner};
pub use weights::{AsUnweighted, AsWeighted};

use crate::graph::{DirectedGraph, Graph, GraphMut, UndirectedGraph};
use crate::{GraphError, GraphType, Result};

/// Forwards the listed read-only [`Graph`] methods to an inner expression. `$s` is the receiver
/// identifier (always `self`), `$inner` the expression yielding the inner graph.
macro_rules! forward_graph {
    ($s:ident => $inner:expr; $($m:ident),* $(,)?) => { $( forward_graph!(@one $s => $inner; $m); )* };
    (@one $s:ident => $inner:expr; graph_type) => {
        fn graph_type(&$s) -> GraphType { $inner.graph_type() }
    };
    (@one $s:ident => $inner:expr; vertices) => {
        fn vertices(&$s) -> impl Iterator<Item = &<Self as Graph>::Vertex> { $inner.vertices() }
    };
    (@one $s:ident => $inner:expr; edges) => {
        fn edges(&$s) -> impl Iterator<Item = &<Self as Graph>::Edge> { $inner.edges() }
    };
    (@one $s:ident => $inner:expr; vertex_count) => {
        fn vertex_count(&$s) -> usize { $inner.vertex_count() }
    };
    (@one $s:ident => $inner:expr; edge_count) => {
        fn edge_count(&$s) -> usize { $inner.edge_count() }
    };
    (@one $s:ident => $inner:expr; contains_vertex) => {
        fn contains_vertex(&$s, v: &<Self as Graph>::Vertex) -> bool { $inner.contains_vertex(v) }
    };
    (@one $s:ident => $inner:expr; contains_edge) => {
        fn contains_edge(&$s, e: &<Self as Graph>::Edge) -> bool { $inner.contains_edge(e) }
    };
    (@one $s:ident => $inner:expr; edge_source) => {
        fn edge_source(&$s, e: &<Self as Graph>::Edge) -> Option<&<Self as Graph>::Vertex> { $inner.edge_source(e) }
    };
    (@one $s:ident => $inner:expr; edge_target) => {
        fn edge_target(&$s, e: &<Self as Graph>::Edge) -> Option<&<Self as Graph>::Vertex> { $inner.edge_target(e) }
    };
    (@one $s:ident => $inner:expr; edge_weight) => {
        fn edge_weight(&$s, e: &<Self as Graph>::Edge) -> Option<f64> { $inner.edge_weight(e) }
    };
    (@one $s:ident => $inner:expr; is_edge_directed) => {
        fn is_edge_directed(&$s, e: &<Self as Graph>::Edge) -> bool { $inner.is_edge_directed(e) }
    };
    (@one $s:ident => $inner:expr; edges_between) => {
        fn edges_between<'a>(&'a $s, u: &'a <Self as Graph>::Vertex, v: &'a <Self as Graph>::Vertex)
            -> impl Iterator<Item = &'a <Self as Graph>::Edge> { $inner.edges_between(u, v) }
    };
}
pub(crate) use forward_graph;
pub(crate) use capability::forward_incidence;

/// Forwards mutation to an inner graph.
macro_rules! forward_graph_mut {
    ($s:ident => $inner:expr; $($m:ident),* $(,)?) => { $( forward_graph_mut!(@one $s => $inner; $m); )* };
    (@one $s:ident => $inner:expr; add_vertex) => {
        fn add_vertex(&mut $s, v: <Self as Graph>::Vertex) -> bool { $inner.add_vertex(v) }
    };
    (@one $s:ident => $inner:expr; remove_vertex) => {
        fn remove_vertex(&mut $s, v: &<Self as Graph>::Vertex) -> bool { $inner.remove_vertex(v) }
    };
    (@one $s:ident => $inner:expr; add_edge_with) => {
        fn add_edge_with(&mut $s, u: &<Self as Graph>::Vertex, v: &<Self as Graph>::Vertex, e: <Self as Graph>::Edge) -> Result<bool> {
            $inner.add_edge_with(u, v, e)
        }
    };
    (@one $s:ident => $inner:expr; remove_edge) => {
        fn remove_edge(&mut $s, e: &<Self as Graph>::Edge) -> bool { $inner.remove_edge(e) }
    };
    (@one $s:ident => $inner:expr; set_edge_weight) => {
        fn set_edge_weight(&mut $s, e: &<Self as Graph>::Edge, w: f64) -> Result<()> { $inner.set_edge_weight(e, w) }
    };
    (@one $s:ident => $inner:expr; new_edge) => {
        fn new_edge(&mut $s) -> Result<<Self as Graph>::Edge> { $inner.new_edge() }
    };
    (@one $s:ident => $inner:expr; new_vertex) => {
        fn new_vertex(&mut $s) -> Result<<Self as Graph>::Vertex> { $inner.new_vertex() }
    };
}
pub(crate) use forward_graph_mut;

impl<G: Graph + ?Sized> Graph for &G {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    forward_graph!(self => (**self); graph_type, vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, edge_weight, is_edge_directed, edges_between);
}
impl<G: DirectedGraph + ?Sized> DirectedGraph for &G {}
impl<G: UndirectedGraph + ?Sized> UndirectedGraph for &G {}

impl<G: Graph + ?Sized> Graph for &mut G {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    forward_graph!(self => (**self); graph_type, vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, edge_weight, is_edge_directed, edges_between);
}
impl<G: DirectedGraph + ?Sized> DirectedGraph for &mut G {}
impl<G: UndirectedGraph + ?Sized> UndirectedGraph for &mut G {}
impl<G: GraphMut + ?Sized> GraphMut for &mut G {
    forward_graph_mut!(self => (**self); add_vertex, remove_vertex, add_edge_with, remove_edge, set_edge_weight, new_edge, new_vertex);
}

/// Read-only view: same graph, but not modifiable (`GraphMut` is simply not implemented).
#[derive(Debug, Clone)]
pub struct Unmodifiable<G>(pub G);

impl<G: Graph> Graph for Unmodifiable<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type().as_unmodifiable()
    }
    forward_graph!(self => self.0; vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, edge_weight, is_edge_directed, edges_between);
}
impl<G: DirectedGraph> DirectedGraph for Unmodifiable<G> {}
impl<G: UndirectedGraph> UndirectedGraph for Unmodifiable<G> {}

/// Directed graph with every edge reversed. Mutations write through with endpoints swapped.
///
/// Incidence capabilities are swapped too: the reverse of an outgoing-only graph can enumerate
/// incoming edges only, so it is rejected where outgoing edges are required.
///
/// ```compile_fail,E0277
/// use jgrapht::graph::capability::OutgoingGraph;
/// use jgrapht::graph::sparse::{NoIncoming, SparseIntDirectedGraph};
/// use jgrapht::view::EdgeReversed;
///
/// fn needs_outgoing<G: OutgoingGraph>(_: &G) {}
/// let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(2, &[(0, 1)]).unwrap();
/// needs_outgoing(&EdgeReversed(&g));
/// ```
#[derive(Debug, Clone)]
pub struct EdgeReversed<G>(pub G);

impl<G: DirectedGraph> Graph for EdgeReversed<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    forward_graph!(self => self.0; graph_type, vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_weight, is_edge_directed);
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_target(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_source(e)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.0.edges_between(v, u)
    }
}
impl<G: DirectedGraph> DirectedGraph for EdgeReversed<G> {}
impl<G: DirectedGraph + GraphMut> GraphMut for EdgeReversed<G> {
    forward_graph_mut!(self => self.0; add_vertex, remove_vertex, remove_edge, set_edge_weight, new_edge, new_vertex);
    fn add_edge_with(&mut self, u: &G::Vertex, v: &G::Vertex, e: G::Edge) -> Result<bool> {
        self.0.add_edge_with(v, u, e)
    }
}

/// Undirected view of a directed graph: edge direction is ignored. Edges cannot be added through
/// the view; everything else writes through.
#[derive(Debug, Clone)]
pub struct AsUndirected<G>(pub G);

impl<G: DirectedGraph> Graph for AsUndirected<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type().directed(false)
    }
    fn is_edge_directed(&self, _: &G::Edge) -> bool {
        false
    }
    forward_graph!(self => self.0; vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, edge_weight);
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.0
            .edges_between(u, v)
            .chain(self.0.edges_between(v, u).filter(move |_| u != v))
    }
}
impl<G: DirectedGraph> UndirectedGraph for AsUndirected<G> {}
impl<G: DirectedGraph + GraphMut> GraphMut for AsUndirected<G> {
    forward_graph_mut!(self => self.0; add_vertex, remove_vertex, remove_edge, set_edge_weight, new_vertex);
    fn add_edge_with(&mut self, _: &G::Vertex, _: &G::Vertex, _: G::Edge) -> Result<bool> {
        Err(GraphError::Unsupported(
            "this graph is read-only for edge insertion".into(),
        ))
    }
    fn new_edge(&mut self) -> Result<G::Edge> {
        Err(GraphError::Unsupported(
            "this graph is read-only for edge insertion".into(),
        ))
    }
}
