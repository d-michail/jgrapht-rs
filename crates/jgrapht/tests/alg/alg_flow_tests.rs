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

//! Ports of the `org.jgrapht.alg.flow` tests: the maximum-flow suite (MaximumFlowAlgorithmTest,
//! run for Edmonds–Karp, Dinic, push-relabel and Boykov–Kolmogorov, each with its own extra
//! tests) and the minimum s-t cut suite (MinimumSourceSinkCutTest, run for Edmonds–Karp and
//! push-relabel). Graph data is generated from the Java sources into `flow_data`.
//!
//! Not ported: the `NullPointerException` cases (references cannot be null) and the check that
//! the returned flow map is unmodifiable (the map is an owned value).

use crate::flow_data::*;
use jgrapht::alg::flow::*;
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::graph::GraphExt;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

const EPS: f64 = DEFAULT_EPSILON;

type Dg = DiGraph<i32>;
type Ug = UGraph<i32>;

fn construct_directed((tails, heads, capacities, sources, sinks): Directed) -> Dg {
    let mut network = Dg::multi().weighted();
    for i in 0..tails.len() {
        network.add_vertex(tails[i]);
        network.add_vertex(heads[i]);
        network
            .add_weighted_edge(&tails[i], &heads[i], capacities[i])
            .unwrap();
    }
    for i in 0..sources.len() {
        network.add_vertex(sources[i]);
        network.add_vertex(sinks[i]);
    }
    network
}

fn construct_undirected(edges: Undirected) -> Ug {
    let mut graph = Ug::simple().weighted();
    for [u, v, w] in edges {
        graph.add_vertex(*u);
        graph.add_vertex(*v);
        graph.add_weighted_edge(u, v, *w as f64).unwrap();
    }
    graph
}

fn generate_graph<D: jgrapht::graph::Direction>(
    rng: &mut StdRng,
) -> BaseGraph<i32, DefaultEdge, D> {
    let mut graph = BaseGraph::<i32, DefaultEdge, D>::simple()
        .weighted()
        .with_vertex_factory(|i| i as i32);
    GnmRandomGraphGenerator::new(100, 500, StdRng::seed_from_u64(rng.random()), false, false)
        .unwrap()
        .generate_default(&mut graph)
        .unwrap();
    let edges: Vec<DefaultEdge> = graph.edges().cloned().collect();
    for e in edges {
        graph
            .set_edge_weight(&e, rng.random_range(0..100) as f64)
            .unwrap();
    }
    graph
}

fn verify_directed<
    S: MaximumFlowAlgorithm<i32, DefaultEdge>,
    G: DirectedGraph<Vertex = i32, Edge = DefaultEdge> + IncidenceGraph,
>(
    network: &G,
    source: i32,
    sink: i32,
    expected: f64,
    max_flow: &MaximumFlow<i32, DefaultEdge>,
    _solver: &S,
) {
    let flow_value = max_flow.value();
    let flow = max_flow.flow_map();
    assert!(
        (expected - flow_value).abs() <= EPS,
        "{expected} vs {flow_value}"
    );
    for e in network.edges() {
        assert!(flow.contains_key(e));
    }
    for (e, f) in flow {
        assert!(network.contains_edge(e));
        assert!(*f >= -EPS);
        assert!(*f <= network.edge_weight(e).unwrap() + EPS);
    }
    for v in network.vertices() {
        let mut balance = 0.0;
        for e in network.out_edges_of(v) {
            balance -= flow[e];
        }
        for e in network.in_edges_of(v) {
            balance += flow[e];
        }
        let want = if *v == source {
            -flow_value
        } else if *v == sink {
            flow_value
        } else {
            0.0
        };
        assert!(
            (want - balance).abs() <= EPS,
            "balance at {v}: {balance} vs {want}"
        );
    }
}

fn verify_undirected<
    S: MaximumFlowAlgorithm<i32, DefaultEdge>,
    G: Graph<Vertex = i32, Edge = DefaultEdge> + IncidenceGraph,
>(
    graph: &G,
    source: i32,
    sink: i32,
    expected: f64,
    solver: &mut S,
) {
    let max_flow = solver.maximum_flow(&source, &sink).unwrap();
    let flow_value = max_flow.value();
    let flow = max_flow.flow_map();
    assert_eq!(expected, flow_value);
    for e in graph.edges() {
        assert!(flow.contains_key(e));
    }
    for (e, f) in flow {
        assert!(graph.contains_edge(e));
        assert!(*f >= -EPS);
        assert!(*f <= graph.edge_weight(e).unwrap() + EPS);
    }
    for u in graph.vertices() {
        let mut balance = 0.0;
        for e in graph.edges_of(u) {
            let v = *max_flow.flow_direction(e).unwrap();
            if *u == v {
                balance += flow[e];
            } else {
                balance -= flow[e];
            }
        }
        let want = if *u == source {
            -flow_value
        } else if *u == sink {
            flow_value
        } else {
            0.0
        };
        assert!((want - balance).abs() <= EPS, "balance at {u}");
    }
}

