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

//! Ports of the `org.jgrapht.alg.connectivity` tests: ConnectivityInspectorTest,
//! StrongConnectivityAlgorithmTest (Gabow and Kosaraju), BiconnectivityInspectorTest,
//! BlockCutpointGraphTest, KConnectivityFlowAlgorithmTest and TreeDynamicConnectivityTest.

use jgrapht::alg::connectivity::*;
use jgrapht::alg::interfaces::StrongConnectivityAlgorithm;
use jgrapht::event::Listenable;
use jgrapht::generate::*;
use jgrapht::graph::props;
use jgrapht::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

fn s(x: &str) -> String {
    x.to_string()
}

fn set<T: Eq + Hash + Clone>(items: &[T]) -> HashSet<T> {
    items.iter().cloned().collect()
}

fn sets<T: Eq + Hash + Clone>(items: &[&[T]]) -> HashSet<std::collections::BTreeSet<T>>
where
    T: Ord,
{
    items.iter().map(|i| i.iter().cloned().collect()).collect()
}

// ---------------------------------------------------------------------------------------------
// ConnectivityInspectorTest
// ---------------------------------------------------------------------------------------------

mod connectivity_inspector {
    use super::*;

    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";
    const V4: &str = "v4";

    #[test]
    fn test_directed_graph() {
        let mut g = Listenable::new(DiGraph::<String>::simple());
        g.add_vertex(s(V1));
        g.add_vertex(s(V2));
        g.add_vertex(s(V3));
        g.add_edge(&s(V1), &s(V2)).unwrap();

        let inspector = ConnectivityInspector::shared();
        g.add_listener(inspector.clone());
        assert!(!inspector.borrow_mut().is_connected(&g));
        g.add_edge(&s(V1), &s(V3)).unwrap();
        assert!(inspector.borrow_mut().is_connected(&g));
    }

    /// Runs `$body` with `$g` bound to a fresh listenable graph of either direction.
    macro_rules! both {
        ($name:ident, |$g:ident, $insp:ident| $body:block) => {
            #[test]
            fn $name() {
                for directed in [false, true] {
                    if directed {
                        let mut $g = Listenable::new(DiGraph::<String>::simple());
                        let $insp = ConnectivityInspector::shared();
                        $body
                    } else {
                        let mut $g = Listenable::new(UGraph::<String>::simple());
                        let $insp = ConnectivityInspector::shared();
                        $body
                    }
                }
            }
        };
    }
    both!(test_listener_vertex_add_before_query, |g, inspector| {
        g.add_listener(inspector.clone());
        g.add_vertex(s(V1));
        g.add_vertex(s(V2));
        let found: HashSet<_> = inspector
            .borrow_mut()
            .connected_sets(&g)
            .into_iter()
            .map(|c| c.into_iter().collect::<BTreeSet<_>>())
            .collect();
        assert_eq!(found, sets(&[&[s(V1)], &[s(V2)]]));
        assert_eq!(inspector.borrow_mut().connected_sets(&g).len(), 2);
    });

    both!(test_listener_vertex_add_after_removal, |g, inspector| {
        g.add_vertex(s(V1));
        g.add_listener(inspector.clone());
        assert!(inspector.borrow_mut().is_connected(&g));
        g.remove_vertex(&s(V1));
        g.add_vertex(s(V2));
        assert_eq!(
            inspector.borrow_mut().connected_sets(&g),
            vec![set(&[s(V2)])]
        );
        g.add_vertex(s(V3));
        let found: HashSet<_> = inspector
            .borrow_mut()
            .connected_sets(&g)
            .into_iter()
            .map(|c| c.into_iter().collect::<BTreeSet<_>>())
            .collect();
        assert_eq!(found, sets(&[&[s(V2)], &[s(V3)]]));
    });

    both!(test_listener_edge_add_before_query, |g, inspector| {
        g.add_vertex(s(V1));
        g.add_vertex(s(V2));
        g.add_listener(inspector.clone());
        g.add_edge(&s(V1), &s(V2)).unwrap();
        assert_eq!(
            inspector.borrow_mut().connected_sets(&g),
            vec![set(&[s(V1), s(V2)])]
        );
    });

