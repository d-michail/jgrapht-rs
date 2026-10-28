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

//! Rust-native checks of `graph::props` (API roadmap R4): the predicates never panic, whatever
//! capability the graph has and even when it breaks the `Graph` integrity contract, and they
//! agree between graph types that differ only in capability.
use crate::common::fixtures::{FixtureGraph, FullFixture, OutgoingFixture};
use jgrapht::graph::props::*;
use jgrapht::graph::sparse::{
    FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedGraph, SparseIntUndirectedGraph,
};

const CYCLE: [(usize, usize); 4] = [(0, 1), (1, 2), (2, 3), (3, 0)];
const PATH: [(usize, usize); 3] = [(0, 1), (1, 2), (2, 3)];
const LOOPS_AND_PARALLEL: [(usize, usize); 4] = [(0, 1), (0, 1), (1, 1), (1, 2)];

#[test]
fn outgoing_only_predicates_agree_with_the_full_graph() {
    for edges in [&CYCLE[..], &PATH[..], &LOOPS_AND_PARALLEL[..]] {
        let no = SparseIntDirectedGraph::<NoIncoming>::from_edges(4, edges).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, edges).unwrap();
        assert_eq!(is_simple(&no), is_simple(&full));
        assert_eq!(is_complete(&no), is_complete(&full));
        assert_eq!(has_multiple_edges(&no), has_multiple_edges(&full));
        assert_eq!(has_self_loops(&no), has_self_loops(&full));
        assert_eq!(is_forest(&no), is_forest(&full));
    }
}

#[test]
fn incidence_predicates_agree_between_lazy_and_full_indexes() {
    for edges in [&CYCLE[..], &PATH[..], &LOOPS_AND_PARALLEL[..]] {
        let lazy = SparseIntDirectedGraph::<LazyIncoming>::from_edges(4, edges).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, edges).unwrap();
        assert_eq!(is_dag(&lazy), is_dag(&full));
        assert_eq!(is_connected(&lazy), is_connected(&full));
        assert_eq!(is_strongly_connected(&lazy), is_strongly_connected(&full));
        assert_eq!(is_tree(&lazy), is_tree(&full));
        assert_eq!(is_cubic(&lazy), is_cubic(&full));
    }
    let cycle = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, &CYCLE).unwrap();
    assert!(!is_dag(&cycle) && is_strongly_connected(&cycle));
    let path = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, &PATH).unwrap();
    assert!(is_dag(&path) && is_tree(&path) && !is_strongly_connected(&path));
}

#[test]
fn undirected_sparse_graphs_satisfy_every_capability_bound() {
    let g = SparseIntUndirectedGraph::new(4, &PATH).unwrap();
    assert!(is_connected(&g) && is_tree(&g) && is_forest(&g) && is_simple(&g));
    assert!(!is_complete(&g));
}

#[test]
fn predicates_do_not_panic_on_a_graph_that_breaks_the_integrity_contract() {
    // edge 0 targets vertex 99, edge 1 has no source, edge 2 has no weight
    let broken = FixtureGraph::inconsistent();
    assert!(!is_forest(&broken));
    // predicates built on `Indexed` (bipartite, connectivity components, ...) rely on the
    // contract and are documented to panic on such a graph; these do not
    let _ = (has_self_loops(&broken), is_empty(&broken));

    let full = FullFixture(FixtureGraph::inconsistent());
    let outgoing = OutgoingFixture(FixtureGraph::inconsistent());
    let _ = is_dag(&full);
    let _ = (is_simple(&outgoing), has_multiple_edges(&outgoing));
    let _ = (is_connected(&full), is_tree(&full), is_cubic(&full));
}

#[test]
fn degenerate_graphs_have_defined_answers() {
    let empty = SparseIntDirectedGraph::<FullIncoming>::from_edges(0, &[]).unwrap();
    assert!(is_empty(&empty) && is_dag(&empty) && is_simple(&empty));
    assert!(!is_connected(&empty) && !is_forest(&empty) && !is_overfull(&empty));
    let single = SparseIntDirectedGraph::<FullIncoming>::from_edges(1, &[]).unwrap();
    assert!(
        is_connected(&single) && is_tree(&single) && is_forest(&single) && is_complete(&single)
    );
    assert!(!is_split(&SparseIntUndirectedGraph::new(0, &[]).unwrap()));
}
