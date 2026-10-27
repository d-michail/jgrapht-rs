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

//! Contract tests for the incidence capability traits (API roadmap R1).
//!
//! The capability methods share names with the fallible `Graph` ones until those are removed, so
//! the calls are qualified.
use crate::common::fixtures::{FullFixture, OutgoingFixture};
use jgrapht::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};

/// Needs only outgoing access, so an outgoing-only graph must satisfy the bound.
fn reachable<G: OutgoingGraph<Vertex = u32>>(g: &G, from: u32) -> Vec<u32> {
    let mut seen = vec![from];
    let mut i = 0;
    while i < seen.len() {
        let v = seen[i];
        i += 1;
        for e in OutgoingGraph::out_edges_of(g, &v) {
            let t = *g.edge_target(e).unwrap();
            if !seen.contains(&t) {
                seen.push(t);
            }
        }
    }
    seen
}

/// Needs the full capability set.
fn total_degree<G: IncidenceGraph>(g: &G) -> usize {
    g.vertices().map(|v| IncidenceGraph::degree_of(g, v)).sum()
}

const EDGES: [(u32, u32, f64); 4] = [(0, 1, 1.0), (1, 2, 2.0), (2, 3, 3.0), (0, 2, 10.0)];

#[test]
fn outgoing_only_graph_satisfies_outgoing_bound() {
    let g = OutgoingFixture::new(&[0, 1, 2, 3], &EDGES);
    assert_eq!(reachable(&g, 0), vec![0, 1, 2, 3]);
    assert_eq!(OutgoingGraph::out_degree_of(&g, &0), 2);
}

#[test]
fn full_graph_satisfies_every_bound() {
    let g = FullFixture::new(&[0, 1, 2, 3], &EDGES);
    assert_eq!(reachable(&g, 1), vec![1, 2, 3]);
    assert_eq!(total_degree(&g), 8);
    assert_eq!(IncomingGraph::in_degree_of(&g, &2), 2);
    assert_eq!(OutgoingGraph::out_degree_of(&g, &2), 1);
    assert_eq!(IncidenceGraph::degree_of(&g, &2), 3);
}

#[test]
fn absent_vertex_has_no_incident_edges() {
    let g = FullFixture::new(&[0, 1], &[(0, 1, 1.0)]);
    assert_eq!(OutgoingGraph::out_edges_of(&g, &9).count(), 0);
    assert_eq!(IncomingGraph::in_edges_of(&g, &9).count(), 0);
    assert_eq!(IncidenceGraph::edges_of(&g, &9).count(), 0);
    assert_eq!(IncidenceGraph::degree_of(&g, &9), 0);
}

#[test]
fn directed_self_loop_counts_once_per_direction_and_twice_in_degree() {
    let g = FullFixture::new(&[0], &[(0, 0, 1.0)]);
    assert_eq!(IncidenceGraph::edges_of(&g, &0).count(), 1);
    assert_eq!(OutgoingGraph::out_degree_of(&g, &0), 1);
    assert_eq!(IncomingGraph::in_degree_of(&g, &0), 1);
    assert_eq!(IncidenceGraph::degree_of(&g, &0), 2);
}

#[test]
fn parallel_edges_are_each_yielded() {
    let g = FullFixture::new(&[0, 1], &[(0, 1, 1.0), (0, 1, 2.0)]);
    assert_eq!(OutgoingGraph::out_degree_of(&g, &0), 2);
    assert_eq!(IncomingGraph::in_degree_of(&g, &1), 2);
}

mod base_graph_parity {
    use jgrapht::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
    use jgrapht::graph::{DiGraph, FastutilMapGraph, Graph, GraphMut, UGraph};
    use jgrapht::graph::{Directed, Undirected};

    /// The capability traits must agree with the fallible `Graph` methods, element by element and
    /// in the same order, for every vertex including absent ones.
    pub(crate) fn check<G>(g: &G, absent: G::Vertex)
    where
        G: IncidenceGraph,
        G::Edge: std::fmt::Debug + Ord,
    {
        check_with(g, absent, true)
    }

    /// Like [`check`], but compares incident edges as sorted lists. For views whose `Graph`
    /// impl derives in/out edges from `edges_of`, the old order differs incidentally.
    pub(crate) fn check_unordered<G>(g: &G, absent: G::Vertex)
    where
        G: IncidenceGraph,
        G::Edge: std::fmt::Debug + Ord,
    {
        check_with(g, absent, false)
    }

