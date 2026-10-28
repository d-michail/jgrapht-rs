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

//! Ports of `graph/DirectedGraphTest`, `graph/WeightedGraphTest`, `graph/IncomingOutgoingEdgesTest`
//! and `graph/GenericGraphsTest`.

use jgrapht::graph::GraphExt;
use jgrapht::*;
use std::collections::HashSet;

fn set<T: Eq + std::hash::Hash>(it: impl IntoIterator<Item = T>) -> HashSet<T> {
    it.into_iter().collect()
}

// ---------------------------------------------------------------- DirectedGraphTest

mod directed_graph_test {
    use super::*;
    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";

    fn multi_triangle() -> DiGraph<&'static str> {
        let mut g = DiGraph::<&str>::multi();
        g.add_vertices([V1, V2, V3]);
        g.add_edge(&V1, &V2).unwrap();
        g.add_edge(&V2, &V1).unwrap();
        g.add_edge(&V2, &V3).unwrap();
        g.add_edge(&V3, &V1).unwrap();
        g
    }

    #[test]
    fn test_edge_set_factory() {
        let mut g = DiGraph::<&str>::multi();
        g.add_vertices([V1, V2, V3]);
        let e1 = g.add_edge(&V1, &V2).unwrap().unwrap();
        let e2 = g.add_edge(&V2, &V1).unwrap().unwrap();
        let e3 = g.add_edge(&V2, &V3).unwrap().unwrap();
        let e4 = g.add_edge(&V3, &V1).unwrap().unwrap();
        assert_eq!(g.edges().copied().collect::<Vec<_>>(), [e1, e2, e3, e4]);
        assert_eq!(
            g.to_string(),
            "([v1, v2, v3], [(v1,v2), (v2,v1), (v2,v3), (v3,v1)])"
        );
    }

    #[test]
    fn test_edge_order_determinism() {
        let mut g = DiGraph::<&str>::multi();
        g.add_vertices([V1, V2, V3]);
        let e1 = g.add_edge(&V1, &V2).unwrap().unwrap();
        let e2 = g.add_edge(&V2, &V3).unwrap().unwrap();
        let e3 = g.add_edge(&V3, &V1).unwrap().unwrap();
        assert_eq!(g.edges().copied().collect::<Vec<_>>(), [e1, e2, e3]);
        assert!(g.test_incidence(&e1, &V1));
        assert!(g.test_incidence(&e1, &V2));
        assert!(!g.test_incidence(&e1, &V3));
        assert_eq!(g.opposite_vertex(&e1, &V1), Some(&V2));
        assert_eq!(g.opposite_vertex(&e1, &V2), Some(&V1));
        assert_eq!(g.to_string(), "([v1, v2, v3], [(v1,v2), (v2,v3), (v3,v1)])");
    }

    #[test]
    fn test_edges_of() {
        let g = multi_triangle();
        assert_eq!(g.edges_of(&V1).count(), 3);
        assert_eq!(g.edges_of(&V2).count(), 3);
        assert_eq!(g.edges_of(&V3).count(), 2);
    }

    #[test]
    fn test_in_degree_of() {
        let g = multi_triangle();
        assert_eq!(
            (
                g.in_degree_of(&V1),
                g.in_degree_of(&V2),
                g.in_degree_of(&V3)
            ),
            (2, 1, 1)
        );
    }

    #[test]
    fn test_out_degree_of() {
        let g = multi_triangle();
        assert_eq!(
            (
                g.out_degree_of(&V1),
                g.out_degree_of(&V2),
                g.out_degree_of(&V3)
            ),
            (1, 2, 1)
        );
    }

    #[test]
    fn test_vertex_order_determinism() {
        let g = multi_triangle();
        assert_eq!(g.vertices().copied().collect::<Vec<_>>(), [V1, V2, V3]);
    }
}

// ---------------------------------------------------------------- WeightedGraphTest

mod weighted_graph_test {
    use super::*;

