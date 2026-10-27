//! Ports of the `org.jgrapht.alg.isomorphism` tests: IsomorphismTestUtil, GraphOrderingTest,
//! IsomorphicGraphMappingTest, the three AHU inspector tests, ColorRefinementIsomorphismInspectorTest
//! and the two VF2 inspector tests.
//!
//! Not ported: the `NullPointerException` cases (references cannot be null) and
//! `AHUForestIsomorphismInspectorTest.testMissingSupplier` (the Rust inspector never needs a fresh
//! vertex, so there is no supplier to be missing).

use jgrapht::alg::isomorphism::*;
use jgrapht::generate::GraphGenerator;
use jgrapht::graph::GraphExt;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

// IsomorphismTestUtil

fn int_graph(vertices: &[i32], edges: &[(i32, i32)]) -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple();
    for v in vertices {
        g.add_vertex(*v);
    }
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
    g
}

fn str_graph(vertices: &[&str], edges: &[(&str, &str)]) -> UGraph<String> {
    let mut g = UGraph::<String>::simple();
    for v in vertices {
        g.add_vertex(v.to_string());
    }
    for (a, b) in edges {
        g.add_edge(&a.to_string(), &b.to_string()).unwrap();
    }
    g
}

fn parse_graph(vertices: &str, edges: &str) -> UGraph<i32> {
    let vs: Vec<i32> = vertices[1..vertices.len() - 1]
        .split(", ")
        .map(|s| s.parse().unwrap())
        .collect();
    let es: Vec<(i32, i32)> = edges[1..edges.len() - 1]
        .split(", ")
        .map(|s| {
            let mut it = s[1..s.len() - 1].split(',');
            (
                it.next().unwrap().parse().unwrap(),
                it.next().unwrap().parse().unwrap(),
            )
        })
        .collect();
    int_graph(&vs, &es)
}

fn generate_mapped_graph<V: Clone + Eq + std::hash::Hash>(
    graph: &UGraph<V>,
    mapping: &HashMap<V, V>,
) -> UGraph<V> {
    let mut iso = UGraph::<V>::simple();
    for v in graph.vertices() {
        iso.add_vertex(mapping[v].clone());
    }
    for e in graph.edges() {
        let (u, v) = graph.edge_endpoints(e).unwrap();
        iso.add_edge(&mapping[u], &mapping[v]).unwrap();
    }
    iso
}

fn generate_isomorphic_graph(
    graph: &UGraph<i32>,
    rng: &mut StdRng,
) -> (UGraph<i32>, HashMap<i32, i32>) {
    let mut permutation: Vec<usize> = (0..graph.vertex_count()).collect();
    permutation.shuffle(rng);
    let vertices: Vec<i32> = graph.vertices().copied().collect();
    let mapping: HashMap<i32, i32> = (0..vertices.len())
        .map(|i| (vertices[i], vertices[permutation[i]]))
        .collect();
    (generate_mapped_graph(graph, &mapping), mapping)
}

fn counting_graph() -> UGraph<i32> {
    UGraph::<i32>::simple().with_vertex_factory(|i| i as i32)
}

fn generate_tree(n: usize, rng: StdRng) -> UGraph<i32> {
    let mut tree = counting_graph();
    jgrapht::generate::BarabasiAlbertGraphGenerator::new(1, 1, n - 1, rng)
        .unwrap()
        .generate_default(&mut tree)
        .unwrap();
    tree
}

fn generate_forest(n: usize, rng: StdRng) -> UGraph<i32> {
    let mut forest = counting_graph();
    jgrapht::generate::BarabasiAlbertForestGenerator::new(n / 10, n, rng)
        .unwrap()
        .generate_default(&mut forest)
        .unwrap();
    forest
}

/// Re-applies `mapping` onto `g1`, `g2` (which may differ from the graphs it was built on).
fn are_isomorphic<'g>(
    g1: &'g UGraph<i32>,
    g2: &'g UGraph<i32>,
    mapping: &IsomorphicGraphMapping<'_, UGraph<i32>>,
) -> bool {
    IsomorphicGraphMapping::from_maps(
        mapping.forward_mapping().clone(),
        mapping.backward_mapping().clone(),
        g1,
        g2,
    )
    .is_valid_isomorphism()
}

fn are_isomorphic_str<'g>(
    g1: &'g UGraph<String>,
    g2: &'g UGraph<String>,
    mapping: &IsomorphicGraphMapping<'_, UGraph<String>>,
) -> bool {
    IsomorphicGraphMapping::from_maps(
        mapping.forward_mapping().clone(),
        mapping.backward_mapping().clone(),
        g1,
        g2,
    )
    .is_valid_isomorphism()
}

fn sorted(v: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut v: Vec<usize> = v.into_iter().collect();
    v.sort_unstable();
    v
}

// GraphOrderingTest

mod graph_ordering_test {
    use super::*;

