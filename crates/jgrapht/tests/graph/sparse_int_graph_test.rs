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

//! Port of `org.jgrapht.opt.graph.sparse.SparseIntGraphTest`.
//!
//! Java's `getEdge` returns an `Integer` and `getAllEdges` a set; here they are `Option<&usize>`
//! and a sorted vector. `UnsupportedOperationException` for graphs without incoming-edge support
//! becomes a compile error: `NoIncoming` graphs do not implement `IncomingGraph`.

use jgrapht::OutgoingGraph;
use jgrapht::graph::IncidenceGraph;
use jgrapht::graph::sparse::*;
use jgrapht::graph::{Graph, GraphExt};

fn sorted<'a>(it: impl Iterator<Item = &'a usize>) -> Vec<usize> {
    let mut v: Vec<usize> = it.copied().collect();
    v.sort_unstable();
    v
}

/// `(degree, in degree, out degree, edges, incoming, outgoing)` per vertex.
type Row = (
    usize,
    usize,
    usize,
    &'static [usize],
    &'static [usize],
    &'static [usize],
);

fn check_type<G: Graph>(g: &G, directed: bool, weighted: bool) {
    let t = g.graph_type();
    assert!(t.allows_cycles);
    assert!(t.multiple_edges);
    assert!(t.self_loops);
    assert_eq!(t.directed, directed);
    assert_eq!(t.is_undirected(), !directed);
    assert!(!t.modifiable);
    assert!(!t.mixed);
    assert_eq!(t.weighted, weighted);
}

fn check_ids<G: Graph<Vertex = usize, Edge = usize>>(g: &G, n: usize, m: usize) {
    assert_eq!(g.vertex_count(), n);
    assert_eq!(g.edge_count(), m);
    assert_eq!(
        g.edges().copied().collect::<Vec<_>>(),
        (0..m).collect::<Vec<_>>()
    );
    assert_eq!(
        g.vertices().copied().collect::<Vec<_>>(),
        (0..n).collect::<Vec<_>>()
    );
    for v in 0..n {
        assert!(g.contains_vertex(&v));
    }
}

fn check_endpoints<G: Graph<Vertex = usize, Edge = usize>>(g: &G, edges: &[(usize, usize)]) {
    for (e, &(s, t)) in edges.iter().enumerate() {
        assert_eq!(g.edge_source(&e), Some(&s));
        assert_eq!(g.edge_target(&e), Some(&t));
    }
}

fn check_rows<G: Graph<Vertex = usize, Edge = usize> + IncidenceGraph>(g: &G, rows: &[Row]) {
    for (v, &(deg, indeg, outdeg, edges, incoming, outgoing)) in rows.iter().enumerate() {
        assert_eq!(g.degree_of(&v), deg);
        assert_eq!(g.in_degree_of(&v), indeg);
        assert_eq!(g.out_degree_of(&v), outdeg);
        assert_eq!(sorted(g.edges_of(&v)), edges);
        assert_eq!(sorted(g.in_edges_of(&v)), incoming);
        assert_eq!(sorted(g.out_edges_of(&v)), outgoing);
    }
}

const UNDIRECTED: [(usize, usize); 7] = [(0, 5), (0, 2), (3, 4), (1, 4), (0, 1), (3, 1), (2, 4)];

const UNDIRECTED_ROWS: [Row; 6] = [
    (3, 3, 3, &[0, 1, 4], &[0, 1, 4], &[0, 1, 4]),
    (3, 3, 3, &[3, 4, 5], &[3, 4, 5], &[3, 4, 5]),
    (2, 2, 2, &[1, 6], &[1, 6], &[1, 6]),
    (2, 2, 2, &[2, 5], &[2, 5], &[2, 5]),
    (3, 3, 3, &[2, 3, 6], &[2, 3, 6], &[2, 3, 6]),
    (1, 1, 1, &[0], &[0], &[0]),
];

fn check_unique_edges<G: Graph<Vertex = usize, Edge = usize>>(g: &G, edges: &[(usize, usize)]) {
    for (i, &(s, t)) in edges.iter().enumerate() {
        assert_eq!(g.get_edge(&s, &t), Some(&i));
    }
    for (j, &(s, t)) in edges.iter().enumerate() {
        assert_eq!(sorted(g.edges_between(&s, &t)), vec![j]);
    }
}

