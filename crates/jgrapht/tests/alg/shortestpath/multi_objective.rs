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

use jgrapht::alg::shortestpath::{LabelCorrectingMultiObjectiveShortestPath, MartinShortestPath};
use jgrapht::{DefaultEdge, DiGraph, GraphMut, GraphMutExt};
use std::collections::HashMap;

fn five_vertex_graph() -> (DiGraph<i32>, HashMap<DefaultEdge, Vec<f64>>) {
    let mut graph = DiGraph::<i32>::pseudo();
    graph.add_vertices(1..=5);
    let mut weights = HashMap::new();
    for (source, target, value) in [
        (1, 2, [1.0, 5.0]),
        (1, 3, [4.0, 2.0]),
        (1, 4, [4.0, 4.0]),
        (2, 4, [1.0, 2.0]),
        (2, 5, [2.0, 5.0]),
        (3, 4, [2.0, 3.0]),
        (3, 5, [6.0, 1.0]),
        (4, 5, [3.0, 3.0]),
    ] {
        let edge = graph.add_edge(&source, &target).unwrap().unwrap();
        weights.insert(edge, value.to_vec());
    }
    (graph, weights)
}

fn three_vertex_graph(
    direct: [f64; 2],
    first: [f64; 2],
    second: [f64; 2],
) -> (DiGraph<i32>, HashMap<DefaultEdge, Vec<f64>>) {
    let mut graph = DiGraph::<i32>::pseudo();
    graph.add_vertices(1..=3);
    let mut weights = HashMap::new();
    for (source, target, value) in [(1, 2, direct), (1, 3, first), (3, 2, second)] {
        let edge = graph.add_edge(&source, &target).unwrap().unwrap();
        weights.insert(edge, value.to_vec());
    }
    (graph, weights)
}

#[test]
fn martin_test_graph_directed() {
    let (graph, weights) = five_vertex_graph();
    let paths = MartinShortestPath::new(&graph, |edge| weights[edge].clone())
        .paths_from(&1)
        .unwrap();

    assert_eq!(paths.paths(&1).len(), 1);
    assert_eq!(paths.paths(&2).len(), 1);
    assert_eq!(paths.paths(&3).len(), 1);
    assert_eq!(paths.paths(&4).len(), 2);
    assert_eq!(paths.paths(&5).len(), 3);
}

#[test]
fn martin_test_no_paths() {
    let mut graph = DiGraph::<i32>::pseudo();
    graph.add_vertices([1, 2]);
    let paths1 = MartinShortestPath::new(&graph, |_| vec![0.0, 0.0])
        .paths_from(&1)
        .unwrap();
    assert_eq!(paths1.paths(&1).len(), 1);
    assert!(paths1.paths(&2).is_empty());

    let paths2 = MartinShortestPath::new(&graph, |_| vec![0.0, 0.0])
        .paths_from(&2)
        .unwrap();
    assert!(paths2.paths(&1).is_empty());
    assert_eq!(paths2.paths(&2).len(), 1);
}

#[test]
fn label_correcting_test_graph_directed() {
    let (graph, weights) = three_vertex_graph([5.0, 2.0], [2.0, 8.0], [1.0, 4.0]);
    let paths =
        LabelCorrectingMultiObjectiveShortestPath::new(&graph, |edge| weights[edge].clone())
            .paths_from(&1)
            .unwrap();

    assert_eq!(paths.paths(&2).len(), 2);
}

#[test]
fn label_correcting_test_dominated_paths() {
    let (graph, weights) = three_vertex_graph([3.0, 3.0], [5.0, 5.0], [5.0, 5.0]);
    let paths =
        LabelCorrectingMultiObjectiveShortestPath::new(&graph, |edge| weights[edge].clone())
            .paths_from(&1)
            .unwrap();

    assert_eq!(paths.paths(&2).len(), 1);
}

#[test]
fn label_correcting_test_approximation() {
    let (graph, weights) = three_vertex_graph([10.0, 10.0], [10.5, 9.8], [0.0, 0.0]);
    let exact = LabelCorrectingMultiObjectiveShortestPath::with_epsilon(
        &graph,
        |edge| weights[edge].clone(),
        0.0,
    )
    .unwrap()
    .paths_from(&1)
    .unwrap();
    assert_eq!(exact.paths(&2).len(), 2);

    let approximate = LabelCorrectingMultiObjectiveShortestPath::with_epsilon(
        &graph,
        |edge| weights[edge].clone(),
        0.10,
    )
    .unwrap()
    .paths_from(&1)
    .unwrap();
    assert_eq!(approximate.paths(&2).len(), 1);
}

#[test]
fn label_correcting_test_no_paths() {
    let mut graph = DiGraph::<i32>::pseudo();
    graph.add_vertices([1, 2]);
    let paths1 = LabelCorrectingMultiObjectiveShortestPath::new(&graph, |_| vec![0.0, 0.0])
        .paths_from(&1)
        .unwrap();
    assert_eq!(paths1.paths(&1).len(), 1);
    assert!(paths1.paths(&2).is_empty());

    let paths2 = LabelCorrectingMultiObjectiveShortestPath::new(&graph, |_| vec![0.0, 0.0])
        .paths_from(&2)
        .unwrap();
    assert!(paths2.paths(&1).is_empty());
    assert_eq!(paths2.paths(&2).len(), 1);
}

#[test]
fn label_correcting_test_compare_with_martin_shortest_path() {
    let (graph, weights) = five_vertex_graph();
    let martin = MartinShortestPath::new(&graph, |edge| weights[edge].clone())
        .paths_from(&1)
        .unwrap();
    let label_correcting =
        LabelCorrectingMultiObjectiveShortestPath::new(&graph, |edge| weights[edge].clone())
            .paths_from(&1)
            .unwrap();

    for vertex in 1..=5 {
        assert_eq!(
            martin.paths(&vertex).len(),
            label_correcting.paths(&vertex).len()
        );
    }
}