    both!(
        test_listener_edge_add_after_removal_and_partial_query,
        |g, inspector| {
            g.add_vertex(s(V1));
            g.add_vertex(s(V2));
            g.add_vertex(s(V3));
            g.add_edge(&s(V1), &s(V2)).unwrap();
            g.add_edge(&s(V2), &s(V3)).unwrap();
            g.add_listener(inspector.clone());
            assert!(inspector.borrow_mut().is_connected(&g));

            g.remove_edge_between(&s(V1), &s(V2));
            assert_eq!(
                inspector.borrow_mut().connected_set_of(&g, &s(V1)).unwrap(),
                set(&[s(V1)])
            );
            assert_eq!(
                inspector.borrow_mut().connected_set_of(&g, &s(V3)).unwrap(),
                set(&[s(V2), s(V3)])
            );
            g.add_edge(&s(V1), &s(V2)).unwrap();
            let all = set(&[s(V1), s(V2), s(V3)]);
            assert_eq!(
                inspector.borrow_mut().connected_set_of(&g, &s(V1)).unwrap(),
                all
            );
            assert_eq!(
                inspector.borrow_mut().connected_set_of(&g, &s(V3)).unwrap(),
                all
            );
            assert_eq!(inspector.borrow_mut().connected_sets(&g), vec![all]);
        }
    );

    #[test]
    fn test_is_graph_connected() {
        let mut g = UGraph::<String>::pseudo();
        assert_eq!(g.vertex_count(), 0);
        for v in [V1, V2, V3, V4] {
            g.add_vertex(s(v));
        }
        assert_eq!(g.edge_count(), 0);
        let _e1 = g.add_edge(&s(V1), &s(V2)).unwrap().unwrap();
        let e2 = g.add_edge(&s(V2), &s(V3)).unwrap().unwrap();
        let _e3 = g.add_edge(&s(V3), &s(V1)).unwrap().unwrap();
        let _e3b = g.add_edge(&s(V3), &s(V1)).unwrap().unwrap();
        g.add_edge(&s(V1), &s(V1)).unwrap();
        g.add_edge(&s(V1), &s(V1)).unwrap();
        assert_eq!(g.edge_count(), 6);

        assert!(!ConnectivityInspector::new().is_connected(&g));

        g.remove_vertex(&s(V4));
        assert!(ConnectivityInspector::new().is_connected(&g));

        g.remove_vertex(&s(V1));
        assert_eq!(g.edge_count(), 1);

        g.remove_edge(&e2);
        g.add_edge(&s(V2), &s(V2)).unwrap();
        assert_eq!(g.edge_count(), 1);

        assert!(!ConnectivityInspector::new().is_connected(&g));
    }
}

// ---------------------------------------------------------------------------------------------
// StrongConnectivityAlgorithmTest
// ---------------------------------------------------------------------------------------------

/// Rotates the vertex labels of a directed graph by `shift` positions (the vertices are added in
/// the same order every time, so the algorithms start from a different logical vertex).
fn rotated<V: Clone + Eq + Hash>(g: &DiGraph<V>, shift: usize) -> DiGraph<V> {
    let vs: Vec<V> = g.vertices_cloned().collect();
    let shifted = |v: &V| {
        let i = vs.iter().position(|x| x == v).unwrap();
        vs[(i + shift) % vs.len()].clone()
    };
    let mut out = DiGraph::<V>::simple();
    out.add_vertices(vs.iter().cloned());
    for e in g.edges() {
        let (a, b) = g.edge_endpoints(e).unwrap();
        out.add_edge(&shifted(a), &shifted(b)).unwrap();
    }
    out
}

fn directed_with_vertices(n: usize) -> DiGraph<String> {
    let mut g = DiGraph::<String>::simple();
    g.add_vertices((1..=n).map(|i| format!("v{i}")));
    g
}

fn edge(g: &mut DiGraph<String>, a: usize, b: usize) {
    g.add_edge(&format!("v{a}"), &format!("v{b}")).unwrap();
}

