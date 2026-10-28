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

//! Ports of `org.jgrapht.generate` tests: GraphGeneratorTest, NamedGraphGeneratorTest, the Gnp/Gnm
//! (+bipartite), Barabási–Albert (+forest), directed scale-free, Kleinberg, LCD, Prüfer,
//! Watts–Strogatz, random regular, planted partition, complement, Petersen and windmill tests.
//!
//! Seeded tests use [`StdRng`] and therefore expect JGraphT's exact graphs.
//! Not ported: the `netgen` tests (network generator not implemented yet).

use jgrapht::alg::connectivity::connected_components;
use jgrapht::alg::isomorphism::VF2GraphIsomorphismInspector;
use jgrapht::alg::metrics::{GraphMeasurer, girth};
use jgrapht::generate::named::{self, build};
use jgrapht::generate::*;
use jgrapht::graph::Direction;
use jgrapht::graph::props::*;
use jgrapht::util::java::RandomSource;
use jgrapht::*;
use rand::RngCore;
use rand::{SeedableRng, rngs::StdRng};
use std::collections::HashSet;

const SEED: i64 = 5;

/// A graph of the given kind whose vertex factory yields `start, start + 1, ...`.
fn graph<D: Direction>(ty: GraphType, start: i32) -> BaseGraph<i32, DefaultEdge, D> {
    let g = BaseGraph::<i32, DefaultEdge, D>::with_graph_type(ty);
    if start == 0 {
        g.with_vertex_factory(|i| i as i32)
    } else {
        g.with_vertex_factory(|i| i as i32 + 1)
    }
}
fn simple_u(start: i32) -> UGraph<i32> {
    graph(GraphType::simple(), start)
}
fn pseudo_u(start: i32) -> UGraph<i32> {
    graph(GraphType::pseudograph(), start)
}
fn simple_d(start: i32) -> DiGraph<i32> {
    graph(GraphType::simple(), start)
}
fn pseudo_d(start: i32) -> DiGraph<i32> {
    graph(GraphType::pseudograph(), start)
}
fn loops_d(start: i32) -> DiGraph<i32> {
    graph(
        GraphType {
            self_loops: true,
            ..GraphType::simple()
        },
        start,
    )
}
fn rng(seed: i64) -> StdRng {
    StdRng::seed_from_u64((seed) as u64)
}
fn has_edges<G: Graph<Vertex = i32>>(g: &G, edges: &[(i32, i32)]) {
    for &(a, b) in edges {
        assert!(g.contains_edge_between(&a, &b), "missing edge ({a}, {b})");
    }
    assert_eq!(g.edge_count(), edges.len());
}

/// Gnp: the edge count of a seeded draw must lie within five standard deviations of `p * pairs`.
fn assert_gnp_edge_count<G: Graph<Vertex = i32>>(g: &G, pairs: usize, prob: f64) {
    let mean = pairs as f64 * prob;
    let sd = (pairs as f64 * prob * (1.0 - prob)).sqrt();
    assert!(
        (g.edge_count() as f64 - mean).abs() <= 5.0 * sd + 1.0,
        "{} edges, expected about {mean}",
        g.edge_count()
    );
}

/// Gnm: exactly `m` edges, no loops unless allowed, no parallel edges unless allowed.
fn assert_gnm_shape<G: Graph<Vertex = i32> + IncidenceGraph>(
    g: &G,
    m: usize,
    loops: bool,
    multi: bool,
    directed: bool,
) {
    assert_eq!(g.edge_count(), m);
    let mut seen = std::collections::HashSet::new();
    for e in g.edges() {
        let (a, b) = (*g.edge_source(e).unwrap(), *g.edge_target(e).unwrap());
        assert!(loops || a != b, "unexpected loop");
        let key = if directed || a <= b { (a, b) } else { (b, a) };
        assert!(multi || seen.insert(key), "unexpected parallel edge");
    }
}

/// Every edge joins the first partition `1..=n1` with the second one.
fn assert_bipartite_edges<G: Graph<Vertex = i32> + IncidenceGraph>(g: &G, n1: i32) {
    for e in g.edges() {
        let (a, b) = (*g.edge_source(e).unwrap(), *g.edge_target(e).unwrap());
        assert!((a <= n1) != (b <= n1), "edge ({a}, {b}) inside a partition");
    }
}

// ---------------------------------------------------------------- GraphGeneratorTest

mod graph_generator_test {
    use super::*;
    const SIZE: usize = 10;