#[test]
fn test_undirected() {
    let g = SparseIntUndirectedGraph::new(6, &UNDIRECTED).unwrap();
    check_ids(&g, 6, 7);
    check_rows(&g, &UNDIRECTED_ROWS);
    check_endpoints(&g, &UNDIRECTED);
    check_type(&g, false, false);
    check_unique_edges(&g, &UNDIRECTED);
}

#[test]
fn test_undirected_with_loops() {
    let edges = [(0, 0), (0, 1), (0, 2), (0, 0), (0, 1), (1, 1), (1, 2)];
    let g = SparseIntUndirectedGraph::new(4, &edges).unwrap();
    check_ids(&g, 4, 7);
    check_rows(
        &g,
        &[
            (
                7,
                7,
                7,
                &[0, 1, 2, 3, 4],
                &[0, 1, 2, 3, 4],
                &[0, 1, 2, 3, 4],
            ),
            (5, 5, 5, &[1, 4, 5, 6], &[1, 4, 5, 6], &[1, 4, 5, 6]),
            (2, 2, 2, &[2, 6], &[2, 6], &[2, 6]),
            (0, 0, 0, &[], &[], &[]),
        ],
    );
    check_endpoints(&g, &edges);
    check_type(&g, false, false);
    assert_eq!(sorted(g.edges_between(&0, &0)), vec![0, 3]);
}

#[test]
fn test_undirected_weighted() {
    let weighted: Vec<_> = UNDIRECTED
        .iter()
        .enumerate()
        .map(|(i, &(s, t))| (s, t, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0][i]))
        .collect();
    let mut g = SparseIntUndirectedWeightedGraph::new(6, &weighted).unwrap();
    check_ids(&g, 6, 7);
    check_rows(&g, &UNDIRECTED_ROWS);
    check_endpoints(&g, &UNDIRECTED);
    for (e, &(_, _, w)) in weighted.iter().enumerate() {
        assert_eq!(g.edge_weight(&e), Some(w));
    }
    assert_eq!(g.edge_weight(&4), Some(5.0));
    g.set_edge_weight(4, 14.0).unwrap();
    assert_eq!(g.edge_weight(&4), Some(14.0));
    assert!(g.set_edge_weight(7, 1.0).is_err());
    check_type(&g, false, true);
    check_unique_edges(&g, &UNDIRECTED);
}

const DIRECTED: [(usize, usize); 13] = [
    (0, 1),
    (1, 0),
    (1, 4),
    (1, 5),
    (1, 6),
    (2, 4),
    (2, 4),
    (2, 4),
    (3, 4),
    (4, 5),
    (5, 6),
    (7, 6),
    (7, 7),
];

const DIRECTED_ROWS: [Row; 8] = [
    (2, 1, 1, &[0, 1], &[1], &[0]),
    (5, 1, 4, &[0, 1, 2, 3, 4], &[0], &[1, 2, 3, 4]),
    (3, 0, 3, &[5, 6, 7], &[], &[5, 6, 7]),
    (1, 0, 1, &[8], &[], &[8]),
    (6, 5, 1, &[2, 5, 6, 7, 8, 9], &[2, 5, 6, 7, 8], &[9]),
    (3, 2, 1, &[3, 9, 10], &[3, 9], &[10]),
    (3, 3, 0, &[4, 10, 11], &[4, 10, 11], &[]),
    (3, 1, 2, &[11, 12], &[12], &[11, 12]),
];

fn check_directed_lookups<G: Graph<Vertex = usize, Edge = usize>>(g: &G) {
    let expected: [((usize, usize), &[usize]); 11] = [
        ((0, 1), &[0]),
        ((1, 0), &[1]),
        ((1, 4), &[2]),
        ((1, 5), &[3]),
        ((1, 6), &[4]),
        ((2, 4), &[5, 6, 7]),
        ((3, 4), &[8]),
        ((4, 5), &[9]),
        ((5, 6), &[10]),
        ((7, 6), &[11]),
        ((7, 7), &[12]),
    ];
    for ((s, t), all) in expected {
        assert_eq!(g.get_edge(&s, &t), Some(&all[0]));
        assert_eq!(sorted(g.edges_between(&s, &t)), all);
    }
}

