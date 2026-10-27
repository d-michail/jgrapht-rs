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

//! Ports of the view tests in `org.jgrapht.graph`: `AsUndirectedGraphTest`,
//! `AsUnweightedGraphTest`, `UnweightedGraphAsWeightedGraphTest`,
//! `WeightedGraphAsWeightedGraphTest`, `AsSubgraphTest`, `MaskSubgraphTest`, `AsGraphUnionTest`.
//!
//! Notes: views that must observe later changes of their base in Java cannot do so here because a
//! view borrows its base; tests that mutate the base mid-test are reshaped (mutate, then view).
//! `null`-argument checks and the `MaskVertexSet`/`MaskEdgeSet` collection classes have no Rust
//! counterpart (the masks are iterator filters).

use jgrapht::*;
use std::collections::{HashMap, HashSet};

fn set<T: Eq + std::hash::Hash>(it: impl IntoIterator<Item = T>) -> HashSet<T> {
    it.into_iter().collect()
}

// ---------------------------------------------------------------- AsUndirectedGraphTest

mod as_undirected_graph_test {
    use super::*;
    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";
    const V4: &str = "v4";

    struct F {
        directed: DiGraph<&'static str>,
        loop_: DefaultEdge,
        e12: DefaultEdge,
        e23: DefaultEdge,
        e24: DefaultEdge,
    }

    fn setup() -> F {
        let mut directed = DiGraph::<&str>::with_loops();
        directed.add_vertices([V1, V2, V3, V4]);
        let e12 = directed.add_edge(&V1, &V2).unwrap().unwrap();
        let e23 = directed.add_edge(&V2, &V3).unwrap().unwrap();
        let e24 = directed.add_edge(&V2, &V4).unwrap().unwrap();
        let loop_ = directed.add_edge(&V4, &V4).unwrap().unwrap();
        F {
            directed,
            loop_,
            e12,
            e23,
            e24,
        }
    }

    #[test]
    fn test_add_edge() {
        let mut f = setup();
        let mut undirected = AsUndirected(&mut f.directed);
        assert!(matches!(
            undirected.add_edge(&V3, &V4),
            Err(GraphError::Unsupported(_))
        ));
        assert_eq!(
            graph_to_string(&undirected),
            "([v1, v2, v3, v4], [{v1,v2}, {v2,v3}, {v2,v4}, {v4,v4}])"
        );
    }

    #[test]
    fn test_add_vertex() {
        let mut f = setup();
        {
            let mut undirected = AsUndirected(&mut f.directed);
            undirected.add_vertex("v5");
            assert!(undirected.contains_vertex(&"v5"));
        }
        assert!(f.directed.contains_vertex(&"v5"));
    }

    #[test]
    fn test_degree_and_edges_of() {
        let f = setup();
        let u = AsUndirected(&f.directed);
        assert_eq!([V1, V2, V3, V4].map(|v| u.degree_of(&v).unwrap()), [1, 3, 1, 3]);
        assert_eq!(set(u.edges_of(&V1).unwrap().copied()), set([f.e12]));
        assert_eq!(set(u.edges_of(&V2).unwrap().copied()), set([f.e12, f.e23, f.e24]));
        assert_eq!(set(u.edges_of(&V3).unwrap().copied()), set([f.e23]));
        assert_eq!(set(u.edges_of(&V4).unwrap().copied()), set([f.e24, f.loop_]));
    }

    #[test]
    fn test_in_out_degrees_and_edges() {
        let f = setup();
        let u = AsUndirected(&f.directed);
        for want in [[1, 3, 1, 3]] {
            assert_eq!([V1, V2, V3, V4].map(|v| u.in_degree_of(&v).unwrap()), want);
            assert_eq!([V1, V2, V3, V4].map(|v| u.out_degree_of(&v).unwrap()), want);
        }
        for v in [V1, V2, V3, V4] {
            assert_eq!(
                set(u.in_edges_of(&v).unwrap().copied()),
                set(u.edges_of(&v).unwrap().copied())
            );
            assert_eq!(
                set(u.out_edges_of(&v).unwrap().copied()),
                set(u.edges_of(&v).unwrap().copied())
            );
        }
    }

    #[test]
    fn test_get_all_edges() {
        let f = setup();
        let u = AsUndirected(&f.directed);
        let edges: Vec<_> = u.edges_between(&V3, &V2).copied().collect();
        assert_eq!(edges, vec![*f.directed.get_edge(&V2, &V3).unwrap()]);
        assert_eq!(
            u.edges_between(&V4, &V4).copied().collect::<Vec<_>>(),
            vec![f.loop_]
        );
    }

