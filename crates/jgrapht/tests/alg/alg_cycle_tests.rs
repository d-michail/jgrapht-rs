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

//! Ports of the `org.jgrapht.alg.cycle` tests: the Ahuja–Orlin–Sharma cyclic exchange, the simple
//! cycle enumerations (DirectedSimpleCyclesTest, JohnsonSimpleCyclesTest, HawickJamesSimpleCyclesTest),
//! CycleDetectorTest and CyclesTest.

use jgrapht::alg::cyclic_exchange::AhujaOrlinSharmaCyclicExchangeLocalAugmentation;
use jgrapht::*;
use std::collections::HashMap;

mod chinese_postman_test {
    use super::*;
    use jgrapht::alg::cycle::ChinesePostman;

    fn undirected(
        graph_type: GraphType,
        vertices: &[i32],
        edges: &[(i32, i32, f64)],
    ) -> UGraph<i32> {
        let mut graph = UGraph::<i32>::with_graph_type(graph_type).weighted();
        graph.add_vertices(vertices.iter().copied());
        for &(source, target, weight) in edges {
            graph
                .add_weighted_edge(&source, &target, weight)
                .unwrap();
        }
        graph
    }

    fn directed(
        graph_type: GraphType,
        vertices: &[i32],
        edges: &[(i32, i32, f64)],
    ) -> DiGraph<i32> {
        let mut graph = DiGraph::<i32>::with_graph_type(graph_type).weighted();
        graph.add_vertices(vertices.iter().copied());
        for &(source, target, weight) in edges {
            graph
                .add_weighted_edge(&source, &target, weight)
                .unwrap();
        }
        graph
    }

    fn verify<G: IncidenceGraph>(graph: &G, expected_weight: f64, expected_length: usize)
    where
        G::Vertex: std::fmt::Debug,
    {
        let path = ChinesePostman::new().get_cpp_solution(graph).unwrap();
        assert_eq!(path.length(), expected_length);
        assert_eq!(path.edge_list().len(), expected_length);
        assert!((path.weight() - expected_weight).abs() < 1e-8);
        assert!(
            (path
                .edge_list()
                .iter()
                .map(|edge| graph.edge_weight(edge).unwrap_or(1.0))
                .sum::<f64>()
                - expected_weight)
                .abs()
                < 1e-8
        );
        assert!(graph.edges().all(|edge| path.edge_list().contains(edge)));
        assert_eq!(path.start_vertex(), path.end_vertex());
        assert_eq!(path.vertex_list().len(), path.edge_list().len() + 1);
        for index in 0..path.edge_list().len() {
            let source = &path.vertex_list()[index];
            let target = &path.vertex_list()[index + 1];
            let edge = &path.edge_list()[index];
            if graph.graph_type().directed {
                assert_eq!(graph.edge_source(edge), Some(source));
                assert_eq!(graph.edge_target(edge), Some(target));
            } else {
                assert_eq!(graph.opposite_vertex(edge, source), Some(target));
            }
        }
    }

    #[test]
    fn empty_single_loop_and_parallel_edge_graphs() {
        let empty = DiGraph::<i32>::simple();
        assert!(ChinesePostman::new().get_cpp_solution(&empty).is_err());

        verify(
            &undirected(GraphType::simple(), &[0, 1], &[(0, 1, 10.)]),
            20.,
            2,
        );
        verify(
            &undirected(
                GraphType::pseudograph(),
                &[0, 1],
                &[(0, 1, 10.), (0, 0, 20.)],
            ),
            40.,
            3,
        );
        verify(
            &undirected(
                GraphType::multigraph(),
                &[1, 2, 3, 4],
                &[
                    (1, 2, 1.),
                    (1, 4, 3.),
                    (2, 3, 20.),
                    (2, 3, 10.),
                    (3, 4, 2.),
                ],
            ),
            42.,
            8,
        );
    }

    #[test]
    fn undirected_weighted_graphs() {
        verify(
            &undirected(
                GraphType::simple(),
                &(1..=8).collect::<Vec<_>>(),
                &[
                    (1, 2, 50.), (1, 3, 50.), (1, 4, 50.), (2, 4, 50.), (2, 5, 70.),
                    (2, 6, 50.), (3, 4, 70.), (3, 7, 70.), (3, 8, 120.), (4, 6, 60.),
                    (5, 6, 70.), (6, 8, 60.), (7, 8, 70.),
                ],
            ),
            1000.,
            16,
        );
        verify(
            &undirected(
                GraphType::simple(),
                &[1, 2, 3, 4, 5],
                &[
                    (1, 2, 8.), (1, 3, 5.), (1, 4, 6.), (2, 3, 5.), (2, 5, 6.),
                    (3, 4, 5.), (3, 5, 5.), (4, 5, 8.),
                ],
            ),
            60.,
            10,
        );
        verify(
            &undirected(
                GraphType::simple(),
                &[1, 2, 3, 4, 5, 6, 7],
                &[
                    (1, 2, 5.), (1, 4, 4.), (1, 5, 1.), (2, 3, 3.), (2, 5, 1.),
                    (2, 7, 1.), (3, 4, 2.), (3, 5, 3.), (3, 6, 1.), (3, 7, 2.),
                    (4, 5, 1.), (6, 7, 3.),
                ],
            ),
            31.,
            15,
        );
        verify(
            &undirected(
                GraphType::simple(),
                &(1..=10).collect::<Vec<_>>(),
                &[
                    (1, 2, 100.), (1, 3, 150.), (1, 4, 200.), (2, 3, 120.),
                    (2, 5, 250.), (3, 6, 200.), (4, 5, 100.), (4, 7, 80.),
                    (4, 8, 160.), (5, 6, 100.), (5, 7, 100.), (6, 7, 150.),
                    (6, 10, 160.), (7, 9, 100.), (8, 9, 40.), (9, 10, 80.),
                ],
            ),
            2590.,
            20,
        );
        verify(
            &undirected(
                GraphType::simple(),
                &[1, 2, 3, 4, 5, 6],
                &[(1, 2, 1.), (2, 3, 1.), (3, 4, 1.), (2, 5, 1.), (3, 6, 1.)],
            ),
            10.,
            10,
        );
    }

    #[test]
    fn directed_weighted_graphs() {
        verify(
            &directed(
                GraphType::pseudograph(),
                &[1, 2, 3, 4],
                &[
                    (1, 2, 1.), (2, 3, 3.), (2, 3, 20.), (3, 4, 10.), (4, 4, 5.),
                    (4, 1, 2.),
                ],
            ),
            54.,
            9,
        );
        verify(
            &directed(
                GraphType::simple(),
                &[1, 2, 3, 4],
                &[(1, 2, 1.), (1, 4, 5.), (2, 3, 2.), (3, 1, 3.), (4, 3, 4.)],
            ),
            18.,
            6,
        );
        verify(
            &directed(
                GraphType::simple(),
                &[1, 2, 3, 4],
                &[
                    (1, 2, 8.), (1, 3, 3.), (2, 3, 10.), (2, 4, 5.), (3, 4, 15.),
                    (4, 1, 4.),
                ],
            ),
            76.,
            10,
        );
        verify(
            &directed(
                GraphType::multigraph(),
                &[1, 2, 3, 4],
                &[
                    (1, 2, 21.), (2, 3, 8.), (3, 1, 5.), (3, 4, 20.), (4, 2, 12.),
                    (4, 2, 2.),
                ],
            ),
            104.,
            9,
        );
        verify(
            &directed(
                GraphType::simple(),
                &[1, 2, 3, 4, 5, 6],
                &[
                    (1, 2, 10.), (1, 3, 20.), (2, 4, 50.), (2, 5, 10.), (3, 4, 20.),
                    (3, 5, 33.), (4, 5, 5.), (4, 6, 12.), (5, 1, 12.), (5, 6, 1.),
                    (6, 3, 22.),
                ],
            ),
            276.,
            17,
        );
    }

    #[test]
    fn directed_unweighted_graph() {
        verify(
            &directed(
                GraphType::simple(),
                &(1..=11).collect::<Vec<_>>(),
                &[
                    (1, 2, 1.), (1, 11, 1.), (2, 3, 1.), (3, 4, 1.), (3, 5, 1.),
                    (4, 9, 1.), (5, 7, 1.), (6, 9, 1.), (7, 6, 1.), (8, 7, 1.),
                    (9, 8, 1.), (9, 10, 1.), (10, 1, 1.), (11, 9, 1.),
                ],
            ),
            22.,
            22,
        );
    }

    #[test]
    fn two_vertex_unweighted_smoke() {
        verify(
            &undirected(GraphType::simple(), &[0, 1], &[(0, 1, 1.)]),
            2.,
            2,
        );
    }
}

mod ahuja_orlin_sharma_cyclic_exchange {
    use super::*;

    type G = DiGraph<i32>;

    fn labels(l: &[(i32, i64)]) -> HashMap<i32, i64> {
        l.iter().copied().collect()
    }

    fn weighted_graph(n: i32) -> G {
        let mut g = G::with_loops().weighted();
        g.add_vertices(0..n);
        g
    }

    fn edge(g: &mut G, a: i32, b: i32, w: f64) {
        let e = g.add_edge(&a, &b).unwrap().unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }

    fn cycle_of(g: &G, bound: usize, labels: &HashMap<i32, i64>, best: bool) -> (Vec<i32>, f64) {
        let c = AhujaOrlinSharmaCyclicExchangeLocalAugmentation::new(g, bound, labels, best)
            .unwrap()
            .local_augmentation_cycle();
        (c.vertex_list().to_vec(), c.weight())
    }

    /// `generateImprovementGraphForPartitionProblem`.
    fn partition_problem(labels: &HashMap<i32, i64>, initial_weight: f64) -> G {
        let mut keys: Vec<i32> = labels.keys().copied().collect();
        keys.sort_unstable();
        let mut g = G::with_loops().weighted();
        g.add_vertices(keys.iter().copied());
        for &v1 in &keys {
            for &v2 in &keys {
                if labels[&v1] != labels[&v2] {
                    g.add_edge(&v1, &v2).unwrap();
                    let e = *g.get_edge(&v1, &v2).unwrap();
                    g.set_edge_weight(&e, initial_weight).unwrap();
                    g.add_edge(&v2, &v1).unwrap();
                    g.set_edge_weight(&e, initial_weight).unwrap();
                }
            }
        }
        g
    }

    fn set_weight(g: &mut G, a: i32, b: i32, w: f64) {
        let e = *g.get_edge(&a, &b).unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }

    #[test]
    fn test_improvement_graph_1() {
        let l: HashMap<i32, i64> = (0..3)
            .flat_map(|i| [(i, 1), (i + 3, 2), (i + 6, 3)])
            .collect();
        let mut g = partition_problem(&l, 1.0);
        set_weight(&mut g, 0, 4, -5.0);
        set_weight(&mut g, 4, 8, -5.0);
        let (c, w) = cycle_of(&g, 2, &l, false);
        assert_eq!(c, vec![0, 4, 0]);
        assert!((w + 4.0).abs() < 1e-7);
    }

    #[test]
    fn test_improvement_graph_2() {
        let l: HashMap<i32, i64> = (0..5)
            .flat_map(|i| [(i, 1), (i + 5, 2), (i + 10, 3), (i + 15, 4), (i + 20, 5)])
            .collect();
        let mut g = partition_problem(&l, 4.9);
        for (a, b) in [(0, 5), (5, 10), (10, 15), (15, 20), (20, 0)] {
            set_weight(&mut g, a, b, -1.0);
        }
        let (c, w) = cycle_of(&g, 5, &l, false);
        assert_eq!(c, vec![0, 5, 10, 15, 20, 0]);
        assert!((w + 5.0).abs() < 1e-7);
    }

    #[test]
    fn test_improvement_graph_3() {
        let l: HashMap<i32, i64> = (0..5)
            .flat_map(|i| [(i, 1), (i + 5, 2), (i + 10, 3), (i + 15, 4), (i + 20, 5)])
            .collect();
        let mut g = partition_problem(&l, 3.9);
        for (a, b) in [(0, 5), (5, 10), (10, 15), (15, 20)] {
            set_weight(&mut g, a, b, -1.0);
        }
        let (c, w) = cycle_of(&g, 5, &l, false);
        assert_eq!(c, vec![0, 5, 10, 15, 20, 0]);
        assert!((w - (3.9 - 4.0)).abs() < 1e-7);
    }

