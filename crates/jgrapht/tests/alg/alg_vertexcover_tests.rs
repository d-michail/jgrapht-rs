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

//! Ports of the `org.jgrapht.alg.vertexcover` tests: the two-approximation tests (Bar-Yehuda–Even,
//! Clarkson, edge based), GreedyVCImplTest and the exact tests of RecursiveExactVCImpl. The
//! exact-solver graphs are extracted from the Java sources into `vertex_cover_data.rs`.

use crate::vertex_cover_data::{UNWEIGHTED, WEIGHTED};
use jgrapht::alg::cover::*;
use jgrapht::alg::interfaces::VertexCoverAlgorithm;
use jgrapht::generate::*;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Mutex;

const TEST_GRAPH_SIZE: usize = 200;
const TEST_REPEATS: usize = 20;

static RANDOM: Mutex<Option<StdRng>> = Mutex::new(None);

fn random_int(bound: i32) -> i32 {
    let mut guard = RANDOM.lock().unwrap();
    guard
        .get_or_insert_with(|| StdRng::seed_from_u64(0_u64))
        .random_range(0..bound)
}

fn is_cover<G: Graph>(g: &G, cover: &VertexCover<G::Vertex>) -> bool {
    g.edges().all(|e| {
        let e = e.borrow();
        {
            cover.vertices.contains(g.edge_source(e).unwrap())
                || cover.vertices.contains(g.edge_target(e).unwrap())
        }
    })
}

fn create_random_pseudo_graph(vertices: usize) -> UGraph<i32> {
    let mut g = UGraph::<i32>::pseudo().with_vertex_factory(|i| i as i32);
    let m = random_int((vertices / 2) as i32) as usize + 1;
    GnmRandomGraphGenerator::new(vertices, m, StdRng::from_os_rng(), false, false)
        .unwrap()
        .generate_default(&mut g)
        .unwrap();
    g
}

fn random_vertex_weights(g: &UGraph<i32>) -> HashMap<i32, f64> {
    g.vertices().map(|&v| (v, random_int(25) as f64)).collect()
}

fn graph_from(edges: &[(i32, i32)]) -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}

macro_rules! two_approx_tests {
    ($module:ident, $solver:ident, weighted = $weighted:tt) => {
        mod $module {
            use super::*;

            #[test]
            fn test_find_2_approximation_cover() {
                for _ in 0..TEST_REPEATS {
                    let g = create_random_pseudo_graph(TEST_GRAPH_SIZE);
                    let cover = $solver::new(&g).unwrap().vertex_cover().unwrap();
                    assert!(is_cover(&g, &cover));
                    assert_eq!(cover.weight, cover.vertices.len() as f64);
                }
            }

            #[test]
            fn test_find_2_approximation_cover_2() {
                for _ in 0..TEST_REPEATS {
                    let g = create_random_pseudo_graph(70);
                    let optimal = RecursiveExactVertexCover::new(&g).unwrap().vertex_cover().unwrap();
                    let cover = $solver::new(&g).unwrap().vertex_cover().unwrap();
                    assert!(is_cover(&g, &cover));
                    assert_eq!(cover.weight, cover.vertices.len() as f64);
                    assert!(cover.weight <= optimal.weight * 2.0);
                }
            }

            two_approx_tests!(@weighted $weighted, $solver);
        }
    };
    (@weighted true, $solver:ident) => {
        #[test]
        fn test_find_2_approximation_weighted_cover() {
            for _ in 0..TEST_REPEATS {
                let g = create_random_pseudo_graph(TEST_GRAPH_SIZE);
                let weights = random_vertex_weights(&g);
                let cover = $solver::with_weights(&g, weights.clone()).unwrap().vertex_cover().unwrap();
                assert!(is_cover(&g, &cover));
                assert_eq!(cover.weight, cover.vertices.iter().map(|v| weights[v]).sum::<f64>());
            }
        }
    };
    (@weighted false, $solver:ident) => {};
}

