//! Ports of the small `org.jgrapht.alg` packages: transform (LineGraphConverterTest), independentset,
//! steiner, partition (BipartitePartitioningTest), centers (GonAlgorithmTest) and densesubgraph
//! (the Goldberg tests).

use crate::common::*;
use jgrapht::alg::centers::GonHeuristic;
use jgrapht::alg::independent_set::ChordalGraphIndependentSetFinder;
use jgrapht::alg::interfaces::{CentersLocationAlgorithm, Partitioning, PartitioningAlgorithm};
use jgrapht::alg::partition::BipartitePartitioning;
use jgrapht::alg::steiner::kmb_steiner_tree;
use jgrapht::alg::transform::line_graph_into;
use jgrapht::generate::*;
use jgrapht::graph::props;
use jgrapht::graph::{Directed, Undirected};
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

mod line_graph_converter_test {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices([0, 1, 2, 3]);
        let mut target = UGraph::<DefaultEdge>::simple().weighted();
        line_graph_into(&g, &mut target).unwrap();
        assert!(props::is_empty(&g));
    }

    #[test]
    fn test_star_graph() {
        let mut star = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
        Star(5).generate_default(&mut star).unwrap();
        let mut target = UGraph::<DefaultEdge>::simple();
        line_graph_into(&star, &mut target).unwrap();
        assert!(props::is_complete(&target));
    }

    fn vertex_set_is_edge_set<G: Graph<Edge = DefaultEdge>, T: Graph<Vertex = DefaultEdge>>(
        g: &G,
        target: &T,
    ) {
        assert_eq!(
            target.vertices().collect::<HashSet<_>>(),
            g.edges().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_undirected_graph() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices([1, 2, 3, 4, 5]);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let (e12, e25, e54, e41, e43, e13) = (
            add(1, 2),
            add(2, 5),
            add(5, 4),
            add(4, 1),
            add(4, 3),
            add(1, 3),
        );
        let mut target = UGraph::<DefaultEdge>::simple();
        line_graph_into(&g, &mut target).unwrap();
        vertex_set_is_edge_set(&g, &target);
        assert_eq!(target.edge_count(), 9);
        for (a, b) in [
            (&e12, &e25),
            (&e25, &e54),
            (&e54, &e43),
            (&e43, &e13),
            (&e12, &e13),
            (&e41, &e12),
            (&e41, &e54),
            (&e41, &e43),
            (&e41, &e13),
        ] {
            assert!(target.contains_edge_between(a, b));
        }
    }

    #[test]
    fn test_directed_graph() {
        let mut g = DiGraph::<i32>::simple();
        g.add_vertices([1, 2, 3, 4]);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let (e12, e14, e23, e31, e32, e34, e43) = (
            add(1, 2),
            add(1, 4),
            add(2, 3),
            add(3, 1),
            add(3, 2),
            add(3, 4),
            add(4, 3),
        );
        let mut target = DiGraph::<DefaultEdge>::simple();
        line_graph_into(&g, &mut target).unwrap();
        vertex_set_is_edge_set(&g, &target);
        assert_eq!(target.edge_count(), 12);
        for (a, b) in [
            (&e12, &e23),
            (&e23, &e32),
            (&e23, &e34),
            (&e23, &e31),
            (&e32, &e23),
            (&e31, &e12),
            (&e31, &e14),
            (&e34, &e43),
            (&e43, &e34),
            (&e43, &e32),
            (&e43, &e31),
            (&e14, &e43),
        ] {
            assert!(target.contains_edge_between(a, b));
        }
    }

    #[test]
    fn self_loop_test_undirected() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3]);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let (e12, e23, e31, e22) = (add(1, 2), add(2, 3), add(3, 1), add(2, 2));
        let mut target = UGraph::<DefaultEdge>::simple();
        line_graph_into(&g, &mut target).unwrap();
        vertex_set_is_edge_set(&g, &target);
        assert_eq!(target.edge_count(), 5);
        for (a, b) in [
            (&e12, &e23),
            (&e12, &e31),
            (&e23, &e31),
            (&e12, &e22),
            (&e22, &e23),
        ] {
            assert!(target.contains_edge_between(a, b));
        }
    }

    #[test]
    fn self_loop_test_directed() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3]);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let (e12, e23, e31, e22) = (add(1, 2), add(2, 3), add(3, 1), add(2, 2));
        let mut target = DiGraph::<DefaultEdge>::pseudo();
        line_graph_into(&g, &mut target).unwrap();
        vertex_set_is_edge_set(&g, &target);
        assert_eq!(target.edge_count(), 6);
        for (a, b) in [
            (&e12, &e23),
            (&e23, &e31),
            (&e31, &e12),
            (&e22, &e22),
            (&e12, &e22),
            (&e22, &e23),
        ] {
            assert!(target.contains_edge_between(a, b));
        }
    }
}

mod chordal_graph_independent_set_finder_test {
    use super::*;

    fn undirected(edges: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple().weighted();
        construct_graph(&mut g, edges);
        g
    }

    #[test]
    fn test_get_maximum_independent_set_1() {
        let graph = UGraph::<i32>::simple();
        assert_eq!(ChordalGraphIndependentSetFinder::new(&graph).independent_set().unwrap().len(), 0);
    }

