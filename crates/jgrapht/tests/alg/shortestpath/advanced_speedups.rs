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
    ContractionHierarchyBidirectionalDijkstra, ContractionHierarchyManyToMany,
    ContractionHierarchyPrecomputation, Dijkstra, ShortestPathAlgorithm,
    TransitNodeRoutingPrecomputation, TransitNodeRoutingShortestPath,
};
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::{DiGraph, Graph, GraphMut, GraphMutExt, GraphWalk, UGraph};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn directed_sample() -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for &(source, target, weight) in &[
        (1, 2, 1.0),
        (2, 3, 1.0),
        (1, 4, 1.0),
        (4, 5, 1.0),
        (5, 6, 1.0),
        (6, 3, 1.0),
        (6, 9, 1.0),
        (7, 4, 1.0),
        (8, 5, 1.0),
    ] {
        graph.add_vertex(source);
        graph.add_vertex(target);
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

#[test]
fn contraction_hierarchy_preserves_paths() {
    let graph = directed_sample();
    let hierarchy = ContractionHierarchyPrecomputation::new(&graph)
        .contraction_hierarchy()
        .unwrap();
    assert_eq!(hierarchy.vertices().len(), graph.vertex_count());
    let query = ContractionHierarchyBidirectionalDijkstra::new(&hierarchy);
    let path = query.path(&1, &3).unwrap().unwrap();
    assert_eq!(path.vertex_list(), vec![1, 2, 3]);
    assert_eq!(path.weight(), 2.0);
    assert!(path.is_valid(&graph));
    assert_eq!(query.path(&3, &1).unwrap(), None);
    assert_eq!(query.path(&7, &9).unwrap().unwrap().weight(), 4.0);
}

#[test]
fn contraction_hierarchy_turns_undirected_edges_into_two_arcs() {
    let mut graph = UGraph::<u32>::simple().weighted();
    for vertex in 1..=3 {
        graph.add_vertex(vertex);
    }
    graph.add_weighted_edge(&1, &2, 1.0).unwrap();
    graph.add_weighted_edge(&2, &3, 1.0).unwrap();
    let hierarchy = ContractionHierarchyPrecomputation::new(&graph)
        .contraction_hierarchy()
        .unwrap();
    assert!(
        hierarchy
            .edges()
            .iter()
            .any(|edge| edge.source == hierarchy.contraction_mapping()[&1]
                && edge.target == hierarchy.contraction_mapping()[&2])
    );
    assert!(
        hierarchy
            .edges()
            .iter()
            .any(|edge| edge.source == hierarchy.contraction_mapping()[&2]
                && edge.target == hierarchy.contraction_mapping()[&1])
    );
    let query = ContractionHierarchyBidirectionalDijkstra::new(&hierarchy);
    assert_eq!(
        query.path(&3, &1).unwrap().unwrap().vertex_list(),
        [3, 2, 1]
    );
}

#[test]
fn ch_many_to_many_materializes_requested_product() {
    let graph = directed_sample();
    let hierarchy = ContractionHierarchyPrecomputation::new(&graph)
        .contraction_hierarchy()
        .unwrap();
    let paths = ContractionHierarchyManyToMany::new(&hierarchy)
        .many_to_many([1, 7, 9], [3, 5])
        .unwrap();
    assert_eq!(paths.weight(&1, &3), Some(2.0));
    assert_eq!(paths.path(&7, &5).unwrap().vertex_list(), [7, 4, 5]);
    assert_eq!(paths.weight(&9, &3), None);
}

#[test]
fn contraction_hierarchy_matches_dijkstra_on_random_graphs() {
    let mut seeds = StdRng::seed_from_u64(0x4348);
    for _ in 0..8 {
        let seed = seeds.random();
        let mut weights = StdRng::seed_from_u64(seed ^ 0x55aa);
        let mut graph = DiGraph::<usize>::simple().weighted();
        GnmRandomGraphGenerator::new(18, 65, StdRng::seed_from_u64(seed), false, false)
            .unwrap()
            .generate_indexed(&mut graph)
            .unwrap();
        let edges: Vec<_> = graph.edges_cloned().collect();
        for edge in edges {
            graph
                .set_edge_weight(&edge, weights.random_range(1..=30) as f64)
                .unwrap();
        }
        let hierarchy = ContractionHierarchyPrecomputation::new(&graph)
            .contraction_hierarchy()
            .unwrap();
        let query = ContractionHierarchyBidirectionalDijkstra::new(&hierarchy);
        for source in 0..18 {
            let expected = Dijkstra::new(&graph).single_source(&source).unwrap();
            for target in 0..18 {
                let actual = query.path(&source, &target).unwrap();
                assert_eq!(
                    actual.as_ref().map_or(f64::INFINITY, GraphWalk::weight),
                    expected.weight(&target).unwrap_or(f64::INFINITY),
                    "seed={seed}, {source}->{target}"
                );
                if let Some(path) = actual {
                    assert!(path.is_valid(&graph));
                }
            }
        }
    }
}

#[test]
fn transit_node_routing_uses_highest_level_and_matches_hierarchy() {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for vertex in 1..=3 {
        graph.add_vertex(vertex);
    }
    let e12 = graph.add_weighted_edge(&1, &2, 1.0).unwrap().unwrap();
    graph.add_weighted_edge(&2, &1, 1.0).unwrap();
    graph.add_weighted_edge(&2, &3, 2.0).unwrap();
    graph.add_weighted_edge(&3, &2, 2.0).unwrap();

    let hierarchy = ContractionHierarchyPrecomputation::new(&graph)
        .contraction_hierarchy()
        .unwrap();
    let routing = TransitNodeRoutingPrecomputation::new(&hierarchy)
        .with_transit_vertex_count(1)
        .unwrap()
        .transit_node_routing()
        .unwrap();
    assert_eq!(routing.transit_vertices().len(), 1);
    assert!(routing.transit_vertices().contains(&2));
    let cv1 = hierarchy.contraction_mapping()[&1];
    assert_eq!(routing.access_vertices().forward(cv1).len(), 1);
    assert_eq!(
        routing.access_vertices().forward(cv1)[0].path.edge_list(),
        [e12]
    );
    assert!(routing.locality_filter().is_local(&1, &3));
    assert!(!routing.locality_filter().is_local(&1, &2));

    let query = TransitNodeRoutingShortestPath::from_routing(routing);
    for source in 1..=3 {
        for target in 1..=3 {
            let actual = query.path(&source, &target).unwrap();
            let expected = ContractionHierarchyBidirectionalDijkstra::new(&hierarchy)
                .path(&source, &target)
                .unwrap();
            assert_eq!(
                actual.as_ref().map(GraphWalk::weight),
                expected.as_ref().map(GraphWalk::weight)
            );
            if let Some(path) = actual {
                assert!(path.is_valid(&graph));
            }
        }
    }
}
