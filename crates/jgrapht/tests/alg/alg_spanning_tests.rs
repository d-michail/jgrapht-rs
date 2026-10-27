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

//! Ports of the `org.jgrapht.alg.spanning` tests: MinimumSpanningTreeTest (Kruskal, Prim,
//! Boruvka), GreedyMultiplicativeSpannerTest and the capacitated spanning tree tests.

use indexmap::IndexSet;
use jgrapht::alg::capacitated::*;
use jgrapht::alg::spanner::GreedyMultiplicativeSpanner;
use jgrapht::alg::spanning::*;
use jgrapht::alg::util::Pair;
use jgrapht::generate::*;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

fn s(x: &str) -> String {
    x.to_string()
}

// ---------------------------------------------------------------------------------------------
// MinimumSpanningTreeTest
// ---------------------------------------------------------------------------------------------

mod minimum_spanning_tree {
    use super::*;

    type W = UGraph<i32>;
    const A: i32 = 'A' as i32;
    const B: i32 = 'B' as i32;
    const C: i32 = 'C' as i32;
    const D: i32 = 'D' as i32;
    const E: i32 = 'E' as i32;
    const F: i32 = 'F' as i32;
    const G: i32 = 'G' as i32;
    const H: i32 = 'H' as i32;

    fn add(g: &mut W, a: i32, b: i32, w: f64) -> DefaultEdge {
        let e = g.add_edge(&a, &b).unwrap().unwrap();
        g.set_edge_weight(&e, w).unwrap();
        e
    }

    fn disconnected() -> (W, Vec<DefaultEdge>) {
        let mut g = W::simple().weighted();
        g.add_vertices([A, B, C, D]);
        let ab = add(&mut g, A, B, 5.0);
        let ac = add(&mut g, A, C, 10.0);
        let bd = add(&mut g, B, D, 15.0);
        add(&mut g, C, D, 20.0);
        g.add_vertices([E, F, G, H]);
        add(&mut g, E, F, 20.0);
        let eg = add(&mut g, E, G, 15.0);
        let gh = add(&mut g, G, H, 10.0);
        let fh = add(&mut g, F, H, 5.0);
        (g, vec![ab, ac, bd, eg, gh, fh])
    }

    fn connected() -> (W, Vec<DefaultEdge>) {
        let mut g = W::simple().weighted();
        g.add_vertices([A, B, C, D, E]);
        let ab = add(&mut g, A, B, 2.0);
        let ac = add(&mut g, A, C, 3.0);
        let bd = add(&mut g, B, D, 5.0);
        add(&mut g, C, D, 20.0);
        let de = add(&mut g, D, E, 5.0);
        add(&mut g, A, E, 100.0);
        (g, vec![ab, ac, bd, de])
    }

    fn check(mst: SpanningTree<DefaultEdge>, edges: &[DefaultEdge], weight: f64) {
        assert_eq!(mst.weight, weight);
        assert!(edges.iter().all(|e| mst.edges.contains(e)));
    }

    macro_rules! mst_tests {
        ($module:ident, $solver:ident, $other:ident) => {
            mod $module {
                use super::*;

                #[test]
                fn test_simple_disconnected_weighted_graph() {
                    let (g, edges) = disconnected();
                    check($solver::new(&g).spanning_tree(), &edges, 60.0);
                }

                #[test]
                fn test_simple_connected_weighted_graph() {
                    let (g, edges) = connected();
                    check($solver::new(&g).spanning_tree(), &edges, 15.0);
                }

                #[test]
                fn test_random_instances() {
                    let mut rng = StdRng::seed_from_u64(33);
                    let mut weights = StdRng::seed_from_u64(34);
                    for _ in 0..100 {
                        let mut g = UGraph::<i32>::pseudo()
                            .weighted()
                            .with_vertex_factory(|i| i as i32);
                        GnpRandomGraphGenerator::new(
                            200,
                            0.5,
                            StdRng::seed_from_u64(rng.random()),
                            false,
                        )
                        .unwrap()
                        .generate_default(&mut g)
                        .unwrap();
                        let edges: Vec<_> = g.edges().cloned().collect();
                        for e in edges {
                            g.set_edge_weight(&e, weights.random::<f64>()).unwrap();
                        }
                        let a = $solver::new(&g).spanning_tree().weight;
                        let b = $other::new(&g).spanning_tree().weight;
                        assert!((a - b).abs() < 1e-9);
                    }
                }
            }
        };
    }