    #[test]
    fn test_get_maximum_independent_set_2() {
        let graph = undirected(&[&[1, 2], &[1, 3], &[1, 4], &[2, 3], &[2, 4], &[3, 4]]);
        assert_eq!(ChordalGraphIndependentSetFinder::new(&graph).independent_set().unwrap().len(), 1);
    }

    #[test]
    fn test_get_maximum_independent_set_3() {
        let graph = undirected(&[&[1, 2], &[1, 3], &[2, 4], &[3, 4]]);
        assert!(ChordalGraphIndependentSetFinder::new(&graph).independent_set().is_none());
    }

    #[test]
    fn test_get_maximum_independent_set_4() {
        let mut graph = UGraph::<i32>::pseudo().weighted();
        construct_graph(
            &mut graph,
            &[
                &[1, 1],
                &[1, 2],
                &[1, 2],
                &[2, 3],
                &[2, 3],
                &[1, 3],
                &[3, 3],
                &[3, 4],
                &[3, 4],
                &[4, 4],
                &[4, 4],
                &[4, 5],
                &[4, 5],
            ],
        );
        let set: Vec<i32> = ChordalGraphIndependentSetFinder::new(&graph)
            .independent_set()
            .unwrap()
            .into_iter()
            .collect();
        assert_eq!(set.len(), 2);
        for (i, a) in set.iter().enumerate() {
            for b in &set[..i] {
                assert!(!graph.contains_edge_between(a, b));
            }
        }
    }
}

mod kou_markowsky_berman_algorithm_test {
    use super::*;

    #[test]
    fn test_returns_original_edges() {
        let mut graph = UGraph::<&str>::simple().weighted();
        graph.add_vertices(["a", "b", "c"]);
        let ab = graph.add_edge(&"a", &"b").unwrap().unwrap();
        let bc = graph.add_edge(&"b", &"c").unwrap().unwrap();
        graph.set_edge_weight(&ab, 2.0).unwrap();
        graph.set_edge_weight(&bc, 3.0).unwrap();
        let tree = kmb_steiner_tree(&graph, &["a", "c"]).unwrap();
        assert_eq!(tree.weight, 5.0);
        assert_eq!(
            tree.edges.iter().collect::<HashSet<_>>(),
            HashSet::from([&ab, &bc])
        );
    }

    #[test]
    fn test_uses_shortest_path_parallel_edge() {
        let mut graph = UGraph::<&str>::multi().weighted();
        graph.add_vertices(["a", "b"]);
        let expensive = graph.add_edge(&"a", &"b").unwrap().unwrap();
        let cheap = graph.add_edge(&"a", &"b").unwrap().unwrap();
        graph.set_edge_weight(&expensive, 10.0).unwrap();
        graph.set_edge_weight(&cheap, 1.0).unwrap();
        let tree = kmb_steiner_tree(&graph, &["a", "b"]).unwrap();
        assert_eq!(tree.weight, 1.0);
        assert_eq!(tree.edges, vec![cheap]);
    }

    fn set_edge_with_weight(g: &mut UGraph<String>, a: &str, b: &str, w: f64) {
        g.add_vertex(a.to_string());
        g.add_vertex(b.to_string());
        if let Some(e) = g.add_edge(&a.to_string(), &b.to_string()).unwrap() {
            g.set_edge_weight(&e, w).unwrap();
        }
    }

    fn tree_vertices(g: &UGraph<String>, edges: &[DefaultEdge]) -> HashSet<String> {
        edges
            .iter()
            .flat_map(|e| {
                [
                    g.edge_source(e).unwrap().clone(),
                    g.edge_target(e).unwrap().clone(),
                ]
            })
            .collect()
    }

    #[test]
    fn test_example_graph_steiner_tree() {
        let mut g = UGraph::<String>::simple().weighted();
        g.add_vertices((1..=9).map(|i| format!("v{i}")));
        for (a, b, w) in [
            ("v1", "v2", 10.0),
            ("v2", "v3", 8.0),
            ("v3", "v4", 9.0),
            ("v4", "v5", 2.0),
            ("v3", "v5", 2.0),
            ("v6", "v5", 1.0),
            ("v9", "v5", 1.0),
            ("v2", "v6", 1.0),
            ("v6", "v7", 1.0),
            ("v1", "v9", 1.0),
            ("v7", "v8", 0.5),
            ("v8", "v9", 0.5),
        ] {
            set_edge_with_weight(&mut g, a, b, w);
        }
        let terminals: Vec<String> = ["v1", "v2", "v3", "v4"].map(String::from).to_vec();
        let tree = kmb_steiner_tree(&g, &terminals).unwrap();
        assert!((tree.weight - 8.0).abs() < 0.001);
        let vs = tree_vertices(&g, &tree.edges);
        for t in &terminals {
            assert!(vs.contains(t), "Missing terminal: {t}");
        }
        assert_eq!(vs.len() - 1, tree.edges.len());
    }

