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

//! Capability bounds of the algorithms (API roadmap R1): algorithms that only follow outgoing
//! edges run on a graph without an incoming index, and algorithms that need incoming edges accept
//! the lazy and full indexes. The rejection of `NoIncoming` is covered by `compile_fail` doctests
//! on the algorithms themselves.
use jgrapht::alg::shortestpath::{
    BfsShortestPath, BidirectionalDijkstraShortestPath, Dijkstra, ShortestPathAlgorithm,
};
use jgrapht::graph::sparse::{
    FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedWeightedGraph,
};
use jgrapht::traverse::{Bfs, Dfs, DfsPostOrder, topological_sort};

const EDGES: [(usize, usize, f64); 5] = [
    (0, 1, 1.0),
    (1, 2, 2.0),
    (0, 2, 5.0),
    (2, 3, 1.0),
    (1, 3, 7.0),
];

#[test]
fn traversals_and_forward_searches_run_without_an_incoming_index() {
    let g = SparseIntDirectedWeightedGraph::<NoIncoming>::from_edges(4, &EDGES).unwrap();
    assert_eq!(Bfs::new(&g, &0).count(), 4);
    assert_eq!(Dfs::new(&g, &0).count(), 4);
    assert_eq!(DfsPostOrder::new(&g, &0).count(), 4);
    assert_eq!(Dijkstra::new(&g).path_weight(&0, &3).unwrap(), 4.0);
    assert_eq!(BfsShortestPath::new(&g).path_weight(&0, &3).unwrap(), 2.0);
}

#[test]
fn reverse_aware_algorithms_accept_lazy_and_full_indexes() {
    let lazy = SparseIntDirectedWeightedGraph::<LazyIncoming>::from_edges(4, &EDGES).unwrap();
    let full = SparseIntDirectedWeightedGraph::<FullIncoming>::from_edges(4, &EDGES).unwrap();
    for weight in [
        BidirectionalDijkstraShortestPath::new(&lazy).path_weight(&0, &3),
        BidirectionalDijkstraShortestPath::new(&full).path_weight(&0, &3),
    ] {
        assert_eq!(weight.unwrap(), 4.0);
    }
    assert_eq!(topological_sort(&lazy).unwrap().len(), 4);
    assert_eq!(topological_sort(&full).unwrap().len(), 4);
}