macro_rules! strong_connectivity_tests {
    ($module:ident, $algo:ident) => {
        mod $module {
            use super::*;

            fn assert_strongly_connected_sets<V>(graph: &DiGraph<V>, expected: &[&[V]])
            where
                V: Clone + Eq + Hash + Ord + std::fmt::Debug,
            {
                let vs: Vec<V> = graph.vertices_cloned().collect();
                for shift in 0..vs.len() {
                    let g = rotated(graph, shift);
                    let rotate = |v: &V| {
                        let i = vs.iter().position(|x| x == v).unwrap();
                        vs[(i + shift) % vs.len()].clone()
                    };
                    let want: HashSet<BTreeSet<V>> = expected
                        .iter()
                        .map(|set| set.iter().map(&rotate).collect())
                        .collect();
                    let inspector = $algo::new(&g);
                    let found = inspector.strongly_connected_sets();
                    assert_eq!(found.len(), want.len());
                    let got: HashSet<BTreeSet<V>> =
                        found.iter().map(|c| c.iter().cloned().collect()).collect();
                    assert_eq!(got, want);

                    let mut actual = HashSet::new();
                    for sg in inspector.strongly_connected_components() {
                        actual.insert(sg.vertices_cloned().collect::<BTreeSet<V>>());
                        assert!($algo::new(&sg).is_strongly_connected());
                    }
                    assert_eq!(actual, want);
                }
            }

            fn names(items: &[&[usize]]) -> Vec<Vec<String>> {
                items
                    .iter()
                    .map(|i| i.iter().map(|n| format!("v{n}")).collect())
                    .collect()
            }

            fn check(g: &DiGraph<String>, expected: &[&[usize]]) {
                let owned = names(expected);
                let refs: Vec<&[String]> = owned.iter().map(|v| v.as_slice()).collect();
                assert_strongly_connected_sets(g, &refs);
            }

            #[test]
            fn test_strongly_connected_1() {
                let mut g = directed_with_vertices(4);
                edge(&mut g, 1, 2);
                edge(&mut g, 2, 1);
                edge(&mut g, 3, 4);
                check(&g, &[&[1, 2], &[3], &[4]]);
            }

            #[test]
            fn test_strongly_connected_2() {
                let mut g = directed_with_vertices(4);
                edge(&mut g, 1, 2);
                edge(&mut g, 2, 1);
                edge(&mut g, 4, 3);
                edge(&mut g, 3, 2);
                check(&g, &[&[1, 2], &[3], &[4]]);
            }

            #[test]
            fn test_strongly_connected_3() {
                let mut g = directed_with_vertices(4);
                for (a, b) in [(1, 2), (2, 3), (3, 1), (1, 4), (2, 4), (3, 4)] {
                    edge(&mut g, a, b);
                }
                check(&g, &[&[1, 2, 3], &[4]]);
            }

            #[test]
            fn test_strongly_connected_4() {
                let mut g = DiGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
                Ring(3).generate_graph(&mut g, None).unwrap();
                assert_strongly_connected_sets(&g, &[&[0, 1, 2]]);
            }

            #[test]
            fn test_strongly_connected_5() {
                // example from Gabow's "Path-based depth-first search for strong and biconnected
                // components" (2000)
                let mut g = directed_with_vertices(6);
                for (a, b) in [
                    (1, 2),
                    (1, 3),
                    (2, 3),
                    (2, 4),
                    (4, 3),
                    (4, 5),
                    (5, 2),
                    (5, 6),
                    (6, 3),
                    (6, 4),
                ] {
                    edge(&mut g, a, b);
                }
                check(&g, &[&[1], &[2, 4, 5, 6], &[3]]);
            }

            #[test]
            fn test_strongly_connected_6() {
                // example from the Wikipedia page on Tarjan's algorithm
                let mut g = directed_with_vertices(8);
                for (a, b) in [
                    (1, 5),
                    (2, 1),
                    (3, 2),
                    (3, 4),
                    (4, 3),
                    (5, 2),
                    (6, 2),
                    (6, 5),
                    (6, 7),
                    (7, 3),
                    (7, 6),
                    (8, 4),
                    (8, 7),
                ] {
                    edge(&mut g, a, b);
                }
                check(&g, &[&[1, 2, 5], &[3, 4], &[6, 7], &[8]]);
            }

            #[test]
            fn test_strongly_connected_7() {
                let mut g = directed_with_vertices(5);
                for (a, b) in [(1, 2), (2, 3), (3, 4), (3, 5), (4, 1), (5, 3)] {
                    edge(&mut g, a, b);
                }
                check(&g, &[&[1, 2, 3, 4, 5]]);
            }

            #[test]
            fn test_strongly_connected_8() {
                let mut g = directed_with_vertices(11);
                for (a, b) in [
                    (1, 2),
                    (1, 4),
                    (2, 3),
                    (2, 5),
                    (3, 1),
                    (3, 7),
                    (4, 3),
                    (5, 6),
                    (5, 7),
                    (6, 7),
                    (6, 8),
                    (6, 9),
                    (6, 10),
                    (7, 5),
                    (8, 10),
                    (9, 10),
                    (10, 9),
                ] {
                    edge(&mut g, a, b);
                }
                check(&g, &[&[1, 2, 3, 4], &[5, 6, 7], &[8], &[9, 10], &[11]]);
            }

            fn component_index(inspector: &$algo<'_, DiGraph<String>>, vs: &[&str]) -> usize {
                let want: HashSet<String> = vs.iter().map(|v| s(v)).collect();
                let matches: Vec<usize> = inspector
                    .strongly_connected_sets()
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| **c == want)
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(matches.len(), 1, "not exactly one match");
                matches[0]
            }

            #[test]
            fn test_condensation() {
                let mut g = directed_with_vertices(5);
                edge(&mut g, 1, 2);
                edge(&mut g, 2, 1);
                edge(&mut g, 3, 4);
                edge(&mut g, 5, 4);
                let inspector = $algo::new(&g);
                let condensation = inspector.condensation();
                assert_eq!(condensation.vertex_count(), 4);

                let i1 = component_index(&inspector, &["v1", "v2"]);
                let i2 = component_index(&inspector, &["v3"]);
                let i3 = component_index(&inspector, &["v4"]);
                let i4 = component_index(&inspector, &["v5"]);
                let comps = inspector.strongly_connected_components();

                let inner: HashSet<DefaultEdge> = comps[i1].edges_cloned().collect();
                let want: HashSet<DefaultEdge> = [
                    *g.edge_between(&s("v1"), &s("v2")).unwrap(),
                    *g.edge_between(&s("v2"), &s("v1")).unwrap(),
                ]
                .into();
                assert_eq!(inner, want);
                assert_eq!(comps[i2].edge_count(), 0);
                assert_eq!(comps[i3].edge_count(), 0);
                assert_eq!(comps[i4].edge_count(), 0);

                assert_eq!(condensation.edge_count(), 2);
                assert!(condensation.edge_between(&i2, &i3).is_some());
                assert!(condensation.edge_between(&i4, &i3).is_some());
            }

            #[test]
            fn test_condensation_2() {
                let mut g = directed_with_vertices(4);
                for (a, b) in [(1, 2), (2, 1), (3, 4), (4, 3), (1, 3), (2, 4)] {
                    edge(&mut g, a, b);
                }
                let inspector = $algo::new(&g);
                let condensation = inspector.condensation();
                assert_eq!(condensation.vertex_count(), 2);
                let i1 = component_index(&inspector, &["v1", "v2"]);
                let i2 = component_index(&inspector, &["v3", "v4"]);
                let comps = inspector.strongly_connected_components();

                let edges_of = |c: &jgrapht::AsSubgraph<&DiGraph<String>>| -> HashSet<DefaultEdge> {
                    c.edges_cloned().collect()
                };
                assert_eq!(
                    edges_of(&comps[i1]),
                    HashSet::from([
                        *g.edge_between(&s("v1"), &s("v2")).unwrap(),
                        *g.edge_between(&s("v2"), &s("v1")).unwrap()
                    ])
                );
                assert_eq!(
                    edges_of(&comps[i2]),
                    HashSet::from([
                        *g.edge_between(&s("v3"), &s("v4")).unwrap(),
                        *g.edge_between(&s("v4"), &s("v3")).unwrap()
                    ])
                );
                assert_eq!(condensation.edge_count(), 1);
                assert!(condensation.edge_between(&i1, &i2).is_some());
            }
        }
    };
}