    fn check<G: Graph<Vertex = &'static str>>(
        g: &G,
        outs: [&[&'static str]; 5],
        ins: [&[&'static str]; 5],
        edges: &[(&'static str, &'static str)],
    ) {
        let ordering = GraphOrdering::new(g);
        assert_eq!(5, ordering.vertex_count());
        let names = ["v1", "v2", "v3", "v4", "v5"];
        let num = |s: &'static str| ordering.vertex_number(&s);
        for (i, name) in names.iter().enumerate() {
            let n = num(name);
            assert_eq!(
                sorted(outs[i].iter().map(|s| num(*s))),
                sorted(ordering.out_edges(n).iter().copied()),
                "out {name}"
            );
            assert_eq!(
                sorted(ins[i].iter().map(|s| num(*s))),
                sorted(ordering.in_edges(n).iter().copied()),
                "in {name}"
            );
        }
        for a in names {
            for b in names {
                assert_eq!(
                    edges.contains(&(a, b)),
                    ordering.has_edge(num(a), num(b)),
                    "{a}-{b}"
                );
            }
        }
    }

    #[test]
    fn undirected_graph() {
        let mut g1 = UGraph::<&'static str>::simple();
        for v in ["v1", "v2", "v3", "v4", "v5"] {
            g1.add_vertex(v);
        }
        for (a, b) in [("v1", "v2"), ("v1", "v3"), ("v1", "v4"), ("v2", "v4")] {
            g1.add_edge(&a, &b).unwrap();
        }
        let nbrs: [&[&'static str]; 5] = [
            &["v2", "v3", "v4"],
            &["v1", "v4"],
            &["v1"],
            &["v1", "v2"],
            &[],
        ];
        let edges = [
            ("v1", "v2"),
            ("v2", "v1"),
            ("v1", "v3"),
            ("v3", "v1"),
            ("v1", "v4"),
            ("v4", "v1"),
            ("v2", "v4"),
            ("v4", "v2"),
        ];
        check(&g1, nbrs, nbrs, &edges);
    }

    #[test]
    fn directed_graph() {
        let mut g1 = DiGraph::<&'static str>::simple();
        for v in ["v1", "v2", "v3", "v4", "v5"] {
            g1.add_vertex(v);
        }
        for (a, b) in [("v1", "v2"), ("v2", "v3"), ("v3", "v2"), ("v3", "v4")] {
            g1.add_edge(&a, &b).unwrap();
        }
        let outs: [&[&'static str]; 5] = [&["v2"], &["v3"], &["v2", "v4"], &[], &[]];
        let ins: [&[&'static str]; 5] = [&[], &["v1", "v3"], &["v2"], &["v3"], &[]];
        let edges = [("v1", "v2"), ("v2", "v3"), ("v3", "v2"), ("v3", "v4")];
        check(&g1, outs, ins, &edges);
    }
}

// IsomorphicGraphMappingTest

mod isomorphic_graph_mapping_test {
    use super::*;

    #[test]
    fn identity() {
        let tree1 = str_graph(
            &["A", "B", "C", "D", "E"],
            &[("A", "B"), ("A", "C"), ("C", "D"), ("C", "E")],
        );
        let identity = IsomorphicGraphMapping::identity(&tree1);
        let tree2 = generate_mapped_graph(
            &tree1,
            &identity
                .forward_mapping()
                .iter()
                .map(|(a, b)| (a.clone(), b.clone()))
                .collect(),
        );
        let a = identity
            .vertex_correspondence(&"A".to_string(), true)
            .unwrap();
        let isomorphism =
            AHURootedTreeIsomorphismInspector::new(&tree1, &"A".to_string(), &tree2, a).unwrap();
        assert!(isomorphism.isomorphism_exists());
        assert!(are_isomorphic_str(&tree1, &tree2, &identity));
    }

    #[test]
    fn composition_of_mappings() {
        let tree1 = str_graph(&["1", "2"], &[("1", "2")]);
        let tree2 = str_graph(&["a", "b"], &[("a", "b")]);
        let tree3 = str_graph(&["A", "B"], &[("A", "B")]);
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&tree1, &tree2).unwrap();
        assert!(iso.isomorphism_exists());
        let mapping12 = iso.mapping().unwrap();
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&tree2, &tree3).unwrap();
        assert!(iso.isomorphism_exists());
        let mapping23 = iso.mapping().unwrap();
        let mapping13 = mapping12.compose(&mapping23);
        assert!(are_isomorphic_str(&tree1, &tree3, &mapping13));
    }

    #[test]
    fn composition_of_random_mappings() {
        let mut random = StdRng::seed_from_u64(0x11_88_11);
        for test in 0..1000u64 {
            let n = 10 + random.random_range(0..150);
            let tree1 = generate_tree(n, StdRng::seed_from_u64(test));
            let tree2 = generate_isomorphic_graph(&tree1, &mut random).0;
            let tree3 = generate_isomorphic_graph(&tree2, &mut random).0;
            let mapping12 = AHUUnrootedTreeIsomorphismInspector::new(&tree1, &tree2)
                .unwrap()
                .mapping()
                .unwrap();
            let mapping23 = AHUUnrootedTreeIsomorphismInspector::new(&tree2, &tree3)
                .unwrap()
                .mapping()
                .unwrap();
            let mapping13 = mapping12.compose(&mapping23);
            assert!(are_isomorphic(&tree1, &tree3, &mapping13));
        }
    }
}

// AHURootedTreeIsomorphismInspectorTest

mod ahu_rooted_tree_isomorphism_inspector_test {
    use super::*;

    fn rooted<'g>(
        t1: &'g UGraph<i32>,
        r1: i32,
        t2: &'g UGraph<i32>,
        r2: i32,
    ) -> AHURootedTreeIsomorphismInspector<'g, UGraph<i32>> {
        AHURootedTreeIsomorphismInspector::new(t1, &r1, t2, &r2).unwrap()
    }

    fn assert_iso(t1: &UGraph<i32>, r1: i32, t2: &UGraph<i32>, r2: i32) {
        let iso = rooted(t1, r1, t2, r2);
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic(t1, t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn single_vertex() {
        let t1 = str_graph(&["1"], &[]);
        let t2 = str_graph(&["A"], &[]);
        let iso =
            AHURootedTreeIsomorphismInspector::new(&t1, &"1".to_string(), &t2, &"A".to_string())
                .unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn corner_case() {
        let mut t1 = int_graph(&(0..=10).collect::<Vec<_>>(), &[]);
        for (a, b) in [
            (10, 0),
            (10, 1),
            (10, 2),
            (10, 3),
            (0, 4),
            (0, 6),
            (0, 7),
            (2, 5),
            (5, 8),
            (4, 9),
        ] {
            t1.add_edge(&a, &b).unwrap();
        }
        let mut vs: Vec<i32> = (0..=9).collect();
        vs.push(11);
        let t2 = int_graph(
            &vs,
            &[
                (11, 1),
                (11, 2),
                (11, 4),
                (11, 7),
                (4, 3),
                (4, 6),
                (4, 0),
                (6, 5),
                (7, 8),
                (8, 9),
            ],
        );
        assert_iso(&t1, 10, &t2, 11);
    }

    #[test]
    fn corner_case2() {
        let t1 = parse_graph(
            "[1, 2, 5, 6, 8, 9, 10, 11, 14, 15]",
            "[{2,1}, {5,1}, {6,1}, {8,1}, {9,6}, {10,1}, {11,6}, {14,5}, {15,1}]",
        );
        let t2 = parse_graph(
            "[1, 18, 3, 19, 4, 5, 8, 9, 12, 13]",
            "[{8,12}, {3,12}, {18,12}, {9,12}, {5,18}, {19,12}, {13,12}, {4,13}, {1,18}]",
        );
        assert_iso(&t1, 1, &t2, 12);
    }

    #[test]
    fn corner_case3() {
        let t1 = parse_graph(
            "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]",
            "[{1,0}, {2,0}, {3,0}, {4,2}, {5,0}, {6,5}, {7,2}, {8,5}, {9,4}, {10,6}, {11,4}, {12,0}, {13,0}]",
        );
        let t2 = parse_graph(
            "[10, 2, 12, 7, 5, 3, 4, 0, 6, 1, 13, 9, 8, 11]",
            "[{2,10}, {12,10}, {7,10}, {5,12}, {3,10}, {4,3}, {0,12}, {6,3}, {1,5}, {13,4}, {9,5}, {8,10}, {11,10}]",
        );
        assert_iso(&t1, 0, &t2, 10);
    }

    #[test]
    fn non_isomorphic() {
        let t1 = str_graph(
            &["A", "B", "C", "D", "E", "F"],
            &[("A", "B"), ("A", "C"), ("B", "F"), ("C", "D"), ("C", "E")],
        );
        let t2 = str_graph(
            &["a", "b", "c", "d", "e", "f"],
            &[("a", "b"), ("a", "c"), ("c", "f"), ("c", "d"), ("c", "e")],
        );
        let iso =
            AHURootedTreeIsomorphismInspector::new(&t1, &"A".to_string(), &t2, &"a".to_string())
                .unwrap();
        assert!(!iso.isomorphism_exists());
        assert!(iso.mapping().is_none());
    }

    #[test]
    fn small() {
        let t1 = str_graph(
            &["A", "B", "C", "D", "E"],
            &[("A", "B"), ("A", "C"), ("C", "D"), ("C", "E")],
        );
        let t2 = str_graph(
            &["a", "b", "c", "d", "e"],
            &[("a", "b"), ("a", "c"), ("b", "e"), ("b", "d")],
        );
        let iso =
            AHURootedTreeIsomorphismInspector::new(&t1, &"A".to_string(), &t2, &"a".to_string())
                .unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn small2() {
        let t1 = int_graph(
            &(1..=13).collect::<Vec<_>>(),
            &[
                (1, 2),
                (1, 3),
                (2, 4),
                (2, 5),
                (2, 6),
                (3, 7),
                (3, 8),
                (3, 9),
                (8, 10),
                (8, 11),
                (9, 12),
                (9, 13),
            ],
        );
        let (t2, mapping) = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x88));
        assert_iso(&t1, 1, &t2, mapping[&1]);
    }

    #[test]
    fn disconnected_tree() {
        let t1 = int_graph(&[1, 2, 3], &[(1, 2)]);
        let t2 = int_graph(&[11, 21, 31], &[(11, 21)]);
        let iso = rooted(&t1, 1, &t2, 11);
        assert!(!iso.isomorphism_exists());
        assert!(iso.mapping().is_none());
        let forest = AHUForestIsomorphismInspector::new(&t1, [1, 3], &t2, [11, 31]).unwrap();
        assert!(forest.isomorphism_exists());
        assert!(are_isomorphic(&t1, &t2, &forest.mapping().unwrap()));
    }

    #[test]
    fn invalid_root() {
        let t1 = str_graph(&["a", "A"], &[]);
        let t2 = str_graph(&[], &[]);
        assert!(
            AHURootedTreeIsomorphismInspector::new(&t1, &"b".to_string(), &t2, &"A".to_string())
                .is_err()
        );
    }

    #[test]
    fn line_graph() {
        let n = 20_000;
        let edges: Vec<(i32, i32)> = (1..n).map(|i| (i, i + 1)).collect();
        let t1 = int_graph(&(1..=n).collect::<Vec<_>>(), &edges);
        let (t2, mapping) = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x88));
        assert_iso(&t1, 1, &t2, mapping[&1]);
    }

    #[test]
    fn huge_number_of_children() {
        let n = 100_000;
        let edges: Vec<(i32, i32)> = (2..=n).map(|i| (1, i)).collect();
        let t1 = int_graph(&(1..=n).collect::<Vec<_>>(), &edges);
        let (t2, mapping) = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x2882));
        assert_iso(&t1, 1, &t2, mapping[&1]);
    }
}

// AHUUnrootedTreeIsomorphismInspectorTest

mod ahu_unrooted_tree_isomorphism_inspector_test {
    use super::*;

    fn assert_iso(t1: &UGraph<i32>, t2: &UGraph<i32>) {
        let iso = AHUUnrootedTreeIsomorphismInspector::new(t1, t2).unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic(t1, t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn empty_graph() {
        let t = str_graph(&[], &[]);
        assert!(AHUUnrootedTreeIsomorphismInspector::new(&t, &t).is_err());
    }

    #[test]
    fn single_vertex() {
        let t1 = str_graph(&["1"], &[]);
        let t2 = str_graph(&["A"], &[]);
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&t1, &t2).unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn unrooted_isomorphism() {
        let t1 = str_graph(&["1", "2", "3"], &[("1", "2"), ("2", "3")]);
        let t2 = str_graph(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&t1, &t2).unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn corner_case() {
        let t1 = int_graph(
            &(0..=10).collect::<Vec<_>>(),
            &[
                (10, 0),
                (10, 1),
                (10, 2),
                (10, 3),
                (0, 4),
                (0, 6),
                (0, 7),
                (2, 5),
                (5, 8),
                (4, 9),
            ],
        );
        let mut vs: Vec<i32> = (0..=9).collect();
        vs.push(11);
        let t2 = int_graph(
            &vs,
            &[
                (11, 1),
                (11, 2),
                (11, 4),
                (11, 7),
                (4, 3),
                (4, 6),
                (4, 0),
                (6, 5),
                (7, 8),
                (8, 9),
            ],
        );
        assert_iso(&t1, &t2);
    }

    #[test]
    fn corner_case2() {
        let t1 = parse_graph(
            "[1, 2, 5, 6, 8, 9, 10, 11, 14, 15]",
            "[{2,1}, {5,1}, {6,1}, {8,1}, {9,6}, {10,1}, {11,6}, {14,5}, {15,1}]",
        );
        let t2 = parse_graph(
            "[1, 18, 3, 19, 4, 5, 8, 9, 12, 13]",
            "[{8,12}, {3,12}, {18,12}, {9,12}, {5,18}, {19,12}, {13,12}, {4,13}, {1,18}]",
        );
        assert_iso(&t1, &t2);
    }

    #[test]
    fn corner_case3() {
        let t1 = parse_graph(
            "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]",
            "[{1,0}, {2,0}, {3,0}, {4,2}, {5,0}, {6,5}, {7,2}, {8,5}, {9,4}, {10,6}, {11,4}, {12,0}, {13,0}]",
        );
        let t2 = parse_graph(
            "[10, 2, 12, 7, 5, 3, 4, 0, 6, 1, 13, 9, 8, 11]",
            "[{2,10}, {12,10}, {7,10}, {5,12}, {3,10}, {4,3}, {0,12}, {6,3}, {1,5}, {13,4}, {9,5}, {8,10}, {11,10}]",
        );
        assert_iso(&t1, &t2);
    }

    #[test]
    fn non_isomorphic_as_unrooted_but_as_rooted() {
        let t1 = str_graph(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        let t2 = str_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]);
        let rooted =
            AHURootedTreeIsomorphismInspector::new(&t1, &"A".to_string(), &t2, &"b".to_string())
                .unwrap();
        assert!(!rooted.isomorphism_exists());
        assert!(rooted.mapping().is_none());
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&t1, &t2).unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn small() {
        let t1 = str_graph(
            &["A", "B", "C", "D", "E"],
            &[("A", "B"), ("A", "C"), ("C", "D"), ("C", "E")],
        );
        let t2 = str_graph(
            &["a", "b", "c", "d", "e"],
            &[("a", "b"), ("a", "c"), ("b", "e"), ("b", "d")],
        );
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&t1, &t2).unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn small2() {
        let t1 = int_graph(
            &(1..=13).collect::<Vec<_>>(),
            &[
                (1, 2),
                (1, 3),
                (2, 4),
                (2, 5),
                (2, 6),
                (3, 7),
                (3, 8),
                (3, 9),
                (8, 10),
                (8, 11),
                (9, 12),
                (9, 13),
            ],
        );
        let t2 = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x88)).0;
        assert_iso(&t1, &t2);
    }

    #[test]
    fn disconnected_tree() {
        let t1 = int_graph(&[1, 2, 3], &[(1, 2)]);
        let t2 = int_graph(&[11, 21, 31], &[(11, 21)]);
        let iso = AHUUnrootedTreeIsomorphismInspector::new(&t1, &t2).unwrap();
        assert!(!iso.isomorphism_exists());
        assert!(iso.mapping().is_none());
        let forest = AHUForestIsomorphismInspector::new(&t1, [1, 3], &t2, [11, 31]).unwrap();
        assert!(forest.isomorphism_exists());
        assert!(are_isomorphic(&t1, &t2, &forest.mapping().unwrap()));
    }

    #[test]
    fn invalid_root() {
        let t1 = str_graph(&["a", "A"], &[]);
        let t2 = str_graph(&[], &[]);
        assert!(AHUUnrootedTreeIsomorphismInspector::new(&t1, &t2).is_err());
    }

    #[test]
    fn line_graph() {
        let n = 20_000;
        let edges: Vec<(i32, i32)> = (1..n).map(|i| (i, i + 1)).collect();
        let t1 = int_graph(&(1..=n).collect::<Vec<_>>(), &edges);
        let t2 = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x88)).0;
        assert_iso(&t1, &t2);
    }

