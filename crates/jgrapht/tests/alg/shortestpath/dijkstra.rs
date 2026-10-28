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

use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::{DefaultEdge, DiGraph, GraphMutExt, UGraph};

fn undirected_fixture() -> (UGraph<&'static str>, [DefaultEdge; 6]) {
    let mut graph = UGraph::<&str>::simple().weighted();
    graph.add_vertices(["v1", "v2", "v3", "v4", "v5"]);
    let edges = [
        graph.add_weighted_edge(&"v1", &"v2", 2.0).unwrap().unwrap(),
        graph.add_weighted_edge(&"v1", &"v3", 3.0).unwrap().unwrap(),
        graph
            .add_weighted_edge(&"v1", &"v5", 100.0)
            .unwrap()
            .unwrap(),
        graph.add_weighted_edge(&"v2", &"v4", 5.0).unwrap().unwrap(),
        graph
            .add_weighted_edge(&"v3", &"v4", 20.0)
            .unwrap()
            .unwrap(),
        graph.add_weighted_edge(&"v4", &"v5", 5.0).unwrap().unwrap(),
    ];
    (graph, edges)
}

#[test]
fn test_path_between() {
    let (graph, [e12, e13, _, e24, _, e45]) = undirected_fixture();
    let algorithm = Dijkstra::new(&graph);

    assert_eq!(
        algorithm.path(&"v1", &"v2").unwrap().unwrap().edge_list(),
        [e12]
    );
    assert_eq!(
        algorithm.path(&"v1", &"v4").unwrap().unwrap().edge_list(),
        [e12, e24]
    );
    assert_eq!(
        algorithm.path(&"v1", &"v5").unwrap().unwrap().edge_list(),
        [e12, e24, e45]
    );
    assert_eq!(
        algorithm.path(&"v3", &"v4").unwrap().unwrap().edge_list(),
        [e13, e12, e24]
    );
}

#[test]
fn test_constructor() {
    let (graph, [e12, e13, _, e24, _, _]) = undirected_fixture();
    let path = Dijkstra::new(&graph)
        .radius(f64::INFINITY)
        .path(&"v3", &"v4")
        .unwrap()
        .unwrap();
    assert_eq!(path.edge_list(), [e13, e12, e24]);
    assert_eq!(path.weight(), 10.0);

    assert!(
        Dijkstra::new(&graph)
            .radius(7.0)
            .path(&"v3", &"v4")
            .unwrap()
            .is_none()
    );
}

fn directed_fixture() -> (DiGraph<&'static str>, [DefaultEdge; 5]) {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["v1", "v2", "v3", "v4", "v5"]);
    let edges = [
        graph.add_weighted_edge(&"v1", &"v2", 3.0).unwrap().unwrap(),
        graph.add_weighted_edge(&"v2", &"v4", 1.0).unwrap().unwrap(),
        graph.add_weighted_edge(&"v1", &"v3", 1.0).unwrap().unwrap(),
        graph.add_weighted_edge(&"v3", &"v2", 1.0).unwrap().unwrap(),
        graph.add_weighted_edge(&"v3", &"v4", 3.0).unwrap().unwrap(),
    ];
    (graph, edges)
}

#[test]
fn test_shortest_path_tree() {
    let (graph, [_, e24, e13, e32, _]) = directed_fixture();
    let paths = Dijkstra::new(&graph).single_source(&"v1").unwrap();

    assert_eq!(paths.source(), &"v1");
    for (vertex, weight) in [
        ("v1", 0.0),
        ("v2", 2.0),
        ("v3", 1.0),
        ("v4", 3.0),
        ("v5", f64::INFINITY),
    ] {
        assert_eq!(
            paths.weight(&vertex),
            Some(weight).filter(|w: &f64| w.is_finite())
        );
    }

    let p11 = paths.path(&"v1").unwrap();
    assert_eq!(p11.start_vertex(), Some(&"v1"));
    assert_eq!(p11.end_vertex(), Some(&"v1"));
    assert_eq!(p11.weight(), 0.0);
    assert!(p11.edge_list().is_empty());

    let p12 = paths.path(&"v2").unwrap();
    assert_eq!(p12.start_vertex(), Some(&"v1"));
    assert_eq!(p12.end_vertex(), Some(&"v2"));
    assert_eq!(p12.weight(), 2.0);
    assert_eq!(p12.edge_list(), [e13, e32]);

    let p13 = paths.path(&"v3").unwrap();
    assert_eq!(p13.weight(), 1.0);
    assert_eq!(p13.edge_list(), [e13]);

    let p14 = paths.path(&"v4").unwrap();
    assert_eq!(p14.weight(), 3.0);
    assert_eq!(p14.edge_list(), [e13, e32, e24]);
    assert!(paths.path(&"v5").is_none());
}

#[test]
fn test_get_path_weight() {
    let (graph, _) = directed_fixture();
    let algorithm = Dijkstra::new(&graph);

    for (target, weight) in [
        ("v1", 0.0),
        ("v2", 2.0),
        ("v3", 1.0),
        ("v4", 3.0),
        ("v5", f64::INFINITY),
    ] {
        assert_eq!(
            algorithm.path_weight(&"v1", &target).unwrap(),
            Some(weight).filter(|w: &f64| w.is_finite())
        );
    }
}

#[test]
fn test_non_negative_weights() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["v1", "v2"]);
    graph.add_weighted_edge(&"v1", &"v2", -100.0).unwrap();

    assert!(Dijkstra::new(&graph).path(&"v1", &"v2").is_err());
}
