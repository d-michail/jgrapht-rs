use jgrapht::util::java::RandomSource;
use rand::{SeedableRng, rngs::StdRng};
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
    CHManyToManyShortestPaths, ContractionHierarchyPrecomputation, DefaultManyToManyShortestPaths,
    Dijkstra, DijkstraManyToManyShortestPaths, DijkstraShortestPath, ManyToManyPaths,
    ShortestPathAlgorithm, SingleSourcePaths,
};
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::graph::{IncidenceGraph, OutgoingGraph};
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, Result, UGraph};
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Copy)]
enum Kind {
    Default,
    Dijkstra,
    ContractionHierarchy,
}

fn compute<G>(
    kind: Kind,
    graph: &G,
    sources: impl IntoIterator<Item = usize>,
    targets: impl IntoIterator<Item = usize>,
) -> Result<ManyToManyPaths<usize, DefaultEdge>>
where
    G: Graph<Vertex = usize, Edge = DefaultEdge> + IncidenceGraph,
{
    let sources = sources.into_iter().collect::<Vec<_>>();
    let targets = targets.into_iter().collect::<Vec<_>>();
    match kind {
        Kind::Default => {
            DefaultManyToManyShortestPaths::new(graph).get_many_to_many_paths(sources, targets)
        }
        Kind::Dijkstra => {
            DijkstraManyToManyShortestPaths::new(graph).get_many_to_many_paths(sources, targets)
        }
        Kind::ContractionHierarchy => {
            let hierarchy =
                ContractionHierarchyPrecomputation::new(graph).compute_contraction_hierarchy()?;
            CHManyToManyShortestPaths::new(&hierarchy).many_to_many(sources, targets)
        }
    }
}

fn add_undirected(graph: &mut UGraph<usize>, source: usize, target: usize, weight: f64) {
    graph.add_vertex(source);
    graph.add_vertex(target);
    graph.add_weighted_edge(&source, &target, weight).unwrap();
}

fn add_directed(graph: &mut DiGraph<usize>, source: usize, target: usize, weight: f64) {
    graph.add_vertex(source);
    graph.add_vertex(target);
    graph.add_weighted_edge(&source, &target, weight).unwrap();
}

fn simple_graph() -> UGraph<usize> {
    let mut graph = UGraph::<usize>::simple().weighted();
    for (source, target, weight) in [
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
    ] {
        add_undirected(&mut graph, source, target, weight);
    }
    graph
}

fn multigraph() -> DiGraph<usize> {
    let mut graph = DiGraph::<usize>::multi().weighted();
    for (source, target, weight) in [
        (1, 2, 1.0),
        (1, 2, 2.0),
        (2, 1, 3.0),
        (2, 1, 4.0),
        (2, 3, 8.0),
        (2, 3, 7.0),
        (3, 2, 6.0),
        (3, 2, 5.0),
        (3, 4, 9.0),
        (3, 4, 10.0),
        (4, 3, 11.0),
        (4, 3, 12.0),
        (4, 5, 16.0),
        (4, 5, 15.0),
        (5, 4, 14.0),
        (5, 4, 13.0),
        (5, 6, 17.0),
        (5, 6, 18.0),
        (6, 5, 19.0),
        (6, 5, 20.0),
        (6, 1, 24.0),
        (6, 1, 23.0),
        (1, 6, 22.0),
        (1, 6, 21.0),
    ] {
        add_directed(&mut graph, source, target, weight);
    }
    graph
}

fn assert_path(
    paths: &ManyToManyPaths<usize, DefaultEdge>,
    source: usize,
    target: usize,
    weight: f64,
    vertices: &[usize],
) {
    assert!((paths.weight(&source, &target).unwrap() - weight).abs() < 1e-9);
    assert_eq!(
        paths.path(&source, &target).unwrap().vertex_list(),
        vertices,
        "{source}->{target}"
    );
}

fn empty_graph(kind: Kind) {
    let graph = DiGraph::<usize>::simple().weighted();
    let paths = compute(kind, &graph, [], []).unwrap();
    assert!(paths.sources().is_empty());
    assert!(paths.targets().is_empty());
}

fn no_path(kind: Kind) {
    let mut graph = DiGraph::<usize>::multi().weighted();
    graph.add_vertices([1, 2]);
    let paths = compute(kind, &graph, [1], [2]).unwrap();
    assert_eq!(paths.weight(&1, &2), None);
    assert!(paths.path(&1, &2).is_none());
}