    #[test]
    fn huge_number_of_children() {
        let n = 100_000;
        let edges: Vec<(i32, i32)> = (2..=n).map(|i| (1, i)).collect();
        let t1 = int_graph(&(1..=n).collect::<Vec<_>>(), &edges);
        let t2 = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x2882)).0;
        assert_iso(&t1, &t2);
    }

    #[test]
    fn huge_random_tree() {
        let t1 = generate_tree(50_000, StdRng::seed_from_u64(0x88));
        let t2 = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x88)).0;
        assert_iso(&t1, &t2);
        assert_iso(&t1, &t2);
    }

    #[test]
    fn random_trees() {
        let mut random = StdRng::seed_from_u64(0x88_88);
        for test in 0..1500u64 {
            let n = 10 + random.random_range(0..100);
            let t1 = generate_tree(n, StdRng::seed_from_u64(test));
            let t2 = generate_isomorphic_graph(&t1, &mut random).0;
            assert_iso(&t1, &t2);
        }
    }
}

// AHUForestIsomorphismInspectorTest

mod ahu_forest_isomorphism_inspector_test {
    use super::*;
    use jgrapht::alg::connectivity::ConnectivityInspector;

    fn roots(forest: &UGraph<i32>) -> HashSet<i32> {
        ConnectivityInspector::new()
            .connected_sets(forest)
            .iter()
            .map(|s| *s.iter().min().unwrap())
            .collect()
    }

