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

//! Ports of the tests of `org.jgrapht.alg.matching`, including the public Blossom V fixtures:
//! `GreedyMaximumCardinalityMatchingTest`, `ApproximateWeightedMatchingTest` with its subclasses,
//! `MaximumCardinalityBipartiteMatchingTest` (Hopcroft–Karp),
//! `MaximumWeightBipartiteMatchingTest`, `KuhnMunkresMinimalWeightBipartitePerfectMatchingTest`,
//! `DenseEdmondsMaximumCardinalityMatchingTest` and `SparseEdmondsMaximumCardinalityMatchingTest`.

use indexmap::IndexSet;
use jgrapht::alg::matching::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

type UG = UGraph<i32>;
type M = Matching<i32, DefaultEdge>;

/// `maxEdges(n)` of the Java tests.
fn max_edges(n: usize) -> usize {
    if n.is_multiple_of(2) {
        n / 2 * (n - 1)
    } else {
        n * ((n - 1) / 2)
    }
}

fn integer_graph() -> UG {
    UG::simple().with_vertex_factory(|i| i as i32)
}

/// `verifyMatching` of the cardinality tests: a matching of the given cardinality whose weight
/// is the sum of its edges.
fn verify_matching<G: Graph>(g: &G, m: &Matching<G::Vertex, G::Edge>, cardinality: usize) {
    let mut matched: HashSet<G::Vertex> = HashSet::new();
    let mut weight = 0.0;
    for e in m.edges() {
        let (source, target) = g.edge_endpoints(e).unwrap();
        assert!(
            matched.insert(source.clone()),
            "vertex is incident to multiple matches in the matching"
        );
        assert!(
            matched.insert(target.clone()),
            "vertex is incident to multiple matches in the matching"
        );
        weight += g.edge_weight(e).unwrap_or(1.0);
    }
    assert!((m.weight() - weight).abs() < 0.0000001);
    assert_eq!(cardinality, m.edges().len());
    assert_eq!(m.edges().len() * 2, matched.len()); // no self-loops
}

/// `isMatching` of `ApproximateWeightedMatchingTest`.
fn is_matching<G: Graph>(g: &G, m: &Matching<G::Vertex, G::Edge>) -> bool {
    let mut matched: HashSet<G::Vertex> = HashSet::new();
    for e in m.edges() {
        let (source, target) = g.edge_endpoints(e).unwrap();
        if !matched.insert(source.clone()) || !matched.insert(target.clone()) {
            return false;
        }
    }
    matched.len() == m.edges().len() * 2
}

fn add_edges<G: GraphMut<Vertex = i32>>(g: &mut G, edges: &[(i32, i32)]) {
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
}

fn add_weighted_edges<G: GraphMut<Vertex = i32>>(g: &mut G, edges: &[(i32, i32, f64)]) {
    for (a, b, w) in edges {
        g.add_weighted_edge(a, b, *w).unwrap();
    }
}

// ---- GreedyMaximumCardinalityMatchingTest ----------------------------------------------------

mod greedy_maximum_cardinality_matching_test {
    use super::*;

    fn run(sorted: bool, repeat: usize) {
        let mut generator =
            GnmRandomGraphGenerator::new(200, 120, StdRng::seed_from_u64(7), false, false).unwrap();
        for _ in 0..repeat {
            let mut graph = integer_graph();
            generator.generate_default(&mut graph).unwrap();
            let m = GreedyMaximumCardinalityMatching::new(&graph, sorted)
                .unwrap()
                .get_matching()
                .unwrap();
            let mut matched = HashSet::new();
            let mut weight = 0.0;
            for e in m.edges() {
                let (source, target) = graph.edge_endpoints(e).unwrap();
                assert!(matched.insert(*source));
                assert!(matched.insert(*target));
                weight += graph.edge_weight(e).unwrap();
            }
            assert!((m.weight() - weight).abs() < 0.0000001);
        }
    }

    #[test]
    fn test_random_graphs() {
        run(false, 100);
    }

    #[test]
    fn test_random_graphs2() {
        run(true, 1);
    }
}

// ---- ApproximateWeightedMatchingTest ---------------------------------------------------------

type Wpg = UGraph<i32>;

fn weighted_pseudograph(vertices: &[i32]) -> Wpg {
    let mut g = Wpg::pseudo().weighted();
    g.add_vertices(vertices.iter().copied());
    g
}

fn two_squares() -> Wpg {
    let mut g = weighted_pseudograph(&[0, 1, 2, 3]);
    add_weighted_edges(
        &mut g,
        &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 0, 1.0)],
    );
    g.add_vertices([4, 5, 6, 7]);
    add_weighted_edges(
        &mut g,
        &[(4, 5, 1.0), (5, 6, 1.0), (6, 7, 1.0), (7, 4, 1.0)],
    );
    g
}

const MULTI_EDGES: [(i32, i32, f64); 8] = [
    (0, 1, 2.0),
    (1, 2, 2.0),
    (2, 3, 2.0),
    (3, 0, 2.0),
    (4, 5, 2.0),
    (5, 6, 2.0),
    (6, 7, 2.0),
    (7, 4, 2.0),
];

fn graph1() -> Wpg {
    let mut g = weighted_pseudograph(&(0..15).collect::<Vec<_>>());
    add_weighted_edges(
        &mut g,
        &[
            (0, 1, 5.0),
            (1, 2, 2.5),
            (2, 3, 5.0),
            (3, 4, 2.5),
            (4, 0, 2.5),
            (0, 13, 2.5),
            (13, 14, 5.0),
            (1, 11, 2.5),
            (11, 12, 5.0),
            (2, 9, 2.5),
            (9, 10, 5.0),
            (3, 7, 2.5),
            (7, 8, 5.0),
            (4, 5, 2.5),
            (5, 6, 5.0),
        ],
    );
    g
}

fn self_loops_graph() -> Wpg {
    let mut g = two_squares();
    add_weighted_edges(
        &mut g,
        &[
            (0, 0, 100.0),
            (1, 1, 200.0),
            (2, 2, -200.0),
            (3, 3, -100.0),
            (4, 4, 0.0),
        ],
    );
    g
}

fn multi_graph() -> Wpg {
    let mut g = two_squares();
    add_weighted_edges(&mut g, &MULTI_EDGES);
    g
}

fn directed_graph() -> DiGraph<i32> {
    let mut g = DiGraph::<i32>::pseudo().weighted();
    g.add_vertices(0..8);
    add_weighted_edges(
        &mut g,
        &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 0, 1.0)],
    );
    add_weighted_edges(
        &mut g,
        &[(4, 5, 1.0), (5, 6, 1.0), (6, 7, 1.0), (7, 4, 1.0)],
    );
    add_weighted_edges(&mut g, &MULTI_EDGES);
    g
}