    #[test]
    fn test_empty_graph_generator() {
        let mut g = loops_d(0);
        let mut result = GeneratorResult::new();
        Empty(SIZE)
            .generate_graph(&mut g, Some(&mut result))
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (SIZE, 0));
        assert!(result.is_empty());
    }

    #[test]
    fn test_linear_graph_generator() {
        let mut g = loops_d(0);
        let mut result = GeneratorResult::new();
        Linear(SIZE)
            .generate_graph(&mut g, Some(&mut result))
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (SIZE, SIZE - 1));
        let (start, end) = (result["Start Vertex"][0], result["End Vertex"][0]);
        for v in g.vertices() {
            let (i, o) = (g.in_degree_of(v), g.out_degree_of(v));
            if *v == start {
                assert_eq!((i, o), (0, 1));
            } else if *v == end {
                assert_eq!((i, o), (1, 0));
            } else {
                assert_eq!((i, o), (1, 1));
            }
        }
    }

    #[test]
    fn test_ring_graph_generator() {
        let mut g = loops_d(0);
        let mut result = GeneratorResult::new();
        Ring(SIZE)
            .generate_graph(&mut g, Some(&mut result))
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (SIZE, SIZE));
        let start = *g.vertices().next().unwrap();
        assert_eq!(g.out_degree_of(&start), 1);
        let mut next = start;
        let mut seen = HashSet::new();
        for _ in 0..SIZE {
            let e = g.out_edges_of(&next).next().unwrap();
            next = *g.edge_target(e).unwrap();
            assert_eq!((g.in_degree_of(&next), g.out_degree_of(&next)), (1, 1));
            assert!(seen.insert(next));
        }
        assert_eq!(next, start);
        assert!(result.is_empty());
    }

    #[test]
    fn test_complete_graph_generator() {
        let mut g = simple_u(0);
        Complete(10).generate_default(&mut g).unwrap();
        assert_eq!(g.edge_count(), 45);
    }

    #[test]
    fn test_complete_graph_generator_with_directed_graph() {
        let mut g = simple_d(0);
        Complete(10).generate_default(&mut g).unwrap();
        assert_eq!(g.edge_count(), 90);
    }

    #[test]
    fn test_complete_graph_generator_with_preexisting_vertices() {
        let mut g = simple_u(0);
        for _ in 0..10 {
            g.add_new_vertex().unwrap();
        }
        Complete(0).generate_default(&mut g).unwrap();
        assert_eq!(g.edge_count(), 45);
    }

    #[test]
    fn test_scale_free_graph_generator() {
        let mut g = loops_d(0);
        ScaleFreeGraphGenerator::new(500, StdRng::from_os_rng())
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert!(is_connected(&g), "generated graph is not connected");
        assert!(ScaleFreeGraphGenerator::new(-50, StdRng::seed_from_u64(31337)).is_err());
        let mut empty = loops_d(0);
        ScaleFreeGraphGenerator::new(0, StdRng::from_os_rng())
            .unwrap()
            .generate_default(&mut empty)
            .unwrap();
        assert_eq!(empty.vertex_count(), 0);
    }

    #[test]
    fn test_complete_bipartite_graph_generator() {
        let mut g = simple_u(0);
        CompleteBipartite(10, 4).generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (14, 40));
    }

    #[test]
    fn test_complete_bipartite_graph_generator_with_preexisting_vertices() {
        let mut g = simple_u(0);
        let a: Vec<i32> = (0..10).map(|_| g.add_new_vertex().unwrap()).collect();
        let b: Vec<i32> = (0..4).map(|_| g.add_new_vertex().unwrap()).collect();
        CompleteBipartite::between(&mut g, &a, &b).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (14, 40));
        assert!(CompleteBipartite::between(&mut g, &[], &b).is_err());
    }

    #[test]
    fn test_hyper_cube_graph_generator() {
        let mut g = simple_u(0);
        Hypercube(4).generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (16, 32));
    }

    #[test]
    fn test_star_graph_generator() {
        let mut g = simple_u(0);
        let mut map = GeneratorResult::new();
        Star(10).generate_graph(&mut g, Some(&mut map)).unwrap();
        assert_eq!((g.edge_count(), g.vertex_count()), (9, 10));
        assert!(map.contains_key("Center Vertex"));
    }

    fn string_graph<D: Direction>(ty: GraphType) -> BaseGraph<String, String, D> {
        BaseGraph::<String, String, D>::with_edge_factory(ty, |i| (i + 1).to_string())
            .with_vertex_factory(|i| (i + 1).to_string())
    }

    fn validate_grid<D: Direction>(
        rows: usize,
        cols: usize,
        g: &BaseGraph<String, String, D>,
        result: &GeneratorResult<String>,
    ) {
        assert_eq!(g.vertex_count(), rows * cols);
        assert_eq!(
            g.edge_count(),
            ((rows - 1) * cols + (cols - 1) * rows) * if g.graph_type().directed { 2 } else { 1 }
        );
        let (mut corner, mut border, mut inner) = (0, 0, 0);
        for v in g.vertices() {
            let neighbors: HashSet<String> = g.neighbor_set_of(v);
            match neighbors.len() {
                2 => corner += 1,
                3 => border += 1,
                4 => inner += 1,
                n => panic!("vertex with illegal number of neighbors ({n})."),
            }
        }
        assert_eq!(corner, 4);
        assert_eq!(border, ((rows - 2) * 2 + (cols - 2) * 2));
        assert_eq!(inner, ((rows - 2) * (cols - 2)));
        assert_eq!(result.len(), 4);
        for v in result.values() {
            assert_eq!(g.neighbor_set_of(&v[0]).len(), 2);
        }
    }

    #[test]
    fn test_grid_graph_generator() {
        let (rows, cols) = (3, 4);
        let mut result = GeneratorResult::new();
        let mut directed = string_graph::<jgrapht::graph::Directed>(GraphType {
            self_loops: true,
            ..GraphType::simple()
        });
        Grid { rows, cols }
            .generate_graph(&mut directed, Some(&mut result))
            .unwrap();
        validate_grid(rows, cols, &directed, &result);
        let mut result = GeneratorResult::new();
        let mut undirected = string_graph::<jgrapht::graph::Undirected>(GraphType::simple());
        Grid { rows, cols }
            .generate_graph(&mut undirected, Some(&mut result))
            .unwrap();
        validate_grid(rows, cols, &undirected, &result);
    }
}

// ---------------------------------------------------------------- NamedGraphGeneratorTest

mod named_graph_generator_test {
    use super::*;

    /// `validateBasics`: vertices, edges, radius, diameter and girth (`None` = acyclic).
    fn basics(
        g: &UGraph<usize>,
        vertices: usize,
        edges: usize,
        radius: f64,
        diameter: f64,
        girth_: Option<usize>,
    ) {
        assert_eq!((g.vertex_count(), g.edge_count()), (vertices, edges));
        let gm = GraphMeasurer::new(g).unwrap();
        assert!(
            (gm.radius().unwrap() - radius).abs() < 1e-8,
            "radius {:?} vs {radius}",
            gm.radius()
        );
        assert!(
            (gm.diameter().unwrap() - diameter).abs() < 1e-8,
            "diameter {:?} vs {diameter}",
            gm.diameter()
        );
        assert_eq!(girth(g), girth_);
    }
    fn automorphisms(g: &UGraph<usize>, expected: usize) {
        assert_eq!(
            VF2GraphIsomorphismInspector::new(g, g)
                .unwrap()
                .iter()
                .count(),
            expected
        );
    }
    const INF: Option<usize> = None;

    macro_rules! named_test {
        ($name:ident, $make:ident, $v:expr, $e:expr, $r:expr, $d:expr, $girth:expr $(, $check:expr)*) => {
            #[test]
            fn $name() {
                let g = build(named::$make());
                basics(&g, $v, $e, $r as f64, $d as f64, $girth);
                $( ($check)(&g); )*
            }
        };
    }

