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

//! Ports of the `org.jgrapht.alg.clique` tests: the Bron–Kerbosch finders (base test run for the
//! three variants, plus the all-variants comparison), ChordalGraphMaxCliqueFinderTest and
//! CliqueMinimalSeparatorDecompositionTest.

use crate::common::construct_graph;
use jgrapht::alg::clique::*;
use jgrapht::alg::connectivity::connected_components;
use jgrapht::alg::interfaces::MaximalCliqueEnumerationAlgorithm;
use jgrapht::generate::*;
use jgrapht::util::HashableSet;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::time::Duration;

fn s(x: &str) -> String {
    x.to_string()
}

fn set(items: &[&str]) -> HashSet<String> {
    items.iter().map(|x| s(x)).collect()
}

fn create_graph() -> UGraph<String> {
    let mut g = UGraph::<String>::simple();
    g.add_vertices((1..=10).map(|i| format!("v{i}")));
    for (a, b) in [
        (1, 2),
        (1, 3),
        (1, 4),
        (2, 3),
        (2, 4),
        (3, 4),
        (5, 6),
        (5, 7),
        (6, 7),
        (3, 5),
        (4, 5),
        (7, 8),
        (1, 9),
        (1, 10),
        (2, 9),
        (2, 10),
        (9, 10),
    ] {
        g.add_edge(&format!("v{a}"), &format!("v{b}")).unwrap();
    }
    g
}

fn as_set(cliques: &[HashSet<String>]) -> HashSet<HashableSet<String>> {
    cliques.iter().cloned().map(HashableSet).collect()
}

macro_rules! bron_kerbosch_tests {
    ($module:ident, $finder:ident) => {
        mod $module {
            use super::*;

            #[test]
            fn test_find_biggest() {
                let g = create_graph();
                let mut finder = $finder::new(&g);
                let biggest: HashSet<HashableSet<String>> = finder
                    .maximum_cliques()
                    .unwrap()
                    .into_iter()
                    .cloned()
                    .map(HashableSet)
                    .collect();
                let expected: HashSet<HashableSet<String>> = [
                    set(&["v1", "v2", "v3", "v4"]),
                    set(&["v1", "v2", "v9", "v10"]),
                ]
                .into_iter()
                .map(HashableSet)
                .collect();
                assert_eq!(biggest, expected);
            }

            #[test]
            fn test_find_all() {
                let g = create_graph();
                let mut finder = $finder::new(&g);
                let cliques = finder.cliques().unwrap();
                assert_eq!(cliques.len(), 5);
                let expected: HashSet<HashableSet<String>> = [
                    set(&["v1", "v2", "v3", "v4"]),
                    set(&["v5", "v6", "v7"]),
                    set(&["v3", "v4", "v5"]),
                    set(&["v7", "v8"]),
                    set(&["v1", "v2", "v9", "v10"]),
                ]
                .into_iter()
                .map(HashableSet)
                .collect();
                assert_eq!(as_set(cliques), expected);
            }

            #[test]
            fn test_non_simple() {
                let mut g = UGraph::<String>::pseudo();
                g.add_vertices([s("1"), s("2")]);
                g.add_edge(&s("1"), &s("2")).unwrap();
                g.add_edge(&s("1"), &s("2")).unwrap();
                assert!($finder::new(&g).cliques().is_err());
            }

            #[test]
            fn test_complete() {
                let size = 6;
                let mut g = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
                Complete(size).generate_default(&mut g).unwrap();
                let mut finder = $finder::new(&g);
                let cliques = finder.cliques().unwrap();
                assert_eq!(cliques.len(), 1);
                assert_eq!(cliques[0].len(), size);
            }

            #[test]
            fn finishes_within_a_timeout() {
                let mut g = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
                Complete(6).generate_default(&mut g).unwrap();
                let mut finder = $finder::with_timeout(&g, Duration::from_secs(60)).unwrap();
                assert_eq!(finder.cliques().unwrap().len(), 1);
                assert!(!finder.is_time_limit_reached());
            }
        }
    };
}

bron_kerbosch_tests!(bron_kerbosch_clique_finder_test, BronKerboschCliqueFinder);
bron_kerbosch_tests!(
    pivot_bron_kerbosch_clique_finder_test,
    PivotBronKerboschCliqueFinder
);
bron_kerbosch_tests!(
    degeneracy_bron_kerbosch_clique_finder_test,
    DegeneracyBronKerboschCliqueFinder
);

