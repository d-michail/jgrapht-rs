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
    ALTAdmissibleHeuristic, AStarShortestPath, BFSShortestPath, DijkstraClosestFirstIterator,
    DijkstraShortestPath, ShortestPathAlgorithm, TreeMeasurer,
};
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

fn bfs_graph() -> (DiGraph<&'static str>, [DefaultEdge; 5]) {
    let mut graph = DiGraph::<&str>::simple();
    for vertex in ["v1", "v2", "v3", "v4", "v5"] {
        graph.add_vertex(vertex);
    }
    let e12 = graph.add_edge(&"v1", &"v2").unwrap().unwrap();
    let e13 = graph.add_edge(&"v1", &"v3").unwrap().unwrap();
    let e24 = graph.add_edge(&"v2", &"v4").unwrap().unwrap();
    let e35 = graph.add_edge(&"v3", &"v5").unwrap().unwrap();
    let e45 = graph.add_edge(&"v4", &"v5").unwrap().unwrap();
    (graph, [e12, e13, e24, e35, e45])
}

#[test]
fn bfs_shortest_path_test_path_between() {
    let (graph, [e12, e13, e24, e35, _]) = bfs_graph();
    let bfs = BFSShortestPath::new(&graph);

    assert_eq!(bfs.path(&"v1", &"v2").unwrap().unwrap().edge_list(), [e12]);
    assert_eq!(
        bfs.path(&"v1", &"v4").unwrap().unwrap().edge_list(),
        [e12, e24]
    );
    assert_eq!(
        bfs.path(&"v1", &"v5").unwrap().unwrap().edge_list(),
        [e13, e35]
    );
    assert!(bfs.path(&"v4", &"v3").unwrap().is_none());
}

#[test]
fn bfs_shortest_path_test_all_paths() {
    let (graph, [e12, e13, e24, e35, _]) = bfs_graph();
    let paths = BFSShortestPath::new(&graph).single_source(&"v1").unwrap();

    assert!(paths.path(&"v1").unwrap().edge_list().is_empty());
    assert_eq!(paths.path(&"v2").unwrap().edge_list(), [e12]);
    assert_eq!(paths.path(&"v3").unwrap().edge_list(), [e13]);
    assert_eq!(paths.path(&"v4").unwrap().edge_list(), [e12, e24]);
    assert_eq!(paths.path(&"v5").unwrap().edge_list(), [e13, e35]);
}