    #[test]
    fn test_improvement_graph_4() {
        let l = labels(&[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
        let mut g = weighted_graph(6);
        for i in 0..6 {
            edge(&mut g, i, (i + 1) % 6, 1.0);
        }
        edge(&mut g, 1, 4, -3.0);
        set_weight(&mut g, 3, 4, -6.0);
        let (c1, w1) = cycle_of(&g, 4, &l, false);
        assert!(w1.abs() < 1e-7);
        assert_eq!(c1, vec![1, 4, 5, 0, 1]);
        let (c2, w2) = cycle_of(&g, 6, &l, false);
        assert_eq!(c2, vec![3, 4, 5, 0, 1, 2, 3]);
        assert!((w2 + 1.0).abs() < 1e-7);
    }

    #[test]
    fn test_improvement_graph_5() {
        let l = labels(&[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
        let mut g = weighted_graph(6);
        for i in 0..6 {
            edge(&mut g, i, i, -1.0);
        }
        for i in 0..6 {
            edge(&mut g, i, (i + 1) % 6, 1.0);
        }
        edge(&mut g, 1, 4, 1.0);
        set_weight(&mut g, 1, 4, -3.0);
        set_weight(&mut g, 3, 4, -6.0);
        let c = AhujaOrlinSharmaCyclicExchangeLocalAugmentation::new(&g, 6, &l, false)
            .unwrap()
            .local_augmentation_cycle();
        assert!((c.weight() + 2.0).abs() < 1e-7);
        assert_eq!(c.vertex_list(), &[0, 0]);
        assert_eq!(c.start_vertex(), Some(&0));
        assert_eq!(c.end_vertex(), Some(&0));
    }

    fn graph_6_7_8(n: i32, edges: &[(i32, i32, f64)]) -> G {
        let mut g = weighted_graph(n);
        for &(a, b, w) in edges {
            edge(&mut g, a, b, w);
        }
        g
    }

    #[test]
    fn test_improvement_graph_6() {
        let l = labels(&[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 1),
            (4, 2),
            (5, 1),
            (6, 2),
            (7, 1),
            (8, 2),
            (9, 3),
            (10, 4),
        ]);
        let g = graph_6_7_8(
            11,
            &[
                (0, 1, -2.0),
                (0, 3, -1.0),
                (0, 5, -3.0),
                (0, 7, -2.0),
                (1, 2, 1.0),
                (1, 4, 0.0),
                (1, 6, 1.0),
                (1, 8, 0.0),
                (2, 9, 0.0),
                (3, 2, 0.0),
                (3, 4, 1.0),
                (3, 6, 1.0),
                (3, 8, 0.0),
                (4, 9, 0.0),
                (5, 2, 0.0),
                (5, 4, 1.0),
                (5, 6, 1.0),
                (5, 8, 1.0),
                (6, 9, 0.0),
                (7, 2, 0.0),
                (7, 4, 1.0),
                (7, 6, 0.0),
                (7, 8, 0.0),
                (8, 9, 0.0),
                (9, 10, 0.0),
                (10, 0, 0.0),
            ],
        );
        let (c, w) = cycle_of(&g, 6, &l, false);
        assert_eq!(c, vec![0, 5, 2, 9, 10, 0]);
        assert!((w + 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_improvement_graph_7() {
        let l = labels(&[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 1),
            (4, 2),
            (5, 1),
            (6, 1),
            (7, 3),
            (8, 4),
        ]);
        let g = graph_6_7_8(
            9,
            &[
                (0, 1, -2.0),
                (0, 3, -1.0),
                (0, 5, -3.0),
                (0, 6, -3.0),
                (1, 2, 1.0),
                (1, 4, 0.0),
                (2, 7, 0.0),
                (3, 2, 0.0),
                (3, 4, 1.0),
                (4, 7, 0.0),
                (5, 2, 0.0),
                (5, 4, 1.0),
                (5, 7, 1.0),
                (6, 2, 0.0),
                (6, 4, 1.0),
                (6, 7, 0.0),
                (7, 8, 0.0),
                (8, 0, 0.0),
            ],
        );
        let (c, w) = cycle_of(&g, 6, &l, false);
        assert_eq!(c, vec![0, 6, 7, 8, 0]);
        assert!((w + 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_improvement_graph_8() {
        let l = labels(&[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 1),
            (4, 2),
            (5, 1),
            (6, 1),
            (7, 3),
            (8, 4),
        ]);
        let g = graph_6_7_8(
            9,
            &[
                (0, 1, -4.0),
                (0, 3, -4.0),
                (0, 5, -3.0),
                (0, 6, -4.0),
                (1, 2, 1.0),
                (1, 4, 1.0),
                (2, 7, 0.0),
                (3, 2, 1.0),
                (3, 4, 0.0),
                (4, 7, 0.0),
                (5, 7, 0.0),
                (6, 7, 0.0),
                (7, 8, 0.0),
                (8, 0, 0.0),
            ],
        );
        let (c, w) = cycle_of(&g, 6, &l, false);
        assert_eq!(c, vec![0, 6, 7, 8, 0]);
        assert!((w + 4.0).abs() < 1e-8);
    }

    #[test]
    fn test_improvement_graph_9() {
        let l = labels(&[(0, 0), (1, 0), (2, 0)]);
        let g = graph_6_7_8(3, &[(0, 1, -4.0), (1, 2, 1.0)]);
        let (c, w) = cycle_of(&g, 6, &l, true);
        assert!(c.is_empty());
        assert_eq!(w, f64::MAX);
    }

    #[test]
    fn test_improvement_graph_10() {
        let l = labels(&(0..7).map(|i| (i, i as i64)).collect::<Vec<_>>());
        let mut g = weighted_graph(7);
        for i in 0..7 {
            for j in 0..7 {
                let w = if i == j {
                    1.0
                } else if (i + j) % 2 == 0 {
                    (i + j) as f64
                } else if j == i + 1 || (i == 6 && j == 1) {
                    (-i - j) as f64
                } else {
                    (-i - j + 1) as f64
                };
                edge(&mut g, i, j, w);
            }
        }
        let (c, w) = cycle_of(&g, 8, &l, true);
        assert_eq!(c, vec![1, 2, 3, 4, 5, 6, 1]);
        assert!((w + 42.0).abs() < 1e-8);
    }

    #[test]
    fn test_improvement_graph_11() {
        let l = labels(&(0..6).map(|i| (i, i as i64)).collect::<Vec<_>>());
        let mut g = weighted_graph(6);
        for i in 0..6 {
            edge(&mut g, i, i, -((i * i) as f64));
        }
        let (c, w) = cycle_of(&g, 4, &l, true);
        assert_eq!(c, vec![5, 5]);
        assert!((w + 50.0).abs() < 1e-8);
    }
}

// DirectedSimpleCyclesTest

mod directed_simple_cycles_test {
    use super::*;
    use jgrapht::alg::cycle::*;

    const MAX_SIZE: usize = 9;
    const RESULTS: [usize; 10] = [0, 1, 3, 8, 24, 89, 415, 2372, 16072, 125673];

    type G = DiGraph<i32>;

    fn graph(n: i32, weighted: bool) -> G {
        let mut g = G::with_loops();
        if weighted {
            g = g.weighted();
        }
        for i in 0..n {
            g.add_vertex(i);
        }
        g
    }

    fn count<A: for<'a> Fn(&'a G) -> Box<dyn DirectedSimpleCycles<i32> + 'a>>(
        alg: A,
        weighted: bool,
    ) {
        let mut graph_ = graph(7, weighted);
        let mut steps: Vec<(Vec<(i32, i32)>, usize)> = vec![
            (vec![(0, 0)], 1),
            (vec![(1, 1)], 2),
            (vec![(0, 1), (1, 0)], 3),
            (vec![(1, 2), (2, 3), (3, 0)], 4),
            (vec![(6, 6)], 5),
        ];
        for (edges, expected) in steps.drain(..) {
            for (a, b) in edges {
                graph_.add_edge(&a, &b).unwrap();
            }
            assert_eq!(expected, alg(&graph_).find_simple_cycles().unwrap().len());
        }
        for size in 1..=MAX_SIZE {
            let mut g = graph(size as i32, weighted);
            for i in 0..size as i32 {
                for j in 0..size as i32 {
                    g.add_edge(&i, &j).unwrap();
                }
            }
            assert_eq!(
                RESULTS[size],
                alg(&g).find_simple_cycles().unwrap().len(),
                "size {size}"
            );
        }
    }

    macro_rules! all_algorithms {
        ($weighted:expr) => {
            count(|g| Box::new(TiernanSimpleCycles::with_graph(g)), $weighted);
            count(|g| Box::new(TarjanSimpleCycles::with_graph(g)), $weighted);
            count(
                |g| Box::new(JohnsonSimpleCycles::new(g).unwrap()),
                $weighted,
            );
            count(
                |g| Box::new(SzwarcfiterLauerSimpleCycles::with_graph(g)),
                $weighted,
            );
            count(
                |g| Box::new(HawickJamesSimpleCycles::with_graph(g)),
                $weighted,
            );
        };
    }

    #[test]
    fn test() {
        all_algorithms!(false);
    }

    #[test]
    fn test_with_weighted_graph() {
        all_algorithms!(true);
    }

    #[test]
    fn missing_graph_is_an_error() {
        assert!(
            TiernanSimpleCycles::<G>::new()
                .find_simple_cycles()
                .is_err()
        );
        assert!(
            HawickJamesSimpleCycles::<G>::new()
                .count_simple_cycles()
                .is_err()
        );
    }
}

// JohnsonSimpleCyclesTest

mod johnson_simple_cycles_test {
    use super::*;
    use jgrapht::alg::cycle::*;

    #[test]
    fn small_example() {
        let mut g = DiGraph::<i32>::with_loops();
        g.add_vertices(1..=6);
        for (a, b) in [(1, 2), (2, 3), (2, 5), (3, 4), (4, 5), (5, 6), (6, 1)] {
            g.add_edge(&a, &b).unwrap();
        }
        let cycles = JohnsonSimpleCycles::new(&g)
            .unwrap()
            .find_simple_cycles()
            .unwrap();
        assert_eq!(vec![vec![1, 2, 3, 4, 5, 6], vec![1, 2, 5, 6]], cycles);
    }

    fn check(mut graph: DiGraph<String>) {
        for v in ["A", "B", "C", "D"] {
            graph.add_vertex(v.to_string());
        }
        for (a, b) in [("A", "B"), ("B", "A"), ("B", "C"), ("C", "B"), ("C", "A")] {
            graph.add_edge(&a.to_string(), &b.to_string()).unwrap();
        }
        let cycles = JohnsonSimpleCycles::new(&graph)
            .unwrap()
            .find_simple_cycles()
            .unwrap();
        let expected: Vec<Vec<String>> = [vec!["A", "B"], vec!["A", "B", "C"], vec!["B", "C"]]
            .iter()
            .map(|c| c.iter().map(|s| s.to_string()).collect())
            .collect();
        assert_eq!(expected, cycles);
        assert_eq!(4, graph.vertex_count());
        assert_eq!(5, graph.edge_count());
    }

    #[test]
    fn distinct_vertex_instances() {
        check(DiGraph::<String>::simple());
    }

    #[test]
    fn distinct_vertex_instances_with_weighted_edges() {
        check(DiGraph::<String>::simple().weighted());
    }

    #[test]
    fn multiple_edges_are_rejected() {
        let mut g = DiGraph::<i32>::multi();
        g.add_vertices(0..2);
        g.add_edge(&0, &1).unwrap();
        g.add_edge(&0, &1).unwrap();
        assert!(JohnsonSimpleCycles::new(&g).is_err());
    }
}

// HawickJamesSimpleCyclesTest

mod hawick_james_simple_cycles_test {
    use super::*;
    use jgrapht::alg::cycle::*;

    type G = DiGraph<String>;

    fn graph(vertices: &[&str], edges: &[(&str, &str)]) -> G {
        let mut g = G::with_loops();
        for v in vertices {
            g.add_vertex(v.to_string());
        }
        for (a, b) in edges {
            g.add_edge(&a.to_string(), &b.to_string()).unwrap();
        }
        g
    }

    fn cycles(g: &G) -> Vec<Vec<String>> {
        HawickJamesSimpleCycles::with_graph(g)
            .find_simple_cycles()
            .unwrap()
    }

    fn contains_all(cycle: &[String], items: &[&str]) -> bool {
        items.iter().all(|i| cycle.iter().any(|c| c == i))
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_cycles_count() {
        let g = graph(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        assert_eq!(
            0,
            HawickJamesSimpleCycles::with_graph(&g)
                .count_simple_cycles()
                .unwrap()
        );
    }

    #[test]
    fn reflexive_cycle_count() {
        let g = graph(&["A"], &[("A", "A")]);
        assert_eq!(
            1,
            HawickJamesSimpleCycles::with_graph(&g)
                .count_simple_cycles()
                .unwrap()
        );
    }

    #[test]
    fn single_direct_cycle_count() {
        let g = graph(&["A", "B"], &[("A", "B"), ("B", "A")]);
        assert_eq!(
            1,
            HawickJamesSimpleCycles::with_graph(&g)
                .count_simple_cycles()
                .unwrap()
        );
    }

    #[test]
    fn indirect_cycle_count() {
        let g = graph(&["A", "B", "C"], &[("A", "B"), ("B", "C"), ("C", "A")]);
        assert_eq!(
            1,
            HawickJamesSimpleCycles::with_graph(&g)
                .count_simple_cycles()
                .unwrap()
        );
    }

    #[test]
    fn no_cycles_find() {
        let g = graph(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        assert!(cycles(&g).is_empty());
    }

    #[test]
    fn reflexive_cycle_find() {
        let g = graph(&["A"], &[("A", "A")]);
        assert_eq!(vec![strings(&["A"])], cycles(&g));
    }

    #[test]
    fn single_direct_find() {
        let g = graph(&["A", "B"], &[("A", "B"), ("B", "A")]);
        let c = cycles(&g);
        assert_eq!(1, c.len());
        assert!(contains_all(&c[0], &["A", "B"]));
    }

    #[test]
    fn indirect_cycle_find() {
        let g = graph(&["A", "B", "C"], &[("A", "B"), ("B", "C"), ("C", "A")]);
        let c = cycles(&g);
        assert_eq!(1, c.len());
        assert!(contains_all(&c[0], &["A", "B", "C"]));
    }

    #[test]
    fn two_cycles() {
        let g = graph(
            &["A", "B", "C"],
            &[("A", "B"), ("B", "A"), ("B", "C"), ("C", "A")],
        );
        let c = cycles(&g);
        assert_eq!(2, c.len());
        assert!(contains_all(&c[0], &["A", "B"]));
        assert!(contains_all(&c[1], &["A", "B", "C"]));
    }

    #[test]
    fn two_sharing_edge() {
        let g = graph(
            &["A", "B", "C", "D"],
            &[("B", "C"), ("A", "B"), ("C", "A"), ("D", "B"), ("C", "D")],
        );
        let c = cycles(&g);
        assert_eq!(2, c.len());
        assert!(contains_all(&c[0], &["A", "B", "C"]));
        assert!(contains_all(&c[1], &["D", "B", "C"]));
    }

    #[test]
    fn simplest_cycles() {
        let g = graph(
            &["A", "B"],
            &[("A", "B"), ("B", "A"), ("A", "A"), ("B", "B")],
        );
        assert_eq!(
            vec![strings(&["A", "B"]), strings(&["A"]), strings(&["B"])],
            cycles(&g)
        );
    }

    #[test]
    fn complex_graph() {
        let g = graph(
            &["A", "B", "C", "D", "E", "F"],
            &[
                ("A", "B"),
                ("B", "C"),
                ("B", "E"),
                ("C", "D"),
                ("D", "E"),
                ("E", "F"),
                ("F", "A"),
            ],
        );
        let c = cycles(&g);
        assert_eq!(2, c.len());
        assert!(contains_all(&c[0], &["A", "B", "C", "D", "E", "F"]));
        assert!(contains_all(&c[1], &["A", "B", "E", "F"]));
    }

    #[test]
    fn consecutive_runs() {
        let g = graph(&["A", "B", "C"], &[("A", "B"), ("B", "C"), ("C", "A")]);
        let mut hjsc = HawickJamesSimpleCycles::with_graph(&g);
        for _ in 0..2 {
            let run = hjsc.find_simple_cycles().unwrap();
            assert_eq!(1, run.len());
            assert!(contains_all(&run[0], &["A", "B", "C"]));
        }
    }

    #[test]
    fn limit_paths1() {
        let g = graph(&["A", "B"], &[("A", "B"), ("B", "A")]);
        let mut hjsc = HawickJamesSimpleCycles::with_graph(&g);
        hjsc.set_path_limit(1);
        assert!(hjsc.find_simple_cycles().unwrap().is_empty());
    }

    #[test]
    fn limit_paths2() {
        let g = graph(&["A", "B", "C"], &[("A", "B"), ("B", "C"), ("C", "A")]);
        let mut hjsc = HawickJamesSimpleCycles::with_graph(&g);
        hjsc.set_path_limit(2);
        assert!(hjsc.find_simple_cycles().unwrap().is_empty());
    }

    #[test]
    fn limit_paths_two_cycles() {
        let g = graph(
            &["A", "B", "C", "D"],
            &[("A", "B"), ("B", "A"), ("C", "D"), ("D", "C")],
        );
        let mut hjsc = HawickJamesSimpleCycles::with_graph(&g);
        hjsc.set_path_limit(2);
        let c = hjsc.find_simple_cycles().unwrap();
        assert_eq!(2, c.len());
        assert!(contains_all(&c[0], &["A", "B"]));
        assert!(contains_all(&c[1], &["C", "D"]));
    }

    #[test]
    fn order() {
        let g = graph(
            &["0", "1", "2", "3", "4", "5"],
            &[
                ("0", "1"),
                ("1", "2"),
                ("2", "3"),
                ("3", "0"),
                ("1", "4"),
                ("4", "5"),
                ("5", "2"),
            ],
        );
        let c = cycles(&g);
        assert_eq!(2, c.len());
        assert_eq!("0,1,2,3", c[0].join(","));
        assert_eq!("0,1,4,5,2,3", c[1].join(","));
    }

    #[test]
    fn consumer_api() {
        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        struct SampleVertex {
            name: &'static str,
            allowed_in_cycles: bool,
        }
        let v = |name, allowed_in_cycles| SampleVertex {
            name,
            allowed_in_cycles,
        };
        let (a, b, c) = (v("A", true), v("B", true), v("C", false));
        let mut graph = DiGraph::<SampleVertex>::with_loops();
        for x in [&a, &b, &c] {
            graph.add_vertex(x.clone());
        }
        for (x, y) in [(&a, &b), (&b, &a), (&b, &c), (&c, &a)] {
            graph.add_edge(x, y).unwrap();
        }
        let mut invalid = Vec::new();
        HawickJamesSimpleCycles::with_graph(&graph)
            .find_simple_cycles_with(&mut |cycle| {
                if cycle.iter().any(|x| !x.allowed_in_cycles) {
                    invalid.push(cycle);
                }
            })
            .unwrap();
        assert_eq!(vec![vec![a, b, c]], invalid);
    }
}

// CycleDetectorTest

mod cycle_detector_test {
    use super::*;
    use jgrapht::alg::cycle::CycleDetector;
    use std::collections::HashSet;

    type G = DiGraph<String>;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn create_graph(g: &mut G) {
        for v in ["v1", "v2", "v3", "v4", "v5", "v6", "v7"] {
            g.add_vertex(s(v));
        }
        for (a, b) in [
            ("v1", "v2"),
            ("v2", "v3"),
            ("v3", "v4"),
            ("v4", "v1"),
            ("v4", "v5"),
            ("v5", "v6"),
            ("v1", "v6"),
            ("v7", "v1"),
        ] {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|x| s(x)).collect()
    }

    fn run_test(g: &G, cyclic: HashSet<String>, acyclic: HashSet<String>) {
        let detector = CycleDetector::new(g);
        assert_eq!(!cyclic.is_empty(), detector.detect_cycles());
        assert_eq!(
            cyclic,
            detector.find_cycles().into_iter().collect::<HashSet<_>>()
        );
        for v in &cyclic {
            assert!(detector.detect_cycles_containing_vertex(v));
            assert_eq!(
                cyclic,
                detector
                    .find_cycles_containing_vertex(v)
                    .into_iter()
                    .collect::<HashSet<_>>()
            );
        }
        for v in &acyclic {
            assert!(!detector.detect_cycles_containing_vertex(v));
            assert!(detector.find_cycles_containing_vertex(v).is_empty());
        }
    }

    #[test]
    fn directed_with_cycle() {
        let mut g = G::with_loops();
        create_graph(&mut g);
        run_test(&g, set(&["v1", "v2", "v3", "v4"]), set(&["v5", "v6", "v7"]));
    }

    #[test]
    fn directed_with_doubled_cycle() {
        let mut g = G::with_loops();
        for v in ["v2", "v1", "v3"] {
            g.add_vertex(s(v));
        }
        for (a, b) in [("v1", "v2"), ("v2", "v3"), ("v3", "v1"), ("v2", "v1")] {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        run_test(&g, set(&["v1", "v2", "v3"]), HashSet::new());
    }

    #[test]
    fn directed_without_cycle() {
        let mut g = G::with_loops();
        create_graph(&mut g);
        g.remove_vertex(&s("v2"));
        let acyclic: HashSet<String> = g.vertices().cloned().collect();
        run_test(&g, HashSet::new(), acyclic);
    }

    #[test]
    fn vertex_equals() {
        let mut graph = G::with_loops();
        graph.add_vertex(s("A"));
        graph.add_vertex(s("B"));
        graph.add_edge(&s("A"), &s("B")).unwrap();
        graph.add_edge(&s("B"), &s("A")).unwrap();
        assert_eq!(2, graph.edge_count());
        assert_eq!(2, graph.vertex_count());
        let detector = CycleDetector::new(&graph);
        let a = s("A");
        assert_eq!(2, detector.find_cycles().len());
        assert!(detector.detect_cycles_containing_vertex(&a));
        assert_eq!(2, detector.find_cycles_containing_vertex(&a).len());
    }
}

// CyclesTest

mod cycles_test {
    use super::*;
    use jgrapht::alg::cycle::simple_cycle_to_graph_path;
    use jgrapht::graph::GraphExt;
    use std::collections::HashSet;

    fn add<G: GraphMut<Vertex = i32>>(g: &mut G, a: i32, b: i32) -> G::Edge {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_edge(&a, &b).unwrap().unwrap()
    }

    fn assert_undirected_cycle<G: Graph<Vertex = i32>>(g: &G, edges: &[G::Edge]) {
        if edges.is_empty() {
            return;
        }
        let mut dup = HashSet::new();
        let touching = |a: &G::Edge, b: &G::Edge| {
            let (sa, ta) = g.edge_endpoints(a).unwrap();
            let (sb, tb) = g.edge_endpoints(b).unwrap();
            sa == sb || sa == tb || ta == sb || ta == tb
        };
        for (i, cur) in edges.iter().enumerate() {
            assert!(dup.insert(cur.clone()));
            if i > 0 {
                assert!(touching(cur, &edges[i - 1]));
            }
        }
        if edges.len() > 1 {
            assert!(touching(&edges[0], edges.last().unwrap()));
        }
    }

    #[test]
    fn undirected1() {
        let mut graph = UGraph::<i32>::simple();
        let cycle = vec![
            add(&mut graph, 0, 1),
            add(&mut graph, 1, 2),
            add(&mut graph, 2, 0),
        ];
        let path = simple_cycle_to_graph_path(&graph, &cycle).unwrap().unwrap();
        assert_eq!(path.start_vertex(), path.end_vertex());
        assert_undirected_cycle(&graph, path.edge_list());
    }

    #[test]
    fn undirected2() {
        let mut graph = UGraph::<i32>::simple();
        let cycle = vec![
            add(&mut graph, 1, 2),
            add(&mut graph, 3, 4),
            add(&mut graph, 0, 1),
            add(&mut graph, 4, 5),
            add(&mut graph, 5, 0),
            add(&mut graph, 2, 3),
        ];
        add(&mut graph, 5, 6);
        add(&mut graph, 6, 7);
        let path = simple_cycle_to_graph_path(&graph, &cycle).unwrap().unwrap();
        assert_eq!(path.start_vertex(), path.end_vertex());
        assert_undirected_cycle(&graph, path.edge_list());
    }

    #[test]
    fn self_loop1() {
        let mut graph = UGraph::<i32>::pseudo();
        let cycle = vec![add(&mut graph, 0, 0)];
        let path = simple_cycle_to_graph_path(&graph, &cycle).unwrap().unwrap();
        assert_eq!(path.start_vertex(), path.end_vertex());
        assert_undirected_cycle(&graph, path.edge_list());
    }

    #[test]
    fn directed1() {
        let mut graph = DiGraph::<i32>::pseudo();
        let cycle = vec![
            add(&mut graph, 0, 1),
            add(&mut graph, 3, 2),
            add(&mut graph, 3, 4),
            add(&mut graph, 1, 2),
            add(&mut graph, 0, 4),
        ];
        let path = simple_cycle_to_graph_path(&graph, &cycle).unwrap().unwrap();
        assert_eq!(path.start_vertex(), path.end_vertex());
        assert_undirected_cycle(&graph, path.edge_list());
    }

    #[test]
    fn undirected_not_simple1() {
        let mut graph = UGraph::<i32>::simple();
        let cycle = vec![
            add(&mut graph, 0, 1),
            add(&mut graph, 1, 2),
            add(&mut graph, 2, 0),
            add(&mut graph, 2, 3),
            add(&mut graph, 3, 4),
            add(&mut graph, 4, 2),
        ];
        assert!(simple_cycle_to_graph_path(&graph, &cycle).is_err());
    }
}

// QueueBFSFundamentalCycleBasisTest, StackBFSFundamentalCycleBasisTest, PatonCycleBaseTest

mod cycle_basis_support {
    use jgrapht::graph::GraphExt;
    use jgrapht::*;

    pub fn add_ev<G: GraphMut<Vertex = i32>>(g: &mut G, a: i32, b: i32) -> G::Edge {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_edge(&a, &b).unwrap().unwrap()
    }

    pub fn add_ev_w<G: GraphMut<Vertex = i32>>(g: &mut G, a: i32, b: i32, w: f64) -> G::Edge {
        let e = add_ev(g, a, b);
        g.set_edge_weight(&e, w).unwrap();
        e
    }

    pub fn add_w<G: GraphMut<Vertex = i32>>(g: &mut G, a: i32, b: i32, w: f64) {
        let e = g.add_edge(&a, &b).unwrap().unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }

    pub fn ge<G: Graph<Vertex = i32>>(g: &G, a: i32, b: i32) -> G::Edge {
        g.get_edge(&a, &b).unwrap().clone()
    }

    /// `assertCycle`: the edges form a closed chain (a closed walk for directed graphs).
    pub fn assert_cycle<G: Graph<Vertex = i32>>(g: &G, edges: &[G::Edge]) {
        if edges.is_empty() {
            return;
        }
        let directed = g.graph_type().directed;
        let mut dup = std::collections::HashSet::new();
        let ends = |e: &G::Edge| {
            let (s, t) = g.edge_endpoints(e).unwrap();
            (*s, *t)
        };
        for (i, cur) in edges.iter().enumerate() {
            assert!(dup.insert(cur.clone()));
            if i > 0 {
                let (cs, ct) = ends(cur);
                let (ps, pt) = ends(&edges[i - 1]);
                if directed {
                    assert!(cs == pt);
                } else {
                    assert!(cs == ps || cs == pt || ct == ps || ct == pt);
                }
            }
        }
        if edges.len() > 1 {
            let (fs, ft) = ends(&edges[0]);
            let (ls, lt) = ends(edges.last().unwrap());
            if directed {
                assert!(fs == lt);
            } else {
                assert!(fs == ls || fs == lt || ft == ls || ft == lt);
            }
        }
    }
}
use cycle_basis_support::*;

mod queue_bfs_fundamental_cycle_basis_test {
    use super::*;
    use jgrapht::alg::cycle::*;

    #[test]
    fn test_simple() {
        let mut graph = UGraph::<i32>::simple();
        add_ev(&mut graph, 0, 1);
        add_ev(&mut graph, 1, 2);
        add_ev(&mut graph, 2, 0);
        let mut cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        assert_eq!(1, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 0, 1)));
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c1.len());
        assert_eq!(3, cb.length());
        assert!((3.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 2, 3);
        add_ev(&mut graph, 3, 0);
        cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(2, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        assert_eq!(6, cb.length());
        assert!((6.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 3, 1);
        cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(3, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        assert_eq!(9, cb.length());
        assert!((9.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 3, 4);
        add_ev(&mut graph, 4, 2);
        cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(4, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 4)));
        assert_eq!(4, c4.len());
        assert_eq!(13, cb.length());
        assert!((13.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 4, 5);
        cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(4, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 4)));
        assert_eq!(4, c4.len());
        assert_eq!(13, cb.length());
        assert!((13.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 5, 2);
        cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(5, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 4)));
        assert_eq!(4, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 4, 5)));
        assert!(c5.contains(&ge(&graph, 2, 4)));
        assert!(c5.contains(&ge(&graph, 2, 5)));
        assert_eq!(3, c5.len());
        assert_eq!(16, cb.length());
        assert!((16.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 5, 6);
        add_ev(&mut graph, 6, 4);
        cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(6, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 4)));
        assert_eq!(4, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 4, 5)));
        assert!(c5.contains(&ge(&graph, 2, 4)));
        assert!(c5.contains(&ge(&graph, 2, 5)));
        assert_eq!(3, c5.len());
        let c6 = cycles[5].clone();
        assert!(c6.contains(&ge(&graph, 5, 6)));
        assert!(c6.contains(&ge(&graph, 5, 2)));
        assert!(c6.contains(&ge(&graph, 2, 4)));
        assert!(c6.contains(&ge(&graph, 4, 6)));
        assert_eq!(4, c6.len());
        assert_eq!(20, cb.length());
        assert!((20.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_multigraphs_with_loops() {
        let mut graph = UGraph::<i32>::pseudo();
        add_ev(&mut graph, 0, 1);
        add_ev(&mut graph, 0, 2);
        add_ev(&mut graph, 0, 3);
        add_ev(&mut graph, 1, 2);
        add_ev(&mut graph, 2, 3);
        add_ev(&mut graph, 1, 4);
        add_ev(&mut graph, 2, 5);
        add_ev(&mut graph, 3, 6);
        add_ev(&mut graph, 4, 5);
        add_ev(&mut graph, 5, 6);
        add_ev(&mut graph, 4, 7);
        add_ev(&mut graph, 5, 8);
        add_ev(&mut graph, 6, 9);
        add_ev(&mut graph, 7, 8);
        let e89_1 = graph.add_edge(&8, &9).unwrap().unwrap();
        add_ev(&mut graph, 7, 9);
        let e89_2 = graph.add_edge(&8, &9).unwrap().unwrap();
        let e89_3 = graph.add_edge(&8, &9).unwrap().unwrap();
        let e89_4 = graph.add_edge(&8, &9).unwrap().unwrap();
        let e77_1 = graph.add_edge(&7, &7).unwrap().unwrap();
        let e77_2 = graph.add_edge(&7, &7).unwrap().unwrap();
        let e77_3 = graph.add_edge(&7, &7).unwrap().unwrap();
        let cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        assert_eq!(13, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 0, 1)));
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 1, 4)));
        assert!(c3.contains(&ge(&graph, 4, 5)));
        assert!(c3.contains(&ge(&graph, 5, 2)));
        assert!(c3.contains(&ge(&graph, 2, 0)));
        assert_eq!(5, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 5)));
        assert!(c4.contains(&ge(&graph, 5, 6)));
        assert!(c4.contains(&ge(&graph, 6, 3)));
        assert!(c4.contains(&ge(&graph, 3, 0)));
        assert_eq!(5, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 0, 1)));
        assert!(c5.contains(&ge(&graph, 1, 4)));
        assert!(c5.contains(&ge(&graph, 4, 7)));
        assert!(c5.contains(&ge(&graph, 7, 8)));
        assert!(c5.contains(&ge(&graph, 8, 5)));
        assert!(c5.contains(&ge(&graph, 5, 2)));
        assert!(c5.contains(&ge(&graph, 2, 0)));
        assert_eq!(7, c5.len());
        let c6 = cycles[5].clone();
        assert!(c6.contains(&ge(&graph, 0, 2)));
        assert!(c6.contains(&ge(&graph, 2, 5)));
        assert!(c6.contains(&ge(&graph, 5, 8)));
        assert!(c6.contains(&e89_1));
        assert!(c6.contains(&ge(&graph, 9, 6)));
        assert!(c6.contains(&ge(&graph, 6, 3)));
        assert!(c6.contains(&ge(&graph, 3, 0)));
        assert_eq!(7, c6.len());
        let c7 = cycles[6].clone();
        assert!(c7.contains(&ge(&graph, 0, 1)));
        assert!(c7.contains(&ge(&graph, 1, 4)));
        assert!(c7.contains(&ge(&graph, 4, 7)));
        assert!(c7.contains(&ge(&graph, 7, 9)));
        assert!(c7.contains(&ge(&graph, 9, 6)));
        assert!(c7.contains(&ge(&graph, 6, 3)));
        assert!(c7.contains(&ge(&graph, 3, 0)));
        assert_eq!(7, c7.len());
        let c8 = cycles[7].clone();
        assert!(c8.contains(&ge(&graph, 0, 2)));
        assert!(c8.contains(&ge(&graph, 2, 5)));
        assert!(c8.contains(&ge(&graph, 5, 8)));
        assert!(c8.contains(&e89_2));
        assert!(c8.contains(&ge(&graph, 9, 6)));
        assert!(c8.contains(&ge(&graph, 6, 3)));
        assert!(c8.contains(&ge(&graph, 3, 0)));
        assert_eq!(7, c8.len());
        let c9 = cycles[8].clone();
        assert!(c9.contains(&ge(&graph, 0, 2)));
        assert!(c9.contains(&ge(&graph, 2, 5)));
        assert!(c9.contains(&ge(&graph, 5, 8)));
        assert!(c9.contains(&e89_3));
        assert!(c9.contains(&ge(&graph, 9, 6)));
        assert!(c9.contains(&ge(&graph, 6, 3)));
        assert!(c9.contains(&ge(&graph, 3, 0)));
        assert_eq!(7, c9.len());
        let c10 = cycles[9].clone();
        assert!(c10.contains(&ge(&graph, 0, 2)));
        assert!(c10.contains(&ge(&graph, 2, 5)));
        assert!(c10.contains(&ge(&graph, 5, 8)));
        assert!(c10.contains(&e89_4));
        assert!(c10.contains(&ge(&graph, 9, 6)));
        assert!(c10.contains(&ge(&graph, 6, 3)));
        assert!(c10.contains(&ge(&graph, 3, 0)));
        assert_eq!(7, c10.len());
        let c11 = cycles[10].clone();
        assert!(c11.contains(&e77_1));
        assert_eq!(1, c11.len());
        let c12 = cycles[11].clone();
        assert!(c12.contains(&e77_2));
        assert_eq!(1, c12.len());
        let c13 = cycles[12].clone();
        assert!(c13.contains(&e77_3));
        assert_eq!(1, c13.len());
        assert_eq!(61, cb.length());
        assert!((61.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_multi_graph_with_multiple_components_with_loops() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_vertex(2);
        graph.add_edge(&0, &1).unwrap();
        graph.add_edge(&0, &2).unwrap();
        let e12_1 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e12_2 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e11_1 = graph.add_edge(&1, &1).unwrap().unwrap();
        let e11_2 = graph.add_edge(&1, &1).unwrap().unwrap();
        graph.add_vertex(3);
        graph.add_vertex(4);
        graph.add_vertex(5);
        graph.add_edge(&3, &4).unwrap();
        graph.add_edge(&3, &5).unwrap();
        let e45_1 = graph.add_edge(&4, &5).unwrap().unwrap();
        let e45_2 = graph.add_edge(&4, &5).unwrap().unwrap();
        let e55_1 = graph.add_edge(&5, &5).unwrap().unwrap();
        let e55_2 = graph.add_edge(&5, &5).unwrap().unwrap();
        let cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        assert_eq!(8, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 0, 1)));
        assert!(c1.contains(&e12_1));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 0, 1)));
        assert!(c2.contains(&e12_2));
        assert!(c2.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&e11_1));
        assert_eq!(1, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&e11_2));
        assert_eq!(1, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 3, 4)));
        assert!(c5.contains(&e45_1));
        assert!(c5.contains(&ge(&graph, 5, 3)));
        assert_eq!(3, c5.len());
        let c6 = cycles[5].clone();
        assert!(c6.contains(&ge(&graph, 3, 4)));
        assert!(c6.contains(&e45_2));
        assert!(c6.contains(&ge(&graph, 5, 3)));
        assert_eq!(3, c6.len());
        let c7 = cycles[6].clone();
        assert!(c7.contains(&e55_1));
        assert_eq!(1, c7.len());
        let c8 = cycles[7].clone();
        assert!(c8.contains(&e55_2));
        assert_eq!(1, c8.len());
        assert_eq!(16, cb.length());
        assert!((16.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_two_parallel_edges() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        let e1 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e2 = graph.add_edge(&0, &1).unwrap().unwrap();
        let cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        assert_eq!(1, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&e1));
        assert!(c1.contains(&e2));
        assert_eq!(2, c1.len());
        assert_eq!(2, cb.length());
        assert!((2.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_more_parallel_edges() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_vertex(2);
        graph.add_vertex(3);
        let e01_1 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e01_2 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e12 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e23_1 = graph.add_edge(&2, &3).unwrap().unwrap();
        let e23_2 = graph.add_edge(&2, &3).unwrap().unwrap();
        let e30 = graph.add_edge(&3, &0).unwrap().unwrap();
        let cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        assert_eq!(3, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&e01_2));
        assert!(c1.contains(&e01_1));
        assert_eq!(2, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&e23_1));
        assert!(c2.contains(&e30));
        assert!(c2.contains(&e01_1));
        assert!(c2.contains(&e12));
        assert_eq!(4, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&e23_2));
        assert!(c3.contains(&e30));
        assert!(c3.contains(&e01_1));
        assert!(c3.contains(&e12));
        assert_eq!(4, c3.len());
        assert_eq!(10, cb.length());
        assert!((10.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_zero_cycle_space_dimension() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_vertex(2);
        graph.add_vertex(3);
        graph.add_edge(&0, &1).unwrap();
        graph.add_edge(&2, &3).unwrap();
        let cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(0, cb.cycles().len());
        assert_eq!(0, cb.length());
        assert!((0.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_empty_graph() {
        let graph = UGraph::<i32>::pseudo();
        let cb = QueueBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(0, cb.cycles().len());
        assert_eq!(0, cb.length());
        assert!((0.0 - cb.weight()).abs() <= 0.0001);
    }
}

mod stack_bfs_fundamental_cycle_basis_test {
    use super::*;
    use jgrapht::alg::cycle::*;

    #[test]
    fn test_simple() {
        let mut graph = UGraph::<i32>::simple();
        add_ev(&mut graph, 0, 1);
        add_ev(&mut graph, 1, 2);
        add_ev(&mut graph, 2, 0);
        let mut cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(1, cb.cycles().len());
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 0, 1)));
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c1.len());
        assert_eq!(3, cb.length());
        assert!((3.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 2, 3);
        add_ev(&mut graph, 3, 0);
        cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(2, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        assert_eq!(6, cb.length());
        assert!((6.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 3, 1);
        cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(3, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        assert_eq!(9, cb.length());
        assert!((9.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 3, 4);
        add_ev(&mut graph, 4, 2);
        cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(4, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 4)));
        assert_eq!(4, c4.len());
        assert_eq!(13, cb.length());
        assert!((13.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 4, 5);
        cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(4, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert!(c4.contains(&ge(&graph, 2, 4)));
        assert_eq!(4, c4.len());
        assert_eq!(13, cb.length());
        assert!((13.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 5, 2);
        cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(5, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 4, 2)));
        assert!(c4.contains(&ge(&graph, 2, 0)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert_eq!(4, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 5, 2)));
        assert!(c5.contains(&ge(&graph, 2, 0)));
        assert!(c5.contains(&ge(&graph, 0, 3)));
        assert!(c5.contains(&ge(&graph, 3, 4)));
        assert!(c5.contains(&ge(&graph, 4, 5)));
        assert_eq!(5, c5.len());
        assert_eq!(18, cb.length());
        assert!((18.0 - cb.weight()).abs() <= 0.0001);
        add_ev(&mut graph, 5, 6);
        add_ev(&mut graph, 6, 4);
        cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        cycles = cb.cycles().iter().cloned().collect();
        assert_eq!(6, cb.cycles().len());
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 1, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert!(c2.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 3)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert!(c3.contains(&ge(&graph, 0, 3)));
        assert_eq!(3, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 4, 2)));
        assert!(c4.contains(&ge(&graph, 2, 0)));
        assert!(c4.contains(&ge(&graph, 0, 3)));
        assert!(c4.contains(&ge(&graph, 3, 4)));
        assert_eq!(4, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 5, 2)));
        assert!(c5.contains(&ge(&graph, 2, 0)));
        assert!(c5.contains(&ge(&graph, 0, 3)));
        assert!(c5.contains(&ge(&graph, 3, 4)));
        assert!(c5.contains(&ge(&graph, 4, 5)));
        assert_eq!(5, c5.len());
        let c6 = cycles[5].clone();
        assert!(c6.contains(&ge(&graph, 5, 6)));
        assert!(c6.contains(&ge(&graph, 6, 4)));
        assert!(c6.contains(&ge(&graph, 4, 5)));
        assert_eq!(3, c6.len());
        assert_eq!(21, cb.length());
        assert!((21.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_multigraphs_with_loops() {
        let mut graph = UGraph::<i32>::pseudo();
        add_ev(&mut graph, 0, 1);
        add_ev(&mut graph, 0, 2);
        add_ev(&mut graph, 0, 3);
        add_ev(&mut graph, 1, 2);
        add_ev(&mut graph, 2, 3);
        add_ev(&mut graph, 1, 4);
        add_ev(&mut graph, 2, 5);
        add_ev(&mut graph, 3, 6);
        add_ev(&mut graph, 4, 5);
        add_ev(&mut graph, 5, 6);
        add_ev(&mut graph, 4, 7);
        add_ev(&mut graph, 5, 8);
        add_ev(&mut graph, 6, 9);
        add_ev(&mut graph, 7, 8);
        let e89_1 = graph.add_edge(&8, &9).unwrap().unwrap();
        add_ev(&mut graph, 7, 9);
        let e89_2 = graph.add_edge(&8, &9).unwrap().unwrap();
        let e89_3 = graph.add_edge(&8, &9).unwrap().unwrap();
        let e89_4 = graph.add_edge(&8, &9).unwrap().unwrap();
        let e77_1 = graph.add_edge(&7, &7).unwrap().unwrap();
        let e77_2 = graph.add_edge(&7, &7).unwrap().unwrap();
        let e77_3 = graph.add_edge(&7, &7).unwrap().unwrap();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(13, cb.cycles().len());
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 1, 2)));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert!(c1.contains(&ge(&graph, 0, 1)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 2, 3)));
        assert!(c2.contains(&ge(&graph, 3, 0)));
        assert!(c2.contains(&ge(&graph, 0, 2)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&ge(&graph, 1, 4)));
        assert!(c3.contains(&ge(&graph, 4, 7)));
        assert!(c3.contains(&ge(&graph, 7, 9)));
        assert!(c3.contains(&ge(&graph, 9, 6)));
        assert!(c3.contains(&ge(&graph, 6, 3)));
        assert!(c3.contains(&ge(&graph, 3, 0)));
        assert!(c3.contains(&ge(&graph, 0, 1)));
        assert_eq!(7, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&ge(&graph, 2, 5)));
        assert!(c4.contains(&ge(&graph, 5, 6)));
        assert!(c4.contains(&ge(&graph, 6, 3)));
        assert!(c4.contains(&ge(&graph, 3, 0)));
        assert!(c4.contains(&ge(&graph, 0, 2)));
        assert_eq!(5, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 4, 5)));
        assert!(c5.contains(&ge(&graph, 5, 6)));
        assert!(c5.contains(&ge(&graph, 6, 9)));
        assert!(c5.contains(&ge(&graph, 9, 7)));
        assert!(c5.contains(&ge(&graph, 7, 4)));
        assert_eq!(5, c5.len());
        let c6 = cycles[5].clone();
        assert!(c6.contains(&ge(&graph, 5, 8)));
        assert!(c6.contains(&ge(&graph, 8, 9)));
        assert!(c6.contains(&ge(&graph, 9, 6)));
        assert!(c6.contains(&ge(&graph, 6, 5)));
        assert_eq!(4, c6.len());
        let c7 = cycles[6].clone();
        assert!(c7.contains(&ge(&graph, 7, 8)));
        assert!(c7.contains(&e89_1));
        assert!(c7.contains(&ge(&graph, 9, 7)));
        assert_eq!(3, c7.len());
        let c8 = cycles[7].clone();
        assert!(c8.contains(&e89_2));
        assert!(c8.contains(&e89_1));
        assert_eq!(2, c8.len());
        let c9 = cycles[8].clone();
        assert!(c9.contains(&e89_3));
        assert!(c9.contains(&e89_1));
        assert_eq!(2, c9.len());
        let c10 = cycles[9].clone();
        assert!(c10.contains(&e89_4));
        assert!(c10.contains(&e89_1));
        assert_eq!(2, c10.len());
        let c11 = cycles[10].clone();
        assert!(c11.contains(&e77_1));
        assert_eq!(1, c11.len());
        let c12 = cycles[11].clone();
        assert!(c12.contains(&e77_2));
        assert_eq!(1, c12.len());
        let c13 = cycles[12].clone();
        assert!(c13.contains(&e77_3));
        assert_eq!(1, c13.len());
        for c in cb.cycles() {
            assert_cycle(&graph, c);
        }
        assert_eq!(39, cb.length());
        assert!((39.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_multi_graph_with_multiple_components_with_loops() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_vertex(2);
        graph.add_edge(&0, &1).unwrap();
        graph.add_edge(&0, &2).unwrap();
        let e12_1 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e12_2 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e11_1 = graph.add_edge(&1, &1).unwrap().unwrap();
        let e11_2 = graph.add_edge(&1, &1).unwrap().unwrap();
        graph.add_vertex(3);
        graph.add_vertex(4);
        graph.add_vertex(5);
        graph.add_edge(&3, &4).unwrap();
        graph.add_edge(&3, &5).unwrap();
        let e45_1 = graph.add_edge(&4, &5).unwrap().unwrap();
        let e45_2 = graph.add_edge(&4, &5).unwrap().unwrap();
        let e55_1 = graph.add_edge(&5, &5).unwrap().unwrap();
        let e55_2 = graph.add_edge(&5, &5).unwrap().unwrap();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(8, cb.cycles().len());
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        let c1 = cycles[0].clone();
        assert!(c1.contains(&ge(&graph, 0, 1)));
        assert!(c1.contains(&e12_1));
        assert!(c1.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&ge(&graph, 0, 1)));
        assert!(c2.contains(&e12_2));
        assert!(c2.contains(&ge(&graph, 2, 0)));
        assert_eq!(3, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&e11_1));
        assert_eq!(1, c3.len());
        let c4 = cycles[3].clone();
        assert!(c4.contains(&e11_2));
        assert_eq!(1, c4.len());
        let c5 = cycles[4].clone();
        assert!(c5.contains(&ge(&graph, 3, 4)));
        assert!(c5.contains(&e45_1));
        assert!(c5.contains(&ge(&graph, 5, 3)));
        assert_eq!(3, c5.len());
        let c6 = cycles[5].clone();
        assert!(c6.contains(&ge(&graph, 3, 4)));
        assert!(c6.contains(&e45_2));
        assert!(c6.contains(&ge(&graph, 5, 3)));
        assert_eq!(3, c6.len());
        let c7 = cycles[6].clone();
        assert!(c7.contains(&e55_1));
        assert_eq!(1, c7.len());
        let c8 = cycles[7].clone();
        assert!(c8.contains(&e55_2));
        assert_eq!(1, c8.len());
        assert_eq!(16, cb.length());
        assert!((16.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_two_parallel_edges() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        let e1 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e2 = graph.add_edge(&0, &1).unwrap().unwrap();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(1, cb.cycles().len());
        let c1 = cb.cycles().first().unwrap().clone();
        assert!(c1.contains(&e1));
        assert!(c1.contains(&e2));
        assert_eq!(2, c1.len());
        assert_eq!(2, cb.length());
        assert!((2.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_more_parallel_edges() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_vertex(2);
        graph.add_vertex(3);
        let e01_1 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e01_2 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e12 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e23_1 = graph.add_edge(&2, &3).unwrap().unwrap();
        let e23_2 = graph.add_edge(&2, &3).unwrap().unwrap();
        let e30 = graph.add_edge(&3, &0).unwrap().unwrap();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(3, cb.cycles().len());
        let cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
        let c1 = cycles[0].clone();
        assert!(c1.contains(&e01_2));
        assert!(c1.contains(&e01_1));
        assert_eq!(2, c1.len());
        let c2 = cycles[1].clone();
        assert!(c2.contains(&e01_1));
        assert!(c2.contains(&e12));
        assert!(c2.contains(&e23_1));
        assert!(c2.contains(&e30));
        assert_eq!(4, c2.len());
        let c3 = cycles[2].clone();
        assert!(c3.contains(&e23_2));
        assert!(c3.contains(&e23_1));
        assert_eq!(2, c3.len());
        assert_eq!(8, cb.length());
        assert!((8.0 - cb.weight()).abs() <= 0.0001);
    }

    #[test]
    fn test_zero_cycle_space_dimension() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_vertex(2);
        graph.add_vertex(3);
        graph.add_edge(&0, &1).unwrap();
        graph.add_edge(&2, &3).unwrap();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(0, cb.cycles().len());
        assert_eq!(0, cb.length());
        assert!((0.0 - cb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn test_empty_graph() {
        let graph = UGraph::<i32>::pseudo();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(0, cb.cycles().len());
        assert_eq!(0, cb.length());
        assert!((0.0 - cb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn test_1() {
        let mut graph = UGraph::<i32>::simple();
        graph.add_vertices(1..=7);
        graph.add_edge(&1, &2).unwrap();
        graph.add_edge(&1, &3).unwrap();
        graph.add_edge(&2, &4).unwrap();
        graph.add_edge(&2, &5).unwrap();
        graph.add_edge(&3, &6).unwrap();
        graph.add_edge(&3, &7).unwrap();
        graph.add_edge(&4, &5).unwrap();
        graph.add_edge(&6, &7).unwrap();
        graph.add_edge(&4, &6).unwrap();
        let cb = StackBFSFundamentalCycleBasis::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let cycles_sizes = [5, 6, 3];
        for (i, cycle) in cb.cycles().iter().take(3).enumerate() {
            assert_cycle(&graph, cycle);
            assert_eq!(cycles_sizes[i], cycle.len());
        }
        assert_eq!(14, cb.length());
        assert!((14.0 - cb.weight()).abs() <= 1e-9);
    }
}

mod paton_cycle_base_test {
    use super::*;
    use jgrapht::alg::connectivity::ConnectivityInspector;
    use jgrapht::alg::cycle::*;
    use jgrapht::generate::{GnpRandomGraphGenerator, GraphGenerator};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const MAX_SIZE: usize = 10;
    const RESULTS: [usize; 11] = [0, 0, 0, 1, 3, 6, 10, 15, 21, 28, 36];

    type G = UGraph<i32>;

    fn graph(n: i32, edges: &[(i32, i32)]) -> G {
        let mut g = G::simple();
        g.add_vertices(0..n);
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    fn basis(g: &G) -> CycleBasis<DefaultEdge> {
        PatonCycleBase::new(g).unwrap().get_cycle_basis().unwrap()
    }

    fn check_result(g: &G, size: usize) {
        assert_eq!(size, basis(g).cycles().len());
    }

    fn complete(size: i32) -> G {
        let mut g = G::simple();
        g.add_vertices(0..size);
        for i in 0..size {
            for j in 0..size {
                if i != j {
                    g.add_edge(&i, &j).unwrap();
                }
            }
        }
        g
    }

    #[test]
    fn algorithm() {
        let steps: [(&[(i32, i32)], usize); 7] = [
            (&[(0, 1), (1, 2), (2, 0)], 1),
            (&[(2, 3), (3, 0)], 2),
            (&[(3, 1)], 3),
            (&[(3, 4), (4, 2)], 4),
            (&[(4, 5)], 4),
            (&[(5, 2)], 5),
            (&[(5, 6), (6, 4)], 6),
        ];
        let mut g = graph(7, &[]);
        for (edges, expected) in steps {
            for (a, b) in edges {
                g.add_edge(a, b).unwrap();
            }
            check_result(&g, expected);
        }
        for size in 1..=MAX_SIZE {
            check_result(&complete(size as i32), RESULTS[size]);
        }
    }

    fn from_edges(n: i32, edges: &[(i32, i32)]) -> G {
        let mut g = G::simple();
        g.add_vertices(1..=n);
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    const EDGES7: [(i32, i32); 9] = [
        (1, 2),
        (1, 3),
        (2, 4),
        (2, 5),
        (3, 6),
        (3, 7),
        (4, 5),
        (6, 7),
        (4, 6),
    ];

    #[test]
    fn paton_cycle_basis() {
        let g = from_edges(7, &EDGES7);
        let cycles_sizes = [3, 5, 3];
        for (i, cycle) in basis(&g).cycles().iter().take(3).enumerate() {
            assert_eq!(cycles_sizes[i], cycle.len());
        }
    }

    #[test]
    fn paton_cycle_basis1() {
        let g = from_edges(
            15,
            &[
                (1, 2),
                (1, 3),
                (1, 4),
                (1, 12),
                (3, 5),
                (3, 6),
                (12, 13),
                (6, 7),
                (6, 8),
                (13, 14),
                (7, 9),
                (8, 10),
                (14, 15),
                (10, 11),
                (2, 11),
                (5, 4),
                (5, 9),
                (9, 10),
                (9, 11),
                (10, 14),
                (11, 15),
            ],
        );
        let ucb = basis(&g);
        let cycles_sizes = [3, 8, 8, 9, 5, 7, 4];
        for (i, cycle) in ucb.cycles().iter().take(7).enumerate() {
            assert_eq!(cycles_sizes[i], cycle.len());
            assert_cycle(&g, cycle);
        }
        assert_eq!(44, ucb.length());
        assert!((44.0 - ucb.weight()).abs() <= 1e-9);
    }

    fn expected_length(cycles: usize) -> usize {
        // lengths of the successive bases: 3, 6, 9, 12, 12, 15, 18
        [0, 3, 6, 9, 12, 15, 18][cycles]
    }

    #[test]
    fn paton_cycle_basis2() {
        let steps: [(&[(i32, i32)], usize); 7] = [
            (&[(0, 1), (1, 2), (2, 0)], 1),
            (&[(2, 3), (3, 0)], 2),
            (&[(3, 1)], 3),
            (&[(3, 4), (4, 2)], 4),
            (&[(4, 5)], 4),
            (&[(5, 2)], 5),
            (&[(5, 6), (6, 4)], 6),
        ];
        let mut g = graph(7, &[]);
        for (edges, expected) in steps {
            for (a, b) in edges {
                g.add_edge(a, b).unwrap();
            }
            let b = basis(&g);
            assert_eq!(expected, b.cycles().len());
            assert_eq!(expected_length(expected), b.length());
        }
        for size in 1..=MAX_SIZE {
            let g = complete(size as i32);
            let b = basis(&g);
            assert_eq!(RESULTS[size], b.cycles().len());
            assert_eq!(3 * RESULTS[size], b.length());
            assert!((3.0 * RESULTS[size] as f64 - b.weight()).abs() <= 1e-9);
            for c in b.cycles() {
                assert_cycle(&g, c);
            }
        }
    }

    #[test]
    fn paton_cycle_basis3() {
        let mut edges = vec![
            (1, 2),
            (1, 3),
            (2, 4),
            (2, 5),
            (3, 6),
            (3, 7),
            (4, 8),
            (4, 9),
            (5, 10),
            (5, 11),
            (6, 12),
            (6, 13),
            (7, 14),
            (7, 15),
            (8, 9),
            (10, 11),
            (12, 13),
            (14, 15),
        ];
        edges.extend([
            (8, 10),
            (9, 11),
            (10, 12),
            (11, 13),
            (12, 14),
            (8, 11),
            (9, 12),
            (10, 13),
            (11, 14),
            (12, 15),
            (8, 12),
            (9, 13),
            (10, 14),
            (11, 15),
            (8, 13),
            (9, 14),
            (10, 15),
            (8, 14),
            (9, 15),
            (8, 15),
        ]);
        let g = from_edges(15, &edges);
        let ucb = basis(&g);
        for cycle in ucb.cycles().iter().take(24) {
            assert_cycle(&g, cycle);
        }
        assert_eq!(85, ucb.length());
        assert!((85.0 - ucb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn paton_cycle_basis4() {
        let g = from_edges(7, &EDGES7);
        let ucb = basis(&g);
        for cycle in ucb.cycles().iter().take(3) {
            assert_cycle(&g, cycle);
        }
        assert_eq!(11, ucb.length());
        assert!((11.0 - ucb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn paton_cycle_basis5() {
        let g = from_edges(
            15,
            &[
                (1, 2),
                (1, 3),
                (2, 4),
                (2, 5),
                (3, 6),
                (3, 7),
                (4, 8),
                (4, 9),
                (5, 10),
                (5, 11),
                (6, 12),
                (6, 13),
                (7, 14),
                (7, 15),
                (8, 9),
                (10, 11),
                (12, 13),
                (14, 15),
                (8, 10),
            ],
        );
        let ucb = basis(&g);
        let cycles_sizes = [3, 3, 3, 5, 3];
        for (i, cycle) in ucb.cycles().iter().take(5).enumerate() {
            assert_cycle(&g, cycle);
            assert_eq!(cycles_sizes[i], cycle.len());
        }
        assert_eq!(17, ucb.length());
        assert!((17.0 - ucb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn paton_cycle_basis6() {
        let g = from_edges(
            7,
            &[
                (1, 2),
                (1, 3),
                (2, 4),
                (2, 5),
                (2, 3),
                (3, 6),
                (3, 7),
                (4, 6),
                (5, 7),
            ],
        );
        let cycles_sizes = [3, 4, 4];
        for (i, cycle) in basis(&g).cycles().iter().take(3).enumerate() {
            assert_eq!(cycles_sizes[i], cycle.len());
        }
    }

    #[test]
    fn paton_cycle_basis7() {
        let g = from_edges(7, &[(1, 2), (1, 3), (2, 4), (2, 5), (4, 5), (4, 3)]);
        let ucb = basis(&g);
        for cycle in ucb.cycles().iter().take(2) {
            assert_cycle(&g, cycle);
        }
        assert_eq!(7, ucb.length());
        assert!((7.0 - ucb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn paton_cycle_basis8() {
        let mut seed = 0u64;
        for _ in 0..10 {
            seed += 1;
            let mut g = G::simple().with_vertex_factory(|i| i as i32);
            GnpRandomGraphGenerator::new(200, 0.7, StdRng::seed_from_u64(seed), false)
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            let ucb = basis(&g);
            let k = ConnectivityInspector::new().connected_sets(&g).len();
            let cycle_space_dimension = g.edge_count() - g.vertex_count() + k;
            assert_eq!(cycle_space_dimension, ucb.cycles().len());
            for cycle in ucb.cycles() {
                assert_cycle(&g, cycle);
            }
        }
    }

    #[test]
    fn zero_cycle_space_dimension() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertices(0..4);
        graph.add_edge(&0, &1).unwrap();
        graph.add_edge(&2, &3).unwrap();
        let cb = PatonCycleBase::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(0, cb.cycles().len());
        assert_eq!(0, cb.length());
        assert!(cb.weight().abs() <= 1e-9);
    }

    #[test]
    fn with_loops() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertices(0..4);
        let e01 = graph.add_edge(&0, &1).unwrap().unwrap();
        let e12 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e23 = graph.add_edge(&2, &3).unwrap().unwrap();
        let e30 = graph.add_edge(&3, &0).unwrap().unwrap();
        let e00 = graph.add_edge(&0, &0).unwrap().unwrap();
        let e11 = graph.add_edge(&1, &1).unwrap().unwrap();
        let e22 = graph.add_edge(&2, &2).unwrap().unwrap();
        let e33 = graph.add_edge(&3, &3).unwrap().unwrap();
        let cb = PatonCycleBase::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let expected: [Vec<DefaultEdge>; 5] = [
            vec![e00],
            vec![e33],
            vec![e12, e23, e30, e01],
            vec![e22],
            vec![e11],
        ];
        for (i, c) in cb.cycles().iter().take(5).enumerate() {
            assert_cycle(&graph, c);
            assert_eq!(&expected[i], c);
        }
        assert_eq!(5, cb.cycles().len());
        assert_eq!(8, cb.length());
        assert!((8.0 - cb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn single_loops() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        let e1 = graph.add_edge(&0, &0).unwrap().unwrap();
        let cb = PatonCycleBase::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        let c = cb.cycles().first().unwrap();
        assert_cycle(&graph, c);
        assert_eq!(&vec![e1], c);
        assert_eq!(1, cb.cycles().len());
        assert_eq!(1, cb.length());
        assert!((1.0 - cb.weight()).abs() <= 1e-9);
    }

    #[test]
    fn multiple_edges() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertex(0);
        graph.add_edge(&0, &0).unwrap();
        graph.add_edge(&0, &0).unwrap();
        assert!(
            PatonCycleBase::new(&graph)
                .unwrap()
                .get_cycle_basis()
                .is_err()
        );
    }

    #[test]
    fn disconnected_and_weights() {
        let mut graph = UGraph::<i32>::pseudo().weighted();
        graph.add_vertices(0..6);
        for (a, b, w) in [
            (0, 1, 2.0),
            (1, 2, 7.0),
            (2, 0, 13.0),
            (3, 4, 102.0),
            (4, 5, 107.0),
            (5, 3, 113.0),
        ] {
            add_w(&mut graph, a, b, w);
        }
        let cb = PatonCycleBase::new(&graph)
            .unwrap()
            .get_cycle_basis()
            .unwrap();
        assert_eq!(2, cb.cycles().len());
        assert_eq!(6, cb.length());
        assert!((344.0 - cb.weight()).abs() <= 1e-9);
    }
}

// HierholzerEulerianCycleTest

mod hierholzer_eulerian_cycle_test {
    use super::*;
    use jgrapht::alg::cycle::{EulerianCycleAlgorithm, HierholzerEulerianCycle};
    use jgrapht::generate::{Complete, GnpRandomGraphGenerator, GraphGenerator};
    use jgrapht::graph::props::is_eulerian;
    use jgrapht::graph::{Direction, GraphExt};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::collections::HashSet;

    fn assert_eulerian<G: IncidenceGraph>(g: &G, cycle: &GraphWalk<G::Vertex, G::Edge>) {
        assert!(is_eulerian(g));
        assert!(g.vertex_count() > 0);
        if g.edge_count() == 0 {
            assert!(cycle.start_vertex().is_none());
            assert!(cycle.end_vertex().is_none());
            assert!(cycle.edge_list().is_empty());
        } else {
            let directed = g.graph_type().directed;
            assert!(cycle.start_vertex().is_some());
            assert!(cycle.start_vertex() == cycle.end_vertex());
            assert_eq!(g.edge_count(), cycle.length());
            let mut dup = HashSet::new();
            let mut prev: Option<&G::Edge> = None;
            for cur in cycle.edge_list() {
                assert!(dup.insert(cur.clone()));
                if let Some(prev) = prev {
                    let (cs, ct) = g.edge_endpoints(cur).unwrap();
                    let (ps, pt) = g.edge_endpoints(prev).unwrap();
                    if directed {
                        assert!(cs == pt);
                    } else {
                        assert!(cs == ps || cs == pt || ct == ps || ct == pt);
                    }
                }
                prev = Some(cur);
            }
        }
    }

    fn complete<D: Direction>(ty: GraphType, n: usize) -> BaseGraph<i32, DefaultEdge, D> {
        let mut g =
            BaseGraph::<i32, DefaultEdge, D>::with_graph_type(ty).with_vertex_factory(|i| i as i32);
        Complete(n).generate_default(&mut g).unwrap();
        g
    }

    #[test]
    fn test_null_eulerian() {
        let g1 = UGraph::<i32>::pseudo();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g1));
        let g2 = DiGraph::<i32>::pseudo();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g2));
    }

    #[test]
    fn test_empty_eulerian() {
        let mut g1 = UGraph::<i32>::pseudo();
        g1.add_vertex(1);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g1));
        g1.add_vertex(2);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g1));
        g1.add_vertex(3);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g1));
        let mut g2 = DiGraph::<i32>::pseudo();
        g2.add_vertex(1);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g2));
        g2.add_vertex(2);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g2));
        g2.add_vertex(3);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g2));
    }

    #[test]
    fn test_undirected_disconnected_eulerian() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertex(1);
        g.add_vertex(2);
        g.add_vertex(3);
        g.add_vertex(4);
        g.add_vertex(5);
        g.add_vertex(6);
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &2).unwrap();
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_undirected_disconnected_non_eulerian() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertex(1);
        g.add_vertex(2);
        g.add_vertex(3);
        g.add_vertex(4);
        g.add_vertex(5);
        g.add_vertex(6);
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &2).unwrap();
        g.add_edge(&5, &6).unwrap();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_directed_disconnected_eulerian() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertex(1);
        g.add_vertex(2);
        g.add_vertex(3);
        g.add_vertex(4);
        g.add_vertex(5);
        g.add_vertex(6);
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &2).unwrap();
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_directed_disconnected_non_eulerian() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertex(1);
        g.add_vertex(2);
        g.add_vertex(3);
        g.add_vertex(4);
        g.add_vertex(5);
        g.add_vertex(6);
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &2).unwrap();
        g.add_edge(&5, &6).unwrap();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_undirected_eulerian_2() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_undirected_eulerian_3() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_undirected_eulerian_4() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertex(1);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_directed_eulerian_2() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1]);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
        g.add_edge(&1, &1).unwrap();
        g.add_edge(&1, &1).unwrap();
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
        g.add_vertices([2]);
        g.add_edge(&2, &1).unwrap();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_directed_eulerian_3() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5]);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &1).unwrap();
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
        g.add_edge(&2, &1).unwrap();
        g.add_edge(&3, &2).unwrap();
        g.add_edge(&4, &3).unwrap();
        g.add_edge(&5, &4).unwrap();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
        g.add_edge(&1, &1).unwrap();
        g.add_edge(&2, &2).unwrap();
        g.add_edge(&3, &3).unwrap();
        g.add_edge(&4, &4).unwrap();
        g.add_edge(&5, &5).unwrap();
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
        g.add_edge(&1, &5).unwrap();
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn test_empty_with_single_vertex_undirected() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertex(1);
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_empty_multiple_vertices_undirected() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertex(1);
        g.add_vertex(2);
        g.add_vertex(3);
        g.add_vertex(4);
        g.add_vertex(5);
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_empty_with_single_vertex_directed() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertex(1);
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_empty_multiple_vertices_directed() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertex(1);
        g.add_vertex(2);
        g.add_vertex(3);
        g.add_vertex(4);
        g.add_vertex(5);
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_undirected_1() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_undirected_2() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6, 7, 8]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&5, &7).unwrap();
        g.add_edge(&5, &7).unwrap();
        g.add_edge(&7, &8).unwrap();
        g.add_edge(&7, &8).unwrap();
        g.add_edge(&5, &8).unwrap();
        g.add_edge(&5, &8).unwrap();
        g.add_edge(&8, &8).unwrap();
        g.add_edge(&8, &8).unwrap();
        g.add_edge(&3, &3).unwrap();
        g.add_edge(&3, &3).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_undirected_4() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&5, &7).unwrap();
        g.add_edge(&5, &7).unwrap();
        g.add_edge(&7, &8).unwrap();
        g.add_edge(&7, &8).unwrap();
        g.add_edge(&5, &8).unwrap();
        g.add_edge(&5, &8).unwrap();
        g.add_edge(&8, &8).unwrap();
        g.add_edge(&8, &8).unwrap();
        g.add_edge(&3, &3).unwrap();
        g.add_edge(&3, &3).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_directed_1() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &3).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_directed_2() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6, 7, 8]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &3).unwrap();
        g.add_edge(&5, &7).unwrap();
        g.add_edge(&7, &8).unwrap();
        g.add_edge(&8, &5).unwrap();
        g.add_edge(&5, &7).unwrap();
        g.add_edge(&7, &8).unwrap();
        g.add_edge(&8, &5).unwrap();
        g.add_edge(&8, &8).unwrap();
        g.add_edge(&8, &8).unwrap();
        g.add_edge(&8, &8).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_directed_3() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6, 7, 8]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&2, &4).unwrap();
        g.add_edge(&4, &2).unwrap();
        g.add_edge(&3, &6).unwrap();
        g.add_edge(&3, &5).unwrap();
        g.add_edge(&5, &3).unwrap();
        g.add_edge(&6, &8).unwrap();
        g.add_edge(&6, &7).unwrap();
        g.add_edge(&7, &6).unwrap();
        g.add_edge(&8, &1).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn test_eulerian_cycle_directed_5() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&3, &1).unwrap();
        g.add_edge(&4, &5).unwrap();
        g.add_edge(&5, &6).unwrap();
        g.add_edge(&6, &4).unwrap();
        g.add_edge(&3, &4).unwrap();
        g.add_edge(&4, &3).unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn undirected_eulerian1() {
        let g = complete::<jgrapht::graph::Undirected>(GraphType::simple(), 6);
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn undirected_eulerian5() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices(1..=6);
        for (a, b) in [
            (1, 2),
            (2, 3),
            (3, 1),
            (4, 5),
            (5, 6),
            (6, 4),
            (3, 4),
            (3, 4),
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        for i in 1..=6 {
            g.add_edge(&i, &i).unwrap();
        }
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn undirected_eulerian6() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices(1..=6);
        for (a, b) in [(1, 2), (2, 3), (3, 1), (4, 5), (5, 6), (6, 4)] {
            g.add_edge(&a, &b).unwrap();
        }
        for i in 1..=6 {
            g.add_edge(&i, &i).unwrap();
        }
        assert!(!HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn undirected_eulerian7() {
        let g = complete::<jgrapht::graph::Undirected>(GraphType::simple(), 5);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g));
    }

    #[test]
    fn directed_eulerian1() {
        let g1 = complete::<jgrapht::graph::Directed>(GraphType::simple(), 6);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g1));
        let g2 = complete::<jgrapht::graph::Directed>(GraphType::simple(), 7);
        assert!(HierholzerEulerianCycle::new().is_eulerian(&g2));
    }

    #[test]
    fn eulerian_cycle_directed4() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices(1..=8);
        for (a, b) in [
            (1, 2),
            (2, 4),
            (2, 3),
            (4, 2),
            (3, 5),
            (3, 6),
            (5, 3),
            (6, 7),
            (6, 8),
            (7, 6),
            (8, 1),
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn eulerian_cycle_undirected3() {
        let mut rng = StdRng::seed_from_u64(17);
        for size in (13..52).step_by(2) {
            let mut g = complete::<jgrapht::graph::Undirected>(GraphType::pseudograph(), size);
            let vertices: Vec<i32> = g.vertices().copied().collect();
            for v in vertices {
                for _ in 0..=rng.random_range(0..10) {
                    g.add_edge(&v, &v).unwrap();
                }
            }
            let edges: Vec<DefaultEdge> = g.edges().cloned().collect();
            for e in edges {
                let (s, t) = {
                    let (s, t) = g.edge_endpoints(&e).unwrap();
                    (*s, *t)
                };
                for _ in 0..=2 * rng.random_range(0..10) {
                    g.add_edge(&s, &t).unwrap();
                }
            }
            let cycle = HierholzerEulerianCycle::new()
                .get_eulerian_cycle(&g)
                .unwrap();
            assert_eulerian(&g, &cycle);
        }
    }

    #[test]
    fn eulerian_cycle_directed() {
        let mut rng = StdRng::seed_from_u64(17);
        for size in (5..52).step_by(2) {
            let mut g = complete::<jgrapht::graph::Directed>(GraphType::pseudograph(), size);
            let vertices: Vec<i32> = g.vertices().copied().collect();
            for v in vertices {
                for _ in 0..=rng.random_range(0..10) {
                    g.add_edge(&v, &v).unwrap();
                }
            }
            let edges: Vec<DefaultEdge> = g.edges().cloned().collect();
            for e in edges {
                let (s, t) = {
                    let (s, t) = g.edge_endpoints(&e).unwrap();
                    (*s, *t)
                };
                for _ in 0..=2 * rng.random_range(0..10) {
                    g.add_edge(&s, &t).unwrap();
                    g.add_edge(&t, &s).unwrap();
                }
            }
            let cycle = HierholzerEulerianCycle::new()
                .get_eulerian_cycle(&g)
                .unwrap();
            assert_eulerian(&g, &cycle);
        }
    }

    fn random_graph<D: Direction>(
        rng: &mut StdRng,
        seed: u64,
        loops: bool,
        extra_loops: bool,
    ) -> BaseGraph<i32, DefaultEdge, D> {
        let mut g = BaseGraph::<i32, DefaultEdge, D>::pseudo().with_vertex_factory(|i| i as i32);
        GnpRandomGraphGenerator::new(50, 0.7, StdRng::seed_from_u64(seed), loops)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        let edges: Vec<DefaultEdge> = g.edges().cloned().collect();
        for e in edges {
            let (s, t) = {
                let (s, t) = g.edge_endpoints(&e).unwrap();
                (*s, *t)
            };
            g.add_edge(&t, &s).unwrap();
        }
        if extra_loops {
            let vertices: Vec<i32> = g.vertices().copied().collect();
            for v in vertices {
                for _ in 0..=rng.random_range(0..10) {
                    g.add_edge(&v, &v).unwrap();
                }
            }
        }
        g
    }

    #[test]
    fn random_undirected() {
        let mut rng = StdRng::seed_from_u64(1);
        for i in 0..100 {
            let g = random_graph::<jgrapht::graph::Undirected>(&mut rng, i, true, true);
            let cycle = HierholzerEulerianCycle::new()
                .get_eulerian_cycle(&g)
                .unwrap();
            assert_eulerian(&g, &cycle);
        }
    }

    #[test]
    fn random_undirected_fixed_seed() {
        let mut rng = StdRng::seed_from_u64(17);
        for i in 0..100 {
            let g = random_graph::<jgrapht::graph::Undirected>(&mut rng, 17 + i, false, false);
            let cycle = HierholzerEulerianCycle::new()
                .get_eulerian_cycle(&g)
                .unwrap();
            assert_eulerian(&g, &cycle);
        }
    }

    #[test]
    fn random_directed() {
        let mut rng = StdRng::seed_from_u64(2);
        for i in 0..100 {
            let g = random_graph::<jgrapht::graph::Directed>(&mut rng, i, true, true);
            let cycle = HierholzerEulerianCycle::new()
                .get_eulerian_cycle(&g)
                .unwrap();
            assert_eulerian(&g, &cycle);
        }
    }

    #[test]
    fn random_directed_fixed_seed() {
        let mut rng = StdRng::seed_from_u64(17);
        for i in 0..100 {
            let g = random_graph::<jgrapht::graph::Directed>(&mut rng, 17 + i, false, false);
            let cycle = HierholzerEulerianCycle::new()
                .get_eulerian_cycle(&g)
                .unwrap();
            assert_eulerian(&g, &cycle);
        }
    }

    #[test]
    fn eulerian_cycle_undirected_vertex_list() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([4, 3, 2]);
        let e42 = g.add_edge(&4, &2).unwrap().unwrap();
        let e34 = g.add_edge(&3, &4).unwrap().unwrap();
        let e32 = g.add_edge(&3, &2).unwrap().unwrap();
        let cycle = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eq!(e32, cycle.edge_list()[0]);
        assert_eq!(e34, cycle.edge_list()[1]);
        assert_eq!(e42, cycle.edge_list()[2]);
        assert_eq!(&[2, 3, 4, 2], cycle.vertex_list());
        assert_eulerian(&g, &cycle);
    }

    #[test]
    fn pseudograph() {
        // GitHub issue 388
        let mut g = UGraph::<char>::pseudo().weighted();
        g.add_vertices(['A', 'B', 'C', 'D', 'E']);
        for (a, b, w) in [
            ('A', 'B', 8.0),
            ('A', 'C', 5.0),
            ('A', 'D', 6.0),
            ('B', 'C', 5.0),
            ('B', 'E', 6.0),
            ('C', 'D', 5.0),
            ('C', 'E', 5.0),
            ('D', 'E', 8.0),
            ('A', 'D', 8.0),
            ('B', 'E', 8.0),
        ] {
            g.add_weighted_edge(&a, &b, w).unwrap();
        }
        let gp = HierholzerEulerianCycle::new()
            .get_eulerian_cycle(&g)
            .unwrap();
        assert_eq!(Some(&'E'), gp.start_vertex());
        assert_eq!(
            &['E', 'B', 'E', 'D', 'A', 'D', 'C', 'B', 'A', 'C', 'E'],
            gp.vertex_list()
        );
        assert_eulerian(&g, &gp);
    }
}