fn different_sources_and_targets_simple_graph(kind: Kind) {
    let graph = simple_graph();
    let paths = compute(kind, &graph, [4, 1, 2], [8, 9, 6]).unwrap();
    for (source, target, weight, vertices) in [
        (4, 8, 2.0, &[4, 5, 8][..]),
        (4, 9, 3.0, &[4, 5, 6, 9]),
        (4, 6, 2.0, &[4, 5, 6]),
        (1, 8, 3.0, &[1, 4, 5, 8]),
        (1, 9, 4.0, &[1, 4, 5, 6, 9]),
        (1, 6, 3.0, &[1, 4, 5, 6]),
        (2, 8, 2.0, &[2, 5, 8]),
        (2, 9, 3.0, &[2, 5, 6, 9]),
        (2, 6, 2.0, &[2, 5, 6]),
    ] {
        assert_path(&paths, source, target, weight, vertices);
    }
}

fn different_sources_and_targets_multigraph(kind: Kind) {
    let graph = multigraph();
    let paths = compute(kind, &graph, [1, 4], [2, 5]).unwrap();
    for (source, target, weight, vertices) in [
        (1, 2, 1.0, &[1, 2][..]),
        (1, 5, 32.0, &[1, 2, 3, 4, 5]),
        (4, 2, 16.0, &[4, 3, 2]),
        (4, 5, 15.0, &[4, 5]),
    ] {
        assert_path(&paths, source, target, weight, vertices);
    }
}

fn sources_equal_targets_simple_graph(kind: Kind) {
    let graph = simple_graph();
    let paths = compute(kind, &graph, [1, 5, 9], [1, 5, 9]).unwrap();
    for (source, target, weight, vertices) in [
        (1, 1, 0.0, &[1][..]),
        (5, 5, 0.0, &[5]),
        (9, 9, 0.0, &[9]),
        (1, 5, 2.0, &[1, 4, 5]),
        (5, 1, 2.0, &[5, 4, 1]),
        (1, 9, 4.0, &[1, 4, 5, 6, 9]),
        (9, 1, 4.0, &[9, 6, 5, 4, 1]),
        (5, 9, 2.0, &[5, 6, 9]),
        (9, 5, 2.0, &[9, 6, 5]),
    ] {
        assert_path(&paths, source, target, weight, vertices);
    }
}

fn sources_equal_targets_multigraph(kind: Kind) {
    let graph = multigraph();
    let paths = compute(kind, &graph, [2, 4, 6], [2, 4, 6]).unwrap();
    for (source, target, weight, vertices) in [
        (2, 2, 0.0, &[2][..]),
        (4, 4, 0.0, &[4]),
        (6, 6, 0.0, &[6]),
        (2, 4, 16.0, &[2, 3, 4]),
        (4, 2, 16.0, &[4, 3, 2]),
        (2, 6, 24.0, &[2, 1, 6]),
        (6, 2, 24.0, &[6, 1, 2]),
        (4, 6, 32.0, &[4, 5, 6]),
        (6, 4, 32.0, &[6, 5, 4]),
    ] {
        assert_path(&paths, source, target, weight, vertices);
    }
}

fn generate_random_graph(n: usize, edges: usize, weights: &mut StdRng) -> DiGraph<usize> {
    let mut graph = DiGraph::<usize>::pseudo().weighted();
    GnmRandomGraphGenerator::new(n, edges - n + 1, StdRng::seed_from_u64(17), false, false)
        .unwrap()
        .generate_indexed(&mut graph)
        .unwrap();
    for vertex in 0..n - 1 {
        graph.add_edge(&vertex, &(vertex + 1)).unwrap();
        graph.add_edge(&(vertex + 1), &vertex).unwrap();
    }
    for edge in graph.edges().copied().collect::<Vec<_>>() {
        graph.set_edge_weight(&edge, weights.next_double()).unwrap();
    }
    graph
}

fn random_vertices(n: usize, count: usize, random: &mut StdRng) -> HashSet<usize> {
    let mut vertices = HashSet::with_capacity(count);
    while vertices.len() < count {
        vertices.insert(random.next_int_bound(n));
    }
    vertices
}

