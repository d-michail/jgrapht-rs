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

//! Incidence capabilities of the views. A view implements exactly the capabilities its base can
//! honor: filters and forwarders keep the base's set, [`EdgeReversed`] swaps incoming and outgoing,
//! and [`AsUndirected`] needs full incidence because every edge then touches both endpoints.

use super::subgraph::{AsSubgraph, MaskSubgraph};
use super::union::GraphUnion;
use super::{AsUndirected, EdgeReversed};
use crate::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use crate::graph::{DirectedGraph, Graph};

/// Capability impls for a view that forwards every incidence query to an inner graph unchanged.
///
/// `[$($gen)*]` are the impl generics, `$ty` the view, `$g` the inner graph type and
/// `$s => $inner` the receiver identifier and the expression yielding the inner graph.
macro_rules! forward_incidence {
    ([$($gen:tt)*] $ty:ty, $g:ty, $s:ident => $inner:expr) => {
        impl<$($gen)*> $crate::graph::capability::OutgoingGraph for $ty
        where
            $g: $crate::graph::capability::OutgoingGraph,
        {
            fn out_edges_of(&$s, v: &<Self as $crate::graph::Graph>::Vertex)
                -> impl Iterator<Item = &<Self as $crate::graph::Graph>::Edge>
            {
                $crate::graph::capability::OutgoingGraph::out_edges_of(&$inner, v)
            }
            fn out_degree_of(&$s, v: &<Self as $crate::graph::Graph>::Vertex) -> usize {
                $crate::graph::capability::OutgoingGraph::out_degree_of(&$inner, v)
            }
        }
        impl<$($gen)*> $crate::graph::capability::IncomingGraph for $ty
        where
            $g: $crate::graph::capability::IncomingGraph,
        {
            fn in_edges_of(&$s, v: &<Self as $crate::graph::Graph>::Vertex)
                -> impl Iterator<Item = &<Self as $crate::graph::Graph>::Edge>
            {
                $crate::graph::capability::IncomingGraph::in_edges_of(&$inner, v)
            }
            fn in_degree_of(&$s, v: &<Self as $crate::graph::Graph>::Vertex) -> usize {
                $crate::graph::capability::IncomingGraph::in_degree_of(&$inner, v)
            }
        }
        impl<$($gen)*> $crate::graph::capability::IncidenceGraph for $ty
        where
            $g: $crate::graph::capability::IncidenceGraph,
        {
            fn edges_of(&$s, v: &<Self as $crate::graph::Graph>::Vertex)
                -> impl Iterator<Item = &<Self as $crate::graph::Graph>::Edge>
            {
                $crate::graph::capability::IncidenceGraph::edges_of(&$inner, v)
            }
            fn degree_of(&$s, v: &<Self as $crate::graph::Graph>::Vertex) -> usize {
                $crate::graph::capability::IncidenceGraph::degree_of(&$inner, v)
            }
        }
    };
}
pub(crate) use forward_incidence;

forward_incidence!([G: Graph + ?Sized] &G, G, self => **self);
forward_incidence!([G: Graph + ?Sized] &mut G, G, self => **self);
forward_incidence!([G: Graph] super::Unmodifiable<G>, G, self => self.0);
forward_incidence!([G: Graph] super::AsUnweighted<G>, G, self => self.0);
forward_incidence!(['a, G: Graph] super::AsWeighted<'a, G>, G, self => self.graph);

// ------------------------------------------------------------------ EdgeReversed

impl<G: DirectedGraph + IncomingGraph> OutgoingGraph for EdgeReversed<G> {
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        IncomingGraph::in_edges_of(&self.0, v)
    }
    fn out_degree_of(&self, v: &G::Vertex) -> usize {
        IncomingGraph::in_degree_of(&self.0, v)
    }
}

impl<G: DirectedGraph + OutgoingGraph> IncomingGraph for EdgeReversed<G> {
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        OutgoingGraph::out_edges_of(&self.0, v)
    }
    fn in_degree_of(&self, v: &G::Vertex) -> usize {
        OutgoingGraph::out_degree_of(&self.0, v)
    }
}

impl<G: DirectedGraph + IncidenceGraph> IncidenceGraph for EdgeReversed<G> {
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        IncidenceGraph::edges_of(&self.0, v)
    }
    fn degree_of(&self, v: &G::Vertex) -> usize {
        IncidenceGraph::degree_of(&self.0, v)
    }
}