    #[test]
    fn test_get_edge() {
        let f = setup();
        let u = AsUndirected(&f.directed);
        assert_eq!(f.directed.get_edge(&V1, &V2), u.get_edge(&V1, &V2));
        assert_eq!(f.directed.get_edge(&V1, &V2), u.get_edge(&V2, &V1));
        assert_eq!(f.directed.get_edge(&V4, &V4), u.get_edge(&V4, &V4));
    }

    #[test]
    fn test_remove_edge_and_vertex() {
        let mut f = setup();
        {
            let mut u = AsUndirected(&mut f.directed);
            u.remove_edge(&f.loop_);
            assert!(!u.contains_edge(&f.loop_));
        }
        assert!(!f.directed.contains_edge(&f.loop_));
        {
            let mut u = AsUndirected(&mut f.directed);
            u.remove_vertex(&V4);
            assert!(!u.contains_vertex(&V4));
        }
        assert!(!f.directed.contains_vertex(&V4));
    }

    #[test]
    fn test_to_string() {
        let f = setup();
        assert_eq!(
            graph_to_string(&f.directed),
            "([v1, v2, v3, v4], [(v1,v2), (v2,v3), (v2,v4), (v4,v4)])"
        );
        assert_eq!(
            graph_to_string(&AsUndirected(&f.directed)),
            "([v1, v2, v3, v4], [{v1,v2}, {v2,v3}, {v2,v4}, {v4,v4}])"
        );
    }
}

// ---------------------------------------------------------------- AsUnweightedGraphTest

mod as_unweighted_graph_test {
    use super::*;

    fn setup() -> (UGraph<&'static str>, [DefaultEdge; 4]) {
        let mut g = UGraph::<&str>::with_loops().weighted();
        g.add_vertices(["v1", "v2", "v3", "v4"]);
        let e12 = g.add_weighted_edge(&"v1", &"v2", 6.0).unwrap().unwrap();
        let e23 = g.add_weighted_edge(&"v2", &"v3", 456.0).unwrap().unwrap();
        let e24 = g.add_weighted_edge(&"v2", &"v4", 0.587).unwrap().unwrap();
        let lp = g
            .add_weighted_edge(&"v4", &"v4", 6781234453486.0)
            .unwrap()
            .unwrap();
        (g, [e12, e23, e24, lp])
    }

    #[test]
    fn get_edge_weights_are_default() {
        let (mut g, edges) = setup();
        let u = AsUnweighted(&mut g);
        for e in edges {
            assert_eq!(u.edge_weight(&e), Some(1.0));
        }
    }

    #[test]
    fn set_edge_weight() {
        let (mut g, [_, e23, ..]) = setup();
        let mut u = AsUnweighted(&mut g);
        assert_eq!(
            u.set_edge_weight(&e23, 81.0),
            Err(GraphError::Unsupported(
                "Edge weight is not supported".into()
            ))
        );
    }

    #[test]
    fn get_type() {
        let (mut g, _) = setup();
        assert!(!AsUnweighted(&mut g).graph_type().weighted);
    }
}

// ---------------------------------------------------------------- *AsWeightedGraphTest

mod as_weighted_graph_tests {
    use super::*;

    const E12_WEIGHT: f64 = -123.54;
    const E23_WEIGHT: f64 = 89.0;
    const E24_WEIGHT: f64 = 3.0;

    fn four_vertex_graph<D: graph::Direction>(
        g: &mut BaseGraph<&'static str, DefaultEdge, D>,
        w: Option<[f64; 4]>,
    ) -> [DefaultEdge; 4] {
        g.add_vertices(["v1", "v2", "v3", "v4"]);
        let mut add = |a, b, i: usize| match w {
            Some(ws) => g.add_weighted_edge(&a, &b, ws[i]).unwrap().unwrap(),
            None => g.add_edge(&a, &b).unwrap().unwrap(),
        };
        [
            add("v1", "v2", 0),
            add("v2", "v3", 1),
            add("v2", "v4", 2),
            add("v4", "v4", 3),
        ]
    }

    fn weights(e: &[DefaultEdge; 4]) -> HashMap<DefaultEdge, f64> {
        HashMap::from([(e[0], E12_WEIGHT), (e[1], E23_WEIGHT), (e[2], E24_WEIGHT)])
    }

    // --- UnweightedGraphAsWeightedGraphTest

