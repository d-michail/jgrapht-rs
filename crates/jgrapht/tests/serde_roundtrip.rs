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

//! Round trips through `serde_json`, the counterpart of JGraphT's `SerializationTest`
//! (`java.io.Serializable` graphs and result types).
#![cfg(feature = "serde")]

use jgrapht::alg::color::greedy_coloring;
use jgrapht::alg::spanning::kruskal;
use jgrapht::graph::sparse::{
    FullIncoming, NoIncoming, SparseIntDirectedGraph, SparseIntDirectedWeightedGraph,
    SparseIntUndirectedGraph, SparseIntUndirectedWeightedGraph,
};
use jgrapht::graph::{BaseGraph, Directed, Undirected};
use jgrapht::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

fn round_trip<T: Serialize + DeserializeOwned>(value: &T) -> T {
    let json = serde_json::to_string(value).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn assert_same<G>(a: &G, b: &G)
where
    G: GraphExt,
    G::Vertex: std::fmt::Debug,
    G::Edge: std::fmt::Debug,
{
    assert_eq!(
        a.vertices_cloned().collect::<Vec<_>>(),
        b.vertices_cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        a.edges_cloned().collect::<Vec<_>>(),
        b.edges_cloned().collect::<Vec<_>>()
    );
    assert_eq!(a.graph_type(), b.graph_type());
    for e in a.edges_cloned() {
        assert_eq!(a.edge_source(&e), b.edge_source(&e));
        assert_eq!(a.edge_target(&e), b.edge_target(&e));
        assert_eq!(a.edge_weight(&e), b.edge_weight(&e));
    }
}

/// A small graph with a loop and a parallel edge where the graph type allows them.
fn fill<D: graph::Direction>(g: &mut BaseGraph<String, DefaultEdge, D>) {
    let weighted = g.graph_type().weighted;
    g.add_vertices(["a", "b", "c", "d"].map(String::from));
    let mut edges = vec![("a", "b"), ("b", "c"), ("c", "d")];
    if g.graph_type().multiple_edges {
        edges.push(("a", "b"));
    }
    if g.graph_type().self_loops {
        edges.push(("d", "d"));
    }
    for (i, (u, v)) in edges.into_iter().enumerate() {
        let e = g.add_edge(&u.to_string(), &v.to_string()).unwrap().unwrap();
        if weighted {
            g.set_edge_weight(&e, 1.5 + i as f64).unwrap();
        }
    }
}

fn check_type<D: graph::Direction>(ty: GraphType) {
    let mut g = BaseGraph::<String, DefaultEdge, D>::with_graph_type(ty);
    fill(&mut g);
    let copy: BaseGraph<String, DefaultEdge, D> = round_trip(&g);
    assert_same(&g, &copy);
    assert_eq!(g.to_string(), copy.to_string());
}

#[test]
fn all_graph_types_round_trip() {
    for weighted in [false, true] {
        for ty in [
            GraphType::simple(),
            GraphType::multigraph(),
            GraphType::pseudograph(),
            GraphType {
                self_loops: true,
                ..GraphType::simple()
            },
        ] {
            let ty = GraphType { weighted, ..ty };
            check_type::<Undirected>(ty);
            check_type::<Directed>(ty);
        }
    }
}

#[test]
fn deserialized_graph_accepts_new_edges() {
    let mut g = UGraph::<i32>::simple();
    g.add_vertices(0..3);
    g.add_edge(&0, &1).unwrap();
    let mut copy: UGraph<i32> = round_trip(&g);
    // no factory survives serialization
    assert!(copy.add_edge(&1, &2).is_err());
    let mut copy = copy.with_default_edge_factory();
    let fresh = copy.add_edge(&1, &2).unwrap().unwrap();
    assert!(!g.contains_edge(&fresh));
    assert_eq!(copy.edge_count(), 2);
}

#[test]
fn fast_lookups_and_counters_survive() {
    let mut g = DiGraph::<u8>::pseudo();
    g.add_vertices(0..3);
    g.add_edge(&0, &1).unwrap();
    let copy: DiGraph<u8> = round_trip(&g);
    assert_eq!(g.has_fast_lookups(), copy.has_fast_lookups());
}

#[test]
fn union_view_round_trip() {
    let mut a = UGraph::<i32>::simple();
    a.add_vertices(0..3);
    a.add_edge(&0, &1).unwrap();
    let mut b = UGraph::<i32>::simple();
    b.add_vertices(2..5);
    b.add_edge(&2, &3).unwrap();
    let u = jgrapht::view::GraphUnion::new(a, b);
    let copy: jgrapht::view::GraphUnion<UGraph<i32>, UGraph<i32>> = round_trip(&u);
    assert_eq!(u.vertex_count(), copy.vertex_count());
    assert_eq!(u.edge_count(), copy.edge_count());
}

#[test]
fn plain_wrappers_round_trip() {
    let mut g = DiGraph::<i32>::simple();
    g.add_vertices(0..3);
    g.add_edge(&0, &1).unwrap();
    g.add_edge(&1, &2).unwrap();
    let copy: jgrapht::view::Unmodifiable<DiGraph<i32>> =
        round_trip(&jgrapht::view::Unmodifiable(g.clone()));
    assert_same(&g, &copy.0);
    let copy: jgrapht::view::EdgeReversed<DiGraph<i32>> =
        round_trip(&jgrapht::view::EdgeReversed(g.clone()));
    assert_same(&g, &copy.0);
}

#[test]
fn sparse_graphs_round_trip() {
    let edges = [(0, 1), (1, 2), (2, 2), (0, 1)];
    let g = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, &edges).unwrap();
    assert_same(&g, &round_trip(&g));
    let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(4, &edges).unwrap();
    assert_same(&g, &round_trip(&g));
    let g = SparseIntUndirectedGraph::new(4, &edges).unwrap();
    assert_same(&g, &round_trip(&g));

    let w = [(0, 1, 1.5), (1, 2, 2.5), (3, 3, 0.5)];
    let g = SparseIntDirectedWeightedGraph::<FullIncoming>::from_edges(4, &w).unwrap();
    assert_same(&g, &round_trip(&g));
    let g = SparseIntUndirectedWeightedGraph::new(4, &w).unwrap();
    assert_same(&g, &round_trip(&g));
}