/// The tests of `ApproximateWeightedMatchingTest`. `$alg` builds the algorithm for the graph
/// `$g`; the numbers are the expectations that differ between the subclasses.
macro_rules! approximate_weighted_matching_tests {
    (
        $g:ident => $alg:expr;
        graph1: ($graph1_edges:expr, $graph1_weight:expr),
        self_loops: ($loops_edges:expr, $loops_weight:expr),
        three_over_four: ($three_edges:expr, $three_weight:expr),
        multi_graph: ($multi_edges:expr, $multi_weight:expr),
        directed: ($directed_edges:expr, $directed_weight:expr)
    ) => {
        fn run<G: IncidenceGraph<Vertex = i32, Edge = DefaultEdge>>(graph: &G) -> M {
            let $g = graph;
            let mut alg = $alg;
            alg.get_matching().unwrap()
        }

        #[test]
        fn test_path1() {
            let mut g = weighted_pseudograph(&[0, 1, 2, 3, 4, 5]);
            add_weighted_edges(
                &mut g,
                &[
                    (0, 1, 1.0),
                    (1, 2, 1.0),
                    (2, 3, 1.0),
                    (3, 4, 1.0),
                    (4, 5, 1.0),
                ],
            );
            let m = run(&g);
            assert_eq!(3, m.edges().len());
            assert!((m.weight() - 3.0).abs() < DEFAULT_EPSILON);
            assert!(m.edges().contains(g.get_edge(&0, &1).unwrap()));
            assert!(m.edges().contains(g.get_edge(&2, &3).unwrap()));
            assert!(m.edges().contains(g.get_edge(&4, &5).unwrap()));
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_path2() {
            let mut g = weighted_pseudograph(&[0, 1, 2, 3, 4, 5]);
            add_weighted_edges(
                &mut g,
                &[
                    (0, 1, 1.0),
                    (1, 2, 5.0),
                    (2, 3, 1.0),
                    (3, 4, 5.0),
                    (4, 5, 1.0),
                ],
            );
            let m = run(&g);
            assert_eq!(2, m.edges().len());
            assert!((m.weight() - 10.0).abs() < DEFAULT_EPSILON);
            assert!(m.edges().contains(g.get_edge(&1, &2).unwrap()));
            assert!(m.edges().contains(g.get_edge(&3, &4).unwrap()));
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_negative_and_zero_edges() {
            let mut g = weighted_pseudograph(&[0, 1, 2, 3]);
            add_weighted_edges(
                &mut g,
                &[
                    (0, 1, -1.0),
                    (1, 2, -5.0),
                    (2, 3, -1.0),
                    (3, 0, -1.0),
                    (3, 1, 0.0),
                    (0, 2, 0.0),
                ],
            );
            let m = run(&g);
            assert_eq!(0, m.edges().len());
            assert!(m.weight().abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_negative_and_zero_edges1() {
            let mut g = weighted_pseudograph(&[0, 1, 2, 3]);
            add_weighted_edges(
                &mut g,
                &[
                    (0, 1, -1.0),
                    (1, 2, -5.0),
                    (2, 3, -1.0),
                    (3, 0, -1.0),
                    (3, 1, -1.0),
                    (0, 2, -1.0),
                ],
            );
            let m = run(&g);
            assert_eq!(0, m.edges().len());
            assert!(m.weight().abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_negative_and_zero_edges2() {
            let mut g = weighted_pseudograph(&(0..13).collect::<Vec<_>>());
            add_weighted_edges(
                &mut g,
                &[
                    (0, 1, 1.0),
                    (1, 3, 1.0),
                    (2, 4, -1.0),
                    (3, 5, -1.0),
                    (4, 6, -1.0),
                    (5, 6, -1.0),
                    (6, 7, -1.0),
                    (6, 8, -1.0),
                    (7, 9, -1.0),
                    (8, 10, -1.0),
                    (9, 11, 1.0),
                    (10, 12, 1.0),
                ],
            );
            let m = run(&g);
            assert!(is_matching(&g, &m));
            assert!(m.weight() >= 2.0);
            assert!(m.edges().len() >= 2);
        }

        #[test]
        fn test_graph1() {
            let g = graph1();
            let m = run(&g);
            assert_eq!($graph1_edges, m.edges().len());
            assert!((m.weight() - $graph1_weight).abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test3over4_approximation() {
            let g = two_squares();
            let m = run(&g);
            assert_eq!($three_edges, m.edges().len());
            assert!((m.weight() - $three_weight).abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_self_loops() {
            let g = self_loops_graph();
            let m = run(&g);
            assert_eq!($loops_edges, m.edges().len());
            assert!((m.weight() - $loops_weight).abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_multi_graph() {
            let g = multi_graph();
            let m = run(&g);
            assert_eq!($multi_edges, m.edges().len());
            assert!((m.weight() - $multi_weight).abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_directed() {
            let g = directed_graph();
            let m = run(&g);
            assert_eq!($directed_edges, m.edges().len());
            assert!((m.weight() - $directed_weight).abs() < DEFAULT_EPSILON);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_disconnected_and_isolated_vertices() {
            let mut g = two_squares();
            g.add_vertices([8, 9, 10, 11]);
            let m = run(&g);
            assert!(m.weight() >= 2.0);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_self_loop() {
            let mut g = weighted_pseudograph(&[0, 1, 2, 3]);
            add_weighted_edges(
                &mut g,
                &[
                    (0, 1, 4.0),
                    (1, 2, 1.0),
                    (2, 3, 8.0),
                    (3, 0, 3.0),
                    (3, 3, 100.0),
                ],
            );
            let m = run(&g);
            assert!(is_matching(&g, &m));
        }

        #[test]
        fn test_bn_graph() {
            // create graphs which have a perfect matching
            for size in 1..100 {
                let mut graph = UG::simple();
                for i in 0..size {
                    graph.add_vertex(i);
                }
                for i in 0..size {
                    graph.add_vertex(i + size);
                    graph.add_edge(&i, &(i + size)).unwrap();
                }
                for i in 0..size {
                    for j in i + 1..size {
                        graph.add_edge(&i, &j).unwrap();
                    }
                }
                let matching = run(&graph);
                let weight = matching.weight();
                assert!(is_matching(&graph, &matching));
                assert!(weight >= size as f64 / 2.0);
            }
        }
    };
}

mod greedy_weighted_matching_test {
    use super::*;

    approximate_weighted_matching_tests! {
        g => GreedyWeightedMatching::new(g, false).unwrap();
        graph1: (7, 35.0),
        self_loops: (4, 4.0),
        three_over_four: (4, 4.0),
        multi_graph: (4, 8.0),
        directed: (4, 8.0)
    }
}

/// `BasePathGrowingWeightedMatchingTest`'s own tests.
macro_rules! base_path_growing_tests {
    () => {
        #[test]
        fn test_dynamic_programming_on_paths() {
            // test 0
            let mut g = Wpg::pseudo().weighted();
            PathGrowingWeightedMatching::new(&g).unwrap();
            g.add_vertices(0..7);
            let mut path = Vec::new();
            for (a, b, w) in [
                (0, 1, 5.0),
                (1, 2, 2.5),
                (2, 3, 5.0),
                (3, 4, 2.5),
                (4, 5, 2.5),
                (5, 6, 5.0),
            ] {
                path.push(g.add_weighted_edge(&a, &b, w).unwrap().unwrap());
            }
            let mut path_solver = PathGrowingWeightedMatching::new(&g).unwrap().path_solver();
            let (weight, matching) = path_solver.get_maximum_weight_matching(&g, &path);
            assert!((weight - 15.0).abs() < DEFAULT_EPSILON);
            assert_eq!(3, matching.len());
            assert!(matching.contains(g.get_edge(&0, &1).unwrap()));
            assert!(matching.contains(g.get_edge(&2, &3).unwrap()));
            assert!(matching.contains(g.get_edge(&5, &6).unwrap()));

            // test 1 (empty path)
            let (weight1, matching1) = path_solver.get_maximum_weight_matching(&g, &[]);
            assert!(weight1.abs() < DEFAULT_EPSILON);
            assert_eq!(0, matching1.len());

            // test 2 (single edge)
            g.add_vertices([7, 8]);
            let path2 = vec![g.add_weighted_edge(&7, &8, 100.0).unwrap().unwrap()];
            let (weight2, matching2) = path_solver.get_maximum_weight_matching(&g, &path2);
            assert!((weight2 - 100.0).abs() < DEFAULT_EPSILON);
            assert_eq!(1, matching2.len());
            assert!(matching2.contains(g.get_edge(&7, &8).unwrap()));

            // test 3 (two edges)
            g.add_vertices([9, 10, 11]);
            let path3 = vec![
                g.add_weighted_edge(&9, &10, 10.0).unwrap().unwrap(),
                g.add_weighted_edge(&10, &11, 15.0).unwrap().unwrap(),
            ];
            let (weight3, matching3) = path_solver.get_maximum_weight_matching(&g, &path3);
            assert!((weight3 - 15.0).abs() < DEFAULT_EPSILON);
            assert_eq!(1, matching3.len());
            assert!(matching3.contains(g.get_edge(&10, &11).unwrap()));
        }

        #[test]
        fn test_approximation_factor_on_random_instances() {
            let (edge_probability, number_vertices, repeat) = (0.7, 100, 10);
            let mut gg = GnpRandomGraphGenerator::new(
                number_vertices,
                edge_probability,
                StdRng::seed_from_u64(33),
                false,
            )
            .unwrap();
            for _ in 0..repeat {
                let mut g = Wpg::pseudo().weighted().with_vertex_factory(|i| i as i32);
                gg.generate_default(&mut g).unwrap();
                let m1 = PathGrowingWeightedMatching::new(&g)
                    .unwrap()
                    .get_matching()
                    .unwrap();
                let m2 = PathGrowingWeightedMatching::with_heuristics(&g, false)
                    .unwrap()
                    .get_matching()
                    .unwrap();
                let m3 = SparseEdmondsMaximumCardinalityMatching::new(&g)
                    .unwrap()
                    .get_matching()
                    .unwrap();
                let m4 = GreedyWeightedMatching::new(&g, false)
                    .unwrap()
                    .get_matching()
                    .unwrap();
                assert!(is_matching(&g, &m1));
                assert!(is_matching(&g, &m2));
                assert!(is_matching(&g, &m3));
                assert!(is_matching(&g, &m4));
                assert!(m1.edges().len() as f64 >= 0.5 * m3.edges().len() as f64);
                assert!(m2.edges().len() as f64 >= 0.5 * m3.edges().len() as f64);
                assert!(m4.edges().len() as f64 >= 0.5 * m3.edges().len() as f64);
            }
        }
    };
}

mod path_growing_weighted_matching_test {
    use super::*;

    approximate_weighted_matching_tests! {
        g => PathGrowingWeightedMatching::new(g).unwrap();
        graph1: (7, 35.0),
        self_loops: (4, 4.0),
        three_over_four: (4, 4.0),
        multi_graph: (4, 8.0),
        directed: (4, 8.0)
    }

    base_path_growing_tests!();
}

mod no_heuristics_path_growing_weighted_matching_test {
    use super::*;

    approximate_weighted_matching_tests! {
        g => PathGrowingWeightedMatching::with_heuristics(g, false).unwrap();
        graph1: (5, 22.5),
        self_loops: (3, 3.0),
        three_over_four: (3, 3.0),
        multi_graph: (3, 6.0),
        directed: (3, 6.0)
    }

    base_path_growing_tests!();
}

// ---- MaximumCardinalityBipartiteMatchingTest (Hopcroft-Karp) ---------------------------------

mod hopcroft_karp_maximum_cardinality_bipartite_matching_test {
    use super::*;

    fn algorithm<'g, G: jgrapht::UndirectedGraph<Vertex = i32> + IncidenceGraph>(
        graph: &'g G,
        partition1: impl IntoIterator<Item = i32>,
        partition2: impl IntoIterator<Item = i32>,
    ) -> HopcroftKarpMaximumCardinalityBipartiteMatching<'g, G> {
        HopcroftKarpMaximumCardinalityBipartiteMatching::new(graph, partition1, partition2).unwrap()
    }

    /// `verifyMatching` of the base class: also certified maximum by `DenseEdmonds`.
    fn verify<G: jgrapht::UndirectedGraph<Vertex = i32, Edge = DefaultEdge> + IncidenceGraph>(
        g: &G,
        m: &M,
        cardinality: usize,
    ) {
        verify_matching(g, m, cardinality);
        let mut matcher = DenseEdmondsMaximumCardinalityMatching::new(g).unwrap();
        assert!(matcher.is_maximum_matching(m).unwrap());
    }

    #[test]
    fn test_bipartite_matching1() {
        let mut graph = UG::simple();
        graph.add_vertices(0..8);
        let e02 = graph.add_edge(&0, &6).unwrap().unwrap();
        let e11 = graph.add_edge(&1, &5).unwrap().unwrap();
        let e20 = graph.add_edge(&2, &4).unwrap().unwrap();
        let bm = algorithm(&graph, 0..4, 4..8).get_matching().unwrap();
        assert_eq!(3, bm.edges().len());
        let expected: HashSet<DefaultEdge> = [e11, e02, e20].into_iter().collect();
        assert_eq!(expected, bm.edges().iter().copied().collect());
    }

    #[test]
    fn test_bipartite_matching2() {
        let mut graph = UG::simple();
        graph.add_vertices(0..12);
        let es: Vec<DefaultEdge> = [(0, 6), (1, 9), (2, 7), (3, 10), (4, 8), (5, 11)]
            .iter()
            .map(|(a, b)| graph.add_edge(a, b).unwrap().unwrap())
            .collect();
        let bm = algorithm(&graph, 0..6, 6..12).get_matching().unwrap();
        assert_eq!(6, bm.edges().len());
        assert_eq!(
            es.into_iter().collect::<HashSet<_>>(),
            bm.edges().iter().copied().collect()
        );
    }

    #[test]
    fn test_empty_matching() {
        let mut graph = UG::simple();
        graph.add_vertices([0, 1]);
        let bm = algorithm(&graph, [0], [1]).get_matching().unwrap();
        assert!(bm.edges().is_empty());
    }

    #[test]
    fn test_graph1() {
        let mut graph = UG::simple();
        graph.add_vertices(0..10);
        add_edges(&mut graph, &[(5, 8), (4, 9), (2, 7), (6, 9), (1, 9)]);
        let matching = algorithm(&graph, 0..7, 7..10).get_matching().unwrap();
        assert_eq!(3, matching.edges().len());
    }

    #[test]
    fn test_graph2() {
        let mut graph = UG::simple();
        graph.add_vertices(0..10);
        add_edges(
            &mut graph,
            &[
                (5, 8),
                (4, 9),
                (2, 7),
                (6, 9),
                (1, 9),
                (0, 8),
                (3, 7),
                (1, 7),
            ],
        );
        let matching = algorithm(&graph, 0..7, 7..10).get_matching().unwrap();
        verify(&graph, &matching, matching.edges().len());
    }

    /// Issue 233 instance.
    #[test]
    fn test_bipartite_matching_issue233() {
        let mut g = UG::simple();
        g.add_vertices(0..4);
        add_edges(&mut g, &[(0, 2), (0, 3), (1, 2)]);
        let m = algorithm(&g, [0, 1], [2, 3]).get_matching().unwrap();
        assert!(m.edges().contains(g.get_edge(&1, &2).unwrap()));
        assert!(m.edges().contains(g.get_edge(&0, &3).unwrap()));
        assert_eq!(2, m.edges().len());
    }

    #[test]
    fn test_pseudo_graph() {
        let mut graph = UG::pseudo();
        graph.add_vertices(0..6);
        add_edges(&mut graph, &[(0, 3), (1, 4), (2, 5), (0, 3), (0, 0)]);
        let matching = algorithm(&graph, 0..3, 3..6).get_matching().unwrap();
        verify(&graph, &matching, 3);
    }

    #[test]
    fn test_random_bipartite_graphs() {
        let mut random = StdRng::seed_from_u64(1);
        let vertices = 100;
        for _ in 0..100 {
            let edges = random.random_range(0..max_edges(vertices) / 2);
            let mut generator = GnmRandomBipartiteGraphGenerator::new(
                vertices,
                vertices / 2,
                edges,
                StdRng::seed_from_u64(0),
            );
            let mut graph = integer_graph();
            let (first, second) = generator.generate_partitions(&mut graph).unwrap();
            let m = algorithm(&graph, first, second).get_matching().unwrap();
            verify(&graph, &m, m.edges().len());
        }
    }
}

// ---- MaximumWeightBipartiteMatchingTest ------------------------------------------------------

mod maximum_weight_bipartite_matching_test {
    use super::*;

    type SG = UGraph<&'static str>;

    fn set_up_graph() -> (SG, IndexSet<&'static str>, IndexSet<&'static str>) {
        let mut graph = SG::simple().weighted();
        for v in ["s1", "s2", "s3", "s4", "t1", "t2", "t3", "t4"] {
            graph.add_vertex(v);
        }
        (
            graph,
            ["s1", "s2", "s3", "s4"].into_iter().collect(),
            ["t1", "t2", "t3", "t4"].into_iter().collect(),
        )
    }

    fn edge(g: &mut SG, a: &'static str, b: &'static str, w: f64) -> DefaultEdge {
        g.add_weighted_edge(&a, &b, w).unwrap().unwrap()
    }

    #[test]
    fn maximum_weight_bipartite_matching1() {
        let (mut graph, p1, p2) = set_up_graph();
        let e1 = edge(&mut graph, "s1", "t1", 1.0);
        let matchings = MaximumWeightBipartiteMatching::new(&graph, p1, p2)
            .unwrap()
            .get_matching()
            .unwrap();
        assert_eq!(1, matchings.edges().len());
        assert!(matchings.edges().contains(&e1));
    }

    #[test]
    fn maximum_weight_bipartite_matching2() {
        let (mut graph, p1, p2) = set_up_graph();
        edge(&mut graph, "s1", "t1", 1.0);
        let e2 = edge(&mut graph, "s2", "t1", 2.0);
        let matchings = MaximumWeightBipartiteMatching::new(&graph, p1, p2)
            .unwrap()
            .get_matching()
            .unwrap();
        assert_eq!(1, matchings.edges().len());
        assert!(matchings.edges().contains(&e2));
    }

    #[test]
    fn maximum_weight_bipartite_matching3() {
        let (mut graph, p1, p2) = set_up_graph();
        edge(&mut graph, "s1", "t1", 2.0);
        let e2 = edge(&mut graph, "s1", "t2", 1.0);
        let e3 = edge(&mut graph, "s2", "t1", 2.0);
        let matchings = MaximumWeightBipartiteMatching::new(&graph, p1, p2)
            .unwrap()
            .get_matching()
            .unwrap();
        assert_eq!(2, matchings.edges().len());
        assert!(matchings.edges().contains(&e2));
        assert!(matchings.edges().contains(&e3));
    }

    #[test]
    fn maximum_weight_bipartite_matching4() {
        let (mut graph, p1, p2) = set_up_graph();
        let e1 = edge(&mut graph, "s1", "t1", 1.0);
        edge(&mut graph, "s1", "t2", 1.0);
        let e3 = edge(&mut graph, "s2", "t2", 1.0);
        let matchings = MaximumWeightBipartiteMatching::new(&graph, p1, p2)
            .unwrap()
            .get_matching()
            .unwrap();
        assert_eq!(2, matchings.edges().len());
        assert!(matchings.edges().contains(&e1));
        assert!(matchings.edges().contains(&e3));
    }

    #[test]
    fn maximum_weight_bipartite_matching5() {
        let (mut graph, p1, p2) = set_up_graph();
        let e1 = edge(&mut graph, "s1", "t1", 1.0);
        edge(&mut graph, "s1", "t2", 2.0);
        let e3 = edge(&mut graph, "s2", "t2", 2.0);
        edge(&mut graph, "s3", "t2", 2.0);
        let e5 = edge(&mut graph, "s3", "t3", 1.0);
        edge(&mut graph, "s4", "t1", 1.0);
        let e7 = edge(&mut graph, "s4", "t4", 1.0);
        let matchings = MaximumWeightBipartiteMatching::new(&graph, p1, p2)
            .unwrap()
            .get_matching()
            .unwrap();
        assert_eq!(4, matchings.edges().len());
        assert!(matchings.edges().contains(&e1));
        assert!(matchings.edges().contains(&e3));
        assert!(matchings.edges().contains(&e5));
        assert!(matchings.edges().contains(&e7));
        assert!((matchings.weight() - 5.0).abs() < 1e-9);
    }

    fn test_random_instance(rng: &mut StdRng, n: i32, p: f64, repeat: usize) {
        for _ in 0..repeat {
            // generate random bipartite
            let mut g = UG::simple().weighted();
            let mut partition_a = IndexSet::new();
            for i in 0..n {
                g.add_vertex(i);
                partition_a.insert(i);
            }
            let mut partition_b = IndexSet::new();
            for i in 0..n {
                g.add_vertex(n + i);
                partition_b.insert(n + i);
            }
            // create edges
            for i in 0..n {
                for j in 0..n {
                    if rng.random::<f64>() < p {
                        g.add_edge(&i, &(n + j)).unwrap();
                    }
                }
            }
            // assign random weights
            for e in g.edges_cloned().collect::<Vec<_>>() {
                g.set_edge_weight(&e, 1000.0 * f64::from(rng.random::<i32>()))
                    .unwrap();
            }

            // compute maximum weight matching
            let mut alg =
                MaximumWeightBipartiteMatching::new(&g, partition_a, partition_b).unwrap();
            let matching = alg.get_matching().unwrap();
            let pot = alg.get_potentials();
            let zero = dashu_float::DBig::ZERO.with_precision(0).value();
            let dec = |x: f64| {
                format!("{x}")
                    .parse::<dashu_float::DBig>()
                    .unwrap()
                    .with_precision(0)
                    .value()
            };

            // assert matching
            let mut degree: HashMap<i32, usize> = g.vertices().map(|v| (*v, 0)).collect();
            for e in matching.edges() {
                let (s, t) = g.edge_endpoints(e).unwrap();
                *degree.get_mut(s).unwrap() += 1;
                *degree.get_mut(t).unwrap() += 1;
            }
            for v in g.vertices() {
                assert!(degree[v] <= 1);
            }
            // assert non-negative potentials
            for v in g.vertices() {
                assert!(pot[v] >= zero);
            }
            // assert non-negative reduced cost for edges
            for e in g.edges() {
                let (s, t) = g.edge_endpoints(e).unwrap();
                let w = dec(g.edge_weight(e).unwrap());
                assert!(w <= &pot[s] + &pot[t]);
            }
            // assert tight edges in matching
            for e in matching.edges() {
                let (s, t) = g.edge_endpoints(e).unwrap();
                let w = dec(g.edge_weight(e).unwrap());
                assert!(w == &pot[s] + &pot[t]);
            }
            // assert free nodes have zero potential
            for v in g.vertices() {
                if degree[v] == 0 {
                    assert!(pot[v] == zero);
                }
            }
        }
    }

    #[test]
    fn test_random_instances_fixed_seed() {
        test_random_instance(&mut StdRng::seed_from_u64(17), 100, 0.7, 2);
    }

    #[test]
    #[ignore = "slow (n = 1000)"]
    fn test_random_instances() {
        let mut rng = StdRng::seed_from_u64(23);
        test_random_instance(&mut rng, 100, 0.8, 1);
        test_random_instance(&mut rng, 1000, 0.8, 1);
    }
}

// ---- KuhnMunkresMinimalWeightBipartitePerfectMatchingTest -------------------------------

mod kuhn_munkres_minimal_weight_bipartite_perfect_matching_test {
    use super::*;

    type Vertex = (u8, u8);

    fn matching(weights: &[&[f64]]) -> Matching<Vertex, DefaultEdge> {
        matching_matrix(weights.iter().map(|row| row.to_vec()).collect())
    }

    fn matching_matrix(weights: Vec<Vec<f64>>) -> Matching<Vertex, DefaultEdge> {
        let n = weights.len();
        let first: Vec<_> = (0..n).map(|i| (0, i as u8)).collect();
        let second: Vec<_> = (0..n).map(|i| (1, i as u8)).collect();
        let mut graph = UGraph::<Vertex>::simple().weighted();
        SimpleWeightedBipartiteGraphMatrixGenerator::new()
            .first(first.clone())
            .second(second.clone())
            .weights(weights)
            .generate_into(&mut graph)
            .unwrap();
        KuhnMunkresMinimalWeightBipartitePerfectMatching::new(
            &graph,
            first.into_iter().collect(),
            second.into_iter().collect(),
        )
        .get_matching()
        .unwrap()
    }

    fn repeated_rows<const N: usize>(rows: &[([f64; N], f64)]) -> Vec<Vec<f64>> {
        rows.iter()
            .map(|(prefix, fill)| {
                let mut row = prefix.to_vec();
                row.resize(rows.len(), *fill);
                row
            })
            .collect()
    }

    #[test]
    fn test_for_empty_graph() {
        let graph = UGraph::<Vertex>::simple().weighted();
        let mut algorithm = KuhnMunkresMinimalWeightBipartitePerfectMatching::new(
            &graph,
            IndexSet::new(),
            IndexSet::new(),
        );
        assert!(algorithm.get_matching().unwrap().is_empty());
    }

    #[test]
    fn test_3x3_simple_assignment_task() {
        assert_eq!(
            matching(&[&[1., 2., 3.], &[5., 4., 6.], &[8., 9., 7.]]).weight(),
            12.
        );
    }

    #[test]
    fn test_3x3_simple_assignment_task_no_2() {
        assert_eq!(
            matching(&[&[1., 2., 3.], &[4., 5., 6.], &[7., 8., 9.]]).weight(),
            15.
        );
    }

    #[test]
    fn test_5x5_assignment_task() {
        assert_eq!(
            matching(&[
                &[1., 2., 3., 4., 5.],
                &[6., 7., 8., 7., 2.],
                &[1., 3., 4., 4., 5.],
                &[3., 6., 2., 8., 7.],
                &[4., 1., 3., 5., 4.],
            ])
            .weight(),
            10.
        );
    }

    #[test]
    fn test_5x5_inverted_assignment_task() {
        assert_eq!(
            matching(&[
                &[7., 6., 5., 4., 3.],
                &[2., 1., 0., 1., 6.],
                &[7., 5., 4., 4., 3.],
                &[5., 2., 6., 0., 1.],
                &[4., 7., 5., 3., 4.],
            ])
            .weight(),
            12.
        );
    }

    #[test]
    fn test_6x6_degenerated_assignment_task() {
        assert_eq!(
            matching(&[
                &[7., 6., 5., 4., 3., 9.],
                &[2., 1., 0., 1., 6., 9.],
                &[7., 5., 4., 4., 3., 9.],
                &[5., 2., 6., 0., 1., 9.],
                &[4., 7., 5., 3., 4., 9.],
                &[9., 9., 9., 9., 9., 9.],
            ])
            .weight(),
            21.
        );
    }

    #[test]
    fn test_6x6_degenerated_assignment_task_no_2() {
        assert_eq!(
            matching(&[
                &[7., 6., 5., 4., 3., 9.],
                &[2., 1., 0., 1., 6., 9.],
                &[7., 5., 4., 4., 3., 9.],
                &[5., 2., 6., 0., 1., 9.],
                &[4., 7., 5., 3., 4., 9.],
                &[3., 5., 8., 7., 1., 9.],
            ])
            .weight(),
            19.
        );
    }

    #[test]
    fn test_5x5_degenerated_assignment_task() {
        assert_eq!(
            matching(&[
                &[9., 6., 5., 9., 3.],
                &[2., 1., 0., 1., 6.],
                &[7., 5., 9., 4., 3.],
                &[9., 2., 6., 0., 1.],
                &[4., 9., 5., 9., 4.],
            ])
            .weight(),
            12.
        );
    }

    #[test]
    fn test_8x8_bulky_assignment_task() {
        let max = 25_965_896.;
        matching(&[
            &[233_160., 1_485_901., 3_245_737., max, max, max, max, max],
            &[238_594., max, max, max, max, max, max, max],
            &[242_403., max, max, max, max, max, max, max],
            &[233_408., max, max, max, max, max, max, max],
            &[233_160., max, max, max, max, max, max, max],
            &[258_074., max, max, max, max, max, max, max],
            &[233_160., max, max, max, max, max, max, max],
            &[233_625., max, max, max, max, max, max, max],
        ]);
    }

    #[test]
    fn test_21x21_bulky_assignment_task() {
        matching_matrix(repeated_rows(&[
            ([284169900., 16680., 27111., 0., 25914.], 13400124.),
            ([284169900., 16680., 27305., 0., 25914.], 13400124.),
            ([284169900., 16834., 60173., 0., 25981.], 13400124.),
            ([284169900., 16680., 43679., 0., 32979.], 13400124.),
            ([16656., 270745874., 270739560., 270769776., 270686589.], 0.),
            (
                [0., 271120238., 271113924., 271144140., 271060953.],
                374364.,
            ),
            (
                [0., 270959812., 270953498., 270983714., 270900527.],
                213938.,
            ),
            ([284182260., 0., 33241., 12360., 0.], 13412484.),
            ([284169900., 16680., 27305., 0., 25914.], 13400124.),
            ([284169900., 17630., 25747., 0., 31348.], 13400124.),
            ([284169900., 34668., 28398., 0., 35157.], 13400124.),
            ([284169900., 17503., 20151., 0., 0.], 13400124.),
            ([284169900., 18099., 77279., 0., 26162.], 13400124.),
            ([284169900., 16804., 27869., 0., 25914.], 13400124.),
            ([284175472., 6700., 0., 5572., 24108.], 13405696.),
            ([284169900., 31543., 22343., 0., 50828.], 13400124.),
            ([284169900., 16623., 27215., 0., 25830.], 13400124.),
            ([284169900., 16680., 27305., 0., 14463.], 13400124.),
            ([284174196., 0., 4296., 4296., 30210.], 13404420.),
            ([284169900., 17377., 27999., 0., 25914.], 13400124.),
            ([284169900., 76034., 27305., 0., 26379.], 13400124.),
        ]));
    }

    #[test]
    fn test_20x20_bulky_assignment_task() {
        // The upstream test is named 20x20 but contains the same 21 rows and columns generated
        // by its source data; preserve that input exactly.
        matching_matrix(repeated_rows(&[
            (
                [284309466., 162348., 179093., 121766., 230867., 175501.],
                284309466.,
            ),
            (
                [284309466., 162348., 179287., 121766., 230867., 133304.],
                284309466.,
            ),
            (
                [284309466., 162502., 212155., 121766., 230934., 192658.],
                284309466.,
            ),
            (
                [284309466., 162348., 195661., 121766., 237932., 175347.],
                284309466.,
            ),
            (
                [
                    13538546., 284309466., 284309466., 284309466., 284309466., 284309466.,
                ],
                284309466.,
            ),
            (
                [
                    13147526., 284309466., 284309466., 284309466., 284309466., 284309466.,
                ],
                284309466.,
            ),
            (
                [
                    13307952., 284309466., 284309466., 284309466., 284309466., 284309466.,
                ],
                284309466.,
            ),
            (
                [284309466., 133308., 172863., 121766., 192593., 145039.],
                284309466.,
            ),
            (
                [284309466., 162348., 179287., 121766., 230867., 175287.],
                284309466.,
            ),
            (
                [284309466., 163298., 177729., 121766., 236301., 184972.],
                284309466.,
            ),
            (
                [284309466., 180336., 180380., 121766., 240110., 181736.],
                284309466.,
            ),
            (
                [284309466., 163171., 172133., 121766., 204953., 175649.],
                284309466.,
            ),
            (
                [284309466., 163767., 229261., 121766., 231115., 205427.],
                284309466.,
            ),
            (
                [284309466., 162472., 179851., 121766., 230867., 221341.],
                284309466.,
            ),
            (
                [284309466., 146796., 146410., 121766., 223489., 156487.],
                284309466.,
            ),
            (
                [284309466., 177211., 174325., 121766., 255781., 235876.],
                284309466.,
            ),
            (
                [284309466., 162291., 179197., 121766., 230783., 175287.],
                284309466.,
            ),
            (
                [284309466., 162348., 179287., 121766., 219416., 175460.],
                284309466.,
            ),
            (
                [284309466., 141372., 151982., 121766., 230867., 161161.],
                284309466.,
            ),
            (
                [284309466., 163045., 179981., 121766., 230867., 175740.],
                284309466.,
            ),
            (
                [284309466., 221702., 179287., 121766., 231332., 175287.],
                284309466.,
            ),
        ]));
    }
}

fn edmonds_graph(vertex_count: usize, edges: &[(i32, i32)]) -> UG {
    let mut graph = UG::simple().with_vertex_factory(|i| i as i32);
    graph.add_vertices((0..vertex_count).map(|v| v as i32));
    add_edges(&mut graph, edges);
    graph
}

fn edmonds_pseudograph(vertex_count: usize, edges: &[(i32, i32)]) -> UG {
    let mut graph = UG::pseudo().with_vertex_factory(|i| i as i32);
    graph.add_vertices((0..vertex_count).map(|v| v as i32));
    add_edges(&mut graph, edges);
    graph
}

fn edmonds_large_graph(edges: &[(i32, i32)]) -> UG {
    let mut graph = integer_graph();
    for &(u, v) in edges {
        graph.add_vertex(u);
        graph.add_vertex(v);
        graph.add_edge(&u, &v).unwrap();
    }
    graph
}

fn dense_solution(graph: &UG, cardinality: Option<usize>) {
    let mut matcher = DenseEdmondsMaximumCardinalityMatching::new(graph).unwrap();
    let matching = matcher.get_matching().unwrap();
    if let Some(cardinality) = cardinality {
        verify_matching(graph, &matching, cardinality);
    }
    assert!(matcher.is_maximum_matching(&matching).unwrap());
}

fn sparse_solution(graph: &UG, cardinality: Option<usize>) {
    let mut matcher = SparseEdmondsMaximumCardinalityMatching::new(graph).unwrap();
    let matching = matcher.get_matching().unwrap();
    if let Some(cardinality) = cardinality {
        verify_matching(graph, &matching, cardinality);
    }
    let cover = matcher.get_odd_set_cover().unwrap().clone();
    assert!(
        SparseEdmondsMaximumCardinalityMatching::is_optimal_matching(
            graph,
            matching.edges(),
            &cover
        )
    );
}

const DISCONNECTED: &[(i32, i32)] = &[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (5, 6), (3, 6)];
const PSEUDO: &[(i32, i32)] = &[(0, 1), (1, 2), (2, 3), (3, 0), (3, 3), (2, 3)];
const GRAPH15: &[(i32, i32)] = &[(5, 1), (4, 3), (0, 6), (4, 2), (2, 1), (3, 6), (5, 0)];
const GRAPH14: &[(i32, i32)] = &[
    (2, 0),
    (2, 6),
    (4, 6),
    (4, 3),
    (6, 7),
    (3, 6),
    (5, 0),
    (2, 5),
    (3, 7),
    (2, 4),
];
const GRAPH13: &[(i32, i32)] = &[(0, 3), (0, 2), (4, 2), (0, 1), (1, 3)];
const GRAPH12: &[(i32, i32)] = &[(3, 2), (3, 1), (0, 3), (0, 1), (2, 1)];
const GRAPH11: &[(i32, i32)] = &[(0, 2), (1, 0), (2, 1), (0, 3), (2, 3)];

// ---- DenseEdmondsMaximumCardinalityMatchingTest ------------------------------------------

mod dense_edmonds_maximum_cardinality_matching_test {
    use super::*;

    #[test]
    fn test_disconnected_graph() {
        dense_solution(&edmonds_graph(7, DISCONNECTED), Some(3));
    }

    #[test]
    fn test_pseudo_graph() {
        let graph = edmonds_pseudograph(7, PSEUDO);
        assert_eq!(graph.edge_count(), 6);
        dense_solution(&graph, Some(2));
    }

    #[test]
    fn test_graph15() {
        dense_solution(&edmonds_graph(8, GRAPH15), None);
    }

    #[test]
    fn test_graph14() {
        dense_solution(&edmonds_graph(8, GRAPH14), None);
    }

    #[test]
    fn test_graph13() {
        dense_solution(&edmonds_graph(5, GRAPH13), None);
    }

    #[test]
    fn test_graph12() {
        dense_solution(&edmonds_graph(4, GRAPH12), None);
    }

    #[test]
    fn test_graph11() {
        dense_solution(&edmonds_graph(4, GRAPH11), None);
    }

    #[test]
    fn test_is_maximum_matching4() {
        let mut graph = UG::simple();
        graph.add_vertices(1..=4);
        add_edges(&mut graph, &[(1, 2), (2, 3), (3, 4), (4, 1)]);
        let edges: IndexSet<_> = [
            graph.get_edge(&1, &2).unwrap(),
            graph.get_edge(&3, &4).unwrap(),
        ]
        .into_iter()
        .copied()
        .collect();
        let maximum = Matching::new(&graph, edges, 2.);
        assert!(
            DenseEdmondsMaximumCardinalityMatching::new(&graph)
                .unwrap()
                .is_maximum_matching(&maximum)
                .unwrap()
        );

        graph.add_vertex(5);
        graph.add_edge(&2, &5).unwrap();
        assert!(
            DenseEdmondsMaximumCardinalityMatching::new(&graph)
                .unwrap()
                .is_maximum_matching(&maximum)
                .unwrap()
        );

        let edges: IndexSet<_> = [*graph.get_edge(&1, &2).unwrap()].into_iter().collect();
        let nonmaximum = Matching::new(&graph, edges, 2.);
        assert!(
            !DenseEdmondsMaximumCardinalityMatching::new(&graph)
                .unwrap()
                .is_maximum_matching(&nonmaximum)
                .unwrap()
        );
    }

    #[test]
    fn test_is_maximum_matching3() {
        dense_solution(
            &edmonds_graph(7, &[(4, 0), (2, 3), (2, 0), (2, 5), (2, 6), (0, 1)]),
            None,
        );
    }

    #[test]
    fn test_is_maximum_matching2() {
        let mut graph = UG::simple();
        graph.add_vertices([0, 1, 2, 3, 4, 5, 7, 8, 9]);
        add_edges(
            &mut graph,
            &[
                (0, 8),
                (9, 7),
                (5, 3),
                (9, 4),
                (3, 2),
                (5, 4),
                (1, 0),
                (3, 8),
                (4, 7),
                (2, 0),
                (8, 5),
                (0, 5),
                (8, 1),
            ],
        );
        let edges = [(0, 1), (2, 3), (4, 9), (5, 8)]
            .into_iter()
            .map(|(u, v)| *graph.get_edge(&u, &v).unwrap())
            .collect();
        let matching = Matching::new(&graph, edges, 4.);
        verify_matching(&graph, &matching, 4);
        assert!(
            DenseEdmondsMaximumCardinalityMatching::new(&graph)
                .unwrap()
                .is_maximum_matching(&matching)
                .unwrap()
        );
    }

    #[test]
    fn test_is_maximum1() {
        dense_solution(
            &edmonds_graph(7, &[(5, 6), (1, 2), (0, 6), (4, 6), (2, 6)]),
            None,
        );
    }

    #[test]
    fn test_random_graphs_large() {
        let mut random = StdRng::seed_from_u64(1);
        let vertices = 100;
        for _ in 0..100 {
            let edges = random.random_range(0..max_edges(vertices) / 2);
            let mut generator = GnmRandomGraphGenerator::new(
                vertices,
                edges,
                StdRng::seed_from_u64(0),
                false,
                false,
            )
            .unwrap();
            let mut graph = integer_graph();
            generator.generate_default(&mut graph).unwrap();
            let mut matcher = DenseEdmondsMaximumCardinalityMatching::new(&graph).unwrap();
            let matching = matcher.get_matching().unwrap();
            verify_matching(&graph, &matching, matching.len());
            assert!(matcher.is_maximum_matching(&matching).unwrap());
        }
    }

    #[test]
    fn test_random_graphs_small() {
        for n in 4..12 {
            for m in 5..max_edges(n) {
                let mut generator =
                    GnmRandomGraphGenerator::new(n, m, StdRng::seed_from_u64(17), false, false)
                        .unwrap();
                for _ in 0..25 {
                    let mut graph = integer_graph();
                    generator.generate_default(&mut graph).unwrap();
                    dense_solution(&graph, None);
                }
            }
        }
    }

    macro_rules! large_test {
        ($name:ident, $data:ident, $cardinality:expr, $perfect:expr) => {
            #[test]
            fn $name() {
                let graph = edmonds_large_graph(crate::matching_data::$data);
                let mut matcher = DenseEdmondsMaximumCardinalityMatching::new(&graph).unwrap();
                let matching = matcher.get_matching().unwrap();
                verify_matching(&graph, &matching, $cardinality);
                assert!(matcher.is_maximum_matching(&matching).unwrap());
                if $perfect {
                    assert!(matching.is_perfect());
                }
            }
        };
    }

    large_test!(test_graph1, GRAPH1, 58, false);
    large_test!(test_graph2, GRAPH2, 100, true);
    large_test!(test_graph3, GRAPH3, 100, false);
    large_test!(test_graph4, GRAPH4, 99, false);
    large_test!(test_graph5, GRAPH5, 98, false);
    large_test!(test_graph6, GRAPH6, 96, false);
    large_test!(test_graph7, GRAPH7, 91, false);
    large_test!(test_graph8, GRAPH8, 86, false);
    large_test!(test_graph9, GRAPH9, 75, false);
    large_test!(test_graph10, GRAPH10, 66, false);
}

// ---- SparseEdmondsMaximumCardinalityMatchingTest -----------------------------------------

mod sparse_edmonds_maximum_cardinality_matching_test {
    use super::*;

    #[test]
    fn test_disconnected_graph() {
        sparse_solution(&edmonds_graph(7, DISCONNECTED), Some(3));
    }

    #[test]
    fn test_pseudo_graph() {
        let graph = edmonds_pseudograph(7, PSEUDO);
        assert_eq!(graph.edge_count(), 6);
        sparse_solution(&graph, Some(2));
    }

    #[test]
    fn test_graph15() {
        sparse_solution(&edmonds_graph(8, GRAPH15), Some(3));
    }

    #[test]
    fn test_graph14() {
        sparse_solution(&edmonds_graph(8, GRAPH14), None);
    }

    #[test]
    fn test_graph13() {
        sparse_solution(&edmonds_graph(5, GRAPH13), None);
    }

    #[test]
    fn test_graph12() {
        sparse_solution(&edmonds_graph(4, GRAPH12), None);
    }

    #[test]
    fn test_graph11() {
        sparse_solution(&edmonds_graph(4, GRAPH11), None);
    }

    #[test]
    fn test_is_maximum_matching3() {
        sparse_solution(
            &edmonds_graph(7, &[(4, 0), (2, 3), (2, 0), (2, 5), (2, 6), (0, 1)]),
            None,
        );
    }

    #[test]
    fn test_is_maximum_matching2() {
        let mut graph = UG::simple();
        graph.add_vertices([0, 1, 2, 3, 4, 5, 7, 8, 9]);
        add_edges(
            &mut graph,
            &[
                (0, 8),
                (9, 7),
                (5, 3),
                (9, 4),
                (3, 2),
                (5, 4),
                (1, 0),
                (3, 8),
                (4, 7),
                (2, 0),
                (8, 5),
                (0, 5),
                (8, 1),
            ],
        );
        let edges = [(0, 1), (2, 3), (4, 9), (5, 8)]
            .into_iter()
            .map(|(u, v)| *graph.get_edge(&u, &v).unwrap())
            .collect();
        verify_matching(&graph, &Matching::new(&graph, edges, 4.), 4);
    }

    #[test]
    fn test_is_maximum1() {
        sparse_solution(
            &edmonds_graph(7, &[(5, 6), (1, 2), (0, 6), (4, 6), (2, 6)]),
            None,
        );
    }

    #[test]
    fn test_random_graphs_large() {
        let mut random = StdRng::seed_from_u64(1);
        let vertices = 100;
        for _ in 0..100 {
            let edges = random.random_range(0..max_edges(vertices) / 2);
            let mut generator = GnmRandomGraphGenerator::new(
                vertices,
                edges,
                StdRng::seed_from_u64(0),
                false,
                false,
            )
            .unwrap();
            let mut graph = integer_graph();
            generator.generate_default(&mut graph).unwrap();
            sparse_solution(&graph, None);
        }
    }

    fn barabasi(seed: i64) {
        let mut generator =
            BarabasiAlbertGraphGenerator::new(6, 6, 250, StdRng::seed_from_u64(seed as u64))
                .unwrap();
        let mut graph = integer_graph();
        generator.generate_default(&mut graph).unwrap();
        sparse_solution(&graph, None);
    }

    #[test]
    fn test_random_graphs_barabasi_large() {
        for seed in 1324..1334 {
            barabasi(seed);
        }
    }

    #[test]
    fn test_random_graphs_barabasi_large_no_seed() {
        for seed in 0..10 {
            barabasi(seed);
        }
    }

    #[test]
    fn test_random_graphs_small() {
        for n in 4..12 {
            for m in 5..max_edges(n) {
                let mut generator =
                    GnmRandomGraphGenerator::new(n, m, StdRng::seed_from_u64(19), false, false)
                        .unwrap();
                for _ in 0..25 {
                    let mut graph = integer_graph();
                    generator.generate_default(&mut graph).unwrap();
                    sparse_solution(&graph, None);
                }
            }
        }
    }

    macro_rules! large_test {
        ($name:ident, $data:ident, $cardinality:expr) => {
            #[test]
            fn $name() {
                let graph = edmonds_large_graph(crate::matching_data::$data);
                sparse_solution(&graph, Some($cardinality));
            }
        };
    }

    large_test!(test_graph1, GRAPH1, 58);
    large_test!(test_graph2, GRAPH2, 100);
    large_test!(test_graph3, GRAPH3, 100);
    large_test!(test_graph4, GRAPH4, 99);
    large_test!(test_graph5, GRAPH5, 98);
    large_test!(test_graph6, GRAPH6, 96);
    large_test!(test_graph7, GRAPH7, 91);
    large_test!(test_graph8, GRAPH8, 86);
    large_test!(test_graph9, GRAPH9, 75);
    large_test!(test_graph10, GRAPH10, 66);
}

mod blossom_v_public_tests {
    use super::*;
    use crate::blossom_data::{
        GENERAL_WEIGHTED_CASES, PERFECT_FAILURE_CASES, PERFECT_WEIGHTED_CASES, WeightedCase,
    };
    use jgrapht::alg::matching::blossom::{
        BlossomVOptions, KolmogorovWeightedMatching, KolmogorovWeightedPerfectMatching,
        ObjectiveSense,
    };

    type WeightedEdges = &'static [(i32, i32, f64)];

    fn weighted_graph(edges: WeightedEdges) -> UG {
        let mut graph = UG::simple().weighted();
        for &(source, target, weight) in edges {
            graph.add_vertex(source);
            graph.add_vertex(target);
            graph.add_weighted_edge(&source, &target, weight).unwrap();
        }
        graph
    }

    fn weighted_graph_for_case(case: &WeightedCase) -> UG {
        let mut graph = if case.pseudograph {
            UG::pseudo().weighted()
        } else {
            UG::simple().weighted()
        };
        for &(source, target, weight) in case.edges {
            graph.add_vertex(source);
            graph.add_vertex(target);
            graph.add_weighted_edge(&source, &target, weight).unwrap();
        }
        graph
    }

    fn check_perfect_graph(graph: &UG, minimum: f64, maximum: f64) {
        for options in BlossomVOptions::all_options() {
            for (objective, expected) in [
                (ObjectiveSense::Minimize, minimum),
                (ObjectiveSense::Maximize, maximum),
            ] {
                let mut algorithm = KolmogorovWeightedPerfectMatching::with_options_and_objective(
                    graph, options, objective,
                )
                .unwrap();
                let matching = algorithm.get_matching().unwrap();
                assert!(matching.is_perfect());
                assert_eq!(matching.weight(), expected);
                assert!(algorithm.test_optimality().unwrap());
            }
        }
    }

    fn check_perfect(edges: WeightedEdges, minimum: f64, maximum: f64) {
        let graph = weighted_graph(edges);
        check_perfect_graph(&graph, minimum, maximum);
    }

    fn check_general(edges: WeightedEdges, minimum: f64, maximum: f64) {
        let graph = weighted_graph(edges);
        for options in BlossomVOptions::all_options() {
            for (objective, expected) in [
                (ObjectiveSense::Minimize, minimum),
                (ObjectiveSense::Maximize, maximum),
            ] {
                let mut algorithm = KolmogorovWeightedMatching::with_options_and_objective(
                    &graph, options, objective,
                )
                .unwrap();
                assert_eq!(algorithm.get_matching().unwrap().weight(), expected);
                assert!(algorithm.test_optimality().unwrap());
            }
        }
    }

    #[test]
    fn kolmogorov_weighted_perfect_matching_java_cases_0_through_15() {
        let cases: &[(WeightedEdges, f64, f64)] = &[
            (
                &[
                    (0, 1, 8.),
                    (0, 2, 10.),
                    (1, 2, 8.),
                    (0, 3, 11.),
                    (1, 3, 5.),
                    (2, 5, 3.),
                    (1, 5, 6.),
                    (2, 4, 3.),
                    (4, 5, 1.),
                    (1, 6, 5.),
                    (3, 6, 4.),
                    (3, 7, 5.),
                    (6, 7, 2.),
                    (5, 7, 6.),
                    (4, 7, 7.),
                    (1, 7, 5.),
                ],
                18.,
                27.,
            ),
            (&[(1, 2, 5.)], 5., 5.),
            (
                &[(1, 2, 8.), (2, 3, 8.), (3, 4, 8.), (4, 1, 8.), (2, 4, 2.)],
                16.,
                16.,
            ),
            (&[(1, 3, 11.), (1, 4, 8.), (2, 3, 8.), (2, 4, 2.)], 13., 16.),
            (
                &[
                    (0, 3, 7.),
                    (0, 4, 5.),
                    (0, 5, 2.),
                    (1, 3, 1.),
                    (1, 4, 3.),
                    (1, 5, 4.),
                    (2, 3, 7.),
                    (2, 4, 10.),
                    (2, 5, 7.),
                ],
                12.,
                21.,
            ),
            (
                &[
                    (0, 3, 7.),
                    (0, 4, 3.),
                    (0, 5, 9.),
                    (1, 3, 8.),
                    (1, 4, 2.),
                    (1, 5, 9.),
                    (2, 3, 6.),
                    (2, 4, 1.),
                    (2, 5, 10.),
                ],
                17.,
                21.,
            ),
            (
                &[
                    (0, 1, 2.),
                    (0, 2, 5.),
                    (0, 3, 1.),
                    (1, 2, 5.),
                    (1, 3, 2.),
                    (2, 3, 1.),
                ],
                3.,
                7.,
            ),
            (
                &[
                    (0, 1, 1.),
                    (0, 2, 4.),
                    (0, 3, 7.),
                    (0, 4, 10.),
                    (1, 2, 5.),
                    (1, 3, 7.),
                    (1, 4, 10.),
                    (2, 3, 10.),
                    (2, 4, 2.),
                    (3, 4, 3.),
                    (2, 5, 8.),
                ],
                12.,
                25.,
            ),
            (
                &[
                    (0, 1, 1.),
                    (0, 2, 6.),
                    (0, 3, 1.),
                    (0, 4, 1.),
                    (1, 2, 6.),
                    (1, 3, 6.),
                    (1, 4, 5.),
                    (2, 3, 7.),
                    (2, 4, 8.),
                    (3, 4, 8.),
                    (0, 5, 8.),
                ],
                20.,
                22.,
            ),
            (
                &[
                    (0, 1, 4.),
                    (0, 2, 4.),
                    (0, 3, 6.),
                    (0, 4, 8.),
                    (1, 2, 8.),
                    (1, 3, 10.),
                    (1, 4, 8.),
                    (2, 3, 4.),
                    (2, 4, 9.),
                    (3, 4, 4.),
                    (0, 5, 9.),
                ],
                21.,
                28.,
            ),
            (
                &[
                    (0, 1, 5.),
                    (0, 2, 1.),
                    (0, 3, 8.),
                    (0, 4, 1.),
                    (1, 2, 2.),
                    (1, 3, 8.),
                    (1, 4, 1.),
                    (2, 3, 8.),
                    (2, 4, 5.),
                    (3, 4, 10.),
                    (0, 5, 8.),
                ],
                17.,
                21.,
            ),
            (
                &[
                    (0, 1, 2.),
                    (0, 2, 6.),
                    (0, 3, 3.),
                    (0, 4, 2.),
                    (1, 2, 7.),
                    (1, 3, 7.),
                    (1, 4, 7.),
                    (2, 3, 4.),
                    (2, 4, 4.),
                    (3, 4, 2.),
                    (0, 5, 2.),
                ],
                11.,
                13.,
            ),
            (
                &[
                    (2, 0, 5.),
                    (2, 1, 9.),
                    (3, 0, 2.),
                    (3, 1, 6.),
                    (3, 2, 7.),
                    (4, 0, 3.),
                    (4, 1, 5.),
                    (4, 2, 5.),
                    (4, 3, 7.),
                    (5, 4, 6.),
                ],
                17.,
                17.,
            ),
            (
                &[
                    (1, 0, 9.),
                    (2, 0, 8.),
                    (2, 1, 3.),
                    (3, 0, 5.),
                    (3, 1, 4.),
                    (3, 2, 10.),
                    (4, 0, 3.),
                    (4, 1, 2.),
                    (4, 2, 4.),
                    (4, 3, 8.),
                    (5, 1, 4.),
                ],
                13.,
                20.,
            ),
            (
                &[
                    (1, 0, 8.),
                    (2, 0, 7.),
                    (2, 1, 10.),
                    (3, 0, 8.),
                    (3, 1, 5.),
                    (3, 2, 3.),
                    (4, 0, 9.),
                    (4, 1, 5.),
                    (4, 2, 4.),
                    (4, 3, 10.),
                    (5, 1, 4.),
                ],
                16.,
                21.,
            ),
        ];
        for &(edges, minimum, maximum) in cases {
            check_perfect(edges, minimum, maximum);
        }

        let empty = UG::simple().weighted();
        for options in BlossomVOptions::all_options() {
            let mut algorithm =
                KolmogorovWeightedPerfectMatching::with_options(&empty, options).unwrap();
            assert_eq!(algorithm.get_matching().unwrap().weight(), 0.0);
            assert!(algorithm.test_optimality().unwrap());
        }
    }

    #[test]
    fn kolmogorov_weighted_matching_java_cases_1_through_5() {
        let cases: &[(WeightedEdges, f64, f64)] = &[
            (&[(0, 1, -1.), (1, 2, 1.), (2, 3, -1.), (3, 0, 1.)], -2., 2.),
            (
                &[
                    (0, 1, 3.),
                    (1, 2, 2.),
                    (2, 3, 1.),
                    (3, 0, 5.),
                    (0, 2, 15.),
                    (1, 3, -15.),
                ],
                -15.,
                15.,
            ),
            (&[(2, 0, -2.), (1, 2, 2.), (0, 1, 3.)], -2., 3.),
            (
                &[
                    (9, 4, 4.),
                    (7, 9, 4.),
                    (4, 7, -4.),
                    (6, 4, -2.),
                    (9, 6, 2.),
                    (8, 9, -2.),
                    (6, 8, 1.),
                    (4, 5, 3.),
                    (6, 3, 3.),
                    (1, 6, 6.),
                    (3, 4, 1.),
                    (4, 2, -5.),
                    (3, 2, 5.),
                    (0, 2, -1.),
                    (8, 1, 7.),
                    (0, 1, -6.),
                    (3, 0, -5.),
                    (1, 3, -3.),
                    (5, 7, -2.),
                    (2, 5, 4.),
                    (7, 2, -6.),
                ],
                -16.,
                19.,
            ),
            (
                &[
                    (9, 5, 4.),
                    (8, 9, -4.),
                    (8, 5, -4.),
                    (5, 7, 4.),
                    (6, 7, 2.),
                    (5, 6, -4.),
                    (8, 4, 4.),
                    (3, 8, -4.),
                    (5, 2, 3.),
                    (7, 9, 2.),
                    (5, 0, -5.),
                    (4, 5, -3.),
                    (4, 0, 5.),
                    (1, 0, -5.),
                    (2, 6, 1.),
                    (0, 2, -5.),
                    (4, 1, 2.),
                    (3, 4, -1.),
                    (1, 3, 2.),
                    (0, 6, 6.),
                ],
                -14.,
                17.,
            ),
        ];
        for &(edges, minimum, maximum) in cases {
            check_general(edges, minimum, maximum);
        }
    }

    #[test]
    fn kolmogorov_weighted_perfect_matching_java_cases_16_through_40() {
        for case in PERFECT_WEIGHTED_CASES {
            let graph = weighted_graph_for_case(case);
            check_perfect_graph(&graph, case.minimum, case.maximum);
        }

        let mut complete = UG::simple().weighted();
        complete.add_vertices(0..20);
        for source in 0..20 {
            for target in source + 1..20 {
                complete.add_weighted_edge(&source, &target, 1.0).unwrap();
            }
        }
        check_perfect_graph(&complete, 10.0, 10.0);
    }

    #[test]
    fn kolmogorov_weighted_matching_java_cases_6_through_19() {
        for case in GENERAL_WEIGHTED_CASES {
            assert!(
                !case.pseudograph,
                "unexpected pseudograph in case {}",
                case.id
            );
            check_general(case.edges, case.minimum, case.maximum);
        }
    }

    #[test]
    fn kolmogorov_weighted_perfect_matching_java_failure_cases_41_through_50() {
        for case in PERFECT_FAILURE_CASES {
            let mut graph = weighted_graph(case.edges);
            graph.add_vertices(case.extra_vertices.iter().copied());
            for options in BlossomVOptions::all_options() {
                let result = KolmogorovWeightedPerfectMatching::with_options(&graph, options)
                    .and_then(|mut algorithm| algorithm.get_matching().map(|_| ()));
                assert!(
                    result.is_err(),
                    "upstream case {} unexpectedly succeeded",
                    case.id
                );
            }
        }
    }
}
