//! Ports of `org.jgrapht.alg.lca` tests: LCATreeTestBase (run for the binary lifting, Euler tour,
//! heavy path and Tarjan finders) and NaiveLCAFinderTest.
//!
//! `null` arguments are not applicable; the forest generators get a fresh seeded `StdRng`.

use jgrapht::alg::connectivity::connected_components;
use jgrapht::alg::interfaces::LowestCommonAncestorAlgorithm;
use jgrapht::alg::lca::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

type Boxed<'a, V> = Box<dyn LowestCommonAncestorAlgorithm<V> + 'a>;

fn binary_lifting<G: Graph>(g: &G, roots: &[G::Vertex]) -> Result<Boxed<'static, G::Vertex>>
where
    G::Vertex: 'static,
{
    Ok(Box::new(BinaryLiftingLcaFinder::with_roots(g, roots)?))
}
fn euler_tour<G: Graph>(g: &G, roots: &[G::Vertex]) -> Result<Boxed<'static, G::Vertex>>
where
    G::Vertex: 'static,
{
    Ok(Box::new(EulerTourRmqLcaFinder::with_roots(g, roots)?))
}
fn heavy_path<G: Graph>(g: &G, roots: &[G::Vertex]) -> Result<Boxed<'static, G::Vertex>>
where
    G::Vertex: 'static,
{
    Ok(Box::new(HeavyPathLcaFinder::with_roots(
        g,
        roots.iter().cloned(),
    )?))
}
fn tarjan<'g, G: Graph>(g: &'g G, roots: &[G::Vertex]) -> Result<Boxed<'g, G::Vertex>> {
    Ok(Box::new(TarjanLcaFinder::with_roots(g, roots.to_vec())?))
}

fn s(x: &str) -> String {
    x.to_string()
}

fn string_graph(vertices: &[&str], edges: &[(&str, &str)]) -> UGraph<String> {
    let mut g = UGraph::<String>::simple();
    g.add_vertices(vertices.iter().map(|v| s(v)));
    for (a, b) in edges {
        g.add_edge(&s(a), &s(b)).unwrap();
    }
    g
}

fn int_graph(n: i32, edges: &[(i32, i32)]) -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple();
    g.add_vertices(1..=n);
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
    g
}

fn forest(trees: usize, n: usize, start: i32, seed: i64) -> UGraph<i32> {
    let mut g = if start == 0 {
        UGraph::<i32>::simple().with_vertex_factory(|i| i as i32)
    } else {
        UGraph::<i32>::simple().with_vertex_factory(|i| i as i32 + 1)
    };
    BarabasiAlbertForestGenerator::new(trees, n, StdRng::seed_from_u64((seed) as u64))
        .unwrap()
        .generate_default(&mut g)
        .unwrap();
    g
}

fn generate_queries(q: usize, vertices: &[i32], random: &mut StdRng) -> Vec<(i32, i32)> {
    (0..q)
        .map(|_| {
            let a = vertices[random.random_range(0..vertices.len() as i32) as usize];
            let b = vertices[random.random_range(0..vertices.len() as i32) as usize];
            (a, b)
        })
        .collect()
}

fn roots_of(g: &UGraph<i32>) -> Vec<i32> {
    connected_components(g).into_iter().map(|c| c[0]).collect()
}

