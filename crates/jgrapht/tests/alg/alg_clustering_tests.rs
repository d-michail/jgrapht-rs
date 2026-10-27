//! Ports of the `org.jgrapht.alg.clustering` tests: UndirectedModularityMeasurerTest,
//! GirvanNewmanClusteringTest, KSpanningTreeClusteringTest, GreedyModularityAlgorithmTest,
//! NaiveGreedyModularityAlgorithmTest and LabelPropagationClusteringTest (graphs and expectations
//! of the last three extracted from the Java sources into `clustering_data.rs`).

use crate::clustering_data::{Case, GREEDY, LP, NAIVE};
use crate::common::construct_graph;
use jgrapht::alg::clustering::*;
use jgrapht::alg::interfaces::{Clustering, ClusteringAlgorithm};
use jgrapht::util::java::JavaRandom;
use jgrapht::*;
use std::collections::HashSet;

fn set(items: &[i32]) -> HashSet<i32> {
    items.iter().copied().collect()
}

fn sets(items: &[&[i32]]) -> Vec<HashSet<i32>> {
    items.iter().map(|i| set(i)).collect()
}

fn as_sets(c: &Clustering<i32>) -> Vec<HashSet<i32>> {
    c.clusters()
        .iter()
        .map(|s| s.iter().copied().collect())
        .collect()
}

mod undirected_modularity_measurer_test {
    use super::*;

    const EDGES: &[(i32, i32)] = &[
        (0, 1),
        (0, 4),
        (1, 2),
        (1, 3),
        (1, 5),
        (2, 3),
        (2, 4),
        (3, 4),
        (5, 6),
        (5, 7),
        (5, 8),
        (6, 7),
        (7, 8),
    ];