macro_rules! max_flow_suite {
    ($m:ident, $ctor:expr) => {
        mod $m {
            use super::*;

            fn run_directed(data: Directed, expected: f64) {
                let network = construct_directed(data);
                let mut solver = $ctor(&network).unwrap();
                for i in 0..data.3.len() {
                    let flow = solver.maximum_flow(&data.3[i], &data.4[i]).unwrap();
                    verify_directed(&network, data.3[i], data.4[i], expected, &flow, &solver);
                }
            }

            fn run_undirected(data: Undirected, source: i32, sink: i32, expected: f64) {
                let graph = construct_undirected(data);
                let mut solver = $ctor(&graph).unwrap();
                verify_undirected(&graph, source, sink, expected, &mut solver);
            }

            #[test]
            fn directed_n0() {
                run_directed(DIRECTED_N0, 5.0);
            }
            #[test]
            fn directed_n1() {
                run_directed(DIRECTED_N1, 0.0);
            }
            #[test]
            fn directed_n2() {
                run_directed(DIRECTED_N2, 2.0);
            }
            #[test]
            fn directed_n3() {
                run_directed(DIRECTED_N3, 4.0);
            }
            #[test]
            fn directed_n4() {
                run_directed(DIRECTED_N4, 2000000000.0);
            }
            #[test]
            fn directed_n6() {
                run_directed(DIRECTED_N6, 20.0);
            }
            #[test]
            fn directed_n7() {
                run_directed(DIRECTED_N7, 31.0);
            }
            #[test]
            fn directed_n8() {
                run_directed(DIRECTED_N8, 23.0);
            }
            #[test]
            fn directed_n9() {
                run_directed(DIRECTED_N9, 22.0);
            }
            #[test]
            fn directed_n10() {
                run_directed(DIRECTED_N10, 173.0);
            }
            #[test]
            fn directed_n11() {
                run_directed(DIRECTED_N11, 450.0);
            }
            #[test]
            fn directed_n12() {
                run_directed(DIRECTED_N12, 203.0);
            }
            #[test]
            fn undirected_n1() {
                run_undirected(UNDIRECTED_N1, 0, 8, 28.0);
            }
            #[test]
            fn undirected_n2() {
                run_undirected(UNDIRECTED_N2, 1, 4, 93.0);
            }
            #[test]
            fn undirected_n3() {
                run_undirected(UNDIRECTED_N3, 1, 49, 104.0);
            }
            #[test]
            fn undirected_n4() {
                run_undirected(UNDIRECTED_N4, 1, 99, 634.0);
            }
            #[test]
            fn undirected_n5() {
                run_undirected(UNDIRECTED_N5, 1, 49, 112.0);
            }
            #[test]
            fn undirected_n6() {
                run_undirected(UNDIRECTED_N6, 1, 69, 194.0);
            }
            #[test]
            fn undirected_n7() {
                run_undirected(UNDIRECTED_N7, 1, 69, 33.0);
            }
            #[test]
            fn undirected_n8() {
                run_undirected(UNDIRECTED_N8, 1, 99, 501.0);
            }
            #[test]
            fn undirected_n9() {
                run_undirected(UNDIRECTED_N9, 1, 2, 0.0);
            }
        }
    };
}

max_flow_suite!(edmonds_karp_mf_impl_test, EdmondsKarpMFImpl::new);
max_flow_suite!(dinic_mf_impl_test, DinicMFImpl::new);
max_flow_suite!(push_relabel_mf_impl_test, PushRelabelMFImpl::new);
max_flow_suite!(boykov_kolmogorov_mf_impl_test, BoykovKolmogorovMFImpl::new);

mod edmonds_karp_extras {
    use super::*;

    #[test]
    fn corner_cases() {
        let mut simple = Dg::multi().weighted();
        simple.add_vertex(0);
        simple.add_vertex(1);
        let e = simple.add_edge(&0, &1).unwrap().unwrap();
        assert!(EdmondsKarpMFImpl::with_epsilon(&simple, -0.1).is_err());
        simple.set_edge_weight(&e, -1.0).unwrap();
        assert!(EdmondsKarpMFImpl::new(&simple).is_err());
        simple.set_edge_weight(&e, 1.0).unwrap();
        let mut solver = EdmondsKarpMFImpl::new(&simple).unwrap();
        assert!(solver.maximum_flow(&2, &0).is_err());
        assert!(solver.maximum_flow(&1, &2).is_err());
        assert!(solver.maximum_flow(&0, &0).is_err());
        assert!(solver.maximum_flow(&0, &1).is_ok());
    }
}

mod dinic_extras {
    use super::*;

    fn graph() -> DiGraph<String> {
        DiGraph::<String>::with_loops().weighted()
    }

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn simple_test1() {
        let mut g = graph();
        g.add_vertex(s("v1"));
        g.add_vertex(s("v2"));
        g.add_weighted_edge(&s("v1"), &s("v2"), 100.0).unwrap();
        let mut dinic = DinicMFImpl::new(&g).unwrap();
        assert_eq!(100.0, dinic.maximum_flow_value(&s("v1"), &s("v2")).unwrap());
    }

    #[test]
    fn simple_test2() {
        let mut g = graph();
        for v in ["v1", "v2", "v3"] {
            g.add_vertex(s(v));
        }
        g.add_weighted_edge(&s("v1"), &s("v2"), 100.0).unwrap();
        g.add_weighted_edge(&s("v2"), &s("v3"), 50.0).unwrap();
        let mut dinic = DinicMFImpl::new(&g).unwrap();
        assert_eq!(50.0, dinic.maximum_flow_value(&s("v1"), &s("v3")).unwrap());
    }

    #[test]
    fn exception_test1() {
        let mut g = graph();
        g.add_vertex(s("v1"));
        let mut dinic = DinicMFImpl::new(&g).unwrap();
        assert!(dinic.maximum_flow_value(&s("v1"), &s("v1")).is_err());
    }