    #[test]
    fn unweighted_set_edge_weight() {
        let mut graph = UGraph::<&str>::with_loops();
        let e = four_vertex_graph(&mut graph, None);
        let mut wg = AsWeighted::new(&mut graph, weights(&e));
        wg.set_edge_weight(&e[0], -999.0).unwrap();
        assert_eq!(wg.edge_weight(&e[0]), Some(-999.0));
    }

    #[test]
    fn unweighted_get_edge_weight_and_default() {
        let mut graph = UGraph::<&str>::with_loops();
        let e = four_vertex_graph(&mut graph, None);
        let wg = AsWeighted::new(&graph, weights(&e));
        assert_eq!(wg.edge_weight(&e[1]), Some(E23_WEIGHT));
        assert_eq!(wg.edge_weight(&e[3]), Some(1.0)); // DEFAULT_EDGE_WEIGHT
        assert!(wg.graph_type().weighted);
    }

    #[test]
    fn create_as_weighted_with_weight_propagation_on_an_unweighted_graph() {
        let g = UGraph::<String>::with_loops();
        assert!(AsWeighted::with_write_through(g, HashMap::new(), true).is_err());
    }

    // --- WeightedGraphAsWeightedGraphTest

    const DEFAULTS: [f64; 4] = [6.0, 456.0, 0.587, 6781234453486.0];

    fn backing() -> (UGraph<&'static str>, [DefaultEdge; 4]) {
        let mut g = UGraph::<&str>::with_loops().weighted();
        let e = four_vertex_graph(&mut g, Some(DEFAULTS));
        (g, e)
    }

    #[test]
    fn weighted_set_edge_weight_without_propagation() {
        let (mut g, e) = backing();
        let mut wg = AsWeighted::with_write_through(&mut g, weights(&e), false).unwrap();
        wg.set_edge_weight(&e[0], -999.0).unwrap();
        assert_eq!(wg.edge_weight(&e[0]), Some(-999.0));
        assert_eq!(wg.base().edge_weight(&e[0]), Some(DEFAULTS[0]));
    }

    #[test]
    fn weighted_set_edge_weight_default_propagation() {
        let (mut g, e) = backing();
        let mut wg = AsWeighted::new(&mut g, weights(&e));
        wg.set_edge_weight(&e[0], -999.0).unwrap();
        assert_eq!(wg.edge_weight(&e[0]), Some(-999.0));
        assert_eq!(wg.base().edge_weight(&e[0]), Some(-999.0));
    }

    #[test]
    fn weighted_set_edge_propagates_changes_to_backing_graph() {
        let (mut g, e) = backing();
        let mut wg = AsWeighted::with_write_through(&mut g, weights(&e), true).unwrap();
        wg.set_edge_weight(&e[0], -999.0).unwrap();
        assert_eq!(wg.edge_weight(&e[0]), Some(-999.0));
        assert_eq!(wg.base().edge_weight(&e[0]), Some(-999.0));
    }

    #[test]
    fn weighted_get_edge_weight_and_default() {
        let (mut g, e) = backing();
        let wg = AsWeighted::with_write_through(&mut g, weights(&e), false).unwrap();
        assert_eq!(wg.edge_weight(&e[1]), Some(E23_WEIGHT));
        assert_eq!(wg.edge_weight(&e[3]), Some(DEFAULTS[3]));
    }

    #[test]
    fn test_weight_function() {
        let mut g1 = UGraph::<i32>::simple().weighted();
        g1.add_edge_with_vertices(0, 1).unwrap();
        g1.add_edge_with_vertices(1, 2).unwrap();
        g1.add_edge_with_vertices(2, 0).unwrap();
        let es: Vec<_> = g1.edges().copied().collect();
        for (e, w) in es.iter().zip([2.0, 3.0, 4.0]) {
            g1.set_edge_weight(e, w).unwrap();
        }
        let g2 =
            AsWeighted::from_function(&g1, |e| g1.edge_weight(e).unwrap().powi(2), true, false)
                .unwrap();
        for _ in 0..2 {
            for e in g1.edges() {
                assert_eq!(
                    g1.edge_weight(e).unwrap() * g1.edge_weight(e).unwrap(),
                    g2.edge_weight(e).unwrap()
                );
            }
        }
        // a function without caching cannot be overwritten
        let mut g3 = AsWeighted::from_function(&g1, |_| 1.0, false, false).unwrap();
        assert!(matches!(
            g3.set_overlay_weight(es[0], 5.0),
            Err(GraphError::Unsupported(_))
        ));
        let _ = &mut g3;
    }
}

// ---------------------------------------------------------------- AsSubgraphTest

mod as_subgraph_test {
    use super::*;

    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";
    const V4: &str = "v4";

