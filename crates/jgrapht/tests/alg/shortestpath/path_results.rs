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
    AStar, AltAdmissibleHeuristic, BidirectionalDijkstraShortestPath, Dijkstra,
    ListSingleSourcePaths, ShortestPathAlgorithm, SingleSourcePaths,
};
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

fn assert_same_weight(expected: f64, actual: f64) {
    assert!(
        expected == actual || (expected - actual).abs() < 1e-9,
        "expected path weight {expected}, got {actual}"
    );
}

fn java_directed_gnp(
    random: &mut StdRng,
    vertex_count: i32,
    probability: f64,
    create_loops: bool,
) -> DiGraph<i32> {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    for vertex in 0..vertex_count {
        graph.add_vertex(vertex);
    }
    // GnpRandomGraphGenerator considers each unordered pair once, drawing for
    // the forward and reverse arcs consecutively. For a directed loop it also
    // makes both draws, so a pseudograph can receive two parallel loops.
    for source in 0..vertex_count {
        for target in source..vertex_count {
            if source == target && !create_loops {
                continue;
            }
            if random.random::<f64>() < probability {
                graph.add_edge(&source, &target).unwrap();
            }
            if random.random::<f64>() < probability {
                graph.add_edge(&target, &source).unwrap();
            }
        }
    }
    graph
}

// ListSingleSourcePathsTest.testPaths, with a fixed seed in place of Java's
// deliberately unseeded random graph.
#[test]
fn list_single_source_paths_test_paths() {
    let mut random = StdRng::seed_from_u64(0x5eed);
    let graph = java_directed_gnp(&mut random, 50, 0.7, false);

    let dijkstra = Dijkstra::new(&graph);
    let expected: Vec<_> = (1..50)
        .map(|target| dijkstra.path(&0, &target).unwrap().unwrap())
        .collect();
    let materialized = expected
        .iter()
        .map(|path| (*path.end_vertex().unwrap(), path.clone()))
        .collect::<HashMap<_, _>>();
    let paths = ListSingleSourcePaths::new(0, materialized);

    assert_eq!(paths.source(), &0);
    assert_eq!(paths.weight(&0), Some(0.0));
    assert_eq!(paths.path(&0).unwrap().vertex_list(), [0]);
    for (target, expected_path) in (1..50).zip(expected) {
        let actual = paths.path(&target).unwrap();
        assert_eq!(paths.weight(&target), Some(expected_path.weight()));
        assert_eq!(actual.edge_list(), expected_path.edge_list());
    }
    assert_eq!(paths.weight(&50), None);
    assert!(paths.path(&50).is_none());
}

// TreeSingleSourcePathsTest.testPaths.
#[test]
fn tree_single_source_paths_test_paths() {
    let mut graph = DiGraph::<i32>::multi().weighted();
    for vertex in 1..=4 {
        graph.add_vertex(vertex);
    }

    let e12 =
        [-5.0, -2.0, 1.0].map(|weight| graph.add_weighted_edge(&1, &2, weight).unwrap().unwrap());
    let e23 =
        [0.0, -2.0, -5.0].map(|weight| graph.add_weighted_edge(&2, &3, weight).unwrap().unwrap());
    let e34 = [-100.0, 100.0, 1.0]
        .map(|weight| graph.add_weighted_edge(&3, &4, weight).unwrap().unwrap());

    let paths = SingleSourcePaths {
        source: 1,
        dist: HashMap::from([(1, 0.0), (2, -5.0), (3, -10.0), (4, -110.0)]),
        pred: HashMap::from([(2, (e12[0], 1)), (3, (e23[2], 2)), (4, (e34[0], 3))]),
    };

    assert_eq!(paths.source(), &1);
    assert_eq!(paths.path(&1).unwrap().vertex_list(), [1]);
    assert_eq!(paths.weight(&1), Some(0.0));
    assert_eq!(paths.path(&2).unwrap().edge_list(), [e12[0]]);
    assert_eq!(paths.weight(&2), Some(-5.0));
    assert_eq!(paths.path(&3).unwrap().edge_list(), [e12[0], e23[2]]);
    assert_eq!(paths.weight(&3), Some(-10.0));
    assert_eq!(
        paths.path(&4).unwrap().edge_list(),
        [e12[0], e23[2], e34[0]]
    );
    assert_eq!(paths.weight(&4), Some(-110.0));
}

fn all_pairs_shortest_paths(seed: i64) {
    let mut random = StdRng::seed_from_u64(seed as u64);
    for _ in 0..5 {
        let mut graph = java_directed_gnp(&mut random, 20, 0.35, true);
        let edges = graph.edges_cloned().collect::<Vec<_>>();
        for edge in edges {
            graph
                .set_edge_weight(&edge, random.random::<f64>())
                .unwrap();
        }

        let mut landmark_set = HashSet::new();
        while landmark_set.len() < 2 {
            landmark_set.insert(random.random_range(0..20));
        }
        let heuristic = AltAdmissibleHeuristic::new(&graph, landmark_set).unwrap();
        let dijkstra = Dijkstra::new(&graph);
        let bidirectional = BidirectionalDijkstraShortestPath::new(&graph);
        let astar = AStar::new(&graph, |_: &i32, _: &i32| 0.0);
        let alt_astar = AStar::new(&graph, |source, target| {
            heuristic.cost_estimate(source, target)
        });

        for source in 0..20 {
            for target in 0..20 {
                let expected = dijkstra
                    .path_weight(&source, &target)
                    .unwrap()
                    .unwrap_or(f64::INFINITY);
                assert_same_weight(
                    expected,
                    bidirectional
                        .path_weight(&source, &target)
                        .unwrap()
                        .unwrap_or(f64::INFINITY),
                );
                assert_same_weight(
                    expected,
                    astar
                        .path(&source, &target)
                        .unwrap()
                        .map_or(f64::INFINITY, |path| path.weight()),
                );
                assert_same_weight(
                    expected,
                    alt_astar
                        .path(&source, &target)
                        .unwrap()
                        .map_or(f64::INFINITY, |path| path.weight()),
                );
            }
        }
    }
}

#[test]
fn all_pairs_shortest_paths_seed_47() {
    all_pairs_shortest_paths(47);
}

#[test]
fn all_pairs_shortest_paths_seed_8() {
    all_pairs_shortest_paths(8);
}

#[test]
fn all_pairs_shortest_paths_seed_13() {
    all_pairs_shortest_paths(13);
}

#[test]
fn all_pairs_shortest_paths_seed_17() {
    all_pairs_shortest_paths(17);
}