    mst_tests!(kruskal, KruskalMinimumSpanningTree, PrimMinimumSpanningTree);
    mst_tests!(prim, PrimMinimumSpanningTree, KruskalMinimumSpanningTree);
    mst_tests!(
        boruvka,
        BoruvkaMinimumSpanningTree,
        KruskalMinimumSpanningTree
    );
}

// ---------------------------------------------------------------------------------------------
// GreedyMultiplicativeSpannerTest
// ---------------------------------------------------------------------------------------------

mod greedy_multiplicative_spanner {
    use super::*;

    fn v(i: usize) -> String {
        format!("v{i}")
    }

    fn graph1() -> UGraph<String> {
        let mut g = UGraph::<String>::pseudo();
        g.add_vertices((0..8).map(v));
        for (a, b) in [
            (0, 1),
            (1, 2),
            (0, 5),
            (1, 5),
            (1, 4),
            (2, 4),
            (2, 3),
            (5, 4),
            (4, 3),
            (5, 6),
            (4, 6),
            (3, 6),
            (3, 7),
            (6, 7),
        ] {
            g.add_edge(&v(a), &v(b)).unwrap();
        }
        g
    }

    fn graph2() -> UGraph<String> {
        let mut g = UGraph::<String>::pseudo();
        g.add_vertices((0..16).map(v));
        for (a, b) in [
            (0, 1),
            (0, 3),
            (0, 2),
            (1, 3),
            (1, 2),
            (3, 2),
            (3, 4),
            (2, 5),
            (4, 5),
            (4, 6),
            (5, 7),
            (6, 7),
            (8, 9),
            (8, 10),
            (8, 11),
            (9, 10),
            (9, 11),
            (10, 11),
            (10, 12),
            (11, 13),
            (12, 14),
            (13, 15),
        ] {
            g.add_edge(&v(a), &v(b)).unwrap();
        }
        g
    }

    fn graph3() -> UGraph<String> {
        let mut g = UGraph::<String>::pseudo().weighted();
        g.add_vertices((0..8).map(v));
        for (a, b, w) in [
            (0, 1, 3.0),
            (1, 2, 15.0),
            (0, 5, 5.0),
            (1, 5, 20.0),
            (1, 4, 100.0),
            (2, 4, 5.0),
            (2, 3, 10.0),
            (5, 4, 1.0),
            (4, 3, 100.0),
            (5, 6, 10.0),
            (4, 6, 20.0),
            (3, 6, 100.0),
            (3, 7, 1000.0),
            (6, 7, 5.0),
        ] {
            let e = g.add_edge(&v(a), &v(b)).unwrap().unwrap();
            g.set_edge_weight(&e, w).unwrap();
        }
        g
    }

    fn run_test(g: &UGraph<String>, k: usize, correct: &[(usize, usize)]) {
        let result = GreedyMultiplicativeSpanner::new(g, k)
            .unwrap()
            .spanner()
            .unwrap();
        let want: HashSet<DefaultEdge> = correct
            .iter()
            .map(|&(a, b)| *g.get_edge(&v(a), &v(b)).unwrap())
            .collect();
        assert_eq!(want.len(), result.edges.len());
        assert!(want.iter().all(|e| result.edges.contains(e)));
    }

    #[test]
    fn test_graph_1() {
        let g = graph1();
        run_test(
            &g,
            2,
            &[
                (0, 1),
                (1, 2),
                (0, 5),
                (1, 4),
                (2, 3),
                (5, 6),
                (4, 6),
                (3, 6),
                (3, 7),
            ],
        );
        run_test(
            &g,
            3,
            &[
                (0, 1),
                (1, 2),
                (0, 5),
                (1, 4),
                (2, 3),
                (5, 6),
                (3, 7),
                (6, 7),
            ],
        );
        let spanner7 = [(0, 1), (1, 2), (0, 5), (1, 4), (2, 3), (5, 6), (3, 7)];
        run_test(&g, 4, &spanner7);
        run_test(&g, 100, &spanner7);
    }