    fn init() -> UGraph<&'static str> {
        let mut g = UGraph::<&str>::simple();
        g.add_vertices([V1, V2, V3, V4]);
        for (a, b) in [(V1, V2), (V2, V3), (V3, V1), (V1, V4)] {
            g.add_edge(&a, &b).unwrap();
        }
        g
    }

    #[test]
    fn test_induced_subgraph_everything() {
        let g = init();
        let sub = AsSubgraph::all(&g);
        assert_eq!(set(g.vertices().copied()), set(sub.vertices().copied()));
        assert_eq!(set(g.edges().copied()), set(sub.edges().copied()));
    }

    #[test]
    fn test_subgraph() {
        let g = init();
        let sub = AsSubgraph::new(&g, Some(vec![V1]), None);
        assert_eq!(set(sub.vertices().copied()), set([V1]));
        assert_eq!(sub.degree_of(&V1).unwrap(), 0);
        assert_eq!(sub.edge_count(), 0);
        let vset = vec![V1, V2, V3];
        let one_edge: Vec<_> = g.edges_between(&V1, &V2).copied().collect();
        let sub = AsSubgraph::new(&g, Some(vset.clone()), Some(one_edge));
        assert_eq!(set(sub.vertices().copied()), set(vset));
        assert_eq!(sub.edge_count(), 1);
    }

    #[test]
    fn test_induced_subgraph_underlying_edge_addition() {
        let mut base = UGraph::<&str>::simple();
        base.add_vertices([V1, V2]);
        let before = AsSubgraph::induced(&base, [V1]);
        assert_eq!(before.vertex_count(), 1);
        drop(before);
        base.add_edge(&V1, &V2).unwrap();
        let sub = AsSubgraph::induced(&base, [V1]);
        assert!(!sub.contains_edge_between(&V1, &V2));
    }

    fn pseudograph_10_edges() -> (UGraph<i32>, DefaultEdge) {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices(1..=5);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        add(1, 2);
        add(1, 2);
        add(1, 3);
        let e14 = add(1, 4);
        add(2, 3);
        add(2, 1);
        add(3, 3);
        add(4, 5);
        add(5, 5);
        add(5, 2);
        (g, e14)
    }

    #[test]
    fn test_edges() {
        let (g, e14) = pseudograph_10_edges();
        let mut sg = AsSubgraph::all(&g);
        assert_eq!(sg.edge_count(), 10);
        sg.remove_vertex(&2);
        assert_eq!(sg.edge_count(), 5);
        assert_eq!(sg.edges_of(&1).unwrap().count(), 2);
        assert!(!sg.contains_vertex(&2));
        assert_eq!(sg.edges_of(&3).unwrap().count(), 2);
        assert_eq!(sg.edges_of(&4).unwrap().count(), 2);
        assert_eq!(sg.edges_of(&5).unwrap().count(), 2);
        sg.remove_edge(&e14);
        assert_eq!(sg.edge_count(), 4);
        assert_eq!([1, 3, 4, 5].map(|v| sg.edges_of(&v).unwrap().count()), [1, 2, 1, 2]);
        drop(sg);
        assert_eq!(g.edge_count(), 10); // the base is untouched
    }

    #[test]
    fn test_non_valid_vertices_filter() {
        let (g, _) = pseudograph_10_edges();
        let sg = AsSubgraph::new(&g, Some(vec![1, 3, 100, 200, 300, 500, 800, 1000]), None);
        assert_eq!(sg.edge_count(), 2);
        assert_eq!(sg.vertex_count(), 2);
    }

    #[test]
    fn test_non_valid_edges_filter() {
        let mut g = UGraph::<i32>::pseudo();
        g.add_vertices(1..=5);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let e1 = add(1, 2);
        add(1, 2);
        let e2 = add(1, 3);
        let e3 = add(1, 4);
        add(2, 3);
        add(2, 1);
        add(3, 3);
        let e4 = add(4, 5);
        let e5 = add(5, 5);
        add(5, 2);
        let non_valid1 = add(5, 1);
        g.remove_edge(&non_valid1);
        let non_valid2 = g.add_edge(&5, &1).unwrap().unwrap();
        g.remove_edge(&non_valid2);
        let sg = AsSubgraph::new(
            &g,
            None,
            Some(vec![e1, e2, e3, e4, e5, non_valid1, non_valid2]),
        );
        assert_eq!(sg.edge_count(), 5);
        assert_eq!(sg.vertex_count(), 5);
    }

    fn in_out_edges<D: graph::Direction>(
        mut g: BaseGraph<i32, DefaultEdge, D>,
    ) -> (BaseGraph<i32, DefaultEdge, D>, [DefaultEdge; 6]) {
        g.add_vertices(1..=5);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let e12 = add(1, 2);
        let e13 = add(1, 3);
        let e23 = add(2, 3);
        let e24_1 = add(2, 4);
        let e24_2 = add(2, 4);
        add(2, 4);
        add(3, 5);
        let e44 = add(4, 4);
        add(4, 5);
        (g, [e12, e13, e23, e24_1, e24_2, e44])
    }

    #[test]
    fn test_in_out_edges_undirected() {
        let (g, [e12, e13, e23, e24_1, e24_2, e44]) = in_out_edges(UGraph::<i32>::pseudo());
        let sg = AsSubgraph::with_edges(&g, [1, 2, 3, 4], [e12, e13, e23, e24_1, e24_2, e44]);
        assert_eq!(sg.edge_count(), 6);
        let expected = [
            set([e12, e13]),
            set([e12, e24_1, e24_2, e23]),
            set([e13, e23]),
            set([e24_1, e24_2, e44]),
        ];
        for (i, want) in expected.iter().enumerate() {
            let v = i as i32 + 1;
            assert_eq!(&set(sg.edges_of(&v).unwrap().copied()), want);
            assert_eq!(&set(sg.in_edges_of(&v).unwrap().copied()), want);
            assert_eq!(&set(sg.out_edges_of(&v).unwrap().copied()), want);
        }
        assert_eq!([1, 2, 3, 4].map(|v| sg.degree_of(&v).unwrap()), [2, 4, 2, 4]);
        assert_eq!([1, 2, 3, 4].map(|v| sg.in_degree_of(&v).unwrap()), [2, 4, 2, 4]);
        assert_eq!([1, 2, 3, 4].map(|v| sg.out_degree_of(&v).unwrap()), [2, 4, 2, 4]);
    }

    #[test]
    fn test_in_out_edges_directed() {
        let (g, [e12, e13, e23, e24_1, e24_2, e44]) = in_out_edges(DiGraph::<i32>::pseudo());
        let sg = AsSubgraph::with_edges(&g, [1, 2, 3, 4], [e12, e13, e23, e24_1, e24_2, e44]);
        assert_eq!(sg.edge_count(), 6);
        assert_eq!(set(sg.edges_of(&1).unwrap().copied()), set([e12, e13]));
        assert_eq!(set(sg.edges_of(&2).unwrap().copied()), set([e12, e24_1, e24_2, e23]));
        assert_eq!(set(sg.edges_of(&3).unwrap().copied()), set([e13, e23]));
        assert_eq!(set(sg.edges_of(&4).unwrap().copied()), set([e24_1, e24_2, e44]));
        assert_eq!([1, 2, 3, 4].map(|v| sg.degree_of(&v).unwrap()), [2, 4, 2, 4]);
        assert_eq!(sg.in_edges_of(&1).unwrap().count(), 0);
        assert_eq!(set(sg.in_edges_of(&2).unwrap().copied()), set([e12]));
        assert_eq!(set(sg.in_edges_of(&3).unwrap().copied()), set([e13, e23]));
        assert_eq!(set(sg.in_edges_of(&4).unwrap().copied()), set([e24_1, e24_2, e44]));
        assert_eq!([1, 2, 3, 4].map(|v| sg.in_degree_of(&v).unwrap()), [0, 1, 2, 3]);
        assert_eq!(set(sg.out_edges_of(&1).unwrap().copied()), set([e12, e13]));
        assert_eq!(set(sg.out_edges_of(&2).unwrap().copied()), set([e24_1, e24_2, e23]));
        assert_eq!(sg.out_edges_of(&3).unwrap().count(), 0);
        assert_eq!(set(sg.out_edges_of(&4).unwrap().copied()), set([e44]));
        assert_eq!([1, 2, 3, 4].map(|v| sg.out_degree_of(&v).unwrap()), [2, 3, 0, 1]);
    }
}