    fn check_with<G>(g: &G, absent: G::Vertex, ordered: bool)
    where
        G: IncidenceGraph,
        G::Edge: std::fmt::Debug + Ord,
    {
        let same = |mut new: Vec<G::Edge>, mut old: Vec<G::Edge>| {
            if !ordered {
                new.sort();
                old.sort();
            }
            assert_eq!(new, old);
        };
        let vs: Vec<_> = g.vertices().cloned().chain([absent]).collect();
        for v in &vs {
            let old_all: Vec<_> = Graph::edges_of(g, v).unwrap().cloned().collect();
            let old_in: Vec<_> = Graph::in_edges_of(g, v).unwrap().cloned().collect();
            let old_out: Vec<_> = Graph::out_edges_of(g, v).unwrap().cloned().collect();
            same(
                IncidenceGraph::edges_of(g, v).cloned().collect(),
                old_all.clone(),
            );
            same(IncomingGraph::in_edges_of(g, v).cloned().collect(), old_in);
            same(
                OutgoingGraph::out_edges_of(g, v).cloned().collect(),
                old_out,
            );
            assert_eq!(
                IncidenceGraph::degree_of(g, v),
                Graph::degree_of(g, v).unwrap()
            );
            assert_eq!(
                IncomingGraph::in_degree_of(g, v),
                Graph::in_degree_of(g, v).unwrap()
            );
            assert_eq!(
                OutgoingGraph::out_degree_of(g, v),
                Graph::out_degree_of(g, v).unwrap()
            );
            if !g.contains_vertex(v) {
                assert!(old_all.is_empty() && IncidenceGraph::degree_of(g, v) == 0);
            }
        }
    }

    pub(crate) fn populate<G: GraphMut<Vertex = i32>>(g: &mut G) {
        for v in 0..5 {
            g.add_vertex(v);
        }
        for (u, v) in [
            (0, 1),
            (1, 2),
            (2, 0),
            (2, 2),
            (0, 1),
            (3, 4),
            (4, 3),
            (3, 3),
        ] {
            g.add_edge(&u, &v).unwrap();
        }
    }

    #[test]
    fn directed_pseudograph() {
        let mut g = DiGraph::<i32>::pseudo();
        populate(&mut g);
        check(&g, 99);
        assert_eq!(IncidenceGraph::degree_of(&g, &2), 4); // 1->2, 2->0, loop 2->2 (twice)
    }

    #[test]
    fn undirected_pseudograph() {
        let mut g = UGraph::<i32>::pseudo();
        populate(&mut g);
        check(&g, 99);
        assert_eq!(IncidenceGraph::degree_of(&g, &3), 4); // two edges to 4, loop counts twice
        assert_eq!(IncomingGraph::in_degree_of(&g, &3), 4);
        assert_eq!(OutgoingGraph::out_degree_of(&g, &3), 4);
    }

    #[test]
    fn map_backed_graphs() {
        let mut d = FastutilMapGraph::<i32, _, Directed>::with_graph_type_and_lookup(
            jgrapht::GraphType::pseudograph().directed(true),
            true,
        );
        populate(&mut d);
        check(&d, 99);
        let mut u = FastutilMapGraph::<i32, _, Undirected>::with_graph_type_and_lookup(
            jgrapht::GraphType::pseudograph(),
            false,
        );
        populate(&mut u);
        check(&u, 99);
    }
}

mod sparse_type_state {
    use super::{reachable, total_degree};
    use jgrapht::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
    use jgrapht::graph::sparse::{
        FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedGraph,
        SparseIntDirectedWeightedGraph,
    };

    const EDGES: [(usize, usize); 5] = [(0, 1), (1, 2), (2, 0), (2, 2), (0, 1)];

    fn requires_incoming<G: IncomingGraph>(_: &G) {}

    #[test]
    fn no_incoming_is_outgoing_only_and_still_traversable() {
        let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(3, &EDGES).unwrap();
        assert_eq!(OutgoingGraph::out_degree_of(&g, &0), 2);
        assert_eq!(OutgoingGraph::out_edges_of(&g, &9).count(), 0);
        // `reachable` is bounded by `OutgoingGraph<Vertex = u32>`; use a usize twin here.
        let mut seen = vec![0usize];
        let mut i = 0;
        while i < seen.len() {
            let v = seen[i];
            i += 1;
            for e in OutgoingGraph::out_edges_of(&g, &v) {
                let t = *jgrapht::graph::GraphExt::edge_endpoints(&g, e).unwrap().1;
                if !seen.contains(&t) {
                    seen.push(t);
                }
            }
        }
        assert_eq!(seen, vec![0, 1, 2]);
        let _ = reachable::<crate::common::fixtures::FullFixture>;
    }

    #[test]
    fn lazy_and_full_agree_with_each_other() {
        let lazy = SparseIntDirectedGraph::<LazyIncoming>::from_edges(3, &EDGES).unwrap();
        let full = SparseIntDirectedGraph::new(3, &EDGES).unwrap();
        requires_incoming(&lazy);
        requires_incoming(&full);
        for v in 0..4 {
            assert_eq!(
                IncomingGraph::in_edges_of(&lazy, &v).collect::<Vec<_>>(),
                IncomingGraph::in_edges_of(&full, &v).collect::<Vec<_>>()
            );
            assert_eq!(
                IncidenceGraph::edges_of(&lazy, &v).collect::<Vec<_>>(),
                IncidenceGraph::edges_of(&full, &v).collect::<Vec<_>>()
            );
        }
        assert_eq!(total_degree(&full), 2 * EDGES.len());
        assert_eq!(IncomingGraph::in_degree_of(&full, &1), 2);
        assert_eq!(IncidenceGraph::degree_of(&full, &2), 4);
    }