strong_connectivity_tests!(gabow, GabowStrongConnectivityInspector);
strong_connectivity_tests!(kosaraju, KosarajuStrongConnectivityInspector);

// ---------------------------------------------------------------------------------------------
// BiconnectivityInspectorTest
// ---------------------------------------------------------------------------------------------

mod biconnectivity_inspector {
    use super::*;

    type Blocks = BTreeSet<Vec<i32>>;

    fn normalize(blocks: Vec<&HashSet<i32>>) -> Blocks {
        blocks
            .into_iter()
            .map(|b| {
                let mut v: Vec<i32> = b.iter().copied().collect();
                v.sort();
                v
            })
            .collect()
    }

    fn expected(blocks: &[&[i32]]) -> Blocks {
        blocks.iter().map(|b| b.to_vec()).collect()
    }

    fn string_graph(edges: &[(&str, &str)]) -> UGraph<String> {
        let mut g = UGraph::<String>::simple();
        g.add_vertices(["0", "1", "2", "3", "4", "5"].map(s));
        for (a, b) in edges {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        g
    }

    #[test]
    fn test_biconnected() {
        let g = string_graph(&[
            ("0", "1"),
            ("1", "2"),
            ("2", "3"),
            ("3", "4"),
            ("4", "5"),
            ("5", "0"),
        ]);
        let inspector = BiconnectivityInspector::new(&g);
        assert!(inspector.is_biconnected());
        assert_eq!(inspector.cutpoints().len(), 0);
    }

    #[test]
    fn test_linear_graph() {
        let n = 5;
        let mut g = UGraph::<usize>::simple().with_vertex_factory(|i| i as usize);
        Linear(n).generate_graph(&mut g, None).unwrap();
        let inspector = BiconnectivityInspector::new(&g);
        assert_eq!(inspector.cutpoints().len(), n - 2);
    }

    #[test]
    fn test_not_biconnected() {
        let g = string_graph(&[
            ("0", "2"),
            ("0", "3"),
            ("3", "1"),
            ("1", "4"),
            ("4", "5"),
            ("5", "3"),
        ]);
        assert_eq!(BiconnectivityInspector::new(&g).cutpoints().len(), 2);
    }

    #[test]
    fn test_border_cases() {
        let mut g = UGraph::<i32>::simple();
        assert!(!BiconnectivityInspector::new(&g).is_biconnected()); // empty graph
        g.add_vertex(0);
        assert!(!BiconnectivityInspector::new(&g).is_biconnected()); // one vertex
        g.add_vertex(1);
        assert!(!BiconnectivityInspector::new(&g).is_biconnected()); // two vertices, no edge
        g.add_edge(&0, &1).unwrap();
        assert!(BiconnectivityInspector::new(&g).is_biconnected()); // one edge
    }

    #[test]
    fn test_connected_components_1() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(1..=5);
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap();
        g.add_edge(&4, &5).unwrap();

        let inspector = BiconnectivityInspector::new(&g);
        assert_eq!(inspector.connected_components().len(), 2);
        assert!(!inspector.is_connected());

        for v in [1, 2, 3] {
            assert_eq!(
                *inspector.connected_component_of(&v).unwrap(),
                set(&[1, 2, 3])
            );
        }
        for v in [4, 5] {
            assert_eq!(*inspector.connected_component_of(&v).unwrap(), set(&[4, 5]));
        }
    }