    #[test]
    fn test_graph_1_with_loops() {
        let mut g = graph1();
        for i in 0..3 {
            g.add_edge(&v(i), &v(i)).unwrap();
        }
        run_test(
            &g,
            2,
            &[
                (0, 1),
                (1, 2),
                (0, 5),
                (1, 4),
                (2, 3),
                (5, 6),
                (4, 6),
                (3, 6),
                (3, 7),
            ],
        );
    }

    #[test]
    fn test_graph_1_with_multiple_edges() {
        let mut g = graph1();
        g.add_edge(&v(0), &v(1)).unwrap();
        g.add_edge(&v(1), &v(2)).unwrap();
        g.add_edge(&v(0), &v(5)).unwrap();
        run_test(
            &g,
            2,
            &[
                (0, 1),
                (1, 2),
                (0, 5),
                (1, 4),
                (2, 3),
                (5, 6),
                (4, 6),
                (3, 6),
                (3, 7),
            ],
        );
    }

    #[test]
    fn test_graph_2() {
        let g = graph2();
        let tail: &[(usize, usize)] = &[
            (8, 9),
            (8, 10),
            (8, 11),
            (10, 12),
            (11, 13),
            (12, 14),
            (13, 15),
        ];
        let with = |head: &[(usize, usize)]| [head, tail].concat();
        run_test(
            &g,
            2,
            &with(&[
                (0, 1),
                (0, 3),
                (0, 2),
                (3, 4),
                (2, 5),
                (4, 5),
                (4, 6),
                (5, 7),
            ]),
        );
        run_test(
            &g,
            3,
            &with(&[
                (0, 1),
                (0, 3),
                (0, 2),
                (3, 4),
                (2, 5),
                (4, 6),
                (5, 7),
                (6, 7),
            ]),
        );
        let spanner7 = with(&[(0, 1), (0, 3), (0, 2), (3, 4), (2, 5), (4, 6), (5, 7)]);
        run_test(&g, 4, &spanner7);
        run_test(&g, 100, &spanner7);
    }

    #[test]
    fn test_graph_3() {
        let g = graph3();
        let spanner3 = [(5, 4), (0, 1), (0, 5), (2, 4), (2, 3), (5, 6), (6, 7)];
        run_test(&g, 2, &spanner3);
        // minimum spanning tree
        run_test(&g, i32::MAX as usize, &spanner3);
    }