    #[test]
    fn weighted_variants_follow_the_same_pattern() {
        let w = [(0, 1, 1.0), (1, 2, 2.0)];
        let no = SparseIntDirectedWeightedGraph::<NoIncoming>::from_edges(3, &w).unwrap();
        let full = SparseIntDirectedWeightedGraph::new(3, &w).unwrap();
        assert_eq!(OutgoingGraph::out_degree_of(&no, &1), 1);
        requires_incoming(&full);
        assert_eq!(IncomingGraph::in_degree_of(&full, &2), 1);
    }
}

mod view_parity {
    use super::base_graph_parity::{check, check_unordered, populate};
    use jgrapht::event::Listenable;
    use jgrapht::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
    use jgrapht::graph::sparse::{NoIncoming, SparseIntDirectedGraph, SparseIntUndirectedGraph};
    use jgrapht::graph::{DiGraph, Graph, UGraph};
    use jgrapht::view::{
        AsSubgraph, AsUndirected, AsUnweighted, AsWeighted, EdgeReversed, GraphUnion, MaskSubgraph,
        Unmodifiable,
    };
    use std::collections::HashMap;

    fn directed() -> DiGraph<i32> {
        let mut g = DiGraph::<i32>::pseudo();
        populate(&mut g);
        g
    }

    fn undirected() -> UGraph<i32> {
        let mut g = UGraph::<i32>::pseudo();
        populate(&mut g);
        g
    }

    #[test]
    fn forwarding_wrappers() {
        let mut g = directed();
        check(&&g, 99);
        check(&Unmodifiable(&g), 99);
        check(&AsUnweighted(&g), 99);
        check(&AsWeighted::new(&g, HashMap::new()), 99);
        check(&Listenable::new(&g), 99);
        check(&&mut g, 99);
    }

    #[test]
    fn edge_reversed_swaps_incoming_and_outgoing() {
        let g = directed();
        let r = EdgeReversed(&g);
        check(&r, 99);
        for v in 0..5 {
            assert_eq!(
                IncomingGraph::in_edges_of(&r, &v).collect::<Vec<_>>(),
                OutgoingGraph::out_edges_of(&g, &v).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn as_undirected_makes_every_edge_incident_both_ways() {
        let g = directed();
        let u = AsUndirected(&g);
        check(&u, 99);
        for v in 0..5 {
            assert_eq!(
                OutgoingGraph::out_edges_of(&u, &v).collect::<Vec<_>>(),
                IncidenceGraph::edges_of(&g, &v).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn subgraphs_filter_incidence() {
        for g in [directed()] {
            check_unordered(&AsSubgraph::induced(&g, [0, 1, 2]), 99);
            check_unordered(&AsSubgraph::all(&g), 99);
            check_unordered(&MaskSubgraph::new(&g, |v: &i32| *v == 1, |_| false), 99);
            let s = AsSubgraph::induced(&g, [0, 1]);
            assert!(IncidenceGraph::edges_of(&s, &2).next().is_none());
            assert!(
                IncidenceGraph::edges_of(&s, &0).all(|e| s.edge_target(e).is_some_and(|t| *t <= 1))
            );
        }
        let u = undirected();
        check_unordered(&AsSubgraph::induced(&u, [3, 4]), 99);
        check_unordered(&MaskSubgraph::new(&u, |v: &i32| *v == 0, |_| false), 99);
    }

    #[test]
    fn union_deduplicates_shared_edges() {
        let g = directed();
        let union = GraphUnion::new(&g, &g);
        check_unordered(&union, 99);
        assert_eq!(
            IncidenceGraph::edges_of(&union, &2).count(),
            IncidenceGraph::edges_of(&g, &2).count()
        );
    }

    #[test]
    fn sparse_undirected_graphs() {
        let g = SparseIntUndirectedGraph::new(4, &[(0, 1), (1, 1), (2, 3), (0, 1)]).unwrap();
        check(&g, 99);
        assert_eq!(IncidenceGraph::degree_of(&g, &1), 4);
        assert_eq!(IncomingGraph::in_degree_of(&g, &1), 4);
        assert_eq!(IncidenceGraph::edges_of(&g, &1).count(), 3);
    }

    /// `EdgeReversed` of an outgoing-only graph is incoming-only: the capability moves with it.
    #[test]
    fn edge_reversed_no_incoming_is_incoming_only() {
        fn needs_incoming<G: IncomingGraph>(_: &G) {}
        let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(3, &[(0, 1), (1, 2)]).unwrap();
        let r = EdgeReversed(&g);
        needs_incoming(&r);
        assert_eq!(IncomingGraph::in_degree_of(&r, &1), 1);
    }
}