fn check_directed(g: &impl IncidenceGraph<Vertex = usize, Edge = usize>, weighted: bool) {
    check_ids(g, 8, 13);
    check_rows(g, &DIRECTED_ROWS);
    check_endpoints(g, &DIRECTED);
    check_type(g, true, weighted);
    check_directed_lookups(g);
}

#[test]
fn test_directed() {
    check_directed(&SparseIntDirectedGraph::new(8, &DIRECTED).unwrap(), false);
}

#[test]
fn test_directed_lazy_incoming() {
    let g = SparseIntDirectedGraph::<LazyIncoming>::from_edges(8, &DIRECTED).unwrap();
    check_directed(&g, false);
}

#[test]
fn test_directed_lazy_no_incoming() {
    let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(8, &DIRECTED).unwrap();
    check_ids(&g, 8, 13);
    for (v, row) in DIRECTED_ROWS.iter().enumerate() {
        assert_eq!(g.out_degree_of(&v), row.2);
        assert_eq!(sorted(g.out_edges_of(&v)), row.5);
    }
    assert_eq!(
        g.outgoing_of(&1)
            .map(|(edge, target)| (*edge, *target))
            .collect::<Vec<_>>(),
        [(1, 0), (2, 4), (3, 5), (4, 6)]
    );
    check_endpoints(&g, &DIRECTED);
    check_type(&g, true, false);
    check_directed_lookups(&g);
}

#[test]
fn test_directed_weighted() {
    let weighted: Vec<_> = DIRECTED
        .iter()
        .enumerate()
        .map(|(i, &(s, t))| (s, t, i as f64))
        .collect();
    let mut g = SparseIntDirectedWeightedGraph::new(8, &weighted).unwrap();
    check_directed(&g, true);
    for e in 0..13 {
        assert_eq!(g.edge_weight(&e), Some(e as f64));
    }
    g.set_edge_weight(4, 14.0).unwrap();
    assert_eq!(g.edge_weight(&4), Some(14.0));
}

#[test]
fn test_invalid_input() {
    assert!(SparseIntDirectedGraph::new(2, &[(0, 2)]).is_err());
    assert!(SparseIntUndirectedGraph::new(2, &[(2, 0)]).is_err());
    assert!(SparseIntUndirectedGraph::from_iter(3, 2, [(0, 1)]).is_err());
}

#[test]
fn test_absent_elements() {
    let g = SparseIntDirectedGraph::new(8, &DIRECTED).unwrap();
    assert!(!g.contains_vertex(&8));
    assert!(!g.contains_edge(&13));
    assert_eq!(g.get_edge(&0, &8), None);
    assert_eq!(g.get_edge(&9, &0), None);
    assert_eq!(g.edges_between(&0, &8).count(), 0);
    assert_eq!(g.edges_of(&8).count(), 0);
    assert_eq!(g.edge_source(&13), None);
    assert_eq!(g.edge_weight(&13), None);
}

#[test]
fn test_empty_graph() {
    let g = SparseIntUndirectedGraph::new(0, &[]).unwrap();
    assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
    let g = SparseIntDirectedGraph::new(3, &[]).unwrap();
    assert_eq!(g.vertices().count(), 3);
    assert_eq!(g.edges().count(), 0);
}

/// Algorithms written against `Graph` run unchanged on the sparse graphs.
#[test]
fn dijkstra_matches_base_graph() {
    use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
    use jgrapht::{DiGraph, GraphMut, GraphMutExt};

    let n = 40;
    let mut state = 12345u64;
    let mut next = move |bound: u64| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) % bound
    };
    let edges: Vec<(usize, usize, f64)> = (0..200)
        .map(|_| (next(n) as usize, next(n) as usize, 1.0 + next(20) as f64))
        .collect();

    let sparse = SparseIntDirectedWeightedGraph::new(n as usize, &edges).unwrap();
    let mut base = DiGraph::<usize>::pseudo().weighted();
    for v in 0..n as usize {
        base.add_vertex(v);
    }
    for &(s, t, w) in &edges {
        base.add_weighted_edge(&s, &t, w).unwrap();
    }

    let sparse_dijkstra = Dijkstra::new(&sparse);
    let base_dijkstra = Dijkstra::new(&base);
    for s in 0..n as usize {
        for t in 0..n as usize {
            assert_eq!(
                sparse_dijkstra.path_weight(&s, &t).unwrap(),
                base_dijkstra.path_weight(&s, &t).unwrap()
            );
        }
    }
}
