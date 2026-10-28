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

use jgrapht::graph::IncidenceGraph;
use jgrapht::graph::{Directed, Undirected};
use jgrapht::{DefaultEdge, Graph, GraphMut, GraphMutExt, GraphType, IntMapGraph, MapGraph};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

static VERTEX_COMPARISONS: AtomicUsize = AtomicUsize::new(0);

// `PartialEq` only adds a comparison counter; equality is still `self.0 == other.0`, so it agrees with `Hash`.
#[derive(Clone, Copy, Debug, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
struct CountedVertex(usize);

impl PartialEq for CountedVertex {
    fn eq(&self, other: &Self) -> bool {
        VERTEX_COMPARISONS.fetch_add(1, Ordering::Relaxed);
        self.0 == other.0
    }
}

impl Eq for CountedVertex {}

fn check_directed<G>(mut graph: G)
where
    G: Graph<Vertex = usize, Edge = DefaultEdge> + GraphMut + IncidenceGraph,
{
    graph.add_vertices(1..=5);
    let mut add = |source, target| graph.add_edge(&source, &target).unwrap().unwrap();
    let e12 = add(1, 2);
    let e23a = add(2, 3);
    let e23b = add(2, 3);
    let e24 = add(2, 4);
    let e44 = add(4, 4);
    let e55a = add(5, 5);
    let e52 = add(5, 2);
    let e55b = add(5, 5);

    assert_eq!(
        [1, 2, 3, 4, 5].map(|v| graph.degree_of(&v)),
        [1, 5, 2, 3, 5]
    );
    assert_eq!(
        graph.edges_of(&2).copied().collect::<HashSet<_>>(),
        HashSet::from([e12, e23a, e23b, e24, e52])
    );
    assert_eq!(
        [1, 2, 3, 4, 5].map(|v| graph.in_degree_of(&v)),
        [0, 2, 2, 2, 2]
    );
    assert_eq!(
        [1, 2, 3, 4, 5].map(|v| graph.out_degree_of(&v)),
        [1, 3, 0, 1, 3]
    );
    assert_eq!(
        graph.in_edges_of(&5).copied().collect::<HashSet<_>>(),
        HashSet::from([e55a, e55b])
    );
    assert_eq!(
        graph.out_edges_of(&5).copied().collect::<HashSet<_>>(),
        HashSet::from([e52, e55a, e55b])
    );
    assert_eq!(
        graph.edges_between(&2, &3).copied().collect::<HashSet<_>>(),
        HashSet::from([e23a, e23b])
    );
    assert_eq!(graph.edges_between(&3, &2).count(), 0);
    assert_eq!(graph.edge_source(&e44), Some(&4));
}

fn check_undirected<G>(mut graph: G)
where
    G: Graph<Vertex = usize, Edge = DefaultEdge> + GraphMut + IncidenceGraph,
{
    graph.add_vertices(1..=5);
    let mut add = |source, target| graph.add_edge(&source, &target).unwrap().unwrap();
    let e12 = add(1, 2);
    let e23a = add(2, 3);
    let e23b = add(2, 3);
    let e24 = add(2, 4);
    let e44 = add(4, 4);
    let e55a = add(5, 5);
    let e52 = add(5, 2);
    let e55b = add(5, 5);

    let expected = [
        HashSet::from([e12]),
        HashSet::from([e12, e23a, e23b, e24, e52]),
        HashSet::from([e23a, e23b]),
        HashSet::from([e24, e44]),
        HashSet::from([e52, e55a, e55b]),
    ];
    assert_eq!(
        [1, 2, 3, 4, 5].map(|v| graph.degree_of(&v)),
        [1, 5, 2, 3, 5]
    );
    for (index, wanted) in expected.iter().enumerate() {
        let vertex = index + 1;
        assert_eq!(
            graph.edges_of(&vertex).copied().collect::<HashSet<_>>(),
            *wanted
        );
        assert_eq!(
            graph.in_edges_of(&vertex).copied().collect::<HashSet<_>>(),
            *wanted
        );
        assert_eq!(
            graph.out_edges_of(&vertex).copied().collect::<HashSet<_>>(),
            *wanted
        );
    }
    assert_eq!(
        graph.edges_between(&2, &3).copied().collect::<HashSet<_>>(),
        HashSet::from([e23a, e23b])
    );
    assert_eq!(
        graph.edges_between(&3, &2).copied().collect::<HashSet<_>>(),
        HashSet::from([e23a, e23b])
    );
}