    fn graph(vertices: i32, edges: &[(i32, i32)]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::pseudo().weighted();
        g.add_vertices(0..vertices);
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    fn modularity(g: &UGraph<i32>, partitions: &[&[i32]]) -> Result<f64> {
        UndirectedModularityMeasurer::new(g)
            .unwrap()
            .modularity(&sets(partitions))
    }

    #[test]
    fn test_optimal_partition() {
        let g = graph(9, EDGES);
        assert!(
            (modularity(&g, &[&[0, 1, 2, 3, 4], &[5, 6, 7, 8]]).unwrap() - 0.4112426).abs() < 1e-6
        );
    }

    #[test]
    fn test_single() {
        let g = graph(9, EDGES);
        assert!(
            modularity(&g, &[&[0, 1, 2, 3, 4, 5, 6, 7, 8]])
                .unwrap()
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn test_suboptimal() {
        let g = graph(9, EDGES);
        assert!(
            (modularity(&g, &[&[0, 3, 4], &[1, 2, 5, 6, 7, 8]]).unwrap() - 0.118343).abs() < 1e-6
        );
    }

    #[test]
    fn test_negative() {
        let g = graph(9, EDGES);
        let singletons: Vec<Vec<i32>> = [0, 3, 4, 1, 2, 5, 6, 7, 8]
            .iter()
            .map(|v| vec![*v])
            .collect();
        let refs: Vec<&[i32]> = singletons.iter().map(|v| v.as_slice()).collect();
        assert!((modularity(&g, &refs).unwrap() + 0.118343).abs() < 1e-6);
    }

    #[test]
    fn test_24() {
        let edges = [
            (0, 1),
            (0, 2),
            (0, 4),
            (1, 2),
            (2, 3),
            (2, 4),
            (2, 5),
            (2, 8),
            (3, 4),
            (3, 6),
            (5, 6),
            (5, 7),
            (6, 7),
            (8, 9),
            (8, 10),
            (9, 10),
            (9, 11),
            (10, 11),
            (11, 12),
            (12, 13),
            (12, 14),
            (13, 14),
            (13, 15),
            (14, 15),
        ];
        let g = graph(16, &edges);
        let m = modularity(
            &g,
            &[
                &[0, 1, 2, 3, 4],
                &[5, 6, 7],
                &[8, 9, 10, 11],
                &[12, 13, 14, 15],
            ],
        )
        .unwrap();
        assert!((0.565104 - m).abs() < 1e-6);
    }

    #[test]
    fn test_with_invalid_partition() {
        let g = graph(9, EDGES);
        assert!(modularity(&g, &[&[0, 3, 4], &[1, 2, 5, 7, 8, 9]]).is_err());
    }

    #[test]
    fn test_with_self_loops() {
        let mut edges = EDGES.to_vec();
        edges.extend([(4, 4), (7, 7)]);
        let g = graph(9, &edges);
        assert!(
            (modularity(&g, &[&[0, 1, 2, 3, 4], &[5, 6, 7, 8]]).unwrap() - 0.42444444444444446)
                .abs()
                < 1e-6
        );
    }
}

mod girvan_newman_clustering_test {
    use super::*;

    #[test]
    fn test_undirected_graph() {
        let mut g = UGraph::<i32>::simple().weighted();
        construct_graph(
            &mut g,
            &[
                &[1, 2],
                &[1, 3],
                &[2, 3],
                &[3, 7],
                &[4, 6],
                &[4, 5],
                &[5, 6],
                &[6, 7],
                &[7, 8],
                &[8, 9],
                &[8, 12],
                &[9, 10],
                &[9, 11],
                &[12, 13],
                &[12, 14],
                &[10, 11],
                &[13, 14],
            ],
        );
        let c1 = GirvanNewmanClustering::new(&g, 2)
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(c1.number_clusters(), 2);
        assert_eq!(
            as_sets(&c1),
            sets(&[&[1, 2, 3, 4, 5, 6, 7], &[8, 9, 10, 11, 12, 13, 14]])
        );
        let c2 = GirvanNewmanClustering::new(&g, 6)
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(
            as_sets(&c2),
            sets(&[
                &[1, 2, 3],
                &[7],
                &[4, 5, 6],
                &[8],
                &[9, 10, 11],
                &[12, 13, 14]
            ])
        );
        let c3 = GirvanNewmanClustering::new(&g, g.vertex_count())
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(c3.number_clusters(), g.vertex_count());
        assert!(GirvanNewmanClustering::new(&g, 0).is_err());
    }
}

mod k_spanning_tree_clustering_test {
    use super::*;

    fn graph(weights: &[f64; 12]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::pseudo().weighted();
        g.add_vertices(0..9);
        let edges = [
            (0, 1),
            (0, 5),
            (1, 2),
            (1, 4),
            (2, 3),
            (3, 4),
            (3, 8),
            (4, 5),
            (4, 7),
            (5, 6),
            (6, 7),
            (7, 8),
        ];
        for ((a, b), w) in edges.iter().zip(weights) {
            let e = g.add_edge(a, b).unwrap().unwrap();
            g.set_edge_weight(&e, *w).unwrap();
        }
        g
    }

    const W1: [f64; 12] = [
        2.0, 1.0, 3.0, 11.0, 4.0, 5.0, 10.0, 6.0, 12.0, 7.0, 8.0, 9.0,
    ];
    const W2: [f64; 12] = [
        2.0, 1.0, 9.0, 11.0, 4.0, 5.0, 10.0, 6.0, 12.0, 7.0, 8.0, 3.0,
    ];

    #[test]
    fn test_1() {
        let c = KSpanningTreeClustering::new(&graph(&W1), 3)
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(c.number_clusters(), 3);
        assert_eq!(as_sets(&c), sets(&[&[0, 1, 2, 3, 4, 5, 6], &[7], &[8]]));
    }

    #[test]
    fn test_2() {
        let c = KSpanningTreeClustering::new(&graph(&W2), 4)
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(c.number_clusters(), 4);
        assert_eq!(as_sets(&c), sets(&[&[0, 1, 5], &[2, 3, 4], &[6], &[7, 8]]));
    }

    #[test]
    fn test_one_cluster() {
        let c = KSpanningTreeClustering::new(&graph(&W1), 1)
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(c.number_clusters(), 1);
    }

    #[test]
    fn test_n_clusters() {
        let c = KSpanningTreeClustering::new(&graph(&W1), 9)
            .unwrap()
            .clustering()
            .unwrap();
        assert_eq!(c.number_clusters(), 9);
    }
}

/// The graph of an extracted case, undirected or directed, as the matching concrete type.
macro_rules! with_graph {
    ($case:expr, |$g:ident| $body:block) => {{
        let case: &Case = $case;
        let ty = GraphType {
            self_loops: case.loops,
            multiple_edges: case.multi,
            weighted: case.weighted,
            ..GraphType::simple()
        };
        if case.directed {
            let mut $g = DiGraph::<i32>::with_graph_type(ty);
            fill(&mut $g, case);
            $body
        } else {
            let mut $g = UGraph::<i32>::with_graph_type(ty);
            fill(&mut $g, case);
            $body
        }
    }};
}

fn fill<D: graph::Direction>(g: &mut BaseGraph<i32, DefaultEdge, D>, case: &Case) {
    g.add_vertices(case.vertices.iter().copied());
    for (a, b, w) in case.edges {
        let e = g.add_edge(a, b).unwrap().unwrap();
        if let Some(w) = w {
            g.set_edge_weight(&e, *w).unwrap();
        }
    }
}

fn check_case(case: &Case, clustering: &Clustering<i32>) {
    assert_eq!(
        Some(clustering.number_clusters()),
        case.expected_count,
        "{}",
        case.name
    );
    assert_eq!(
        as_sets(clustering),
        sets(case.expected_clusters),
        "{}",
        case.name
    );
}

mod greedy_modularity_algorithm_test {
    use super::*;

    #[test]
    fn all_cases() {
        for case in GREEDY {
            let c = with_graph!(case, |g| {
                GreedyModularityAlgorithm::new(&g)
                    .with_java_hash(|v| *v)
                    .clustering()
                    .unwrap()
            });
            check_case(case, &c);
        }
    }
}

mod naive_greedy_modularity_algorithm_test {
    use super::*;

    #[test]
    fn all_cases() {
        for case in NAIVE {
            let c = with_graph!(case, |g| {
                NaiveGreedyModularityAlgorithm::new(&g)
                    .unwrap()
                    .clustering()
                    .unwrap()
            });
            // (the naive algorithm lists communities in its own merge order)
            assert_eq!(
                Some(c.number_clusters()),
                case.expected_count,
                "{}",
                case.name
            );
            let mut got = as_sets(&c);
            let mut want = sets(case.expected_clusters);
            let key = |s: &HashSet<i32>| s.iter().copied().min();
            got.sort_by_key(key);
            want.sort_by_key(key);
            assert_eq!(got, want, "{}", case.name);
        }
    }
}

mod label_propagation_clustering_test {
    use super::*;

    #[test]
    fn all_cases() {
        for case in LP {
            let c = with_graph!(case, |g| {
                LabelPropagationClustering::new(&g, 0, JavaRandom::new(case.seed.unwrap()))
                    .unwrap()
                    .clustering()
                    .unwrap()
            });
            check_case(case, &c);
        }
    }

    #[test]
    fn rejects_directed_graphs() {
        assert!(
            LabelPropagationClustering::new(&DiGraph::<i32>::simple(), 0, JavaRandom::new(1))
                .is_err()
        );
    }
}