    #[test]
    fn disconnected_test() {
        let mut g = graph();
        g.add_vertex(s("v1"));
        g.add_vertex(s("v2"));
        let mut dinic = DinicMFImpl::new(&g).unwrap();
        assert_eq!(0.0, dinic.maximum_flow_value(&s("v1"), &s("v2")).unwrap());
    }

    #[test]
    fn simple_test3() {
        let mut g = graph();
        for v in ["v1", "v2", "v3", "v4"] {
            g.add_vertex(s(v));
        }
        for (a, b, w) in [
            ("v1", "v2", 2.0),
            ("v2", "v3", 2.0),
            ("v3", "v4", 2.0),
            ("v2", "v4", 1.0),
            ("v1", "v3", 1.0),
        ] {
            g.add_weighted_edge(&s(a), &s(b), w).unwrap();
        }
        let mut dinic = DinicMFImpl::new(&g).unwrap();
        assert_eq!(2.0, dinic.maximum_flow_value(&s("v1"), &s("v2")).unwrap());
    }
}

mod push_relabel_extras {
    use super::*;

    #[test]
    fn simple_directed_weighted_graph() {
        let mut graph = DiGraph::<i32>::simple().weighted();
        for v in [-1, -2, 0, 1] {
            graph.add_vertex(v);
        }
        for (a, b, w) in [
            (-1, 0, 1.0),
            (0, -2, 0.9999999999999999),
            (-1, 1, 1.0),
            (1, -2, 1.66498),
            (0, 1, 0.66498),
            (1, 0, 0.66498),
        ] {
            graph.add_weighted_edge(&a, &b, w).unwrap();
        }
        let mut mf = PushRelabelMFImpl::new(&graph).unwrap();
        assert!((2.0 - mf.minimum_cut(&-1, &-2).unwrap().capacity()).abs() <= 1e-9);
    }

    #[test]
    fn push_relabel_with_non_identical_node() {
        let mut g1 = DiGraph::<String>::simple();
        for v in ["v0", "v1", "v2", "v3", "v4"] {
            g1.add_vertex(v.to_string());
        }
        for (a, b) in [
            ("v0", "v2"),
            ("v3", "v4"),
            ("v1", "v0"),
            ("v0", "v4"),
            ("v0", "v1"),
            ("v2", "v1"),
        ] {
            g1.add_edge(&a.to_string(), &b.to_string()).unwrap();
        }
        let mut mf1 = PushRelabelMFImpl::new(&g1).unwrap();
        assert_eq!(
            0.0,
            mf1.maximum_flow_value(&"v3".to_string(), &"v0".to_string())
                .unwrap()
        );
    }
}

mod boykov_kolmogorov_extras {
    use super::*;

    fn build<D: jgrapht::graph::Direction>(edges: &[[i32; 3]]) -> BaseGraph<i32, DefaultEdge, D> {
        let mut graph = BaseGraph::<i32, DefaultEdge, D>::with_loops().weighted();
        for [u, v, w] in edges {
            graph.add_vertex(*u);
            graph.add_vertex(*v);
            graph.add_weighted_edge(u, v, *w as f64).unwrap();
        }
        graph
    }

    #[test]
    fn test1_to_10() {
        for (i, (edges, source, sink, directed, undirected)) in BK_CASES.iter().enumerate() {
            let d = build::<jgrapht::graph::Directed>(edges);
            let u = build::<jgrapht::graph::Undirected>(edges);
            let got = BoykovKolmogorovMFImpl::new(&d)
                .unwrap()
                .maximum_flow_value(source, sink)
                .unwrap();
            assert!(
                (directed - got).abs() <= 1e-9,
                "test{} directed: {got}",
                i + 1
            );
            let got = BoykovKolmogorovMFImpl::new(&u)
                .unwrap()
                .maximum_flow_value(source, sink)
                .unwrap();
            assert!(
                (undirected - got).abs() <= 1e-9,
                "test{} undirected: {got}",
                i + 1
            );
        }
    }
}

// MinimumSourceSinkCutTest

fn verify_cut<G: Graph<Vertex = i32, Edge = DefaultEdge>>(
    network: &G,
    directed: bool,
    source: i32,
    sink: i32,
    expected: f64,
    cut: &jgrapht::alg::flow::MinimumCut<i32, DefaultEdge>,
) {
    let source_partition = cut.source_partition().clone();
    let sink_partition = cut.sink_partition().clone();
    let cut_edges = cut.cut_edges().clone();
    assert_eq!(expected, cut.capacity());
    assert!(source_partition.contains(&source));
    assert!(sink_partition.contains(&sink));
    assert!(source_partition.is_disjoint(&sink_partition));
    let union: HashSet<i32> = source_partition
        .iter()
        .chain(sink_partition.iter())
        .copied()
        .collect();
    assert_eq!(union, network.vertices().copied().collect::<HashSet<_>>());
    let expected_edges: HashSet<DefaultEdge> = network
        .edges()
        .filter(|e| {
            let (u, v) = network.edge_endpoints(e).unwrap();
            if directed {
                source_partition.contains(u) && sink_partition.contains(v)
            } else {
                source_partition.contains(u) ^ source_partition.contains(v)
            }
        })
        .cloned()
        .collect();
    assert_eq!(
        expected_edges,
        cut_edges.iter().cloned().collect::<HashSet<_>>()
    );
    let total: f64 = cut_edges
        .iter()
        .map(|e| network.edge_weight(e).unwrap())
        .sum();
    assert_eq!(cut.capacity(), total);
}

