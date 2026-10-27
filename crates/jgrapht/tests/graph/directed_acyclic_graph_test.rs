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

//! Port of `org.jgrapht.graph.DirectedAcyclicGraphTest`.
//!
//! The Java class maintains a topological order incrementally and offers a fail-fast iterator;
//! here a DAG is a [`DiGraph`] that refuses cycle-closing edges and the order comes from
//! [`DirectedExt::topological_order`]. `testIterationBehaviors` (`ConcurrentModificationException`)
//! has no Rust counterpart because the borrow checker forbids mutation during iteration.

use jgrapht::alg::cycle::CycleDetector;
use jgrapht::graph::DirectedExt;
use jgrapht::traverse::TopologicalOrder;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use std::collections::{HashMap, HashSet};

/// `RepeatableRandomGraphGenerator`: `edges` distinct random directed edges over `vertices` vertices.
fn repeatable_random<G: GraphMut<Vertex = i64>>(
    g: &mut G,
    vertices: usize,
    edges: usize,
    seed: u64,
) {
    let mut rng = StdRng::seed_from_u64(seed);
    let vs: Vec<i64> = (0..vertices).map(|_| g.add_new_vertex().unwrap()).collect();
    let mut ids = HashSet::new();
    for _ in 0..edges {
        let id = loop {
            let id = rng.random_range(0..vertices * (vertices - 1));
            if ids.insert(id) {
                break id;
            }
        };
        let from = id / vertices;
        let mut to = id % (vertices - 1);
        if to >= from {
            to += 1;
        }
        let _ = g.add_edge(&vs[from], &vs[to]);
    }
}

fn long_dag() -> DiGraph<i64> {
    DiGraph::<i64>::dag().with_vertex_factory(|i| i as i64)
}

#[test]
fn test_cycle_detection_in_random_graph_build() {
    for i in 0..50 {
        let mut source = DiGraph::<i64>::simple().with_vertex_factory(|i| i as i64);
        repeatable_random(&mut source, 20, 200, i);
        let mut dag = DiGraph::<i64>::dag();
        let mut compare = DiGraph::<i64>::simple();
        for v in source.vertices() {
            dag.add_vertex(*v);
            compare.add_vertex(*v);
        }
        for e in source.edges() {
            let (s, t) = (
                *source.edge_source(e).unwrap(),
                *source.edge_target(e).unwrap(),
            );
            let dag_rejected = matches!(dag.add_edge(&s, &t), Err(GraphError::CycleProhibited));
            let compare_edge = compare.add_edge(&s, &t).unwrap();
            let cycle_detected = CycleDetector::new(&compare).detect_cycles();
            assert_eq!(dag_rejected, cycle_detected);
            if cycle_detected {
                compare.remove_edge(&compare_edge.unwrap());
            }
        }
        assert_eq!(
            compare.vertices().collect::<HashSet<_>>(),
            dag.vertices().collect::<HashSet<_>>()
        );
        for sv in compare.vertices() {
            for oe in compare.out_edges_of(sv).unwrap() {
                let tv = compare.edge_target(oe).unwrap();
                assert!(dag.contains_edge_between(sv, tv));
            }
        }
    }
}

#[test]
fn test_topo_iteration_order_linear_graph() {
    let mut dag = long_dag();
    jgrapht::generate::GraphGenerator::generate(
        &mut jgrapht::generate::Linear(100),
        &mut dag,
        |i| i as i64,
    )
    .unwrap();
    let internal: Vec<i64> = dag.topological_order().unwrap();
    let compare: Vec<i64> = TopologicalOrder::new(&dag).copied().collect();
    assert_eq!(internal, compare);
}

#[test]
fn test_topo_iteration_order_complex_graph() {
    for seed in 0..20 {
        let mut dag = long_dag();
        repeatable_random(&mut dag, 100, 500, seed);
        let order = dag.topological_order().unwrap();
        // every edge goes forward in the order
        let pos: HashMap<_, _> = order.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        for e in dag.edges() {
            assert!(pos[dag.edge_source(e).unwrap()] < pos[dag.edge_target(e).unwrap()]);
        }
    }
}

#[test]
fn test_when_vertex_is_not_in_graph_then_throw_exception() {
    let mut dag = DiGraph::<i64>::dag();
    assert_eq!(dag.add_edge(&1, &2), Err(GraphError::VertexNotFound));
}

fn abcd_dag() -> DiGraph<&'static str> {
    let mut g = DiGraph::<&str>::dag();
    g.add_vertices(["A", "B", "C", "D"]);
    g.add_edge(&"A", &"B").unwrap();
    g.add_edge(&"B", &"C").unwrap();
    g.add_edge(&"B", &"D").unwrap();
    g
}

fn abc_triangle_dag() -> DiGraph<&'static str> {
    let mut g = DiGraph::<&str>::dag();
    g.add_vertices(["A", "B", "C"]);
    g.add_edge(&"A", &"B").unwrap();
    g.add_edge(&"B", &"C").unwrap();
    g.add_edge(&"A", &"C").unwrap();
    g
}

