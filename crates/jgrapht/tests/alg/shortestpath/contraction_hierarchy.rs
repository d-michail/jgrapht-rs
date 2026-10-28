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
    ContractionHierarchy, ContractionHierarchyBidirectionalDijkstra,
    ContractionHierarchyPrecomputation, Dijkstra, ShortestPathAlgorithm,
};
use jgrapht::generate::{GnmRandomGraphGenerator, GnpRandomGraphGenerator, GraphGenerator};
use jgrapht::graph::OutgoingGraph;
use jgrapht::util::java::RandomSource;
use jgrapht::{DiGraph, Graph, GraphError, GraphMut, GraphMutExt, UGraph};
use rand::{SeedableRng, rngs::StdRng};

const SEED: i64 = 19;

fn precompute<G: Graph>(graph: &G) -> ContractionHierarchy<'_, G> {
    ContractionHierarchyPrecomputation::with_random_supplier(graph, 4, || {
        StdRng::seed_from_u64((SEED) as u64)
    })
    .compute_contraction_hierarchy()
    .unwrap()
}

fn has_edge<G: Graph>(h: &ContractionHierarchy<'_, G>, s: &G::Vertex, t: &G::Vertex) -> bool {
    let m = h.contraction_mapping();
    h.edges()
        .iter()
        .any(|e| e.source == m[s] && e.target == m[t])
}

fn edge_between<'a, G: Graph>(
    h: &'a ContractionHierarchy<'_, G>,
    s: &G::Vertex,
    t: &G::Vertex,
) -> &'a jgrapht::alg::shortestpath::ContractionEdge<G::Edge> {
    let m = h.contraction_mapping();
    h.edges()
        .iter()
        .find(|e| e.source == m[s] && e.target == m[t])
        .unwrap()
}

fn assert_simple<G: Graph>(h: &ContractionHierarchy<'_, G>) {
    let mut pairs = std::collections::HashSet::new();
    for e in h.edges() {
        assert_ne!(e.source, e.target);
        assert!(pairs.insert((e.source, e.target)), "multiple edges");
    }
}

mod precomputation {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_empty_graph() {
        let graph = DiGraph::<i32>::simple().weighted();
        let h = precompute(&graph);
        assert!(h.vertices().is_empty());
        assert!(h.edges().is_empty());
        assert!(h.contraction_mapping().is_empty());
    }