// HowardMinimumMeanCycleTest

mod howard_minimum_mean_cycle_test {
    use super::*;
    use jgrapht::alg::cycle::{HowardMinimumMeanCycle, MinimumCycleMeanAlgorithm};

    const GRAPH1: &[(i32, i32, i32)] = &[
        (1, 2, 1),
        (1, 3, 10),
        (2, 3, 3),
        (3, 4, 2),
        (4, 1, 8),
        (4, 2, 0),
    ];
    const GRAPH2: &[(i32, i32, i32)] = &[(1, 3, 7), (3, 2, 3), (2, 0, 7), (2, 1, 5)];
    const GRAPH3: &[(i32, i32, i32)] = &[
        (0, 2, 16),
        (0, 3, 0),
        (3, 0, 14),
        (5, 0, 16),
        (0, 8, 12),
        (5, 1, 13),
        (1, 6, 4),
        (6, 1, 15),
        (7, 1, 2),
        (1, 9, 8),
        (2, 6, 3),
        (7, 2, 15),
        (9, 2, 10),
        (3, 6, 1),
        (3, 8, 8),
        (8, 3, 18),
        (4, 6, 7),
        (4, 9, 13),
        (5, 6, 3),
        (8, 6, 7),
        (7, 8, 7),
        (9, 7, 17),
    ];
    const GRAPH4: &[(i32, i32, i32)] = &[
        (0, 3, 19),
        (4, 0, 0),
        (0, 5, 8),
        (5, 0, 17),
        (0, 7, 10),
        (8, 0, 15),
        (1, 4, 14),
        (7, 1, 10),
        (3, 2, 14),
        (2, 4, 3),
        (2, 5, 1),
        (2, 9, 1),
        (5, 3, 18),
        (6, 3, 4),
        (3, 7, 2),
        (8, 3, 8),
        (5, 4, 17),
        (6, 4, 5),
        (8, 4, 15),
        (9, 4, 17),
        (6, 5, 1),
        (5, 7, 19),
        (9, 5, 12),
        (6, 8, 15),
        (8, 6, 19),
        (7, 9, 6),
    ];
    const GRAPH5: &[(i32, i32, i32)] = &[
        (6, 0, 11),
        (8, 0, 5),
        (4, 1, 3),
        (2, 3, 6),
        (7, 2, 9),
        (3, 4, 19),
        (3, 9, 6),
        (4, 9, 8),
        (6, 5, 5),
        (5, 9, 16),
        (6, 7, 16),
        (6, 8, 12),
        (8, 9, 12),
    ];
    const GRAPH6: &[(i32, i32, i32)] = &[
        (0, 2, 16),
        (0, 3, 0),
        (3, 0, 14),
        (5, 0, 16),
        (0, 8, 12),
        (13, 0, 13),
        (0, 14, 4),
        (14, 0, 15),
        (2, 1, 2),
        (1, 4, 8),
        (1, 8, 3),
        (9, 1, 15),
        (11, 1, 10),
        (1, 14, 1),
        (2, 4, 8),
        (4, 2, 18),
        (2, 7, 7),
        (2, 10, 13),
        (2, 11, 3),
        (5, 3, 7),
        (3, 7, 7),
        (8, 3, 17),
        (3, 12, 19),
        (13, 3, 0),
        (3, 14, 8),
        (14, 3, 17),
        (4, 6, 10),
        (7, 4, 15),
        (4, 11, 14),
        (14, 4, 10),
        (8, 5, 14),
        (5, 9, 3),
        (5, 10, 1),
        (5, 14, 1),
        (8, 6, 18),
        (9, 6, 4),
        (6, 10, 2),
        (11, 6, 8),
        (13, 6, 17),
        (14, 6, 5),
        (9, 7, 15),
        (10, 7, 17),
        (11, 7, 1),
        (7, 12, 19),
        (14, 7, 12),
        (8, 10, 15),
        (10, 8, 19),
        (8, 13, 6),
        (9, 10, 2),
        (12, 9, 17),
        (13, 9, 8),
        (10, 13, 3),
        (11, 14, 2),
        (12, 14, 5),
        (14, 12, 13),
        (14, 13, 5),
    ];
    const GRAPH7: &[(i32, i32, i32)] = &[
        (0, 1, 5),
        (2, 0, 8),
        (0, 3, 8),
        (4, 0, 10),
        (0, 5, 7),
        (5, 0, 8),
        (6, 0, 2),
        (8, 0, 7),
        (11, 0, 3),
        (0, 14, 1),
        (2, 1, 12),
        (1, 6, 16),
        (1, 12, 11),
        (12, 1, 18),
        (1, 13, 10),
        (2, 3, 9),
        (2, 4, 9),
        (4, 2, 12),
        (5, 2, 15),
        (7, 2, 10),
        (2, 8, 10),
        (8, 2, 8),
        (2, 10, 12),
        (2, 12, 6),
        (12, 2, 10),
        (3, 5, 13),
        (3, 9, 8),
        (11, 3, 8),
        (13, 3, 2),
        (7, 4, 17),
        (8, 4, 17),
        (12, 4, 11),
        (5, 6, 13),
        (8, 5, 8),
        (9, 5, 2),
        (5, 10, 11),
        (5, 11, 6),
        (5, 12, 12),
        (12, 5, 17),
        (5, 13, 13),
        (6, 8, 7),
        (6, 9, 17),
        (6, 13, 4),
        (6, 14, 15),
        (14, 6, 19),
        (7, 8, 18),
        (8, 7, 19),
        (7, 11, 18),
        (11, 7, 8),
        (12, 7, 10),
        (7, 13, 4),
        (13, 7, 17),
        (8, 9, 15),
        (9, 8, 8),
        (9, 12, 11),
        (9, 14, 3),
        (10, 11, 1),
        (11, 10, 12),
        (10, 13, 17),
        (11, 12, 2),
        (12, 11, 18),
        (11, 13, 9),
        (13, 11, 5),
        (11, 14, 3),
        (14, 12, 8),
        (14, 13, 9),
    ];