    #[test]
    fn test_random_graph_steiner_tree() {
        let mut g = UGraph::<String>::simple()
            .weighted()
            .with_vertex_factory(|i| i.to_string());
        GnpRandomGraphGenerator::new(25, 0.5, StdRng::from_os_rng(), false)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        let mut rand = StdRng::from_os_rng();
        let edges: Vec<DefaultEdge> = g.edges().cloned().collect();
        for e in &edges {
            let w =
                rand.random_range(0..10) as f64 + (1.0 + rand.random_range(0..10) as f64) / 10.0;
            g.set_edge_weight(e, w).unwrap();
        }
        let mut shuffled: Vec<String> = g.vertices().cloned().collect();
        jgrapht::util::java::java_shuffle(&mut shuffled, &mut StdRng::from_os_rng());
        let selected: Vec<String> = shuffled[..10].to_vec();
        let tree = kmb_steiner_tree(&g, &selected).unwrap();
        let vs = tree_vertices(&g, &tree.edges);
        for v in &selected {
            assert!(vs.contains(v), "Missing terminal: {v}");
        }
        assert_eq!(vs.len() - 1, tree.edges.len());
    }
}

mod bipartite_partitioning_test {
    use super::*;

    fn check(g: &impl Graph<Vertex = i32>) -> bool {
        BipartitePartitioning::new(g).is_bipartite()
    }

    #[test]
    fn test_bipartite_10() {
        let mut g = UGraph::<i32>::pseudo();
        assert!(props::is_bipartite(&g));
        g.add_vertex(1);
        assert!(props::is_bipartite(&g));
        g.add_vertex(2);
        assert!(props::is_bipartite(&g));
        g.add_edge(&1, &2).unwrap();
        assert!(props::is_bipartite(&g));
        g.add_vertex(3);
        assert!(props::is_bipartite(&g));
        g.add_edge(&2, &3).unwrap();
        assert!(props::is_bipartite(&g));
        g.add_edge(&3, &1).unwrap();
        assert!(!props::is_bipartite(&g));
    }

    fn cycle<D: graph::Direction>(g: &mut BaseGraph<i32, DefaultEdge, D>, n: i32) {
        for i in 0..n {
            g.add_vertex(i);
            if i > 0 {
                g.add_edge(&i, &(i - 1)).unwrap();
            }
        }
        g.add_edge(&(n - 1), &0).unwrap();
    }

    #[test]
    fn test_bipartite_20() {
        let mut g = UGraph::<i32>::pseudo();
        cycle(&mut g, 100);
        assert!(props::is_bipartite(&g));
    }

    #[test]
    fn test_bipartite_30() {
        let mut g = UGraph::<i32>::pseudo();
        cycle(&mut g, 101);
        assert!(!props::is_bipartite(&g));
    }

    #[test]
    fn test_bipartite_40() {
        let mut g = DiGraph::<i32>::pseudo();
        cycle(&mut g, 101);
        assert!(!props::is_bipartite(&g));
    }

    #[test]
    fn test_random_bipartite() {
        let mut generator =
            GnpRandomBipartiteGraphGenerator::new(10, 10, 0.8, StdRng::from_os_rng()).unwrap();
        for _ in 0..100 {
            let mut g = UGraph::<i32>::pseudo().with_vertex_factory(|i| i as i32);
            generator.generate_default(&mut g).unwrap();
            assert!(props::is_bipartite(&g));
        }
    }

    fn check_is_bipartite_partition<D: graph::Direction>(mut g: BaseGraph<i32, DefaultEdge, D>) {
        g.add_vertices([1, 2, 3, 4]);
        let mut a: HashSet<i32> = HashSet::from([1, 2]);
        let b: HashSet<i32> = HashSet::from([3, 4]);
        assert!(props::is_bipartite_partition(&g, &a, &b));
        for (x, y) in [(1, 3), (1, 4), (1, 3), (2, 3), (2, 4), (4, 1), (3, 1)] {
            g.add_edge(&x, &y).unwrap();
        }
        assert!(props::is_bipartite_partition(&g, &a, &b));
        a.remove(&1);
        assert!(!props::is_bipartite_partition(&g, &a, &b));
        a.insert(1);
        assert!(props::is_bipartite_partition(&g, &a, &b));
        let e11 = g.add_edge(&1, &1).unwrap().unwrap();
        assert!(!props::is_bipartite_partition(&g, &a, &b));
        g.remove_edge(&e11);
        assert!(props::is_bipartite_partition(&g, &a, &b));
        let e44 = g.add_edge(&4, &4).unwrap().unwrap();
        assert!(!props::is_bipartite_partition(&g, &a, &b));
        g.remove_edge(&e44);
        assert!(props::is_bipartite_partition(&g, &a, &b));
        g.add_edge(&4, &3).unwrap();
        assert!(!props::is_bipartite_partition(&g, &a, &b));
    }

    #[test]
    fn test_is_bipartite_partition() {
        check_is_bipartite_partition(UGraph::<i32>::pseudo());
        check_is_bipartite_partition(DiGraph::<i32>::pseudo());
    }

    fn int_graph() -> UGraph<i32> {
        UGraph::<i32>::simple().with_vertex_factory(|i| i as i32)
    }

    fn assert_bipartite(g: &UGraph<i32>) {
        let mut finder = BipartitePartitioning::new(g);
        assert!(finder.is_bipartite());
        let p = finder.partitioning().unwrap().clone();
        assert!(finder.is_valid_partitioning(&p));
    }

