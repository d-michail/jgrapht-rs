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

use jgrapht::GraphExt;
use jgrapht::alg::shortestpath::{Dijkstra, FloydWarshallShortestPaths, ShortestPathAlgorithm};
use jgrapht::generate::{GnmRandomGraphGenerator, GnpRandomGraphGenerator, GraphGenerator};
use jgrapht::graph::{IncidenceGraph, OutgoingGraph};
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::borrow::Borrow;

fn verify_all_pairs<G: Graph<Vertex = usize, Edge = DefaultEdge> + OutgoingGraph>(
    graph: &G,
    check_hops: bool,
) {
    let floyd_warshall = FloydWarshallShortestPaths::new(graph).unwrap();
    let dijkstra = Dijkstra::new(graph);
    for source in graph.vertices() {
        let source = source.borrow();
        for target in graph.vertices() {
            let target = target.borrow();
            let expected = dijkstra.path(source, target).unwrap();
            let actual = floyd_warshall.path(source, target).unwrap();
            match (expected, actual) {
                (None, None) => {}
                (Some(expected), Some(actual)) => {
                    assert!((expected.weight() - actual.weight()).abs() < 0.01);
                    assert!(actual.is_valid(graph));
                    if check_hops && !actual.edge_list().is_empty() {
                        assert_eq!(
                            floyd_warshall.first_hop(source, target).as_ref(),
                            actual.vertex_list().get(1)
                        );
                        assert_eq!(
                            floyd_warshall.last_hop(source, target).as_ref(),
                            actual.vertex_list().get(actual.vertex_list().len() - 2)
                        );
                    }
                }
                pair => panic!("Dijkstra/Floyd-Warshall reachability mismatch: {pair:?}"),
            }
        }
    }
}

#[test]
fn test_compare_with_dijkstra() {
    for round in 0..10 {
        let mut directed = DiGraph::<usize>::simple();
        GnmRandomGraphGenerator::new(10, 15, StdRng::seed_from_u64(0xF100 + round), false, false)
            .unwrap()
            .generate_indexed(&mut directed)
            .unwrap();
        verify_all_pairs(&directed, false);

        let mut undirected = UGraph::<usize>::simple();
        GnmRandomGraphGenerator::new(10, 15, StdRng::seed_from_u64(0xF200 + round), false, false)
            .unwrap()
            .generate_indexed(&mut undirected)
            .unwrap();
        verify_all_pairs(&undirected, true);
    }
}

#[test]
fn test_weighted_edges() {
    let mut graph = DiGraph::<&str>::simple().weighted();
    graph.add_vertices(["a", "b"]);
    let edge = graph.add_weighted_edge(&"a", &"b", 5.0).unwrap().unwrap();
    let floyd_warshall = FloydWarshallShortestPaths::new(&graph).unwrap();

    assert_eq!(floyd_warshall.path_weight(&"a", &"b").unwrap(), Some(5.0));
    let path = floyd_warshall.path(&"a", &"b").unwrap().unwrap();
    assert_eq!(path.edge_list(), [edge]);
    assert_eq!(path.start_vertex(), Some(&"a"));
    assert_eq!(path.end_vertex(), Some(&"b"));
    assert_eq!(path.weight(), 5.0);
    assert_eq!(floyd_warshall.first_hop(&"a", &"b"), Some("b"));
    assert_eq!(floyd_warshall.last_hop(&"a", &"b"), Some("a"));
    assert!(floyd_warshall.path(&"b", &"a").unwrap().is_none());
}

fn parallel_fixture() -> (DiGraph<i32>, [DefaultEdge; 9]) {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(1..=4);
    let mut edges = Vec::new();
    for (source, target, weight) in [
        (1, 2, -5.0),
        (1, 2, -2.0),
        (1, 2, 1.0),
        (2, 3, 0.0),
        (2, 3, -2.0),
        (2, 3, -5.0),
        (3, 4, -100.0),
        (3, 4, 100.0),
        (3, 4, 1.0),
    ] {
        edges.push(
            graph
                .add_weighted_edge(&source, &target, weight)
                .unwrap()
                .unwrap(),
        );
    }
    (graph, edges.try_into().unwrap())
}

