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
use jgrapht::alg::shortestpath::{EppsteinKShortestPath, EppsteinShortestPathIterator};
use jgrapht::generate::{GnpRandomGraphGenerator, GraphGenerator};
use jgrapht::{DiGraph, GraphMut, GraphMutExt, UGraph};
use rand::Rng;
use rand::{SeedableRng, rngs::StdRng};
use std::collections::HashSet;

fn graph(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for &(source, target, weight) in edges {
        graph.add_vertex(source);
        graph.add_vertex(target);
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn weights(
    iterator: impl Iterator<Item = jgrapht::GraphWalk<u32, jgrapht::DefaultEdge>>,
    count: usize,
) -> Vec<f64> {
    iterator.take(count).map(|path| path.weight()).collect()
}

#[test]
fn validates_graph_and_vertices() {
    let directed = graph(&[(1, 2, 1.0)]);
    assert!(EppsteinShortestPathIterator::new(&directed, &0, &2).is_err());
    assert!(EppsteinShortestPathIterator::new(&directed, &1, &3).is_err());

    let mut undirected = UGraph::<u32>::simple().weighted();
    undirected.add_vertex(1);
    undirected.add_vertex(2);
    undirected.add_weighted_edge(&1, &2, 1.0).unwrap();
    assert!(EppsteinShortestPathIterator::new(&undirected, &1, &2).is_err());

    let mut pseudograph = DiGraph::<u32>::pseudo().weighted();
    pseudograph.add_vertex(1);
    assert!(EppsteinShortestPathIterator::new(&pseudograph, &1, &1).is_err());
}

#[test]
fn empty_singleton_and_unique_path_cases() {
    let mut disconnected = DiGraph::<u32>::simple().weighted();
    disconnected.add_vertex(1);
    disconnected.add_vertex(2);
    assert_eq!(
        EppsteinShortestPathIterator::new(&disconnected, &1, &2)
            .unwrap()
            .next(),
        None
    );

    let singleton = EppsteinShortestPathIterator::new(&disconnected, &1, &1)
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(singleton.vertex_list(), [1]);
    assert_eq!(singleton.weight(), 0.0);

    let chain = graph(&[(1, 2, 1.0), (2, 3, 1.0)]);
    let mut iterator = EppsteinShortestPathIterator::new(&chain, &1, &3).unwrap();
    let path = iterator.next().unwrap();
    assert_eq!(path.vertex_list(), [1, 2, 3]);
    assert_eq!(path.weight(), 2.0);
    assert!(path.is_valid(&chain));
    assert_eq!(iterator.next(), None);
}

#[test]
fn java_simple_graph_golden_weights() {
    let g = graph(&[
        (1, 2, 2.0),
        (2, 3, 20.0),
        (3, 4, 14.0),
        (1, 5, 13.0),
        (2, 6, 27.0),
        (3, 7, 14.0),
        (4, 8, 15.0),
        (5, 6, 9.0),
        (6, 7, 10.0),
        (7, 8, 25.0),
        (5, 9, 15.0),
        (6, 10, 20.0),
        (7, 11, 12.0),
        (8, 12, 7.0),
        (9, 10, 18.0),
        (10, 11, 8.0),
        (11, 12, 11.0),
    ]);
    let iterator = EppsteinShortestPathIterator::new(&g, &1, &12).unwrap();
    assert_eq!(
        weights(iterator, 20),
        [55.0, 58.0, 59.0, 61.0, 62.0, 64.0, 65.0, 68.0, 68.0, 71.0]
    );
}

#[test]
fn java_simple_graph_2_and_3_golden_weights() {
    let simple_2 = graph(&[
        (1, 2, 5.0),
        (1, 3, 6.0),
        (2, 3, 7.0),
        (2, 4, 8.0),
        (3, 4, 9.0),
    ]);
    assert_eq!(
        weights(
            EppsteinShortestPathIterator::new(&simple_2, &1, &4).unwrap(),
            10
        ),
        [13.0, 15.0, 21.0]
    );

    let simple_3 = graph(&[
        (0, 1, 6.0),
        (2, 0, 9.0),
        (4, 0, 4.0),
        (0, 5, 6.0),
        (0, 6, 5.0),
        (2, 1, 1.0),
        (1, 4, 9.0),
        (4, 1, 2.0),
        (1, 5, 7.0),
        (1, 6, 5.0),
        (2, 4, 1.0),
        (2, 5, 0.0),
        (3, 4, 4.0),
        (4, 3, 4.0),
        (4, 5, 6.0),
        (5, 4, 8.0),
        (4, 6, 3.0),
        (6, 5, 0.0),
    ]);
    assert_eq!(
        weights(
            EppsteinShortestPathIterator::new(&simple_3, &5, &4).unwrap(),
            10
        ),
        [8.0, 16.0, 19.0, 19.0, 22.0, 23.0, 24.0, 25.0, 25.0, 26.0]
    );
}

#[test]
fn java_cycle_golden_weights_and_valid_walks() {
    let two_cycle = graph(&[(1, 2, 1.0), (2, 1, 1.0)]);
    let mut iterator = EppsteinShortestPathIterator::new(&two_cycle, &1, &2).unwrap();
    for expected in [1.0, 3.0, 5.0, 7.0, 9.0] {
        let path = iterator.next().unwrap();
        assert_eq!(path.weight(), expected);
        assert!(path.is_valid(&two_cycle));
    }

    let graph = graph(&[
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (4, 1, 1.0),
        (1, 5, 2.0),
        (5, 6, 2.0),
        (6, 7, 2.0),
        (7, 1, 2.0),
        (3, 6, 2.0),
        (6, 3, 2.0),
    ]);
    assert_eq!(
        weights(
            EppsteinShortestPathIterator::new(&graph, &1, &6).unwrap(),
            18
        ),
        [
            4.0, 4.0, 8.0, 8.0, 8.0, 8.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0,
            12.0, 12.0, 12.0,
        ]
    );
}

#[test]
fn java_rest_heap_and_k_shortest_golden_weights() {
    let g = graph(&[
        (1, 2, 2.0),
        (1, 3, 3.0),
        (1, 4, 4.0),
        (1, 5, 5.0),
        (1, 6, 6.0),
        (1, 7, 7.0),
        (1, 8, 8.0),
        (1, 9, 9.0),
        (2, 10, 1.0),
        (3, 10, 1.0),
        (4, 10, 1.0),
        (5, 10, 1.0),
        (6, 10, 1.0),
        (7, 10, 1.0),
        (8, 10, 1.0),
        (9, 10, 1.0),
    ]);
    assert_eq!(
        weights(EppsteinShortestPathIterator::new(&g, &1, &10).unwrap(), 20),
        [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    );

    let cyclic = graph(&[
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (4, 3, 1.0),
        (4, 5, 1.0),
        (5, 4, 1.0),
    ]);
    let paths = EppsteinKShortestPath::new(&cyclic)
        .paths(&1, &3, 6)
        .unwrap();
    assert_eq!(
        paths.iter().map(|path| path.weight()).collect::<Vec<_>>(),
        [2.0, 4.0, 6.0, 6.0, 8.0, 8.0]
    );
}

#[test]
fn java_k_shortest_facade_contracts() {
    let mut disconnected = DiGraph::<u32>::simple().weighted();
    disconnected.add_vertices([1, 2]);
    assert!(
        EppsteinKShortestPath::new(&disconnected)
            .paths(&1, &2, 0)
            .unwrap()
            .is_empty()
    );
    assert!(
        EppsteinKShortestPath::new(&disconnected)
            .paths(&0, &2, 1)
            .is_err()
    );
    assert!(
        EppsteinKShortestPath::new(&disconnected)
            .paths(&1, &3, 1)
            .is_err()
    );

    let graph = graph(&[
        (1, 2, 2.0),
        (2, 3, 20.0),
        (3, 4, 14.0),
        (1, 5, 13.0),
        (2, 6, 27.0),
        (3, 7, 14.0),
        (4, 8, 15.0),
        (5, 6, 9.0),
        (6, 7, 10.0),
        (7, 8, 25.0),
        (5, 9, 15.0),
        (6, 10, 20.0),
        (7, 11, 12.0),
        (8, 12, 7.0),
        (9, 10, 18.0),
        (10, 11, 8.0),
        (11, 12, 11.0),
    ]);
    let paths = EppsteinKShortestPath::new(&graph)
        .paths(&1, &12, 12)
        .unwrap();
    assert_eq!(
        paths.iter().map(|path| path.weight()).collect::<Vec<_>>(),
        [55.0, 58.0, 59.0, 61.0, 62.0, 64.0, 65.0, 68.0, 68.0, 71.0]
    );
}

#[test]
fn java_random_graph_workload_returns_distinct_ordered_valid_walks() {
    let mut values = StdRng::seed_from_u64(13);
    for _ in 0..1000 {
        let mut graph = DiGraph::<usize>::simple().weighted();
        GnpRandomGraphGenerator::new(100, 0.5, StdRng::seed_from_u64(13), false)
            .unwrap()
            .generate_indexed(&mut graph)
            .unwrap();
        for edge in graph.edges_cloned().collect::<Vec<_>>() {
            graph
                .set_edge_weight(&edge, values.random_range(0..10) as f64)
                .unwrap();
        }
        let source = values.random_range(0..100);
        let target = values.random_range(0..100);
        let paths: Vec<_> = EppsteinShortestPathIterator::new(&graph, &source, &target)
            .unwrap()
            .take(10)
            .collect();
        assert!(
            paths
                .windows(2)
                .all(|pair| pair[0].weight() <= pair[1].weight())
        );
        assert!(paths.iter().all(|path| path.is_valid(&graph)));
        assert_eq!(
            paths
                .iter()
                .map(|path| path.edge_list().to_vec())
                .collect::<HashSet<_>>()
                .len(),
            paths.len()
        );
    }
}