    fn assert_iso(f1: &UGraph<i32>, r1: HashSet<i32>, f2: &UGraph<i32>, r2: HashSet<i32>) {
        let iso = AHUForestIsomorphismInspector::new(f1, r1, f2, r2).unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic(f1, f2, &iso.mapping().unwrap()));
    }

    #[test]
    fn empty_graph() {
        let t = str_graph(&[], &[]);
        assert!(AHUForestIsomorphismInspector::new(&t, [], &t, []).is_err());
    }

    #[test]
    fn single_vertex() {
        let t1 = str_graph(&["1"], &[]);
        let t2 = str_graph(&["A"], &[]);
        let iso =
            AHUForestIsomorphismInspector::new(&t1, ["1".to_string()], &t2, ["A".to_string()])
                .unwrap();
        assert!(iso.isomorphism_exists());
        assert!(are_isomorphic_str(&t1, &t2, &iso.mapping().unwrap()));
    }

    #[test]
    fn invalid_root() {
        let t1 = str_graph(&["a", "A"], &[]);
        let t2 = str_graph(&[], &[]);
        assert!(
            AHUForestIsomorphismInspector::new(&t1, ["b".to_string()], &t2, ["A".to_string()])
                .is_err()
        );
    }

    #[test]
    fn small_forest() {
        let t1 = str_graph(&["a", "b", "c", "d"], &[("a", "b"), ("a", "c")]);
        let t2 = str_graph(&["A", "B", "C", "D"], &[("B", "A"), ("A", "C")]);
        let iso = AHUForestIsomorphismInspector::new(
            &t1,
            ["b".to_string(), "d".to_string()],
            &t2,
            ["A".to_string(), "D".to_string()],
        )
        .unwrap();
        assert!(!iso.isomorphism_exists());
    }

    #[test]
    fn small_forest2() {
        let (vs, es) = (
            "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]",
            "[{2,1}, {3,0}, {4,0}, {5,1}, {6,1}, {7,0}, {8,1}, {9,6}, {10,1}, {11,6}, \
             {12,0}, {13,7}, {14,5}, {15,1}, {16,0}, {17,0}, {18,17}, {19,7}]",
        );
        let map: HashMap<i32, i32> = [
            (0, 12),
            (1, 10),
            (2, 0),
            (3, 8),
            (4, 3),
            (5, 16),
            (6, 7),
            (7, 18),
            (8, 11),
            (9, 17),
            (10, 6),
            (11, 14),
            (12, 9),
            (13, 5),
            (14, 15),
            (15, 2),
            (16, 19),
            (17, 13),
            (18, 4),
            (19, 1),
        ]
        .into();
        let forest1 = parse_graph(vs, es);
        let forest2 = generate_mapped_graph(&forest1, &map);
        let roots1 = roots(&forest1);
        let roots2 = roots1.iter().map(|r| map[r]).collect();
        assert_iso(&forest1, roots1, &forest2, roots2);
    }

    #[test]
    fn huge_number_of_children() {
        let n = 100_000;
        let edges: Vec<(i32, i32)> = (2..=n).map(|i| (1, i)).collect();
        let t1 = int_graph(&(1..=n).collect::<Vec<_>>(), &edges);
        let (t2, mapping) = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x2882));
        assert_iso(&t1, [1].into(), &t2, [mapping[&1]].into());
    }

    #[test]
    fn random_forests() {
        let mut random = StdRng::seed_from_u64(0x2312);
        for test in 0..1000u64 {
            let n = 10 + random.random_range(0..200);
            let t1 = generate_forest(n, StdRng::seed_from_u64(test));
            let (t2, mapping) = generate_isomorphic_graph(&t1, &mut random);
            let roots1 = roots(&t1);
            let roots2 = roots1.iter().map(|r| mapping[r]).collect();
            assert_iso(&t1, roots1, &t2, roots2);
        }
    }

    #[test]
    fn huge_random_forest() {
        let t1 = generate_forest(50_000, StdRng::seed_from_u64(0x88));
        let (t2, mapping) = generate_isomorphic_graph(&t1, &mut StdRng::seed_from_u64(0x88));
        let roots1 = roots(&t1);
        let roots2 = roots1.iter().map(|r| mapping[r]).collect();
        assert_iso(&t1, roots1, &t2, roots2);
    }
}

// ColorRefinementIsomorphismInspectorTest

mod color_refinement_isomorphism_inspector_test {
    use super::*;