    #[test]
    fn test_default_weighted_edge() {
        let mut g = UGraph::<i32>::pseudo().weighted();
        g.add_vertices([1, 2]);
        let e = g.add_edge(&1, &2).unwrap().unwrap();
        assert!((g.edge_weight(&e).unwrap() - 1.0).abs() < 1e-9);
        g.set_edge_weight(&e, 3.0).unwrap();
        assert!((g.edge_weight(&e).unwrap() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_string_as_weighted_edge() {
        let mut g = BaseGraph::<i32, String, graph::Undirected>::without_edge_factory(
            GraphType::pseudograph(),
        )
        .weighted();
        g.add_vertices([1, 2]);
        assert_eq!(g.add_edge_with(&1, &2, "1-2".to_string()), Ok(true));
        let e = "1-2".to_string();
        assert!((g.edge_weight(&e).unwrap() - 1.0).abs() < 1e-9);
        g.set_edge_weight(&e, 3.0).unwrap();
        assert!((g.edge_weight(&e).unwrap() - 3.0).abs() < 1e-9);
        assert!(g.contains_edge(&e));
        g.remove_edge(&e);
        assert!(!g.contains_edge(&e));
    }

    #[test]
    fn test_invalid_edge_on_weighted_graph() {
        // JGraphT throws IllegalArgumentException; here the edge is simply unknown.
        let g = BaseGraph::<i32, String, graph::Undirected>::without_edge_factory(
            GraphType::pseudograph(),
        )
        .weighted();
        assert_eq!(g.edge_weight(&"1-2".to_string()), None);
    }

    #[test]
    fn test_invalid_edge_on_weighted_graph_set() {
        let mut g = BaseGraph::<i32, String, graph::Undirected>::without_edge_factory(
            GraphType::pseudograph(),
        )
        .weighted();
        assert_eq!(
            g.set_edge_weight(&"1-2".to_string(), 2.0),
            Err(GraphError::EdgeNotFound)
        );
    }

    #[test]
    fn test_default_edge_on_weighted_graphs() {
        let mut g = UGraph::<i32>::pseudo().weighted();
        g.add_vertices([1, 2]);
        let e = g.add_edge(&1, &2).unwrap().unwrap();
        assert!((g.edge_weight(&e).unwrap() - 1.0).abs() < 1e-9);
        g.set_edge_weight(&e, 3.0).unwrap();
        assert!((g.edge_weight(&e).unwrap() - 3.0).abs() < 1e-9);
        assert_eq!(g.edge_source(&e), Some(&1));
        assert_eq!(g.edge_target(&e), Some(&2));
    }

    #[test]
    fn test_clone_with_custom_edge_type() {
        let mut graph1 = BaseGraph::<i32, String, graph::Directed>::without_edge_factory(
            GraphType::multigraph(),
        )
        .weighted();
        graph1.add_vertices([1, 2, 3]);
        let edge1 = "Edge1".to_string();
        graph1.add_edge_with(&1, &2, edge1.clone()).unwrap();
        let graph2 = graph1.clone();
        assert_eq!(graph1, graph2);
        graph1.set_edge_weight(&edge1, 15.0).unwrap();
        let graph3 = graph1.clone();
        assert_eq!(graph1, graph3);
        assert_ne!(graph2, graph3);
    }
}

// ---------------------------------------------------------------- IncomingOutgoingEdgesTest

mod incoming_outgoing_edges_test {
    use super::*;
    use jgrapht::graph::Direction;

    fn all_types() -> [GraphType; 4] {
        [
            GraphType::pseudograph(),
            GraphType::multigraph(),
            GraphType {
                self_loops: true,
                ..GraphType::simple()
            },
            GraphType::simple(),
        ]
    }

    fn test_add_duplicate_edge<D: Direction>(mut g: BaseGraph<i32, DefaultEdge, D>) {
        g.add_vertices([1, 2, 3]);
        let e = DefaultEdge(1000);
        assert_eq!(g.add_edge_with(&1, &2, e), Ok(true));
        assert_eq!(g.edge_count(), 1);
        let edges = |it: Vec<&DefaultEdge>| it.into_iter().copied().collect::<HashSet<_>>();
        let check = |g: &BaseGraph<i32, DefaultEdge, D>| {
            if D::DIRECTED {
                assert!(edges(g.in_edges_of(&1).collect()).is_empty());
                assert_eq!(g.in_degree_of(&1), 0);
                assert_eq!(edges(g.out_edges_of(&1).collect()), set([e]));
                assert_eq!(g.out_degree_of(&1), 1);
                assert_eq!(edges(g.in_edges_of(&2).collect()), set([e]));
                assert_eq!(g.in_degree_of(&2), 1);
                assert!(edges(g.out_edges_of(&2).collect()).is_empty());
                assert_eq!(g.out_degree_of(&2), 0);
                assert!(edges(g.in_edges_of(&3).collect()).is_empty());
                assert_eq!((g.in_degree_of(&3), g.out_degree_of(&3)), (0, 0));
            } else {
                assert_eq!(edges(g.edges_of(&1).collect()), set([e]));
                assert_eq!(g.degree_of(&1), 1);
                assert_eq!(edges(g.edges_of(&2).collect()), set([e]));
                assert_eq!(g.degree_of(&2), 1);
                assert!(edges(g.edges_of(&3).collect()).is_empty());
                assert_eq!(g.degree_of(&3), 0);
            }
        };
        check(&g);
        assert_eq!(g.add_edge_with(&1, &3, e), Err(GraphError::IntrusiveEdge));
        assert_eq!(g.edge_count(), 1);
        check(&g);
    }

    #[test]
    fn test_directed_graph() {
        let mut g = DiGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5]);
        assert_eq!(g.vertex_count(), 5);
        assert!((1..=5).all(|v| g.contains_vertex(&v)));
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let e12 = add(1, 2);
        let e23_1 = add(2, 3);
        let e23_2 = add(2, 3);
        let e24 = add(2, 4);
        let e44 = add(4, 4);
        let e55_1 = add(5, 5);
        let e52 = add(5, 2);
        let e55_2 = add(5, 5);
        let es = |it: Vec<&DefaultEdge>| it.into_iter().copied().collect::<HashSet<_>>();
        assert_eq!([1, 2, 3, 4, 5].map(|v| g.degree_of(&v)), [1, 5, 2, 3, 5]);
        assert_eq!(es(g.edges_of(&1).collect()), set([e12]));
        assert_eq!(
            es(g.edges_of(&2).collect()),
            set([e12, e23_1, e23_2, e24, e52])
        );
        assert_eq!(es(g.edges_of(&3).collect()), set([e23_1, e23_2]));
        assert_eq!(es(g.edges_of(&4).collect()), set([e24, e44]));
        assert_eq!(es(g.edges_of(&5).collect()), set([e52, e55_1, e55_2]));
        assert_eq!([1, 2, 3, 4, 5].map(|v| g.in_degree_of(&v)), [0, 2, 2, 2, 2]);
        assert!(es(g.in_edges_of(&1).collect()).is_empty());
        assert_eq!(es(g.in_edges_of(&2).collect()), set([e12, e52]));
        assert_eq!(es(g.in_edges_of(&3).collect()), set([e23_1, e23_2]));
        assert_eq!(es(g.in_edges_of(&4).collect()), set([e24, e44]));
        assert_eq!(es(g.in_edges_of(&5).collect()), set([e55_1, e55_2]));
        assert_eq!(
            [1, 2, 3, 4, 5].map(|v| g.out_degree_of(&v)),
            [1, 3, 0, 1, 3]
        );
        assert_eq!(es(g.out_edges_of(&1).collect()), set([e12]));
        assert_eq!(es(g.out_edges_of(&2).collect()), set([e23_1, e23_2, e24]));
        assert!(es(g.out_edges_of(&3).collect()).is_empty());
        assert_eq!(es(g.out_edges_of(&4).collect()), set([e44]));
        assert_eq!(es(g.out_edges_of(&5).collect()), set([e52, e55_1, e55_2]));
        for ty in all_types() {
            test_add_duplicate_edge(DiGraph::<i32>::with_graph_type(ty));
        }
    }

    #[test]
    fn test_undirected_graph() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices([1, 2, 3, 4, 5]);
        assert_eq!(g.vertex_count(), 5);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let e12 = add(1, 2);
        let e23_1 = add(2, 3);
        let e23_2 = add(2, 3);
        let e24 = add(2, 4);
        let e44 = add(4, 4);
        let e55_1 = add(5, 5);
        let e52 = add(5, 2);
        let e55_2 = add(5, 5);
        let es = |it: Vec<&DefaultEdge>| it.into_iter().copied().collect::<HashSet<_>>();
        assert_eq!([1, 2, 3, 4, 5].map(|v| g.degree_of(&v)), [1, 5, 2, 3, 5]);
        let expected = [
            set([e12]),
            set([e12, e23_1, e23_2, e24, e52]),
            set([e23_1, e23_2]),
            set([e24, e44]),
            set([e52, e55_1, e55_2]),
        ];
        for (i, want) in expected.iter().enumerate() {
            let v = i as i32 + 1;
            assert_eq!(&es(g.edges_of(&v).collect()), want);
            assert_eq!(&es(g.in_edges_of(&v).collect()), want);
            assert_eq!(&es(g.out_edges_of(&v).collect()), want);
            assert_eq!(g.in_degree_of(&v), g.degree_of(&v));
            assert_eq!(g.out_degree_of(&v), g.degree_of(&v));
        }
        for ty in all_types() {
            test_add_duplicate_edge(UGraph::<i32>::with_graph_type(ty));
        }
    }
}