#[test]
fn pseudograph_test_1() {
    let (graph, [e12, _, _, _, _, e23, e34, _, _]) = parallel_fixture();
    let floyd_warshall = FloydWarshallShortestPaths::new(&graph).unwrap();

    let expected = [
        (1, [0.0, -5.0, -10.0, -110.0]),
        (2, [f64::INFINITY, 0.0, -5.0, -105.0]),
        (3, [f64::INFINITY, f64::INFINITY, 0.0, -100.0]),
        (4, [f64::INFINITY, f64::INFINITY, f64::INFINITY, 0.0]),
    ];
    for (source, weights) in expected {
        let paths = floyd_warshall.single_source(&source).unwrap();
        for (target, weight) in (1..=4).zip(weights) {
            assert_eq!(
                paths.weight(&target),
                Some(weight).filter(|w: &f64| w.is_finite())
            );
        }
        assert!(paths.path(&source).unwrap().edge_list().is_empty());
    }
    let paths1 = floyd_warshall.single_source(&1).unwrap();
    assert_eq!(paths1.path(&2).unwrap().edge_list(), [e12]);
    assert_eq!(paths1.path(&3).unwrap().edge_list(), [e12, e23]);
    assert_eq!(paths1.path(&4).unwrap().edge_list(), [e12, e23, e34]);
    let paths2 = floyd_warshall.single_source(&2).unwrap();
    assert_eq!(paths2.path(&3).unwrap().edge_list(), [e23]);
    assert_eq!(paths2.path(&4).unwrap().edge_list(), [e23, e34]);
    assert_eq!(
        floyd_warshall
            .single_source(&3)
            .unwrap()
            .path(&4)
            .unwrap()
            .edge_list(),
        [e34]
    );
    assert_eq!(floyd_warshall.shortest_paths_count(), 6);

    let expected_first = [
        ((1, 2), Some(2)),
        ((1, 3), Some(2)),
        ((1, 4), Some(2)),
        ((2, 3), Some(3)),
        ((2, 4), Some(3)),
        ((3, 4), Some(4)),
    ];
    let expected_last = [
        ((1, 2), Some(1)),
        ((1, 3), Some(2)),
        ((1, 4), Some(3)),
        ((2, 3), Some(2)),
        ((2, 4), Some(3)),
        ((3, 4), Some(3)),
    ];
    for source in 1..=4 {
        for target in 1..=4 {
            let first = expected_first
                .iter()
                .find_map(|((pair_source, pair_target), value)| {
                    (*pair_source == source && *pair_target == target).then_some(*value)
                })
                .flatten();
            let last = expected_last
                .iter()
                .find_map(|((pair_source, pair_target), value)| {
                    (*pair_source == source && *pair_target == target).then_some(*value)
                })
                .flatten();
            assert_eq!(floyd_warshall.first_hop(&source, &target), first);
            assert_eq!(floyd_warshall.last_hop(&source, &target), last);
        }
    }
}

#[test]
fn pseudograph_test_get_path_weight() {
    let (graph, _) = parallel_fixture();
    let floyd_warshall = FloydWarshallShortestPaths::new(&graph).unwrap();

    assert_eq!(floyd_warshall.path_weight(&2, &1).unwrap(), None);
    assert_eq!(floyd_warshall.path_weight(&2, &2).unwrap(), Some(0.0));
    assert_eq!(floyd_warshall.path_weight(&2, &3).unwrap(), Some(-5.0));
    assert_eq!(floyd_warshall.path_weight(&2, &4).unwrap(), Some(-105.0));
}

#[test]
fn pseudograph_test_loops() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices([1, 2]);
    graph.add_weighted_edge(&1, &2, 5.0).unwrap();
    graph.add_weighted_edge(&2, &1, 15.0).unwrap();
    for vertex in [1, 1, 1, 2, 2, 2] {
        graph.add_edge(&vertex, &vertex).unwrap();
    }
    let floyd_warshall = FloydWarshallShortestPaths::new(&graph).unwrap();

    for vertex in [1, 2] {
        let path = floyd_warshall.path(&vertex, &vertex).unwrap().unwrap();
        assert_eq!(path.start_vertex(), Some(&vertex));
        assert_eq!(path.end_vertex(), Some(&vertex));
        assert_eq!(path.length(), 0);
        assert_eq!(path.weight(), 0.0);
        assert!(path.edge_list().is_empty());
        assert_eq!(path.vertex_list(), [vertex]);
    }
    assert_eq!(floyd_warshall.path_weight(&1, &2).unwrap(), Some(5.0));
    assert_eq!(floyd_warshall.path_weight(&2, &1).unwrap(), Some(15.0));
}

fn random_pseudograph<G>(mut graph: G, seed: u64)
where
    G: Graph<Vertex = usize, Edge = DefaultEdge> + GraphMut + IncidenceGraph,
{
    const N: usize = 50;
    const P: f64 = 0.85;
    let mut generator =
        GnpRandomGraphGenerator::new(N, P, StdRng::seed_from_u64(seed), true).unwrap();
    generator.generate_default(&mut graph).unwrap();
    let mut random = StdRng::seed_from_u64(seed ^ 0x55aa);
    for edge in graph.edges_cloned().collect::<Vec<_>>() {
        graph.set_edge_weight(&edge, random.random()).unwrap();
    }
    let vertices = graph.vertices_cloned().collect::<Vec<_>>();
    for &source in &vertices {
        for _ in 0..(N as f64 * P) as usize {
            let target = vertices[random.random_range(0..vertices.len())];
            graph
                .add_weighted_edge(&source, &target, random.random())
                .unwrap();
        }
    }

    let floyd_warshall = FloydWarshallShortestPaths::new(&graph).unwrap();
    for source in graph.vertices() {
        let source = source.borrow();
        let dijkstra = Dijkstra::new(&graph).single_source(source).unwrap();
        let floyd_paths = floyd_warshall.single_source(source).unwrap();
        for target in graph.vertices() {
            let target = target.borrow();
            assert!(
                (dijkstra.weight(target).unwrap() - floyd_paths.weight(target).unwrap()).abs()
                    < 1e-9
            );
            assert_eq!(
                floyd_paths
                    .path(target)
                    .map(|path| path.edge_list().to_vec()),
                floyd_warshall
                    .path(source, target)
                    .unwrap()
                    .map(|path| path.edge_list().to_vec())
            );
        }
    }
}

#[test]
fn pseudograph_test_random_graphs() {
    for round in 0..20 {
        random_pseudograph(
            DiGraph::<usize>::pseudo()
                .weighted()
                .with_vertex_factory(|index| index as usize),
            0xD100 + round,
        );
        random_pseudograph(
            UGraph::<usize>::pseudo()
                .weighted()
                .with_vertex_factory(|index| index as usize),
            0xD200 + round,
        );
    }
}
