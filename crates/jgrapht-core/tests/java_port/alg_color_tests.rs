//! Ports of the `org.jgrapht.alg.color` tests: BaseColoringTest (run for greedy, random greedy x2,
//! saturation degree, smallest degree last and largest degree first), the algorithm specific tests,
//! BrownBacktrackColoringTest (data in `brown_data.rs`), ChordalGraphColoringTest and
//! ColorRefinementAlgorithmTest.

use crate::brown_data::CASES;
use jgrapht::alg::color::*;
use jgrapht::alg::interfaces::VertexColoringAlgorithm;
use jgrapht::generate::*;
use jgrapht::util::java::JavaRandom;
use jgrapht::*;
use std::collections::HashMap;

type G = UGraph<i32>;

fn graph_with(vertices: std::ops::RangeInclusive<i32>, edges: &[(i32, i32)], pseudo: bool) -> G {
    let mut g = if pseudo { G::pseudo() } else { G::simple() };
    g.add_vertices(vertices);
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
    g
}

fn create_graph_1() -> G {
    graph_with(
        1..=5,
        &[
            (1, 2),
            (1, 3),
            (1, 3),
            (1, 4),
            (1, 5),
            (2, 3),
            (3, 4),
            (3, 5),
        ],
        false,
    )
}

fn case(name: &str) -> &'static crate::brown_data::Case {
    CASES.iter().find(|c| c.name == name).unwrap()
}

fn create_myciel_3() -> G {
    graph_with(1..=11, case("myciel3Test").edges, false)
}

fn create_myciel_4() -> G {
    graph_with(1..=23, case("myciel4Test").edges, false)
}

fn create_dsatur_non_optimal() -> G {
    graph_with(
        1..=7,
        &[
            (1, 2),
            (1, 3),
            (1, 4),
            (2, 3),
            (2, 5),
            (4, 6),
            (4, 7),
            (5, 6),
            (5, 7),
            (6, 7),
        ],
        false,
    )
}

fn assert_coloring(g: &G, coloring: &Coloring<i32>, expected: usize) {
    let n = g.vertex_count();
    assert!(coloring.count <= n);
    assert_eq!(coloring.count, expected);
    for v in g.vertices() {
        let c = coloring.colors[v];
        assert!(c < n);
    }
    for e in g.edges() {
        assert_ne!(
            coloring.colors[g.edge_source(e).unwrap()],
            coloring.colors[g.edge_target(e).unwrap()]
        );
    }
}

fn check_random_graph_coloring(compute: &dyn Fn(&G) -> Coloring<i32>, rng: JavaRandom) {
    let (tests, n, p) = (5, 20, 0.35);
    let mut gen_ = GnpRandomGraphGenerator::new(n, p, rng, false).unwrap();
    for _ in 0..tests {
        let mut g = G::simple().with_vertex_factory(|i| i as i32);
        gen_.generate_default(&mut g).unwrap();
        let coloring = compute(&g);
        assert!(coloring.count <= n);
        for v in g.vertices() {
            assert!(coloring.colors[v] < n);
        }
        for e in g.edges() {
            assert_ne!(
                coloring.colors[g.edge_source(e).unwrap()],
                coloring.colors[g.edge_target(e).unwrap()]
            );
        }
    }
}

macro_rules! base_coloring_tests {
    ($module:ident, |$g:ident| $make:expr, dsatur = $dsatur:expr) => {
        mod $module {
            use super::*;

            fn make($g: &G) -> impl VertexColoringAlgorithm<i32> + '_ {
                $make
            }

            #[test]
            fn test_myciel_3() {
                let g = create_myciel_3();
                assert_coloring(&g, &make(&g).coloring().unwrap(), 4);
            }

            #[test]
            fn test_myciel_4() {
                let g = create_myciel_4();
                assert_coloring(&g, &make(&g).coloring().unwrap(), 5);
            }

            #[test]
            fn test_dsatur_non_optimal() {
                let g = create_dsatur_non_optimal();
                assert_coloring(&g, &make(&g).coloring().unwrap(), $dsatur);
            }

            #[test]
            fn test_graph_1() {
                let g = create_graph_1();
                assert_coloring(&g, &make(&g).coloring().unwrap(), 3);
            }

            #[test]
            fn test_complete_graph() {
                let n = 20;
                let mut g = G::simple().with_vertex_factory(|i| i as i32);
                Complete(n).generate_default(&mut g).unwrap();
                assert_eq!(make(&g).coloring().unwrap().count, n);
            }

            #[test]
            fn test_random_fixed_seed_17() {
                check_random_graph_coloring(&|g| make(g).coloring().unwrap(), JavaRandom::new(17));
            }

            #[test]
            fn test_random() {
                check_random_graph_coloring(
                    &|g| make(g).coloring().unwrap(),
                    JavaRandom::from_time(),
                );
            }
        }
    };
}