    fn test_on_graph(
        edges: &[(i32, i32, i32)],
        expected_mean: f64,
        expected_cycle: Option<&[(i32, i32)]>,
    ) {
        let mut graph = DiGraph::<i32>::pseudo().weighted();
        for (u, v, w) in edges {
            graph.add_vertex(*u);
            graph.add_vertex(*v);
            graph.add_weighted_edge(u, v, *w as f64).unwrap();
        }
        let expected_path = expected_cycle.map(|path| {
            let start = path[0].0;
            let end = path[path.len() - 1].1;
            let mut edge_list = Vec::new();
            let mut vertices = vec![start];
            let mut weight = 0.0;
            for (source, target) in path {
                let mut minimum_weight = f64::INFINITY;
                let mut minimum_weight_edge = None;
                for edge in graph.edges_between(source, target) {
                    let w = graph.edge_weight(edge).unwrap();
                    if w < minimum_weight {
                        minimum_weight = w;
                        minimum_weight_edge = Some(*edge);
                    }
                }
                edge_list.push(minimum_weight_edge.unwrap());
                vertices.push(*target);
                weight += minimum_weight;
            }
            assert_eq!(end, *vertices.last().unwrap());
            GraphWalk::new(vertices, edge_list, weight)
        });
        let mut mmc = HowardMinimumMeanCycle::new(&graph);
        let actual_path = mmc.get_cycle();
        let actual_mean = mmc.get_cycle_mean();
        assert!((expected_mean - actual_mean).abs() <= 1e-9 || expected_mean == actual_mean);
        assert_eq!(expected_path, actual_path);
    }