macro_rules! min_cut_suite {
    ($m:ident, $ctor:expr, $other:expr) => {
        mod $m {
            use super::*;

            fn run_directed(data: Directed, source: i32, sink: i32, expected: f64) {
                let mut network = construct_directed(data);
                network.add_vertex(source);
                network.add_vertex(sink);
                let mut mc = $ctor(&network).unwrap();
                let cut = mc.minimum_cut(&source, &sink).unwrap();
                verify_cut(&network, true, source, sink, expected, &cut);
            }

            fn run_undirected(network: Ug, source: i32, sink: i32, expected: f64) {
                let mut mc = $ctor(&network).unwrap();
                let cut = mc.minimum_cut(&source, &sink).unwrap();
                verify_cut(&network, false, source, sink, expected, &cut);
            }

            #[test]
            fn problematic_case() {
                let mut network = Ug::simple().weighted();
                for (u, v, w) in [
                    (1, 2, 0.0),
                    (1, 4, 1.0),
                    (1, 5, 1.0),
                    (4, 5, 1.0),
                    (2, 3, 1.0),
                    (2, 6, 1.0),
                    (3, 6, 1.0),
                    (3, 4, 0.0),
                ] {
                    network.add_vertex(u);
                    network.add_vertex(v);
                    network.add_weighted_edge(&u, &v, w).unwrap();
                }
                run_undirected(network, 1, 6, 0.0);
            }

            #[test]
            fn directed_n0() {
                run_directed(DIRECTED_N0, 1, 4, 5.0);
            }
            #[test]
            fn directed_n1() {
                run_directed(DIRECTED_N1, 1, 4057218, 0.0);
            }
            #[test]
            fn directed_n2() {
                run_directed(DIRECTED_N2, 3, 6, 2.0);
            }
            #[test]
            fn directed_n3() {
                run_directed(DIRECTED_N3, 5, 6, 4.0);
            }
            #[test]
            fn directed_n4() {
                run_directed(DIRECTED_N4, 1, 4, 2000000000.0);
            }
            #[test]
            fn directed_n6() {
                run_directed(DIRECTED_N6, 1, 50, 20.0);
            }
            #[test]
            fn directed_n7() {
                run_directed(DIRECTED_N7, 1, 50, 31.0);
            }
            #[test]
            fn directed_n8() {
                run_directed(DIRECTED_N8, 0, 5, 23.0);
            }
            #[test]
            fn directed_n9() {
                run_directed(DIRECTED_N9, 0, 8, 22.0);
            }
            #[test]
            fn directed_n10() {
                run_directed(DIRECTED_N10, 1, 99, 173.0);
            }
            #[test]
            fn directed_n11() {
                run_directed(DIRECTED_N11, 1, 99, 450.0);
            }
            #[test]
            fn directed_n12() {
                run_directed(DIRECTED_N12, 1, 99, 203.0);
            }
            #[test]
            fn undirected_n1() {
                run_undirected(construct_undirected(UNDIRECTED_N1), 0, 8, 28.0);
            }
            #[test]
            fn undirected_n2() {
                run_undirected(construct_undirected(UNDIRECTED_N2), 1, 4, 93.0);
            }
            #[test]
            fn undirected_n3() {
                run_undirected(construct_undirected(UNDIRECTED_N3), 1, 49, 104.0);
            }
            #[test]
            fn undirected_n4() {
                run_undirected(construct_undirected(UNDIRECTED_N4), 1, 99, 634.0);
            }
            #[test]
            fn undirected_n5() {
                run_undirected(construct_undirected(UNDIRECTED_N5), 1, 49, 112.0);
            }
            #[test]
            fn undirected_n6() {
                run_undirected(construct_undirected(UNDIRECTED_N6), 1, 69, 194.0);
            }
            #[test]
            fn undirected_n7() {
                run_undirected(construct_undirected(UNDIRECTED_N7), 1, 69, 33.0);
            }
            #[test]
            fn undirected_n8() {
                run_undirected(construct_undirected(UNDIRECTED_N8), 1, 99, 501.0);
            }
            #[test]
            fn undirected_n9() {
                run_undirected(construct_undirected(UNDIRECTED_N9), 1, 2, 0.0);
            }

            #[test]
            fn random_directed_graphs() {
                let mut rng = StdRng::seed_from_u64(7);
                for _ in 0..500 {
                    let network = generate_graph::<jgrapht::graph::Directed>(&mut rng);
                    let (source, sink) = (0, network.vertex_count() as i32 - 1);
                    let mut mc = $ctor(&network).unwrap();
                    let mut other = $other(&network).unwrap();
                    let expected = other.minimum_cut(&source, &sink).unwrap().capacity();
                    let cut = mc.minimum_cut(&source, &sink).unwrap();
                    verify_cut(&network, true, source, sink, expected, &cut);
                }
            }

            #[test]
            fn random_undirected_graphs() {
                let mut rng = StdRng::seed_from_u64(8);
                for _ in 0..500 {
                    let network = generate_graph::<jgrapht::graph::Undirected>(&mut rng);
                    let (source, sink) = (0, network.vertex_count() as i32 - 1);
                    let mut mc = $ctor(&network).unwrap();
                    let mut other = $other(&network).unwrap();
                    let expected = other.minimum_cut(&source, &sink).unwrap().capacity();
                    let cut = mc.minimum_cut(&source, &sink).unwrap();
                    verify_cut(&network, false, source, sink, expected, &cut);
                }
            }
        }
    };
}

