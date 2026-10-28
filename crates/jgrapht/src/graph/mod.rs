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

//! Graph traits and the default implementation.

mod base;
pub mod builder;
pub mod capability;
mod ext;
pub mod map;
pub mod props;
pub mod sparse;

pub use base::{
    BaseGraph, DefaultEdge, DefaultGraphSpecifics, DiGraph, Directed, Direction, UGraph, Undirected,
};
pub use capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
pub use ext::{
    DirectedExt, GraphExt, GraphMutExt, remove_vertex_and_preserve_connectivity,
    remove_vertices_and_preserve_connectivity,
};
pub use map::{
    FastutilMapGraph, FastutilMapIntVertexGraph, FastutilMapIntVertexSpecifics,
    FastutilMapSpecifics,
};

use crate::{GraphType, Result};
use std::hash::Hash;

/// Read-only access to a graph. Vertices and edges are user-provided values.
///
/// Iterator-returning methods borrow from the graph; no intermediate sets are built.
/// For undirected graphs `edge_source`/`edge_target` return the endpoints in insertion order.
///
/// # Integrity contract
///
/// The trait is safe to implement, but algorithms rely on these invariants and may panic or
/// return meaningless results when an implementation breaks them:
///
/// * every edge yielded by [`Graph::edges`] or by an incidence query has endpoints that are
///   vertices of the graph and a weight;
/// * [`Graph::edge_source`], [`Graph::edge_target`] and [`Graph::edge_weight`] return `None` only
///   for an edge that is not in the graph;
/// * [`Graph::edges_between`] is empty if either vertex is absent, and incidence queries return
///   nothing for an absent vertex.
pub trait Graph {
    /// The vertex type.
    type Vertex: Clone + Eq + Hash;
    /// The edge type.
    type Edge: Clone + Eq + Hash;

    /// Returns the graph type.
    fn graph_type(&self) -> GraphType;

    /// Returns the vertices.
    fn vertices(&self) -> impl Iterator<Item = &Self::Vertex>;
    /// Returns the edges.
    fn edges(&self) -> impl Iterator<Item = &Self::Edge>;
    /// Returns the vertex count.
    fn vertex_count(&self) -> usize;
    /// Returns the edge count.
    fn edge_count(&self) -> usize;

    /// Returns the contains vertex.
    fn contains_vertex(&self, v: &Self::Vertex) -> bool;
    /// Returns the contains edge.
    fn contains_edge(&self, e: &Self::Edge) -> bool;

    /// Returns the edge source.
    fn edge_source(&self, e: &Self::Edge) -> Option<&Self::Vertex>;
    /// Returns the edge target.
    fn edge_target(&self, e: &Self::Edge) -> Option<&Self::Vertex>;
    /// Weight of the edge; `1.0` for unweighted graphs. `None` if the edge does not exist.
    fn edge_weight(&self, e: &Self::Edge) -> Option<f64>;

    /// Edges from `u` to `v` (either orientation for undirected graphs).
    fn edges_between<'a>(
        &'a self,
        u: &'a Self::Vertex,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Edge>;

    /// Whether this particular edge is directed. Only mixed graphs (unions of directed and
    /// undirected graphs) can answer differently from [`GraphType::directed`].
    fn is_edge_directed(&self, _e: &Self::Edge) -> bool {
        self.graph_type().directed
    }
    /// Returns the edge.
    fn get_edge<'a>(&'a self, u: &'a Self::Vertex, v: &'a Self::Vertex) -> Option<&'a Self::Edge> {
        self.edges_between(u, v).next()
    }
    /// Returns the contains edge between.
    fn contains_edge_between<'a>(&'a self, u: &'a Self::Vertex, v: &'a Self::Vertex) -> bool {
        self.get_edge(u, v).is_some()
    }
}

/// Marker for graphs whose edges have a direction. (Incidence queries live on the capability
/// traits in [`capability`], where undirected edges count as both leaving and entering.)
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
    /// Sets the edge weight.
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

    /// Creates an edge with the graph's factory. `Ok(None)` means the factory could not produce an edge.
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
///
/// # Panics
///
/// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
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

/// JGraphT's `AbstractGraph.toString`: `([v1, v2], [(v1,v2)])` — directed edges render as
/// `(s,t)`, others as `{s,t}`; edges other than [`DefaultEdge`] are prefixed with `label=`.
///
/// # Panics
///
/// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
pub fn graph_to_string<G>(g: &G) -> String
where
    G: Graph,
    G::Vertex: std::fmt::Display,
    G::Edge: std::fmt::Display + 'static,
{
    use std::fmt::Write;
    let labelled = std::any::TypeId::of::<G::Edge>() != std::any::TypeId::of::<DefaultEdge>();
    let mut out = String::from("([");
    for (i, v) in g.vertices().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{v}");
    }
    out.push_str("], [");
    for (i, e) in g.edges().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        if labelled {
            let _ = write!(out, "{e}=");
        }
        let (open, close) = if g.is_edge_directed(e) {
            ('(', ')')
        } else {
            ('{', '}')
        };
        let (s, t) = (
            g.edge_source(e).expect("edge"),
            g.edge_target(e).expect("edge"),
        );
        let _ = write!(out, "{open}{s},{t}{close}");
    }
    out.push_str("])");
    out
}
