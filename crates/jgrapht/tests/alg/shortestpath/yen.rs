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

use jgrapht::alg::shortestpath::{YenKShortestPath, YenShortestPathIterator};
use jgrapht::graph::IncidenceGraph;
use jgrapht::util::java::RandomSource;
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, GraphWalk, UGraph};
use std::collections::HashSet;

const SIMPLE_GRAPH_1: &[(u32, u32, f64)] = &[
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
];

const SIMPLE_GRAPH_2: &[(u32, u32, f64)] = &[
    (1, 2, 5.0),
    (1, 3, 6.0),
    (2, 3, 7.0),
    (2, 4, 8.0),
    (3, 4, 9.0),
];

const SIMPLE_GRAPH_3: &[(u32, u32, f64)] = &[
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
];

const CYCLIC_GRAPH_2: &[(u32, u32, f64)] = &[
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
];

const CYCLIC_GRAPH_3: &[(u32, u32, f64)] = &[
    (1, 2, 1.0),
    (2, 3, 1.0),
    (3, 4, 1.0),
    (4, 3, 1.0),
    (4, 5, 1.0),
    (5, 4, 1.0),
];

const PSEUDOGRAPH_1: &[(u32, u32, f64)] = &[
    (1, 2, 3.0),
    (1, 4, 2.0),
    (1, 5, 4.0),
    (2, 3, 4.0),
    (2, 2, 0.0),
    (3, 5, 3.0),
    (4, 1, 0.0),
    (4, 3, 2.0),
    (4, 4, 0.0),
    (4, 6, 0.0),
    (5, 3, 2.0),
    (5, 6, 2.0),
];

const PSEUDOGRAPH_2: &[(u32, u32, f64)] = &[
    (1, 1, 0.0),
    (1, 1, 1.0),
    (1, 2, 2.0),
    (1, 2, 3.0),
    (1, 2, 4.0),
    (2, 2, 5.0),
    (2, 3, 6.0),
    (2, 3, 7.0),
    (3, 3, 8.0),
    (3, 4, 9.0),
    (3, 4, 10.0),
    (4, 4, 11.0),
];

const PSEUDOGRAPH_3: &[(u32, u32, f64)] = &[
    (1, 2, 1.0),
    (1, 2, 2.0),
    (1, 2, 3.0),
    (2, 3, 4.0),
    (2, 3, 5.0),
];

fn directed(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    add_edges(&mut graph, edges);
    graph
}

fn directed_pseudo(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::pseudo().weighted();
    add_edges(&mut graph, edges);
    graph
}

fn undirected_pseudo(edges: &[(u32, u32, f64)]) -> UGraph<u32> {
    let mut graph = UGraph::<u32>::pseudo().weighted();
    add_edges(&mut graph, edges);
    graph
}