min_cut_suite!(
    edmonds_karp_minimum_st_cut_test,
    EdmondsKarpMFImpl::new,
    PushRelabelMFImpl::new
);
min_cut_suite!(
    push_relabel_minimum_st_cut_test,
    PushRelabelMFImpl::new,
    EdmondsKarpMFImpl::new
);

mod push_relabel_cut_extras {
    use super::*;

    #[test]
    fn disconnected1() {
        let mut network = Dg::simple().weighted();
        for v in 0..6 {
            network.add_vertex(v);
        }
        for (a, b) in [
            (2, 4),
            (3, 4),
            (1, 4),
            (0, 1),
            (2, 0),
            (1, 0),
            (4, 0),
            (4, 1),
            (1, 3),
            (4, 3),
        ] {
            network.add_edge(&a, &b).unwrap();
        }
        let mut pr = PushRelabelMFImpl::new(&network).unwrap();
        assert_eq!(0.0, pr.minimum_cut(&0, &5).unwrap().capacity());
    }

    #[test]
    fn disconnected2() {
        let mut network = Dg::simple().weighted();
        for v in 0..3 {
            network.add_vertex(v);
        }
        network.add_edge(&0, &1).unwrap();
        let mut pr = PushRelabelMFImpl::new(&network).unwrap();
        assert_eq!(0.0, pr.minimum_cut(&0, &2).unwrap().capacity());
    }
}

// GusfieldTreeAlgorithmsTestBase, GusfieldEquivalentFlowTreeTest, GusfieldGomoryHuCutTreeTest

mod gusfield {
    use super::*;
    use jgrapht::alg::stoer_wagner::StoerWagnerMinimumCut;
    use jgrapht::generate::GnpRandomGraphGenerator;
    use jgrapht::graph::props::is_tree;
    use std::collections::VecDeque;

    type Network = UGraph<i32>;

    fn network(vertices: &[i32], edges: &[(i32, i32, f64)]) -> Network {
        let mut g = Network::simple().weighted();
        for v in vertices {
            g.add_vertex(*v);
        }
        for (u, v, w) in edges {
            g.add_weighted_edge(u, v, *w).unwrap();
        }
        g
    }

    /// Weights of the edges on the unique path between `i` and `j` in a tree.
    fn path_weights(tree: &Network, i: i32, j: i32) -> Vec<f64> {
        let mut pred = std::collections::HashMap::new();
        let mut queue = VecDeque::from([i]);
        pred.insert(i, i);
        while let Some(u) = queue.pop_front() {
            for v in tree.neighbors_of(&u) {
                if !pred.contains_key(v) {
                    pred.insert(*v, u);
                    queue.push_back(*v);
                }
            }
        }
        let mut weights = Vec::new();
        let mut v = j;
        while v != i {
            let p = pred[&v];
            weights.push(tree.edge_weight(tree.get_edge(&v, &p).unwrap()).unwrap());
            v = p;
        }
        weights
    }

    fn min_weight(tree: &Network) -> f64 {
        tree.edges()
            .map(|e| tree.edge_weight(e).unwrap())
            .fold(f64::INFINITY, f64::min)
    }

    fn validate_equivalent_flow_tree(net: &Network) {
        let mut alg = GusfieldEquivalentFlowTree::new(net).unwrap();
        let tree = alg.equivalent_flow_tree().unwrap();
        assert!(is_tree(&tree));
        assert_eq!(StoerWagnerMinimumCut::new(net).unwrap().min_cut_weight(), min_weight(&tree));
        let mut st = PushRelabelMFImpl::new(net).unwrap();
        for i in net.vertices() {
            for j in net.vertices() {
                if j <= i {
                    continue;
                }
                let expected = st.minimum_cut(i, j).unwrap().capacity();
                assert_eq!(expected, alg.maximum_flow_value(i, j).unwrap());
                assert_eq!(expected, alg.maximum_flow_value(j, i).unwrap());
                let cheapest = path_weights(&tree, *i, *j)
                    .into_iter()
                    .fold(f64::INFINITY, f64::min);
                assert_eq!(expected, cheapest);
            }
        }
    }

    fn cut_weight(net: &Network, partition: &indexmap::IndexSet<i32>) -> f64 {
        net.edges()
            .filter(|e| {
                let (u, v) = net.edge_endpoints(e).unwrap();
                partition.contains(u) ^ partition.contains(v)
            })
            .map(|e| net.edge_weight(e).unwrap())
            .sum()
    }