    fn assert_not_bipartite(g: &UGraph<i32>) {
        let mut finder = BipartitePartitioning::new(g);
        assert!(!finder.is_bipartite());
        assert!(finder.partitioning().is_none());
    }

    #[test]
    fn test_empty_graph() {
        assert_bipartite(&int_graph());
    }

    #[test]
    fn test_bipartite() {
        let mut random = StdRng::seed_from_u64((0x88) as u64);
        for _ in 0..100 {
            let mut graph = int_graph();
            let (a, b) = (
                1 + random.random_range(0..100) as usize,
                1 + random.random_range(0..200) as usize,
            );
            CompleteBipartite(a, b)
                .generate_default(&mut graph)
                .unwrap();
            assert_bipartite(&graph);
        }
    }

    #[test]
    fn test_bipartite_2() {
        let mut random = StdRng::seed_from_u64((0x88) as u64);
        for _ in 0..100 {
            let mut graph = int_graph();
            let n1 = 1 + random.random_range(0..100) as usize;
            let n2 = 1 + random.random_range(0..200) as usize;
            let m = 4 * n1 * n2 / 10;
            GnmRandomBipartiteGraphGenerator::new(n1, n2, m, StdRng::from_os_rng())
                .generate_default(&mut graph)
                .unwrap();
            assert_bipartite(&graph);
        }
    }

    #[test]
    fn test_star_graph() {
        let mut graph = int_graph();
        Star(100).generate_default(&mut graph).unwrap();
        assert_bipartite(&graph);
    }

    #[test]
    fn test_forest() {
        let mut random = StdRng::seed_from_u64((0x88) as u64);
        for _ in 0..100 {
            let mut graph = int_graph();
            let t = 10 + random.random_range(0..50) as usize;
            let n = 100 + random.random_range(0..200) as usize;
            BarabasiAlbertForestGenerator::new(t, n, StdRng::from_os_rng())
                .unwrap()
                .generate_default(&mut graph)
                .unwrap();
            assert_bipartite(&graph);
        }
    }

    #[test]
    fn test_complete() {
        let mut graph = int_graph();
        Complete(100).generate_default(&mut graph).unwrap();
        assert_not_bipartite(&graph);
    }

    #[test]
    fn test_even_cycle() {
        let mut graph = int_graph();
        Ring(100).generate_default(&mut graph).unwrap();
        assert_bipartite(&graph);
    }

    #[test]
    fn test_odd_cycle() {
        let mut graph = int_graph();
        Ring(101).generate_default(&mut graph).unwrap();
        assert_not_bipartite(&graph);
    }

    #[test]
    fn partitioning_accessors() {
        let p = Partitioning::new(vec![
            indexmap::IndexSet::from([1, 2]),
            indexmap::IndexSet::from([3]),
        ]);
        assert_eq!(p.number_partitions(), 2);
        assert_eq!(p.partitions().len(), 2);
        assert_eq!(p.partition(1).len(), 1);
        let _ = (
            check as fn(&UGraph<i32>) -> bool,
            std::marker::PhantomData::<(Directed, Undirected)>,
        );
    }
}

mod gon_algorithm_test {
    use super::*;
    use jgrapht::GraphError;

    fn weighted_undirected(edges: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple().weighted();
        construct_graph(&mut g, edges);
        g
    }

    fn from_matrix(dist: &[&[i32]]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple().weighted();
        for i in 0..dist.len() {
            for j in i + 1..dist.len() {
                g.add_vertex(i as i32);
                g.add_vertex(j as i32);
                let e = g.add_edge(&(i as i32), &(j as i32)).unwrap().unwrap();
                g.set_edge_weight(&e, dist[i][j] as f64).unwrap();
            }
        }
        g
    }

    const DUMMY5: [&[i32]; 5] = [
        &[0, 8, 10, 11, 15],
        &[8, 0, 2, 3, 7],
        &[10, 2, 0, 1, 5],
        &[11, 3, 1, 0, 4],
        &[15, 7, 5, 4, 0],
    ];

    fn gon() -> GonHeuristic<i32> {
        GonHeuristic::with_rng(StdRng::from_os_rng())
    }

    #[test]
    fn test_directed_graph() {
        let mut graph = DiGraph::<i32>::simple().weighted();
        graph.add_edge_with_vertices(1, 2).unwrap();
        assert!(gon().centers(&graph, 1).is_err());
    }

    #[test]
    fn test_empty_graph() {
        let graph = UGraph::<i32>::simple().weighted();
        assert!(gon().centers(&graph, 1).is_err());
    }

    #[test]
    fn test_no_complete_graph() {
        let mut graph = UGraph::<i32>::simple().weighted();
        graph.add_vertices([0, 1]);
        assert!(gon().centers(&graph, 1).is_err());
    }

    #[test]
    fn test_get_one_center() {
        let graph = weighted_undirected(&[&[1, 2, 5]]);
        let centers = gon().centers(&graph, 1).unwrap();
        assert_eq!(centers.len(), 1);
        assert!(centers.contains(&1) || centers.contains(&2));
        assert!(!centers.contains(&5));
    }