// ---------------------------------------------------------------- MaskSubgraphTest

mod mask_subgraph_test {
    use super::*;

    #[test]
    fn test_unmodifiable() {
        let g = UGraph::<i32>::pseudo();
        assert!(
            !MaskSubgraph::new(&g, |v: &i32| *v == 5, |_: &DefaultEdge| false)
                .graph_type()
                .modifiable
        );
    }

    fn fixture<D: graph::Direction>(
        mut g: BaseGraph<i32, DefaultEdge, D>,
    ) -> (BaseGraph<i32, DefaultEdge, D>, [DefaultEdge; 7]) {
        g.add_vertices(1..=5);
        let mut add = |a, b| g.add_edge(&a, &b).unwrap().unwrap();
        let e12 = add(1, 2);
        let e13 = add(1, 3);
        let e23 = add(2, 3);
        let e24_1 = add(2, 4);
        let e24_2 = add(2, 4);
        let e24_3 = add(2, 4);
        add(3, 5);
        let e44 = add(4, 4);
        add(4, 5);
        (g, [e12, e13, e23, e24_1, e24_2, e24_3, e44])
    }

    #[test]
    fn test_in_out_edges_undirected() {
        let (g, [e12, e13, e23, e24_1, e24_2, e24_3, e44]) = fixture(UGraph::<i32>::pseudo());
        let sg = MaskSubgraph::new(&g, |v: &i32| *v == 5, |e: &DefaultEdge| *e == e24_3);
        assert_eq!(sg.edge_count(), 6);
        let expected = [
            set([e12, e13]),
            set([e12, e24_1, e24_2, e23]),
            set([e13, e23]),
            set([e24_1, e24_2, e44]),
        ];
        for (i, want) in expected.iter().enumerate() {
            let v = i as i32 + 1;
            assert_eq!(&set(sg.edges_of(&v).unwrap().copied()), want);
            assert_eq!(&set(sg.in_edges_of(&v).unwrap().copied()), want);
            assert_eq!(&set(sg.out_edges_of(&v).unwrap().copied()), want);
        }
        assert_eq!([1, 2, 3, 4].map(|v| sg.degree_of(&v).unwrap()), [2, 4, 2, 4]);
        assert_eq!([1, 2, 3, 4].map(|v| sg.in_degree_of(&v).unwrap()), [2, 4, 2, 4]);
        assert_eq!([1, 2, 3, 4].map(|v| sg.out_degree_of(&v).unwrap()), [2, 4, 2, 4]);
    }

