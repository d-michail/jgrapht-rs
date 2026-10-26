//! Graph traits and the default implementation.

mod base;
mod ext;
pub mod props;

pub use base::{BaseGraph, DefaultEdge, DiGraph, Directed, Direction, UGraph, Undirected};
pub use ext::{
    DirectedExt, GraphExt, GraphMutExt, remove_vertex_and_preserve_connectivity,
    remove_vertices_and_preserve_connectivity,
};

use crate::{GraphType, Result};
use std::hash::Hash;

/// Read-only access to a graph. Vertices and edges are user-provided values.
///
/// Iterator-returning methods borrow from the graph; no intermediate sets are built.
/// For undirected graphs `edge_source`/`edge_target` return the endpoints in insertion order.
pub trait Graph {
    type Vertex: Clone + Eq + Hash;
    type Edge: Clone + Eq + Hash;

    fn graph_type(&self) -> GraphType;

    fn vertices(&self) -> impl Iterator<Item = &Self::Vertex>;
    fn edges(&self) -> impl Iterator<Item = &Self::Edge>;
    fn vertex_count(&self) -> usize;
    fn edge_count(&self) -> usize;

    fn contains_vertex(&self, v: &Self::Vertex) -> bool;
    fn contains_edge(&self, e: &Self::Edge) -> bool;

    fn edge_source(&self, e: &Self::Edge) -> Option<&Self::Vertex>;
    fn edge_target(&self, e: &Self::Edge) -> Option<&Self::Vertex>;
    /// Weight of the edge; `1.0` for unweighted graphs. `None` if the edge does not exist.
    fn edge_weight(&self, e: &Self::Edge) -> Option<f64>;

    /// All edges touching `v` (empty if `v` is absent). Directed graphs yield in- and out-edges,
    /// a self-loop appears once.
    fn edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge>;

    /// Edges from `u` to `v` (either orientation for undirected graphs).
    fn edges_between<'a>(
        &'a self,
        u: &'a Self::Vertex,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Edge>;

    fn get_edge<'a>(&'a self, u: &'a Self::Vertex, v: &'a Self::Vertex) -> Option<&'a Self::Edge> {
        self.edges_between(u, v).next()
    }
    fn contains_edge_between<'a>(&'a self, u: &'a Self::Vertex, v: &'a Self::Vertex) -> bool {
        self.get_edge(u, v).is_some()
    }
    /// Degree; self-loops count twice.
    fn degree_of(&self, v: &Self::Vertex) -> usize {
        self.edges_of(v)
            .map(|e| {
                if self.edge_source(e) == self.edge_target(e) {
                    2
                } else {
                    1
                }
            })
            .sum()
    }
}

/// A graph whose edges have a direction.
pub trait DirectedGraph: Graph {
    fn in_edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge>;
    fn out_edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge>;
    fn in_degree_of(&self, v: &Self::Vertex) -> usize {
        self.in_edges_of(v).count()
    }
    fn out_degree_of(&self, v: &Self::Vertex) -> usize {
        self.out_edges_of(v).count()
    }
}

/// Marker for graphs whose edges are unordered pairs.
pub trait UndirectedGraph: Graph {}

/// Mutation. Failure is reported through [`crate::GraphError`].
pub trait GraphMut: Graph {
    /// Returns `false` if the vertex was already present.
    fn add_vertex(&mut self, v: Self::Vertex) -> bool;
    /// Removes `v` and its incident edges. Returns `false` if absent.
    fn remove_vertex(&mut self, v: &Self::Vertex) -> bool;
    /// Adds the given edge object. `Ok(false)` if the edge already exists, or if a parallel edge
    /// would be created in a graph without multiple edges.
    fn add_edge_with(&mut self, u: &Self::Vertex, v: &Self::Vertex, e: Self::Edge) -> Result<bool>;
    /// Removes an edge, returning `false` if it was absent.
    fn remove_edge(&mut self, e: &Self::Edge) -> bool;
    fn set_edge_weight(&mut self, e: &Self::Edge, w: f64) -> Result<()>;
    /// Produces a fresh edge object from the graph's edge factory.
    fn new_edge(&mut self) -> Result<Self::Edge>;

    /// Creates an edge with the graph's factory. `Ok(None)` mirrors JGraphT returning `null`.
    fn add_edge(&mut self, u: &Self::Vertex, v: &Self::Vertex) -> Result<Option<Self::Edge>> {
        let e = self.new_edge()?;
        Ok(self.add_edge_with(u, v, e.clone())?.then_some(e))
    }
}