    named_test!(
        test_doyle_graph,
        doyle,
        27,
        54,
        3,
        3,
        Some(5),
        |g: &UGraph<usize>| assert!(is_eulerian(g)),
        |g| automorphisms(g, 54)
    );
    named_test!(test_bull_graph, bull, 5, 5, 2, 3, Some(3));
    named_test!(test_claw_graph, claw, 4, 3, 1, 2, INF, |g: &UGraph<
        usize,
    >| assert!(
        is_bipartite(g)
    ));
    named_test!(
        test_bucky_ball_graph,
        bucky_ball,
        60,
        90,
        9,
        9,
        Some(5),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(test_clebsch_graph, clebsch, 16, 40, 2, 2, Some(4), |g| {
        automorphisms(g, 1920)
    });
    named_test!(test_groetzsch_graph, grotzsch, 11, 20, 2, 2, Some(4));
    named_test!(
        test_bidiakis_cube_graph,
        bidiakis_cube,
        12,
        18,
        3,
        3,
        Some(4),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_blanusa_first_snark_graph,
        blanusa_first_snark,
        18,
        27,
        4,
        4,
        Some(5),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_blanusa_second_snark_graph,
        blanusa_second_snark,
        18,
        27,
        4,
        4,
        Some(5),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_double_star_snark_graph,
        double_star_snark,
        30,
        45,
        4,
        4,
        Some(6)
    );
    named_test!(
        test_brinkmann_graph,
        brinkmann,
        21,
        42,
        3,
        3,
        Some(5),
        |g: &UGraph<usize>| assert!(is_eulerian(g))
    );
    named_test!(test_gosset_graph, gosset, 56, 756, 3, 3, Some(3));
    named_test!(
        test_chvatal_graph,
        chvatal,
        12,
        24,
        2,
        2,
        Some(4),
        |g: &UGraph<usize>| assert!(is_eulerian(g))
    );
    named_test!(test_kittell_graph, kittell, 23, 63, 3, 4, Some(3));
    named_test!(
        test_coxeter_graph,
        coxeter,
        28,
        42,
        4,
        4,
        Some(7),
        |g: &UGraph<usize>| assert!(is_cubic(g)),
        |g| automorphisms(g, 336)
    );
    named_test!(test_diamond_graph, diamond, 4, 5, 1, 2, Some(3));
    named_test!(
        test_ellingham_horton_54_graph,
        ellingham_horton54,
        54,
        81,
        9,
        10,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g)),
        |g| automorphisms(g, 32)
    );
    named_test!(
        test_ellingham_horton_78_graph,
        ellingham_horton78,
        78,
        117,
        7,
        13,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g)),
        |g| automorphisms(g, 16)
    );
    named_test!(test_errera_graph, errera, 17, 45, 3, 4, Some(3));
    named_test!(
        test_folkman_graph,
        folkman,
        20,
        40,
        3,
        4,
        Some(4),
        |g: &UGraph<usize>| assert!(is_bipartite(g) && is_eulerian(g)),
        |g| automorphisms(g, 3840)
    );
    named_test!(
        test_franklin_graph,
        franklin,
        12,
        18,
        3,
        3,
        Some(4),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g)),
        |g| automorphisms(g, 48)
    );
    named_test!(
        test_frucht_graph,
        frucht,
        12,
        18,
        3,
        4,
        Some(3),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_goldner_harary_graph,
        goldner_harary,
        11,
        27,
        2,
        2,
        Some(3)
    );
    named_test!(
        test_heawood_graph,
        heawood,
        14,
        21,
        3,
        3,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g)),
        |g| automorphisms(g, 336)
    );
    named_test!(
        test_herschel_graph,
        herschel,
        11,
        18,
        3,
        4,
        Some(4),
        |g: &UGraph<usize>| assert!(is_bipartite(g)),
        |g| automorphisms(g, 12)
    );
    named_test!(
        test_hoffman_graph,
        hoffman,
        16,
        32,
        3,
        4,
        Some(4),
        |g: &UGraph<usize>| assert!(is_bipartite(g)),
        |g| automorphisms(g, 48)
    );
    named_test!(
        test_krackhardt_kite_graph,
        krackhardt_kite,
        10,
        18,
        2,
        4,
        Some(3)
    );
    named_test!(
        test_klein_3_regular_graph,
        klein3_regular,
        56,
        84,
        6,
        6,
        Some(7),
        |g: &UGraph<usize>| assert!(is_cubic(g)),
        |g| automorphisms(g, 336)
    );
    named_test!(
        test_klein_7_regular_graph,
        klein7_regular,
        24,
        84,
        3,
        3,
        Some(3),
        |g| automorphisms(g, 336)
    );
    named_test!(
        test_moser_spindle_graph,
        moser_spindle,
        7,
        11,
        2,
        2,
        Some(3),
        |g| automorphisms(g, 8)
    );
    named_test!(
        test_pappus_graph,
        pappus,
        18,
        27,
        4,
        4,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g)),
        |g| automorphisms(g, 216)
    );
    named_test!(test_poussin_graph, poussin, 15, 39, 3, 3, Some(3));
    named_test!(test_schlaefli_graph, schlafli, 27, 216, 2, 2, Some(3));
    named_test!(
        test_tietze_graph,
        tietze,
        12,
        18,
        3,
        3,
        Some(3),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_tutte_graph,
        tutte,
        46,
        69,
        5,
        8,
        Some(4),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_thomsen_graph,
        thomsen,
        6,
        9,
        2,
        2,
        Some(4),
        |g: &UGraph<usize>| assert!(is_bipartite(g))
    );

    // GeneralizedPetersenGraphGeneratorTest
    #[test]
    fn test_cubical_graph() {
        let mut g = simple_u(1);
        GeneralizedPetersen { n: 4, k: 1 }
            .generate_default(&mut g)
            .unwrap();
        let gm = GraphMeasurer::new(&g).unwrap();
        assert_eq!(
            (
                g.vertex_count(),
                g.edge_count(),
                gm.radius(),
                gm.diameter(),
                girth(&g)
            ),
            (8, 12, Some(3.0), Some(3.0), Some(4))
        );
        assert!(is_bipartite(&g) && is_cubic(&g));
    }
    named_test!(
        test_petersen_graph,
        petersen,
        10,
        15,
        2,
        2,
        Some(5),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_duerer_graph,
        durer,
        12,
        18,
        3,
        4,
        Some(3),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_dodecahedron_graph,
        dodecahedron,
        20,
        30,
        5,
        5,
        Some(5),
        |g: &UGraph<usize>| assert!(is_cubic(g))
    );
    named_test!(
        test_desargues_graph,
        desargues,
        20,
        30,
        5,
        5,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g))
    );
    named_test!(
        test_nauru_graph,
        nauru,
        24,
        36,
        4,
        4,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g))
    );
    named_test!(
        test_moebius_kantor_graph,
        mobius_kantor,
        16,
        24,
        4,
        4,
        Some(6),
        |g: &UGraph<usize>| assert!(is_cubic(g) && is_bipartite(g))
    );
}

// ---------------------------------------------------------------- Windmill / GeneralizedPetersen validations

mod windmill_graphs_generator_test {
    use super::*;

    fn verify_degree(g: &UGraph<i32>, mode: WindmillMode, m: usize, n: usize) {
        let vs: Vec<i32> = g.vertices_cloned().collect();
        if mode == WindmillMode::DutchWindmill {
            assert_eq!(g.degree_of(&vs[0]), 2 * m);
            assert!(vs[1..].iter().all(|v| g.degree_of(v) == 2));
        } else {
            assert_eq!(g.degree_of(&vs[0]), m * (n - 1));
            assert!(vs[1..].iter().all(|v| g.degree_of(v) == n - 1));
        }
    }

    fn windmill(mode: WindmillMode, m: usize, n: usize, vertices: usize, edges: usize) {
        let mut g = simple_u(0);
        Windmill { mode, m, n }.generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (vertices, edges));
        verify_degree(&g, mode, m, n);
    }

    #[test]
    fn test_graph_1a() {
        windmill(WindmillMode::Windmill, 3, 4, 10, 18);
    }
    #[test]
    fn test_graph_2a() {
        windmill(WindmillMode::Windmill, 4, 3, 9, 12);
    }
    #[test]
    fn test_graph_3a() {
        windmill(WindmillMode::Windmill, 3, 5, 13, 30);
    }
    #[test]
    fn test_graph_2b() {
        windmill(WindmillMode::DutchWindmill, 4, 3, 9, 12);
    }
    #[test]
    fn test_graph_3b() {
        windmill(WindmillMode::DutchWindmill, 3, 5, 13, 15);
    }
    #[test]
    fn test_butterfly_graph() {
        let g = build(named::butterfly());
        assert_eq!((g.vertex_count(), g.edge_count()), (5, 6));
        let gm = GraphMeasurer::new(&g).unwrap();
        assert_eq!(
            (gm.radius(), gm.diameter(), girth(&g)),
            (Some(1.0), Some(2.0), Some(3))
        );
        assert!(is_eulerian(&g));
    }
    #[test]
    fn invalid_parameters_are_rejected() {
        let mut g = simple_u(0);
        assert!(
            Windmill {
                mode: WindmillMode::Windmill,
                m: 1,
                n: 4
            }
            .generate_default(&mut g)
            .is_err()
        );
        assert!(
            Windmill {
                mode: WindmillMode::Windmill,
                m: 2,
                n: 2
            }
            .generate_default(&mut g)
            .is_err()
        );
        assert!(
            GeneralizedPetersen { n: 2, k: 1 }
                .generate_default(&mut g)
                .is_err()
        );
        assert!(
            GeneralizedPetersen { n: 5, k: 3 }
                .generate_default(&mut g)
                .is_err()
        );
        assert_eq!(g.vertex_count(), 0);
    }
}

// ---------------------------------------------------------------- Gnp

mod gnp_random_graph_generator_test {
    use super::*;

    fn gnp(n: usize, p: f64, loops: bool) -> GnpRandomGraphGenerator<StdRng> {
        GnpRandomGraphGenerator::new(n, p, rng(SEED), loops).unwrap()
    }