base_coloring_tests!(greedy_coloring_test, |g| GreedyColoring::new(g), dsatur = 4);
base_coloring_tests!(
    random_greedy_coloring_test,
    |g| RandomGreedyColoring::new(g, JavaRandom::new(13)),
    dsatur = 3
);
base_coloring_tests!(
    random_greedy_coloring_2_test,
    |g| RandomGreedyColoring::new(g, JavaRandom::new(15)),
    dsatur = 4
);
base_coloring_tests!(
    saturation_degree_coloring_test,
    |g| SaturationDegreeColoring::new(g),
    dsatur = 4
);
base_coloring_tests!(
    smallest_degree_last_coloring_test,
    |g| SmallestDegreeLastColoring::new(g),
    dsatur = 3
);
base_coloring_tests!(
    largest_degree_first_coloring_test,
    |g| LargestDegreeFirstColoring::new(g),
    dsatur = 4
);

fn colors_of(c: Coloring<i32>) -> HashMap<i32, usize> {
    c.colors
}

#[test]
fn greedy_coloring_specific() {
    let g = create_graph_1();
    let coloring = GreedyColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!((c[&1], c[&2], c[&3], c[&4], c[&5]), (0, 1, 2, 1, 1));
}

#[test]
fn saturation_degree_specific() {
    let g = graph_with(1..=6, &[(2, 3), (4, 5), (4, 6), (5, 6), (5, 3)], true);
    let coloring = SaturationDegreeColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!(
        (c[&1], c[&2], c[&3], c[&4], c[&5], c[&6]),
        (0, 0, 1, 1, 0, 2)
    );
}

#[test]
fn smallest_degree_last_specific() {
    let g = create_graph_1();
    let coloring = SmallestDegreeLastColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!((c[&1], c[&2], c[&3], c[&4], c[&5]), (2, 0, 1, 0, 0));
}

#[test]
fn smallest_degree_last_non_simple() {
    let mut edges = vec![(2, 3), (4, 5), (4, 6)];
    edges.extend(std::iter::repeat_n((5, 6), 8));
    let g = graph_with(1..=6, &edges, true);
    let coloring = SmallestDegreeLastColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!(
        (c[&1], c[&2], c[&3], c[&4], c[&5], c[&6]),
        (0, 1, 0, 2, 1, 0)
    );
}

#[test]
fn largest_degree_first_specific() {
    let coloring = LargestDegreeFirstColoring::new(&create_graph_1())
        .coloring()
        .unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!((c[&1], c[&2], c[&3], c[&4], c[&5]), (0, 2, 1, 2, 2));
}

#[test]
fn largest_degree_first_coloring_1() {
    let g = graph_with(
        1..=6,
        &[
            (1, 2),
            (1, 3),
            (1, 3),
            (1, 4),
            (1, 5),
            (2, 3),
            (3, 4),
            (3, 5),
            (3, 6),
            (5, 6),
        ],
        false,
    );
    let coloring = LargestDegreeFirstColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!(
        (c[&1], c[&2], c[&3], c[&4], c[&5], c[&6]),
        (1, 2, 0, 2, 2, 1)
    );
}

#[test]
fn largest_degree_first_non_simple() {
    let mut edges = vec![(2, 3), (4, 5), (4, 6)];
    edges.extend(std::iter::repeat_n((5, 6), 20000));
    let g = graph_with(1..=6, &edges, true);
    let coloring = LargestDegreeFirstColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!(
        (c[&1], c[&2], c[&3], c[&4], c[&5], c[&6]),
        (0, 0, 1, 2, 0, 1)
    );
}

