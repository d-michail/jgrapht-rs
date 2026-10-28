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

//! Incidence capability traits.
//!
//! Whether a graph can enumerate the edges leaving, entering or touching a vertex is a property of
//! its type, not a runtime state. These traits express that, so an algorithm asks for exactly the
//! access it uses and a graph that cannot provide it is rejected at compile time:
//!
//! | Trait | Provides | Typical use |
//! |---|---|---|
//! | [`OutgoingGraph`] | `out_edges_of`, `out_degree_of` | BFS, Dijkstra, forward DFS |
//! | [`IncomingGraph`] | `in_edges_of`, `in_degree_of` | reverse traversal, bidirectional search |
//! | [`IncidenceGraph`] | `edges_of`, `degree_of` (and both of the above) | degree scoring, coloring |
//!
//! `OutgoingGraph` and `IncomingGraph` are independent: the base [`Graph`] trait promises neither,
//! so an edge-reversed view of an outgoing-only graph is expressible as an incoming-only graph.
//! Every in-tree backend can enumerate outgoing edges, but third-party graphs and views need not.
//!
//! # Semantics shared by all three traits
//!
//! * An absent vertex has no incident edges: the iterators are empty and the degrees are `0`.
//! * Undirected edges (including those of a mixed graph) count as both leaving and entering; an
//!   undirected self-loop is yielded once but contributes `2` to a degree. A directed self-loop
//!   contributes `1` to each of `in_degree_of` and `out_degree_of`.
//! * Parallel edges are yielded once each, in the graph's own incidence order.
//! * Implementations must be consistent with [`Graph::edge_source`], [`Graph::edge_target`] and
//!   [`Graph::is_edge_directed`]: every yielded edge exists, and has `v` as the required endpoint.
//!
//! The traits are re-exported from [`crate::graph`] and the crate root.
//!
//! ```compile_fail,E0277
//! # use jgrapht::graph::{Graph, capability::IncomingGraph};
//! # use jgrapht::GraphType;
//! // A graph that implements only `Graph` cannot be used where incoming edges are required.
//! struct Empty;
//! # impl Graph for Empty {
//! #     type Vertex = u32;
//! #     type Edge = u32;
//! #     fn graph_type(&self) -> GraphType { GraphType::pseudograph().directed(true) }
//! #     fn vertices(&self) -> impl Iterator<Item = &u32> { [].iter() }
//! #     fn edges(&self) -> impl Iterator<Item = &u32> { [].iter() }
//! #     fn vertex_count(&self) -> usize { 0 }
//! #     fn edge_count(&self) -> usize { 0 }
//! #     fn contains_vertex(&self, _: &u32) -> bool { false }
//! #     fn contains_edge(&self, _: &u32) -> bool { false }
//! #     fn edge_source(&self, _: &u32) -> Option<&u32> { None }
//! #     fn edge_target(&self, _: &u32) -> Option<&u32> { None }
//! #     fn edge_weight(&self, _: &u32) -> Option<f64> { None }
//! #     fn edges_between<'a>(&'a self, _: &'a u32, _: &'a u32) -> impl Iterator<Item = &'a u32> { [].iter() }
//! # }
//! fn needs_incoming<G: IncomingGraph>(_: &G) {}
//! needs_incoming(&Empty);
//! ```

use super::Graph;

/// Degree contribution of an edge: an undirected self-loop counts twice, every other edge once.
fn mixed_weight<G: Graph + ?Sized>(g: &G, e: &G::Edge) -> usize {
    if !g.is_edge_directed(e) && g.edge_source(e) == g.edge_target(e) {
        2
    } else {
        1
    }
}

/// A graph that can enumerate the edges leaving a vertex.
pub trait OutgoingGraph: Graph {
    /// Edges leaving `v`; undirected edges count as leaving. Empty if `v` is absent.
    fn out_edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge>;

    /// Number of edges leaving `v`; an undirected self-loop counts twice. `0` if `v` is absent.
    fn out_degree_of(&self, v: &Self::Vertex) -> usize {
        OutgoingGraph::out_edges_of(self, v)
            .map(|e| mixed_weight(self, e))
            .sum()
    }
}

/// A graph that can enumerate the edges entering a vertex.
pub trait IncomingGraph: Graph {
    /// Edges entering `v`; undirected edges count as entering. Empty if `v` is absent.
    fn in_edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge>;

    /// Number of edges entering `v`; an undirected self-loop counts twice. `0` if `v` is absent.
    fn in_degree_of(&self, v: &Self::Vertex) -> usize {
        IncomingGraph::in_edges_of(self, v)
            .map(|e| mixed_weight(self, e))
            .sum()
    }
}

/// A graph that can enumerate every edge touching a vertex, in both directions.
pub trait IncidenceGraph: OutgoingGraph + IncomingGraph {
    /// All edges touching `v`; a self-loop appears once. Empty if `v` is absent.
    fn edges_of(&self, v: &Self::Vertex) -> impl Iterator<Item = &Self::Edge>;

    /// Degree of `v`; a self-loop counts twice. `0` if `v` is absent.
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