    #[test]
    fn graph1() {
        test_on_graph(GRAPH1, 1.6666666666666667, Some(&[(2, 3), (3, 4), (4, 2)]));
    }

    #[test]
    fn graph2() {
        test_on_graph(GRAPH2, 5.0, Some(&[(1, 3), (3, 2), (2, 1)]));
    }

    #[test]
    fn graph3() {
        test_on_graph(GRAPH3, 7.0, Some(&[(0, 3), (3, 0)]));
    }

    #[test]
    fn graph4() {
        test_on_graph(GRAPH4, 8.25, Some(&[(0, 7), (7, 9), (9, 4), (4, 0)]));
    }

    #[test]
    fn graph5() {
        test_on_graph(GRAPH5, f64::INFINITY, None);
    }

    #[test]
    fn graph6() {
        test_on_graph(
            GRAPH6,
            3.6000000000000001,
            Some(&[(14, 6), (6, 10), (10, 13), (13, 3), (3, 14)]),
        );
    }

    #[test]
    fn graph7() {
        test_on_graph(
            GRAPH7,
            4.4285714285714288,
            Some(&[(0, 14), (14, 13), (13, 3), (3, 9), (9, 5), (5, 11), (11, 0)]),
        );
    }
}

// ChordalityInspectorTest

mod chordality_inspector_test {
    use super::*;
    use crate::common::construct_graph;
    use jgrapht::alg::cycle::{ChordalityInspector, IterationOrder};
    use jgrapht::generate::named::{self, build};

