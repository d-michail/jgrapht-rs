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

use jgrapht::graph::{DirectedExt, props};
use jgrapht::*;

#[test]
fn undirected_simple() {
    let mut g = UGraph::<&str>::simple();
    let e = g.add_edge_with_vertices("a", "b").unwrap().unwrap();
    g.add_edge_with_vertices("b", "c").unwrap();
    assert_eq!(g.vertex_count(), 3);
    assert_eq!(g.edge_count(), 2);
    assert!(g.edge_between(&"b", &"a").is_some());
    assert_eq!(g.degree_of(&"b"), 2);
    assert_eq!(g.add_edge(&"a", &"b").unwrap(), None); // parallel refused
    assert_eq!(g.add_edge(&"a", &"a"), Err(GraphError::LoopsNotAllowed));
    assert_eq!(g.opposite_vertex(&e, &"a"), Some(&"b"));
    assert!(props::is_tree(&g) && props::is_simple(&g));
    assert!(g.remove_vertex(&"b"));
    assert_eq!(g.edge_count(), 0);
}

#[test]
fn pseudograph_loops_and_multi() {
    let mut g = UGraph::<u32>::pseudo();
    g.add_vertex(1);
    g.add_edge(&1, &1).unwrap().unwrap();
    g.add_edge(&1, &1).unwrap().unwrap();
    assert_eq!(g.degree_of(&1), 4);
    assert_eq!(g.edges_of(&1).count(), 2);
    assert!(props::has_multiple_edges(&g) && props::has_self_loops(&g));
}

#[test]
fn directed_weighted() {
    let mut g = DiGraph::<u32>::simple().weighted();
    g.add_vertices([1, 2, 3]);
    g.add_weighted_edge(&1, &2, 2.5).unwrap();
    g.add_weighted_edge(&2, &3, 1.0).unwrap();
    g.add_edge(&2, &1).unwrap();
    assert_eq!(g.out_degree_of(&2), 2);
    assert_eq!(g.in_degree_of(&2), 1);
    assert_eq!(g.successors_of(&1).collect::<Vec<_>>(), [&2]);
    assert_eq!(g.predecessors_of(&3).collect::<Vec<_>>(), [&2]);
    assert!(g.edge_between(&2, &1).is_some() && g.edge_between(&3, &2).is_none());
    assert_eq!(g.total_weight(), 4.5);
    assert!(!props::is_dag(&g));
}

#[test]
fn unweighted_rejects_weights() {
    let mut g = DiGraph::<u32>::simple();
    g.add_vertices([1, 2]);
    let e = g.add_edge(&1, &2).unwrap().unwrap();
    assert_eq!(g.set_edge_weight(&e, 3.0), Err(GraphError::NotWeighted));
    assert_eq!(g.edge_weight(&e), Some(1.0));
}

#[test]
fn dag_prohibits_cycles() {
    let mut g = DiGraph::<u32>::dag();
    g.add_vertices([1, 2, 3]);
    g.add_edge(&1, &2).unwrap();
    g.add_edge(&2, &3).unwrap();
    assert_eq!(g.add_edge(&3, &1), Err(GraphError::CycleProhibited));
    assert!(props::is_dag(&g));
}

#[test]
fn add_graph_copies() {
    let mut a = UGraph::<u32>::simple();
    a.add_edge_with_vertices(1, 2).unwrap();
    a.add_edge_with_vertices(2, 3).unwrap();
    let mut b = UGraph::<u32>::simple();
    assert!(b.add_graph(&a).unwrap());
    assert_eq!((b.vertex_count(), b.edge_count()), (3, 2));
    assert!(props::is_connected(&b) && !props::is_complete(&b));
}