    #[test]
    fn test_negative_weights_graph() {
        let mut g = UGraph::<String>::pseudo().weighted();
        g.add_vertices((0..3).map(v));
        for (a, b, w) in [(0, 1, 1.0), (1, 2, -1.0), (2, 0, 1.0)] {
            let e = g.add_edge(&v(a), &v(b)).unwrap().unwrap();
            g.set_edge_weight(&e, w).unwrap();
        }
        assert!(
            GreedyMultiplicativeSpanner::new(&g, 2)
                .unwrap()
                .spanner()
                .is_err()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// EsauWilliamsCapacitatedMinimumSpanningTreeTest
// ---------------------------------------------------------------------------------------------

mod capacitated_common {
    use super::*;

    pub type WG = UGraph<i32>;

    /// Complete graph on `n` vertices; `weights` lists `(i, j, w)`.
    pub fn complete(n: i32, weights: &[(i32, i32, f64)], skip: Option<(i32, i32)>) -> WG {
        let mut g = WG::simple().weighted();
        g.add_vertices(0..n);
        for i in 0..n {
            for j in i + 1..n {
                if Some((i, j)) != skip {
                    g.add_edge(&i, &j).unwrap();
                }
            }
        }
        for &(i, j, w) in weights {
            let e = *g.get_edge(&i, &j).unwrap();
            g.set_edge_weight(&e, w).unwrap();
        }
        g
    }

    pub fn demands(items: &[(i32, f64)]) -> HashMap<i32, f64> {
        items.iter().copied().collect()
    }

    pub fn part(c: &CapacitatedSpanningTree<i32, DefaultEdge>, v: i32) -> Pair<IndexSet<i32>, f64> {
        c.partition[&c.labels[&v]].clone()
    }

    pub fn pair(set: &[i32], w: f64) -> Pair<IndexSet<i32>, f64> {
        Pair::of(set.iter().copied().collect(), w)
    }

    pub fn only_edges(
        g: &WG,
        c: &CapacitatedSpanningTree<i32, DefaultEdge>,
        allowed: &[(i32, i32)],
    ) {
        let allowed: HashSet<DefaultEdge> = allowed
            .iter()
            .map(|(a, b)| *g.get_edge(a, b).unwrap())
            .collect();
        for e in &c.edges {
            assert!(allowed.contains(e));
        }
    }

    pub const INSTANCE_1: [(i32, i32, f64); 21] = [
        (0, 1, 5.0),
        (0, 2, 6.0),
        (0, 3, 9.0),
        (0, 4, 10.0),
        (0, 5, 11.0),
        (0, 6, 15.0),
        (1, 2, 9.0),
        (1, 3, 6.0),
        (1, 4, 6.0),
        (1, 5, 8.0),
        (1, 6, 17.0),
        (2, 3, 7.0),
        (2, 4, 9.0),
        (2, 5, 8.0),
        (2, 6, 12.0),
        (3, 4, 10.0),
        (3, 5, 5.0),
        (3, 6, 11.0),
        (4, 5, 14.0),
        (4, 6, 9.0),
        (5, 6, 8.0),
    ];
    pub const INSTANCE_2: [(i32, i32, f64); 15] = [
        (0, 1, 7.0),
        (0, 2, 5.0),
        (0, 3, 1.0),
        (0, 4, 2.0),
        (0, 5, 8.0),
        (1, 2, 8.0),
        (1, 3, 5.0),
        (1, 4, 2.0),
        (1, 5, 2.0),
        (2, 3, 2.0),
        (2, 4, 5.0),
        (2, 5, 6.0),
        (3, 4, 9.0),
        (3, 5, 5.0),
        (4, 5, 1.0),
    ];
}

mod esau_williams {
    use super::capacitated_common::*;
    use super::*;

    fn esau_williams<'g>(
        g: &'g WG,
        capacity: f64,
        demands: HashMap<i32, f64>,
        ops: usize,
    ) -> Result<EsauWilliamsCapacitatedMinimumSpanningTree<'g, WG>> {
        EsauWilliamsCapacitatedMinimumSpanningTree::new(g, 0, capacity, demands, ops)
    }

    #[test]
    fn test_instance_1() {
        let g = complete(7, &INSTANCE_1, None);
        let d = demands(&[(1, 1.0), (2, 1.0), (3, 2.0), (4, 1.0), (5, 1.0), (6, 1.0)]);
        let c = esau_williams(&g, 3.0, d, 1)
            .unwrap()
            .capacitated_spanning_tree()
            .unwrap();
        assert!((c.weight - 42.0).abs() < 1e-7);
        assert_eq!(part(&c, 1), pair(&[1, 4], 2.0));
        assert_eq!(part(&c, 2), pair(&[2, 5, 6], 3.0));
        assert_eq!(part(&c, 3), pair(&[3], 2.0));
        assert_eq!(c.labels[&1], c.labels[&4]);
        assert_eq!(c.labels[&2], c.labels[&5]);
        assert_eq!(c.labels[&2], c.labels[&6]);
        assert_ne!(c.labels[&1], c.labels[&2]);
        assert_ne!(c.labels[&1], c.labels[&3]);
        assert_ne!(c.labels[&2], c.labels[&3]);
        only_edges(&g, &c, &[(0, 1), (0, 2), (0, 3), (1, 4), (2, 5), (5, 6)]);
    }

    fn check_instance_2(g: &WG) {
        let d = demands(&[(1, 2.0), (2, 1.0), (3, 2.0), (4, 3.0), (5, 2.0)]);
        let c = esau_williams(g, 4.0, d, 1)
            .unwrap()
            .capacitated_spanning_tree()
            .unwrap();
        assert!((c.weight - 14.0).abs() < 1e-7);
        assert_eq!(part(&c, 1), pair(&[1, 5], 4.0));
        assert_eq!(part(&c, 2), pair(&[2, 3], 3.0));
        assert_eq!(part(&c, 4), pair(&[4], 3.0));
        assert_eq!(c.labels[&1], c.labels[&5]);
        assert_eq!(c.labels[&2], c.labels[&3]);
        assert_ne!(c.labels[&1], c.labels[&3]);
        assert_ne!(c.labels[&1], c.labels[&4]);
        assert_ne!(c.labels[&3], c.labels[&4]);
        only_edges(g, &c, &[(0, 1), (0, 3), (0, 4), (1, 5), (2, 3)]);
    }

    #[test]
    fn test_instance_2() {
        check_instance_2(&complete(6, &INSTANCE_2, None));
    }

    /// Instance 2 without edge (0, 5): an incomplete graph.
    #[test]
    fn test_instance_3() {
        let weights: Vec<_> = INSTANCE_2
            .iter()
            .copied()
            .filter(|&(i, j, _)| (i, j) != (0, 5))
            .collect();
        check_instance_2(&complete(6, &weights, Some((0, 5))));
    }

    #[test]
    fn test_instance_with_randomness() {
        let g = complete(7, &INSTANCE_1, None);
        let d = demands(&[(1, 1.0), (2, 1.0), (3, 2.0), (4, 1.0), (5, 1.0), (6, 1.0)]);
        for _ in 0..30 {
            let c = esau_williams(&g, 3.0, d.clone(), 20)
                .unwrap()
                .capacitated_spanning_tree()
                .unwrap();
            assert!(c.is_capacitated_spanning_tree(&g, &0, 3.0, &d));
        }
    }

    #[test]
    fn test_unconnected_graph() {
        let mut g = UGraph::<i32>::with_loops();
        g.add_vertices([0, 1]);
        let d = demands(&[(0, 1.0), (1, 1.0)]);
        assert!(esau_williams(&g, 30.0, d, 1).is_err());
    }

    #[test]
    fn test_violated_capacity_constraint() {
        let mut g = UGraph::<i32>::with_loops();
        g.add_vertices([0, 1]);
        g.add_edge(&0, &1).unwrap();
        let d = demands(&[(0, 1.0), (1, 1.0)]);
        assert!(esau_williams(&g, -1.0, d, 1).is_err());
    }

    #[test]
    fn test_invalid_graph() {
        let mut g = UGraph::<i32>::with_loops();
        g.add_vertices(0..5);
        for i in 0..4 {
            g.add_edge(&i, &(i + 1)).unwrap();
        }
        let d = demands(&[(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)]);
        let mut alg = esau_williams(&g, 2.0, d, 1).unwrap();
        assert!(alg.capacitated_spanning_tree().is_err());
    }
}

// ---------------------------------------------------------------------------------------------
// AhujaOrlinSharmaCapacitatedMinimumSpanningTreeTest
// ---------------------------------------------------------------------------------------------

mod ahuja_orlin_sharma {
    use super::capacitated_common::*;
    use super::*;