fn assert_correct_paths<G: Graph<Vertex = usize, Edge = DefaultEdge> + OutgoingGraph>(
    graph: &G,
    paths: &ManyToManyPaths<usize, DefaultEdge>,
    sources: &HashSet<usize>,
    targets: &HashSet<usize>,
) {
    let dijkstra = DijkstraShortestPath::new(graph);
    for source in sources {
        let expected = dijkstra.single_source(source).unwrap();
        for target in targets {
            let expected_path = expected.path(target);
            let actual_path = paths.path(source, target).cloned();
            match (expected_path, actual_path) {
                (None, None) => assert_eq!(paths.weight(source, target), None),
                (Some(expected), Some(actual)) => {
                    assert!((expected.weight() - actual.weight()).abs() < 1e-9);
                    assert_eq!(expected.vertex_list(), actual.vertex_list());
                }
                pair => panic!("{source}->{target}: {pair:?}"),
            }
        }
    }
}

fn random_graphs(
    kind: Kind,
    n: usize,
    degree: usize,
    set_sizes: &[(usize, usize)],
    iterations: usize,
) {
    let mut random = StdRng::seed_from_u64(17);
    for &(source_count, target_count) in set_sizes {
        for _ in 0..iterations {
            let graph = generate_random_graph(n, degree * n, &mut random);
            let sources = random_vertices(n, source_count, &mut random);
            let targets = random_vertices(n, target_count, &mut random);
            let source_targets = compute(
                kind,
                &graph,
                sources.iter().copied(),
                targets.iter().copied(),
            )
            .unwrap();
            let source_sources = compute(
                kind,
                &graph,
                sources.iter().copied(),
                sources.iter().copied(),
            )
            .unwrap();
            assert_correct_paths(&graph, &source_targets, &sources, &targets);
            assert_correct_paths(&graph, &source_sources, &sources, &sources);
        }
    }
}

fn assert_single_source<G: Graph<Vertex = usize, Edge = DefaultEdge> + OutgoingGraph>(
    graph: &G,
    actual: &SingleSourcePaths<usize, DefaultEdge>,
    source: usize,
) {
    let expected = DijkstraShortestPath::new(graph)
        .single_source(&source)
        .unwrap();
    assert_eq!(actual.source(), &source);
    for target in graph.vertices() {
        assert_eq!(actual.weight(target), expected.weight(target));
        assert_eq!(actual.path(target), expected.path(target));
    }
}

mod default_many_to_many {
    use super::*;

    #[test]
    fn test_empty_graph() {
        empty_graph(Kind::Default);
    }

    #[test]
    fn test_no_path() {
        no_path(Kind::Default);
    }

    #[test]
    fn test_different_sources_and_targets_simple_graph() {
        different_sources_and_targets_simple_graph(Kind::Default);
    }

    #[test]
    fn test_different_sources_and_targets_multigraph() {
        different_sources_and_targets_multigraph(Kind::Default);
    }

    #[test]
    fn test_sources_equal_targets_simple_graph() {
        sources_equal_targets_simple_graph(Kind::Default);
    }

    #[test]
    fn test_sources_equal_targets_multigraph() {
        sources_equal_targets_multigraph(Kind::Default);
    }

    #[test]
    fn test_on_random_graphs() {
        random_graphs(Kind::Default, 30, 5, &[(5, 10), (5, 5), (10, 5)], 10);
    }

    struct CountingDijkstra<'g, G: Graph> {
        inner: Dijkstra<'g, G>,
        calls: Rc<Cell<usize>>,
    }

    impl<G: OutgoingGraph> ShortestPathAlgorithm<G> for CountingDijkstra<'_, G> {
        fn single_source(
            &self,
            source: &G::Vertex,
        ) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
            self.inner.single_source(source)
        }

        fn path(
            &self,
            source: &G::Vertex,
            sink: &G::Vertex,
        ) -> Result<Option<jgrapht::GraphWalk<G::Vertex, G::Edge>>> {
            self.calls.set(self.calls.get() + 1);
            self.inner.path(source, sink)
        }
    }

    #[test]
    fn test_get_paths_consults_provided_algorithm_function() {
        let mut graph = DiGraph::<usize>::simple().weighted();
        for (source, target, weight) in [(1, 2, 1.0), (2, 3, 1.0), (1, 3, 3.0)] {
            add_directed(&mut graph, source, target, weight);
        }
        let calls = Rc::new(Cell::new(0));
        let counter = Rc::clone(&calls);
        let algorithm =
            DefaultManyToManyShortestPaths::with_algorithm(&graph, move |graph| CountingDijkstra {
                inner: Dijkstra::new(graph),
                calls: Rc::clone(&counter),
            });
        let paths = algorithm.single_source(&1).unwrap();
        assert_single_source(&graph, &paths, 1);
        assert!(calls.get() > 0);
    }
}