#[test]
fn result_types_round_trip() {
    let mut g = UGraph::<i32>::simple().weighted();
    g.add_vertices(0..4);
    for (u, v, w) in [(0, 1, 1.0), (1, 2, 2.0), (2, 3, 3.0), (0, 3, 4.0)] {
        let e = g.add_edge(&u, &v).unwrap().unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }
    let tree = kruskal(&g);
    assert_eq!(tree, round_trip(&tree));

    let coloring = greedy_coloring(&g);
    let copy = round_trip(&coloring);
    assert_eq!(coloring.colors, copy.colors);
    assert_eq!(coloring.count, copy.count);

    let walk = GraphWalk::from_vertex_list(&g, vec![0, 1, 2], 3.0).unwrap();
    let copy = round_trip(&walk);
    assert_eq!(walk.vertex_list(), copy.vertex_list());
    assert_eq!(walk.edge_list(), copy.edge_list());
    assert_eq!(walk.weight(), copy.weight());
}

#[test]
fn wrong_direction_is_rejected() {
    let mut g = DiGraph::<i32>::simple();
    g.add_vertices(0..2);
    g.add_edge(&0, &1).unwrap();
    let json = serde_json::to_string(&g).unwrap();
    let err = serde_json::from_str::<UGraph<i32>>(&json).unwrap_err();
    assert!(err.to_string().contains("directed"), "{err}");
}

#[test]
fn rule_violations_are_rejected() {
    let mut g = DiGraph::<i32>::pseudo();
    g.add_vertices(0..2);
    g.add_edge(&0, &0).unwrap();
    // the same document read as a graph without self-loops
    let json = serde_json::to_string(&g)
        .unwrap()
        .replace("\"self_loops\":true", "\"self_loops\":false");
    assert!(serde_json::from_str::<DiGraph<i32>>(&json).is_err());

    // a cycle in a graph that must stay acyclic
    let mut cyclic = DiGraph::<i32>::simple();
    cyclic.add_vertices(0..2);
    cyclic.add_edge(&0, &1).unwrap();
    cyclic.add_edge(&1, &0).unwrap();
    let json = serde_json::to_string(&cyclic)
        .unwrap()
        .replace("\"allows_cycles\":true", "\"allows_cycles\":false");
    assert!(serde_json::from_str::<DiGraph<i32>>(&json).is_err());

    // duplicate vertex
    let json = r#"{"graph_type":{"directed":true,"self_loops":false,"multiple_edges":false,
        "weighted":false,"allows_cycles":true,"modifiable":true,"mixed":false},
        "vertices":[1,1],"edges":[],"next_edge_id":0,"next_vertex_id":0,"fast_lookups":false}"#;
    assert!(serde_json::from_str::<DiGraph<i32>>(json).is_err());
}

#[test]
fn bad_walk_and_sparse_input_are_rejected() {
    let bad = r#"{"vertices":[1,2,3],"edges":[7],"weight":1.0}"#;
    assert!(serde_json::from_str::<GraphWalk<i32, u8>>(bad).is_err());
    let ok = r#"{"vertices":[1,2],"edges":[7],"weight":1.0}"#;
    assert!(serde_json::from_str::<GraphWalk<i32, u8>>(ok).is_ok());

    let bad = r#"{"num_vertices":2,"edges":[[0,5]]}"#;
    assert!(serde_json::from_str::<SparseIntDirectedGraph>(bad).is_err());
}

#[test]
fn subgraph_round_trip_and_validation() {
    use jgrapht::view::AsSubgraph;
    let mut g = UGraph::<i32>::simple();
    g.add_vertices(0..5);
    for (u, v) in [(0, 1), (1, 2), (2, 3), (3, 4)] {
        g.add_edge(&u, &v).unwrap();
    }
    let sub = AsSubgraph::new(g.clone(), Some(vec![1, 2, 3]), None);
    assert_eq!((sub.vertex_count(), sub.edge_count()), (3, 2));
    let copy: AsSubgraph<UGraph<i32>> = round_trip(&sub);
    assert_eq!(
        sub.vertices_cloned().collect::<Vec<_>>(),
        copy.vertices_cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        sub.edges_cloned().collect::<Vec<_>>(),
        copy.edges_cloned().collect::<Vec<_>>()
    );

    // a vertex that the base graph does not have
    let json = serde_json::to_string(&sub).unwrap();
    let broken = json.replacen("\"vertices\":[1,2,3]", "\"vertices\":[1,2,3,99]", 1);
    assert_ne!(json, broken);
    assert!(serde_json::from_str::<AsSubgraph<UGraph<i32>>>(&broken).is_err());
}