    const ORDERS: [IterationOrder; 2] = [IterationOrder::Mcs, IterationOrder::LexBfs];

    fn undirected(edges: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::with_loops().weighted();
        construct_graph(&mut g, edges);
        g
    }

    fn pseudograph(edges: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::pseudo().weighted();
        construct_graph(&mut g, edges);
        g
    }

    fn assert_is_hole<G: Graph>(graph: &G, path: &GraphWalk<G::Vertex, G::Edge>) {
        let cycle = path.vertex_list();
        assert!(cycle.len() > 4);
        for i in 0..cycle.len() - 1 {
            assert!(graph.contains_edge_between(&cycle[i], &cycle[i + 1]));
        }
        for i in 0..cycle.len() - 2 {
            for j in 0..cycle.len() - 2 {
                if i.abs_diff(j) > 1 {
                    assert!(!graph.contains_edge_between(&cycle[i], &cycle[j]));
                }
            }
        }
    }

    #[test]
    fn iteration_order() {
        for order in ORDERS {
            let graph = UGraph::<i32>::with_loops();
            assert_eq!(
                order,
                ChordalityInspector::with_order(&graph, order).iteration_order()
            );
        }
    }

    #[test]
    fn get_chordless_cycle() {
        let mut graph = UGraph::<i32>::with_loops();
        let upper_bound = 100;
        for i in 0..upper_bound {
            graph.add_vertex(i);
            graph.add_vertex(i + 1);
            graph.add_edge(&i, &(i + 1)).unwrap();
        }
        graph.add_edge(&0, &upper_bound).unwrap();
        let mut inspector = ChordalityInspector::new(&graph);
        let path = inspector.get_hole().expect("a hole").clone();
        assert_is_hole(&graph, &path);
    }