    #[test]
    fn test_in_out_edges_directed() {
        let (g, [e12, e13, e23, e24_1, e24_2, e24_3, e44]) = fixture(DiGraph::<i32>::pseudo());
        let sg = MaskSubgraph::new(&g, |v: &i32| *v == 5, |e: &DefaultEdge| *e == e24_3);
        assert_eq!(sg.edge_count(), 6);
        assert_eq!(set(sg.edges_of(&1).unwrap().copied()), set([e12, e13]));
        assert_eq!(set(sg.edges_of(&2).unwrap().copied()), set([e12, e24_1, e24_2, e23]));
        assert_eq!(set(sg.edges_of(&3).unwrap().copied()), set([e13, e23]));
        assert_eq!(set(sg.edges_of(&4).unwrap().copied()), set([e24_1, e24_2, e44]));
        assert_eq!([1, 2, 3, 4].map(|v| sg.degree_of(&v).unwrap()), [2, 4, 2, 4]);
        assert_eq!(sg.in_edges_of(&1).unwrap().count(), 0);
        assert_eq!(set(sg.in_edges_of(&2).unwrap().copied()), set([e12]));
        assert_eq!(set(sg.in_edges_of(&3).unwrap().copied()), set([e13, e23]));
        assert_eq!(set(sg.in_edges_of(&4).unwrap().copied()), set([e24_1, e24_2, e44]));
        assert_eq!([1, 2, 3, 4].map(|v| sg.in_degree_of(&v).unwrap()), [0, 1, 2, 3]);
        assert_eq!(set(sg.out_edges_of(&1).unwrap().copied()), set([e12, e13]));
        assert_eq!(set(sg.out_edges_of(&2).unwrap().copied()), set([e24_1, e24_2, e23]));
        assert_eq!(sg.out_edges_of(&3).unwrap().count(), 0);
        assert_eq!(set(sg.out_edges_of(&4).unwrap().copied()), set([e44]));
        assert_eq!([1, 2, 3, 4].map(|v| sg.out_degree_of(&v).unwrap()), [2, 3, 0, 1]);
    }
}

// ---------------------------------------------------------------- AsGraphUnionTest

mod as_graph_union_test {
    use super::*;

    const V0: &str = "v0";
    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";
    const V4: &str = "v4";

    struct F {
        e: [DefaultEdge; 8],
        undirected1: UGraph<&'static str>,
        undirected2: UGraph<&'static str>,
        directed1: DiGraph<&'static str>,
        directed2: DiGraph<&'static str>,
    }