#[test]
fn test_determine_ancestors_00() {
    assert_eq!(abcd_dag().ancestors_of(&"C"), HashSet::from(["B", "A"]));
}
#[test]
fn test_determine_ancestors_01() {
    assert!(abcd_dag().ancestors_of(&"A").is_empty());
}
#[test]
fn test_determine_ancestors_02() {
    assert_eq!(
        abc_triangle_dag().ancestors_of(&"C"),
        HashSet::from(["B", "A"])
    );
}
#[test]
fn test_determine_descendants_00() {
    assert_eq!(abcd_dag().descendants_of(&"B"), HashSet::from(["C", "D"]));
}
#[test]
fn test_determine_descendants_01() {
    assert!(abcd_dag().descendants_of(&"C").is_empty());
}
#[test]
fn test_determine_descendants_02() {
    assert_eq!(
        abc_triangle_dag().descendants_of(&"A"),
        HashSet::from(["B", "C"])
    );
}

#[test]
fn test_remove_all_vertices_should_not_delete_topology_if_the_graph_has_vertices_left() {
    let mut dag = DiGraph::<&str>::dag();
    let vertices = ["a", "b", "c", "d", "e"];
    dag.add_vertices(vertices);
    for (s, t) in [("e", "a"), ("e", "b"), ("a", "d"), ("b", "c")] {
        dag.add_edge(&s, &t).unwrap();
    }
    for v in &vertices[..vertices.len() - 2] {
        dag.remove_vertex(v);
    }
    assert!(!dag.topological_order().unwrap().is_empty());
}

#[test]
fn test_multiple_edges_01() {
    let mut graph = DiGraph::<&str>::dag_multi();
    graph.add_vertices(["A", "B", "C"]);
    for (s, t) in [
        ("A", "B"),
        ("A", "B"),
        ("B", "C"),
        ("B", "C"),
        ("A", "C"),
        ("A", "C"),
    ] {
        graph.add_edge(&s, &t).unwrap();
    }
    assert_eq!(graph.edge_count(), 6);
    assert_eq!(graph.descendants_of(&"A"), HashSet::from(["B", "C"]));
    assert_eq!(graph.topological_order().unwrap(), ["A", "B", "C"]);
}

fn set_up_dag_with_multiple_edges(
    levels: usize,
    per_level: usize,
    p: f64,
    rng: &mut StdRng,
) -> DiGraph<i64> {
    let mut g = DiGraph::<i64>::multi().with_vertex_factory(|i| i as i64);
    let mut vertices: Vec<Vec<i64>> = Vec::new();
    for i in 0..levels {
        let row: Vec<i64> = (0..per_level)
            .map(|_| g.add_new_vertex().unwrap())
            .collect();
        vertices.push(row);
        if i == 0 {
            continue;
        }
        for k in 0..per_level {
            for l in 0..per_level {
                if rng.random::<f64>() < p {
                    g.add_edge(&vertices[i - 1][k], &vertices[i][l]).unwrap();
                }
                if rng.random::<f64>() < p {
                    g.add_edge(&vertices[i - 1][k], &vertices[i][l]).unwrap();
                }
            }
        }
    }
    g
}

fn replay_and_test_dag(source: &DiGraph<i64>, rng: &mut StdRng) {
    let mut edges: Vec<DefaultEdge> = source.edges().copied().collect();
    edges.shuffle(rng);
    let mut graph = DiGraph::<i64>::dag_multi();
    for v in source.vertices() {
        graph.add_vertex(*v);
    }
    for e in &edges {
        graph
            .add_edge(
                source.edge_source(e).unwrap(),
                source.edge_target(e).unwrap(),
            )
            .unwrap();
    }
    let topo: HashMap<i64, usize> = graph
        .topological_order()
        .unwrap()
        .into_iter()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    for e in graph.edges() {
        assert!(topo[graph.edge_source(e).unwrap()] < topo[graph.edge_target(e).unwrap()]);
    }
}

#[test]
fn test_multiple_edges_02() {
    let mut rng = StdRng::seed_from_u64(17);
    let source = set_up_dag_with_multiple_edges(20, 20, 0.5, &mut rng);
    replay_and_test_dag(&source, &mut rng);
}

#[test]
fn test_multiple_edges_03() {
    let mut rng = StdRng::from_os_rng();
    for p in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7] {
        let source = set_up_dag_with_multiple_edges(20, 20, p, &mut rng);
        replay_and_test_dag(&source, &mut rng);
    }
}

// `testFastLookupGraphSpecificsStrategyAndArrayUnenforcedSet` / `...AndHashSet` only check that a
// DAG built with a different edge-set strategy still iterates a -> b; all graphs here share one
// strategy, so a single check stands in for both.
#[test]
fn test_fast_lookup_graph_specifics_strategy() {
    let mut graph = DiGraph::<&str>::dag();
    graph.add_vertices(["A", "B"]);
    graph.add_edge(&"A", &"B").unwrap();
    assert_eq!(graph.topological_order().unwrap(), ["A", "B"]);
}