macro_rules! lca_tree_tests {
    ($module:ident, $solver:ident, $euler:expr) => {
        mod $module {
            use super::*;

            /// A solver of another kind, to compare answers with.
            fn other<G: Graph>(g: &G, roots: &[G::Vertex]) -> Result<Boxed<'static, G::Vertex>>
            where
                G::Vertex: 'static,
            {
                if $euler {
                    binary_lifting(g, roots)
                } else {
                    euler_tour(g, roots)
                }
            }

            #[test]
            fn test_invalid_node() {
                let g = string_graph(&["b", "a", "c"], &[("a", "b"), ("a", "c")]);
                assert!(
                    $solver(&g, &[s("a")])
                        .unwrap()
                        .lca(&s("d"), &s("d"))
                        .is_err()
                );
            }

            #[test]
            fn test_not_explored_node() {
                let g = string_graph(&["b", "a", "c", "d"], &[("a", "b"), ("a", "c")]);
                assert_eq!(
                    $solver(&g, &[s("a")])
                        .unwrap()
                        .lca(&s("a"), &s("d"))
                        .unwrap(),
                    None
                );
            }

            #[test]
            fn test_one_node() {
                let g = string_graph(&["a"], &[]);
                assert_eq!(
                    $solver(&g, &[s("a")])
                        .unwrap()
                        .lca(&s("a"), &s("a"))
                        .unwrap(),
                    Some(s("a"))
                );
            }

            #[test]
            fn test_two_roots_in_the_same_tree() {
                let g = string_graph(&["b", "a", "c", "d"], &[("a", "b"), ("c", "d")]);
                // Tarjan reports the shared tree only when answering a query
                assert!(
                    $solver(&g, &[s("a"), s("b")])
                        .and_then(|alg| alg.lca(&s("a"), &s("b")))
                        .is_err()
                );
            }

            #[test]
            fn test_two_roots_in_the_same_tree_2() {
                let g = string_graph(&["b", "a", "c", "d"], &[("a", "b"), ("c", "d")]);
                assert!(
                    $solver(&g, &[s("b"), s("a")])
                        .and_then(|alg| alg.lca(&s("a"), &s("b")))
                        .is_err()
                );
            }

            #[test]
            fn test_disconnect_small_graph() {
                let g = string_graph(&["a", "b"], &[]);
                let alg = $solver(&g, &[s("a")]).unwrap();
                assert_eq!(alg.lca(&s("a"), &s("b")).unwrap(), None);
                assert_eq!(alg.lca(&s("b"), &s("a")).unwrap(), None);
                assert_eq!(alg.lca(&s("a"), &s("a")).unwrap(), Some(s("a")));
                assert_eq!(alg.lca(&s("b"), &s("b")).unwrap(), Some(s("b")));
            }

            #[test]
            fn test_disconnect_small_graph_2() {
                let g = string_graph(&["a", "b", "c"], &[]);
                let alg = $solver(&g, &[s("a")]).unwrap();
                for (x, y) in [
                    ("b", "c"),
                    ("c", "b"),
                    ("c", "a"),
                    ("a", "c"),
                    ("a", "b"),
                    ("b", "a"),
                ] {
                    assert_eq!(alg.lca(&s(x), &s(y)).unwrap(), None);
                }
                assert_eq!(alg.lca(&s("a"), &s("a")).unwrap(), Some(s("a")));
                assert_eq!(alg.lca(&s("b"), &s("b")).unwrap(), Some(s("b")));
            }

            #[test]
            fn test_empty_graph() {
                let g = string_graph(&[], &[]);
                assert!($solver(&g, &[s("a")]).is_err());
            }

            #[test]
            fn test_empty_set_of_roots() {
                let g = string_graph(&["a"], &[]);
                assert!($solver(&g, &[]).is_err());
            }

            #[test]
            fn test_root_not_in_graph() {
                let g = string_graph(&["a"], &[]);
                assert!($solver(&g, &[s("b")]).is_err());
            }

            #[test]
            fn test_graph_all_possible_queries() {
                let n = 100;
                let g = forest(1, n, 0, 0x8888);
                let root = *g.vertices().next().unwrap();
                let alg1 = $solver(&g, &[root]).unwrap();
                let alg2 = other(&g, &[root]).unwrap();
                let queries: Vec<(i32, i32)> = (0..n as i32)
                    .flat_map(|i| (0..n as i32).map(move |j| (i, j)))
                    .collect();
                assert_eq!(
                    alg1.batch_lca(&queries).unwrap(),
                    alg2.batch_lca(&queries).unwrap()
                );
            }

            #[test]
            fn test_long_chain() {
                let (n, q) = (2000, 100_000);
                let mut random = StdRng::seed_from_u64((0x88) as u64);
                let g = int_graph(n, &(1..n).map(|i| (i, i + 1)).collect::<Vec<_>>());
                let vertices: Vec<i32> = g.vertices().copied().collect();
                let queries = generate_queries(q, &vertices, &mut random);
                let alg = $solver(&g, &[n]).unwrap();
                let lcas = alg.batch_lca(&queries).unwrap();
                for (i, (a, b)) in queries.iter().enumerate() {
                    assert_eq!(lcas[i], Some(*a.max(b)));
                }
            }

            #[test]
            fn test_binary_tree() {
                let g = string_graph(
                    &["a", "b", "c", "d", "e"],
                    &[("a", "b"), ("b", "c"), ("b", "d"), ("d", "e")],
                );
                let alg = $solver(&g, &[s("a")]).unwrap();
                assert_eq!(alg.lca(&s("c"), &s("e")).unwrap(), Some(s("b")));
                assert_eq!(alg.lca(&s("b"), &s("d")).unwrap(), Some(s("b")));
                assert_eq!(alg.lca(&s("d"), &s("e")).unwrap(), Some(s("d")));
            }

            #[test]
            fn test_small_tree() {
                let g = int_graph(
                    11,
                    &[
                        (1, 2),
                        (2, 4),
                        (2, 5),
                        (2, 6),
                        (4, 7),
                        (4, 8),
                        (6, 9),
                        (1, 3),
                        (3, 10),
                        (3, 11),
                    ],
                );
                let alg = $solver(&g, &[1]).unwrap();
                for ((a, b), want) in [
                    ((10, 11), 3),
                    ((8, 9), 2),
                    ((5, 11), 1),
                    ((5, 6), 2),
                    ((4, 2), 2),
                    ((4, 5), 2),
                    ((2, 2), 2),
                    ((8, 6), 2),
                    ((7, 8), 4),
                ] {
                    assert_eq!(alg.lca(&a, &b).unwrap(), Some(want), "lca({a}, {b})");
                }
            }

            #[test]
            fn test_small_tree_2() {
                let g = int_graph(
                    20,
                    &[
                        (2, 1),
                        (3, 1),
                        (4, 1),
                        (5, 1),
                        (6, 2),
                        (7, 5),
                        (8, 7),
                        (9, 3),
                        (10, 2),
                        (11, 9),
                        (12, 6),
                        (13, 4),
                        (14, 6),
                        (15, 2),
                        (16, 10),
                        (17, 15),
                        (18, 6),
                        (19, 14),
                        (20, 11),
                    ],
                );
                let alg = $solver(&g, &[1]).unwrap();
                for ((a, b), want) in [
                    ((9, 14), 1),
                    ((10, 9), 1),
                    ((15, 15), 15),
                    ((1, 17), 1),
                    ((3, 3), 3),
                    ((3, 1), 1),
                    ((11, 14), 1),
                    ((18, 19), 6),
                    ((12, 2), 2),
                    ((16, 14), 2),
                ] {
                    assert_eq!(alg.lca(&a, &b).unwrap(), Some(want), "lca({a}, {b})");
                }
            }

            #[test]
            fn test_non_binary_tree_batch() {
                let g = string_graph(
                    &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"],
                    &[
                        ("a", "b"),
                        ("b", "c"),
                        ("c", "d"),
                        ("d", "e"),
                        ("b", "f"),
                        ("b", "g"),
                        ("c", "h"),
                        ("c", "i"),
                        ("i", "j"),
                    ],
                );
                let alg = $solver(&g, &[s("a")]).unwrap();
                assert_eq!(alg.lca(&s("b"), &s("h")).unwrap(), Some(s("b")));
                assert_eq!(alg.lca(&s("j"), &s("f")).unwrap(), Some(s("b")));
                assert_eq!(alg.lca(&s("j"), &s("h")).unwrap(), Some(s("c")));
                let queries = [(s("b"), s("h")), (s("j"), s("f")), (s("j"), s("h"))];
                assert_eq!(
                    alg.batch_lca(&queries).unwrap(),
                    [Some(s("b")), Some(s("b")), Some(s("c"))]
                );
                assert_eq!(
                    $solver(&g, &[s("b")])
                        .unwrap()
                        .lca(&s("h"), &s("b"))
                        .unwrap(),
                    Some(s("b"))
                );
            }

            #[test]
            fn random_huge_connected_tree() {
                let (n, q) = (100_000, 200_000);
                let mut random = StdRng::seed_from_u64((0x88) as u64);
                let g = forest(1, n, 1, random.random::<i64>());
                let vertices: Vec<i32> = g.vertices().copied().collect();
                let alg1 = $solver(&g, &[vertices[0]]).unwrap();
                let alg2 = other(&g, &[vertices[0]]).unwrap();
                let queries = generate_queries(q, &vertices, &mut random);
                assert_eq!(
                    alg1.batch_lca(&queries).unwrap(),
                    alg2.batch_lca(&queries).unwrap()
                );
            }

            #[test]
            fn random_huge_possibly_disconnected_tree() {
                let (n, q) = (100_000, 200_000);
                let mut random = StdRng::seed_from_u64((0x55) as u64);
                let num_trees = 100 + random.random_range(0..200) as usize;
                let g = forest(num_trees, n, 1, random.random::<i64>());
                let vertices: Vec<i32> = g.vertices().copied().collect();
                let roots = roots_of(&g);
                let alg1 = $solver(&g, &roots).unwrap();
                let alg2 = other(&g, &roots).unwrap();
                let queries = generate_queries(q, &vertices, &mut random);
                assert_eq!(
                    alg1.batch_lca(&queries).unwrap(),
                    alg2.batch_lca(&queries).unwrap()
                );
            }

            #[test]
            fn test_small_connected_trees() {
                let mut random = StdRng::seed_from_u64((0x88) as u64);
                for _ in 0..10_000 {
                    let n = 10 + random.random_range(0..100) as usize;
                    let g = forest(1, n, 1, random.random::<i32>() as i64);
                    let vertices: Vec<i32> = g.vertices().copied().collect();
                    let roots = [vertices[0]];
                    let alg1 = $solver(&g, &roots).unwrap();
                    let alg2 = other(&g, &roots).unwrap();
                    let queries = generate_queries(50, &vertices, &mut random);
                    assert_eq!(
                        alg1.batch_lca(&queries).unwrap(),
                        alg2.batch_lca(&queries).unwrap()
                    );
                }
            }

            #[test]
            fn test_small_disconnected_trees() {
                let mut random = StdRng::seed_from_u64((0x88) as u64);
                for _ in 0..10_000 {
                    let n = 10 + random.random_range(0..200) as usize;
                    let g = forest(n / 10, n, 1, random.random::<i32>() as i64);
                    let roots = roots_of(&g);
                    let alg1 = $solver(&g, &roots).unwrap();
                    let alg2 = other(&g, &roots).unwrap();
                    let vertices: Vec<i32> = g.vertices().copied().collect();
                    let queries = generate_queries(50, &vertices, &mut random);
                    assert_eq!(
                        alg1.batch_lca(&queries).unwrap(),
                        alg2.batch_lca(&queries).unwrap()
                    );
                }
            }
        }
    };
}

