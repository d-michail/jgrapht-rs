//! Ports of the `org.jgrapht.alg.cycle` tests: the Ahuja–Orlin–Sharma cyclic exchange, the simple
//! cycle enumerations (DirectedSimpleCyclesTest, JohnsonSimpleCyclesTest, HawickJamesSimpleCyclesTest),
//! CycleDetectorTest and CyclesTest.

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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut graph = UGraph::<i32>::pseudo();
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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut cycles: Vec<Vec<DefaultEdge>> = cb.cycles().iter().cloned().collect();
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
        let mut graph = UGraph::<i32>::pseudo();
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