mod dijkstra_many_to_many {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_empty_graph() {
        empty_graph(Kind::Dijkstra);
    }

    #[test]
    fn test_no_path() {
        no_path(Kind::Dijkstra);
    }

    #[test]
    fn test_no_path_multiset() {
        let mut graph = DiGraph::<usize>::multi().weighted();
        graph.add_vertices([1, 2, 3]);
        let paths = compute(Kind::Dijkstra, &graph, [1], [2, 3]).unwrap();
        for target in [2, 3] {
            assert_eq!(paths.weight(&1, &target), None);
            assert!(paths.path(&1, &target).is_none());
        }
    }

    #[test]
    fn test_different_sources_and_targets_simple_graph() {
        different_sources_and_targets_simple_graph(Kind::Dijkstra);
    }

    #[test]
    fn test_different_sources_and_targets_multigraph() {
        different_sources_and_targets_multigraph(Kind::Dijkstra);
    }

    #[test]
    fn test_sources_equal_targets_simple_graph() {
        sources_equal_targets_simple_graph(Kind::Dijkstra);
    }

    #[test]
    fn test_sources_equal_targets_multigraph() {
        sources_equal_targets_multigraph(Kind::Dijkstra);
    }

    #[test]
    fn test_on_random_graphs() {
        random_graphs(Kind::Dijkstra, 100, 20, &[(50, 30), (40, 40), (30, 50)], 50);
    }

    fn directed_graph(vertices: &[usize], edges: &[(usize, usize, f64)]) -> DiGraph<usize> {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertices(vertices.iter().copied());
        for &(source, target, weight) in edges {
            graph.add_weighted_edge(&source, &target, weight).unwrap();
        }
        graph
    }

    #[test]
    fn test_get_paths_single_source_matches_dijkstra() {
        let graph = directed_graph(
            &[1, 2, 3, 4, 5],
            &[(1, 2, 1.5), (2, 3, 2.0), (1, 3, 5.0), (3, 4, 1.0)],
        );
        let paths = DijkstraManyToManyShortestPaths::new(&graph)
            .single_source(&1)
            .unwrap();
        assert_single_source(&graph, &paths, 1);
        assert_eq!(paths.path(&1).unwrap().length(), 0);
    }

    #[test]
    fn test_get_paths_rejects_vertex_not_in_graph() {
        let graph = directed_graph(&[1], &[]);
        assert!(
            DijkstraManyToManyShortestPaths::new(&graph)
                .single_source(&42)
                .is_err()
        );
    }

    #[test]
    fn test_get_paths_single_vertex_graph() {
        let graph = directed_graph(&[7], &[]);
        let paths = DijkstraManyToManyShortestPaths::new(&graph)
            .single_source(&7)
            .unwrap();
        assert_single_source(&graph, &paths, 7);
        assert_eq!(paths.path(&7).unwrap().length(), 0);
    }

    #[test]
    fn test_get_paths_source_has_no_outgoing_edges() {
        let graph = directed_graph(&[1, 2, 3], &[(2, 3, 1.0)]);
        let paths = DijkstraManyToManyShortestPaths::new(&graph)
            .single_source(&1)
            .unwrap();
        assert_single_source(&graph, &paths, 1);
    }

    #[test]
    fn test_get_paths_disconnected_graph() {
        let graph = directed_graph(
            &[1, 2, 3, 10, 11],
            &[(1, 2, 1.0), (2, 3, 1.0), (10, 11, 1.0)],
        );
        let paths = DijkstraManyToManyShortestPaths::new(&graph)
            .single_source(&1)
            .unwrap();
        assert_single_source(&graph, &paths, 1);
    }

    #[test]
    fn test_get_paths_cyclic_graph_with_self_loop_on_source() {
        let graph = directed_graph(
            &[1, 2, 3, 4],
            &[
                (1, 1, 5.0),
                (1, 2, 1.0),
                (2, 3, 1.0),
                (3, 2, 1.0),
                (3, 4, 1.0),
            ],
        );
        let paths = DijkstraManyToManyShortestPaths::new(&graph)
            .single_source(&1)
            .unwrap();
        assert_single_source(&graph, &paths, 1);
        assert_eq!(paths.path(&1).unwrap().length(), 0);
    }