    #[test]
    fn test_directed_graph1() {
        let mut graph = DiGraph::<i32>::pseudo().weighted();
        graph.add_vertices([1, 2, 3]);
        graph.add_weighted_edge(&1, &2, 1.0).unwrap();
        graph.add_weighted_edge(&2, &3, 1.0).unwrap();
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 3);
        assert_eq!(h.edges().len(), 2);
        assert!(has_edge(&h, &1, &2));
        assert!(has_edge(&h, &2, &3));
    }

    #[test]
    fn test_directed_graph2() {
        let mut graph = DiGraph::<i32>::pseudo().weighted();
        graph.add_vertices([1, 2, 3]);
        for (s, t) in [(1, 2), (2, 1), (2, 3), (3, 2), (3, 1), (1, 3)] {
            graph.add_weighted_edge(&s, &t, 1.0).unwrap();
        }
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 3);
        assert_eq!(h.edges().len(), 6);
    }

    #[test]
    fn test_directed_graph3() {
        let mut graph = DiGraph::<i32>::simple().weighted();
        graph.add_vertices([1, 3, 2, 4, 5]);
        for (s, t) in [(1, 3), (2, 3), (3, 4), (3, 5)] {
            graph.add_weighted_edge(&s, &t, 1.0).unwrap();
        }
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 5);
        assert_eq!(h.edges().len(), 4);
        for (s, t) in [(1, 3), (2, 3), (3, 4), (3, 5)] {
            assert!(has_edge(&h, &s, &t));
        }
    }

    #[test]
    fn test_undirected_graph1() {
        let mut graph = UGraph::<i32>::simple().weighted();
        graph.add_vertices([1, 2, 3]);
        graph.add_weighted_edge(&1, &2, 1.0).unwrap();
        graph.add_weighted_edge(&2, &3, 1.0).unwrap();
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 3);
        assert_eq!(h.edges().len(), 4);
        for (s, t) in [(1, 2), (2, 1), (2, 3), (3, 2)] {
            assert!(has_edge(&h, &s, &t));
        }
    }

    #[test]
    fn test_undirected_graph2() {
        let mut graph = UGraph::<i32>::pseudo().weighted();
        graph.add_vertices([1, 2, 3]);
        for (s, t) in [(1, 2), (2, 3), (3, 1)] {
            graph.add_weighted_edge(&s, &t, 1.0).unwrap();
        }
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(graph.vertex_count(), 3);
        assert_eq!(h.vertices().len(), 3);
        assert_eq!(h.edges().len(), 6);
    }

    #[test]
    fn test_undirected_graph3() {
        let mut graph = UGraph::<i32>::pseudo().weighted();
        graph.add_vertices([1, 3, 2, 4, 5]);
        for (s, t) in [(1, 3), (2, 3), (3, 4), (3, 5)] {
            graph.add_weighted_edge(&s, &t, 1.0).unwrap();
        }
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 5);
        assert_eq!(h.edges().len(), 8);
        for (s, t) in [(1, 3), (2, 3), (3, 4), (3, 5)] {
            assert!(has_edge(&h, &s, &t));
            assert!(has_edge(&h, &t, &s));
        }
    }

    #[test]
    fn test_undirected_graph4() {
        let mut graph = UGraph::<i32>::simple().weighted();
        graph.add_vertices(1..=9);
        let edges = [
            (1, 2, 3.0),
            (1, 4, 1.0),
            (2, 3, 3.0),
            (2, 5, 1.0),
            (3, 6, 1.0),
            (4, 5, 1.0),
            (4, 7, 1.0),
            (5, 6, 1.0),
            (5, 8, 1.0),
            (6, 9, 1.0),
            (7, 8, 3.0),
            (8, 9, 3.0),
        ];
        for (s, t, w) in edges {
            graph.add_weighted_edge(&s, &t, w).unwrap();
        }
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 9);
        assert_eq!(h.edges().len(), 24);
        for (s, t, _) in edges {
            assert!(has_edge(&h, &s, &t));
            assert!(has_edge(&h, &t, &s));
        }
    }

    #[test]
    fn test_pseudograph() {
        let mut graph = DiGraph::<i32>::pseudo().weighted();
        graph.add_vertices([1, 2]);
        for (s, t, w) in [
            (1, 2, 1.0),
            (1, 2, 2.0),
            (1, 2, 3.0),
            (2, 1, 1.0),
            (2, 1, 2.0),
            (2, 2, 1.0),
            (2, 2, 2.0),
        ] {
            graph.add_weighted_edge(&s, &t, w).unwrap();
        }
        let h = precompute(&graph);
        assert_simple(&h);
        assert_eq!(h.vertices().len(), 2);
        assert_eq!(h.edges().len(), 2);
        assert_eq!(edge_between(&h, &1, &2).weight, 1.0);
        assert_eq!(edge_between(&h, &2, &1).weight, 1.0);
    }

    #[test]
    fn test_on_random_graphs() {
        for i in 0..20u64 {
            let mut graph = DiGraph::<usize>::pseudo().weighted();
            GnpRandomGraphGenerator::new(30, 0.2, StdRng::seed_from_u64(i), false)
                .unwrap()
                .generate_indexed(&mut graph)
                .unwrap();
            let mut random = StdRng::seed_from_u64((SEED) as u64);
            let edges: Vec<_> = graph.edges().copied().collect();
            for e in &edges {
                graph.set_edge_weight(e, random.next_double()).unwrap();
            }
            let h = ContractionHierarchyPrecomputation::new(&graph)
                .compute_contraction_hierarchy()
                .unwrap();

            // correct mapping
            let mapping = h.contraction_mapping();
            assert_eq!(mapping.len(), graph.vertex_count());
            let unique: std::collections::HashSet<_> = mapping.values().collect();
            assert_eq!(unique.len(), graph.vertex_count());
            assert!(graph.vertices().all(|v| mapping.contains_key(v)));
            assert_eq!(h.vertices().len(), unique.len());
            for (v, &id) in mapping {
                assert_eq!(h.vertices()[id].vertex, *v);
                assert_eq!(h.vertices()[id].vertex_id, id);
            }
            // no edges removed, and weights are not larger than the original ones
            for e in graph.edges() {
                let (s, t) = (graph.edge_source(e).unwrap(), graph.edge_target(e).unwrap());
                assert!(has_edge(&h, s, t));
                assert!(graph.edge_weight(e).unwrap() >= edge_between(&h, s, t).weight);
            }
            // non shortcut edges hold a minimum weight original edge
            for ce in h.edges() {
                let Some(original) = &ce.edge else { continue };
                let weight = graph.edge_weight(original).unwrap();
                assert!((weight - ce.weight).abs() < 1e-9);
                let s = &h.vertices()[ce.source].vertex;
                let t = &h.vertices()[ce.target].vertex;
                let mut contains = false;
                for e in graph.edges_between(s, t) {
                    assert!(graph.edge_weight(e).unwrap() >= weight);
                    contains |= e == original;
                }
                assert!(contains);
            }
        }
    }

    #[test]
    fn test_negative_weight_is_rejected() {
        let mut graph = DiGraph::<i32>::simple().weighted();
        graph.add_vertices([1, 2]);
        graph.add_weighted_edge(&1, &2, -1.0).unwrap();
        let result =
            ContractionHierarchyPrecomputation::new(&graph).compute_contraction_hierarchy();
        assert!(matches!(result, Err(GraphError::InvalidWeight(_))));
    }

    #[test]
    fn test_result_is_independent_of_parallelism() {
        let mut graph = UGraph::<usize>::simple().weighted();
        GnmRandomGraphGenerator::new(60, 200, StdRng::seed_from_u64(5), false, false)
            .unwrap()
            .generate_indexed(&mut graph)
            .unwrap();
        let signature = |p: usize| {
            let h = ContractionHierarchyPrecomputation::with_random_supplier(&graph, p, || {
                StdRng::seed_from_u64((SEED) as u64)
            })
            .compute_contraction_hierarchy()
            .unwrap();
            let levels: Vec<_> = h.vertices().iter().map(|v| v.contraction_level).collect();
            (levels, h.edges().len())
        };
        // the same number of workers always yields the same hierarchy
        assert_eq!(signature(3), signature(3));
        assert_eq!(signature(1), signature(1));
    }
}