    #[test]
    fn test_zero_nodes() {
        let mut g = pseudo_d(0);
        GnpRandomGraphGenerator::new(0, 1.0, StdRng::from_os_rng(), false)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.edge_count(), g.vertex_count()), (0, 0));
    }

    #[test]
    fn test_bad_parameters() {
        assert!(GnpRandomGraphGenerator::new(10, -1.0, rng(1), false).is_err());
        assert!(GnpRandomGraphGenerator::new(10, 2.0, rng(1), false).is_err());
    }

    #[test]
    fn test_directed_graph_gnp_1() {
        let mut g = pseudo_d(1);
        gnp(6, 0.5, false).generate_default(&mut g).unwrap();
        assert_eq!(g.vertex_count(), 6);
        assert_gnp_edge_count(&g, 30, 0.5);
    }

    #[test]
    fn test_directed_graph_gnp_2() {
        let mut g = pseudo_d(0);
        gnp(6, 1.0, false).generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (6, 30));
    }

    #[test]
    fn test_directed_graph_gnp_3() {
        let mut g = pseudo_d(1);
        gnp(6, 0.1, false).generate_default(&mut g).unwrap();
        assert_eq!(g.vertex_count(), 6);
        assert_gnp_edge_count(&g, 30, 0.1);
    }

    #[test]
    fn test_directed_graph_gnp_4_with_loops() {
        let mut g = pseudo_d(1);
        gnp(6, 0.2, true).generate_default(&mut g).unwrap();
        assert_gnp_edge_count(&g, 36, 0.2);
    }

    #[test]
    fn test_undirected_graph_gnp_1() {
        let mut g = pseudo_u(1);
        gnp(6, 0.5, false).generate_default(&mut g).unwrap();
        assert_gnp_edge_count(&g, 15, 0.5);
    }

    #[test]
    fn test_undirected_graph_gnp_2() {
        let mut g = pseudo_u(1);
        gnp(6, 1.0, false).generate_default(&mut g).unwrap();
        assert_eq!(g.edge_count(), 15);
        assert!((1..=6).all(|a| (a + 1..=6).all(|b| g.contains_edge_between(&a, &b))));
    }

    #[test]
    fn test_undirected_graph_gnp_3() {
        let mut g = pseudo_u(1);
        gnp(6, 0.3, false).generate_default(&mut g).unwrap();
        assert_gnp_edge_count(&g, 15, 0.3);
    }

    #[test]
    fn test_undirected_graph_gnp_4_with_loops() {
        let mut g = pseudo_u(1);
        gnp(6, 0.3, true).generate_default(&mut g).unwrap();
        assert_gnp_edge_count(&g, 21, 0.3);
    }
}

// ---------------------------------------------------------------- Gnm

mod gnm_random_graph_generator_test {
    use super::*;

    fn gnm(n: usize, m: usize, loops: bool, multi: bool) -> GnmRandomGraphGenerator<StdRng> {
        GnmRandomGraphGenerator::new(n, m, rng(SEED), loops, multi).unwrap()
    }

    #[test]
    fn test_zero_nodes_and_edges() {
        let mut g = pseudo_d(0);
        gnm(0, 0, false, false).generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
        let mut g = pseudo_d(0);
        gnm(10, 0, false, false).generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (10, 0));
    }

    #[test]
    fn test_directed_graph_gnp_1() {
        let mut g = pseudo_d(1);
        gnm(6, 18, false, false).generate_default(&mut g).unwrap();
        assert_gnm_shape(&g, 18, false, false, true);
    }

    #[test]
    fn test_directed_graph_gnp_1_with_loops() {
        let mut g = pseudo_d(1);
        gnm(6, 18, true, false).generate_default(&mut g).unwrap();
        assert_gnm_shape(&g, 18, true, false, true);
    }

    #[test]
    fn test_directed_graph_gnp_1_with_multiple_edges() {
        let mut g = pseudo_d(1);
        gnm(6, 18, false, true).generate_default(&mut g).unwrap();
        assert_gnm_shape(&g, 18, false, true, true);
    }

    #[test]
    fn test_directed_graph_gnp_1_with_loops_and_multiple_edges() {
        let mut g = pseudo_d(1);
        gnm(6, 18, true, true).generate_default(&mut g).unwrap();
        assert_gnm_shape(&g, 18, true, true, true);
    }

    #[test]
    fn test_undirected_graph_gnp_1() {
        let mut g = pseudo_u(1);
        gnm(6, 15, false, false).generate_default(&mut g).unwrap();
        assert_gnm_shape(&g, 15, false, false, false);
    }

    #[test]
    fn test_undirected_graph_gnp_1_with_loops() {
        let mut g = pseudo_u(1);
        gnm(6, 15, true, false).generate_default(&mut g).unwrap();
        assert_gnm_shape(&g, 15, true, false, false);
    }

    #[test]
    fn test_not_allowed_loops_or_multiple_edges() {
        let mut g = simple_d(0);
        assert!(gnm(6, 18, true, false).generate_default(&mut g).is_err());
        let mut g = simple_d(0);
        assert!(gnm(6, 18, false, true).generate_default(&mut g).is_err());
    }

    #[test]
    fn test_edge_limits_directed() {
        let mut g1 = pseudo_d(0);
        assert!(gnm(5, 21, false, false).generate_default(&mut g1).is_err());
        let mut g2 = pseudo_d(0);
        gnm(5, 20, false, false).generate_default(&mut g2).unwrap();
        let mut g3 = pseudo_d(0);
        gnm(5, 25, true, false).generate_default(&mut g3).unwrap();
        let mut g4 = pseudo_d(0);
        gnm(5, 25, false, true).generate_default(&mut g4).unwrap();
    }

    #[test]
    fn test_edge_limits_undirected() {
        let mut g1 = simple_u(0);
        assert!(gnm(5, 11, false, false).generate_default(&mut g1).is_err());
        let mut g2 = simple_u(0);
        gnm(5, 10, false, false).generate_default(&mut g2).unwrap();
        let mut g3 = pseudo_u(0);
        gnm(5, 15, true, false).generate_default(&mut g3).unwrap();
        let mut g4 = pseudo_u(0);
        gnm(5, 15, false, true).generate_default(&mut g4).unwrap();
    }

    #[test]
    fn test_maximum_allowed_edges() {
        type G = GnmRandomGraphGenerator<StdRng>;
        let max = i32::MAX as usize;
        for (directed, loops_edges) in [
            (false, [0, 0, 0, max, 45, 55, max, max]),
            (true, [0, 0, 0, max, 90, 110, max, max]),
        ] {
            // n = 0
            for (l, m) in [(false, false), (false, true), (true, false), (true, true)] {
                assert_eq!(G::maximum_allowed_edges(0, directed, l, m), 0);
            }
            assert_eq!(G::maximum_allowed_edges(1, directed, false, false), 0);
            assert_eq!(
                G::maximum_allowed_edges(1, directed, true, false),
                if directed { 2 } else { 1 }
            );
            assert_eq!(G::maximum_allowed_edges(1, directed, false, true), 0);
            assert_eq!(G::maximum_allowed_edges(1, directed, true, true), max);
            assert_eq!(
                G::maximum_allowed_edges(10, directed, false, false),
                loops_edges[4]
            );
            assert_eq!(
                G::maximum_allowed_edges(10, directed, true, false),
                loops_edges[5]
            );
            assert_eq!(G::maximum_allowed_edges(10, directed, false, true), max);
            assert_eq!(G::maximum_allowed_edges(10, directed, true, true), max);
            assert_eq!(
                G::maximum_allowed_edges(200000, directed, false, false),
                max
            );
        }
    }

    /// Same seed => same sequence of edges (and generating twice with one generator continues the stream).
    fn same_topology<G: Graph<Vertex = i32>>(a: &G, b: &G) -> bool {
        let order = |g: &G| -> std::collections::HashMap<i32, usize> {
            g.vertices().enumerate().map(|(i, v)| (*v, i)).collect()
        };
        let (oa, ob) = (order(a), order(b));
        let edges = |g: &G, o: &std::collections::HashMap<i32, usize>| -> HashSet<(usize, usize)> {
            let mut s = HashSet::new();
            for e in g.edges() {
                let (x, y) = (o[g.edge_source(e).unwrap()], o[g.edge_target(e).unwrap()]);
                s.insert((x, y));
                if !g.graph_type().directed {
                    s.insert((y, x));
                }
            }
            s
        };
        edges(a, &oa) == edges(b, &ob)
    }

    #[test]
    fn test_generate_directed_graph() {
        let (mut g0, mut g1, mut g2, mut g3) = (simple_d(0), simple_d(0), simple_d(0), simple_d(0));
        let mut first = GnmRandomGraphGenerator::new(11, 100, rng(17), false, false).unwrap();
        first.generate_default(&mut g0).unwrap();
        first.generate_default(&mut g1).unwrap();
        let mut second = GnmRandomGraphGenerator::new(11, 100, rng(17), false, false).unwrap();
        second.generate_default(&mut g2).unwrap();
        second.generate_default(&mut g3).unwrap();
        assert!(same_topology(&g0, &g2));
        assert!(same_topology(&g1, &g3));
    }

    #[test]
    fn test_generate_listenable_undirected_graph() {
        let make = || jgrapht::event::Listenable::new(simple_u(0));
        let (mut g0, mut g1, mut g2, mut g3) = (make(), make(), make(), make());
        let mut first = GnmRandomGraphGenerator::new(11, 50, rng(17), false, false).unwrap();
        first.generate_default(&mut g0).unwrap();
        first.generate_default(&mut g1).unwrap();
        let mut second = GnmRandomGraphGenerator::new(11, 50, rng(17), false, false).unwrap();
        second.generate_default(&mut g2).unwrap();
        second.generate_default(&mut g3).unwrap();
        assert!(same_topology(&g0, &g2));
        assert!(same_topology(&g1, &g3));
    }

    #[test]
    fn test_bad_vertex_factory() {
        // a factory that always yields the same vertex cannot produce 10 distinct vertices
        let mut g = BaseGraph::<String, DefaultEdge, jgrapht::graph::Directed>::simple()
            .with_vertex_factory(|_| "v".to_string());
        assert!(
            GnmRandomGraphGenerator::new(10, 3, StdRng::from_os_rng(), false, false)
                .unwrap()
                .generate_default(&mut g)
                .is_err()
        );
    }
}