#[test]
fn largest_degree_first_simple() {
    let g = graph_with(1..=6, &[(2, 3), (4, 5), (4, 6), (5, 6), (5, 3)], true);
    let coloring = LargestDegreeFirstColoring::new(&g).coloring().unwrap();
    assert_eq!(coloring.count, 3);
    let c = colors_of(coloring);
    assert_eq!(
        (c[&1], c[&2], c[&3], c[&4], c[&5], c[&6]),
        (0, 0, 1, 1, 0, 2)
    );
}

#[test]
fn self_loops_are_rejected() {
    let g = graph_with(1..=2, &[(1, 1), (1, 2)], true);
    assert!(GreedyColoring::new(&g).coloring().is_err());
}

mod brown_backtrack_coloring_test {
    use super::*;

    fn verify(g: &G, chi: usize, coloring: &Coloring<i32>) {
        assert_eq!(coloring.count, chi);
        for e in g.edges() {
            assert_ne!(
                coloring.colors[g.edge_source(e).unwrap()],
                coloring.colors[g.edge_target(e).unwrap()]
            );
        }
        for v in g.vertices() {
            let c = coloring.colors[v];
            assert!((1..=chi).contains(&c));
        }
    }

    #[test]
    fn test_clique() {
        let mut g = G::simple().with_vertex_factory(|i| i as i32);
        Complete(6).generate_default(&mut g).unwrap();
        let bbc = BrownBacktrackColoring::new(&g);
        assert_eq!(bbc.chromatic_number(), 6);
        verify(&g, 6, &bbc.coloring().unwrap());
    }

    fn run(name: &str) {
        let c = case(name);
        let mut g = G::simple();
        for &(a, b) in c.edges {
            g.add_edge_with_vertices(a, b).unwrap();
        }
        let bbc = BrownBacktrackColoring::new(&g);
        assert_eq!(bbc.chromatic_number(), c.chromatic);
        verify(&g, c.chromatic, &bbc.coloring().unwrap());
    }

    #[test]
    fn myciel_3() {
        run("myciel3Test");
    }
    #[test]
    fn myciel_4() {
        run("myciel4Test");
    }
    #[test]
    fn queen_5() {
        run("queen5Test");
    }
    #[test]
    fn queen_6() {
        run("queen6Test");
    }
    #[test]
    #[ignore = "tagged optional in JGraphT: very slow"]
    fn insertions_2_3() {
        run("insertions2_3Test");
    }
    #[test]
    #[ignore = "tagged optional in JGraphT: very slow"]
    fn jean() {
        run("jeanTest");
    }
    #[test]
    fn huck() {
        run("huckTest");
    }
    #[test]
    fn mugg_100() {
        run("mugg100Test");
    }
}

mod chordal_graph_coloring_test {
    use super::*;
    use crate::common::construct_graph;

    fn assert_is_coloring(g: &G, coloring: &Coloring<i32>) {
        for v in g.vertices() {
            for e in g.edges_of(v) {
                let other = g.opposite_vertex(e, v).unwrap();
                if v != other {
                    assert_ne!(coloring.colors[v], coloring.colors[other]);
                }
            }
        }
    }

    fn undirected(edges: &[&[i32]]) -> G {
        let mut g = G::simple().weighted();
        construct_graph(&mut g, edges);
        g
    }

    #[test]
    fn test_get_coloring_1() {
        let g = undirected(&[]);
        let coloring = ChordalGraphColoring::new(&g).coloring().unwrap();
        assert_eq!(coloring.count, 0);
        assert!(coloring.colors.is_empty());
    }

    #[test]
    fn test_get_coloring_2() {
        let g = undirected(&[&[1, 2], &[1, 3], &[2, 3]]);
        let coloring = ChordalGraphColoring::new(&g).coloring().unwrap();
        assert_eq!(coloring.count, 3);
        assert_is_coloring(&g, &coloring);
    }