mod bidirectional_dijkstra {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn weighted_graph(edges: &[(i32, i32, f64)]) -> UGraph<i32> {
        let mut graph = UGraph::<i32>::simple().weighted();
        for &(s, t, w) in edges {
            graph.add_vertex(s);
            graph.add_vertex(t);
            graph.add_weighted_edge(&s, &t, w).unwrap();
        }
        graph
    }

    #[test]
    fn test_empty_graph() {
        let graph = UGraph::<i32>::simple().weighted();
        let h = precompute(&graph);
        let _ = ContractionHierarchyBidirectionalDijkstra::new(&h);
    }

    #[test]
    fn test_source_not_present() {
        let mut graph = UGraph::<i32>::simple().weighted();
        graph.add_vertex(2);
        let h = precompute(&graph);
        let dijkstra = ContractionHierarchyBidirectionalDijkstra::new(&h);
        assert!(matches!(
            dijkstra.path(&1, &2),
            Err(GraphError::VertexNotFound)
        ));
    }

    #[test]
    fn test_target_not_present() {
        let mut graph = UGraph::<i32>::simple().weighted();
        graph.add_vertex(1);
        let h = precompute(&graph);
        let dijkstra = ContractionHierarchyBidirectionalDijkstra::new(&h);
        assert!(matches!(
            dijkstra.path(&1, &2),
            Err(GraphError::VertexNotFound)
        ));
    }

    #[test]
    fn test_no_path() {
        let mut graph = UGraph::<i32>::simple().weighted();
        graph.add_vertices([1, 2]);
        let h = precompute(&graph);
        let dijkstra = ContractionHierarchyBidirectionalDijkstra::new(&h);
        assert_eq!(dijkstra.path(&1, &2).unwrap(), None);
    }

    #[test]
    fn test_simple_graph() {
        let graph = weighted_graph(&[
            (1, 2, 3.0),
            (1, 4, 1.0),
            (2, 3, 3.0),
            (2, 5, 1.0),
            (3, 6, 1.0),
            (4, 5, 1.0),
            (4, 7, 1.0),
            (5, 6, 1.0),
            (5, 8, 1.0),
            (6, 9, 1.0),
            (7, 8, 3.0),
            (8, 9, 3.0),
        ]);
        let h = precompute(&graph);
        let dijkstra = ContractionHierarchyBidirectionalDijkstra::new(&h);
        let vertices = |s, t| {
            dijkstra
                .path(&s, &t)
                .unwrap()
                .unwrap()
                .vertex_list()
                .to_vec()
        };
        assert_eq!(vertices(1, 1), [1]);
        assert_eq!(vertices(1, 2), [1, 2]);
        assert_eq!(vertices(1, 3), [1, 4, 5, 6, 3]);
        assert_eq!(vertices(1, 9), [1, 4, 5, 6, 9]);
        assert_eq!(vertices(7, 1), [7, 4, 1]);
        assert_eq!(vertices(8, 2), [8, 5, 2]);
    }

    #[test]
    fn test_ring_graph() {
        let size = 100;
        let mut graph = UGraph::<usize>::simple().weighted();
        let mut random = StdRng::seed_from_u64((SEED) as u64);
        graph.add_vertices(0..size);
        for i in 0..size {
            let j = (i + 1) % size;
            graph.add_edge(&i, &j).unwrap();
            let e = *graph.get_edge(&i, &j).unwrap();
            graph.set_edge_weight(&e, random.next_double()).unwrap();
        }
        check_against_dijkstra(&graph, 0);
    }