    #[test]
    fn test_get_two_centers() {
        let graph = weighted_undirected(&[&[1, 2, 5]]);
        let centers = gon().centers(&graph, 2).unwrap();
        assert_eq!(centers.len(), 2);
        assert!(centers.contains(&1) && centers.contains(&2));
    }

    #[test]
    fn test_get_more_centers() {
        let graph = weighted_undirected(&[&[1, 2, 5]]);
        assert!(gon().centers(&graph, 3).is_err());
    }

    #[test]
    fn test_get_one_center_with_min_cov_radius() {
        let graph = from_matrix(&[&[0, 5, 5], &[5, 0, 5], &[5, 5, 0]]);
        let mut g = gon();
        g.centers(&graph, 1).unwrap();
        assert_eq!(g.covering_radius().unwrap(), 5.0);
    }

    #[test]
    fn test_dummy_graph_5_two_centers() {
        let graph = from_matrix(&DUMMY5);
        let mut gon = GonHeuristic::with_initial_centers(HashSet::from([0]));
        let centers = gon.centers(&graph, 2).unwrap();
        assert!(centers.contains(&4));
        assert_eq!(gon.covering_radius().unwrap(), 7.0);
        let a = gon.assignment().unwrap();
        assert_eq!((a[&1], a[&2], a[&3]), (4, 4, 4));
    }

    #[test]
    fn test_dummy_graph_5_three_centers() {
        let graph = from_matrix(&DUMMY5);
        let mut gon = GonHeuristic::with_initial_centers(HashSet::from([0, 4]));
        let centers = gon.centers(&graph, 3).unwrap();
        assert!(centers.contains(&1));
        assert_eq!(gon.covering_radius().unwrap(), 3.0);
        let a = gon.assignment().unwrap();
        assert_eq!((a[&2], a[&3]), (1, 1));
    }

    #[test]
    fn test_dummy_graph_5_four_centers() {
        let graph = from_matrix(&DUMMY5);
        let mut gon = GonHeuristic::with_initial_centers(HashSet::from([0, 4, 1]));
        let centers = gon.centers(&graph, 4).unwrap();
        assert!(centers.contains(&3));
        assert_eq!(gon.covering_radius().unwrap(), 1.0);
        assert_eq!(gon.assignment().unwrap()[&2], 3);
    }

    #[test]
    fn test_early_assignment() {
        let gon = GonHeuristic::<i32>::with_initial_centers(HashSet::from([0, 4, 1]));
        assert!(matches!(gon.assignment(), Err(GraphError::Unsupported(_))));
        assert!(matches!(
            gon.covering_radius(),
            Err(GraphError::Unsupported(_))
        ));
    }

    #[test]
    fn test_kro_a200_five_centers() {
        let text = include_str!("fixtures/kroA200.tsp");
        let mut tokens = text.split_whitespace();
        let n = 200;
        let mut coords = Vec::new();
        for _ in 0..n {
            let _id: i32 = tokens.next().unwrap().parse().unwrap();
            let x: i32 = tokens.next().unwrap().parse().unwrap();
            let y: i32 = tokens.next().unwrap().parse().unwrap();
            coords.push((x, y));
        }
        let mut graph = UGraph::<i32>::simple().weighted();
        for i in 0..n {
            for j in i + 1..n {
                let (dx, dy) = (coords[i].0 - coords[j].0, coords[i].1 - coords[j].1);
                graph.add_vertex(i as i32);
                graph.add_vertex(j as i32);
                let e = graph.add_edge(&(i as i32), &(j as i32)).unwrap().unwrap();
                graph
                    .set_edge_weight(&e, ((dx * dx + dy * dy) as f64).sqrt())
                    .unwrap();
            }
        }
        let mut gon = gon();
        for (k, bound) in [(5, 911.41), (10, 598.81), (20, 389.30), (40, 258.25)] {
            gon.centers(&graph, k).unwrap();
            let r = gon.covering_radius().unwrap();
            assert!(r <= 2.0 * bound, "For k={k}, covering radius is {r}");
        }
    }
}

mod goldberg_maximum_density_subgraph_tests {
    use super::*;
    use jgrapht::alg::densest::GoldbergMaximumDensitySubgraph;
    use jgrapht::alg::util::Pair;
    use jgrapht::util::OrderedF64;
    use jgrapht::view::AsSubgraph;

    const EPS: f64 = 1e-5;

    /// `Pair<Integer, Double>` vertices of the node-weight tests.
    type P = Pair<i32, OrderedF64>;

    fn p(i: i32, w: f64) -> P {
        Pair::of(i, OrderedF64(w))
    }

    fn weight(v: &P) -> f64 {
        v.second.0
    }

    fn add_edges_and_weights<G: GraphMut>(
        g: &mut G,
        edges: &[(G::Vertex, G::Vertex)],
        weights: &[f64],
    ) {
        for ((a, b), w) in edges.iter().zip(weights) {
            let e = g.add_edge(a, b).unwrap().unwrap();
            g.set_edge_weight(&e, *w).unwrap();
        }
    }