#[test]
fn all_variants_random_instances() {
    let mut rng = StdRng::seed_from_u64((33) as u64);
    for _ in 0..10 {
        let mut g = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
        GnpRandomGraphGenerator::new(
            30,
            0.5,
            StdRng::seed_from_u64((rng.random::<i64>()) as u64),
            false,
        )
        .unwrap()
        .generate_default(&mut g)
        .unwrap();
        let collect = |cliques: &[HashSet<i32>]| -> HashSet<HashableSet<i32>> {
            cliques.iter().cloned().map(HashableSet).collect()
        };
        let c1 = collect(BronKerboschCliqueFinder::new(&g).cliques().unwrap());
        let c2 = collect(PivotBronKerboschCliqueFinder::new(&g).cliques().unwrap());
        let c3 = collect(
            DegeneracyBronKerboschCliqueFinder::new(&g)
                .cliques()
                .unwrap(),
        );
        assert_eq!(c1.len(), c2.len());
        assert_eq!(c2.len(), c3.len());
        assert_eq!(c1, c2);
        assert_eq!(c2, c3);
    }
}

mod chordal_graph_max_clique_finder_test {
    use super::*;

    fn assert_is_clique<G: Graph>(g: &G, set: &HashSet<G::Vertex>) {
        for a in set {
            for b in set {
                assert!(a == b || g.contains_edge_between(a, b));
            }
        }
    }

    #[test]
    fn test_get_maximum_clique_1() {
        let g = UGraph::<i32>::simple();
        let clique = ChordalGraphMaxCliqueFinder::new(&g).clique().unwrap();
        assert_eq!(clique.len(), 0);
    }

    #[test]
    fn test_get_maximum_clique_2() {
        let mut g = UGraph::<i32>::simple().weighted();
        construct_graph(
            &mut g,
            &[
                &[1, 2],
                &[3, 4],
                &[3, 5],
                &[3, 6],
                &[4, 5],
                &[4, 6],
                &[5, 6],
            ],
        );
        let clique = ChordalGraphMaxCliqueFinder::new(&g).clique().unwrap();
        assert_eq!(clique.len(), 4);
        assert_is_clique(&g, &clique);
    }

    #[test]
    fn test_get_maximum_clique_3() {
        let mut g = UGraph::<i32>::simple().weighted();
        construct_graph(&mut g, &[&[1, 2], &[2, 3], &[3, 4], &[4, 1]]);
        assert!(ChordalGraphMaxCliqueFinder::new(&g).clique().is_none());
    }

    #[test]
    fn test_get_maximum_clique_4() {
        let mut g = UGraph::<i32>::pseudo().weighted();
        construct_graph(
            &mut g,
            &[
                &[1, 1],
                &[1, 1],
                &[1, 2],
                &[1, 2],
                &[1, 2],
                &[2, 2],
                &[2, 2],
            ],
        );
        let clique = ChordalGraphMaxCliqueFinder::new(&g).clique().unwrap();
        assert_eq!(clique.len(), 2);
        assert_is_clique(&g, &clique);
    }
}

mod clique_minimal_separator_decomposition_test {
    use super::*;