    const WIKI_EDGES: [(i32, i32); 16] = [
        (1, 3),
        (1, 2),
        (2, 4),
        (3, 4),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 8),
        (7, 9),
        (9, 10),
        (9, 11),
        (11, 12),
        (12, 13),
        (13, 14),
        (12, 14),
        (7, 14),
    ];

    fn check_wiki(g: &UGraph<i32>, expected_bridges: &[(i32, i32)]) {
        let inspector = BiconnectivityInspector::new(g);
        assert!(inspector.is_connected());
        let cut: HashSet<i32> = inspector.cutpoints().iter().copied().collect();
        assert_eq!(cut, set(&[4, 5, 6, 7, 9]));

        let bridges: HashSet<DefaultEdge> = inspector.bridges().iter().copied().collect();
        let want: HashSet<DefaultEdge> = expected_bridges
            .iter()
            .map(|(a, b)| *g.edge_between(a, b).unwrap())
            .collect();
        assert_eq!(bridges, want);

        let b0: &[i32] = &[1, 2, 3, 4];
        let b1: &[i32] = &[4, 5];
        let b2: &[i32] = &[5, 6];
        let b3: &[i32] = &[6, 7];
        let b4: &[i32] = &[7, 8];
        let b5: &[i32] = &[9, 10];
        let b6: &[i32] = &[7, 9, 11, 12, 13, 14];
        let blocks = |v: i32| normalize(inspector.blocks_of(&v).unwrap());
        for v in [1, 2, 3] {
            assert_eq!(blocks(v), expected(&[b0]));
        }
        assert_eq!(blocks(8), expected(&[b4]));
        for v in [11, 12, 13, 14] {
            assert_eq!(blocks(v), expected(&[b6]));
        }
        assert_eq!(blocks(4), expected(&[b0, b1]));
        assert_eq!(blocks(5), expected(&[b1, b2]));
        assert_eq!(blocks(6), expected(&[b2, b3]));
        assert_eq!(blocks(7), expected(&[b3, b4, b6]));
        assert_eq!(blocks(9), expected(&[b5, b6]));
    }

    #[test]
    fn test_wiki_graph() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(1..=14);
        for (a, b) in WIKI_EDGES {
            g.add_edge(&a, &b).unwrap();
        }
        check_wiki(&g, &[(4, 5), (5, 6), (6, 7), (7, 8), (9, 10)]);
    }

    #[test]
    fn test_multi_graph() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices(0..=2);
        let bridge = g.add_edge(&0, &1).unwrap().unwrap();
        g.add_edge(&1, &1).unwrap();
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&1, &2).unwrap();

        let inspector = BiconnectivityInspector::new(&g);
        assert_eq!(
            inspector.cutpoints().iter().copied().collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            inspector.bridges().iter().copied().collect::<Vec<_>>(),
            vec![bridge]
        );
        let blocks = normalize(inspector.blocks().iter().collect());
        assert_eq!(blocks, expected(&[&[0, 1], &[1, 2]]));
    }

    #[test]
    fn test_multi_graph_2() {
        let mut g = UGraph::<i32>::multi();
        g.add_vertices(1..=14);
        for (a, b) in WIKI_EDGES.iter().chain(WIKI_EDGES.iter()) {
            g.add_edge(a, b).unwrap();
        }
        check_wiki(&g, &[]);
    }

    #[test]
    fn test_github_issue_bug_798() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(0..=3);
        g.add_edge(&0, &1).unwrap();
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&0, &2).unwrap();
        let e03 = g.add_edge(&0, &3).unwrap().unwrap();

        let bi = BiconnectivityInspector::new(&g);
        assert!(!bi.is_biconnected());
        assert_eq!(bi.cutpoints().len(), 1);
        assert!(bi.cutpoints().contains(&0));
        assert_eq!(bi.bridges().len(), 1);
        assert!(bi.bridges().contains(&e03));
        assert_eq!(bi.blocks_of(&0).unwrap().len(), 2);
        assert_eq!(bi.blocks_of(&1).unwrap().len(), 1);
        assert_eq!(bi.blocks_of(&2).unwrap().len(), 1);
        assert_eq!(bi.blocks_of(&3).unwrap().len(), 1);
        assert_eq!(bi.blocks().len(), 2);
    }
}