// ---------------------------------------------------------------- bipartite Gnm / Gnp

mod bipartite_random_generators {
    use super::*;

    #[test]
    fn gnm_zero_nodes() {
        let mut g = pseudo_d(0);
        GnmRandomBipartiteGraphGenerator::new(0, 0, 10, StdRng::from_os_rng())
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
    }

    #[test]
    fn gnm_directed() {
        let mut g = pseudo_d(1);
        GnmRandomBipartiteGraphGenerator::new(4, 4, 20, rng(SEED))
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 8);
        assert_eq!(g.edge_count(), 20);
        assert_bipartite_edges(&g, 4);
    }

    #[test]
    fn gnm_undirected() {
        let mut g = simple_u(1);
        GnmRandomBipartiteGraphGenerator::new(4, 4, 10, rng(SEED))
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.edge_count(), 10);
        assert_bipartite_edges(&g, 4);
    }

    #[test]
    fn gnm_edges_limit() {
        let mut g = simple_u(0);
        assert!(
            GnmRandomBipartiteGraphGenerator::new(4, 4, 17, rng(SEED))
                .generate_default(&mut g)
                .is_err()
        );
        let mut g = simple_d(0);
        assert!(
            GnmRandomBipartiteGraphGenerator::new(4, 4, 33, rng(SEED))
                .generate_default(&mut g)
                .is_err()
        );
    }

    #[test]
    fn gnp_zero_nodes_and_bad_parameters() {
        let mut g = pseudo_d(1);
        GnpRandomBipartiteGraphGenerator::new(0, 0, 0.5, StdRng::from_os_rng())
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
        assert!(GnpRandomBipartiteGraphGenerator::new(10, 10, -1.0, rng(1)).is_err());
        assert!(GnpRandomBipartiteGraphGenerator::new(10, 10, 2.0, rng(1)).is_err());
    }

    #[test]
    fn gnp_directed() {
        let mut g = pseudo_d(1);
        GnpRandomBipartiteGraphGenerator::new(4, 4, 0.5, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_gnp_edge_count(&g, 32, 0.5);
        assert_bipartite_edges(&g, 4);
        let mut g = pseudo_d(1);
        GnpRandomBipartiteGraphGenerator::new(4, 4, 1.0, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (8, 32));
        let mut g = pseudo_d(1);
        GnpRandomBipartiteGraphGenerator::new(4, 4, 0.1, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_gnp_edge_count(&g, 32, 0.1);
        assert_bipartite_edges(&g, 4);
    }

    #[test]
    fn gnp_undirected() {
        let mut g = simple_u(1);
        GnpRandomBipartiteGraphGenerator::new(4, 4, 0.5, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_gnp_edge_count(&g, 16, 0.5);
        assert_bipartite_edges(&g, 4);
        let mut g = simple_u(1);
        GnpRandomBipartiteGraphGenerator::new(4, 4, 1.0, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (8, 16));
        let mut g = simple_u(1);
        GnpRandomBipartiteGraphGenerator::new(4, 4, 0.0, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (8, 0));
    }

    #[test]
    fn partitions_are_returned() {
        let mut g = simple_u(0);
        let (a, b) = GnpRandomBipartiteGraphGenerator::new(3, 5, 0.5, rng(1))
            .unwrap()
            .generate_partitions(&mut g)
            .unwrap();
        assert_eq!((a.len(), b.len()), (3, 5));
        let (small, large) = ordered_partitions(b, a);
        assert_eq!((small.len(), large.len()), (3, 5));
    }
}

// ---------------------------------------------------------------- Barabási–Albert

mod barabasi_albert_tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_bad_parameters() {
        assert!(BarabasiAlbertGraphGenerator::new(0, 10, 100, rng(1)).is_err());
        assert!(BarabasiAlbertGraphGenerator::new(2, 0, 100, rng(1)).is_err());
        assert!(BarabasiAlbertGraphGenerator::new(2, 3, 100, rng(1)).is_err());
        assert!(BarabasiAlbertGraphGenerator::new(3, 2, 2, rng(1)).is_err());
    }

    #[test]
    fn test_undirected_and_directed() {
        for (m0, m, n, seed) in [(3, 2, 10, 5), (1, 1, 20, 7)] {
            let mut g = simple_u(1);
            BarabasiAlbertGraphGenerator::new(m0, m, n, rng(seed))
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!(g.vertex_count(), n);
        }
        for (m0, m, n, seed) in [(3, 2, 10, 5), (1, 1, 20, 13)] {
            let mut g = simple_d(1);
            BarabasiAlbertGraphGenerator::new(m0, m, n, rng(seed))
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!(g.vertex_count(), n);
        }
    }

    #[test]
    fn test_undirected_with_graph_which_already_has_some_vertices() {
        let mut g = simple_u(1);
        g.add_vertex(1000);
        BarabasiAlbertGraphGenerator::new(3, 2, 10, rng(5))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 11);
    }

    #[test]
    fn forest_bad_parameters() {
        assert!(BarabasiAlbertForestGenerator::new(0, 10, rng(1)).is_err());
        assert!(BarabasiAlbertForestGenerator::new(10, 9, rng(1)).is_err());
    }

    #[test]
    fn forest_undirected() {
        let mut g = simple_u(1);
        BarabasiAlbertForestGenerator::new(5, 20, rng(5))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 20);
        assert_eq!(connected_components(&g).len(), 5);
        let mut g = simple_u(1);
        BarabasiAlbertForestGenerator::new(20, 20, StdRng::from_os_rng())
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(connected_components(&g).len(), 20);
        let mut g = simple_u(1);
        BarabasiAlbertForestGenerator::new(1, 20, rng(7))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(connected_components(&g).len(), 1);
    }

    #[test]
    fn forest_rejects_directed_and_nonempty_targets() {
        let mut g = simple_d(1);
        assert!(
            BarabasiAlbertForestGenerator::new(2, 10, rng(5))
                .unwrap()
                .generate_default(&mut g)
                .is_err()
        );
        let mut g = simple_u(1);
        g.add_vertex(1000);
        assert!(
            BarabasiAlbertForestGenerator::new(3, 10, rng(5))
                .unwrap()
                .generate_default(&mut g)
                .is_err()
        );
    }

    #[test]
    fn forest_random_trees() {
        let mut random = rng(0x88);
        for _ in 0..2000 {
            let n = 10 + random.next_int_bound(100);
            let t = 1 + random.next_int_bound(n);
            let mut g = simple_u(1);
            BarabasiAlbertForestGenerator::new(t, n, StdRng::from_os_rng())
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!(g.vertex_count(), n);
            assert_eq!(connected_components(&g).len(), t);
        }
    }
}

