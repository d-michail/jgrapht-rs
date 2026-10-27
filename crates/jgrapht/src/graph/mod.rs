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
    /// a self-loop appears once. Fails with [`crate::GraphError::Unsupported`] on graphs that do
    /// not index incoming edges.
    fn edges_of(&self, v: &Self::Vertex) -> Result<impl Iterator<Item = &Self::Edge>>;

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
    /// Edges entering `v`; undirected edges count as entering (`incomingEdgesOf`).
    fn in_edges_of(&self, v: &Self::Vertex) -> Result<impl Iterator<Item = &Self::Edge>> {
        Ok(self
            .edges_of(v)?
            .filter(move |e| !self.is_edge_directed(e) || self.edge_target(e) == Some(v)))
    }
    /// Edges leaving `v`; undirected edges count as leaving (`outgoingEdgesOf`).
    fn out_edges_of(&self, v: &Self::Vertex) -> Result<impl Iterator<Item = &Self::Edge>> {
        Ok(self
            .edges_of(v)?
            .filter(move |e| !self.is_edge_directed(e) || self.edge_source(e) == Some(v)))
    }
    /// `inDegreeOf`: incoming edge count; equals [`Graph::degree_of`] for undirected graphs.
    fn in_degree_of(&self, v: &Self::Vertex) -> Result<usize> {
        let t = self.graph_type();
        if t.directed {
            Ok(self.in_edges_of(v)?.count())
        } else if t.mixed {
            Ok(self.in_edges_of(v)?.map(|e| self.mixed_weight(e)).sum())
        } else {
            self.degree_of(v)
        }
    }
    /// `outDegreeOf`: outgoing edge count; equals [`Graph::degree_of`] for undirected graphs.
    fn out_degree_of(&self, v: &Self::Vertex) -> Result<usize> {
        let t = self.graph_type();
        if t.directed {
            Ok(self.out_edges_of(v)?.count())
        } else if t.mixed {
            Ok(self.out_edges_of(v)?.map(|e| self.mixed_weight(e)).sum())
        } else {
            self.degree_of(v)
        }
    }
    #[doc(hidden)]
    /// Degree contribution of an edge in a mixed graph: undirected loops count twice.
    fn mixed_weight(&self, e: &Self::Edge) -> usize {
        if !self.is_edge_directed(e) && self.edge_source(e) == self.edge_target(e) {
            2
        } else {
            1
        }
    }

    fn get_edge<'a>(&'a self, u: &'a Self::Vertex, v: &'a Self::Vertex) -> Option<&'a Self::Edge> {
        self.edges_between(u, v).next()
    }
    fn contains_edge_between<'a>(&'a self, u: &'a Self::Vertex, v: &'a Self::Vertex) -> bool {
        self.get_edge(u, v).is_some()
    }
    /// Degree; self-loops count twice.
    fn degree_of(&self, v: &Self::Vertex) -> Result<usize> {
        Ok(self
            .edges_of(v)?
            .map(|e| {
                if self.edge_source(e) == self.edge_target(e) {
                    2
                } else {
                    1
                }
            })
            .sum())
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

/// JGraphT's `AbstractGraph.toString`: `([v1, v2], [(v1,v2)])` — directed edges render as
/// `(s,t)`, others as `{s,t}`; edges other than [`DefaultEdge`] are prefixed with `label=`.
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