    #[test]
    fn perfect_elimination_order() {
        for order in ORDERS {
            let graph = undirected(&[&[1, 2], &[1, 3], &[2, 3], &[2, 4], &[3, 4]]);
            let mut inspector = ChordalityInspector::with_order(&graph, order);
            let order1 = inspector
                .get_perfect_elimination_order()
                .expect("chordal")
                .to_vec();
            assert!(inspector.get_hole().is_none());
            // the result is cached, so asking again gives the same order
            let order2 = inspector
                .get_perfect_elimination_order()
                .expect("chordal")
                .to_vec();
            assert_eq!(order1, order2);
            assert!(inspector.get_hole().is_none());
        }
    }

    #[test]
    fn is_chordal1() {
        let graph = UGraph::<i32>::with_loops();
        let mut inspector = ChordalityInspector::new(&graph);
        assert!(inspector.is_chordal());
        assert!(inspector.get_perfect_elimination_order().is_some());
    }

    fn chordal(graph: &UGraph<i32>) {
        let mut inspector = ChordalityInspector::new(graph);
        assert!(inspector.is_chordal());
        assert!(inspector.get_hole().is_none());
        assert!(inspector.get_perfect_elimination_order().is_some());
    }

    fn not_chordal<G: IncidenceGraph>(graph: &G)
    where
        G: jgrapht::graph::UndirectedGraph,
    {
        let mut inspector = ChordalityInspector::new(graph);
        assert!(!inspector.is_chordal());
        let path = inspector.get_hole().expect("a hole").clone();
        assert_is_hole(graph, &path);
        assert!(inspector.get_perfect_elimination_order().is_none());
    }
    #[test]
    fn is_chordal2() {
        chordal(&undirected(&[&[1, 2], &[1, 3], &[2, 3], &[2, 4], &[3, 4]]));
    }

    #[test]
    fn is_chordal3() {
        chordal(&undirected(&[
            &[1, 2],
            &[2, 3],
            &[3, 1],
            &[4, 5],
            &[5, 6],
            &[6, 4],
        ]));
    }

    #[test]
    fn is_chordal4() {
        chordal(&undirected(&[
            &[1, 2],
            &[1, 3],
            &[2, 3],
            &[3, 4],
            &[3, 5],
            &[4, 5],
            &[5, 6],
            &[5, 7],
            &[6, 7],
            &[7, 8],
            &[7, 9],
            &[8, 9],
            &[9, 1],
            &[10, 1],
            &[3, 7],
            &[1, 7],
        ]));
    }

    #[test]
    fn is_chordal5() {
        not_chordal(&undirected(&[
            &[1, 2],
            &[2, 3],
            &[3, 4],
            &[4, 1],
            &[1, 5],
            &[5, 2],
            &[2, 6],
            &[6, 3],
            &[3, 7],
            &[7, 4],
            &[4, 8],
            &[8, 1],
            &[5, 6],
            &[6, 7],
            &[7, 8],
            &[8, 5],
            &[5, 7],
            &[6, 8],
        ]));
    }

    #[test]
    fn is_chordal6() {
        chordal(&pseudograph(&[
            &[1, 1],
            &[1, 2],
            &[1, 2],
            &[1, 3],
            &[3, 1],
            &[2, 3],
        ]));
    }

    #[test]
    fn is_chordal7() {
        not_chordal(&pseudograph(&[
            &[1, 1],
            &[1, 2],
            &[2, 1],
            &[2, 2],
            &[3, 3],
            &[4, 4],
            &[2, 3],
            &[2, 3],
            &[3, 4],
            &[4, 5],
            &[5, 2],
        ]));
    }

    #[test]
    fn is_chordal8() {
        not_chordal(&build(named::ellingham_horton78()));
    }

    #[test]
    fn is_chordal9() {
        not_chordal(&build(named::gosset()));
    }

    #[test]
    fn is_chordal10() {
        not_chordal(&build(named::klein3_regular()));
    }

    #[test]
    fn is_chordal11() {
        not_chordal(&build(named::schlafli()));
    }

    #[test]
    fn is_chordal12() {
        not_chordal(&build(named::bucky_ball()));
    }