// ---------------------------------------------------------------- directed scale-free, Kleinberg, LCD

mod scale_free_kleinberg_lcd {
    use super::*;
    const S: i64 = 17;

    fn dsf(
        a: f32,
        g_: f32,
        di: f32,
        d_o: f32,
        e: i64,
        n: i64,
    ) -> Result<DirectedScaleFreeGraphGenerator<StdRng>> {
        DirectedScaleFreeGraphGenerator::new(a, g_, di, d_o, e, n, rng(S))
    }

    #[test]
    fn dsf_bad_parameters() {
        assert!(dsf(-0.5, 0.33, 0.5, 0.5, 500, 500).is_err());
        assert!(dsf(0.33, -0.5, 0.5, 0.5, 500, 500).is_err());
        assert!(dsf(0.66, 0.66, 0.5, 0.5, 500, 500).is_err());
        assert!(dsf(0.33, 0.33, -0.5, 0.5, 500, 500).is_err());
        assert!(dsf(0.33, 0.33, 0.5, -0.5, 500, 500).is_err());
        assert!(dsf(0.33, 0.33, 0.5, 0.5, -1, -1).is_err());
    }

    #[test]
    fn dsf_incompatible_graph() {
        let mut generator = dsf(0.33, 0.33, 0.5, 0.5, 1000, 0).unwrap();
        generator.set_allowing_multiple_edges(true);
        generator.set_allowing_self_loops(false);
        assert!(generator.generate_default(&mut loops_d(0)).is_err());
        let mut generator = dsf(0.33, 0.33, 0.5, 0.5, 1000, 0).unwrap();
        generator.set_allowing_multiple_edges(false);
        generator.set_allowing_self_loops(true);
        let mut multigraph: DiGraph<i32> = graph(GraphType::multigraph(), 0);
        assert!(generator.generate_default(&mut multigraph).is_err());
    }

    #[test]
    fn dsf_number_of_edges_and_nodes() {
        let mut generator = dsf(0.33, 0.33, 0.5, 0.5, 1000, 0).unwrap();
        generator.set_allowing_multiple_edges(false);
        let mut g = pseudo_d(0);
        generator.generate_default(&mut g).unwrap();
        assert_eq!(g.edge_count(), 1000);
        let mut generator = dsf(0.33, 0.33, 0.5, 0.5, -1, 1000).unwrap();
        generator.set_allowing_multiple_edges(false);
        let mut g = loops_d(0);
        generator.generate_default(&mut g).unwrap();
        assert_eq!(g.vertex_count(), 1000);
    }

    #[test]
    fn dsf_zero_cases() {
        let mut generator = dsf(0.33, 0.33, 0.5, 0.5, -1, 0).unwrap();
        generator.set_allowing_multiple_edges(false);
        let mut g = pseudo_d(0);
        generator.generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
        let mut generator = dsf(0.33, 0.33, 0.5, 0.5, 0, 0).unwrap();
        let mut g = pseudo_d(0);
        generator.generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
    }

    #[test]
    fn dsf_no_out_or_in_degree_zero() {
        let mut generator = dsf(0.3, 0.0, 0.5, 0.5, -1, 1000).unwrap();
        generator.set_allowing_multiple_edges(false);
        let mut g = loops_d(0);
        generator.generate_default(&mut g).unwrap();
        assert_eq!(g.vertices().filter(|v| g.out_degree_of(v) == 0).count(), 0);
        let mut generator = dsf(0.0, 0.3, 0.5, 0.5, -1, 1000).unwrap();
        generator.set_allowing_multiple_edges(false);
        let mut g = loops_d(0);
        generator.generate_default(&mut g).unwrap();
        assert_eq!(g.vertices().filter(|v| g.in_degree_of(v) == 0).count(), 0);
    }

    #[test]
    fn kleinberg_bad_parameters() {
        assert!(KleinbergSmallWorldGraphGenerator::new(-1, 1, 1, 1, rng(1)).is_err());
        assert!(KleinbergSmallWorldGraphGenerator::new(5, 0, 1, 1, rng(1)).is_err());
        assert!(KleinbergSmallWorldGraphGenerator::new(5, 9, 1, 1, rng(1)).is_err());
        assert!(KleinbergSmallWorldGraphGenerator::new(5, 1, -1, 1, rng(1)).is_err());
        assert!(KleinbergSmallWorldGraphGenerator::new(5, 1, 1, -1, rng(1)).is_err());
    }

    #[test]
    fn kleinberg_graphs() {
        let mut g = simple_u(0);
        KleinbergSmallWorldGraphGenerator::new(5, 2, 3, 2, rng(5))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 25);
        let mut g = simple_d(0);
        KleinbergSmallWorldGraphGenerator::new(5, 2, 3, 2, rng(5))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 25);
        let mut g = simple_d(0);
        KleinbergSmallWorldGraphGenerator::new(5, 2, 3, 0, rng(5))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 25);
    }

    #[test]
    fn lcd_bad_parameters() {
        assert!(LinearizedChordDiagramGraphGenerator::new(0, 10, rng(1)).is_err());
        assert!(LinearizedChordDiagramGraphGenerator::new(5, 0, rng(1)).is_err());
    }

    #[test]
    fn lcd_rejects_graphs_without_parallel_edges() {
        for mut g in [
            graph::<jgrapht::graph::Undirected>(GraphType::multigraph(), 1),
            graph(GraphType::simple(), 1),
        ] {
            assert!(
                LinearizedChordDiagramGraphGenerator::new(10, 2, rng(5))
                    .unwrap()
                    .generate_default(&mut g)
                    .is_err()
            );
        }
        for mut g in [
            graph::<jgrapht::graph::Directed>(GraphType::multigraph(), 1),
            graph(GraphType::simple(), 1),
        ] {
            assert!(
                LinearizedChordDiagramGraphGenerator::new(10, 2, rng(5))
                    .unwrap()
                    .generate_default(&mut g)
                    .is_err()
            );
        }
    }

    #[test]
    fn lcd_pseudographs() {
        for m in [1, 2] {
            let mut g = pseudo_u(1);
            LinearizedChordDiagramGraphGenerator::new(20, m, rng(5))
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!(g.vertex_count(), 20);
        }
        let mut g = pseudo_d(1);
        LinearizedChordDiagramGraphGenerator::new(20, 1, rng(5))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 20);
    }
}