#[test]
fn dijkstra_closest_first_iterator_test_undirected() {
    let mut graph = UGraph::<&str>::pseudo().weighted();
    for vertex in ["1", "2", "3", "4", "5"] {
        graph.add_vertex(vertex);
    }
    for (source, target, weight) in [
        ("1", "2", 2.0),
        ("1", "3", 3.0),
        ("1", "5", 100.0),
        ("2", "4", 5.0),
        ("3", "4", 20.0),
        ("4", "5", 5.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    assert_eq!(
        DijkstraClosestFirstIterator::new(&graph, &"3")
            .unwrap()
            .collect::<Vec<_>>(),
        ["3", "1", "2", "4", "5"]
    );
    assert_eq!(
        DijkstraClosestFirstIterator::new(&graph, &"1")
            .unwrap()
            .collect::<Vec<_>>(),
        ["1", "2", "3", "4", "5"]
    );
    assert_eq!(
        DijkstraClosestFirstIterator::with_radius(&graph, &"1", 11.0)
            .unwrap()
            .collect::<Vec<_>>(),
        ["1", "2", "3", "4"]
    );

    let mut bounded = DijkstraClosestFirstIterator::with_radius(&graph, &"3", 12.0).unwrap();
    assert_eq!(bounded.by_ref().collect::<Vec<_>>(), ["3", "1", "2", "4"]);
    let paths = bounded.paths();
    assert_eq!(paths.weight(&"4"), Some(10.0));
    assert_eq!(paths.weight(&"2"), Some(5.0));
    assert_eq!(paths.weight(&"1"), Some(3.0));
}

fn fill_java_gnp<G>(graph: &mut G, random: &mut StdRng, vertex_count: i32, probability: f64)
where
    G: GraphMut<Vertex = i32, Edge = DefaultEdge>,
{
    for vertex in 0..vertex_count {
        graph.add_vertex(vertex);
    }
    let directed = graph.graph_type().directed;
    for source in 0..vertex_count {
        for target in source..vertex_count {
            if random.random::<f64>() < probability {
                graph.add_edge(&source, &target).unwrap();
            }
            if directed && random.random::<f64>() < probability {
                graph.add_edge(&target, &source).unwrap();
            }
        }
    }
    let edges = graph.edges_cloned().collect::<Vec<_>>();
    for edge in edges {
        graph
            .set_edge_weight(&edge, random.random::<f64>())
            .unwrap();
    }
}

fn assert_same_weight(expected: f64, actual: f64) {
    assert!(
        expected == actual || (expected - actual).abs() < 1e-9,
        "expected path weight {expected}, got {actual}"
    );
}

fn check_alt_paths<G: Graph<Vertex = i32, Edge = DefaultEdge> + IncidenceGraph>(
    graph: &G,
    random: &mut StdRng,
) {
    let mut landmarks = HashSet::new();
    while landmarks.len() < 2 {
        landmarks.insert(random.random_range(0..30));
    }
    let heuristic = ALTAdmissibleHeuristic::new(graph, landmarks).unwrap();
    let dijkstra = DijkstraShortestPath::new(graph);
    let astar = AStarShortestPath::new(graph, |source, target| {
        heuristic.cost_estimate(source, target)
    });
    for source in graph.vertices() {
        for target in graph.vertices() {
            let expected = dijkstra
                .path_weight(source, target)
                .unwrap()
                .unwrap_or(f64::INFINITY);
            let actual = astar
                .path(source, target)
                .unwrap()
                .map_or(f64::INFINITY, |path| path.weight());
            assert_same_weight(expected, actual);
        }
    }
}

#[test]
fn alt_admissible_heuristic_test_random() {
    let mut random = StdRng::seed_from_u64(47);
    for _ in 0..3 {
        let mut graph = DiGraph::<i32>::pseudo().weighted();
        fill_java_gnp(&mut graph, &mut random, 30, 0.35);
        check_alt_paths(&graph, &mut random);
    }
    for _ in 0..3 {
        let mut graph = UGraph::<i32>::pseudo().weighted();
        fill_java_gnp(&mut graph, &mut random, 30, 0.35);
        check_alt_paths(&graph, &mut random);
    }
}

fn check_alt_admissibility<G: Graph<Vertex = i32, Edge = DefaultEdge> + IncidenceGraph>(graph: &G) {
    for landmark in graph.vertices() {
        let heuristic = ALTAdmissibleHeuristic::new(graph, [*landmark]).unwrap();
        for source in graph.vertices() {
            let paths = DijkstraShortestPath::new(graph)
                .single_source(source)
                .unwrap();
            for target in graph.vertices() {
                let estimate = heuristic.cost_estimate(source, target);
                let distance = paths.weight(target).unwrap_or(f64::INFINITY);
                assert!(
                    estimate <= distance + 1e-9
                        || (estimate.is_infinite() && distance.is_infinite()),
                    "estimate {estimate} exceeds distance {distance} from {source} to {target}"
                );
            }
        }
    }
}

#[test]
fn alt_admissible_heuristic_test_random_admissible() {
    let mut random = StdRng::seed_from_u64(33);
    for _ in 0..3 {
        let mut graph = DiGraph::<i32>::pseudo().weighted();
        fill_java_gnp(&mut graph, &mut random, 35, 0.3);
        check_alt_admissibility(&graph);
    }
    for _ in 0..3 {
        let mut graph = UGraph::<i32>::pseudo().weighted();
        fill_java_gnp(&mut graph, &mut random, 35, 0.3);
        check_alt_admissibility(&graph);
    }
}

fn center_set(graph: &UGraph<i32>) -> HashSet<i32> {
    TreeMeasurer::new(graph)
        .graph_center()
        .unwrap()
        .into_iter()
        .collect()
}

#[test]
fn tree_measurer_test_no_centers() {
    assert!(center_set(&UGraph::simple()).is_empty());
}

#[test]
fn tree_measurer_test_two_centers() {
    let mut tree = UGraph::<i32>::simple();
    for vertex in 1..=4 {
        tree.add_vertex(vertex);
    }
    for (source, target) in [(1, 2), (2, 3), (3, 4)] {
        tree.add_edge(&source, &target).unwrap();
    }
    assert_eq!(center_set(&tree), HashSet::from([2, 3]));
}

#[test]
fn tree_measurer_test_one_center() {
    let mut tree = UGraph::<i32>::simple();
    for vertex in 1..=5 {
        tree.add_vertex(vertex);
    }
    for (source, target) in [(1, 2), (2, 3), (3, 4), (4, 5)] {
        tree.add_edge(&source, &target).unwrap();
    }
    assert_eq!(center_set(&tree), HashSet::from([3]));
}