    #[test]
    fn is_perfect_elimination_order1() {
        for it in ORDERS {
            let graph = undirected(&[&[1, 2], &[1, 3], &[1, 4], &[2, 4], &[3, 4]]);
            let order = [1, 2, 3, 4];
            assert!(
                !ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
        }
    }

    #[test]
    fn is_perfect_elimination_order2() {
        for it in ORDERS {
            let graph = undirected(&[&[1, 2], &[1, 4], &[2, 3], &[3, 4]]);
            let order = [1, 2, 4, 3];
            // not a perfect elimination order: cycle 1->2->3->4->1 has no chord
            assert!(
                !ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
            let mut graph = graph;
            graph.add_edge(&2, &4).unwrap();
            // valid: no induced cycles of length > 3
            assert!(
                ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
        }
    }

    #[test]
    fn is_perfect_elimination_order3() {
        for it in ORDERS {
            let graph = undirected(&[
                &[1, 2],
                &[1, 3],
                &[1, 4],
                &[2, 3],
                &[3, 4],
                &[3, 5],
                &[3, 6],
                &[4, 5],
                &[5, 6],
                &[6, 7],
                &[6, 8],
                &[6, 9],
                &[7, 8],
                &[8, 9],
                &[8, 10],
                &[9, 10],
            ]);
            let order = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
            assert!(
                ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
        }
    }

    #[test]
    fn is_perfect_elimination_order4() {
        for it in ORDERS {
            let graph = undirected(&[
                &[1, 2],
                &[1, 3],
                &[1, 4],
                &[2, 3],
                &[3, 4],
                &[3, 5],
                &[3, 6],
                &[3, 7],
                &[4, 5],
                &[5, 6],
                &[5, 7],
                &[6, 7],
                &[6, 8],
                &[7, 9],
                &[7, 10],
                &[7, 11],
                &[9, 10],
                &[9, 11],
                &[9, 12],
                &[10, 11],
                &[11, 12],
            ]);
            let order = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
            assert!(
                ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
        }
    }

    #[test]
    fn is_perfect_elimination_order5() {
        for it in ORDERS {
            let graph = undirected(&[
                &[1, 2],
                &[1, 3],
                &[2, 3],
                &[2, 4],
                &[3, 4],
                &[3, 5],
                &[4, 5],
                &[4, 6],
                &[5, 6],
            ]);
            let order = [1, 2, 5, 6, 4, 3];
            assert!(
                !ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
        }
    }

    #[test]
    fn is_perfect_elimination_order6() {
        for it in ORDERS {
            let graph = undirected(&[
                &[1, 2],
                &[2, 3],
                &[2, 4],
                &[3, 4],
                &[4, 5],
                &[4, 6],
                &[5, 6],
                &[6, 7],
                &[6, 8],
                &[7, 8],
                &[8, 9],
                &[8, 10],
                &[9, 10],
                &[10, 1],
                &[10, 2],
            ]);
            let order = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
            assert!(
                !ChordalityInspector::with_order(&graph, it).is_perfect_elimination_order(&order)
            );
        }
    }
}

// ChordalGraphMinimalVertexSeparatorFinderTest

mod chordal_graph_minimal_vertex_separator_finder_test {
    use super::*;
    use jgrapht::alg::cycle::ChordalGraphMinimalVertexSeparatorFinder;
    use std::collections::HashSet;

    type Separators = Vec<(HashSet<i32>, usize)>;

    /// Parallel edges in the data are ignored, as `addEdge` does for a graph without them.
    fn undirected(edges: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::with_loops().weighted();
        for e in edges {
            g.add_vertex(e[0]);
            g.add_vertex(e[1]);
            g.add_edge(&e[0], &e[1]).unwrap();
        }
        g
    }

    fn separators(graph: &UGraph<i32>) -> Option<(HashSet<Vec<i32>>, Separators)> {
        let mut finder = ChordalGraphMinimalVertexSeparatorFinder::new(graph);
        let keys: Option<HashSet<Vec<i32>>> = finder.get_minimal_separators().map(|s| {
            s.into_iter()
                .map(|v| {
                    let mut v = v.clone();
                    v.sort_unstable();
                    v
                })
                .collect()
        });
        let with: Option<Separators> =
            finder
                .get_minimal_separators_with_multiplicities()
                .map(|m| {
                    m.iter()
                        .map(|(k, c)| (k.iter().copied().collect(), *c))
                        .collect()
                });
        keys.zip(with)
    }

    fn check(graph: &UGraph<i32>, expected: &[(&[i32], usize)]) {
        let (keys, with) = separators(graph).expect("chordal graph");
        let expected_sets: Vec<(HashSet<i32>, usize)> = expected
            .iter()
            .map(|(s, c)| (s.iter().copied().collect(), *c))
            .collect();
        assert_eq!(expected_sets.len(), keys.len());
        assert_eq!(expected_sets.len(), with.len());
        for entry in &expected_sets {
            assert!(with.contains(entry), "missing {entry:?}");
        }
    }

    #[test]
    fn get_minimal_separators1() {
        check(&UGraph::<i32>::with_loops(), &[]);
    }

    #[test]
    fn get_minimal_separators2() {
        check(
            &undirected(&[&[1, 2], &[1, 3], &[2, 3], &[2, 4], &[3, 4]]),
            &[(&[2, 3], 1)],
        );
    }

    #[test]
    fn get_minimal_separators3() {
        let g = undirected(&[
            &[1, 2],
            &[1, 3],
            &[2, 3],
            &[3, 4],
            &[3, 5],
            &[3, 6],
            &[3, 8],
            &[3, 10],
            &[3, 11],
            &[4, 5],
            &[4, 6],
            &[5, 6],
            &[6, 7],
            &[6, 8],
            &[6, 10],
            &[6, 11],
            &[7, 8],
            &[7, 10],
            &[8, 9],
            &[8, 10],
            &[9, 10],
        ]);
        check(
            &g,
            &[(&[3], 1), (&[3, 6], 2), (&[8, 10], 1), (&[6, 8, 10], 1)],
        );
    }

    #[test]
    fn get_minimal_separators4() {
        let g = undirected(&[
            &[1, 2],
            &[2, 8],
            &[2, 9],
            &[3, 8],
            &[3, 9],
            &[4, 6],
            &[4, 8],
            &[5, 6],
            &[5, 8],
            &[6, 7],
            &[6, 8],
            &[6, 9],
            &[7, 8],
            &[7, 9],
            &[8, 9],
            &[8, 10],
            &[8, 11],
            &[8, 12],
            &[9, 10],
            &[9, 11],
            &[9, 12],
            &[10, 11],
            &[11, 12],
        ]);
        check(
            &g,
            &[(&[2], 1), (&[6, 8], 2), (&[8, 9], 3), (&[8, 9, 11], 1)],
        );
    }

    #[test]
    fn get_minimal_separators5() {
        let g = undirected(&[&[1, 2], &[1, 3], &[2, 4], &[3, 4]]);
        assert!(separators(&g).is_none());
    }

    #[test]
    fn get_minimal_separators6() {
        let g = undirected(&[
            &[1, 1],
            &[1, 1],
            &[1, 2],
            &[2, 3],
            &[2, 3],
            &[2, 3],
            &[2, 5],
            &[3, 3],
            &[3, 4],
            &[5, 3],
            &[5, 3],
            &[5, 4],
        ]);
        check(&g, &[(&[2], 1), (&[3, 5], 1)]);
    }
}

// WeakChordalityInspectorTest

mod weak_chordality_inspector_test {
    use super::*;
    use jgrapht::alg::cycle::WeakChordalityInspector;
    use jgrapht::generate::named::{self, build};
    use jgrapht::generate::{Complete, GraphGenerator};

    /// A vertex-edge graph with parallel edges ignored (`TestUtil.createUndirected`).
    fn undirected(edges: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::with_loops().weighted();
        for e in edges {
            g.add_vertex(e[0]);
            g.add_vertex(e[1]);
            g.add_edge(&e[0], &e[1]).unwrap();
        }
        g
    }

    fn is_hole<G: Graph>(graph: &G, path: &GraphWalk<G::Vertex, G::Edge>) -> bool {
        let vertices = path.vertex_list();
        if vertices.len() < 6 || vertices[0] != vertices[vertices.len() - 1] {
            return false;
        }
        for i in 0..vertices.len() - 1 {
            if !graph.contains_edge_between(&vertices[i], &vertices[i + 1]) {
                return false;
            }
        }
        for i in 0..vertices.len() - 2 {
            for j in 0..vertices.len() - 2 {
                if i.abs_diff(j) > 1 && graph.contains_edge_between(&vertices[i], &vertices[j]) {
                    return false;
                }
            }
        }
        true
    }

    fn is_anti_hole<G: Graph>(graph: &G, path: &GraphWalk<G::Vertex, G::Edge>) -> bool {
        let vertices = path.vertex_list();
        if vertices.len() < 6 || vertices[0] != vertices[vertices.len() - 1] {
            return false;
        }
        for i in 0..vertices.len() - 1 {
            if graph.contains_edge_between(&vertices[i], &vertices[i + 1]) {
                return false;
            }
        }
        for i in 0..vertices.len() - 2 {
            for j in 0..vertices.len() - 2 {
                if i.abs_diff(j) > 1 && !graph.contains_edge_between(&vertices[i], &vertices[j]) {
                    return false;
                }
            }
        }
        true
    }

    fn check<G: IncidenceGraph>(graph: &G, weakly_chordal: bool) {
        let mut inspector = WeakChordalityInspector::new(graph);
        assert_eq!(weakly_chordal, inspector.is_weakly_chordal());
        let certificate = inspector.get_certificate();
        if weakly_chordal {
            assert!(certificate.is_none());
        } else {
            let path = certificate.expect("a certificate");
            assert!(is_hole(graph, path) || is_anti_hole(graph, path));
        }
    }

    #[test]
    fn is_weakly_chordal1() {
        check(&UGraph::<i32>::with_loops(), true);
    }

    #[test]
    fn is_weakly_chordal2() {
        check(
            &undirected(&[&[1, 2], &[1, 3], &[2, 3], &[2, 4], &[3, 4]]),
            true,
        );
    }

    #[test]
    fn is_weakly_chordal3() {
        check(&undirected(&[&[1, 2], &[1, 3], &[2, 4], &[3, 4]]), true);
    }

    #[test]
    fn is_weakly_chordal4() {
        check(
            &undirected(&[&[1, 2], &[2, 3], &[3, 4], &[4, 5], &[5, 1]]),
            false,
        );
    }

    #[test]
    fn is_weakly_chordal5() {
        check(
            &undirected(&[
                &[1, 3],
                &[1, 4],
                &[1, 5],
                &[1, 6],
                &[2, 4],
                &[2, 5],
                &[2, 6],
                &[2, 7],
                &[3, 5],
                &[3, 6],
                &[3, 7],
                &[4, 6],
                &[4, 7],
                &[5, 7],
            ]),
            false,
        );
    }

    #[test]
    fn is_weakly_chordal6() {
        check(
            &undirected(&[
                &[1, 1],
                &[1, 1],
                &[1, 2],
                &[1, 2],
                &[1, 2],
                &[1, 3],
                &[2, 4],
                &[2, 4],
                &[2, 4],
                &[3, 4],
                &[4, 4],
                &[4, 4],
                &[4, 4],
            ]),
            true,
        );
    }

    #[test]
    fn is_weakly_chordal7() {
        check(
            &undirected(&[
                &[1, 2],
                &[1, 3],
                &[2, 4],
                &[2, 7],
                &[2, 8],
                &[2, 10],
                &[2, 5],
                &[3, 5],
                &[3, 6],
                &[4, 7],
                &[5, 8],
                &[5, 9],
                &[5, 6],
                &[6, 9],
                &[7, 8],
                &[7, 10],
                &[8, 9],
                &[8, 10],
                &[9, 10],
            ]),
            false,
        );
    }

    #[test]
    fn is_weakly_chordal8() {
        let mut graph = UGraph::<i32>::with_loops();
        let bound = 100;
        for i in 0..bound {
            graph.add_vertex(i);
            graph.add_vertex(i + 1);
            graph.add_edge(&i, &(i + 1)).unwrap();
        }
        graph.add_edge(&0, &bound).unwrap();
        check(&graph, false);
    }

    #[test]
    fn is_weakly_chordal9() {
        let mut graph = UGraph::<i32>::with_loops().with_vertex_factory(|i| i as i32);
        Complete(50).generate_default(&mut graph).unwrap();
        check(&graph, true);
    }

    #[test]
    fn is_weakly_chordal10() {
        check(&build(named::dodecahedron()), false);
    }

    #[test]
    fn is_weakly_chordal11() {
        check(&build(named::bull()), true);
    }

    #[test]
    fn is_weakly_chordal12() {
        check(&build(named::bucky_ball()), false);
    }

    #[test]
    fn is_weakly_chordal13() {
        check(&build(named::clebsch()), false);
    }

    #[test]
    fn is_weakly_chordal14() {
        check(&build(named::grotzsch()), false);
    }

    #[test]
    fn is_weakly_chordal15() {
        check(&build(named::bidiakis_cube()), false);
    }

    #[test]
    fn is_weakly_chordal16() {
        check(&build(named::blanusa_first_snark()), false);
    }

    #[test]
    fn is_weakly_chordal17() {
        check(&build(named::double_star_snark()), false);
    }

    #[test]
    fn is_weakly_chordal18() {
        check(&build(named::brinkmann()), false);
    }

    #[test]
    fn is_weakly_chordal19() {
        check(&build(named::gosset()), false);
    }

    #[test]
    fn is_weakly_chordal20() {
        check(&build(named::chvatal()), false);
    }

    #[test]
    fn is_weakly_chordal21() {
        check(&build(named::kittell()), false);
    }

    #[test]
    fn is_weakly_chordal22() {
        check(&build(named::coxeter()), false);
    }

    #[test]
    fn is_weakly_chordal23() {
        check(&build(named::ellingham_horton78()), false);
    }

    #[test]
    fn is_weakly_chordal24() {
        check(&build(named::errera()), false);
    }

    #[test]
    fn is_weakly_chordal25() {
        check(&build(named::folkman()), false);
    }

    #[test]
    fn is_weakly_chordal26() {
        check(&build(named::krackhardt_kite()), true);
    }
}

// BergeGraphInspectorTest

mod berge_graph_inspector_test {
    use super::*;
    use jgrapht::alg::cycle::BergeGraphInspector;
    use jgrapht::generate::named::{self, build};
    use jgrapht::view::AsSubgraph;
    use std::collections::HashSet;

    /// The complement of `g` (an edge for every non-adjacent pair of distinct vertices).
    fn complement_of<G: Graph>(g: &G) -> UGraph<G::Vertex> {
        let vs: Vec<G::Vertex> = g.vertices().cloned().collect();
        let mut c = UGraph::<G::Vertex>::simple();
        c.add_vertices(vs.iter().cloned());
        for i in 0..vs.len() {
            for j in i + 1..vs.len() {
                if !g.contains_edge_between(&vs[i], &vs[j]) {
                    c.add_edge(&vs[i], &vs[j]).unwrap();
                }
            }
        }
        c
    }

    /// `verifyCertificate`: the certificate spans an odd cycle without chords, in the graph or in
    /// its complement.
    fn verify_certificate<G: IncidenceGraph>(g: &G, dut: &BergeGraphInspector<G::Vertex, G::Edge>) -> bool {
        let Some(certificate) = dut.get_certificate() else {
            return false;
        };
        let set: Vec<G::Vertex> = certificate.vertex_list().to_vec();
        let check = |subg: &dyn Fn() -> (usize, usize, bool)| {
            let (vertices, edges, all_degree_two) = subg();
            vertices == edges && vertices % 2 == 1 && all_degree_two
        };
        if dut.certificate_is_in_complement() {
            let comp = complement_of(g);
            let subg = AsSubgraph::induced(&comp, set);
            check(&|| {
                (
                    subg.vertex_count(),
                    subg.edge_count(),
                    subg.vertices().all(|t| subg.edges_of(t).count() == 2),
                )
            })
        } else {
            let subg = AsSubgraph::induced(g, set);
            check(&|| {
                (
                    subg.vertex_count(),
                    subg.edge_count(),
                    subg.vertices().all(|t| subg.edges_of(t).count() == 2),
                )
            })
        }
    }

    fn graph() -> UGraph<i32> {
        UGraph::<i32>::simple()
    }

    #[test]
    fn check_pyramid() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&2, &3).unwrap();
        stimulus.add_edge(&3, &1).unwrap();
        stimulus.add_vertex(4);
        stimulus.add_vertex(5);
        stimulus.add_vertex(6);
        stimulus.add_vertex(7);
        stimulus.add_edge(&4, &7).unwrap();
        stimulus.add_edge(&5, &7).unwrap();
        stimulus.add_edge(&6, &7).unwrap();
        stimulus.add_vertex(8);
        stimulus.add_vertex(9);
        stimulus.add_vertex(10);
        stimulus.add_vertex(11);
        stimulus.add_vertex(12);
        stimulus.add_vertex(13);
        stimulus.add_vertex(14);
        stimulus.add_vertex(15);
        stimulus.add_vertex(16);
        stimulus.add_edge(&8, &11).unwrap();
        stimulus.add_edge(&11, &4).unwrap();
        stimulus.add_edge(&9, &12).unwrap();
        stimulus.add_edge(&12, &5).unwrap();
        stimulus.add_edge(&10, &13).unwrap();
        stimulus.add_edge(&13, &6).unwrap();
        stimulus.add_edge(&1, &14).unwrap();
        stimulus.add_edge(&14, &8).unwrap();
        stimulus.add_edge(&2, &15).unwrap();
        stimulus.add_edge(&15, &9).unwrap();
        stimulus.add_edge(&3, &16).unwrap();
        stimulus.add_edge(&16, &10).unwrap();
        assert!(dut.contains_pyramid(&stimulus).unwrap());
        dut.is_berge_with_certificate(&stimulus, true).unwrap();
        assert!(verify_certificate(&stimulus, &dut));
        stimulus.add_edge(&8, &2).unwrap();
        dut.is_berge_with_certificate(&stimulus, true).unwrap();
        assert!(verify_certificate(&stimulus, &dut));
        stimulus.remove_edge_between(&8, &2);
        stimulus.add_edge(&9, &3).unwrap();
        dut.is_berge_with_certificate(&stimulus, true).unwrap();
        assert!(verify_certificate(&stimulus, &dut));
        stimulus.remove_edge_between(&9, &3);
        stimulus.add_edge(&10, &1).unwrap();
        dut.is_berge_with_certificate(&stimulus, true).unwrap();
        assert!(verify_certificate(&stimulus, &dut));
        stimulus.add_edge(&11, &2).unwrap();
        assert!(!dut.contains_pyramid(&stimulus).unwrap());
    }

    #[test]
    fn check_jewel() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_vertex(5);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&2, &3).unwrap();
        stimulus.add_edge(&3, &4).unwrap();
        stimulus.add_edge(&4, &5).unwrap();
        stimulus.add_edge(&5, &1).unwrap();
        stimulus.add_vertex(6);
        stimulus.add_vertex(7);
        stimulus.add_vertex(8);
        stimulus.add_edge(&1, &6).unwrap();
        stimulus.add_edge(&6, &7).unwrap();
        stimulus.add_edge(&7, &8).unwrap();
        stimulus.add_edge(&8, &4).unwrap();
        assert!(dut.contains_jewel(&stimulus).unwrap());
        dut.is_berge_with_certificate(&stimulus, true).unwrap();
        assert!(verify_certificate(&stimulus, &dut));
        stimulus.add_edge(&1, &3).unwrap();
        assert!(!dut.contains_jewel(&stimulus).unwrap());
    }

    #[test]
    fn check_is_y_x_complete() {
        let mut stimulus = graph();
        let dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_edge(&1, &4).unwrap();
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&1, &3).unwrap();
        let mut x: HashSet<i32> = HashSet::new();
        x.insert(2);
        x.insert(3);
        x.insert(4);
        assert!(dut.is_yx_complete(&stimulus, &1, &x));
        stimulus.remove_edge_between(&1, &4);
        assert!(!dut.is_yx_complete(&stimulus, &1, &x));
        stimulus.add_edge(&1, &4).unwrap();
        x.clear();
        x.insert(2);
        x.insert(1);
        assert!(!dut.is_yx_complete(&stimulus, &3, &x));
    }

    #[test]
    fn check_configuration_type_2() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_vertex(5);
        stimulus.add_vertex(6);
        stimulus.add_vertex(7);
        stimulus.add_vertex(8);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&2, &3).unwrap();
        stimulus.add_edge(&3, &4).unwrap();
        stimulus.add_edge(&1, &6).unwrap();
        stimulus.add_edge(&2, &6).unwrap();
        stimulus.add_edge(&4, &6).unwrap();
        stimulus.add_edge(&1, &5).unwrap();
        stimulus.add_edge(&5, &7).unwrap();
        stimulus.add_edge(&7, &8).unwrap();
        stimulus.add_edge(&4, &8).unwrap();
        assert!(dut.has_configuration_type2(&stimulus).unwrap());
        stimulus.add_edge(&3, &6).unwrap();
        assert!(dut.has_configuration_type2(&stimulus).unwrap());
        stimulus.add_edge(&7, &6).unwrap();
        assert!(!dut.has_configuration_type2(&stimulus).unwrap());
        stimulus.remove_edge_between(&3, &6);
        stimulus.remove_edge_between(&4, &8);
        assert!(!dut.has_configuration_type2(&stimulus).unwrap());
    }

    #[test]
    fn check_configuration_type_3() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_vertex(5);
        stimulus.add_vertex(6);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&3, &4).unwrap();
        stimulus.add_edge(&1, &4).unwrap();
        stimulus.add_edge(&2, &3).unwrap();
        stimulus.add_edge(&3, &5).unwrap();
        stimulus.add_edge(&4, &6).unwrap();
        stimulus.add_vertex(7);
        stimulus.add_edge(&1, &7).unwrap();
        stimulus.add_edge(&2, &7).unwrap();
        stimulus.add_edge(&5, &7).unwrap();
        stimulus.add_vertex(8);
        stimulus.add_vertex(9);
        stimulus.add_vertex(10);
        stimulus.add_edge(&5, &8).unwrap();
        stimulus.add_edge(&8, &9).unwrap();
        stimulus.add_edge(&9, &10).unwrap();
        stimulus.add_edge(&10, &6).unwrap();
        assert!(dut.has_configuration_type3(&stimulus).unwrap());
        stimulus.add_edge(&4, &7).unwrap();
        assert!(!dut.has_configuration_type3(&stimulus).unwrap());
    }

    #[test]
    fn check_clean_odd_hole() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_vertex(5);
        stimulus.add_vertex(6);
        stimulus.add_vertex(7);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&3, &2).unwrap();
        stimulus.add_edge(&4, &3).unwrap();
        stimulus.add_edge(&4, &5).unwrap();
        stimulus.add_edge(&6, &5).unwrap();
        stimulus.add_edge(&6, &7).unwrap();
        stimulus.add_edge(&7, &1).unwrap();
        assert!(dut.contains_clean_shortest_odd_hole(&stimulus).unwrap());
        stimulus.add_edge(&3, &7).unwrap();
        stimulus.add_edge(&4, &7).unwrap();
        assert!(!dut.contains_clean_shortest_odd_hole(&stimulus).unwrap());
    }

    #[test]
    fn check_contains_shortest_odd_hole() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_vertex(5);
        stimulus.add_vertex(6);
        stimulus.add_vertex(7);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&3, &2).unwrap();
        stimulus.add_edge(&4, &3).unwrap();
        stimulus.add_edge(&4, &5).unwrap();
        stimulus.add_edge(&6, &5).unwrap();
        stimulus.add_edge(&6, &7).unwrap();
        stimulus.add_edge(&7, &1).unwrap();
        stimulus.add_vertex(8);
        stimulus.add_edge(&3, &8).unwrap();
        stimulus.add_edge(&8, &7).unwrap();
        stimulus.add_edge(&8, &5).unwrap();
        assert!(dut.contains_clean_shortest_odd_hole(&stimulus).unwrap());
        stimulus.remove_vertex(&8);
        assert!(dut.contains_clean_shortest_odd_hole(&stimulus).unwrap());
    }

    #[test]
    fn check_routine_3() {
        let mut stimulus = graph();
        let dut = BergeGraphInspector::new();
        stimulus.add_vertex(1);
        stimulus.add_vertex(2);
        stimulus.add_vertex(3);
        stimulus.add_vertex(4);
        stimulus.add_edge(&1, &2).unwrap();
        stimulus.add_edge(&2, &3).unwrap();
        stimulus.add_edge(&2, &4).unwrap();
        stimulus.add_edge(&1, &3).unwrap();
        stimulus.add_edge(&1, &4).unwrap();
        let golden: HashSet<Vec<i32>> = [vec![1, 2], vec![1, 2, 3, 4]].into_iter().collect();
        let actual: HashSet<Vec<i32>> = dut
            .routine3(&stimulus)
            .unwrap()
            .into_iter()
            .map(|mut set| {
                set.sort_unstable();
                set
            })
            .collect();
        assert_eq!(golden, actual);
    }

    #[test]
    fn check_petersen_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::petersen());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_dodecahedron_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::dodecahedron());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_bull_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::bull());
        assert!(dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(!verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_butterfly_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::butterfly());
        assert!(dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(!verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_claw_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::claw());
        assert!(dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(!verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_groetzsch_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::grotzsch());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_diamond_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::diamond());
        assert!(dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(!verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_frucht_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::frucht());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_moser_spindle_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::moser_spindle());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_tietze_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::tietze());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_thomsen_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::thomsen());
        assert!(dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(!verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_tutte_graph() {
        let mut dut = BergeGraphInspector::new();
        let stimulus = build(named::tutte());
        assert!(!dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(verify_certificate(&stimulus, &dut));
    }

    #[test]
    fn check_empty_graph() {
        let mut stimulus = graph();
        let mut dut = BergeGraphInspector::new();
        let number_of_vertices = 15; // Java draws it from 14..17
        stimulus.add_vertices(0..number_of_vertices);
        assert!(dut.is_berge_with_certificate(&stimulus, true).unwrap());
        assert!(!verify_certificate(&stimulus, &dut));
    }
}