// ---------------------------------------------------------------------------------------------
// BlockCutpointGraphTest
// ---------------------------------------------------------------------------------------------

mod block_cutpoint_graph {
    use super::*;

    fn validate<G: Graph<Vertex = i32> + IncidenceGraph>(graph: &G) {
        let bc = BlockCutpointGraph::new(graph);
        assert!(props::is_bipartite(bc.graph()));
        assert!(props::is_forest(bc.graph()));
        assert_eq!(
            ConnectivityInspector::new().connected_sets(graph).len(),
            ConnectivityInspector::new()
                .connected_sets(bc.graph())
                .len()
        );

        let inspector = BiconnectivityInspector::new(graph);
        let blocks = inspector.blocks();
        let cutpoints = inspector.cutpoints();
        assert_eq!(blocks.len() + cutpoints.len(), bc.graph().vertex_count());

        // every cutpoint is contained in the blocks it is attached to
        for c in cutpoints {
            let node = bc.block_of(c).unwrap();
            assert_eq!(bc.node(node).len(), 1);
            assert!(bc.node(node).contains(c));
            for n in bc.graph().neighbor_list_of(&node) {
                assert!(bc.node(n).contains(c));
            }
        }
        // every block is adjacent to exactly its cutpoints
        for b in bc.block_nodes() {
            let in_block = bc.node(b).iter().filter(|v| cutpoints.contains(*v)).count();
            assert_eq!(in_block, bc.graph().degree_of(&b));
        }
    }