// ---------------------------------------------------------------- GenericGraphsTest
// The Foo/Bar vertex- and edge-subclass variance tests exercise Java generics only and have no
// Rust counterpart; the remaining ones are ported.

mod generic_graphs_test {
    use super::*;

    #[test]
    fn test_legal_insert_string_graph() {
        let mut object_graph = DiGraph::<String>::with_loops();
        let (v1, v2) = ("Vertex1".to_string(), "Vertex2".to_string());
        object_graph.add_vertex(v1.clone());
        object_graph.add_vertex(v2.clone());
        object_graph.add_edge(&v1, &v2).unwrap();
    }

    #[derive(Clone, Debug)]
    struct EquivVertex;
    impl PartialEq for EquivVertex {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }
    impl Eq for EquivVertex {}
    impl std::hash::Hash for EquivVertex {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            1.hash(state)
        }
    }

    #[test]
    fn test_equal_but_not_same_vertex() {
        let (v1, v2) = (EquivVertex, EquivVertex);
        let mut g = BaseGraph::<EquivVertex, DefaultEdge, graph::Directed>::pseudo();
        g.add_vertex(v1.clone());
        g.add_vertex(v2.clone());
        g.add_edge_with(&v1, &v2, DefaultEdge(0)).unwrap();
        assert_eq!(g.degree_of(&v1), 2);
        assert_eq!(g.degree_of(&v2), 2);
    }
}