fn add_edges<G>(graph: &mut G, edges: &[(u32, u32, f64)])
where
    G: GraphMut<Vertex = u32, Edge = DefaultEdge>,
{
    for &(source, target, weight) in edges {
        graph.add_vertex(source);
        graph.add_vertex(target);
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
}

fn collect_loopless<G: Graph<Vertex = u32, Edge = DefaultEdge> + IncidenceGraph>(
    graph: &G,
    source: u32,
    target: u32,
) -> Vec<GraphWalk<u32, DefaultEdge>> {
    YenShortestPathIterator::new(graph, &source, &target)
        .unwrap()
        .map(|path| path.unwrap())
        .inspect(|path| {
            assert!(path.is_valid(graph));
            assert_eq!(
                path.vertex_list().iter().collect::<HashSet<_>>().len(),
                path.vertex_list().len()
            );
        })
        .collect()
}

fn weights(paths: &[GraphWalk<u32, DefaultEdge>]) -> Vec<f64> {
    paths.iter().map(|path| path.weight()).collect()
}

mod yen_shortest_path_iterator_test {
    use super::*;

    #[test]
    fn test_no_source_graph() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertex(2);
        assert!(YenShortestPathIterator::new(&graph, &1, &2).is_err());
    }

    #[test]
    fn test_no_sink_graph() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertex(1);
        assert!(YenShortestPathIterator::new(&graph, &1, &2).is_err());
    }

    #[test]
    fn test_no_path_in_graph() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertices([1, 2]);
        assert!(collect_loopless(&graph, 1, 2).is_empty());
    }

    #[test]
    fn test_no_path_left() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertices([1, 2]);
        let mut iterator = YenShortestPathIterator::new(&graph, &1, &2).unwrap();
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_source_equals_target() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertex(1);
        let paths = collect_loopless(&graph, 1, 1);
        assert_eq!(weights(&paths), [0.0]);
        assert_eq!(paths[0].vertex_list(), [1]);
    }

    #[test]
    fn test_only_shortest_path_graph() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertices([1, 2, 3]);
        let first = graph.add_weighted_edge(&1, &2, 1.0).unwrap().unwrap();
        let second = graph.add_weighted_edge(&2, &3, 1.0).unwrap().unwrap();
        let paths = collect_loopless(&graph, 1, 3);
        assert_eq!(weights(&paths), [2.0]);
        assert_eq!(paths[0].edge_list(), [first, second]);
    }

    #[test]
    fn test_simple_graph_1() {
        let graph = directed(SIMPLE_GRAPH_1);
        assert_eq!(
            weights(&collect_loopless(&graph, 1, 12)),
            [55.0, 58.0, 59.0, 61.0, 62.0, 64.0, 65.0, 68.0, 68.0, 71.0]
        );
    }

    #[test]
    fn test_simple_graph_2() {
        let graph = directed(SIMPLE_GRAPH_2);
        assert_eq!(weights(&collect_loopless(&graph, 1, 4)), [13.0, 15.0, 21.0]);
    }

    #[test]
    fn test_simple_graph_3() {
        let graph = directed(SIMPLE_GRAPH_3);
        assert_eq!(weights(&collect_loopless(&graph, 1, 4)), [9.0, 13.0, 15.0]);
    }

    #[test]
    fn test_simple_graph_4() {
        let graph = directed(&[
            (1, 2, 5.0),
            (2, 3, 8.0),
            (2, 4, 7.0),
            (1, 4, 6.0),
            (4, 3, 9.0),
        ]);
        assert_eq!(weights(&collect_loopless(&graph, 1, 3)), [13.0, 15.0, 21.0]);
    }

    #[test]
    fn test_cyclic_graph_1() {
        let graph = directed(&[(1, 2, 1.0), (2, 1, 1.0)]);
        assert_eq!(weights(&collect_loopless(&graph, 1, 2)), [1.0]);
    }

    #[test]
    fn test_cyclic_graph_2() {
        let graph = directed(CYCLIC_GRAPH_2);
        assert_eq!(weights(&collect_loopless(&graph, 1, 6)), [4.0, 4.0]);
    }

    #[test]
    fn test_cyclic_graph_3() {
        let graph = directed(CYCLIC_GRAPH_3);
        assert_eq!(weights(&collect_loopless(&graph, 1, 3)), [2.0]);
    }

    #[test]
    fn test_pseudograph_1() {
        let graph = undirected_pseudo(PSEUDOGRAPH_1);
        assert_eq!(
            weights(&collect_loopless(&graph, 1, 5)),
            [2.0, 4.0, 4.0, 4.0, 5.0, 6.0, 7.0, 9.0, 10.0, 11.0]
        );
    }

    #[test]
    fn test_pseudograph_2() {
        let graph = undirected_pseudo(PSEUDOGRAPH_2);
        assert_eq!(weights(&collect_loopless(&graph, 2, 3)), [6.0, 7.0]);
        assert_eq!(
            weights(&collect_loopless(&graph, 1, 3)),
            [8.0, 9.0, 9.0, 10.0, 10.0, 11.0]
        );
        assert_eq!(
            weights(&collect_loopless(&graph, 1, 4)),
            [
                17.0, 18.0, 18.0, 18.0, 19.0, 19.0, 19.0, 19.0, 20.0, 20.0, 20.0, 21.0,
            ]
        );
    }

    #[test]
    fn test_pseudograph_3() {
        let mut graph = UGraph::<&str>::multi();
        graph.add_vertices([
            "19", "1e", "1c", "1b", "1d", "1f", "16", "17", "12", "14", "18", "15", "21",
        ]);
        let e1 = graph.add_edge(&"19", &"1e").unwrap().unwrap();
        graph.add_edge(&"19", &"1c").unwrap();
        graph.add_edge(&"19", &"1b").unwrap();
        graph.add_edge(&"19", &"1d").unwrap();
        let e5 = graph.add_edge(&"19", &"1f").unwrap().unwrap();
        let e6 = graph.add_edge(&"19", &"16").unwrap().unwrap();
        graph.add_edge(&"12", &"17").unwrap();
        graph.add_edge(&"12", &"14").unwrap();
        graph.add_edge(&"12", &"15").unwrap();
        let e10 = graph.add_edge(&"12", &"16").unwrap().unwrap();
        let e11 = graph.add_edge(&"12", &"16").unwrap().unwrap();
        let e12 = graph.add_edge(&"12", &"18").unwrap().unwrap();
        let e13 = graph.add_edge(&"12", &"21").unwrap().unwrap();
        let e14 = graph.add_edge(&"21", &"1f").unwrap().unwrap();

        let paths = YenKShortestPath::new(&graph)
            .get_paths(&"1e", &"18", 7)
            .unwrap();
        assert_eq!(paths.len(), 3);
        let expected_1 = vec![e1, e6, e10, e12];
        let expected_2 = vec![e1, e6, e11, e12];
        assert!(
            (paths[0].edge_list() == expected_1 && paths[1].edge_list() == expected_2)
                || (paths[0].edge_list() == expected_2 && paths[1].edge_list() == expected_1)
        );
        assert_eq!(paths[2].edge_list(), [e1, e5, e14, e13, e12]);
    }

    #[test]
    fn test_not_shortest_path_edges_graph() {
        let graph = directed(&[
            (1, 2, 1.0),
            (1, 3, 3.0),
            (1, 4, 4.0),
            (1, 5, 5.0),
            (1, 6, 6.0),
            (1, 7, 7.0),
            (1, 8, 8.0),
            (1, 9, 9.0),
        ]);
        assert_eq!(weights(&collect_loopless(&graph, 1, 2)), [1.0]);
    }

    #[test]
    fn test_forbid_all() {
        let graph = undirected_pseudo(PSEUDOGRAPH_1);
        let validator = |_: &GraphWalk<u32, DefaultEdge>, _: &DefaultEdge| false;
        assert!(
            YenShortestPathIterator::with_validator(&graph, &1, &5, validator)
                .unwrap()
                .next()
                .is_none()
        );
    }

    #[test]
    fn test_non_trivial_path_validator() {
        let graph = directed_pseudo(PSEUDOGRAPH_3);
        let validator = |_: &GraphWalk<u32, DefaultEdge>, edge: &DefaultEdge| {
            !(graph.edge_source(edge) == Some(&1)
                && graph.edge_target(edge) == Some(&2)
                && graph.edge_weight(edge) == Some(2.0)
                || graph.edge_source(edge) == Some(&2)
                    && graph.edge_target(edge) == Some(&3)
                    && graph.edge_weight(edge) == Some(4.0))
        };
        let paths: Vec<_> = YenShortestPathIterator::with_validator(&graph, &1, &3, validator)
            .unwrap()
            .collect::<jgrapht::Result<_>>()
            .unwrap();
        assert_eq!(weights(&paths), [6.0, 8.0]);
    }
}