    /// `GoldbergMaximumDensitySubgraphTestBase.test`
    fn check<'g, G: Graph>(
        g: &'g G,
        solver: &mut GoldbergMaximumDensitySubgraph<'g, G>,
        expected_density: f64,
        expected_vertices: &[G::Vertex],
    ) where
        G::Vertex: 'static,
    {
        let computed = solver.calculate_densest().unwrap();
        assert!(
            (solver.density().unwrap() - expected_density).abs() < EPS,
            "density {} vs {expected_density}",
            solver.density().unwrap()
        );
        let expected = AsSubgraph::induced(g, expected_vertices.iter().cloned());
        assert!(graph_equals(&expected, &computed));
    }

    fn small2_edges<V: Clone>(v: &[V]) -> Vec<(V, V)> {
        [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 7),
            (1, 7),
            (2, 7),
            (3, 7),
            (4, 2),
        ]
        .iter()
        .map(|&(a, b)| (v[a].clone(), v[b].clone()))
        .collect()
    }
    const SMALL2_WEIGHTS: [f64; 11] = [3.0, 2.0, 1.0, 2.0, 1.0, 3.0, 1.0, 2.0, 1.0, 4.0, 1.0];

    fn medium_edges<V: Clone>(v: &[V]) -> (Vec<(V, V)>, Vec<f64>) {
        let (mut edges, mut weights) = (Vec::new(), Vec::new());
        for i in 1..=50usize {
            edges.push((v[i].clone(), v[i / 2].clone()));
            weights.push(1.0 / ((i + 1) as f64).log10());
        }
        for j in 50..=100usize {
            edges.push((v[j].clone(), v[1].clone()));
            weights.push(100.0 / j as f64);
        }
        (edges, weights)
    }

    fn medium_expected<V: Clone>(v: &[V]) -> Vec<V> {
        let mut e: Vec<V> = v[50..=100].to_vec();
        e.extend([v[0].clone(), v[1].clone(), v[2].clone()]);
        e
    }

    mod edge_density {
        use super::*;
        type G<D> = BaseGraph<i32, DefaultEdge, D>;

        fn solver<'g, D: graph::Direction>(
            g: &'g G<D>,
        ) -> GoldbergMaximumDensitySubgraph<'g, G<D>> {
            GoldbergMaximumDensitySubgraph::new(g, -1, -2, EPS).unwrap()
        }

        #[test]
        fn test_empty_1() {
            let g = G::<Undirected>::multi().weighted();
            check(&g, &mut solver(&g), 0.0, &[]);
        }

        #[test]
        fn test_empty_2() {
            let mut g = G::<Undirected>::multi().weighted();
            g.add_vertices([0, 1]);
            check(&g, &mut solver(&g), 0.0, &[]);
        }

        #[test]
        fn test_minimal() {
            let mut g = G::<Undirected>::multi().weighted();
            g.add_vertices([0, 1]);
            add_edges_and_weights(&mut g, &[(0, 1)], &[10.0]);
            check(&g, &mut solver(&g), 5.0, &[0, 1]);
        }

        #[test]
        fn test_small_1() {
            let mut g = G::<Undirected>::multi().weighted();
            g.add_vertices(0..5);
            add_edges_and_weights(
                &mut g,
                &[(0, 3), (0, 1), (0, 2), (4, 2), (0, 4), (2, 3)],
                &[2.0, 1.0, 1.0, 1.0, 3.0, 1.0],
            );
            check(&g, &mut solver(&g), 2.0, &[0, 2, 3, 4]);
        }

        #[test]
        fn test_small_2() {
            let mut g = G::<Undirected>::simple().weighted();
            g.add_vertices(0..8);
            let v: Vec<i32> = (0..8).collect();
            add_edges_and_weights(&mut g, &small2_edges(&v), &SMALL2_WEIGHTS);
            check(&g, &mut solver(&g), 2.66666666, &[0, 1, 2, 3, 4, 7]);
        }

        #[test]
        fn test_small_weights() {
            let mut g = G::<Directed>::simple().weighted();
            g.add_vertices(0..5);
            add_edges_and_weights(
                &mut g,
                &[(0, 3), (0, 1), (0, 2), (4, 2), (0, 4), (2, 3)],
                &[0.0002, 0.00000001, 0.001, 0.0009, 0.003, 0.001],
            );
            check(&g, &mut solver(&g), 0.001633333, &[0, 2, 4]);
        }

        #[test]
        fn test_medium() {
            let mut g = G::<Directed>::multi().weighted();
            let v: Vec<i32> = (0..=100).collect();
            g.add_vertices(v.clone());
            let (edges, weights) = medium_edges(&v);
            add_edges_and_weights(&mut g, &edges, &weights);
            check(&g, &mut solver(&g), 1.411760, &medium_expected(&v));
        }

        #[test]
        fn source_and_sink_must_not_be_in_the_graph() {
            let mut g = G::<Undirected>::multi().weighted();
            g.add_vertices([0, 1, -1]);
            assert!(GoldbergMaximumDensitySubgraph::new(&g, -1, -2, EPS).is_err());
        }
    }

    macro_rules! node_weight_tests {
        ($name:ident, $ctor:ident, [$e1:expr, $e2:expr, $e3:expr, $e4:expr, $e5:expr, $e6:expr, $e7:expr]) => {
            mod $name {
                use super::*;
                type G<D> = BaseGraph<P, DefaultEdge, D>;

                fn solver<'g, D: graph::Direction>(
                    g: &'g G<D>,
                ) -> GoldbergMaximumDensitySubgraph<'g, G<D>> {
                    GoldbergMaximumDensitySubgraph::$ctor(g, p(-2, 0.0), p(-1, 0.0), EPS, weight)
                        .unwrap()
                }

                #[test]
                fn test_empty_1() {
                    let g = G::<Undirected>::multi().weighted();
                    check(&g, &mut solver(&g), $e1, &[]);
                }

                #[test]
                fn test_empty_2() {
                    let mut g = G::<Undirected>::multi().weighted();
                    let (p1, p2) = (p(0, 1.3), p(1, 2.1));
                    g.add_vertices([p1.clone(), p2.clone()]);
                    let expected: Vec<P> = $e2(&p1, &p2);
                    check(&g, &mut solver(&g), $e3, &expected);
                }

                #[test]
                fn test_minimal() {
                    let mut g = G::<Directed>::simple().weighted();
                    let (v1, v2) = (p(1, 1.5), p(0, 2.5));
                    g.add_vertices([v1.clone(), v2.clone()]);
                    add_edges_and_weights(&mut g, &[(v1.clone(), v2.clone())], &[10.0]);
                    check(&g, &mut solver(&g), $e4, &[v1, v2]);
                }

                #[test]
                fn test_small_1() {
                    let mut g = G::<Undirected>::simple().weighted();
                    let v = vec![p(0, 1.51), p(1, 1.0), p(2, 1.0)];
                    g.add_vertices(v.clone());
                    add_edges_and_weights(
                        &mut g,
                        &[(v[0].clone(), v[1].clone()), (v[0].clone(), v[2].clone())],
                        &[4.0, 2.0],
                    );
                    let idx: &[usize] = $e5;
                    let expected: Vec<P> = idx.iter().map(|&i| v[i].clone()).collect();
                    check(&g, &mut solver(&g), $e6, &expected);
                }

                #[test]
                fn test_small_2() {
                    let mut g = G::<Undirected>::simple().weighted();
                    let v: Vec<P> = (0..=7).map(|i| p(i, 1.1)).collect();
                    g.add_vertices(v.clone());
                    add_edges_and_weights(&mut g, &small2_edges(&v), &SMALL2_WEIGHTS);
                    let expected: Vec<P> =
                        [0, 1, 2, 3, 4, 7].iter().map(|&i| v[i].clone()).collect();
                    check(&g, &mut solver(&g), $e7.0, &expected);
                }

                #[test]
                fn test_medium() {
                    let mut g = G::<Directed>::multi().weighted();
                    let v: Vec<P> = (0..=100).map(|i| p(i, 1.0)).collect();
                    g.add_vertices(v.clone());
                    let (edges, weights) = medium_edges(&v);
                    add_edges_and_weights(&mut g, &edges, &weights);
                    check(&g, &mut solver(&g), $e7.1, &medium_expected(&v));
                }
            }
        };
    }

    node_weight_tests!(
        node_weights,
        with_node_weights,
        [
            0.0,
            |_: &P, p2: &P| vec![p2.clone()],
            2.1,
            7.0,
            &[0, 1],
            3.255,
            (3.76666666, 2.411760)
        ]
    );
    node_weight_tests!(
        node_weight_per_edge_weight,
        with_node_weight_per_edge_weight,
        [
            0.0,
            |_: &P, _: &P| Vec::<P>::new(),
            0.0,
            2.5,
            &[0, 1, 2],
            1.709401,
            (2.424242, 1.411760)
        ]
    );
}

