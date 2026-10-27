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

use jgrapht::graph::OutgoingGraph;
use jgrapht::alg::shortestpath::{IntVertexDijkstraShortestPath, ShortestPathAlgorithm};
use jgrapht::{DefaultEdge, DiGraph, GraphMutExt, UGraph};

fn assert_weights<G>(graph: &G, expected: &[(i32, &[f64])])
where
    G: jgrapht::Graph<Vertex = i32, Edge = DefaultEdge> + OutgoingGraph,
{
    let algorithm = IntVertexDijkstraShortestPath::new(graph);
    for &(source, weights) in expected {
        let paths = algorithm.single_source(&source).unwrap();
        for (target, &weight) in graph.vertices().zip(weights) {
            let actual = paths.weight(target);
            assert!(
                (actual == weight) || (actual.is_infinite() && weight.is_infinite()),
                "distance {source} -> {target}: expected {weight}, got {actual}"
            );
        }
    }
}

fn undirected(first: i32) -> UGraph<i32> {
    let mut graph = UGraph::<i32>::pseudo().weighted();
    graph.add_vertices([first, 1, 2, 3, 4]);
    for (source, target, weight) in [
        (first, 1, 2.0),
        (first, 2, 3.0),
        (first, 4, 100.0),
        (1, 3, 5.0),
        (2, 3, 20.0),
        (3, 4, 5.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn check_undirected(first: i32) {
    let graph = undirected(first);
    assert_weights(
        &graph,
        &[
            (first, &[0.0, 2.0, 3.0, 7.0, 12.0]),
            (1, &[2.0, 0.0, 5.0, 5.0, 10.0]),
            (2, &[3.0, 5.0, 0.0, 10.0, 15.0]),
            (3, &[7.0, 5.0, 10.0, 0.0, 5.0]),
            (4, &[12.0, 10.0, 15.0, 5.0, 0.0]),
        ],
    );
}

#[test]
fn test_undirected() {
    check_undirected(0);
}

#[test]
fn test_undirected_with_id_map() {
    check_undirected(100);
}

fn check_directed(offset: i32) {
    let vertices = (0..=5).map(|vertex| offset + vertex).collect::<Vec<_>>();
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(vertices.iter().copied());
    let mut edges = Vec::new();
    for (source, target, weight) in [
        (0, 1, 10.0),
        (0, 2, 20.0),
        (2, 4, 33.0),
        (2, 3, 20.0),
        (1, 4, 10.0),
        (1, 3, 50.0),
        (3, 4, 20.0),
        (4, 5, 1.0),
        (3, 5, 2.0),
    ] {
        edges.push(
            graph
                .add_weighted_edge(&(offset + source), &(offset + target), weight)
                .unwrap()
                .unwrap(),
        );
    }

    let inf = f64::INFINITY;
    assert_weights(
        &graph,
        &[
            (offset, &[0.0, 10.0, 20.0, 40.0, 20.0, 21.0]),
            (offset + 1, &[inf, 0.0, inf, 50.0, 10.0, 11.0]),
            (offset + 2, &[inf, inf, 0.0, 20.0, 33.0, 22.0]),
            (offset + 3, &[inf, inf, inf, 0.0, 20.0, 2.0]),
            (offset + 4, &[inf, inf, inf, inf, 0.0, 1.0]),
            (offset + 5, &[inf, inf, inf, inf, inf, 0.0]),
        ],
    );

    let paths = IntVertexDijkstraShortestPath::new(&graph)
        .single_source(&offset)
        .unwrap();
    assert!(paths.path(&offset).unwrap().edge_list().is_empty());
    assert_eq!(paths.path(&(offset + 1)).unwrap().edge_list(), [edges[0]]);
    assert_eq!(paths.path(&(offset + 2)).unwrap().edge_list(), [edges[1]]);
    assert_eq!(
        paths.path(&(offset + 3)).unwrap().edge_list(),
        [edges[1], edges[3]]
    );
    assert_eq!(
        paths.path(&(offset + 4)).unwrap().edge_list(),
        [edges[0], edges[4]]
    );
    assert_eq!(
        paths.path(&(offset + 5)).unwrap().edge_list(),
        [edges[0], edges[4], edges[7]]
    );
}

#[test]
fn test_directed() {
    check_directed(0);
}

#[test]
fn test_directed_with_id_map() {
    check_directed(10_000);
}

#[test]
fn test_non_negative_weights() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices([1, 2]);
    graph.add_weighted_edge(&1, &2, -100.0).unwrap();

    assert!(
        IntVertexDijkstraShortestPath::new(&graph)
            .path(&1, &2)
            .is_err()
    );
}