    fn validate_gomory_hu_tree(net: &Network) {
        let mut alg = GusfieldGomoryHuCutTree::new(net).unwrap();
        let tree = alg.gomory_hu_tree().unwrap();
        assert!(is_tree(&tree));
        let expected_min_cut = StoerWagnerMinimumCut::new(net).unwrap().min_cut_weight();
        assert_eq!(expected_min_cut, min_weight(&tree));
        let global = alg.global_minimum_cut().unwrap();
        assert_eq!(expected_min_cut, global.capacity());
        let partition = global.source_partition().clone();
        assert_eq!(expected_min_cut, cut_weight(net, &partition));
        let mut st = PushRelabelMFImpl::new(net).unwrap();
        for i in net.vertices() {
            for j in net.vertices() {
                if j <= i {
                    continue;
                }
                let expected = st.minimum_cut(i, j).unwrap().capacity();
                assert_eq!(expected, alg.maximum_flow_value(i, j).unwrap());
                assert_eq!(expected, alg.maximum_flow_value(j, i).unwrap());
                assert_eq!(expected, alg.minimum_cut(j, i).unwrap().capacity());
                let cut = alg.minimum_cut(i, j).unwrap();
                assert_eq!(expected, cut.capacity());
                let source_partition = cut.source_partition().clone();
                assert!(source_partition.contains(i));
                let sink_partition = cut.sink_partition().clone();
                assert!(sink_partition.contains(j));
                assert!(source_partition.is_disjoint(&sink_partition));
                assert_eq!(expected, cut_weight(net, &source_partition));
                let copy = alg.gomory_hu_tree().unwrap();
                let cheapest = path_weights(&copy, *i, *j)
                    .into_iter()
                    .fold(f64::INFINITY, f64::min);
                assert_eq!(expected, cheapest);
            }
        }
    }

    macro_rules! tree_suite {
        ($m:ident, $validate:path) => {
            mod $m {
                use super::*;

                #[test]
                fn triangle_graph() {
                    $validate(&network(
                        &[0, 1, 2],
                        &[(0, 1, 3.0), (1, 2, 4.0), (0, 2, 7.0)],
                    ));
                }

                #[test]
                fn square_graph() {
                    $validate(&network(
                        &[1, 2, 3, 4, 5, 6],
                        &[
                            (1, 2, 1.0),
                            (3, 4, 1.0),
                            (5, 6, 1.0),
                            (5, 1, 1.0),
                            (1, 3, 1.0),
                            (6, 2, 1.0),
                            (2, 4, 1.0),
                        ],
                    ));
                }

                #[test]
                fn gomory_hu_example_graph() {
                    $validate(&network(
                        &[1, 2, 3, 4, 5, 6],
                        &[
                            (1, 2, 10.0),
                            (1, 6, 8.0),
                            (2, 6, 3.0),
                            (2, 3, 4.0),
                            (2, 5, 2.0),
                            (6, 3, 2.0),
                            (6, 4, 2.0),
                            (6, 5, 3.0),
                            (5, 3, 4.0),
                            (5, 4, 7.0),
                            (3, 4, 5.0),
                        ],
                    ));
                }

                #[test]
                fn graph_with_no_edges() {
                    $validate(&network(&[1, 2], &[]));
                }

                #[test]
                fn wikipedia_graph() {
                    $validate(&network(
                        &[0, 1, 2, 3, 4, 5],
                        &[
                            (0, 1, 1.0),
                            (0, 2, 7.0),
                            (1, 2, 1.0),
                            (1, 3, 3.0),
                            (1, 4, 2.0),
                            (2, 4, 4.0),
                            (3, 4, 1.0),
                            (3, 5, 6.0),
                            (4, 5, 2.0),
                        ],
                    ));
                }

                #[test]
                fn disconnected_graph() {
                    $validate(&network(
                        &[0, 1, 2, 3, 4],
                        &[(0, 1, 3.0), (1, 2, 4.0), (0, 2, 7.0), (3, 4, 9.0)],
                    ));
                }

                #[test]
                fn random_graphs() {
                    let mut rand = StdRng::seed_from_u64(0);
                    for _ in 0..10 {
                        let vertices = rand.random_range(10..=20);
                        let p = 0.01 * rand.random_range(50..=85) as f64;
                        let mut graph = Network::simple()
                            .weighted()
                            .with_vertex_factory(|i| i as i32);
                        GnpRandomGraphGenerator::new(
                            vertices,
                            p,
                            StdRng::seed_from_u64(rand.random()),
                            false,
                        )
                        .unwrap()
                        .generate_default(&mut graph)
                        .unwrap();
                        let edges: Vec<DefaultEdge> = graph.edges().cloned().collect();
                        for e in edges {
                            graph
                                .set_edge_weight(&e, rand.random_range(0..150) as f64)
                                .unwrap();
                        }
                        $validate(&graph);
                    }
                }
            }
        };
    }

    tree_suite!(
        gusfield_equivalent_flow_tree_test,
        validate_equivalent_flow_tree
    );
    tree_suite!(gusfield_gomory_hu_cut_tree_test, validate_gomory_hu_tree);
}

// PadbergRaoOddMinimumCutsetTest

mod padberg_rao_odd_minimum_cutset_test {
    use super::*;
    use indexmap::IndexSet;
    use jgrapht::alg::connectivity::ConnectivityInspector;
    use jgrapht::generate::GnpRandomGraphGenerator;
    use rand::seq::IndexedRandom;

    type Network = UGraph<i32>;
    type Pr<'g> = PadbergRaoOddMinimumCutset<'g, Network>;

    fn network(vertices: &[i32], edges: &[(i32, i32, f64)]) -> Network {
        let mut g = Network::simple().weighted();
        for v in vertices {
            g.add_vertex(*v);
        }
        for (u, v, w) in edges {
            g.add_weighted_edge(u, v, *w).unwrap();
        }
        g
    }