    #[test]
    fn test_get_paths_called_twice_returns_consistent_results() {
        let mut random = StdRng::seed_from_u64(11);
        let graph = random_directed_graph(&mut random, 30);
        let algorithm = DijkstraManyToManyShortestPaths::new(&graph);
        let first = algorithm.single_source(&0).unwrap();
        let second = algorithm.single_source(&0).unwrap();
        for target in graph.vertices() {
            assert_eq!(first.weight(target), second.weight(target));
            assert_eq!(first.path(target), second.path(target));
        }
    }

    fn random_directed_graph(random: &mut StdRng, n: usize) -> DiGraph<usize> {
        let mut graph = DiGraph::<usize>::simple().weighted();
        graph.add_vertices(0..n);
        for source in 0..n {
            for target in 0..n {
                if source != target && random.next_double() < 0.2 {
                    graph
                        .add_weighted_edge(
                            &source,
                            &target,
                            1.0 + random.next_int_bound(1000) as f64,
                        )
                        .unwrap();
                }
            }
        }
        graph
    }

    #[test]
    fn test_get_paths_fuzz_against_dijkstra_oracle() {
        for seed in [1, 2, 3, 5, 7, 11, 13, 17, 19, 23] {
            let sizes = [10, 20, 35, 50, 80];
            let n = sizes[seed % sizes.len()];
            let mut random = StdRng::seed_from_u64((seed as i64) as u64);
            let graph = random_directed_graph(&mut random, n);
            let algorithm = DijkstraManyToManyShortestPaths::new(&graph);
            for source in graph.vertices() {
                let paths = algorithm.single_source(source).unwrap();
                assert_single_source(&graph, &paths, *source);
            }
        }
    }
}

mod contraction_hierarchy_many_to_many {
    use super::*;

    #[test]
    fn test_empty_graph() {
        empty_graph(Kind::ContractionHierarchy);
    }

    #[test]
    fn test_no_path() {
        no_path(Kind::ContractionHierarchy);
    }

    #[test]
    fn test_different_sources_and_targets_simple_graph() {
        different_sources_and_targets_simple_graph(Kind::ContractionHierarchy);
    }

    #[test]
    fn test_different_sources_and_targets_multigraph() {
        different_sources_and_targets_multigraph(Kind::ContractionHierarchy);
    }

    #[test]
    fn test_sources_equal_targets_simple_graph() {
        sources_equal_targets_simple_graph(Kind::ContractionHierarchy);
    }

    #[test]
    fn test_sources_equal_targets_multigraph() {
        sources_equal_targets_multigraph(Kind::ContractionHierarchy);
    }

    #[test]
    fn test_more_sources_than_targets_1() {
        let graph = simple_graph();
        let paths = compute(Kind::ContractionHierarchy, &graph, [1, 3, 7, 9], [5]).unwrap();
        for (source, vertices) in [
            (1, &[1, 4, 5][..]),
            (3, &[3, 6, 5]),
            (7, &[7, 4, 5]),
            (9, &[9, 6, 5]),
        ] {
            assert_path(&paths, source, 5, 2.0, vertices);
        }
    }

    #[test]
    fn test_more_sources_than_targets_2() {
        let graph = multigraph();
        let paths = compute(Kind::ContractionHierarchy, &graph, [2, 3, 4, 5, 6], [1]).unwrap();
        for (source, weight, vertices) in [
            (2, 3.0, &[2, 1][..]),
            (3, 8.0, &[3, 2, 1]),
            (4, 19.0, &[4, 3, 2, 1]),
            (5, 32.0, &[5, 4, 3, 2, 1]),
            (6, 23.0, &[6, 1]),
        ] {
            assert_path(&paths, source, 1, weight, vertices);
        }
    }

    #[test]
    fn test_on_random_graphs() {
        random_graphs(
            Kind::ContractionHierarchy,
            40,
            5,
            &[(10, 15), (10, 10), (15, 10)],
            10,
        );
    }

    #[test]
    fn test_get_paths_matches_dijkstra_oracle() {
        let graph = simple_graph();
        let hierarchy = ContractionHierarchyPrecomputation::new(&graph)
            .compute_contraction_hierarchy()
            .unwrap();
        let paths = CHManyToManyShortestPaths::new(&hierarchy)
            .single_source(&1)
            .unwrap();
        assert_single_source(&graph, &paths, 1);
    }
}