// ---------------------------------------------------------------- Prüfer, Watts–Strogatz, random regular

mod prufer_watts_regular {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn prufer_sequences() {
        let mut tree = simple_u(1);
        PruferTreeGenerator::from_sequence(&[])
            .unwrap()
            .generate_default(&mut tree)
            .unwrap();
        assert_eq!(tree.vertex_count(), 2);
        assert!(PruferTreeGenerator::from_sequence(&[10]).is_err());
        let mut tree = simple_u(1);
        PruferTreeGenerator::from_sequence(&[4, 4, 4, 5])
            .unwrap()
            .generate_default(&mut tree)
            .unwrap();
        assert_eq!(tree.vertex_count(), 6);
        let mut degrees: Vec<usize> = tree.vertices().map(|v| tree.degree_of(v)).collect();
        degrees.sort_unstable();
        assert_eq!(degrees, [1, 1, 1, 1, 2, 4]);
    }

    #[test]
    fn prufer_errors() {
        assert!(PruferTreeGenerator::new(0, rng(1)).is_err());
        let mut dag: DiGraph<i32> = graph(
            GraphType {
                allows_cycles: false,
                ..GraphType::simple()
            },
            1,
        );
        assert!(
            PruferTreeGenerator::new(10, StdRng::from_os_rng())
                .unwrap()
                .generate_default(&mut dag)
                .is_err()
        );
        let mut tree = simple_u(0);
        Complete(10).generate_default(&mut tree).unwrap();
        assert!(
            PruferTreeGenerator::new(100, rng(0x99))
                .unwrap()
                .generate_default(&mut tree)
                .is_err()
        );
    }

    #[test]
    fn prufer_trees() {
        let mut tree = simple_u(0);
        PruferTreeGenerator::new(1, rng(0x99))
            .unwrap()
            .generate_default(&mut tree)
            .unwrap();
        assert!(is_tree(&tree));
        let mut random = rng(0x88);
        for _ in 0..500 {
            let mut tree = simple_u(1);
            let n = 1 + random.next_int_bound(5000);
            PruferTreeGenerator::new(n, StdRng::seed_from_u64((random.next_u64() as i64) as u64))
                .unwrap()
                .generate_default(&mut tree)
                .unwrap();
            assert!(is_tree(&tree));
        }
        let mut tree = simple_u(0);
        PruferTreeGenerator::new(100_000, rng(0x99))
            .unwrap()
            .generate_default(&mut tree)
            .unwrap();
        assert!(is_tree(&tree));
    }

    fn ws(n: usize, k: usize, p: f64, seed: i64) -> Result<WattsStrogatzGraphGenerator<StdRng>> {
        WattsStrogatzGraphGenerator::new(n, k, p, false, rng(seed))
    }

    #[test]
    fn watts_strogatz_bad_parameters() {
        assert!(ws(2, 1, 0.5, 1).is_err());
        assert!(ws(10, 9, 0.5, 1).is_err());
        assert!(ws(11, 11, 0.5, 1).is_err());
        assert!(ws(10, 2, -1.0, 1).is_err());
        assert!(ws(10, 2, 2.0, 1).is_err());
    }

    #[test]
    fn watts_strogatz_graphs() {
        for (n, k, p, edges) in [
            (6, 4, 0.0, 12),
            (6, 4, 0.5, 12),
            (6, 4, 0.8, 12),
            (12, 6, 0.0, 36),
            (12, 6, 0.7, 36),
        ] {
            let mut g = simple_u(0);
            ws(n, k, p, SEED).unwrap().generate_default(&mut g).unwrap();
            assert_eq!(
                (g.vertex_count(), g.edge_count()),
                (n, edges),
                "n={n} k={k} p={p}"
            );
        }
        let mut g = simple_u(0);
        WattsStrogatzGraphGenerator::new(6, 4, 0.5, true, rng(SEED))
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!(g.vertex_count(), 6);
        let mut g = simple_d(0);
        ws(6, 4, 0.0, SEED)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (6, 12));
        let mut g: UGraph<String> =
            BaseGraph::simple().with_vertex_factory(|i| (i + 1).to_string());
        ws(10, 2, 0.1, SEED)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (10, 10));
    }

    fn regular(n: usize, d: usize) -> Result<RandomRegularGraphGenerator<StdRng>> {
        RandomRegularGraphGenerator::new(n, d, rng(SEED))
    }

    #[test]
    fn random_regular_bad_parameters() {
        assert!(regular(10, 15).is_err());
        assert!(regular(5, 3).is_err());
        let mut g: DiGraph<i32> = graph(GraphType::simple(), 0);
        assert!(regular(10, 2).unwrap().generate_default(&mut g).is_err());
    }

    fn all_degrees<G: Graph<Vertex = i32> + IncidenceGraph>(g: &G, d: usize) {
        assert!(g.vertices().all(|v| g.degree_of(v) == d));
    }

    #[test]
    fn random_regular_graphs() {
        let mut g = pseudo_u(0);
        regular(100, 20).unwrap().generate_default(&mut g).unwrap();
        all_degrees(&g, 20);
        let mut g = pseudo_u(0);
        regular(10, 10).unwrap().generate_default(&mut g).unwrap();
        all_degrees(&g, 10);
        let mut g = simple_u(0);
        regular(100, 20).unwrap().generate_default(&mut g).unwrap();
        all_degrees(&g, 20);
        let mut g = simple_u(0);
        regular(10, 9).unwrap().generate_default(&mut g).unwrap();
        all_degrees(&g, 9);
        let mut g = simple_u(0);
        regular(0, 0).unwrap().generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
        let mut g = simple_u(0);
        regular(10, 0).unwrap().generate_default(&mut g).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (10, 0));
    }
}

// ---------------------------------------------------------------- planted partition

mod planted_partition_tests {
    use super::*;

    fn pp(
        l: usize,
        k: usize,
        p: f64,
        q: f64,
        loops: bool,
    ) -> Result<PlantedPartitionGraphGenerator<StdRng>> {
        PlantedPartitionGraphGenerator::new(l, k, p, q, rng(SEED), loops)
    }

    #[test]
    fn bad_parameters() {
        assert!(pp(5, 10, -0.5, 0.1, false).is_err());
        assert!(pp(5, 10, 0.5, -0.1, false).is_err());
        assert!(pp(5, 10, 1.5, 0.1, false).is_err());
        assert!(pp(5, 10, 0.5, 1.1, false).is_err());
        let mut g = simple_u(0);
        assert!(
            pp(5, 10, 0.5, 0.1, true)
                .unwrap()
                .generate_default(&mut g)
                .is_err()
        );
    }

    #[test]
    fn empty_graphs() {
        for (l, k) in [(0, 10), (5, 0)] {
            let mut g = simple_u(0);
            pp(l, k, 0.5, 0.1, false)
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!((g.vertex_count(), g.edge_count()), (0, 0));
        }
    }

