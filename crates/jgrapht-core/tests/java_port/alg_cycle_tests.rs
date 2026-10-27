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
