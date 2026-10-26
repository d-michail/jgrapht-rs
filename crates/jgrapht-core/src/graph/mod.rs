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

    /// Edges entering `v`; every incident edge for undirected graphs (`incomingEdgesOf`).
    fn in_edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge> {
        let directed = self.graph_type().directed;
        self.edges_of(v)
            .filter(move |e| !directed || self.edge_target(e) == Some(v))
    }
    /// Edges leaving `v`; every incident edge for undirected graphs (`outgoingEdgesOf`).
    fn out_edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge> {
        let directed = self.graph_type().directed;
        self.edges_of(v)
            .filter(move |e| !directed || self.edge_source(e) == Some(v))
    }
    /// `inDegreeOf`: incoming edge count; equals [`Graph::degree_of`] for undirected graphs.
    fn in_degree_of(&self, v: &Self::Vertex) -> usize {
        if self.graph_type().directed {
            self.in_edges_of(v).count()
        } else {
            self.degree_of(v)
        }
    }
    /// `outDegreeOf`: outgoing edge count; equals [`Graph::degree_of`] for undirected graphs.
    fn out_degree_of(&self, v: &Self::Vertex) -> usize {
        if self.graph_type().directed {
            self.out_edges_of(v).count()
        } else {
            self.degree_of(v)
        }
    }

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

/// Marker for graphs whose edges have a direction. (The incoming/outgoing accessors live on
/// [`Graph`], where they degrade to "all incident edges" for undirected graphs.)
pub trait DirectedGraph: Graph {}

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
    /// Produces a fresh vertex object from the graph's vertex factory.
    fn new_vertex(&mut self) -> Result<Self::Vertex>;

    /// Creates a vertex with the graph's factory and adds it (`addVertex()` in JGraphT).
    fn add_new_vertex(&mut self) -> Result<Self::Vertex> {
        let v = self.new_vertex()?;
        self.add_vertex(v.clone());
        Ok(v)
    }

    /// Creates an edge with the graph's factory. `Ok(None)` mirrors JGraphT returning `null`.
    fn add_edge(&mut self, u: &Self::Vertex, v: &Self::Vertex) -> Result<Option<Self::Edge>> {
        let e = self.new_edge()?;
        Ok(self.add_edge_with(u, v, e.clone())?.then_some(e))
    }
}

/// JGraphT's `AbstractGraph.equals`: same direction, same vertex set, and every edge of `a` is an
/// edge of `b` (by edge value) with the same endpoints (unordered when undirected) and weight.
pub fn graph_equals<A, B>(a: &A, b: &B) -> bool
where
    A: Graph,
    B: Graph<Vertex = A::Vertex, Edge = A::Edge>,
{
    if a.graph_type().directed != b.graph_type().directed
        || a.vertex_count() != b.vertex_count()
        || a.edge_count() != b.edge_count()
    {
        return false;
    }
    if !a.vertices().all(|v| b.contains_vertex(v)) {
        return false;
    }
    let directed = a.graph_type().directed;
    a.edges().all(|e| {
        let (Some((s, t)), Some((bs, bt))) = (a.edge_endpoints(e), b.edge_endpoints(e)) else {
            return false;
        };
        let ends = if directed {
            s == bs && t == bt
        } else {
            (s == bs && t == bt) || (s == bt && t == bs)
        };
        ends && a.edge_weight(e).map(f64::to_bits) == b.edge_weight(e).map(f64::to_bits)
    })
}

/// Hash consistent with [`graph_equals`] (insertion-order independent; edge direction matters
/// only for directed graphs).
pub fn graph_hash<G: Graph>(g: &G) -> u64 {
    use std::hash::Hasher;
    fn h<T: Hash>(t: &T) -> u64 {
        let mut s = std::collections::hash_map::DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
    let directed = g.graph_type().directed;
    let mut total = g.vertices().map(h).fold(0u64, u64::wrapping_add);
    for e in g.edges() {
        let (s, t) = g.edge_endpoints(e).expect("edge endpoints");
        let (hs, ht) = (h(s), h(t));
        let ends = if directed {
            hs.wrapping_mul(31).wrapping_add(ht)
        } else {
            hs.wrapping_add(ht)
        };
        let w = g.edge_weight(e).map_or(0, f64::to_bits);
        total = total.wrapping_add(h(&(h(e), ends, w)));
    }
    total
}