    #[test]
    fn simple_graph_bounds() {
        let (l, k) = (5usize, 10usize);
        for (p, q, upper, lower) in [
            (0.0, 0.1, Some(k * k * l * (l - 1) / 2), None),
            (0.5, 0.0, Some(l * k * (k - 1) / 2), None),
            (1.0, 0.1, None, Some(l * k * (k - 1) / 2)),
            (0.5, 1.0, None, Some(k * k * l * (l - 1) / 2)),
        ] {
            let mut g = simple_u(0);
            pp(l, k, p, q, false)
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!(g.vertex_count(), l * k);
            if let Some(u) = upper {
                assert!(g.edge_count() <= u);
            }
            if let Some(lo) = lower {
                assert!(g.edge_count() >= lo);
            }
        }
    }

    #[test]
    fn directed_graph_bounds() {
        let (l, k) = (5usize, 10usize);
        for (p, q, upper, lower) in [
            (0.0, 0.1, Some(k * k * l * (l - 1)), None),
            (0.5, 0.0, Some(l * k * (k - 1)), None),
            (1.0, 0.1, None, Some(l * k * (k - 1))),
            (0.5, 1.0, None, Some(k * k * l * (l - 1))),
        ] {
            let mut g = loops_d(0);
            pp(l, k, p, q, false)
                .unwrap()
                .generate_default(&mut g)
                .unwrap();
            assert_eq!(g.vertex_count(), l * k);
            if let Some(u) = upper {
                assert!(g.edge_count() <= u);
            }
            if let Some(lo) = lower {
                assert!(g.edge_count() >= lo);
            }
        }
    }

    #[test]
    fn complete_graphs() {
        let (l, k) = (5, 10);
        let mut g = simple_u(0);
        pp(l, k, 1.0, 1.0, false)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert!(g.vertices().all(|v| g.degree_of(v) == l * k - 1));
        let mut g = loops_d(0);
        pp(l, k, 1.0, 1.0, false)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        assert!(g.vertices().all(|v| g.degree_of(v) == 2 * (l * k - 1)));
    }

    #[test]
    fn get_communities() {
        let (l, k) = (5, 10);
        let mut g = simple_u(0);
        let communities = pp(l, k, 0.5, 0.1, false)
            .unwrap()
            .generate_communities(&mut g)
            .unwrap();
        let truth: Vec<Vec<i32>> = (0..l)
            .map(|i| (0..k).map(|j| (i * k + j) as i32).collect())
            .collect();
        assert_eq!(communities, truth);
    }

    #[test]
    fn generate_only_once() {
        let mut gen_ = pp(5, 10, 0.5, 0.1, false).unwrap();
        gen_.generate_default(&mut simple_u(0)).unwrap();
        assert!(gen_.generate_default(&mut simple_u(0)).is_err());
    }
}

// ---------------------------------------------------------------- Complement

mod complement_tests {
    use super::*;

    fn weighted_simple() -> UGraph<i32> {
        UGraph::<i32>::simple().weighted()
    }

    #[test]
    fn test_empty_graph() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices([0, 1, 2, 3]);
        let mut target = weighted_simple();
        Complement::new(&g, false)
            .unwrap()
            .generate_into(&mut target)
            .unwrap();
        assert!(is_complete(&target));
        let mut target2 = weighted_simple();
        Complement::new(&target, false)
            .unwrap()
            .generate_into(&mut target2)
            .unwrap();
        assert_eq!(target2.edge_count(), 0);
        assert_eq!(
            target2.vertices().collect::<HashSet<_>>(),
            g.vertices().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_undirected_graph() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices([0, 1, 2, 3]);
        for (a, b) in [(0, 1), (1, 2), (0, 2)] {
            g.add_edge(&a, &b).unwrap();
        }
        let mut target = weighted_simple();
        Complement::new(&g, false)
            .unwrap()
            .generate_into(&mut target)
            .unwrap();
        assert_eq!(
            target.vertices_cloned().collect::<HashSet<_>>(),
            HashSet::from([0, 1, 2, 3])
        );
        has_edges(&target, &[(0, 3), (2, 3), (1, 3)]);
    }

    #[test]
    fn test_directed_graph() {
        let mut g = DiGraph::<i32>::simple();
        g.add_vertices([0, 1, 2]);
        for (a, b) in [(0, 1), (1, 2), (0, 2)] {
            g.add_edge(&a, &b).unwrap();
        }
        // JGraphT builds the complement of a directed graph into an undirected target here, so only
        // the first orientation of each missing pair survives; ours is directed to keep all three.
        let mut target = DiGraph::<i32>::simple().weighted();
        Complement::new(&g, false)
            .unwrap()
            .generate_into(&mut target)
            .unwrap();
        has_edges(&target, &[(1, 0), (2, 1), (2, 0)]);
    }

    #[test]
    fn test_graph_with_self_loops() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices([0, 1, 2]);
        for (a, b) in [(0, 1), (1, 2), (0, 2)] {
            g.add_edge(&a, &b).unwrap();
        }
        let mut target = UGraph::<i32>::pseudo();
        Complement::new(&g, true)
            .unwrap()
            .generate_into(&mut target)
            .unwrap();
        assert_eq!(target.edge_count(), 3);
        assert!(
            target
                .vertices()
                .all(|v| target.contains_edge_between(v, v))
        );
    }
}

// ---------------------------------------------------------------- correctness properties over many seeds

mod generator_properties {
    use super::*;

    /// A simple graph rejects loops and parallel edges, so success already proves simplicity;
    /// the checks below pin down the exact counts and degrees.
    #[test]
    fn random_regular_is_simple_regular_for_many_seeds() {
        for (n, d) in [(6, 4), (10, 8), (12, 3), (20, 7), (30, 10), (9, 2)] {
            for seed in 0..40 {
                let mut g = simple_u(0);
                RandomRegularGraphGenerator::new(n, d, rng(seed))
                    .unwrap()
                    .generate_default(&mut g)
                    .unwrap();
                assert_eq!(g.vertex_count(), n);
                assert_eq!(g.edge_count(), n * d / 2, "n={n} d={d} seed={seed}");
                assert!(g.vertices().all(|v| g.degree_of(v) == d));
            }
        }
    }

    #[test]
    fn barabasi_albert_has_exact_edge_count_and_min_degree() {
        for (m0, m, n) in [(1, 1, 30), (3, 2, 40), (4, 4, 50), (5, 3, 60)] {
            for seed in 0..25 {
                let mut g = simple_u(0);
                BarabasiAlbertGraphGenerator::new(m0, m, n, rng(seed))
                    .unwrap()
                    .generate_default(&mut g)
                    .unwrap();
                assert_eq!(g.vertex_count(), n);
                assert_eq!(g.edge_count(), m0 * (m0 - 1) / 2 + m * (n - m0));
                assert!(g.vertices().all(|v| g.degree_of(v) >= m.min(m0 - 1).max(1)));
            }
        }
    }

    #[test]
    fn directed_scale_free_hits_targets_for_many_seeds() {
        for seed in 0..25 {
            let mut gen_ =
                DirectedScaleFreeGraphGenerator::new(0.33, 0.33, 0.5, 0.5, 500, -1, rng(seed))
                    .unwrap();
            let mut g = pseudo_d(0);
            gen_.generate_default(&mut g).unwrap();
            assert_eq!(g.edge_count(), 500);
            let mut gen_ =
                DirectedScaleFreeGraphGenerator::new(0.33, 0.33, 0.5, 0.5, -1, 300, rng(seed))
                    .unwrap();
            let mut g = pseudo_d(0);
            gen_.generate_default(&mut g).unwrap();
            assert_eq!(g.vertex_count(), 300);
        }
    }
}
