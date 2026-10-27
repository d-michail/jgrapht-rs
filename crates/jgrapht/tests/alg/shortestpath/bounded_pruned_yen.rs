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

use jgrapht::alg::shortestpath::{
    AStarSpurEngine, BoundedPrunedYenKShortestPath, DijkstraSpurEngine, YenKShortestPath,
};
use jgrapht::{DiGraph, Graph, GraphMut, GraphMutExt};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn graph(vertices: std::ops::RangeInclusive<u32>, edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for vertex in vertices {
        graph.add_vertex(vertex);
    }
    for &(source, target, weight) in edges {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn weights(paths: &[jgrapht::GraphWalk<u32, jgrapht::DefaultEdge>]) -> Vec<f64> {
    paths.iter().map(|path| path.weight).collect()
}

fn assert_matches(graph: &DiGraph<u32>, source: u32, sink: u32, k: usize) {
    let expected = YenKShortestPath::new(graph)
        .get_paths(&source, &sink, k)
        .unwrap();
    let bounded = BoundedPrunedYenKShortestPath::new(graph);
    let actual = bounded.get_paths(&source, &sink, k).unwrap();
    assert_eq!(weights(&actual), weights(&expected));
    assert!(
        actual
            .windows(2)
            .all(|pair| pair[0].weight <= pair[1].weight)
    );
}

fn assert_matches_both(graph: &DiGraph<u32>, source: u32, sink: u32, k: usize) {
    let expected = weights(
        &YenKShortestPath::new(graph)
            .get_paths(&source, &sink, k)
            .unwrap(),
    );
    let dijkstra = BoundedPrunedYenKShortestPath::with_engine(graph, DijkstraSpurEngine::new());
    let astar = BoundedPrunedYenKShortestPath::with_engine(graph, AStarSpurEngine::new());
    assert_eq!(
        weights(&dijkstra.get_paths(&source, &sink, k).unwrap()),
        expected
    );
    assert_eq!(
        weights(&astar.get_paths(&source, &sink, k).unwrap()),
        expected
    );
    assert_eq!(dijkstra.engine_name(), "Dijkstra");
    assert_eq!(astar.engine_name(), "AStar");
    if source != sink && !expected.is_empty() {
        assert!(astar.stats().expanded_vertices > 0);
    }
}

fn layered_dag(layers: u32, width: u32, seed: u64) -> DiGraph<u32> {
    let sink = 1 + layers * width;
    let mut graph = graph(0..=sink, &[]);
    let mut rng = StdRng::seed_from_u64(seed);
    for target in 1..=width {
        graph
            .add_weighted_edge(&0, &target, rng.random_range(1..=5) as f64)
            .unwrap();
    }
    for layer in 0..layers - 1 {
        let first = 1 + layer * width;
        let next = first + width;
        for source in first..first + width {
            for target in next..next + width {
                if rng.random_bool(0.5) {
                    graph
                        .add_weighted_edge(&source, &target, rng.random_range(1..=5) as f64)
                        .unwrap();
                }
            }
        }
    }
    let last = 1 + (layers - 1) * width;
    for source in last..last + width {
        graph
            .add_weighted_edge(&source, &sink, rng.random_range(1..=5) as f64)
            .unwrap();
    }
    graph
}

fn random_dag_with_chain(n: u32, extra_edges: u32, seed: u64) -> DiGraph<u32> {
    let mut graph = graph(0..=n - 1, &[]);
    let mut rng = StdRng::seed_from_u64(seed);
    for source in 0..n - 1 {
        graph
            .add_weighted_edge(&source, &(source + 1), rng.random_range(1..=5) as f64)
            .unwrap();
    }
    let mut placed = 0;
    let mut attempts = 0;
    while placed < extra_edges && attempts < extra_edges * 10 + 50 {
        attempts += 1;
        let source = rng.random_range(0..n - 1);
        let target = rng.random_range(source + 1..n);
        if graph.contains_edge_between(&source, &target) {
            continue;
        }
        graph
            .add_weighted_edge(&source, &target, rng.random_range(1..=10) as f64)
            .unwrap();
        placed += 1;
    }
    graph
}

fn random_digraph(n: u32, edges: u32, seed: u64) -> DiGraph<u32> {
    let mut graph = graph(0..=n - 1, &[]);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut placed = 0;
    let mut attempts = 0;
    while placed < edges && attempts < edges * 10 + 50 {
        attempts += 1;
        let source = rng.random_range(0..n);
        let target = rng.random_range(0..n);
        if source == target || graph.contains_edge_between(&source, &target) {
            continue;
        }
        graph
            .add_weighted_edge(&source, &target, rng.random_range(1..=9) as f64)
            .unwrap();
        placed += 1;
    }
    graph
}

fn grid(rows: u32, columns: u32, seed: u64) -> DiGraph<u32> {
    let mut graph = graph(0..=rows * columns - 1, &[]);
    let mut rng = StdRng::seed_from_u64(seed);
    for row in 0..rows {
        for column in 0..columns {
            let source = row * columns + column;
            if column + 1 < columns {
                graph
                    .add_weighted_edge(&source, &(source + 1), rng.random_range(1..=3) as f64)
                    .unwrap();
            }
            if row + 1 < rows {
                graph
                    .add_weighted_edge(&source, &(source + columns), rng.random_range(1..=3) as f64)
                    .unwrap();
            }
        }
    }
    graph
}

#[test]
fn diamond_equal_weight_and_long_direct_path_match_yen() {
    let graph = graph(
        1..=4,
        &[
            (1, 2, 1.0),
            (2, 4, 2.0),
            (1, 3, 2.0),
            (3, 4, 1.0),
            (1, 4, 10.0),
        ],
    );
    assert_matches(&graph, 1, 4, 100);
}

#[test]
fn cyclic_and_zero_weight_graphs_match_yen() {
    let cyclic = graph(
        1..=4,
        &[
            (1, 2, 0.0),
            (2, 3, 0.0),
            (3, 1, 0.0),
            (3, 4, 1.0),
            (2, 4, 3.0),
            (1, 4, 5.0),
        ],
    );
    assert_matches(&cyclic, 1, 4, 20);
}

#[test]
fn source_sink_and_zero_k_edge_cases_match_yen() {
    let graph = graph(1..=2, &[(1, 2, 7.5)]);
    assert_matches(&graph, 1, 1, 3);
    assert_matches(&graph, 1, 2, 5);
    assert!(
        BoundedPrunedYenKShortestPath::new(&graph)
            .get_paths(&1, &2, 0)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unreachable_sink_is_empty() {
    let graph = graph(1..=2, &[]);
    let bounded = BoundedPrunedYenKShortestPath::new(&graph);
    assert!(bounded.get_paths(&1, &2, 5).unwrap().is_empty());
    assert_eq!(bounded.stats().shortest_path_calls, 1);
}

#[test]
fn path_chain_skips_every_impossible_spur() {
    let n = 100u32;
    let edges: Vec<_> = (0..n - 1).map(|v| (v, v + 1, 1.0)).collect();
    let graph = graph(0..=n - 1, &edges);
    let bounded = BoundedPrunedYenKShortestPath::new(&graph);
    let paths = bounded.get_paths(&0, &(n - 1), 5).unwrap();
    assert_eq!(paths.len(), 1);
    let stats = bounded.stats();
    assert_eq!(stats.spur_tasks_created, 0);
    assert_eq!(stats.skipped_impossible_spur_tasks, (n - 1) as usize);
}

#[test]
fn layered_dags_match_for_several_k_and_have_coherent_stats() {
    let mut graph = DiGraph::<u32>::simple().weighted();
    let layers = 5u32;
    let width = 4u32;
    let sink = 1 + layers * width;
    for vertex in 0..=sink {
        graph.add_vertex(vertex);
    }
    let mut rng = StdRng::seed_from_u64(42);
    for target in 1..=width {
        graph
            .add_weighted_edge(&0, &target, rng.random_range(1..20) as f64)
            .unwrap();
    }
    for layer in 0..layers - 1 {
        let first = 1 + layer * width;
        let next = first + width;
        for source in first..first + width {
            for target in next..next + width {
                graph
                    .add_weighted_edge(&source, &target, rng.random_range(1..20) as f64)
                    .unwrap();
            }
        }
    }
    let last = 1 + (layers - 1) * width;
    for source in last..last + width {
        graph
            .add_weighted_edge(&source, &sink, rng.random_range(1..20) as f64)
            .unwrap();
    }

    for k in [1, 2, 5, 10, 30] {
        assert_matches(&graph, 0, sink, k);
    }
    let bounded = BoundedPrunedYenKShortestPath::new(&graph);
    bounded.get_paths(&0, &sink, 25).unwrap();
    let stats = bounded.stats();
    assert!(stats.shortest_path_calls > 0);
    assert!(stats.spur_tasks_materialized <= stats.spur_tasks_created);
    assert!(stats.materialized_candidates <= stats.spur_tasks_materialized);
}

#[test]
fn negative_weight_reachable_to_sink_is_rejected() {
    let graph = graph(1..=2, &[(1, 2, -1.0)]);
    assert!(
        BoundedPrunedYenKShortestPath::new(&graph)
            .get_paths(&1, &2, 1)
            .is_err()
    );
}

#[test]
fn tie_equal_weights_matches_standard_yen() {
    let graph = graph(1..=4, &[(1, 2, 1.0), (2, 4, 1.0), (1, 3, 1.0), (3, 4, 1.0)]);
    assert_matches_both(&graph, 1, 4, 4);
}

#[test]
fn random_dag_and_grid_match_standard_yen() {
    assert_matches_both(&random_dag_with_chain(80, 241, 99), 0, 79, 20);
    assert_matches_both(&grid(8, 8, 7), 0, 63, 10);
}

#[test]
fn astar_engine_matches_standard_yen_and_counts_expansions() {
    let graph = layered_dag(5, 5, 42);
    assert_matches_both(&graph, 0, 26, 15);
}

#[test]
fn weights_are_non_decreasing() {
    let graph = layered_dag(6, 4, 1);
    let paths = BoundedPrunedYenKShortestPath::new(&graph)
        .get_paths(&0, &25, 30)
        .unwrap();
    assert!(
        paths
            .windows(2)
            .all(|pair| pair[0].weight <= pair[1].weight)
    );
}

#[test]
fn large_k_dense_dag_matches_standard_yen() {
    let graph = layered_dag(4, 5, 314);
    assert_matches_both(&graph, 0, 21, 500);
}

#[test]
fn both_engines_match_standard_yen_across_seeds() {
    for seed in 1..=5 {
        let graph = layered_dag(5, 4, seed);
        assert_matches_both(&graph, 0, 21, 12);
    }
}

#[test]
fn fuzz_random_dags_and_cyclic_graphs_match_standard_yen() {
    let mut meta = StdRng::seed_from_u64(20_260_509);
    for _ in 0..40 {
        let n = meta.random_range(8..=87);
        let extras = meta.random_range(0..2 * n);
        let k = meta.random_range(1..=25) as usize;
        let seed = meta.random();
        assert_matches_both(&random_dag_with_chain(n, extras, seed), 0, n - 1, k);
    }
    for _ in 0..30 {
        let n = meta.random_range(6..=45);
        let edges = n + meta.random_range(0..2 * n);
        let k = meta.random_range(1..=20) as usize;
        let seed = meta.random();
        assert_matches_both(&random_digraph(n, edges, seed), 0, n - 1, k);
    }
}

#[test]
fn fuzz_grids_and_chains_match_standard_yen() {
    for seed in [11, 22, 33, 44, 55] {
        for rows in [3, 5, 8] {
            for columns in [3, 5, 8] {
                let graph = grid(rows, columns, seed);
                assert_matches_both(&graph, 0, rows * columns - 1, 8);
            }
        }
    }
    let chain = random_dag_with_chain(20, 0, 99);
    assert_matches_both(&chain, 0, 19, 5);
}