    #[test]
    fn test_get_coloring_3() {
        let g = undirected(&[&[1, 2], &[1, 3], &[2, 4], &[3, 4]]);
        assert!(ChordalGraphColoring::new(&g).coloring().is_none());
    }

    #[test]
    fn test_get_coloring_4() {
        let g = undirected(&[
            &[1, 2],
            &[2, 3],
            &[3, 4],
            &[4, 5],
            &[5, 6],
            &[6, 7],
            &[7, 8],
            &[8, 9],
            &[9, 10],
            &[10, 1],
            &[2, 4],
            &[4, 6],
            &[6, 8],
            &[8, 10],
            &[10, 2],
            &[2, 6],
            &[2, 8],
            &[4, 8],
            &[4, 10],
            &[6, 10],
        ]);
        let coloring = ChordalGraphColoring::new(&g).coloring().unwrap();
        assert_is_coloring(&g, &coloring);
        assert_eq!(coloring.count, 5);
    }

    #[test]
    fn test_get_coloring_5() {
        let mut g = G::pseudo().weighted();
        construct_graph(
            &mut g,
            &[
                &[1, 1],
                &[2, 2],
                &[2, 3],
                &[2, 3],
                &[2, 4],
                &[3, 4],
                &[3, 4],
                &[3, 4],
                &[4, 4],
                &[4, 4],
                &[5, 5],
                &[5, 5],
            ],
        );
        let coloring = ChordalGraphColoring::new(&g).coloring().unwrap();
        assert_is_coloring(&g, &coloring);
        assert_eq!(coloring.count, 3);
    }
}

mod color_refinement_algorithm_test {
    use super::*;

    fn colors(g: &impl Graph<Vertex = i32>) -> HashMap<i32, usize> {
        ColorRefinementAlgorithm::new(g).coloring().unwrap().colors
    }

    #[test]
    fn test_tree() {
        let g = graph_with(
            1..=8,
            &[(1, 2), (2, 3), (3, 4), (4, 5), (3, 6), (6, 7), (6, 8)],
            false,
        );
        let c = colors(&g);
        assert_eq!(c[&1], c[&5]);
        assert_ne!(c[&1], c[&7]);
        assert_eq!(c[&2], c[&4]);
        assert_eq!(c[&7], c[&8]);
        assert_ne!(c[&1], c[&2]);
        assert_ne!(c[&2], c[&3]);
        assert_ne!(c[&3], c[&6]);
    }

    #[test]
    fn test_regular() {
        let g = graph_with(
            1..=6,
            &[(1, 2), (2, 3), (3, 1), (4, 5), (5, 6), (6, 4)],
            false,
        );
        let c = colors(&g);
        for v in 2..=6 {
            assert_eq!(c[&1], c[&v]);
        }
    }

    #[test]
    fn test_graph_1() {
        let g = graph_with(
            1..=11,
            &[
                (1, 2),
                (2, 3),
                (2, 4),
                (2, 6),
                (2, 11),
                (3, 4),
                (4, 6),
                (5, 6),
                (6, 7),
                (7, 8),
                (8, 9),
                (8, 10),
                (8, 11),
                (9, 10),
                (9, 11),
                (10, 11),
            ],
            false,
        );
        let c = colors(&g);
        for i in 1..11 {
            for j in i + 1..=11 {
                if i != 9 || j != 10 {
                    assert_ne!(c[&i], c[&j]);
                }
            }
        }
        assert_eq!(c[&9], c[&10]);
    }

    #[test]
    fn test_directed_graph_1() {
        let mut g = DiGraph::<i32>::simple();
        g.add_vertices(1..=8);
        for (a, b) in [
            (1, 2),
            (2, 4),
            (3, 2),
            (4, 2),
            (4, 5),
            (4, 6),
            (5, 8),
            (6, 7),
            (7, 8),
            (8, 4),
            (8, 6),
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        let c = colors(&g);
        for i in 1..9 {
            for j in i + 1..9 {
                if (i == 1 && j == 3) || (i == 5 && j == 7) {
                    assert_eq!(c[&i], c[&j]);
                } else {
                    assert_ne!(c[&i], c[&j]);
                }
            }
        }
    }
}