    #[test]
    fn test_on_random_graphs() {
        let n = 100;
        let mut random = StdRng::seed_from_u64((SEED) as u64);
        for _ in 0..20 {
            let mut graph = DiGraph::<usize>::pseudo().weighted();
            GnmRandomGraphGenerator::new(
                n,
                5 * n - n + 1,
                StdRng::seed_from_u64(SEED as u64),
                true,
                true,
            )
            .unwrap()
            .generate_indexed(&mut graph)
            .unwrap();
            // make the graph connected
            for i in 0..n - 1 {
                graph.add_edge(&i, &(i + 1)).unwrap();
                graph.add_edge(&(i + 1), &i).unwrap();
            }
            let edges: Vec<_> = graph.edges().copied().collect();
            for e in &edges {
                graph.set_edge_weight(e, random.next_double()).unwrap();
            }
            check_against_dijkstra(&graph, 0);
        }
    }

    fn check_against_dijkstra<G: OutgoingGraph>(graph: &G, source: G::Vertex)
    where
        G::Vertex: std::fmt::Debug,
        G::Edge: std::fmt::Debug,
    {
        let expected = Dijkstra::new(graph).single_source(&source).unwrap();
        let h = precompute(graph);
        let actual = ContractionHierarchyBidirectionalDijkstra::new(&h)
            .single_source(&source)
            .unwrap();
        for sink in graph.vertices() {
            assert_eq!(expected.path(sink), actual.path(sink), "sink {sink:?}");
        }
    }
}

/// Hierarchies of the fixture graphs for 1, 3 and 4 workers. The contraction order depends on the
/// random tie-breaking values, so instead of fixed levels the structure of the hierarchy and the
/// queries on it are checked: levels order the edges, and the bidirectional search finds the same
/// paths as Dijkstra from every source.
mod fixture_graphs {
    use super::*;
    use std::collections::HashMap;

    struct Case {
        directed: bool,
        n: usize,
        edges: Vec<(usize, usize, f64)>,
    }

    fn graphs() -> HashMap<String, Case> {
        let text = include_str!("../fixtures/ch_graphs.txt");
        let mut lines = text.lines();
        let mut result = HashMap::new();
        while let Some(header) = lines.next() {
            let h: Vec<_> = header.split(' ').collect();
            let (n, m): (usize, usize) = (h[3].parse().unwrap(), h[4].parse().unwrap());
            let edges = (0..m)
                .map(|_| {
                    let t: Vec<_> = lines.next().unwrap().split(' ').collect();
                    (
                        t[0].parse().unwrap(),
                        t[1].parse().unwrap(),
                        t[2].parse().unwrap(),
                    )
                })
                .collect();
            result.insert(
                h[1].to_string(),
                Case {
                    directed: h[2] == "d",
                    n,
                    edges,
                },
            );
        }
        result
    }

    fn check<G: OutgoingGraph<Vertex = usize>>(graph: &G, p: usize, name: &str)
    where
        G::Edge: std::fmt::Debug,
    {
        let h = ContractionHierarchyPrecomputation::with_random_supplier(graph, p, || {
            StdRng::seed_from_u64(19)
        })
        .compute_contraction_hierarchy()
        .unwrap();
        for e in h.edges() {
            let (ls, lt) = (
                h.vertices()[e.source].contraction_level,
                h.vertices()[e.target].contraction_level,
            );
            assert_ne!(ls, lt, "{name} p={p}");
            assert_eq!(e.is_upward, ls < lt, "{name} p={p}");
        }
        for source in 0..graph.vertex_count() {
            let expected = Dijkstra::new(graph).single_source(&source).unwrap();
            let actual = ContractionHierarchyBidirectionalDijkstra::new(&h)
                .single_source(&source)
                .unwrap();
            for sink in graph.vertices() {
                assert_eq!(
                    expected.path(sink).map(|w| w.weight()),
                    actual.path(sink).map(|w| w.weight()),
                    "{name} p={p} {source}->{sink}"
                );
            }
        }
    }

    #[test]
    fn hierarchy_is_valid_and_answers_queries() {
        let graphs = graphs();
        assert_eq!(graphs.len(), 4);
        for (name, case) in &graphs {
            for p in [1, 3, 4] {
                if case.directed {
                    let mut g = DiGraph::<usize>::pseudo().weighted();
                    g.add_vertices(0..case.n);
                    for &(s, t, w) in &case.edges {
                        g.add_weighted_edge(&s, &t, w).unwrap();
                    }
                    check(&g, p, name);
                } else {
                    let mut g = UGraph::<usize>::pseudo().weighted();
                    g.add_vertices(0..case.n);
                    for &(s, t, w) in &case.edges {
                        g.add_weighted_edge(&s, &t, w).unwrap();
                    }
                    check(&g, p, name);
                }
            }
        }
    }
}
