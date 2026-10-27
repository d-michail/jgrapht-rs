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

use super::astar::{LABYRINTH_1, LABYRINTH_2, Node, labyrinth, manhattan};
use jgrapht::alg::shortestpath::{
    ALTAdmissibleHeuristic, BidirectionalAStarShortestPath, DijkstraShortestPath,
    ShortestPathAlgorithm,
};
use jgrapht::graph::sparse::SparseIntUndirectedWeightedGraph;
use jgrapht::{DiGraph, GraphMut, GraphMutExt, UGraph};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::collections::HashSet;

#[test]
fn test_empty_graph() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertex("S");

    let paths = BidirectionalAStarShortestPath::new(&graph, |_: &&str, _: &&str| 0.0)
        .single_source(&"S")
        .unwrap();
    assert_eq!(paths.weight(&"S"), Some(0.0));
    assert_eq!(paths.path(&"S").unwrap().vertex_list(), ["S"]);
}

#[test]
fn test_simple_graph() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["S", "T", "Y", "X", "Z"]);
    for (source, target, weight) in [
        ("S", "T", 10.0),
        ("S", "Y", 5.0),
        ("T", "Y", 2.0),
        ("T", "X", 1.0),
        ("Y", "T", 3.0),
        ("Y", "Z", 2.0),
        ("Y", "X", 9.0),
        ("X", "Z", 4.0),
        ("Z", "X", 6.0),
        ("Z", "S", 7.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    let heuristic = |source: &&str, target: &&str| match (*source, *target) {
        ("S", "Z") => 7.0,
        ("Y", "Z") => 2.0,
        ("T", "Z") | ("X", "Z") => 4.0,
        ("T", "S") => 8.0,
        ("Y", "S") => 5.0,
        ("X", "S") => 11.0,
        ("Z", "S") => 7.0,
        _ => 0.0,
    };

    let path = BidirectionalAStarShortestPath::new(&graph, heuristic)
        .path(&"S", &"Z")
        .unwrap()
        .unwrap();
    assert_eq!(path.vertex_list(), ["S", "Y", "Z"]);

    let materialized = BidirectionalAStarShortestPath::new(&graph, heuristic)
        .single_source(&"S")
        .unwrap();
    assert_eq!(materialized.path(&"Z").unwrap(), path);
}

#[test]
fn test_labyrinth_1() {
    let (graph, source, target) = labyrinth(&LABYRINTH_1);
    let path = BidirectionalAStarShortestPath::new(&graph, manhattan)
        .path(&source, &target)
        .unwrap()
        .unwrap();
    assert_eq!(path.weight(), 47.0);
    assert_eq!(path.edge_list().len(), 47);
    assert_eq!(path.vertex_list().len(), 48);

    let euclidean = |source: &Node, target: &Node| {
        let x = source.0.abs_diff(target.0) as f64;
        let y = source.1.abs_diff(target.1) as f64;
        (x * x + y * y).sqrt()
    };
    let path = BidirectionalAStarShortestPath::new(&graph, euclidean)
        .path(&source, &target)
        .unwrap()
        .unwrap();
    assert_eq!(path.weight(), 47.0);
    assert_eq!(path.edge_list().len(), 47);
}

#[test]
fn test_labyrinth_2() {
    let (graph, source, target) = labyrinth(&LABYRINTH_2);
    assert!(
        BidirectionalAStarShortestPath::new(&graph, manhattan)
            .path(&source, &target)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_multigraph() {
    let mut graph = UGraph::<Node>::multi().weighted();
    let n1 = (0, 0);
    let n2 = (1, 0);
    let n3 = (2, 0);
    graph.add_vertices([n1, n2, n3]);
    for (source, target, weight) in [
        (n1, n2, 5.0),
        (n1, n2, 4.0),
        (n1, n2, 8.0),
        (n2, n3, 7.0),
        (n2, n3, 9.0),
        (n2, n3, 2.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    let path = BidirectionalAStarShortestPath::new(&graph, manhattan)
        .path(&n1, &n3)
        .unwrap()
        .unwrap();
    assert_eq!(path.weight(), 6.0);
    assert_eq!(path.edge_list().len(), 2);
}

#[test]
fn test_inconsistent_heuristic() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(0..=3);
    for (source, target, weight) in [
        (0, 1, 0.5822723681370429),
        (0, 3, 0.8512429683406786),
        (3, 0, 0.22867383417976428),
        (1, 2, 0.1531858692059932),
        (3, 1, 0.9639222864568235),
        (2, 2, 0.23262564370920258),
        (2, 2, 0.6166416559599189),
        (3, 3, 0.6088954021459719),
        (3, 3, 0.2476189990121238),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    let heuristic = |source: &i32, target: &i32| match (*source, *target) {
        (0, 1) => 0.5822723681370429,
        (3, 1) => 0.8109462023168071,
        (3, 2) => 0.9639222864568235,
        (0, 2) => 0.7354582373430361,
        _ => 0.0,
    };

    let path = BidirectionalAStarShortestPath::new(&graph, heuristic)
        .path(&3, &2)
        .unwrap()
        .unwrap();
    assert!((path.weight() - 0.9641320715228003).abs() < 1e-9);
}

#[test]
fn test_random_graphs() {
    const N: usize = 1000;
    const P: f64 = 0.40;

    for iteration in 0..10 {
        let mut random = StdRng::seed_from_u64(0xB1D1_A57A + iteration);
        let mut edges = Vec::new();
        for source in 0..N {
            for target in source + 1..N {
                if random.random::<f64>() < P {
                    edges.push((source, target, random.random_range(0..10) as f64));
                }
            }
        }
        let graph = SparseIntUndirectedWeightedGraph::new(N, &edges).unwrap();

        let mut landmarks = HashSet::new();
        while landmarks.len() < 5 {
            landmarks.insert(random.random_range(0..N));
        }
        let heuristic = ALTAdmissibleHeuristic::new(&graph, landmarks).unwrap();
        let dijkstra = DijkstraShortestPath::new(&graph);
        let bidirectional = BidirectionalAStarShortestPath::new(&graph, |source, target| {
            heuristic.cost_estimate(source, target)
        });

        for _ in 0..10 {
            let source = random.random_range(0..N);
            let target = random.random_range(0..N);
            let expected = dijkstra.path(&source, &target).unwrap();
            let actual = bidirectional.path(&source, &target).unwrap();
            match (expected, actual) {
                (None, None) => {}
                (Some(expected), Some(actual)) => {
                    assert!(
                        (expected.weight() - actual.weight()).abs() < 1e-9,
                        "iteration {iteration}, {source}->{target}: expected {}, got {}",
                        expected.weight(),
                        actual.weight()
                    );
                }
                pair => panic!("iteration {iteration}, {source}->{target}: {pair:?}"),
            }
        }
    }
}