    fn run_test(net: &Network, odd_vertices: &IndexSet<i32>, use_tree_compression: bool) {
        let mut pr = Pr::new(net).unwrap();
        let cut = pr
            .minimum_odd_cut(odd_vertices.clone(), use_tree_compression)
            .unwrap()
            .expect("an odd cut-set exists");
        let cut_value = cut.capacity();
        let source_partition = cut.source_partition().clone();
        let sink_partition = cut.sink_partition().clone();
        let cut_edges: HashSet<DefaultEdge> = cut.cut_edges().iter().copied().collect();
        assert!(source_partition.is_disjoint(&sink_partition));
        let union: HashSet<i32> = source_partition
            .iter()
            .chain(sink_partition.iter())
            .copied()
            .collect();
        assert_eq!(net.vertices().copied().collect::<HashSet<_>>(), union);
        assert!(Pr::is_odd_vertex_set(&source_partition, odd_vertices));
        assert!(Pr::is_odd_vertex_set(&sink_partition, odd_vertices));
        let expected_cut_edges: HashSet<DefaultEdge> = net
            .edges()
            .filter(|e| {
                let (u, v) = net.edge_endpoints(e).unwrap();
                source_partition.contains(u) ^ source_partition.contains(v)
            })
            .cloned()
            .collect();
        assert_eq!(expected_cut_edges, cut_edges);
        let expected_weight: f64 = cut_edges.iter().map(|e| net.edge_weight(e).unwrap()).sum();
        assert_eq!(expected_weight, cut_value);

        // exhaustive check over the edges of a Gomory-Hu tree
        let mut gusfield = GusfieldGomoryHuCutTree::new(net).unwrap();
        let mut tree = gusfield.gomory_hu_tree().unwrap();
        let edges: Vec<DefaultEdge> = tree.edges().cloned().collect();
        let mut found_best = false;
        for edge in edges {
            let (source, target) = {
                let (u, v) = tree.edge_endpoints(&edge).unwrap();
                (*u, *v)
            };
            let edge_weight = tree.edge_weight(&edge).unwrap();
            tree.remove_edge(&edge);
            let partition: IndexSet<i32> = ConnectivityInspector::new()
                .connected_set_of(&tree, &source)
                .unwrap()
                .into_iter()
                .collect();
            if Pr::is_odd_vertex_set(&partition, odd_vertices) {
                assert!(cut_value <= edge_weight);
                found_best |= cut_value == edge_weight;
            }
            tree.add_edge_with(&source, &target, edge).unwrap();
            tree.set_edge_weight(&edge, edge_weight).unwrap();
        }
        assert!(found_best);
    }

    fn set(items: &[i32]) -> IndexSet<i32> {
        items.iter().copied().collect()
    }

    #[test]
    fn is_odd_set_method() {
        let vertices = set(&[1, 2, 3, 4, 5, 6]);
        assert!(Pr::is_odd_vertex_set(&vertices, &set(&[1, 2, 3, 7])));
        assert!(!Pr::is_odd_vertex_set(&vertices, &set(&[1, 2, 3, 4])));
    }

    #[test]
    fn example_graph() {
        let net = network(
            &[1, 2, 3, 4, 5, 6],
            &[
                (1, 2, 10.0),
                (1, 6, 8.0),
                (2, 6, 3.0),
                (2, 3, 4.0),
                (2, 5, 2.0),
                (6, 3, 2.0),
                (6, 4, 2.0),
                (6, 5, 3.0),
                (5, 3, 4.0),
                (5, 4, 7.0),
                (3, 4, 5.0),
            ],
        );
        let odd = set(&[2, 3, 5, 6]);
        run_test(&net, &odd, true);
        run_test(&net, &odd, false);
    }

    #[test]
    fn disconnected_graph() {
        let net = network(
            &[0, 1, 2, 3, 4],
            &[(0, 1, 3.0), (1, 2, 4.0), (0, 2, 7.0), (3, 4, 9.0)],
        );
        let odd = set(&[0, 1, 2, 4]);
        run_test(&net, &odd, true);
        run_test(&net, &odd, false);
    }

    #[test]
    fn graph() {
        let mut net = Network::simple().weighted();
        for v in [7, 10, 12, 3, 1, 5, 6] {
            net.add_vertex(v);
        }
        for (u, v, w) in [(1, 12, 1.0), (3, 5, 1.0), (5, 6, 1.0), (6, 12, 4.0)] {
            net.add_weighted_edge(&u, &v, w).unwrap();
        }
        let odd = set(&[7, 10, 12, 3]);
        run_test(&net, &odd, true);
        run_test(&net, &odd, false);
    }

    #[test]
    fn odd_number_of_odd_vertices_is_rejected() {
        let net = network(&[0, 1, 2], &[(0, 1, 1.0)]);
        let mut pr = Pr::new(&net).unwrap();
        assert!(matches!(
            pr.minimum_odd_cut(set(&[0, 1, 2]), true),
            Err(jgrapht::GraphError::InvalidParameter {
                name: "odd_vertices",
                ..
            })
        ));
        // no odd vertices, no odd cut-set
        assert!(pr.minimum_odd_cut(set(&[]), true).unwrap().is_none());
        assert!(pr.minimum_odd_cut(set(&[]), false).unwrap().is_none());
    }