    fn undirected(n: i32, edges: &[(i32, i32)]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::with_loops();
        for v in 1..=n {
            g.add_vertex(v);
        }
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    fn map(m: &IsomorphicGraphMapping<'_, UGraph<i32>>, v: i32, forward: bool) -> i32 {
        *m.vertex_correspondence(&v, forward).unwrap()
    }

    fn all_distinct(m: &IsomorphicGraphMapping<'_, UGraph<i32>>, n: i32) {
        for i in 1..=n {
            for j in i + 1..=n {
                assert_ne!(map(m, i, true), map(m, j, true));
                assert_ne!(map(m, i, false), map(m, j, false));
            }
        }
    }

    #[test]
    fn mappings_for_regular_graphs() {
        let g1 = undirected(6, &[(1, 2), (2, 3), (3, 1), (4, 5), (5, 6), (6, 4)]);
        let g2 = undirected(6, &[(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 1)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert_eq!(Err(IsomorphismUndecidable), inspector.isomorphism_exists());
        assert!(inspector.mappings().is_empty());
        assert!(!inspector.is_coloring_discrete());
        assert!(!inspector.is_forest());
    }

    #[test]
    fn mappings_for_trees() {
        let g1 = undirected(7, &[(1, 2), (1, 3), (2, 4), (2, 5), (3, 6), (3, 7)]);
        let g2 = undirected(7, &[(1, 2), (2, 3), (2, 4), (4, 5), (5, 6), (5, 7)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert_eq!(Ok(true), inspector.isomorphism_exists());
        assert!(!inspector.is_coloring_discrete());
        assert!(inspector.is_forest());
        let m = inspector.mappings().remove(0);
        assert_eq!(4, map(&m, 1, true));
        for v in [2, 3] {
            assert!([2, 5].contains(&map(&m, v, true)));
        }
        for v in [4, 5, 6, 7] {
            assert!([1, 3, 6, 7].contains(&map(&m, v, true)));
        }
        assert_eq!(1, map(&m, 4, true));
        for v in [2, 5] {
            assert!([2, 3].contains(&map(&m, v, false)));
        }
        for v in [1, 3, 6, 7] {
            assert!([4, 5, 6, 7].contains(&map(&m, v, false)));
        }
        all_distinct(&m, 7);
    }

    #[test]
    fn mappings_for_isomorphic_graphs_of_size6() {
        let g1 = undirected(6, &[(1, 2), (1, 3), (1, 6), (2, 3), (3, 4), (4, 5)]);
        let g2 = undirected(6, &[(1, 2), (1, 3), (1, 5), (2, 6), (3, 5), (3, 4)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert_eq!(Ok(true), inspector.isomorphism_exists());
        assert!(inspector.is_coloring_discrete());
        assert!(!inspector.is_forest());
        let m = inspector.mappings().remove(0);
        for (v, w) in [(1, 3), (2, 5), (3, 1), (4, 2), (5, 6), (6, 4)] {
            assert_eq!(w, map(&m, v, true));
        }
        for (v, w) in [(1, 3), (2, 4), (3, 1), (4, 6), (5, 2), (6, 5)] {
            assert_eq!(w, map(&m, v, false));
        }
        assert!(m.is_valid_isomorphism());
    }

    #[test]
    fn mapping_for_graph_with_different_number_of_nodes() {
        let g1 = undirected(5, &[(1, 3), (2, 5), (3, 4), (4, 5)]);
        let g2 = undirected(6, &[(1, 3), (2, 5), (3, 4), (4, 5)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert!(inspector.mappings().is_empty());
        assert_eq!(Ok(false), inspector.isomorphism_exists());
        assert!(!inspector.is_coloring_discrete());
        assert!(!inspector.is_forest());
    }

    #[test]
    fn mappings_for_graph_with_different_number_of_color_classes() {
        // as in the Java test, the edges end up in graph1 only
        let g1 = undirected(4, &[(1, 2), (1, 3), (2, 3), (3, 4), (2, 4)]);
        let g2 = undirected(4, &[]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert!(inspector.mappings().is_empty());
        assert_eq!(Ok(false), inspector.isomorphism_exists());
        assert!(!inspector.is_coloring_discrete());
        assert!(!inspector.is_forest());
    }

    #[test]
    fn mappings_for_isomorphic_forests() {
        let g1 = undirected(7, &[(1, 2), (1, 3), (4, 5), (5, 6), (6, 7)]);
        let g2 = undirected(7, &[(1, 2), (1, 3), (3, 4), (5, 6), (6, 7)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert_eq!(Ok(true), inspector.isomorphism_exists());
        assert!(!inspector.is_coloring_discrete());
        assert!(inspector.is_forest());
        let m = inspector.mappings().remove(0);
        assert_eq!(6, map(&m, 1, true));
        for (v, options) in [
            (2, [5, 7]),
            (3, [5, 7]),
            (4, [2, 4]),
            (5, [1, 3]),
            (6, [1, 3]),
            (7, [2, 4]),
        ] {
            assert!(options.contains(&map(&m, v, true)), "vertex {v}");
        }
        all_distinct(&m, 7);
    }

    #[test]
    fn mappings_for_not_isomorphic_forests() {
        let g1 = undirected(4, &[(1, 2), (1, 3), (1, 4)]);
        let g2 = undirected(4, &[(1, 2), (1, 3), (2, 4)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert!(inspector.mappings().is_empty());
        assert_eq!(Ok(false), inspector.isomorphism_exists());
        assert!(!inspector.is_coloring_discrete());
        assert!(!inspector.is_forest());
    }

    #[test]
    fn two_discrete_graphs_non_isomorphic() {
        let g1 = undirected(6, &[(1, 2), (2, 3), (3, 4), (3, 5), (4, 5), (5, 6)]);
        let g2 = undirected(6, &[(1, 2), (2, 3), (2, 4), (2, 5), (3, 5), (3, 6), (4, 5)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g2).unwrap();
        assert_eq!(Ok(false), inspector.isomorphism_exists());
        assert!(inspector.mappings().is_empty());
    }

    #[test]
    fn two_equal_graphs() {
        let g1 = undirected(6, &[(1, 2), (2, 3), (3, 4), (3, 5), (4, 5), (5, 6)]);
        let mut inspector = ColorRefinementIsomorphismInspector::new(&g1, &g1).unwrap();
        assert_eq!(Ok(true), inspector.isomorphism_exists());
        assert!(!inspector.mappings().is_empty());
    }
}

// SubgraphIsomorphismTestUtils

mod subgraph_utils {
    use super::*;
    use jgrapht::util::java::JavaRandom;

    pub type G = DiGraph<i32>;

    pub fn is_correct_matching<H: Graph<Vertex = i32>>(
        rel: &IsomorphicGraphMapping<'_, H>,
        g1: &H,
        g2: &H,
    ) -> bool {
        for u1 in g2.vertices() {
            let v1 = rel.vertex_correspondence(u1, false);
            for u2 in g2.vertices() {
                if u1 == u2 {
                    continue;
                }
                let v2 = rel.vertex_correspondence(u2, false);
                if v1 == v2 {
                    return false;
                }
                let (v1, v2) = (v1.unwrap(), v2.unwrap());
                if g1.contains_edge_between(v1, v2) != g2.contains_edge_between(u1, u2) {
                    return false;
                }
            }
        }
        true
    }

    pub fn all_matchings_correct<H: Graph<Vertex = i32>>(
        vf2: &VF2SubgraphIsomorphismInspector<'_, H>,
        g1: &H,
        g2: &H,
    ) -> bool {
        vf2.iter().all(|m| is_correct_matching(&m, g1, g2))
    }

    pub fn random_subgraph(g1: &G, vertex_count: usize, seed: i64) -> G {
        let mut map: HashMap<i32, i32> = HashMap::new();
        let mut g2 = G::with_loops();
        let n = g1.vertex_count() as i32;
        let mut rnd = JavaRandom::new(seed);
        let mut i = 0;
        while (i as usize) < vertex_count {
            for v in g1.vertices() {
                if rnd.next_int_bound(n) == 0 && !map.contains_key(v) {
                    let u = i;
                    i += 1;
                    g2.add_vertex(u);
                    map.insert(*v, u);
                }
            }
        }
        for e in g1.edges() {
            let (v1, v2) = g1.edge_endpoints(e).unwrap();
            if let (Some(u1), Some(u2)) = (map.get(v1), map.get(v2)) {
                g2.add_edge(u1, u2).unwrap();
            }
        }
        g2
    }

    pub fn random_graph(vertex_count: i32, edge_count: usize, seed: i64) -> G {
        let mut g = G::with_loops();
        for i in 0..vertex_count {
            g.add_vertex(i);
        }
        let mut rnd = JavaRandom::new(seed);
        let mut i = 0;
        while i < edge_count {
            let source = rnd.next_int_bound(vertex_count);
            let target = rnd.next_int_bound(vertex_count);
            if source != target && !g.contains_edge_between(&source, &target) {
                g.add_edge(&source, &target).unwrap();
                i += 1;
            }
        }
        g
    }

    /// Every injective, edge-consistent mapping of `g2`'s vertices into `g1`, built by brute
    /// force (`getMatchings`), compared with what the inspector found.
    pub fn contains_all_matchings<'g, H: Graph<Vertex = i32>>(
        vf2: &VF2SubgraphIsomorphismInspector<'g, H>,
        g1: &'g H,
        g2: &'g H,
    ) -> bool {
        fn permutations(used: &mut Vec<bool>, len: usize) -> Vec<Vec<usize>> {
            if len == 0 {
                return vec![vec![]];
            }
            let mut perms = Vec::new();
            for i in 0..used.len() {
                if !used[i] {
                    used[i] = true;
                    let mut sub = permutations(used, len - 1);
                    used[i] = false;
                    for p in &mut sub {
                        p.push(i);
                    }
                    perms.extend(sub);
                }
            }
            perms
        }
        let (n1, n2) = (g1.vertex_count(), g2.vertex_count());
        let (o1, o2) = (GraphOrdering::new(g1), GraphOrdering::new(g2));
        let mut matchings = Vec::new();
        'perm: for perm in permutations(&mut vec![false; n1], n2) {
            let core2 = perm;
            for e in g2.edges() {
                let (u1, u2) = g2.edge_endpoints(e).unwrap();
                let (v1, v2) = (core2[*u1 as usize] as i32, core2[*u2 as usize] as i32);
                if !g1.contains_edge_between(&v1, &v2) {
                    continue 'perm;
                }
            }
            let mut core1 = vec![NULL_NODE; n1];
            for (i, &c) in core2.iter().enumerate() {
                core1[c] = i;
            }
            for e in g1.edges() {
                let (v1, v2) = g1.edge_endpoints(e).unwrap();
                let (u1, u2) = (core1[*v1 as usize], core1[*v2 as usize]);
                if u1 == NULL_NODE || u2 == NULL_NODE {
                    continue;
                }
                if !g2.contains_edge_between(&(u1 as i32), &(u2 as i32)) {
                    continue 'perm;
                }
            }
            matchings.push(IsomorphicGraphMapping::from_cores(&o1, &o2, &core1, &core2));
        }
        let mut correct = true;
        for rel1 in vf2.iter() {
            match matchings
                .iter()
                .position(|rel2| rel1.is_equal_mapping(rel2))
            {
                Some(i) => {
                    matchings.remove(i);
                }
                None => correct = false,
            }
        }
        correct && matchings.is_empty()
    }
}

// VF2GraphIsomorphismInspectorTest

mod vf2_graph_isomorphism_inspector_test {
    use super::subgraph_utils::*;
    use super::*;

    #[test]
    fn automorphism() {
        let mut g1 = UGraph::<String>::simple();
        for v in ["v1", "v2", "v3"] {
            g1.add_vertex(v.to_string());
        }
        for (a, b) in [("v1", "v2"), ("v2", "v3"), ("v3", "v1")] {
            g1.add_edge(&a.to_string(), &b.to_string()).unwrap();
        }
        let vf2 = VF2GraphIsomorphismInspector::new(&g1, &g1).unwrap();
        let mut iter = vf2.iter();
        let mut mappings: HashSet<&str> = [
            "[v1=v1 v2=v2 v3=v3]",
            "[v1=v1 v2=v3 v3=v2]",
            "[v1=v2 v2=v1 v3=v3]",
            "[v1=v2 v2=v3 v3=v1]",
            "[v1=v3 v2=v1 v3=v2]",
            "[v1=v3 v2=v2 v3=v1]",
        ]
        .into();
        for _ in 0..6 {
            assert!(mappings.remove(iter.next().unwrap().to_string().as_str()));
        }
        assert!(!iter.has_next());

        let mut g2 = DiGraph::<i32>::with_loops();
        for v in 1..=3 {
            g2.add_vertex(v);
        }
        g2.add_edge(&1, &2).unwrap();
        g2.add_edge(&3, &2).unwrap();
        let vf3 = VF2GraphIsomorphismInspector::new(&g2, &g2).unwrap();
        let mut iter2 = vf3.iter();
        let mappings2 = ["[1=1 2=2 3=3]", "[1=3 2=2 3=1]"];
        assert!(mappings2.contains(&iter2.next().unwrap().to_string().as_str()));
        assert!(mappings2.contains(&iter2.next().unwrap().to_string().as_str()));
        assert!(!iter2.has_next());
    }

    #[test]
    fn subgraph() {
        let g1 = random_graph(10, 30, 12345);
        let g2 = random_subgraph(&g1, 7, 54321);
        let vf2 = VF2GraphIsomorphismInspector::new(&g1, &g2).unwrap();
        assert!(!vf2.isomorphism_exists());
    }

    #[test]
    fn subgraph2() {
        fn graph(n: i32, edges: &[(i32, i32)]) -> UGraph<i32> {
            let mut g = UGraph::<i32>::simple();
            for v in 0..n {
                g.add_vertex(v);
            }
            for (a, b) in edges {
                g.add_edge(a, b).unwrap();
            }
            g
        }
        let g1 = graph(5, &[(0, 1), (0, 4), (1, 2), (1, 4), (2, 3), (2, 4), (3, 4)]);
        let g2 = graph(6, &[(4, 2), (2, 0), (0, 1), (1, 3), (3, 4), (4, 0), (1, 4)]);
        let g3 = graph(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (5, 2), (5, 3), (5, 4)]);
        let g4 = graph(6, &[(0, 1), (1, 2), (2, 3), (4, 5), (4, 2), (5, 2), (5, 3)]);
        assert!(
            !VF2GraphIsomorphismInspector::new(&g2, &g1)
                .unwrap()
                .isomorphism_exists()
        );
        assert!(
            !VF2GraphIsomorphismInspector::new(&g1, &g2)
                .unwrap()
                .isomorphism_exists()
        );
        assert!(
            !VF2GraphIsomorphismInspector::new(&g3, &g4)
                .unwrap()
                .isomorphism_exists()
        );
    }
}

// VF2SubgraphIsomorphismInspectorTest

mod vf2_subgraph_isomorphism_inspector_test {
    use super::subgraph_utils::*;
    use super::*;
    use jgrapht::graph::Undirected;
    use jgrapht::util::java::JavaRandom;
    use std::cmp::Ordering;

    fn sg(vertices: &[i32], edges: &[(i32, i32)]) -> UGraph<i32> {
        int_graph(vertices, edges)
    }

    fn dg(vertices: &[i32], edges: &[(i32, i32)]) -> DiGraph<i32> {
        let mut g = DiGraph::<i32>::with_loops();
        for v in vertices {
            g.add_vertex(*v);
        }
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    /// Asserts the iterator yields exactly the `expected` mappings, in any order.
    fn assert_mappings<'a, 'g, G: Graph + 'g>(
        mut iter: VF2MappingIterator<'a, 'g, G>,
        expected: &[&str],
    ) where
        G::Vertex: std::fmt::Display,
    {
        let mut left: HashSet<&str> = expected.iter().copied().collect();
        assert_eq!(left.len(), expected.len());
        for _ in 0..expected.len() {
            let m = iter
                .next()
                .expect("fewer mappings than expected")
                .to_string();
            assert!(left.remove(m.as_str()), "unexpected mapping {m}");
        }
        assert!(!iter.has_next());
    }

    fn first<G: Graph>(vf2: &VF2SubgraphIsomorphismInspector<'_, G>) -> String
    where
        G::Vertex: std::fmt::Display,
    {
        vf2.iter().next().unwrap().to_string()
    }

    #[test]
    fn graph_types() {
        let dg1 = dg(&[1, 2], &[(1, 2)]);
        let sg1 = sg(&[1, 2], &[(1, 2)]);
        let mut mg1 = UGraph::<i32>::multi();
        let mut pg1 = UGraph::<i32>::pseudo();
        for g in [&mut mg1, &mut pg1] {
            g.add_vertex(1);
            g.add_vertex(2);
            g.add_edge(&1, &2).unwrap();
        }
        // GT-1, GT-2: multigraphs and pseudographs
        assert!(VF2SubgraphIsomorphismInspector::new(&mg1, &mg1).is_err());
        assert!(VF2SubgraphIsomorphismInspector::new(&pg1, &pg1).is_err());
        // GT-3: simple graphs
        assert!(
            VF2SubgraphIsomorphismInspector::new(&sg1, &sg1)
                .unwrap()
                .iter()
                .has_next()
        );
        // GT-4: directed graphs
        let gt4 = VF2SubgraphIsomorphismInspector::new(&dg1, &dg1).unwrap();
        assert_eq!("[1=1 2=2]", first(&gt4));
        // GT-5..GT-9: combinations with multigraphs and pseudographs
        assert!(VF2SubgraphIsomorphismInspector::new(&sg1, &mg1).is_err());
        assert!(VF2SubgraphIsomorphismInspector::new(&sg1, &pg1).is_err());
        assert!(VF2SubgraphIsomorphismInspector::new(&pg1, &mg1).is_err());
        // GT-10 (simple graph + directed graph) cannot be written: direction is part of the
        // graph's type, so the compiler rejects the combination.
    }

    #[test]
    fn edge_cases_simple_graph() {
        // ECS-1
        let sg0v = sg(&[], &[]);
        let sg0v2 = sg(&[], &[]);
        let vfs1 = VF2SubgraphIsomorphismInspector::new(&sg0v, &sg0v2).unwrap();
        assert_eq!("[]", first(&vfs1));
        // ECS-2
        let sg4v3e = sg(&[1, 2, 3, 4], &[(1, 2), (3, 2), (3, 4)]);
        let vfs2 = VF2SubgraphIsomorphismInspector::new(&sg4v3e, &sg0v).unwrap();
        assert_eq!("[1=~~ 2=~~ 3=~~ 4=~~]", first(&vfs2));
        // ECS-3
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&sg0v, &sg4v3e)
                .unwrap()
                .isomorphism_exists()
        );
        // ECS-4
        let sg1v = sg(&[5], &[]);
        let vfs4 = VF2SubgraphIsomorphismInspector::new(&sg4v3e, &sg1v).unwrap();
        assert_mappings(
            vfs4.iter(),
            &[
                "[1=5 2=~~ 3=~~ 4=~~]",
                "[1=~~ 2=5 3=~~ 4=~~]",
                "[1=~~ 2=~~ 3=5 4=~~]",
                "[1=~~ 2=~~ 3=~~ 4=5]",
            ],
        );
        // ECS-5
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&sg0v, &sg1v)
                .unwrap()
                .isomorphism_exists()
        );
        // ECS-6
        let sg3v0e = sg(&[5, 6, 7], &[]);
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&sg4v3e, &sg3v0e)
                .unwrap()
                .isomorphism_exists()
        );
        // ECS-7
        let sg2v0e = sg(&[1, 2], &[]);
        let vfs7 = VF2SubgraphIsomorphismInspector::new(&sg3v0e, &sg2v0e).unwrap();
        assert_mappings(
            vfs7.iter(),
            &[
                "[5=1 6=2 7=~~]",
                "[5=1 6=~~ 7=2]",
                "[5=2 6=1 7=~~]",
                "[5=~~ 6=1 7=2]",
                "[5=2 6=~~ 7=1]",
                "[5=~~ 6=2 7=1]",
            ],
        );
        // ECS-8
        let sg2v1e = sg(&[5, 6], &[(5, 6)]);
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&sg3v0e, &sg2v1e)
                .unwrap()
                .isomorphism_exists()
        );
        // ECS-9
        let sg5k = sg(
            &[0, 1, 2, 3, 4],
            &[
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (1, 2),
                (1, 3),
                (1, 4),
                (2, 3),
                (2, 4),
                (3, 4),
            ],
        );
        let sg4k = sg(
            &[0, 1, 2, 3],
            &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        );
        let sg3k = sg(&[0, 1, 2], &[(0, 1), (0, 2), (1, 2)]);
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&sg4k, &sg5k)
                .unwrap()
                .isomorphism_exists()
        );
        // ECS-10
        let vfs10 = VF2SubgraphIsomorphismInspector::new(&sg4k, &sg3k).unwrap();
        assert_mappings(
            vfs10.iter(),
            &[
                "[0=0 1=1 2=2 3=~~]",
                "[0=0 1=1 2=~~ 3=2]",
                "[0=0 1=~~ 2=1 3=2]",
                "[0=~~ 1=0 2=1 3=2]",
                "[0=1 1=0 2=2 3=~~]",
                "[0=1 1=0 2=~~ 3=2]",
                "[0=1 1=~~ 2=0 3=2]",
                "[0=~~ 1=1 2=0 3=2]",
                "[0=2 1=1 2=0 3=~~]",
                "[0=2 1=1 2=~~ 3=0]",
                "[0=2 1=~~ 2=1 3=0]",
                "[0=~~ 1=2 2=1 3=0]",
                "[0=0 1=2 2=1 3=~~]",
                "[0=0 1=2 2=~~ 3=1]",
                "[0=0 1=~~ 2=2 3=1]",
                "[0=~~ 1=0 2=2 3=1]",
                "[0=1 1=2 2=0 3=~~]",
                "[0=1 1=2 2=~~ 3=0]",
                "[0=1 1=~~ 2=2 3=0]",
                "[0=~~ 1=1 2=2 3=0]",
                "[0=2 1=0 2=1 3=~~]",
                "[0=2 1=0 2=~~ 3=1]",
                "[0=2 1=~~ 2=0 3=1]",
                "[0=~~ 1=2 2=0 3=1]",
            ],
        );
        // ECS-11
        let vfs11 = VF2SubgraphIsomorphismInspector::new(&sg4v3e, &sg4v3e).unwrap();
        assert_mappings(vfs11.iter(), &["[1=1 2=2 3=3 4=4]", "[1=4 2=3 3=2 4=1]"]);
        // ECS-12
        let sg6v4enc = sg(&[0, 1, 2, 3, 4, 5], &[(1, 2), (2, 3), (3, 1), (4, 5)]);
        let sg5v4enc = sg(&[6, 7, 8, 9, 10], &[(7, 6), (9, 8), (10, 9), (8, 10)]);
        let vfs12 = VF2SubgraphIsomorphismInspector::new(&sg6v4enc, &sg5v4enc).unwrap();
        assert_mappings(
            vfs12.iter(),
            &[
                "[0=~~ 1=8 2=10 3=9 4=7 5=6]",
                "[0=~~ 1=9 2=8 3=10 4=7 5=6]",
                "[0=~~ 1=10 2=9 3=8 4=7 5=6]",
                "[0=~~ 1=8 2=10 3=9 4=6 5=7]",
                "[0=~~ 1=9 2=8 3=10 4=6 5=7]",
                "[0=~~ 1=10 2=9 3=8 4=6 5=7]",
                "[0=~~ 1=10 2=8 3=9 4=7 5=6]",
                "[0=~~ 1=8 2=9 3=10 4=7 5=6]",
                "[0=~~ 1=9 2=10 3=8 4=7 5=6]",
                "[0=~~ 1=10 2=8 3=9 4=6 5=7]",
                "[0=~~ 1=8 2=9 3=10 4=6 5=7]",
                "[0=~~ 1=9 2=10 3=8 4=6 5=7]",
            ],
        );
    }

    #[test]
    fn edge_cases_directed_graph() {
        // ECD-1
        let dg0v = dg(&[], &[]);
        let dg0v2 = dg(&[], &[]);
        let vf1 = VF2SubgraphIsomorphismInspector::new(&dg0v, &dg0v2).unwrap();
        assert_eq!("[]", first(&vf1));
        // ECD-2
        let dg4v3e = dg(&[1, 2, 3, 4], &[(1, 2), (3, 2), (3, 4)]);
        let vf2 = VF2SubgraphIsomorphismInspector::new(&dg4v3e, &dg0v).unwrap();
        assert_eq!("[1=~~ 2=~~ 3=~~ 4=~~]", first(&vf2));
        // ECD-3
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&dg0v, &dg4v3e)
                .unwrap()
                .isomorphism_exists()
        );
        // ECD-4
        let dg1v = dg(&[5], &[]);
        let vf4 = VF2SubgraphIsomorphismInspector::new(&dg4v3e, &dg1v).unwrap();
        assert_mappings(
            vf4.iter(),
            &[
                "[1=5 2=~~ 3=~~ 4=~~]",
                "[1=~~ 2=5 3=~~ 4=~~]",
                "[1=~~ 2=~~ 3=5 4=~~]",
                "[1=~~ 2=~~ 3=~~ 4=5]",
            ],
        );
        // ECD-5
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&dg0v, &dg1v)
                .unwrap()
                .isomorphism_exists()
        );
        // ECD-6
        let dg3v0e = dg(&[5, 6, 7], &[]);
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&dg4v3e, &dg3v0e)
                .unwrap()
                .isomorphism_exists()
        );
        // ECD-7
        let dg2v0e = dg(&[1, 2], &[]);
        let vf7 = VF2SubgraphIsomorphismInspector::new(&dg3v0e, &dg2v0e).unwrap();
        assert_mappings(
            vf7.iter(),
            &[
                "[5=1 6=2 7=~~]",
                "[5=1 6=~~ 7=2]",
                "[5=2 6=1 7=~~]",
                "[5=~~ 6=1 7=2]",
                "[5=2 6=~~ 7=1]",
                "[5=~~ 6=2 7=1]",
            ],
        );
        // ECD-8
        let dg2v1e = dg(&[5, 6], &[(5, 6)]);
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&dg3v0e, &dg2v1e)
                .unwrap()
                .isomorphism_exists()
        );
        // ECD-9
        let dg5c = dg(
            &[0, 1, 2, 3, 4],
            &[
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (1, 2),
                (1, 3),
                (1, 4),
                (2, 3),
                (2, 4),
                (3, 4),
            ],
        );
        let dg4c = dg(
            &[0, 1, 2, 3],
            &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        );
        assert!(
            !VF2SubgraphIsomorphismInspector::new(&dg4c, &dg5c)
                .unwrap()
                .isomorphism_exists()
        );
        // ECD-10
        let vf10 = VF2SubgraphIsomorphismInspector::new(&dg5c, &dg4c).unwrap();
        assert_mappings(
            vf10.iter(),
            &[
                "[0=0 1=1 2=2 3=3 4=~~]",
                "[0=0 1=1 2=2 3=~~ 4=3]",
                "[0=0 1=1 2=~~ 3=2 4=3]",
                "[0=0 1=~~ 2=1 3=2 4=3]",
                "[0=~~ 1=0 2=1 3=2 4=3]",
            ],
        );
        // ECD-11
        let vf11 = VF2SubgraphIsomorphismInspector::new(&dg4v3e, &dg4v3e).unwrap();
        assert_mappings(vf11.iter(), &["[1=1 2=2 3=3 4=4]"]);
        // ECD-12
        let dg6v4enc = dg(&[0, 1, 2, 3, 4, 5], &[(1, 2), (2, 3), (3, 1), (4, 5)]);
        let dg5v4enc = dg(&[6, 7, 8, 9, 10], &[(7, 6), (9, 8), (10, 9), (8, 10)]);
        let vf12 = VF2SubgraphIsomorphismInspector::new(&dg6v4enc, &dg5v4enc).unwrap();
        assert_mappings(
            vf12.iter(),
            &[
                "[0=~~ 1=8 2=10 3=9 4=7 5=6]",
                "[0=~~ 1=9 2=8 3=10 4=7 5=6]",
                "[0=~~ 1=10 2=9 3=8 4=7 5=6]",
            ],
        );
    }

    #[test]
    fn exhaustive() {
        // DET-1
        let g1 = sg(&[0, 1, 2, 3, 4], &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        let g2 = sg(&[0, 1, 2], &[(0, 1), (1, 2)]);
        let vf2 = VF2SubgraphIsomorphismInspector::new(&g1, &g2).unwrap();
        assert!(contains_all_matchings(&vf2, &g1, &g2));
        // DET-2
        let g3 = dg(
            &[0, 1, 2, 3, 4, 5],
            &[
                (0, 1),
                (0, 5),
                (1, 4),
                (2, 1),
                (2, 4),
                (3, 1),
                (4, 0),
                (5, 2),
                (5, 4),
            ],
        );
        let g4 = dg(&[0, 1, 2, 3], &[(0, 3), (1, 2), (1, 3), (2, 3), (2, 0)]);
        let vf3 = VF2SubgraphIsomorphismInspector::new(&g3, &g4).unwrap();
        assert!(contains_all_matchings(&vf3, &g3, &g4));
        // DET-3
        let g5 = sg(&[0, 1, 2, 3], &[(0, 1), (1, 2), (2, 3)]);
        let g6 = sg(&[0, 1, 2], &[(0, 1), (1, 2), (2, 0)]);
        let vf4 = VF2SubgraphIsomorphismInspector::new(&g5, &g6).unwrap();
        assert!(contains_all_matchings(&vf4, &g5, &g6));
    }

    #[test]
    fn random_graphs() {
        let mut rnd = JavaRandom::new(54321);
        for i in 1..50 {
            let vertex_count = 2 + rnd.next_int_bound(i);
            let edge_count =
                vertex_count + rnd.next_int_bound(vertex_count * (vertex_count - 1)) / 2;
            let sub_vertex_count = 1 + rnd.next_int_bound(vertex_count);
            let g1 = random_graph(vertex_count, edge_count as usize, i as i64);
            let g2 = random_subgraph(&g1, sub_vertex_count as usize, i as i64);
            let vf2 = VF2SubgraphIsomorphismInspector::new(&g1, &g2).unwrap();
            assert!(all_matchings_correct(&vf2, &g1, &g2));
        }
    }

    #[test]
    fn random_graphs_exhaustive() {
        let mut rnd = JavaRandom::new(12345);
        for i in 1..100 {
            let vertex_count = 3 + rnd.next_int_bound(5);
            let edge_count = rnd.next_int_bound(vertex_count * (vertex_count - 1));
            let sub_vertex_count = 2 + rnd.next_int_bound(vertex_count);
            let sub_edge_count = rnd.next_int_bound(sub_vertex_count * (sub_vertex_count - 1));
            let g1 = random_graph(vertex_count, edge_count as usize, i as i64);
            let g2 = random_graph(sub_vertex_count, sub_edge_count as usize, i as i64);
            let vf2 = VF2SubgraphIsomorphismInspector::new(&g1, &g2).unwrap();
            assert!(contains_all_matchings(&vf2, &g1, &g2), "round {i}");
        }
    }

    #[test]
    fn semantic_check() {
        type G = BaseGraph<String, i32, Undirected>;
        let make = |vertices: &[&str], edges: &[(&str, &str, i32)]| {
            let mut g = G::without_edge_factory(GraphType::simple());
            for v in vertices {
                g.add_vertex(v.to_string());
            }
            for (a, b, e) in edges {
                g.add_edge_with(&a.to_string(), &b.to_string(), *e).unwrap();
            }
            g
        };
        let g1 = make(
            &["a", "b", "A", "B"],
            &[("a", "b", 3), ("b", "B", 1), ("B", "A", 2), ("A", "a", 4)],
        );
        let g2 = make(&["A", "b", "B"], &[("A", "b", 6), ("b", "B", 5)]);
        let vertex_comp = || -> VertexComparator<'static, G> {
            Box::new(|o1: &String, o2: &String| {
                if o1.to_lowercase() == o2.to_lowercase() {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            })
        };
        let edge_comp = || -> EdgeComparator<'static, G> {
            Box::new(|o1: &i32, o2: &i32| (o1 % 2).cmp(&(o2 % 2)))
        };
        let always_equal_edges =
            || -> EdgeComparator<'static, G> { Box::new(|_, _| Ordering::Equal) };
        let always_equal_vertices =
            || -> VertexComparator<'static, G> { Box::new(|_, _| Ordering::Equal) };
        // SEM-1
        let vf2 = VF2SubgraphIsomorphismInspector::with_options(
            &g1,
            &g2,
            Some(vertex_comp()),
            Some(edge_comp()),
            true,
        )
        .unwrap();
        assert_mappings(vf2.iter(), &["[A=A B=b a=~~ b=B]"]);
        // SEM-2
        let vf3 = VF2SubgraphIsomorphismInspector::with_options(
            &g1,
            &g2,
            Some(vertex_comp()),
            Some(always_equal_edges()),
            true,
        )
        .unwrap();
        assert_mappings(vf3.iter(), &["[A=A B=b a=~~ b=B]", "[A=~~ B=B a=A b=b]"]);
        // SEM-3
        let vf4 = VF2SubgraphIsomorphismInspector::with_options(
            &g1,
            &g2,
            Some(always_equal_vertices()),
            Some(edge_comp()),
            true,
        )
        .unwrap();
        assert_mappings(vf4.iter(), &["[A=A B=b a=~~ b=B]", "[A=A B=~~ a=b b=B]"]);
    }

    #[test]
    fn huge_graph() {
        let n = 700;
        let g1 = random_graph(n, (n * n / 50) as usize, 12345);
        let g2 = random_subgraph(&g1, (n / 2) as usize, 54321);
        let vf2 = VF2SubgraphIsomorphismInspector::new(&g1, &g2).unwrap();
        assert!(vf2.isomorphism_exists());
    }
}