// ---------------------------------------------------------------- EqualsAndHashCodeTest
// (`testHashcodeEquals`, `testUndirectedEquality`, ... included)

mod equals_and_hash_code_test {
    use super::*;
    use jgrapht::graph::{Directed, Undirected};
    use jgrapht::{graph_equals, graph_hash};
    use std::hash::{Hash, Hasher};

    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";
    const V4: &str = "v4";

    fn hash_of<T: Hash>(t: &T) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }

    fn triangle_with_isolated<D: jgrapht::graph::Direction>(
        order: [&'static str; 4],
        g: &mut BaseGraph<&'static str, DefaultEdge, D>,
        edges: &[(DefaultEdge, &'static str, &'static str)],
    ) {
        for v in order {
            g.add_vertex(v);
        }
        for (e, s, t) in edges {
            g.add_edge_with(s, t, *e).unwrap();
        }
    }

    fn check_default_graph<D: jgrapht::graph::Direction>(
        new: impl Fn() -> BaseGraph<&'static str, DefaultEdge, D>,
    ) {
        let (e12, e23, e31) = (DefaultEdge::new(), DefaultEdge::new(), DefaultEdge::new());
        let mut g1 = new();
        triangle_with_isolated(
            [V1, V2, V3, V4],
            &mut g1,
            &[(e12, V1, V2), (e23, V2, V3), (e31, V3, V1)],
        );
        let mut g2 = new();
        triangle_with_isolated(
            [V4, V3, V2, V1],
            &mut g2,
            &[(e31, V3, V1), (e23, V2, V3), (e12, V1, V2)],
        );
        let mut g3 = new();
        triangle_with_isolated([V4, V3, V2, V1], &mut g3, &[(e31, V3, V1), (e23, V2, V3)]);
        assert!(g2 == g1);
        assert!(g3 != g2);
        assert_eq!(hash_of(&g2), hash_of(&g1));
    }

    #[test]
    fn test_default_directed_graph() {
        check_default_graph(DiGraph::<&str>::with_loops);
    }

    #[test]
    fn test_simple_graph() {
        check_default_graph(UGraph::<&str>::simple);
    }

    fn string_graph() -> BaseGraph<&'static str, String, Directed> {
        BaseGraph::without_edge_factory(GraphType {
            self_loops: true,
            ..GraphType::simple()
        })
    }

    #[test]
    fn test_graphs_with_non_intrusive_edge() {
        let mut g1 = string_graph();
        g1.add_vertices([V1, V2, V3]);
        g1.add_edge_with(&V1, &V2, "v1v2".into()).unwrap();
        g1.add_edge_with(&V3, &V1, "v3v1".into()).unwrap();
        let mut g2 = string_graph();
        g2.add_vertices([V3, V2, V1]);
        g2.add_edge_with(&V3, &V1, "v3v1".into()).unwrap();
        g2.add_edge_with(&V1, &V2, "v1v2".into()).unwrap();
        let mut g3 = string_graph();
        g3.add_vertices([V3, V2, V1]);
        g3.add_edge_with(&V3, &V1, "v3v1".into()).unwrap();
        g3.add_edge_with(&V1, &V2, "v1v2".into()).unwrap();
        g3.add_edge_with(&V2, &V3, "v2v3".into()).unwrap();
        assert!(g1 == g2);
        assert!(g2 != g3);
        assert_eq!(hash_of(&g2), hash_of(&g1));
    }

    #[test]
    fn test_pseudograph() {
        let (e121, e23, e31, e122, e11) = (
            DefaultEdge::new(),
            DefaultEdge::new(),
            DefaultEdge::new(),
            DefaultEdge::new(),
            DefaultEdge::new(),
        );
        let mut g1 = UGraph::<&str>::pseudo();
        g1.add_vertices([V1, V2, V3]);
        for (e, s, t) in [
            (e121, V1, V2),
            (e23, V2, V3),
            (e31, V3, V1),
            (e122, V1, V2),
            (e11, V1, V1),
        ] {
            g1.add_edge_with(&s, &t, e).unwrap();
        }
        let mut g2 = UGraph::<&str>::pseudo();
        g2.add_vertices([V3, V2, V1]);
        for (e, s, t) in [
            (e11, V1, V1),
            (e121, V1, V2),
            (e31, V3, V1),
            (e23, V2, V3),
            (e122, V1, V2),
        ] {
            g2.add_edge_with(&s, &t, e).unwrap();
        }
        let mut g3 = UGraph::<&str>::pseudo();
        g3.add_vertices([V3, V2, V1]);
        for (e, s, t) in [(e11, V1, V1), (e121, V1, V2), (e31, V3, V1), (e23, V2, V3)] {
            g3.add_edge_with(&s, &t, e).unwrap();
        }
        assert!(g1 == g2);
        assert!(g2 != g3);
        assert_eq!(hash_of(&g2), hash_of(&g1));
    }

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct CustomEdge(String);

    #[test]
    fn test_graphs_with_custom_edges() {
        let make = |a: &str, b: &str| {
            let mut g = BaseGraph::<&str, CustomEdge, Undirected>::without_edge_factory(
                GraphType::simple(),
            );
            g.add_vertices([V1, V2, V3]);
            g.add_edge_with(&V1, &V2, CustomEdge(a.into())).unwrap();
            g.add_edge_with(&V3, &V1, CustomEdge(b.into())).unwrap();
            g
        };
        let (g1, g2, g3) = (
            make("v1-v2", "v3-v1"),
            make("v1-v2", "v3-v1"),
            make("v1::v2", "v3-v1"),
        );
        assert!(g1 == g2);
        assert!(g2 != g3);
        assert_eq!(hash_of(&g2), hash_of(&g1));
    }

    #[test]
    fn test_as_weighted_graphs() {
        let (e12, e23, e31) = (DefaultEdge::new(), DefaultEdge::new(), DefaultEdge::new());
        let build = |order: [(DefaultEdge, &'static str, &'static str); 3]| {
            let mut g = UGraph::<&str>::simple();
            g.add_vertices([V1, V2, V3]);
            for (e, s, t) in order {
                g.add_edge_with(&s, &t, e).unwrap();
            }
            g
        };
        let g1 = build([(e12, V1, V2), (e23, V2, V3), (e31, V3, V1)]);
        let g2 = build([(e12, V1, V2), (e23, V2, V3), (e31, V3, V1)]);
        let weights = |a: f64, b: f64, c: f64| {
            std::collections::HashMap::from([(e12, a), (e23, b), (e31, c)])
        };
        let g3 = AsWeighted::new(&g1, weights(10.0, 20.0, 30.0));
        let g4 = AsWeighted::new(&g2, weights(10.0, 20.0, 30.0));
        let g5 = AsWeighted::new(&g2, weights(100.0, 200.0, 300.0));
        assert!(g1 == g2);
        assert_eq!(graph_hash(&g2), graph_hash(&g1));
        assert!(graph_equals(&g3, &g4));
        assert_eq!(graph_hash(&g4), graph_hash(&g3));
        assert!(!graph_equals(&g4, &g5));
    }

    fn int_edge_graph<D: jgrapht::graph::Direction>(weighted: bool) -> BaseGraph<i32, i32, D> {
        let g = BaseGraph::<i32, i32, D>::without_edge_factory(GraphType::simple());
        if weighted { g.weighted() } else { g }
    }

    #[test]
    fn test_hashcode_equals() {
        let make = |w: f64| {
            let mut g = int_edge_graph::<Directed>(true);
            g.add_vertices([0, 1]);
            g.add_edge_with(&0, &1, 1).unwrap();
            g.set_edge_weight(&1, w).unwrap();
            g
        };
        assert_ne!(make(2.0 + 1e-8), make(2.0 - 1e-8));
    }

    #[test]
    fn test_undirected_equality() {
        let make = |a: i32, b: i32| {
            let mut g = int_edge_graph::<Undirected>(false);
            g.add_vertices([0, 1]);
            g.add_edge_with(&a, &b, 1).unwrap();
            g
        };
        let (g, h) = (make(0, 1), make(1, 0));
        assert_eq!(graph_hash(&g), graph_hash(&h));
        assert_eq!(g, h);
    }

    #[test]
    fn test_directed_equality() {
        let make = |a: i32, b: i32| {
            let mut g = int_edge_graph::<Directed>(false);
            g.add_vertices([0, 1]);
            g.add_edge_with(&a, &b, 1).unwrap();
            g
        };
        let (g, h) = (make(0, 1), make(1, 0));
        assert_ne!(graph_hash(&g), graph_hash(&h));
        assert_ne!(g, h);
    }
}