lca_tree_tests!(binary_lifting_lca_finder_test, binary_lifting, false);
lca_tree_tests!(euler_tour_rmq_lca_finder_test, euler_tour, true);
lca_tree_tests!(heavy_path_lca_finder_test, heavy_path, false);
lca_tree_tests!(tarjan_lca_finder_test, tarjan, false);

mod naive_lca_finder_test {
    use super::*;

    fn digraph(vertices: &[&str], edges: &[(&str, &str)], loops: bool) -> DiGraph<String> {
        let mut g = if loops {
            DiGraph::<String>::with_graph_type(GraphType {
                self_loops: true,
                ..GraphType::simple()
            })
        } else {
            DiGraph::<String>::simple()
        };
        g.add_vertices(vertices.iter().map(|v| s(v)));
        for (a, b) in edges {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        g
    }

    fn check_lcas<G: jgrapht::DirectedGraph<Vertex = String>>(
        finder: &NaiveLcaFinder<G>,
        a: &str,
        b: &str,
        expected: &[&str],
    ) {
        let set = finder.lca_set(&s(a), &s(b)).unwrap();
        assert_eq!(set, expected.iter().map(|x| s(x)).collect::<HashSet<_>>());
    }

    #[test]
    fn test_normal_cases() {
        let g = digraph(
            &["a", "b", "c", "d", "e", "f", "g", "h"],
            &[
                ("a", "b"),
                ("b", "c"),
                ("c", "d"),
                ("d", "e"),
                ("b", "f"),
                ("b", "g"),
                ("f", "e"),
                ("e", "h"),
            ],
            false,
        );
        let finder = NaiveLcaFinder::new(&g).unwrap();
        for (a, b, want) in [
            ("f", "h", "f"),
            ("h", "f", "f"),
            ("g", "h", "b"),
            ("c", "c", "c"),
            ("a", "e", "a"),
        ] {
            assert_eq!(finder.lca(&s(a), &s(b)).unwrap(), Some(s(want)));
            check_lcas(&finder, a, b, &[want]);
        }
    }

    #[test]
    fn test_no_lca() {
        let g = digraph(
            &["a", "b", "c", "d", "e", "f", "g", "h", "i"],
            &[
                ("a", "b"),
                ("b", "c"),
                ("c", "d"),
                ("d", "e"),
                ("f", "g"),
                ("f", "h"),
                ("g", "i"),
                ("h", "i"),
            ],
            false,
        );
        let finder = NaiveLcaFinder::new(&g).unwrap();
        assert_eq!(finder.lca(&s("i"), &s("e")).unwrap(), None);
        assert!(finder.lca_set(&s("i"), &s("e")).unwrap().is_empty());
    }

    #[test]
    fn test_loops() {
        let g = digraph(
            &["a", "b", "c", "d", "e", "f", "g", "h", "i"],
            &[
                ("a", "b"),
                ("b", "c"),
                ("c", "d"),
                ("d", "e"),
                ("b", "f"),
                ("b", "g"),
                ("f", "e"),
                ("e", "h"),
                ("h", "e"),
                ("h", "h"),
                ("i", "i"),
            ],
            true,
        );
        let finder = NaiveLcaFinder::new(&g).unwrap();
        assert_eq!(finder.lca(&s("h"), &s("f")).unwrap(), Some(s("f")));
        assert_eq!(finder.lca(&s("a"), &s("i")).unwrap(), None);
        check_lcas(&finder, "h", "f", &["f"]);
        assert!(finder.lca_set(&s("a"), &s("i")).unwrap().is_empty());
    }

    #[test]
    fn test_arrival_order() {
        let g = digraph(
            &["a", "b", "c", "g", "e", "h"],
            &[
                ("a", "b"),
                ("b", "c"),
                ("a", "g"),
                ("b", "g"),
                ("g", "e"),
                ("e", "h"),
            ],
            true,
        );
        let finder = NaiveLcaFinder::new(&g).unwrap();
        assert_eq!(finder.lca(&s("b"), &s("h")).unwrap(), Some(s("b")));
        assert_eq!(finder.lca(&s("c"), &s("e")).unwrap(), Some(s("b")));
        check_lcas(&finder, "b", "h", &["b"]);
        check_lcas(&finder, "c", "e", &["b"]);
    }

    #[test]
    fn test_two_lcas() {
        let g = digraph(
            &["a", "b", "c", "d"],
            &[("a", "c"), ("a", "d"), ("b", "c"), ("b", "d")],
            true,
        );
        let finder = NaiveLcaFinder::new(&g).unwrap();
        check_lcas(&finder, "c", "d", &["a", "b"]);
    }

    #[test]
    fn test_lca_is_one_of_the_nodes() {
        let mut g = DiGraph::<i32>::with_graph_type(GraphType {
            self_loops: true,
            ..GraphType::simple()
        });
        g.add_vertex(0);
        for i in 1..=10 {
            g.add_vertex(i);
            g.add_edge(&(i - 1), &i).unwrap();
        }
        g.add_edge(&0, &10).unwrap();
        let finder = NaiveLcaFinder::new(&g).unwrap();
        assert_eq!(finder.lca_set(&1, &10).unwrap(), HashSet::from([1]));
    }

    #[test]
    fn test_lca() {
        let mut g = DiGraph::<String>::with_graph_type(GraphType {
            self_loops: true,
            ..GraphType::simple()
        });
        for (a, b) in [
            ("a", "b"),
            ("b", "c"),
            ("a", "d"),
            ("d", "e"),
            ("e", "f"),
            ("f", "c"),
        ] {
            g.add_edge_with_vertices(s(a), s(b)).unwrap();
        }
        let finder = NaiveLcaFinder::new(&g).unwrap();
        check_lcas(&finder, "c", "e", &["e"]);
    }

    #[test]
    fn rejects_undirected_graphs_and_unknown_vertices() {
        let u = UGraph::<i32>::simple();
        assert!(jgrapht::graph::props::require_directed(&u).is_err());
        let g = digraph(&["a"], &[], false);
        let finder = NaiveLcaFinder::new(&g).unwrap();
        assert!(finder.lca(&s("a"), &s("z")).is_err());
    }
}
