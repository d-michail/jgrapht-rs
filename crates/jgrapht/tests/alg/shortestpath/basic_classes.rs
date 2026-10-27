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

use jgrapht::alg::shortestpath::*;
use jgrapht::*;

fn weighted_directed(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for &(source, target, weight) in edges {
        graph.add_vertex(source);
        graph.add_vertex(target);
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

#[test]
fn java_class_names_share_the_shortest_path_contract() {
    let graph = weighted_directed(&[(1, 2, 2.0), (1, 3, 1.0), (3, 2, 1.0), (2, 4, 1.0)]);
    assert_eq!(
        DijkstraShortestPath::new(&graph)
            .path_weight(&1, &4)
            .unwrap(),
        Some(3.0)
    );
    assert_eq!(
        BellmanFordShortestPath::new(&graph)
            .path_weight(&1, &4)
            .unwrap(),
        Some(3.0)
    );
}

#[test]
fn floyd_warshall_hops_and_path_count() {
    let graph = weighted_directed(&[(1, 2, -5.0), (2, 3, -5.0), (3, 4, -100.0)]);
    let paths = FloydWarshallShortestPaths::new(&graph).unwrap();
    assert_eq!(paths.shortest_paths_count(), 6);
    assert_eq!(paths.first_hop(&1, &4), Some(2));
    assert_eq!(paths.last_hop(&1, &4), Some(3));
    assert_eq!(paths.path_weight(&1, &4).unwrap(), Some(-110.0));
    assert_eq!(paths.first_hop(&1, &1), None);
}

#[test]
fn johnson_class_handles_negative_edges() {
    let graph = weighted_directed(&[(1, 2, 4.0), (1, 3, 5.0), (3, 2, -3.0)]);
    let paths = JohnsonShortestPaths::new(&graph).unwrap();
    assert_eq!(paths.path_weight(&1, &2).unwrap(), Some(2.0));
    assert_eq!(paths.path(&1, &2).unwrap().unwrap().vertex_list(), [1, 3, 2]);
}

#[test]
fn bidirectional_class_radius_and_astar() {
    let graph = weighted_directed(&[(1, 2, 2.0), (2, 3, 3.0), (1, 3, 9.0)]);
    assert!(
        BidirectionalDijkstraShortestPath::with_radius(&graph, 4.0)
            .unwrap()
            .path(&1, &3)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        BidirectionalAStarShortestPath::new(&graph, |_: &u32, _: &u32| 0.0)
            .path_weight(&1, &3)
            .unwrap(),
        Some(5.0)
    );
}