    type Tree = CapacitatedSpanningTree<i32, DefaultEdge>;

    fn weighted(n: i32, edges: &[(i32, i32, f64)]) -> WG {
        let mut g = WG::simple().weighted();
        g.add_vertices(0..n);
        for &(a, b, w) in edges {
            let e = g.add_edge(&a, &b).unwrap().unwrap();
            g.set_edge_weight(&e, w).unwrap();
        }
        g
    }

    /// A starting solution from labels, partition members and the tree edges.
    fn initial(
        g: &WG,
        labels: &[(i32, usize)],
        parts: &[(usize, &[i32], f64)],
        edges: &[(i32, i32)],
        weight: f64,
    ) -> Tree {
        CapacitatedSpanningTree {
            labels: labels.iter().copied().collect(),
            partition: parts.iter().map(|&(l, set, w)| (l, pair(set, w))).collect(),
            edges: edges
                .iter()
                .map(|(a, b)| *g.get_edge(a, b).unwrap())
                .collect(),
            weight,
        }
    }

    fn options(
        best_improvement: bool,
        ops: usize,
        vertex: bool,
        subtree: bool,
        tabu: bool,
        tabu_time: usize,
        limit: usize,
    ) -> AhujaOrlinSharmaOptions {
        AhujaOrlinSharmaOptions {
            best_improvement,
            number_of_operations: ops,
            use_vertex_operation: vertex,
            use_subtree_operation: subtree,
            use_tabu_search: tabu,
            tabu_time,
            upper_limit_tabu_exchanges: limit,
        }
    }