// ------------------------------------------------------------------ AsUndirected

// Every edge touches both endpoints, so all three capabilities need the base's full incidence.
impl<G: DirectedGraph + IncidenceGraph> OutgoingGraph for AsUndirected<G> {
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        IncidenceGraph::edges_of(&self.0, v)
    }
}

impl<G: DirectedGraph + IncidenceGraph> IncomingGraph for AsUndirected<G> {
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        IncidenceGraph::edges_of(&self.0, v)
    }
}

impl<G: DirectedGraph + IncidenceGraph> IncidenceGraph for AsUndirected<G> {
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        IncidenceGraph::edges_of(&self.0, v)
    }
    fn degree_of(&self, v: &G::Vertex) -> usize {
        IncidenceGraph::degree_of(&self.0, v)
    }
}

// ------------------------------------------------------------------ MaskSubgraph

impl<G, FV, FE> OutgoingGraph for MaskSubgraph<G, FV, FE>
where
    G: OutgoingGraph,
    FV: Fn(&G::Vertex) -> bool,
    FE: Fn(&G::Edge) -> bool,
{
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertex_ok(v);
        OutgoingGraph::out_edges_of(&self.graph, v).filter(move |e| ok && self.edge_ok(e))
    }
}

impl<G, FV, FE> IncomingGraph for MaskSubgraph<G, FV, FE>
where
    G: IncomingGraph,
    FV: Fn(&G::Vertex) -> bool,
    FE: Fn(&G::Edge) -> bool,
{
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertex_ok(v);
        IncomingGraph::in_edges_of(&self.graph, v).filter(move |e| ok && self.edge_ok(e))
    }
}

impl<G, FV, FE> IncidenceGraph for MaskSubgraph<G, FV, FE>
where
    G: IncidenceGraph,
    FV: Fn(&G::Vertex) -> bool,
    FE: Fn(&G::Edge) -> bool,
{
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertex_ok(v);
        IncidenceGraph::edges_of(&self.graph, v).filter(move |e| ok && self.edge_ok(e))
    }
}

// ------------------------------------------------------------------ AsSubgraph

impl<G: OutgoingGraph> OutgoingGraph for AsSubgraph<G> {
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertices.contains(v);
        OutgoingGraph::out_edges_of(&self.base, v).filter(move |e| ok && self.edges.contains(*e))
    }
}

impl<G: IncomingGraph> IncomingGraph for AsSubgraph<G> {
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertices.contains(v);
        IncomingGraph::in_edges_of(&self.base, v).filter(move |e| ok && self.edges.contains(*e))
    }
}

impl<G: IncidenceGraph> IncidenceGraph for AsSubgraph<G> {
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertices.contains(v);
        IncidenceGraph::edges_of(&self.base, v).filter(move |e| ok && self.edges.contains(*e))
    }
}

// ------------------------------------------------------------------ GraphUnion

impl<A, B> OutgoingGraph for GraphUnion<A, B>
where
    A: OutgoingGraph,
    B: OutgoingGraph<Vertex = A::Vertex, Edge = A::Edge>,
{
    fn out_edges_of(&self, v: &A::Vertex) -> impl Iterator<Item = &A::Edge> {
        OutgoingGraph::out_edges_of(&self.a, v)
            .chain(OutgoingGraph::out_edges_of(&self.b, v).filter(|e| !self.a.contains_edge(e)))
    }
}

impl<A, B> IncomingGraph for GraphUnion<A, B>
where
    A: IncomingGraph,
    B: IncomingGraph<Vertex = A::Vertex, Edge = A::Edge>,
{
    fn in_edges_of(&self, v: &A::Vertex) -> impl Iterator<Item = &A::Edge> {
        IncomingGraph::in_edges_of(&self.a, v)
            .chain(IncomingGraph::in_edges_of(&self.b, v).filter(|e| !self.a.contains_edge(e)))
    }
}

impl<A, B> IncidenceGraph for GraphUnion<A, B>
where
    A: IncidenceGraph,
    B: IncidenceGraph<Vertex = A::Vertex, Edge = A::Edge>,
{
    fn edges_of(&self, v: &A::Vertex) -> impl Iterator<Item = &A::Edge> {
        IncidenceGraph::edges_of(&self.a, v)
            .chain(IncidenceGraph::edges_of(&self.b, v).filter(|e| !self.a.contains_edge(e)))
    }
}