two_approx_tests!(
    bar_yehuda_even_two_approx_vertex_cover_test,
    BarYehudaEvenTwoApproxVertexCover,
    weighted = true
);
two_approx_tests!(
    clarkson_two_approx_vertex_cover_test,
    ClarksonTwoApproxVertexCover,
    weighted = true
);
two_approx_tests!(
    edge_based_two_approx_vertex_cover_test,
    EdgeBasedTwoApproxVertexCover,
    weighted = false
);

mod greedy_vertex_cover_test {
    use super::*;

    #[test]
    fn test_find_greedy_cover() {
        for _ in 0..TEST_REPEATS {
            let g = create_random_pseudo_graph(TEST_GRAPH_SIZE);
            let cover = GreedyVertexCover::new(&g).unwrap().vertex_cover().unwrap();
            assert!(is_cover(&g, &cover));
            assert_eq!(cover.weight, cover.vertices.len() as f64);
        }
    }

    #[test]
    fn test_find_greedy_weighted_cover() {
        for _ in 0..TEST_REPEATS {
            let g = create_random_pseudo_graph(TEST_GRAPH_SIZE);
            let weights = random_vertex_weights(&g);
            let cover = GreedyVertexCover::with_weights(&g, weights.clone())
                .unwrap()
                .vertex_cover()
                .unwrap();
            assert!(is_cover(&g, &cover));
            assert_eq!(
                cover.weight,
                cover.vertices.iter().map(|v| weights[v]).sum::<f64>()
            );
        }
    }
}

mod recursive_exact_vertex_cover_test {
    use super::*;

    fn solve(g: &UGraph<i32>) -> VertexCover<i32> {
        RecursiveExactVertexCover::new(g)
            .unwrap()
            .vertex_cover()
            .unwrap()
    }

    fn check(edges: &[(i32, i32)], vertices: &[i32], expected: f64) {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(vertices.iter().copied());
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        let cover = solve(&g);
        assert!(is_cover(&g, &cover));
        assert_eq!(cover.weight, expected);
    }

    #[test]
    fn test_4_cycle() {
        check(&[(0, 1), (1, 2), (2, 3), (3, 0)], &[0, 1, 2, 3], 2.0);
    }

    #[test]
    fn test_wheel() {
        check(
            &[
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 1),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
            ],
            &[0, 1, 2, 3, 4, 5, 6, 7],
            5.0,
        );
    }

    #[test]
    fn test_cubic() {
        check(
            &[
                (0, 1),
                (0, 9),
                (0, 7),
                (1, 2),
                (1, 5),
                (2, 3),
                (2, 4),
                (3, 4),
                (3, 5),
                (4, 11),
                (5, 6),
                (6, 7),
                (6, 8),
                (7, 8),
                (8, 10),
                (9, 10),
                (9, 11),
                (10, 11),
            ],
            &(0..12).collect::<Vec<_>>(),
            7.0,
        );
    }

    #[test]
    fn test_whisker() {
        check(
            &[(0, 2), (1, 2), (2, 3), (3, 4), (3, 5)],
            &[0, 1, 2, 3, 4, 5],
            2.0,
        );
    }

    #[test]
    fn test_exact_minimum_cover_unweighted() {
        for (i, case) in UNWEIGHTED.iter().enumerate() {
            let g = graph_from(case.edges);
            let cover = solve(&g);
            assert!(is_cover(&g, &cover), "case {}", i + 1);
            assert_eq!(cover.weight, case.expected, "case {}", i + 1);
        }
    }

    #[test]
    fn test_exact_minimum_cover_weighted() {
        for (i, case) in WEIGHTED.iter().enumerate() {
            let g = graph_from(case.edges);
            let weights: HashMap<i32, f64> = case
                .weights
                .unwrap()
                .iter()
                .enumerate()
                .map(|(v, &w)| (v as i32, w as f64))
                .collect();
            let cover = RecursiveExactVertexCover::with_weights(&g, weights)
                .unwrap()
                .vertex_cover()
                .unwrap();
            assert!(is_cover(&g, &cover), "case {}", i + 1);
            assert_eq!(cover.weight, case.expected, "case {}", i + 1);
        }
    }
}