    fn solve_from(
        start: Tree,
        g: &WG,
        capacity: f64,
        d: &HashMap<i32, f64>,
        length_bound: usize,
    ) -> Tree {
        AhujaOrlinSharmaCapacitatedMinimumSpanningTree::with_initial_solution(
            start,
            g,
            0,
            capacity,
            d.clone(),
            length_bound,
            options(false, 0, true, true, false, 0, 0),
        )
        .unwrap()
        .capacitated_spanning_tree()
        .unwrap()
    }

    fn unit_demands(range: std::ops::Range<i32>) -> HashMap<i32, f64> {
        range.map(|i| (i, 1.0)).collect()
    }

    #[test]
    fn test_instance_1() {
        let g = weighted(
            7,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (1, 4, 2.0),
                (1, 5, 1.0),
                (2, 4, 1.0),
                (2, 5, 2.0),
                (3, 6, 1.0),
            ],
        );
        let d = unit_demands(1..7);
        let start = initial(
            &g,
            &[(1, 0), (2, 1), (3, 2), (4, 0), (5, 1), (6, 2)],
            &[(0, &[1, 4], 2.0), (1, &[2, 5], 2.0), (2, &[3, 6], 2.0)],
            &[(0, 1), (0, 2), (0, 3), (1, 4), (2, 5), (3, 6)],
            8.0,
        );
        let c = solve_from(start, &g, 2.0, &d, 2);
        assert!(c.is_capacitated_spanning_tree(&g, &0, 2.0, &d));
        assert!((c.weight - 6.0).abs() < 1e-7);
        assert_eq!(part(&c, 1), pair(&[1, 5], 2.0));
        assert_eq!(part(&c, 2), pair(&[2, 4], 2.0));
        assert_eq!(part(&c, 3), pair(&[3, 6], 2.0));
        assert_eq!(c.labels[&1], c.labels[&5]);
        assert_eq!(c.labels[&2], c.labels[&4]);
        assert_eq!(c.labels[&3], c.labels[&6]);
        assert_ne!(c.labels[&1], c.labels[&2]);
        assert_ne!(c.labels[&1], c.labels[&3]);
        assert_ne!(c.labels[&2], c.labels[&3]);
        only_edges(&g, &c, &[(0, 1), (0, 2), (0, 3), (1, 5), (2, 4), (3, 6)]);
    }

    fn check_instance_2(c: &Tree, g: &WG) {
        assert!((c.weight - 14.0).abs() < 1e-7);
        assert_eq!(part(c, 1), pair(&[1, 5], 4.0));
        assert_eq!(part(c, 2), pair(&[2, 3], 3.0));
        assert_eq!(part(c, 4), pair(&[4], 3.0));
        assert_eq!(c.labels[&1], c.labels[&5]);
        assert_eq!(c.labels[&2], c.labels[&3]);
        assert_ne!(c.labels[&1], c.labels[&3]);
        assert_ne!(c.labels[&1], c.labels[&4]);
        assert_ne!(c.labels[&3], c.labels[&4]);
        only_edges(g, c, &[(0, 1), (0, 3), (0, 4), (1, 5), (3, 2)]);
    }

    #[test]
    fn test_instance_2() {
        let g = complete(6, &INSTANCE_2, None);
        let d = demands(&[(1, 2.0), (2, 1.0), (3, 2.0), (4, 3.0), (5, 2.0)]);
        let c = AhujaOrlinSharmaCapacitatedMinimumSpanningTree::with_options(
            &g,
            0,
            4.0,
            d.clone(),
            7,
            options(false, 1, true, true, false, 0, 0),
        )
        .unwrap()
        .capacitated_spanning_tree()
        .unwrap();
        assert!(c.is_capacitated_spanning_tree(&g, &0, 4.0, &d));
        check_instance_2(&c, &g);
    }