mod zhang_shasha_tree_edit_distance_test {
    use super::*;
    use OperationType::*;
    use jgrapht::alg::similarity::{EditOperation, OperationType, ZhangShashaTreeEditDistance};

    const ARTICLE_TREE_1: &[(i32, i32)] = &[(1, 2), (1, 3), (2, 4), (2, 5), (5, 6)];
    const ARTICLE_TREE_2: &[(i32, i32)] = &[(1, 5), (1, 3), (5, 2), (2, 4), (2, 6)];
    const TREE_1: &[(i32, i32)] = &[
        (1, 2),
        (1, 4),
        (2, 5),
        (3, 6),
        (4, 10),
        (5, 6),
        (5, 7),
        (5, 9),
        (7, 8),
        (10, 4),
    ];
    const TREE_2: &[(i32, i32)] = &[
        (0, 1),
        (0, 3),
        (1, 4),
        (1, 0),
        (2, 5),
        (3, 0),
        (3, 9),
        (4, 5),
        (4, 6),
        (4, 8),
        (4, 1),
        (5, 2),
        (5, 4),
        (6, 7),
        (6, 4),
        (7, 6),
        (8, 4),
        (9, 3),
    ];
    const TREE_3: &[(i32, i32)] = &[
        (0, 3),
        (0, 6),
        (0, 4),
        (0, 9),
        (1, 8),
        (1, 2),
        (2, 5),
        (2, 1),
        (2, 4),
        (3, 0),
        (4, 7),
        (4, 2),
        (4, 0),
        (5, 2),
        (6, 0),
        (7, 4),
        (8, 1),
        (9, 0),
    ];