#[test]
fn fastutil_map_graph_directed() {
    check_directed(
        MapGraph::<usize, DefaultEdge, Directed>::with_graph_type_and_lookup(
            GraphType::pseudograph(),
            true,
        ),
    );
}

#[test]
fn fastutil_map_graph_undirected() {
    check_undirected(MapGraph::<usize>::with_graph_type_and_lookup(
        GraphType::pseudograph(),
        true,
    ));
}

#[test]
fn fastutil_map_int_vertex_graph_directed() {
    check_directed(
        IntMapGraph::<DefaultEdge, Directed>::with_graph_type_and_lookup(
            GraphType::pseudograph(),
            true,
        ),
    );
}

#[test]
fn fastutil_map_int_vertex_graph_undirected() {
    check_undirected(
        IntMapGraph::<DefaultEdge, Undirected>::with_graph_type_and_lookup(
            GraphType::pseudograph(),
            true,
        ),
    );
}

#[test]
fn optional_endpoint_index_tracks_removal() {
    let mut graph =
        IntMapGraph::<DefaultEdge>::with_graph_type_and_lookup(GraphType::pseudograph(), false);
    assert!(!graph.has_fast_lookups());
    graph.add_vertices([1, 2]);
    let edge = graph.add_edge(&1, &2).unwrap().unwrap();
    assert_eq!(graph.edge_between(&1, &2), Some(&edge));
    assert!(graph.remove_edge(&edge));
    assert_eq!(graph.edge_between(&1, &2), None);

    let mut indexed =
        IntMapGraph::<DefaultEdge>::with_graph_type_and_lookup(GraphType::pseudograph(), true);
    assert!(indexed.has_fast_lookups());
    indexed.add_vertices([1, 2]);
    let first = indexed.add_edge(&1, &2).unwrap().unwrap();
    let second = indexed.add_edge(&2, &1).unwrap().unwrap();
    assert_eq!(indexed.edges_between(&1, &2).count(), 2);
    assert!(indexed.remove_edge(&first));
    assert_eq!(indexed.edge_between(&1, &2), Some(&second));
    assert!(indexed.remove_vertex(&2));
    assert_eq!(indexed.edges_between(&1, &2).count(), 0);
}

#[test]
fn fast_lookup_avoids_incident_edge_scan() {
    fn build(fast_lookups: bool) -> MapGraph<CountedVertex, DefaultEdge, Directed> {
        let mut graph =
            MapGraph::with_graph_type_and_lookup(GraphType::pseudograph(), fast_lookups);
        graph.add_vertices((0..=1_000).map(CountedVertex));
        for target in 1..=1_000 {
            graph
                .add_edge(&CountedVertex(0), &CountedVertex(target))
                .unwrap();
        }
        graph
    }

    let indexed = build(true);
    VERTEX_COMPARISONS.store(0, Ordering::Relaxed);
    assert_eq!(
        indexed
            .edges_between(&CountedVertex(0), &CountedVertex(2_000))
            .count(),
        0
    );
    let indexed_comparisons = VERTEX_COMPARISONS.load(Ordering::Relaxed);

    let scanning = build(false);
    VERTEX_COMPARISONS.store(0, Ordering::Relaxed);
    assert_eq!(
        scanning
            .edges_between(&CountedVertex(0), &CountedVertex(2_000))
            .count(),
        0
    );
    let scanning_comparisons = VERTEX_COMPARISONS.load(Ordering::Relaxed);

    assert!(indexed_comparisons < 50, "{indexed_comparisons}");
    assert!(scanning_comparisons >= 1_000, "{scanning_comparisons}");
}
