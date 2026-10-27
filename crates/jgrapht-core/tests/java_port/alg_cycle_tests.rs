//! Ports of the `org.jgrapht.alg.cycle` tests (so far the Ahuja–Orlin–Sharma cyclic exchange).

use jgrapht::alg::cyclic_exchange::AhujaOrlinSharmaCyclicExchangeLocalAugmentation;
use jgrapht::*;
use std::collections::HashMap;

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
