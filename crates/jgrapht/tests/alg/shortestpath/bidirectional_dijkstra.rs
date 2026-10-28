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

use jgrapht::GraphExt;
use jgrapht::alg::shortestpath::{
    BidirectionalDijkstraShortestPath, Dijkstra, ShortestPathAlgorithm,
};
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::graph::IncidenceGraph;
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::borrow::Borrow;

fn directed_graph() -> DiGraph<&'static str> {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["1", "2", "3", "4", "5"]);
    for (source, target, weight) in [
        ("1", "2", 3.0),
        ("3", "1", 3.0),
        ("2", "4", 3.0),
        ("3", "5", 5.0),
        ("5", "4", 5.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn undirected_graph() -> UGraph<&'static str> {
    let mut graph = UGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["1", "2", "3", "4", "5"]);
    for (source, target, weight) in [
        ("1", "2", 3.0),
        ("3", "1", 3.0),
        ("2", "4", 3.0),
        ("3", "5", 5.0),
        ("5", "4", 5.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn assert_main_path<G: Graph<Vertex = &'static str, Edge = DefaultEdge> + IncidenceGraph>(
    graph: &G,
    radius: f64,
) {
    let algorithm = BidirectionalDijkstraShortestPath::with_radius(graph, radius).unwrap();
    let path = algorithm.path(&"3", &"4").unwrap().unwrap();
    assert_eq!(path.start_vertex(), Some(&"3"));
    assert_eq!(path.end_vertex(), Some(&"4"));
    assert_eq!(path.length(), 3);
    assert_eq!(path.weight(), 9.0);
    assert_eq!(path.vertex_list(), ["3", "1", "2", "4"]);
    assert_eq!(
        path.edge_list(),
        [
            *graph.edge_between(&"3", &"1").unwrap(),
            *graph.edge_between(&"1", &"2").unwrap(),
            *graph.edge_between(&"2", &"4").unwrap(),
        ]
    );

    let materialized = algorithm.single_source(&"3").unwrap();
    assert_eq!(materialized.path(&"4").unwrap(), path);
}

#[test]
fn test_graph_directed() {
    assert_main_path(&directed_graph(), f64::INFINITY);
}

#[test]
fn test_graph_directed_radius() {
    assert_main_path(&directed_graph(), 9.5);
}

#[test]
fn test_graph_undirected() {
    assert_main_path(&undirected_graph(), f64::INFINITY);
}

#[test]
fn test_source_target_equal_undirected() {
    let graph = undirected_graph();
    let path = BidirectionalDijkstraShortestPath::new(&graph)
        .path(&"3", &"3")
        .unwrap()
        .unwrap();
    assert_eq!(path.start_vertex(), Some(&"3"));
    assert_eq!(path.end_vertex(), Some(&"3"));
    assert_eq!(path.length(), 0);
    assert_eq!(path.weight(), 0.0);
    assert_eq!(path.vertex_list(), ["3"]);
    assert!(path.edge_list().is_empty());
}

#[test]
fn test_graph_directed_no_path() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["1", "2", "3", "4", "5"]);
    for (source, target, weight) in [
        ("1", "2", 3.0),
        ("3", "1", 3.0),
        ("4", "2", 3.0),
        ("3", "5", 5.0),
        ("4", "5", 5.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    assert!(
        BidirectionalDijkstraShortestPath::new(&graph)
            .path(&"3", &"4")
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_single_edge_path() {
    let mut graph = DiGraph::<&str>::pseudo();
    graph.add_vertices(["1", "2", "3"]);
    for (source, target) in [("1", "2"), ("1", "3"), ("3", "1")] {
        graph.add_edge(&source, &target).unwrap();
    }
    let path = BidirectionalDijkstraShortestPath::new(&graph)
        .path(&"1", &"2")
        .unwrap()
        .unwrap();
    assert_eq!(path.length(), 1);
    assert_eq!(path.vertex_list(), ["1", "2"]);
    assert_eq!(path.edge_list(), [*graph.edge_between(&"1", &"2").unwrap()]);
}

#[test]
fn test_simple_1() {
    let mut graph = DiGraph::<&str>::pseudo();
    graph.add_vertices(["1", "3", "4"]);
    for (source, target) in [("4", "3"), ("3", "1"), ("4", "1")] {
        graph.add_edge(&source, &target).unwrap();
    }
    let path = BidirectionalDijkstraShortestPath::new(&graph)
        .path(&"4", &"1")
        .unwrap()
        .unwrap();
    assert_eq!(path.length(), 1);
    assert_eq!(path.weight(), 1.0);
    assert_eq!(path.vertex_list(), ["4", "1"]);
    assert_eq!(path.edge_list(), [*graph.edge_between(&"4", &"1").unwrap()]);
}

#[test]
fn test_graph_all_pairs_directed() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(0..=20);
    for range in [0..10, 11..21] {
        for source in range.clone() {
            for target in range.clone() {
                if source != target {
                    graph.add_weighted_edge(&source, &target, 1.0).unwrap();
                }
            }
        }
    }
    graph.add_weighted_edge(&0, &10, 100.0).unwrap();
    graph.add_weighted_edge(&10, &11, 100.0).unwrap();

    let algorithm = BidirectionalDijkstraShortestPath::new(&graph);
    for source in 0..10 {
        for target in 11..21 {
            let path = algorithm.path(&source, &target).unwrap().unwrap();
            let expected = match (source == 0, target == 11) {
                (true, true) => 200.0,
                (true, false) | (false, true) => 201.0,
                (false, false) => 202.0,
            };
            assert_eq!(path.weight(), expected);
        }
    }
}

fn assert_matches_dijkstra<G>(graph: &G, radius: f64)
where
    G: Graph<Vertex = usize, Edge = DefaultEdge> + IncidenceGraph,
{
    let dijkstra = Dijkstra::new(graph).radius(radius);
    let bidirectional = BidirectionalDijkstraShortestPath::with_radius(graph, radius).unwrap();
    for source in graph.vertices() {
        let source = source.borrow();
        for target in graph.vertices() {
            let target = target.borrow();
            let expected = dijkstra.path(source, target).unwrap();
            let actual = bidirectional.path(source, target).unwrap();
            match (expected, actual) {
                (None, None) => {}
                (Some(expected), Some(actual)) => {
                    assert_eq!(expected.length(), actual.length());
                    assert!((expected.weight() - actual.weight()).abs() < 0.0001);
                    assert!(actual.is_valid(graph));
                    assert_eq!(expected.start_vertex(), actual.start_vertex());
                    assert_eq!(expected.end_vertex(), actual.end_vertex());
                }
                pair => panic!("Dijkstra/bidirectional reachability mismatch: {pair:?}"),
            }
        }
    }
}

fn random_directed(weighted: bool) -> DiGraph<usize> {
    let mut graph = if weighted {
        DiGraph::<usize>::pseudo().weighted()
    } else {
        DiGraph::<usize>::pseudo()
    };
    GnmRandomGraphGenerator::new(20, 100, StdRng::seed_from_u64(1), false, false)
        .unwrap()
        .generate_indexed(&mut graph)
        .unwrap();
    if weighted {
        let mut random = StdRng::seed_from_u64(7);
        for edge in graph.edges_cloned().collect::<Vec<_>>() {
            graph.set_edge_weight(&edge, random.random()).unwrap();
        }
    }
    graph
}

#[test]
fn test_random_graphs_directed() {
    assert_matches_dijkstra(&random_directed(false), f64::INFINITY);
}

#[test]
fn test_random_graphs_weighted_directed() {
    assert_matches_dijkstra(&random_directed(true), f64::INFINITY);
}

#[test]
fn test_random_graphs_directed_with_radius() {
    assert_matches_dijkstra(&random_directed(false), 2.5);
}

#[test]
fn test_wrong_parameters() {
    let mut graph = DiGraph::<&str>::pseudo();
    graph.add_vertices(["1", "2"]);
    graph.add_edge(&"1", &"2").unwrap();

    assert!(BidirectionalDijkstraShortestPath::with_radius(&graph, -2.0).is_err());
    let bounded = BidirectionalDijkstraShortestPath::with_radius(&graph, 2.0).unwrap();
    assert!(bounded.path(&"3", &"2").is_err());
    assert!(bounded.path(&"2", &"3").is_err());
}