    #[test]
    fn random_graph_test() {
        let mut gen_ =
            GnpRandomGraphGenerator::new(50, 0.5, StdRng::seed_from_u64(0), false).unwrap();
        for _ in 0..5 {
            let mut g = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
            gen_.generate_default(&mut g).unwrap();
            validate(&g);
        }
    }

    #[test]
    fn random_directed_graph_test() {
        let mut gen_ =
            GnpRandomGraphGenerator::new(50, 0.5, StdRng::seed_from_u64(0), false).unwrap();
        for _ in 0..5 {
            let mut g = DiGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
            gen_.generate_default(&mut g).unwrap();
            validate(&g);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// KConnectivityFlowAlgorithmTest
// ---------------------------------------------------------------------------------------------

mod k_connectivity {
    use super::*;

    fn undirected(n: i32, edges: &[(i32, i32)]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(if n == 3 { 1..=3 } else { 0..=n - 1 });
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    fn directed(n: i32, edges: &[(i32, i32)]) -> DiGraph<i32> {
        let mut g = DiGraph::<i32>::simple();
        g.add_vertices(if n == 3 { 1..=3 } else { 0..=n - 1 });
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    const TRIANGLE: [(i32, i32); 3] = [(1, 2), (2, 3), (3, 1)];
    const CUBE: [(i32, i32); 12] = [
        (0, 1),
        (0, 2),
        (0, 4),
        (1, 3),
        (1, 5),
        (2, 3),
        (2, 6),
        (3, 7),
        (4, 5),
        (4, 6),
        (5, 7),
        (6, 7),
    ];
    const DIRECTED_LARGE: [(i32, i32); 18] = [
        (0, 1),
        (0, 2),
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
        (2, 3),
        (2, 4),
        (2, 4),
        (3, 4),
        (3, 5),
        (4, 0),
        (4, 1),
        (4, 2),
        (4, 5),
        (5, 0),
        (5, 1),
        (5, 2),
    ];

    #[test]
    fn test_edge_connectivity_small_undirected_graph() {
        let g = undirected(3, &TRIANGLE);
        assert_eq!(KConnectivityFlowAlgorithm::new(&g).edge_connectivity(), 2);
    }

    #[test]
    fn test_vertex_connectivity_small_undirected_graph() {
        let g = undirected(3, &TRIANGLE);
        assert_eq!(
            KConnectivityFlowAlgorithm::new(&g)
                .vertex_connectivity()
                .unwrap(),
            2
        );
    }

    #[test]
    fn test_edge_connectivity_large_undirected_graph() {
        let g = undirected(8, &CUBE);
        assert_eq!(KConnectivityFlowAlgorithm::new(&g).edge_connectivity(), 3);
    }

    #[test]
    fn test_vertex_connectivity_large_undirected_graph() {
        let g = undirected(8, &CUBE);
        assert_eq!(
            KConnectivityFlowAlgorithm::new(&g)
                .vertex_connectivity()
                .unwrap(),
            3
        );
    }

    #[test]
    fn test_edge_connectivity_small_directed_graph() {
        let g = directed(3, &TRIANGLE);
        assert_eq!(KConnectivityFlowAlgorithm::new(&g).edge_connectivity(), 1);
    }

    #[test]
    fn test_vertex_connectivity_small_directed_graph() {
        let g = directed(3, &TRIANGLE);
        assert_eq!(
            KConnectivityFlowAlgorithm::new(&g)
                .vertex_connectivity()
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_edge_connectivity_large_directed_graph() {
        let g = directed(6, &DIRECTED_LARGE);
        assert_eq!(KConnectivityFlowAlgorithm::new(&g).edge_connectivity(), 2);
    }

    #[test]
    fn test_vertex_connectivity_large_directed_graph() {
        let g = directed(6, &DIRECTED_LARGE);
        assert_eq!(
            KConnectivityFlowAlgorithm::new(&g)
                .vertex_connectivity()
                .unwrap(),
            2
        );
    }
}

// ---------------------------------------------------------------------------------------------
// TreeDynamicConnectivityTest
// ---------------------------------------------------------------------------------------------

mod tree_dynamic_connectivity {
    use super::*;

    #[test]
    fn test_add_node_remove_node() {
        for tree_size in 1..50 {
            let mut c = TreeDynamicConnectivity::<i32>::new();
            for node in 0..tree_size {
                assert!(!c.contains(&node));
                assert!(c.add(node));
                assert!(c.contains(&node));
                assert!(!c.add(node));
                assert!(c.remove(&node));
                assert!(!c.contains(&node));
                assert!(!c.remove(&node));
            }
        }
    }

    fn generate_tree(nodes: usize, start: usize, seed: i64) -> UGraph<usize> {
        let mut tree = UGraph::<usize>::simple();
        BarabasiAlbertForestGenerator::new(1, nodes, StdRng::seed_from_u64(seed as u64))
            .unwrap()
            .generate(&mut tree, |i| i + start)
            .unwrap();
        tree
    }

    fn connect_tree(g: &UGraph<usize>, c: &mut TreeDynamicConnectivity<usize>) {
        for v in g.vertices() {
            assert!(!c.contains(v));
            assert!(c.add(*v));
            assert!(c.contains(v));
        }
        for e in g.edges() {
            let (a, b) = g.edge_endpoints(e).unwrap();
            assert!(!c.connected(a, b));
            assert!(c.link(*a, *b));
            assert!(c.connected(a, b));
        }
    }

    fn destroy_tree(g: &UGraph<usize>, c: &mut TreeDynamicConnectivity<usize>) {
        for v in g.vertices() {
            assert!(c.contains(v));
            assert!(c.remove(v));
            assert!(!c.contains(v));
        }
    }

    #[test]
    fn test_2_trees() {
        for first_size in 1..50 {
            for second_size in 1..50 {
                let first = generate_tree(first_size, 0, 17 + first_size as i64);
                let second = generate_tree(second_size, first_size, 99 + second_size as i64);
                let mut c = TreeDynamicConnectivity::<usize>::new();
                connect_tree(&first, &mut c);
                connect_tree(&second, &mut c);

                for v1 in first.vertices() {
                    for v2 in second.vertices() {
                        assert!(!c.connected(v1, v2));

                        assert!(c.link(*v1, *v2));
                        assert!(c.connected(v1, v2));

                        assert!(!c.link(*v1, *v2));
                        assert!(c.connected(v1, v2));

                        assert!(c.cut(v1, v2).unwrap());
                        assert!(!c.connected(v1, v2));

                        assert!(!c.cut(v1, v2).unwrap());
                        assert!(!c.connected(v1, v2));
                    }
                }
                destroy_tree(&first, &mut c);
                destroy_tree(&second, &mut c);
            }
        }
    }
}