    fn int_graph(simple: bool, vertices: &[i32], edges: &[(i32, i32)]) -> UGraph<i32> {
        let mut g = if simple {
            UGraph::<i32>::simple()
        } else {
            UGraph::<i32>::pseudo()
        };
        g.add_vertices(vertices.iter().copied());
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    fn str_graph(names: &str, edges: &[(&str, &str)]) -> UGraph<String> {
        let mut g = UGraph::<String>::simple();
        g.add_vertices(names.chars().map(|c| c.to_string()));
        for (a, b) in edges {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        g
    }

    fn sets<T: Eq + std::hash::Hash + Clone>(items: &[&[T]]) -> HashSet<HashableSet<T>> {
        items
            .iter()
            .map(|i| HashableSet(i.iter().cloned().collect()))
            .collect()
    }

    fn atoms_of<T: Eq + std::hash::Hash + Clone, G: IncidenceGraph<Vertex = T>>(
        c: &mut CliqueMinimalSeparatorDecomposition<G>,
    ) -> HashSet<HashableSet<T>> {
        c.atoms().iter().cloned().collect()
    }

    fn separators_of<T: Eq + std::hash::Hash + Clone, G: IncidenceGraph<Vertex = T>>(
        c: &mut CliqueMinimalSeparatorDecomposition<G>,
    ) -> HashSet<HashableSet<T>> {
        c.separators().iter().cloned().collect()
    }

    #[test]
    fn test_simple_graph_1() {
        let g = int_graph(
            true,
            &[1, 2, 3, 4],
            &[(1, 2), (2, 3), (3, 4), (1, 3), (2, 4)],
        );
        assert_eq!((g.vertex_count(), g.edge_count()), (4, 5));
        let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
        assert_eq!(
            (
                cmsd.minimal_triangulation().vertex_count(),
                cmsd.minimal_triangulation().edge_count()
            ),
            (4, 5)
        );
        assert_eq!(cmsd.atoms().len(), 2);
        assert_eq!(atoms_of(&mut cmsd), sets(&[&[1, 2, 3], &[2, 3, 4]]));
        assert_eq!(cmsd.separators().len(), 1);
        assert_eq!(separators_of(&mut cmsd), sets(&[&[2, 3]]));
    }

    #[test]
    fn test_pseudograph_1() {
        let g = int_graph(
            false,
            &[1, 2, 3, 4],
            &[
                (1, 1),
                (1, 2),
                (2, 2),
                (2, 3),
                (2, 3),
                (3, 4),
                (1, 3),
                (2, 4),
                (2, 4),
                (2, 4),
            ],
        );
        assert_eq!((g.vertex_count(), g.edge_count()), (4, 10));
        let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
        assert_eq!(
            (
                cmsd.minimal_triangulation().vertex_count(),
                cmsd.minimal_triangulation().edge_count()
            ),
            (4, 5)
        );
        assert_eq!(atoms_of(&mut cmsd), sets(&[&[1, 2, 3], &[2, 3, 4]]));
        assert_eq!(cmsd.atoms().len(), 2);
        assert_eq!(separators_of(&mut cmsd), sets(&[&[2, 3]]));
        assert_eq!(cmsd.separators().len(), 1);
    }

    #[test]
    fn test_simple_graph_2() {
        let g = int_graph(true, &[1, 2, 3, 4], &[(1, 2), (2, 3), (3, 4), (4, 1)]);
        let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
        assert_eq!(
            (
                cmsd.minimal_triangulation().vertex_count(),
                cmsd.minimal_triangulation().edge_count()
            ),
            (4, 5)
        );
        assert_eq!(cmsd.atoms().len(), 1);
        assert_eq!(atoms_of(&mut cmsd), sets(&[&[1, 2, 3, 4]]));
        assert_eq!(cmsd.separators().len(), 0);
    }

    #[test]
    fn test_berry_2010() {
        let g = str_graph(
            "abcdefghijk",
            &[
                ("a", "b"),
                ("b", "c"),
                ("c", "d"),
                ("d", "e"),
                ("e", "g"),
                ("f", "g"),
                ("d", "f"),
                ("a", "k"),
                ("k", "j"),
                ("c", "k"),
                ("d", "j"),
                ("c", "j"),
                ("d", "k"),
                ("f", "k"),
                ("f", "j"),
                ("g", "j"),
                ("g", "k"),
                ("h", "j"),
                ("h", "i"),
                ("i", "k"),
            ],
        );
        assert_eq!((g.vertex_count(), g.edge_count()), (11, 20));
        let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
        assert_eq!(
            (
                cmsd.minimal_triangulation().vertex_count(),
                cmsd.minimal_triangulation().edge_count()
            ),
            (11, 20 + 3)
        );
        let strs = |items: &[&[&str]]| -> HashSet<HashableSet<String>> {
            items.iter().map(|i| HashableSet(set(i))).collect()
        };
        assert_eq!(cmsd.atoms().len(), 4);
        assert_eq!(
            atoms_of(&mut cmsd),
            strs(&[
                &["a", "b", "c", "k"],
                &["c", "d", "j", "k"],
                &["h", "i", "j", "k"],
                &["d", "e", "f", "g", "j", "k"]
            ])
        );
        assert_eq!(cmsd.separators().len(), 3);
        assert_eq!(
            separators_of(&mut cmsd),
            strs(&[&["c", "k"], &["j", "k"], &["d", "j", "k"]])
        );
    }

    #[test]
    fn test_tarjan_1985() {
        let g = str_graph(
            "abcdefghijk",
            &[
                ("a", "c"),
                ("a", "d"),
                ("a", "f"),
                ("b", "c"),
                ("b", "g"),
                ("c", "d"),
                ("c", "f"),
                ("c", "h"),
                ("d", "e"),
                ("d", "f"),
                ("d", "i"),
                ("e", "j"),
                ("f", "k"),
                ("g", "h"),
                ("h", "k"),
                ("i", "j"),
                ("i", "k"),
            ],
        );
        assert_eq!((g.vertex_count(), g.edge_count()), (11, 17));
        let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
        // (JGraphT disables the edge count check of the triangulation here)
        assert_eq!(cmsd.minimal_triangulation().vertex_count(), 11);
        let strs = |items: &[&[&str]]| -> HashSet<HashableSet<String>> {
            items.iter().map(|i| HashableSet(set(i))).collect()
        };
        assert_eq!(cmsd.atoms().len(), 4);
        assert_eq!(
            atoms_of(&mut cmsd),
            strs(&[
                &["a", "c", "d", "f"],
                &["b", "c", "g", "h"],
                &["d", "e", "i", "j"],
                &["c", "d", "f", "h", "i", "k"]
            ])
        );
        assert_eq!(cmsd.separators().len(), 3);
        assert_eq!(
            separators_of(&mut cmsd),
            strs(&[&["c", "d", "f"], &["c", "h"], &["d", "i"]])
        );
    }

    #[test]
    fn test_graph_1() {
        let g = str_graph(
            "abcdefghijklmn",
            &[
                ("a", "b"),
                ("a", "d"),
                ("b", "e"),
                ("c", "e"),
                ("d", "e"),
                ("e", "f"),
                ("d", "g"),
                ("d", "h"),
                ("f", "k"),
                ("g", "i"),
                ("h", "i"),
                ("d", "i"),
                ("e", "j"),
                ("i", "j"),
                ("j", "k"),
                ("i", "l"),
                ("i", "m"),
                ("i", "n"),
                ("j", "m"),
                ("j", "n"),
            ],
        );
        assert_eq!((g.vertex_count(), g.edge_count()), (14, 20));
        let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
        assert_eq!(
            (
                cmsd.minimal_triangulation().vertex_count(),
                cmsd.minimal_triangulation().edge_count()
            ),
            (14, 20 + 3)
        );
        let strs = |items: &[&[&str]]| -> HashSet<HashableSet<String>> {
            items.iter().map(|i| HashableSet(set(i))).collect()
        };
        assert_eq!(cmsd.atoms().len(), 9);
        assert_eq!(
            atoms_of(&mut cmsd),
            strs(&[
                &["a", "b", "d", "e"],
                &["c", "e"],
                &["d", "g", "i"],
                &["d", "h", "i"],
                &["d", "e", "i", "j"],
                &["e", "f", "j", "k"],
                &["i", "l"],
                &["i", "j", "m"],
                &["i", "j", "n"],
            ])
        );
        assert_eq!(cmsd.separators().len(), 6);
        assert_eq!(
            separators_of(&mut cmsd),
            strs(&[
                &["d", "e"],
                &["e"],
                &["d", "i"],
                &["i"],
                &["e", "j"],
                &["i", "j"]
            ])
        );
        let counts = cmsd.full_component_count().clone();
        assert_eq!(counts.len(), 6);
        for (sep, want) in [
            (&["d", "e"][..], 2),
            (&["e"][..], 2),
            (&["d", "i"][..], 3),
            (&["i"][..], 2),
            (&["e", "j"][..], 2),
            (&["i", "j"][..], 3),
        ] {
            assert_eq!(counts[&HashableSet(set(sep))], want, "{sep:?}");
        }
    }

    #[test]
    fn test_random_graphs() {
        for rounds in (0..42).rev() {
            let n = 80 + rounds;
            let m = (1.1 * (n as f64).ln() / n as f64 * (n * (n - 1) / 2) as f64).ceil() as usize;
            let g = loop {
                let mut g = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32 + 1);
                GnmRandomGraphGenerator::new(n as usize, m, StdRng::from_os_rng(), false, false)
                    .unwrap()
                    .generate_default(&mut g)
                    .unwrap();
                if connected_components(&g).len() == 1 {
                    break g;
                }
            };
            let mut cmsd = CliqueMinimalSeparatorDecomposition::new(&g).unwrap();
            let fill = cmsd.fill_edges().len();
            assert_eq!(
                cmsd.minimal_triangulation().edge_count(),
                g.edge_count() + fill
            );
            let separators: Vec<_> = cmsd.separators().to_vec();
            for sep in &separators {
                assert!(!sep.is_empty());
                assert!(
                    sep.iter()
                        .all(|a| sep.iter().all(|b| a == b || g.contains_edge_between(a, b)))
                );
            }
            assert!(separators.len() < cmsd.atoms().len());
            assert_eq!(separators.len(), cmsd.full_component_count().len());
            for sep in &separators {
                assert!(cmsd.full_component_count()[sep] >= 2);
            }
        }
    }
}