    #[test]
    fn random_graphs() {
        let mut rand = StdRng::seed_from_u64(0);
        for _ in 0..8 {
            let vertices = rand.random_range(10..=30);
            let p = 0.01 * rand.random_range(50..=85) as f64;
            let mut graph = Network::simple()
                .weighted()
                .with_vertex_factory(|i| i as i32);
            GnpRandomGraphGenerator::new(vertices, p, StdRng::seed_from_u64(rand.random()), false)
                .unwrap()
                .generate_default(&mut graph)
                .unwrap();
            let edges: Vec<DefaultEdge> = graph.edges().cloned().collect();
            for e in edges {
                graph
                    .set_edge_weight(&e, rand.random_range(0..150) as f64)
                    .unwrap();
            }
            for _ in 0..8 {
                let mut max = vertices - 1;
                if max % 2 == 1 {
                    max -= 1;
                }
                let min = 2;
                let count =
                    min + 2 * (rand.random::<f64>() * ((max - min) / 2 + 1) as f64) as usize;
                let all: Vec<i32> = graph.vertices().copied().collect();
                let odd: IndexSet<i32> = all.choose_multiple(&mut rand, count).copied().collect();
                run_test(&graph, &odd, true);
                run_test(&graph, &odd, false);
            }
        }
    }
}

// CapacityScalingMinimumCostFlowTest

mod capacity_scaling_minimum_cost_flow_test {
    use crate::mincost_data::CASES;
    use jgrapht::alg::flow::mincost::*;
    use jgrapht::graph::GraphExt;
    use jgrapht::*;
    use std::collections::HashMap;

    const EPS: f64 = 1e-9;

    fn run(case: &[&[i64]], scaling_factor: i32, cost: f64) {
        let mut graph = DiGraph::<i32>::with_loops().weighted();
        let mut supply: HashMap<i32, i32> = HashMap::new();
        let mut lower: HashMap<DefaultEdge, i32> = HashMap::new();
        let mut upper: HashMap<DefaultEdge, i32> = HashMap::new();
        for data in case {
            if data.len() == 2 {
                graph.add_vertex(data[0] as i32);
                supply.insert(data[0] as i32, data[1] as i32);
            } else {
                let (u, v) = (data[0] as i32, data[1] as i32);
                graph.add_vertex(u);
                graph.add_vertex(v);
                let edge = graph
                    .add_weighted_edge(&u, &v, data[4] as f64)
                    .unwrap()
                    .unwrap();
                lower.insert(edge, data[2] as i32);
                upper.insert(edge, data[3] as i32);
            }
        }
        let problem = MinimumCostFlowProblemImpl::with_lower_bounds(
            &graph,
            |v| supply.get(v).copied().unwrap_or(0),
            |e| upper[e],
            |e| lower.get(e).copied().unwrap_or(0),
        );
        let mut algorithm = CapacityScalingMinimumCostFlow::with_scaling_factor(scaling_factor);
        let solution = algorithm.solve(&problem).unwrap();
        let flow = solution.flow();
        assert!(
            (cost - flow.cost).abs() <= EPS,
            "cost {} vs {cost}",
            flow.cost
        );
        assert!(solution.is_optimal(EPS));
        assert!(check_flow_and_dual_solution(
            solution.dual_solution(),
            flow,
            &problem
        ));
    }

    fn check_flow_and_dual_solution(
        dual: &HashMap<i32, f64>,
        flow: &MinimumCostFlow<DefaultEdge>,
        problem: &MinimumCostFlowProblemImpl<'_, DiGraph<i32>>,
    ) -> bool {
        let graph = problem.graph();
        for vertex in graph.vertices() {
            let supply = problem.node_supply(vertex);
            let flow_in: i32 = graph
                .in_edges_of(vertex)
                .map(|e| flow.flow(e).unwrap() as i32)
                .sum();
            let flow_out: i32 = graph
                .out_edges_of(vertex)
                .map(|e| flow.flow(e).unwrap() as i32)
                .sum();
            if supply != flow_out - flow_in {
                return false;
            }
        }
        for edge in graph.edges() {
            let f = flow.flow(edge).unwrap();
            if (problem.arc_capacity_lower_bound(edge) as f64) > f
                || (problem.arc_capacity_upper_bound(edge) as f64) < f
            {
                return false;
            }
        }
        for (edge, value) in &flow.flow_map {
            if *value < problem.arc_capacity_upper_bound(edge) as f64 {
                let (s, t) = graph.edge_endpoints(edge).unwrap();
                if graph.edge_weight(edge).unwrap() + dual[t] - dual[s] < -EPS {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn get_minimum_cost_flow() {
        for (number, fixed, case, cost) in CASES {
            if *fixed != 0 {
                run(case, *fixed, *cost);
            } else {
                for scaling_factor in 1..=5 {
                    run(case, scaling_factor, *cost);
                }
            }
            let _ = number;
        }
    }

    #[test]
    fn rejects_bad_problems() {
        // undirected network
        let g = UGraph::<i32>::simple().weighted();
        let problem = MinimumCostFlowProblemImpl::new(&g, |_| 0, |_| 1);
        assert!(
            CapacityScalingMinimumCostFlow::new()
                .get_minimum_cost_flow(&problem)
                .is_err()
        );
        // supplies that do not sum to zero
        let mut g = DiGraph::<i32>::with_loops().weighted();
        g.add_vertex(0);
        g.add_vertex(1);
        let problem = MinimumCostFlowProblemImpl::new(&g, |v| if *v == 0 { 1 } else { 0 }, |_| 1);
        assert!(
            CapacityScalingMinimumCostFlow::new()
                .get_minimum_cost_flow(&problem)
                .is_err()
        );
        // no feasible solution: supply 1 at an isolated vertex
        let problem = MinimumCostFlowProblemImpl::new(&g, |v| if *v == 0 { 1 } else { -1 }, |_| 1);
        assert!(
            CapacityScalingMinimumCostFlow::new()
                .get_minimum_cost_flow(&problem)
                .is_err()
        );
    }
}