    fn read_graph(edges: &[(i32, i32)]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple();
        for &(a, b) in edges {
            g.add_edge_with_vertices(a, b).unwrap();
        }
        g
    }

    fn one_vertex() -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple();
        g.add_vertex(1);
        g
    }

    fn op(ty: OperationType, a: i32, b: Option<i32>) -> EditOperation<i32> {
        EditOperation::new(ty, a, b)
    }

    fn test_on_trees(
        t1: &UGraph<i32>,
        r1: i32,
        t2: &UGraph<i32>,
        r2: i32,
        expected_distance: f64,
        expected: &[EditOperation<i32>],
    ) {
        let mut ted = ZhangShashaTreeEditDistance::new(t1, r1, t2, r2).unwrap();
        assert!((ted.distance() - expected_distance).abs() < 1e-9);
        let actual: HashSet<EditOperation<i32>> = ted.edit_operations().iter().cloned().collect();
        assert_eq!(actual, expected.iter().cloned().collect::<HashSet<_>>());
    }

    #[test]
    fn test_ted_tree_with_one_vertex_to_tree_with_one_vertex() {
        test_on_trees(
            &one_vertex(),
            1,
            &one_vertex(),
            1,
            0.0,
            &[op(Change, 1, Some(1))],
        );
    }

    #[test]
    fn test_ted_tree_with_one_vertex_to_article_tree_2() {
        let expected = [
            op(Change, 1, Some(1)),
            op(Insert, 2, None),
            op(Insert, 3, None),
            op(Insert, 4, None),
            op(Insert, 5, None),
            op(Insert, 6, None),
        ];
        test_on_trees(
            &one_vertex(),
            1,
            &read_graph(ARTICLE_TREE_2),
            1,
            5.0,
            &expected,
        );
    }

    #[test]
    fn test_ted_article_tree_1_to_tree_with_one_vertex() {
        let expected = [
            op(Change, 1, Some(1)),
            op(Remove, 2, None),
            op(Remove, 3, None),
            op(Remove, 4, None),
            op(Remove, 5, None),
            op(Remove, 6, None),
        ];
        test_on_trees(
            &read_graph(ARTICLE_TREE_1),
            1,
            &one_vertex(),
            1,
            5.0,
            &expected,
        );
    }

    #[test]
    fn test_ted_article_tree_1_to_article_tree_2() {
        let expected = [
            op(Remove, 5, None),
            op(Insert, 5, None),
            op(Change, 1, Some(1)),
            op(Change, 2, Some(2)),
            op(Change, 3, Some(3)),
            op(Change, 4, Some(4)),
            op(Change, 6, Some(6)),
        ];
        test_on_trees(
            &read_graph(ARTICLE_TREE_1),
            1,
            &read_graph(ARTICLE_TREE_2),
            1,
            2.0,
            &expected,
        );
    }

    #[test]
    fn test_ted_tree_1_to_article_tree_2() {
        let expected = [
            op(Change, 1, Some(1)),
            op(Remove, 4, None),
            op(Change, 10, Some(3)),
            op(Remove, 2, None),
            op(Change, 5, Some(5)),
            op(Remove, 9, None),
            op(Remove, 7, None),
            op(Remove, 8, None),
            op(Insert, 2, None),
            op(Change, 6, Some(6)),
            op(Remove, 3, None),
            op(Insert, 4, None),
        ];
        test_on_trees(
            &read_graph(TREE_1),
            1,
            &read_graph(ARTICLE_TREE_2),
            1,
            9.0,
            &expected,
        );
    }

    #[test]
    fn test_ted_article_tree_1_to_tree_1() {
        let expected = [
            op(Change, 1, Some(1)),
            op(Insert, 4, None),
            op(Change, 3, Some(10)),
            op(Change, 2, Some(2)),
            op(Change, 5, Some(5)),
            op(Insert, 9, None),
            op(Insert, 7, None),
            op(Insert, 8, None),
            op(Change, 6, Some(6)),
            op(Insert, 3, None),
            op(Remove, 4, None),
        ];
        test_on_trees(
            &read_graph(ARTICLE_TREE_1),
            1,
            &read_graph(TREE_1),
            1,
            7.0,
            &expected,
        );
    }

    #[test]
    fn test_ted_tree_2_to_tree_3() {
        let expected = [
            op(Change, 0, Some(0)),
            op(Remove, 3, None),
            op(Change, 9, Some(9)),
            op(Remove, 1, None),
            op(Change, 4, Some(4)),
            op(Remove, 8, None),
            op(Remove, 6, None),
            op(Change, 7, Some(7)),
            op(Remove, 5, None),
            op(Change, 2, Some(2)),
            op(Insert, 5, None),
            op(Insert, 1, None),
            op(Insert, 8, None),
            op(Insert, 6, None),
            op(Insert, 3, None),
        ];
        test_on_trees(
            &read_graph(TREE_2),
            0,
            &read_graph(TREE_3),
            0,
            10.0,
            &expected,
        );
    }

    #[test]
    fn rejects_non_trees() {
        let mut g = read_graph(&[(1, 2), (2, 3), (3, 1)]);
        g.add_vertex(9);
        assert!(ZhangShashaTreeEditDistance::new(&g, 1, &one_vertex(), 1).is_err());
    }
}