    fn setup() -> F {
        let e: [DefaultEdge; 8] = std::array::from_fn(|_| DefaultEdge::new());
        let [e1, e2, e3, e4, e5, e6, e7, e8] = e;
        let mut undirected1 = UGraph::<&str>::pseudo().weighted();
        let mut undirected2 = UGraph::<&str>::pseudo().weighted();
        let mut directed1 = DiGraph::<&str>::pseudo();
        let mut directed2 = DiGraph::<&str>::pseudo();
        undirected1.add_vertices([V0, V1, V4]);
        directed1.add_vertices([V0, V1, V4]);
        undirected2.add_vertices([V1, V2, V3, V4]);
        directed2.add_vertices([V1, V2, V3, V4]);
        for (s, t, ed) in [(V0, V1, e1), (V1, V4, e2), (V4, V0, e3), (V4, V4, e8)] {
            undirected1.add_edge_with(&s, &t, ed).unwrap();
            directed1.add_edge_with(&s, &t, ed).unwrap();
        }
        for (s, t, ed) in [(V4, V1, e7), (V1, V2, e4), (V2, V3, e5), (V3, V4, e6)] {
            undirected2.add_edge_with(&s, &t, ed).unwrap();
            directed2.add_edge_with(&s, &t, ed).unwrap();
        }
        F {
            e,
            undirected1,
            undirected2,
            directed1,
            directed2,
        }
    }