    #[test]
    fn test_instance_3() {
        let g = weighted(
            6,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (0, 4, 1.0),
                (1, 5, 1.0),
                (2, 5, 3.0),
                (3, 5, 2.0),
                (4, 5, 0.0),
            ],
        );
        let d = demands(&[(1, 1.0), (2, 1.0), (3, 1.0), (4, 2.0), (5, 1.0)]);
        let start = initial(
            &g,
            &[(1, 0), (2, 1), (3, 2), (4, 3), (5, 1)],
            &[
                (0, &[1], 1.0),
                (1, &[2, 5], 2.0),
                (2, &[3], 1.0),
                (3, &[4], 2.0),
            ],
            &[(0, 1), (0, 2), (0, 3), (0, 4), (2, 5)],
            7.0,
        );
        let c = solve_from(start, &g, 2.0, &d, 2);
        assert!(c.is_capacitated_spanning_tree(&g, &0, 2.0, &d));
        assert!((c.weight - 5.0).abs() < 1e-7);
        assert_eq!(part(&c, 1), pair(&[1, 5], 2.0));
        assert_eq!(part(&c, 2), pair(&[2], 1.0));
        assert_eq!(part(&c, 3), pair(&[3], 1.0));
        assert_eq!(part(&c, 4), pair(&[4], 2.0));
        assert_eq!(c.labels[&1], c.labels[&5]);
        for (a, b) in [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)] {
            assert_ne!(c.labels[&a], c.labels[&b]);
        }
    }

    /// The initial solution should not be changed.
    #[test]
    fn test_instance_4() {
        let g = weighted(
            8,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (1, 4, 1.0),
                (2, 5, 1.0),
                (3, 6, 1.0),
                (4, 7, 1.0),
                (5, 7, 3.0),
                (6, 7, 2.0),
            ],
        );
        let d = unit_demands(1..8);
        let start = initial(
            &g,
            &[(1, 0), (2, 1), (3, 2), (4, 0), (5, 1), (6, 2), (7, 1)],
            &[(0, &[1, 4], 2.0), (1, &[2, 5, 7], 3.0), (2, &[3, 6], 2.0)],
            &[(0, 1), (0, 2), (0, 3), (1, 4), (2, 5), (3, 6), (5, 7)],
            7.0,
        );
        let c = solve_from(start, &g, 3.0, &d, 3);
        assert!(c.is_capacitated_spanning_tree(&g, &0, 3.0, &d));
        assert!((c.weight - 7.0).abs() < 1e-7);
        assert_eq!(part(&c, 1), pair(&[1, 4, 7], 3.0));
        assert_eq!(part(&c, 2), pair(&[2, 5], 2.0));
        assert_eq!(part(&c, 3), pair(&[3, 6], 2.0));
        only_edges(
            &g,
            &c,
            &[(0, 1), (0, 2), (0, 3), (1, 4), (2, 5), (3, 6), (4, 7)],
        );
    }

    /// A simple subtree exchange.
    #[test]
    fn test_instance_5() {
        let g = weighted(
            9,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (1, 4, 1.0),
                (2, 5, 1.0),
                (3, 6, 1.0),
                (4, 7, 1.0),
                (5, 7, 3.0),
                (6, 7, 2.0),
                (7, 8, 1.0),
            ],
        );
        let d = unit_demands(1..9);
        let start = initial(
            &g,
            &[
                (1, 0),
                (2, 1),
                (3, 2),
                (4, 0),
                (5, 1),
                (6, 2),
                (7, 1),
                (8, 1),
            ],
            &[
                (0, &[1, 4], 2.0),
                (1, &[2, 5, 7, 8], 4.0),
                (2, &[3, 6], 2.0),
            ],
            &[
                (0, 1),
                (0, 2),
                (0, 3),
                (1, 4),
                (2, 5),
                (3, 6),
                (5, 7),
                (7, 8),
            ],
            8.0,
        );
        let c = solve_from(start, &g, 4.0, &d, 3);
        assert!(c.is_capacitated_spanning_tree(&g, &0, 4.0, &d));
        assert!((c.weight - 8.0).abs() < 1e-7);
        assert_eq!(part(&c, 1), pair(&[1, 4, 7, 8], 4.0));
        assert_eq!(part(&c, 2), pair(&[2, 5], 2.0));
        assert_eq!(part(&c, 3), pair(&[3, 6], 2.0));
    }

    #[test]
    fn test_instance_6() {
        let g = complete(6, &INSTANCE_2, None);
        let d = demands(&[(1, 2.0), (2, 1.0), (3, 2.0), (4, 3.0), (5, 2.0)]);
        let start = initial(
            &g,
            &[(1, 0), (2, 1), (3, 2), (4, 3), (5, 4)],
            &[
                (0, &[1], 2.0),
                (1, &[2], 1.0),
                (2, &[3], 2.0),
                (3, &[4], 3.0),
                (4, &[5], 2.0),
            ],
            &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
            8.0,
        );
        let c = solve_from(start, &g, 4.0, &d, 7);
        assert!(c.is_capacitated_spanning_tree(&g, &0, 4.0, &d));
        check_instance_2(&c, &g);
    }

    /// The pseudo random graph of `generateComplicatedTestExample`.
    fn complicated(n: i32) -> (WG, HashMap<i32, f64>) {
        let mut g = WG::simple().weighted();
        let mut d = HashMap::new();
        for i in 0..n {
            g.add_vertex(i);
            d.insert(i, ((i * i * i) % 5) as f64 + 1.0);
        }
        for i in 0..n {
            for j in i + 1..n {
                let w = match (i + j) % 5 {
                    1 => Some((7 * i + 5 * j) % 183),
                    2 => Some((i * j) % 253),
                    3 => Some((11 * i + 17 * j) % 193),
                    _ => None,
                };
                if let Some(w) = w {
                    let e = g.add_edge(&i, &j).unwrap().unwrap();
                    g.set_edge_weight(&e, w as f64).unwrap();
                }
            }
        }
        (g, d)
    }

    fn run_complicated(opts: AhujaOrlinSharmaOptions) {
        let (g, d) = complicated(40);
        let capacity = 30.0;
        let c = AhujaOrlinSharmaCapacitatedMinimumSpanningTree::with_options(
            &g,
            0,
            capacity,
            d.clone(),
            7,
            opts,
        )
        .unwrap()
        .capacitated_spanning_tree()
        .unwrap();
        assert!(c.is_capacitated_spanning_tree(&g, &0, capacity, &d));
    }

    #[test]
    fn test_instance_vertex() {
        run_complicated(options(false, 1, true, false, false, 0, 0));
    }

    #[test]
    fn test_instance_vertex_tabu() {
        run_complicated(options(false, 1, true, false, true, 10, 15));
    }

    #[test]
    fn test_instance_subtree() {
        run_complicated(options(false, 1, false, true, false, 0, 0));
    }

    #[test]
    fn test_instance_subtree_tabu() {
        run_complicated(options(false, 1, false, true, true, 10, 15));
    }

    #[test]
    fn test_instance_vertex_and_subtree() {
        run_complicated(options(false, 1, true, true, false, 0, 0));
    }

    #[test]
    fn test_instance_vertex_and_subtree_tabu() {
        run_complicated(options(false, 1, true, true, true, 10, 50));
    }

    #[test]
    fn test_instance_vertex_best_improvement() {
        run_complicated(options(true, 1, true, false, false, 0, 0));
    }

    #[test]
    fn test_instance_vertex_tabu_best_improvement() {
        run_complicated(options(true, 1, true, false, true, 10, 15));
    }

    #[test]
    fn test_unconnected_graph() {
        let (mut g, mut d) = complicated(20);
        g.add_vertex(50);
        d.insert(50, 1.0);
        assert!(
            AhujaOrlinSharmaCapacitatedMinimumSpanningTree::with_options(
                &g,
                0,
                30.0,
                d,
                7,
                options(true, 1, true, false, true, 10, 15)
            )
            .is_err()
        );
    }

    #[test]
    fn test_violated_capacity_constraint() {
        let (g, d) = complicated(10);
        assert!(
            AhujaOrlinSharmaCapacitatedMinimumSpanningTree::with_options(
                &g,
                0,
                -1.0,
                d,
                7,
                options(true, 1, true, false, true, 10, 15)
            )
            .is_err()
        );
    }
}