mod yen_error_reporting {
    use super::*;
    use jgrapht::{GraphError, WeightProblem};

    #[test]
    fn a_failure_is_an_item_not_the_end_of_the_iteration() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertices([1, 2, 3]);
        for (a, b, w) in [(1, 2, 1.0), (2, 3, -1.0)] {
            graph.add_edge(&a, &b).unwrap();
            let e = *graph.get_edge(&a, &b).unwrap();
            graph.set_edge_weight(&e, w).unwrap();
        }
        let mut iterator = YenShortestPathIterator::new(&graph, &1, &3).unwrap();
        assert!(matches!(
            iterator.next(),
            Some(Err(GraphError::InvalidWeight(WeightProblem::NegativeOrNaN)))
        ));
        assert!(iterator.next().is_none());

        let collected: jgrapht::Result<Vec<_>> = YenShortestPathIterator::new(&graph, &1, &3)
            .unwrap()
            .collect();
        assert!(collected.is_err());
        assert!(YenKShortestPath::new(&graph).get_paths(&1, &3, 2).is_err());
    }
}

mod yen_k_shortest_path_test {
    use super::*;
    use jgrapht::GraphExt;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_k_equals_zero() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertices([1, 2]);
        assert!(
            YenKShortestPath::new(&graph)
                .get_paths(&1, &2, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_no_source_graph() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertex(2);
        assert!(YenKShortestPath::new(&graph).get_paths(&1, &2, 1).is_err());
    }

    #[test]
    fn test_no_sink_graph() {
        let mut graph = DiGraph::<u32>::simple().weighted();
        graph.add_vertex(1);
        assert!(YenKShortestPath::new(&graph).get_paths(&1, &2, 1).is_err());
    }

    #[test]
    fn test_cyclic_graph() {
        let graph = directed(CYCLIC_GRAPH_3);
        let paths = YenKShortestPath::new(&graph).get_paths(&1, &3, 1).unwrap();
        assert_eq!(weights(&paths), [2.0]);
    }

    #[test]
    fn test_less_than_k_paths() {
        let graph = directed(SIMPLE_GRAPH_1);
        let paths = YenKShortestPath::new(&graph)
            .get_paths(&1, &12, 12)
            .unwrap();
        assert_eq!(
            weights(&paths),
            [55.0, 58.0, 59.0, 61.0, 62.0, 64.0, 65.0, 68.0, 68.0, 71.0]
        );
    }

    #[test]
    fn test_on_random_graphs() {
        let mut random = StdRng::seed_from_u64(13);
        for _ in 0..50 {
            let mut graph = DiGraph::<u32>::pseudo().weighted();
            graph.add_vertices(0..25);
            for source in 0..25 {
                for target in source + 1..25 {
                    if random.next_double() < 0.1 {
                        graph.add_edge(&source, &target).unwrap();
                    }
                    if random.next_double() < 0.1 {
                        graph.add_edge(&target, &source).unwrap();
                    }
                }
            }
            for edge in graph.edges_cloned().collect::<Vec<_>>() {
                graph.set_edge_weight(&edge, random.next_double()).unwrap();
            }
            let source = (random.next_double() * 25.0) as u32;
            let target = (random.next_double() * 25.0) as u32;

            let edges = graph.edges_cloned().collect::<Vec<_>>();
            let mut selection_random = StdRng::seed_from_u64(13);
            let mut forbidden = HashSet::new();
            while forbidden.len() < 5 {
                forbidden.insert(edges[selection_random.next_int_bound(edges.len())]);
            }
            let all = YenKShortestPath::new(&graph)
                .get_paths(&source, &target, usize::MAX)
                .unwrap();
            let validator =
                |_: &GraphWalk<u32, DefaultEdge>, edge: &DefaultEdge| !forbidden.contains(edge);
            let validated = YenKShortestPath::with_validator(&graph, validator)
                .get_paths(&source, &target, usize::MAX)
                .unwrap();
            let expected: Vec<_> = all
                .into_iter()
                .filter(|path| {
                    path.edge_list()
                        .iter()
                        .all(|edge| !forbidden.contains(edge))
                })
                .collect();
            assert_eq!(validated, expected);
            assert_eq!(
                validated
                    .iter()
                    .map(|path| path.edge_list().to_vec())
                    .collect::<HashSet<_>>()
                    .len(),
                validated.len()
            );
        }
    }
}