// ---------------------------------------------------------------- CloneTest / AsUnmodifiableGraphTest
// `ParanoidGraph` (equal-but-not-identical vertices), `null` arguments and the
// `UnsupportedOperationException` checks of `AsUnmodifiableGraph` are enforced by the Rust type
// system (`Unmodifiable` simply does not implement `GraphMut`).

mod clone_and_unmodifiable_test {
    use super::*;

    #[test]
    fn test_clone_specifics_bug() {
        let mut g1 = UGraph::<&str>::simple();
        g1.add_vertices(["1", "2", "3"]);
        g1.add_edge(&"1", &"2").unwrap();
        g1.add_edge(&"2", &"3").unwrap();
        let mut g2 = g1.clone();
        assert_eq!(g2.edge_count(), 2);
        let e = *g2.get_edge(&"1", &"2").unwrap();
        assert!(g2.remove_edge(&e));
        assert!(g2.remove_edge_between(&"2", &"3").is_some());
        assert_eq!(g2.edge_count(), 0);
        assert_eq!(g1.edge_count(), 2); // the original is untouched
    }

    fn unmodifiable_fixture() -> (UGraph<&'static str>, DefaultEdge) {
        let mut base = UGraph::<&str>::with_loops().weighted();
        base.add_vertices(["v1", "v2", "v3", "v4"]);
        let e12 = base.add_weighted_edge(&"v1", &"v2", 6.0).unwrap().unwrap();
        base.add_weighted_edge(&"v2", &"v3", 456.0).unwrap();
        base.add_weighted_edge(&"v2", &"v4", 0.587).unwrap();
        base.add_weighted_edge(&"v4", &"v4", 6781234453486.0)
            .unwrap();
        (base, e12)
    }

    #[test]
    fn test_unmodifiable_reads_through() {
        let (base, e12) = unmodifiable_fixture();
        let view = Unmodifiable(&base);
        assert!(!view.graph_type().modifiable);
        assert_eq!(view.vertex_count(), 4);
        assert_eq!(view.edge_count(), 4);
        assert_eq!(view.edge_weight(&e12), Some(6.0));
        assert_eq!(base.edge_weight(&e12), Some(6.0));
    }
}
