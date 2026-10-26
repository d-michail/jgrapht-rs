//! Graph views: lightweight wrappers implementing the graph traits over another graph.
//!
//! A view owns its inner graph value `G`; pass `&graph` to borrow (shared references to graphs
//! implement the read traits too).

mod subgraph;
mod union;
mod weights;

pub use subgraph::{AsSubgraph, Mask, MaskSubgraph};
pub use union::GraphUnion;
pub use weights::{AsUnweighted, AsWeighted};

use crate::GraphType;
use crate::graph::{DirectedGraph, Graph, UndirectedGraph};

impl<G: Graph + ?Sized> Graph for &G {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        (**self).graph_type()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        (**self).vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        (**self).edges()
    }
    fn vertex_count(&self) -> usize {
        (**self).vertex_count()
    }
    fn edge_count(&self) -> usize {
        (**self).edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        (**self).contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        (**self).contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        (**self).edge_source(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        (**self).edge_target(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        (**self).edge_weight(e)
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        (**self).edges_of(v)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        (**self).edges_between(u, v)
    }
}

impl<G: DirectedGraph + ?Sized> DirectedGraph for &G {
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        (**self).in_edges_of(v)
    }
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        (**self).out_edges_of(v)
    }
}

impl<G: UndirectedGraph + ?Sized> UndirectedGraph for &G {}

/// Read-only view: same graph, but not modifiable (`GraphMut` is simply not implemented).
#[derive(Debug, Clone)]
pub struct Unmodifiable<G>(pub G);

impl<G: Graph> Graph for Unmodifiable<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type().as_unmodifiable()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.0.vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.0.edges()
    }
    fn vertex_count(&self) -> usize {
        self.0.vertex_count()
    }
    fn edge_count(&self) -> usize {
        self.0.edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.0.contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.0.contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_source(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_target(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.0.edge_weight(e)
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.edges_of(v)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.0.edges_between(u, v)
    }
}
impl<G: DirectedGraph> DirectedGraph for Unmodifiable<G> {
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.in_edges_of(v)
    }
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.out_edges_of(v)
    }
}
impl<G: UndirectedGraph> UndirectedGraph for Unmodifiable<G> {}

/// Directed graph with every edge reversed.
#[derive(Debug, Clone)]
pub struct EdgeReversed<G>(pub G);

impl<G: DirectedGraph> Graph for EdgeReversed<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.0.vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.0.edges()
    }
    fn vertex_count(&self) -> usize {
        self.0.vertex_count()
    }
    fn edge_count(&self) -> usize {
        self.0.edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.0.contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.0.contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_target(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_source(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.0.edge_weight(e)
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.edges_of(v)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.0.edges_between(v, u)
    }
}
impl<G: DirectedGraph> DirectedGraph for EdgeReversed<G> {
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.out_edges_of(v)
    }
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.in_edges_of(v)
    }
}

/// Undirected view of a directed graph: edge direction is ignored.
#[derive(Debug, Clone)]
pub struct AsUndirected<G>(pub G);

impl<G: DirectedGraph> Graph for AsUndirected<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type().directed(false)
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.0.vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.0.edges()
    }
    fn vertex_count(&self) -> usize {
        self.0.vertex_count()
    }
    fn edge_count(&self) -> usize {
        self.0.edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.0.contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.0.contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_source(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_target(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.0.edge_weight(e)
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.edges_of(v)
    }
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