    fn check_vertices_and_edges<G: Graph<Vertex = &'static str, Edge = DefaultEdge>>(
        u: &G,
        e: &[DefaultEdge; 8],
    ) {
        assert_eq!(set(u.vertices().copied()), set([V0, V1, V2, V3, V4]));
        assert_eq!(set(u.edges().copied()), set(e.iter().copied()));
        let [e1, e2, e3, e4, e5, e6, e7, e8] = *e;
        assert_eq!(set(u.edges_of(&V0).unwrap().copied()), set([e1, e3]));
        assert_eq!(set(u.edges_of(&V1).unwrap().copied()), set([e1, e2, e4, e7]));
        assert_eq!(set(u.edges_of(&V2).unwrap().copied()), set([e4, e5]));
        assert_eq!(set(u.edges_of(&V3).unwrap().copied()), set([e5, e6]));
        assert_eq!(set(u.edges_of(&V4).unwrap().copied()), set([e2, e3, e6, e7, e8]));
        assert_eq!(
            [V0, V1, V2, V3, V4].map(|v| u.degree_of(&v).unwrap()),
            [2, 4, 2, 2, 6]
        );
    }

    #[test]
    fn test_undirected_graph_union() {
        let f = setup();
        let u = GraphUnion::new(&f.undirected1, &f.undirected2);
        let t = u.graph_type();
        assert!(t.is_undirected() && t.weighted && !t.modifiable);
        check_vertices_and_edges(&u, &f.e);
        let [e1, e2, e3, e4, e5, e6, e7, e8] = f.e;
        for v in [V0, V1, V2, V3, V4] {
            assert_eq!(
                set(u.in_edges_of(&v).unwrap().copied()),
                set(u.edges_of(&v).unwrap().copied())
            );
            assert_eq!(
                set(u.out_edges_of(&v).unwrap().copied()),
                set(u.edges_of(&v).unwrap().copied())
            );
            assert_eq!(u.in_degree_of(&v).unwrap(), u.degree_of(&v).unwrap());
            assert_eq!(u.out_degree_of(&v).unwrap(), u.degree_of(&v).unwrap());
        }
        let _ = (e1, e2, e3, e4, e5, e6, e7, e8);
        assert_eq!(u.get_edge(&V1, &V4), Some(&e2));
        assert_eq!(u.get_edge(&V4, &V1), Some(&e2));
    }

    #[test]
    fn test_directed_graph_union() {
        let f = setup();
        let u = GraphUnion::new(&f.directed1, &f.directed2);
        let t = u.graph_type();
        assert!(t.directed && t.weighted && !t.modifiable);
        check_vertices_and_edges(&u, &f.e);
        let [e1, e2, e3, e4, e5, e6, e7, e8] = f.e;
        assert_eq!(set(u.in_edges_of(&V0).unwrap().copied()), set([e3]));
        assert_eq!(set(u.in_edges_of(&V1).unwrap().copied()), set([e1, e7]));
        assert_eq!(set(u.in_edges_of(&V2).unwrap().copied()), set([e4]));
        assert_eq!(set(u.in_edges_of(&V3).unwrap().copied()), set([e5]));
        assert_eq!(set(u.in_edges_of(&V4).unwrap().copied()), set([e2, e6, e8]));
        assert_eq!(
            [V0, V1, V2, V3, V4].map(|v| u.in_degree_of(&v).unwrap()),
            [1, 2, 1, 1, 3]
        );
        assert_eq!(set(u.out_edges_of(&V0).unwrap().copied()), set([e1]));
        assert_eq!(set(u.out_edges_of(&V1).unwrap().copied()), set([e2, e4]));
        assert_eq!(set(u.out_edges_of(&V2).unwrap().copied()), set([e5]));
        assert_eq!(set(u.out_edges_of(&V3).unwrap().copied()), set([e6]));
        assert_eq!(set(u.out_edges_of(&V4).unwrap().copied()), set([e3, e7, e8]));
        assert_eq!(
            [V0, V1, V2, V3, V4].map(|v| u.out_degree_of(&v).unwrap()),
            [1, 2, 1, 1, 3]
        );
        assert!(!f.directed1.contains_edge_between(&V4, &V1));
        assert!(!f.directed2.contains_edge_between(&V1, &V4));
        assert_eq!(u.get_edge(&V1, &V4), Some(&e2));
        assert_eq!(u.get_edge(&V4, &V1), Some(&e7));
    }

    #[test]
    fn test_mixed_graph_union() {
        let f = setup();
        let u = GraphUnion::new(&f.undirected1, &f.directed2);
        let t = u.graph_type();
        assert!(t.is_mixed() && t.weighted && !t.modifiable);
        check_vertices_and_edges(&u, &f.e);
        let [e1, e2, e3, e4, e5, e6, e7, e8] = f.e;
        assert_eq!(set(u.in_edges_of(&V0).unwrap().copied()), set([e1, e3]));
        assert_eq!(set(u.in_edges_of(&V1).unwrap().copied()), set([e1, e2, e7]));
        assert_eq!(set(u.in_edges_of(&V2).unwrap().copied()), set([e4]));
        assert_eq!(set(u.in_edges_of(&V3).unwrap().copied()), set([e5]));
        assert_eq!(set(u.in_edges_of(&V4).unwrap().copied()), set([e2, e3, e6, e8]));
        assert_eq!(
            [V0, V1, V2, V3, V4].map(|v| u.in_degree_of(&v).unwrap()),
            [2, 3, 1, 1, 5]
        );
        assert_eq!(set(u.out_edges_of(&V0).unwrap().copied()), set([e1, e3]));
        assert_eq!(set(u.out_edges_of(&V1).unwrap().copied()), set([e1, e2, e4]));
        assert_eq!(set(u.out_edges_of(&V2).unwrap().copied()), set([e5]));
        assert_eq!(set(u.out_edges_of(&V3).unwrap().copied()), set([e6]));
        assert_eq!(set(u.out_edges_of(&V4).unwrap().copied()), set([e2, e3, e7, e8]));
        assert_eq!(
            [V0, V1, V2, V3, V4].map(|v| u.out_degree_of(&v).unwrap()),
            [2, 3, 1, 1, 5]
        );
        assert!(u.contains_edge_between(&V0, &V1)); // undirected edge
        assert!(u.contains_edge_between(&V1, &V0));
        assert!(u.contains_edge_between(&V3, &V4)); // directed edge
        assert!(!u.contains_edge_between(&V4, &V3));
    }

    #[test]
    fn test_weight_combiner() {
        let mut g1 = UGraph::<i32>::simple().weighted();
        g1.add_vertices([0, 1]);
        let edge = g1.add_edge(&0, &1).unwrap().unwrap();
        g1.set_edge_weight(&edge, 10.0).unwrap();
        let mut g2 = UGraph::<i32>::simple().weighted();
        g2.add_vertices([0, 1]);
        g2.add_edge_with(&0, &1, edge).unwrap();
        // in Java the weight lives inside the shared `DefaultWeightedEdge`; here each graph keeps its own
        g2.set_edge_weight(&edge, 10.0).unwrap();
        let g2_masked = AsWeighted::new(&g2, HashMap::from([(edge, 20.0)]));
        for (c, want) in [
            (WeightCombiner::Sum, 30.0),
            (WeightCombiner::First, 10.0),
            (WeightCombiner::Second, 20.0),
            (WeightCombiner::Max, 20.0),
            (WeightCombiner::Min, 10.0),
            (WeightCombiner::Mult, 200.0),
        ] {
            let u = GraphUnion::with_combiner(&g1, &g2_masked, c);
            assert_eq!(u.edge_weight(&edge), Some(want), "{c:?}");
        }
        assert_eq!(g1.edge_weight(&edge), Some(10.0));
        assert_eq!(g2.edge_weight(&edge), Some(10.0));
        assert_eq!(g2_masked.edge_weight(&edge), Some(20.0));
    }
}
