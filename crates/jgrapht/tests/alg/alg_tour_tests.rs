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

//! Ports of the `org.jgrapht.alg.tour` tests: the Hamiltonian path searches (Backtracking, Held–Karp,
//! DAG), the TSP heuristics (nearest neighbour, nearest and farthest insertion, greedy, two-approx,
//! 2-opt, Palmer) and the exact Held–Karp tour, plus the geometric smoke tests.
//!
//! Sizes are reduced from the Java tests where building the graph is quadratic (geometric tests: 300
//! instead of 1000 points; Palmer: at most 42 vertices in the random tests, 200 in the big one).
//!
//! Not ported: `NullPointerException` cases and the factory checks that reject negative state counts
//! (unsigned here).

use jgrapht::alg::tour::*;
use jgrapht::generate::{Complete, GraphGenerator};
use jgrapht::graph::GraphExt;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::hash::Hash;

type Ug = UGraph<i32>;
type Dg = DiGraph<i32>;

fn ug(n: i32, edges: &[(i32, i32)]) -> Ug {
    let mut g = Ug::simple();
    for i in 0..n {
        g.add_vertex(i);
    }
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
    g
}

fn dg(n: i32, edges: &[(i32, i32)]) -> Dg {
    let mut g = Dg::simple();
    for i in 0..n {
        g.add_vertex(i);
    }
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
    g
}

fn path_graph(n: i32) -> Ug {
    let edges: Vec<(i32, i32)> = (0..n - 1).map(|i| (i, i + 1)).collect();
    ug(n, &edges)
}

fn complete_graph(n: i32) -> Ug {
    let mut edges = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            edges.push((i, j));
        }
    }
    ug(n, &edges)
}

fn s(x: &str) -> String {
    x.to_string()
}

fn weighted_str(vertices: &[&str], edges: &[(&str, &str, f64)]) -> UGraph<String> {
    let mut g = UGraph::<String>::simple().weighted();
    for v in vertices {
        g.add_vertex(s(v));
    }
    for (a, b, w) in edges {
        g.add_weighted_edge(&s(a), &s(b), *w).unwrap();
    }
    g
}

/// `assertHamiltonian` of the TSP tests: a closed walk through every vertex exactly once.
fn assert_hamiltonian<G: Graph>(g: &G, path: &GraphWalk<G::Vertex, G::Edge>)
where
    G::Vertex: std::fmt::Debug,
{
    let vertices = path.vertex_list();
    let edges = path.edge_list();
    assert_eq!(path.start_vertex(), path.end_vertex());
    assert_eq!(path.start_vertex(), vertices.first());
    if g.vertex_count() == 1 {
        assert!(edges.is_empty());
        assert!(path.weight().abs() <= 1e-9);
        return;
    }
    assert_eq!(g.vertex_count(), vertices.len() - 1);
    assert_eq!(g.vertex_count(), edges.len());
    let mut weight = 0.0;
    let mut v = path.start_vertex().unwrap().clone();
    let mut visited = HashSet::new();
    for e in edges {
        v = g.opposite_vertex(e, &v).unwrap().clone();
        assert!(visited.insert(v.clone()));
        weight += g.edge_weight(e).unwrap();
    }
    assert!((path.weight() - weight).abs() <= 1e-9);
    assert_eq!(visited.len(), g.vertex_count());
    let start = &vertices[0];
    let mut seen = HashSet::new();
    seen.insert(start.clone());
    for (i, v) in vertices.iter().enumerate().skip(1) {
        if i == vertices.len() - 1 {
            assert_eq!(v, start);
        } else {
            assert!(seen.insert(v.clone()));
        }
    }
    assert_eq!(seen.len(), g.vertex_count());
}

mod christofides_three_halves_approx_metric_tsp_test {
    use super::*;

    fn graph(edges: &[(i32, i32, f64)]) -> Ug {
        let mut graph = Ug::simple().weighted();
        for &(source, target, weight) in edges {
            graph.add_vertex(source);
            graph.add_vertex(target);
            graph.add_weighted_edge(&source, &target, weight).unwrap();
        }
        graph
    }

    fn run_tour(edges: &[(i32, i32, f64)]) -> (Ug, GraphWalk<i32, DefaultEdge>) {
        let graph = graph(edges);
        let tour = ChristofidesThreeHalvesApproxMetricTSP::new()
            .get_tour(&graph)
            .unwrap()
            .unwrap();
        (graph, tour)
    }

    fn test_instance(edges: &[(i32, i32, f64)], optimum: f64) {
        let (graph, tour) = run_tour(edges);
        assert_hamiltonian(&graph, &tour);
        assert!(tour.weight() <= 1.5 * optimum);
    }

    #[test]
    fn rejects_directed_empty_and_incomplete_graphs() {
        assert!(
            ChristofidesThreeHalvesApproxMetricTSP::new()
                .get_tour(&Ug::simple().weighted())
                .is_err()
        );

        let mut incomplete = Ug::simple().weighted();
        incomplete.add_vertices([0, 1]);
        assert!(
            ChristofidesThreeHalvesApproxMetricTSP::new()
                .get_tour(&incomplete)
                .is_err()
        );
    }

    #[test]
    fn unique_tours() {
        let (graph, tour) = run_tour(&[(1, 2, 5.0)]);
        assert_hamiltonian(&graph, &tour);
        assert_eq!(tour.weight(), 10.0);

        let (graph, tour) = run_tour(&[(1, 2, 5.0), (1, 3, 5.0), (2, 3, 9.0)]);
        assert_hamiltonian(&graph, &tour);
        assert_eq!(tour.weight(), 19.0);
    }

    #[test]
    fn upstream_weighted_instances() {
        test_instance(
            &[
                (1, 0, 2.),
                (2, 0, 5.),
                (2, 1, 6.),
                (3, 0, 2.),
                (3, 1, 4.),
                (3, 2, 5.),
            ],
            15.,
        );
        test_instance(
            &[
                (1, 0, 8.),
                (2, 0, 4.),
                (2, 1, 4.),
                (3, 0, 5.),
                (3, 1, 8.),
                (3, 2, 6.),
                (4, 0, 7.),
                (4, 1, 7.),
                (4, 2, 5.),
                (4, 3, 6.),
            ],
            26.,
        );
        test_instance(
            &[
                (1, 0, 3.),
                (2, 0, 6.),
                (2, 1, 7.),
                (3, 0, 6.),
                (3, 1, 7.),
                (3, 2, 7.),
                (4, 0, 5.),
                (4, 1, 6.),
                (4, 2, 9.),
                (4, 3, 9.),
                (5, 0, 3.),
                (5, 1, 2.),
                (5, 2, 10.),
                (5, 3, 10.),
                (5, 4, 9.),
            ],
            33.,
        );
        test_instance(
            &[
                (1, 0, 6.),
                (2, 0, 2.),
                (2, 1, 9.),
                (3, 0, 7.),
                (3, 1, 1.),
                (3, 2, 8.),
                (4, 0, 2.),
                (4, 1, 7.),
                (4, 2, 3.),
                (4, 3, 8.),
                (5, 0, 5.),
                (5, 1, 5.),
                (5, 2, 6.),
                (5, 3, 6.),
                (5, 4, 3.),
                (6, 0, 4.),
                (6, 1, 5.),
                (6, 2, 5.),
                (6, 3, 6.),
                (6, 4, 2.),
                (6, 5, 5.),
            ],
            24.,
        );
        test_instance(
            &[
                (1, 0, 1.),
                (2, 0, 3.),
                (2, 1, 2.),
                (3, 0, 5.),
                (3, 1, 6.),
                (3, 2, 8.),
                (4, 0, 4.),
                (4, 1, 5.),
                (4, 2, 7.),
                (4, 3, 4.),
                (5, 0, 6.),
                (5, 1, 7.),
                (5, 2, 9.),
                (5, 3, 6.),
                (5, 4, 8.),
                (6, 0, 6.),
                (6, 1, 7.),
                (6, 2, 9.),
                (6, 3, 6.),
                (6, 4, 8.),
                (6, 5, 9.),
                (7, 0, 4.),
                (7, 1, 5.),
                (7, 2, 7.),
                (7, 3, 4.),
                (7, 4, 6.),
                (7, 5, 7.),
                (7, 6, 6.),
            ],
            39.,
        );
        test_instance(
            &[
                (1, 0, 5.),
                (2, 0, 4.),
                (2, 1, 5.),
                (3, 0, 3.),
                (3, 1, 7.),
                (3, 2, 6.),
                (4, 0, 5.),
                (4, 1, 7.),
                (4, 2, 6.),
                (4, 3, 5.),
                (5, 0, 5.),
                (5, 1, 8.),
                (5, 2, 7.),
                (5, 3, 6.),
                (5, 4, 8.),
                (6, 0, 5.),
                (6, 1, 8.),
                (6, 2, 7.),
                (6, 3, 6.),
                (6, 4, 8.),
                (6, 5, 7.),
                (7, 0, 5.),
                (7, 1, 7.),
                (7, 2, 6.),
                (7, 3, 5.),
                (7, 4, 7.),
                (7, 5, 6.),
                (7, 6, 8.),
                (8, 0, 5.),
                (8, 1, 6.),
                (8, 2, 5.),
                (8, 3, 4.),
                (8, 4, 6.),
                (8, 5, 5.),
                (8, 6, 8.),
                (8, 7, 7.),
                (9, 0, 5.),
                (9, 1, 5.),
                (9, 2, 4.),
                (9, 3, 3.),
                (9, 4, 5.),
                (9, 5, 4.),
                (9, 6, 8.),
                (9, 7, 7.),
                (9, 8, 6.),
            ],
            52.,
        );
    }
}

/// `HamiltonianPathValidator.assertHamiltonianPath`.
fn assert_hamiltonian_path<G: Graph>(
    graph: &G,
    result: &HamiltonianPathSearchResult<G::Vertex, G::Edge>,
) where
    G::Vertex: std::fmt::Debug,
{
    assert_eq!(SearchStatus::PathFound, result.status());
    let path = result.path().expect("a path");
    let vertices = path.vertex_list();
    let edges = path.edge_list();
    let n = graph.vertex_count();
    assert_eq!(n, vertices.len());
    assert_eq!(n.saturating_sub(1), edges.len());
    let seen: HashSet<&G::Vertex> = vertices.iter().collect();
    assert_eq!(n, seen.len(), "duplicate vertex");
    assert!(graph.vertices().all(|v| seen.contains(v)));
    assert_eq!(Some(&vertices[0]), path.start_vertex());
    assert_eq!(Some(&vertices[n - 1]), path.end_vertex());
    let mut weight = 0.0;
    for (i, e) in edges.iter().enumerate() {
        let (u, v) = (&vertices[i], &vertices[i + 1]);
        assert!(graph.contains_edge(e));
        let (src, tgt) = graph.edge_endpoints(e).unwrap();
        if graph.graph_type().directed {
            assert!(u == src && v == tgt, "edge direction at {i}");
        } else {
            assert!((u == src && v == tgt) || (u == tgt && v == src));
        }
        weight += graph.edge_weight(e).unwrap();
    }
    assert!((weight - path.weight()).abs() <= 1e-9);
}

fn assert_proven_absent<V, E>(result: &HamiltonianPathSearchResult<V, E>) {
    assert_eq!(SearchStatus::ProvenAbsent, result.status());
    assert!(result.path().is_none());
}

fn bt<G: IncidenceGraph>(graph: &G) -> HamiltonianPathSearchResult<G::Vertex, G::Edge> {
    BacktrackingHamiltonianPath::new().get_path(graph).unwrap()
}

fn random_undirected(n: i32, p: f64, rnd: &mut StdRng) -> Ug {
    let mut g = Ug::simple();
    for i in 0..n {
        g.add_vertex(i);
    }
    for i in 0..n {
        for j in i + 1..n {
            if rnd.random::<f64>() < p {
                g.add_edge(&i, &j).unwrap();
            }
        }
    }
    g
}

fn random_directed(n: i32, p: f64, rnd: &mut StdRng) -> Dg {
    let mut g = Dg::simple();
    for i in 0..n {
        g.add_vertex(i);
    }
    for i in 0..n {
        for j in 0..n {
            if i != j && rnd.random::<f64>() < p {
                g.add_edge(&i, &j).unwrap();
            }
        }
    }
    g
}

// DagHamiltonianPathTest

mod dag_hamiltonian_path_test {
    use super::*;

    fn dag<G: IncidenceGraph>(
        graph: &G,
    ) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        DagHamiltonianPath::new().get_path(graph)
    }

    #[test]
    fn empty_graph_throws() {
        assert!(dag(&Dg::simple()).is_err());
    }

    #[test]
    fn undirected_graph_throws() {
        let mut g = Ug::simple();
        g.add_vertex(0);
        assert!(dag(&g).is_err());
    }

    #[test]
    fn cyclic_directed_graph_throws() {
        assert!(dag(&dg(3, &[(0, 1), (1, 2), (2, 0)])).is_err());
    }

    #[test]
    fn single_vertex_returns_singleton_path() {
        let mut g = DiGraph::<String>::simple();
        g.add_vertex(s("only"));
        let result = dag(&g).unwrap();
        assert_eq!(SearchStatus::PathFound, result.status());
        assert_eq!(1, result.path().unwrap().vertex_list().len());
    }

    #[test]
    fn linear_chain_has_path() {
        let g = dg(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
        let result = dag(&g).unwrap();
        assert_hamiltonian_path(&g, &result);
        let path = result.path().unwrap();
        assert_eq!(Some(&0), path.start_vertex());
        assert_eq!(Some(&5), path.end_vertex());
    }

    #[test]
    fn chain_plus_forward_shortcuts_has_path() {
        let g = dg(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (0, 2), (1, 3)]);
        assert_hamiltonian_path(&g, &dag(&g).unwrap());
    }

    #[test]
    fn parallel_chains_have_no_path() {
        let g = dg(6, &[(0, 1), (1, 2), (3, 4), (4, 5)]);
        assert_proven_absent(&dag(&g).unwrap());
    }

    #[test]
    fn diamond_dag_without_hamiltonian_path() {
        let g = dg(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        assert_proven_absent(&dag(&g).unwrap());
    }

    #[test]
    fn diamond_dag_with_cross_edge_has_path() {
        let g = dg(4, &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 3)]);
        assert_hamiltonian_path(&g, &dag(&g).unwrap());
    }

    #[test]
    fn cross_validation_against_backtracking_on_random_dags() {
        use jgrapht::util::java::java_shuffle;
        let mut random = StdRng::seed_from_u64(0xDA6DA6);
        for n in 2..=8 {
            for p in [0.3, 0.5, 0.8] {
                for _ in 0..25 {
                    let mut order: Vec<i32> = (0..n).collect();
                    let mut graph = Dg::simple();
                    for i in 0..n {
                        graph.add_vertex(i);
                    }
                    java_shuffle(&mut order, &mut random);
                    for i in 0..n as usize {
                        for j in i + 1..n as usize {
                            if random.random::<f64>() < p {
                                graph.add_edge(&order[i], &order[j]).unwrap();
                            }
                        }
                    }
                    let dag_result = dag(&graph).unwrap();
                    let bt_result = bt(&graph);
                    assert_eq!(bt_result.path().is_some(), dag_result.path().is_some());
                    if dag_result.path().is_some() {
                        assert_hamiltonian_path(&graph, &dag_result);
                    }
                }
            }
        }
    }
}

// HeldKarpHamiltonianPathTest

mod held_karp_hamiltonian_path_test {
    use super::*;

    fn dp<G: IncidenceGraph>(graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        HeldKarpHamiltonianPath::new().get_path(graph)
    }

    fn cross_check<G: IncidenceGraph>(graph: &G)
    where
        G::Vertex: std::fmt::Debug,
    {
        let dp_result = dp(graph).unwrap();
        let bt_result = bt(graph);
        assert_eq!(bt_result.path().is_some(), dp_result.path().is_some());
        if dp_result.path().is_some() {
            assert_hamiltonian_path(graph, &dp_result);
        }
        if bt_result.path().is_some() {
            assert_hamiltonian_path(graph, &bt_result);
        }
    }

    #[test]
    fn empty_graph_throws() {
        assert!(dp(&Ug::simple()).is_err());
    }

    #[test]
    fn single_vertex_returns_singleton_path() {
        let mut g = UGraph::<String>::simple();
        g.add_vertex(s("only"));
        let result = dp(&g).unwrap();
        assert_eq!(SearchStatus::PathFound, result.status());
        let path = result.path().unwrap();
        assert_eq!(1, path.vertex_list().len());
        assert_eq!(Some(&s("only")), path.start_vertex());
        assert_eq!(Some(&s("only")), path.end_vertex());
        assert_eq!(0, path.edge_list().len());
    }

    #[test]
    fn too_many_vertices_throws() {
        assert!(
            dp(&ug(
                HeldKarpHamiltonianPath::DEFAULT_MAX_VERTICES as i32 + 1,
                &[]
            ))
            .is_err()
        );
    }

    #[test]
    fn custom_max_vertices_accepted() {
        let mut small = HeldKarpHamiltonianPath::with_max_vertices(4).unwrap();
        let ok = path_graph(4);
        assert_hamiltonian_path(&ok, &small.get_path(&ok).unwrap());
        assert!(small.get_path(&ug(5, &[])).is_err());
    }

    #[test]
    fn constructor_rejects_out_of_range_max_vertices() {
        assert!(HeldKarpHamiltonianPath::with_max_vertices(0).is_err());
        assert!(
            HeldKarpHamiltonianPath::with_max_vertices(
                HeldKarpHamiltonianPath::HARD_MAX_VERTICES + 1
            )
            .is_err()
        );
    }

    #[test]
    fn undirected_path_graph() {
        let g = path_graph(6);
        assert_hamiltonian_path(&g, &dp(&g).unwrap());
    }

    #[test]
    fn undirected_complete_graph() {
        let g = complete_graph(5);
        assert_hamiltonian_path(&g, &dp(&g).unwrap());
    }

    #[test]
    fn directed_chain_has_path() {
        let g = dg(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
        let result = dp(&g).unwrap();
        assert_hamiltonian_path(&g, &result);
        let path = result.path().unwrap();
        assert_eq!(Some(&0), path.start_vertex());
        assert_eq!(Some(&5), path.end_vertex());
    }

    #[test]
    fn star_with_three_leaves_has_no_path() {
        let mut g = UGraph::<String>::simple();
        for v in ["hub", "a", "b", "c"] {
            g.add_vertex(s(v));
        }
        for v in ["a", "b", "c"] {
            g.add_edge(&s("hub"), &s(v)).unwrap();
        }
        assert_proven_absent(&dp(&g).unwrap());
    }

    #[test]
    fn disconnected_has_no_path() {
        assert_proven_absent(&dp(&ug(4, &[(0, 1), (2, 3)])).unwrap());
    }

    #[test]
    fn self_loops_and_parallel_edges_are_handled() {
        let mut g = Ug::pseudo();
        for i in 0..3 {
            g.add_vertex(i);
        }
        for (a, b) in [(0, 1), (1, 2), (0, 0), (1, 1), (0, 1)] {
            g.add_edge(&a, &b).unwrap();
        }
        assert_hamiltonian_path(&g, &dp(&g).unwrap());
    }

    #[test]
    fn weight_is_sum_of_edge_weights() {
        let mut g = Ug::simple().weighted();
        for i in 0..4 {
            g.add_vertex(i);
        }
        g.add_weighted_edge(&0, &1, 1.0).unwrap();
        g.add_weighted_edge(&1, &2, 2.0).unwrap();
        g.add_weighted_edge(&2, &3, 3.0).unwrap();
        let result = dp(&g).unwrap();
        assert_hamiltonian_path(&g, &result);
        assert!((6.0 - result.path().unwrap().weight()).abs() <= 1e-9);
    }

    #[test]
    fn cross_validation_against_backtracking_undirected() {
        let mut random = StdRng::seed_from_u64(0xBEEFFACE12345678u64);
        for n in 2..=8 {
            for p in [0.3, 0.5, 0.8] {
                for _ in 0..25 {
                    cross_check(&random_undirected(n, p, &mut random));
                }
            }
        }
    }

    #[test]
    fn cross_validation_against_backtracking_directed() {
        let mut random = StdRng::seed_from_u64(0xFEEDBEEF87654321u64);
        for n in 2..=8 {
            for p in [0.3, 0.5, 0.8] {
                for _ in 0..25 {
                    cross_check(&random_directed(n, p, &mut random));
                }
            }
        }
    }
}

// BacktrackingHamiltonianPathTest, BacktrackingHamiltonianPathBoundedTest, ...RandomTest

mod backtracking_hamiltonian_path_test {
    use super::*;

    #[test]
    fn empty_graph_throws() {
        assert!(
            BacktrackingHamiltonianPath::new()
                .get_path(&Ug::simple())
                .is_err()
        );
    }

    #[test]
    fn single_vertex_returns_singleton_path() {
        let mut g = UGraph::<String>::simple();
        g.add_vertex(s("only"));
        let result = bt(&g);
        assert_eq!(SearchStatus::PathFound, result.status());
        let path = result.path().unwrap();
        assert_eq!(1, path.vertex_list().len());
        assert_eq!(Some(&s("only")), path.start_vertex());
        assert_eq!(0, path.edge_list().len());
        assert_eq!(0.0, path.weight());
    }

    #[test]
    fn undirected_path_graph() {
        let g = path_graph(5);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn undirected_cycle_graph() {
        let edges: Vec<(i32, i32)> = (0..6).map(|i| (i, (i + 1) % 6)).collect();
        let g = ug(6, &edges);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn undirected_complete_graph_k4() {
        let g = complete_graph(4);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn disconnected_undirected_graph_has_no_path() {
        assert_proven_absent(&bt(&ug(4, &[(0, 1), (2, 3)])));
    }

    #[test]
    fn star_with_three_leaves_has_no_path() {
        let mut g = UGraph::<String>::simple();
        for v in ["hub", "a", "b", "c"] {
            g.add_vertex(s(v));
        }
        for v in ["a", "b", "c"] {
            g.add_edge(&s("hub"), &s(v)).unwrap();
        }
        assert_proven_absent(&bt(&g));
    }

    #[test]
    fn two_isolated_vertices_have_no_path() {
        assert_proven_absent(&bt(&ug(2, &[])));
    }

    #[test]
    fn directed_chain_has_path() {
        let g = dg(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        let result = bt(&g);
        assert_hamiltonian_path(&g, &result);
        let path = result.path().unwrap();
        assert_eq!(Some(&0), path.start_vertex());
        assert_eq!(Some(&4), path.end_vertex());
    }

    #[test]
    fn directed_chain_missing_middle_edge_has_no_path() {
        assert_proven_absent(&bt(&dg(4, &[(0, 1), (2, 3)])));
    }

    #[test]
    fn disjoint_strongly_connected_components_have_no_path() {
        assert_proven_absent(&bt(&dg(4, &[(0, 1), (1, 0), (2, 3), (3, 2)])));
    }

    #[test]
    fn chain_of_strongly_connected_components_has_path() {
        let g = dg(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)]);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn directed_graph_with_unreachable_vertex_has_no_path() {
        assert_proven_absent(&bt(&dg(4, &[(0, 1), (0, 2)])));
    }

    #[test]
    fn directed_acyclic_graph_with_hamiltonian_path() {
        let g = dg(4, &[(0, 1), (1, 2), (2, 3), (0, 2)]);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn weighted_undirected_graph_returns_correct_weight() {
        let mut g = Ug::simple().weighted();
        for i in 0..4 {
            g.add_vertex(i);
        }
        g.add_weighted_edge(&0, &1, 1.5).unwrap();
        g.add_weighted_edge(&1, &2, 2.5).unwrap();
        g.add_weighted_edge(&2, &3, 3.5).unwrap();
        let result = bt(&g);
        assert_hamiltonian_path(&g, &result);
        assert!((7.5 - result.path().unwrap().weight()).abs() <= 1e-9);
    }

    fn pseudo(n: i32, edges: &[(i32, i32)]) -> Ug {
        let mut g = Ug::pseudo();
        for i in 0..n {
            g.add_vertex(i);
        }
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    #[test]
    fn self_loops_are_ignored() {
        let g = pseudo(3, &[(0, 1), (1, 2), (0, 0), (2, 2)]);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn self_loop_on_cut_vertex_is_ignored() {
        let g = pseudo(3, &[(0, 1), (1, 2), (1, 1)]);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn self_loops_on_multiple_cut_vertices_are_ignored() {
        let g = pseudo(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (1, 1), (2, 2), (3, 3)]);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    #[test]
    fn square_with_three_bridged_triangles_has_no_path() {
        let g = ug(
            13,
            &[
                (0, 1),
                (1, 2),
                (2, 3),
                (0, 3),
                (4, 5),
                (5, 6),
                (4, 6),
                (7, 8),
                (8, 9),
                (7, 9),
                (10, 11),
                (11, 12),
                (10, 12),
                (0, 4),
                (1, 7),
                (2, 10),
            ],
        );
        assert_proven_absent(&HeldKarpHamiltonianPath::new().get_path(&g).unwrap());
        assert_proven_absent(&bt(&g));
    }

    #[test]
    fn undirected_graph_with_articulation_still_has_path() {
        let g = ug(5, &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 2)]);
        assert_hamiltonian_path(&g, &bt(&g));
    }

    // BacktrackingHamiltonianPathBoundedTest

    fn search(graph: &Ug, max_states: u64) -> HamiltonianPathSearchResult<i32, DefaultEdge> {
        BacktrackingHamiltonianPath::new()
            .search_with_state_limit(graph, max_states)
            .unwrap()
    }

    #[test]
    fn zero_max_states_throws() {
        assert!(
            BacktrackingHamiltonianPath::new()
                .search_with_state_limit(&path_graph(3), 0)
                .is_err()
        );
    }

    #[test]
    fn empty_graph_throws_bounded() {
        assert!(
            BacktrackingHamiltonianPath::new()
                .search_with_state_limit(&Ug::simple(), 100)
                .is_err()
        );
    }

    #[test]
    fn easy_path_returns_found_under_high_limit() {
        let g = path_graph(6);
        let result = search(&g, 1_000_000);
        assert_hamiltonian_path(&g, &result);
        assert!(result.states_expanded() > 0);
    }

    #[test]
    fn disconnected_graph_is_proven_absent_before_any_state() {
        let result = search(&ug(4, &[(0, 1), (2, 3)]), 10);
        assert_eq!(SearchStatus::ProvenAbsent, result.status());
        assert!(result.path().is_none());
        assert_eq!(0, result.states_expanded());
    }

    #[test]
    fn small_limit_on_hard_graph_returns_aborted() {
        let result = search(&complete_graph(12), 2);
        assert_eq!(SearchStatus::Aborted, result.status());
        assert!(result.path().is_none());
        assert_eq!(2, result.states_expanded());
    }

    #[test]
    fn states_expanded_never_exceeds_max_states() {
        let g = complete_graph(10);
        for budget in 1..=20 {
            assert!(search(&g, budget).states_expanded() <= budget);
        }
    }

    #[test]
    fn exactly_enough_budget_finds_path() {
        let g = path_graph(8);
        assert_hamiltonian_path(&g, &search(&g, 8));
    }

    #[test]
    fn bounded_search_does_not_affect_subsequent_unbounded_call() {
        let mut algo = BacktrackingHamiltonianPath::new();
        let hard = complete_graph(12);
        let bounded = algo.search_with_state_limit(&hard, 2).unwrap();
        assert_eq!(SearchStatus::Aborted, bounded.status());
        let unbounded = algo.get_path(&hard).unwrap();
        assert_hamiltonian_path(&hard, &unbounded);
    }

    #[test]
    fn factories() {
        let proven = HamiltonianPathSearchResult::<i32, DefaultEdge>::proven_absent(7);
        assert_eq!(SearchStatus::ProvenAbsent, proven.status());
        assert!(proven.path().is_none());
        assert_eq!(7, proven.states_expanded());
        let aborted = HamiltonianPathSearchResult::<i32, DefaultEdge>::aborted(42);
        assert_eq!(SearchStatus::Aborted, aborted.status());
        assert!(aborted.path().is_none());
        assert_eq!(42, aborted.states_expanded());
    }

    // BacktrackingHamiltonianPathRandomTest

    fn brute_force_exists<G: Graph<Vertex = i32>>(graph: &G) -> bool {
        fn permute<G: Graph<Vertex = i32>>(graph: &G, v: &mut Vec<i32>, from: usize) -> bool {
            if from == v.len() {
                return (1..v.len()).all(|i| graph.contains_edge_between(&v[i - 1], &v[i]));
            }
            for i in from..v.len() {
                v.swap(from, i);
                if permute(graph, v, from + 1) {
                    return true;
                }
                v.swap(from, i);
            }
            false
        }
        let mut vertices: Vec<i32> = graph.vertices_cloned().collect();
        vertices.len() == 1 || permute(graph, &mut vertices, 0)
    }

    fn run_one<G: Graph<Vertex = i32> + IncidenceGraph>(graph: &G) {
        let oracle = brute_force_exists(graph);
        let result = bt(graph);
        assert_eq!(oracle, result.path().is_some());
        if result.path().is_some() {
            assert_hamiltonian_path(graph, &result);
        }
    }

    const SEED: i64 = 0xC0FFEE0123456789u64 as i64;

    #[test]
    fn undirected_random_graphs_match_oracle() {
        let mut random = StdRng::seed_from_u64(SEED as u64);
        for n in 2..=6 {
            for p in [0.3, 0.5, 0.8] {
                for _ in 0..40 {
                    run_one(&random_undirected(n, p, &mut random));
                }
            }
        }
    }

    #[test]
    fn directed_random_graphs_match_oracle() {
        let mut random = StdRng::seed_from_u64((SEED ^ 0x5A5A5A5A) as u64);
        for n in 2..=6 {
            for p in [0.3, 0.5, 0.8] {
                for _ in 0..40 {
                    run_one(&random_directed(n, p, &mut random));
                }
            }
        }
    }
}

// Shared helpers of the TSP heuristic tests

fn matrix_graph(all_dist: &[&[i32]]) -> Ug {
    let n = all_dist.len();
    let mut graph = Ug::simple().weighted();
    for i in 0..n {
        for j in i + 1..n {
            graph.add_vertex(i as i32);
            graph.add_vertex(j as i32);
            graph
                .add_weighted_edge(&(i as i32), &(j as i32), all_dist[i][j] as f64)
                .unwrap();
        }
    }
    graph
}

fn wiki_four_cities() -> UGraph<String> {
    weighted_str(
        &["A", "B", "C", "D"],
        &[
            ("A", "B", 20.0),
            ("A", "C", 42.0),
            ("A", "D", 35.0),
            ("B", "C", 30.0),
            ("B", "D", 34.0),
            ("C", "D", 12.0),
        ],
    )
}

fn star_graph() -> UGraph<String> {
    weighted_str(
        &["1", "2", "3", "4", "5", "6"],
        &[
            ("1", "2", 1.0),
            ("1", "3", 1.0),
            ("1", "4", 1.0),
            ("1", "5", 2.0),
            ("1", "6", 2.0),
            ("2", "3", 2.0),
            ("2", "4", 1.0),
            ("2", "5", 1.0),
            ("2", "6", 2.0),
            ("3", "4", 1.0),
            ("3", "5", 2.0),
            ("3", "6", 1.0),
            ("4", "5", 1.0),
            ("4", "6", 1.0),
            ("5", "6", 1.0),
        ],
    )
}

fn unweighted_complete(n: i32) -> Ug {
    let mut g = Ug::simple().with_vertex_factory(|i| i as i32);
    Complete(n as usize).generate_default(&mut g).unwrap();
    g
}

fn mst_weight<G: Graph>(g: &G) -> f64 {
    jgrapht::alg::spanning::kruskal(g).weight
}

// TwoApproxMetricTSPTest

mod two_approx_metric_tsp_test {
    use super::*;

    #[test]
    fn wiki_example_symmetric_4_cities() {
        let g = wiki_four_cities();
        let tour = TwoApproxMetricTSP::new().get_tour(&g).unwrap().unwrap();
        assert_hamiltonian(&g, &tour);
        assert!(2.0 * mst_weight(&g) >= tour.weight());
    }

    #[test]
    fn complete() {
        for i in 1..50 {
            let g = unweighted_complete(i);
            let tour = TwoApproxMetricTSP::new().get_tour(&g).unwrap().unwrap();
            assert_hamiltonian(&g, &tour);
            assert!(2.0 * mst_weight(&g) >= tour.weight());
        }
    }

    #[test]
    fn star() {
        let g = star_graph();
        let tour = TwoApproxMetricTSP::new().get_tour(&g).unwrap().unwrap();
        assert_hamiltonian(&g, &tour);
        assert!(2.0 * mst_weight(&g) >= tour.weight());
    }

    #[test]
    fn invalid_instance_not_complete() {
        let g = weighted_str(&["A", "B", "C"], &[("A", "B", 20.0), ("A", "C", 42.0)]);
        assert!(TwoApproxMetricTSP::new().get_tour(&g).is_err());
    }
}

// TwoOptHeuristicTSPTest, TwoOptHeuristicTSPFloatingPointTest

mod two_opt_heuristic_tsp_test {
    use super::*;

    #[test]
    fn wiki_example_symmetric_4_cities() {
        let g = wiki_four_cities();
        let tour = TwoOptHeuristicTSP::new().get_tour(&g).unwrap().unwrap();
        assert_hamiltonian(&g, &tour);
    }

    #[test]
    fn complete() {
        for i in 1..50 {
            let g = unweighted_complete(i);
            let tour = TwoOptHeuristicTSP::new().get_tour(&g).unwrap().unwrap();
            assert_hamiltonian(&g, &tour);
        }
    }

    #[test]
    fn star() {
        let g = star_graph();
        let tour = TwoOptHeuristicTSP::new().get_tour(&g).unwrap().unwrap();
        assert_hamiltonian(&g, &tour);
        assert!(2.0 * mst_weight(&g) >= tour.weight());
    }

    #[test]
    fn invalid_instance_not_complete() {
        let g = weighted_str(&["A", "B", "C"], &[("A", "B", 20.0), ("A", "C", 42.0)]);
        assert!(TwoOptHeuristicTSP::new().get_tour(&g).is_err());
    }

    struct Fixed(GraphWalk<i32, DefaultEdge>);

    impl HamiltonianCycleAlgorithm<Ug> for Fixed {
        fn get_tour(&mut self, _graph: &Ug) -> Result<Option<GraphWalk<i32, DefaultEdge>>> {
            Ok(Some(self.0.clone()))
        }
    }

    #[test]
    fn equivalent_moves_do_not_cycle() {
        for large_weight in [1.0, 1e16, 1e30, 1.8e307] {
            for construct_tour in [false, true] {
                let mut graph = Ug::simple().weighted();
                for i in 0..4 {
                    graph.add_vertex(i);
                }
                let weights = [
                    [0.0, large_weight, large_weight, 0.0],
                    [large_weight, 0.0, large_weight, 1.0],
                    [large_weight, large_weight, 0.0, 1.0],
                    [0.0, 1.0, 1.0, 0.0],
                ];
                for i in 0..4 {
                    for j in i + 1..4 {
                        graph
                            .add_weighted_edge(&(i as i32), &(j as i32), weights[i][j])
                            .unwrap();
                    }
                }
                let initial = GraphWalk::from_vertex_list(
                    &graph,
                    vec![0, 1, 2, 3, 0],
                    2.0 * large_weight + 1.0,
                )
                .unwrap();
                let mut algorithm = TwoOptHeuristicTSP::with_initializer(Fixed(initial.clone()));
                let improved = if construct_tour {
                    algorithm.get_tour(&graph).unwrap().unwrap()
                } else {
                    algorithm.improve_tour(&graph, &initial).unwrap()
                };
                assert_hamiltonian(&graph, &improved);
                assert_eq!(initial.weight(), improved.weight());
            }
        }
    }
}

// HeldKarpTSPTest

mod held_karp_tsp_test {
    use super::*;

    fn directed(vertices: &[&str], edges: &[(&str, &str, f64)]) -> DiGraph<String> {
        let mut g = DiGraph::<String>::simple().weighted();
        for v in vertices {
            g.add_vertex(s(v));
        }
        for (a, b, w) in edges {
            g.add_weighted_edge(&s(a), &s(b), *w).unwrap();
        }
        g
    }

    fn check<G: Graph>(g: &G, weight: f64)
    where
        G::Vertex: std::fmt::Debug,
    {
        let tour = HeldKarpTSP::new().get_tour(g).unwrap().expect("a tour");
        assert_hamiltonian(g, &tour);
        assert!(
            (tour.weight() - weight).abs() <= 1e-9,
            "{} vs {weight}",
            tour.weight()
        );
    }

    #[test]
    fn directed_graph() {
        let g = directed(
            &["0", "1", "2", "3", "4"],
            &[
                ("0", "1", 9.0),
                ("0", "3", 8.0),
                ("1", "0", 7.0),
                ("1", "2", 1.0),
                ("1", "4", 3.0),
                ("2", "0", 5.0),
                ("2", "4", 4.0),
                ("3", "2", 6.0),
                ("4", "3", 7.0),
                ("4", "1", 1.0),
            ],
        );
        check(&g, 26.0);
    }

    #[test]
    fn directed_graph2() {
        let g = directed(
            &["0", "1", "2", "3"],
            &[
                ("1", "3", 578985.0),
                ("1", "2", 316670.0),
                ("2", "3", 121118.0),
                ("3", "2", 585978.0),
                ("0", "1", 220022.0),
                ("2", "1", 62190.0),
                ("0", "3", 599952.0),
                ("3", "1", 540561.0),
                ("0", "2", 960850.0),
                ("2", "0", 781797.0),
            ],
        );
        check(&g, 2166782.0);
    }

    #[test]
    fn undirected_graph() {
        let g = weighted_str(
            &["1", "2", "3", "4"],
            &[
                ("1", "2", 10.0),
                ("1", "3", 15.0),
                ("1", "4", 20.0),
                ("2", "3", 35.0),
                ("2", "4", 25.0),
                ("3", "4", 30.0),
            ],
        );
        check(&g, 80.0);
    }

    #[test]
    fn undirected_graph2() {
        let weights = [
            [0, 8, 7, 5, 6],
            [8, 0, 3, 1, 7],
            [7, 3, 0, 8, 6],
            [5, 1, 8, 0, 1],
            [6, 7, 6, 1, 0],
        ];
        let mut g = Ug::simple().weighted();
        for i in 0..5 {
            g.add_vertex(i);
        }
        for i in 0..5 {
            for j in i + 1..5 {
                g.add_weighted_edge(&i, &j, weights[i as usize][j as usize] as f64)
                    .unwrap();
            }
        }
        check(&g, 18.0);
    }

    #[test]
    fn directed_weighted_pseudograph() {
        let weights = [
            [0, 9, 3, 3, 7],
            [9, 0, 10, 7, 5],
            [3, 10, 0, 1, 1],
            [3, 7, 1, 0, 10],
            [7, 5, 1, 10, 0],
        ];
        let mut g = Dg::pseudo().weighted();
        for i in 0..5 {
            g.add_vertex(i);
        }
        for i in 0..5 {
            for j in 0..5 {
                g.add_weighted_edge(&i, &j, weights[i as usize][j as usize] as f64)
                    .unwrap();
            }
        }
        check(&g, 19.0);
    }

    #[test]
    fn wiki_example_symmetric_4_cities() {
        check(&wiki_four_cities(), 97.0);
    }

    #[test]
    fn no_solution_directed_graph() {
        let g = directed(
            &["0", "1", "2", "3"],
            &[
                ("2", "1", 200526.0),
                ("1", "3", 427820.0),
                ("3", "1", 375699.0),
                ("3", "2", 541104.0),
                ("0", "2", 311063.0),
            ],
        );
        assert!(HeldKarpTSP::new().get_tour(&g).unwrap().is_none());
    }

    #[test]
    fn no_solution_undirected_graph() {
        let g = weighted_str(
            &["0", "1", "2", "3"],
            &[("0", "1", 1.0), ("0", "2", 1.0), ("0", "3", 1.0)],
        );
        assert!(HeldKarpTSP::new().get_tour(&g).unwrap().is_none());
    }

    #[test]
    fn invalid_instance_empty() {
        assert!(
            HeldKarpTSP::new()
                .get_tour(&DiGraph::<String>::simple())
                .is_err()
        );
    }

    #[test]
    fn one_vertex_graph() {
        let mut g = UGraph::<String>::simple().weighted();
        g.add_vertex(s("A"));
        let tour = HeldKarpTSP::new().get_tour(&g).unwrap().unwrap();
        assert_hamiltonian(&g, &tour);
    }

    #[test]
    fn random_graphs() {
        use rand::Rng;
        let mut random = StdRng::seed_from_u64(123);
        for _ in 0..500 {
            let mut g = DiGraph::<String>::multi();
            let n = 2 + random.random_range(0..19);
            for i in 0..n {
                g.add_vertex(i.to_string());
            }
            for i in 0..n - 1 {
                g.add_edge(&i.to_string(), &(i + 1).to_string()).unwrap();
            }
            g.add_edge(&(n - 1).to_string(), &0.to_string()).unwrap();
            let m = n * (1 + random.random_range(0..6));
            for _ in 0..m {
                let u = random.random_range(0..n);
                let v = random.random_range(0..n);
                if u != v {
                    g.add_edge(&u.to_string(), &v.to_string()).unwrap();
                }
            }
            let tour = HeldKarpTSP::new().get_tour(&g).unwrap().expect("a tour");
            assert_hamiltonian(&g, &tour);
        }
    }
}

// PalmerHamiltonianCycleTest

mod palmer_hamiltonian_cycle_test {
    use super::*;
    use jgrapht::graph::props::has_ore_property;
    use jgrapht::util::java::java_shuffle;
    use rand::Rng;

    #[test]
    fn small_graph() {
        let mut graph = UGraph::<String>::simple();
        for v in ["A", "B", "C", "D"] {
            graph.add_vertex(s(v));
        }
        for (a, b) in [("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")] {
            graph.add_edge(&s(a), &s(b)).unwrap();
        }
        let tour = PalmerHamiltonianCycle::new()
            .get_tour(&graph)
            .unwrap()
            .unwrap();
        assert_hamiltonian(&graph, &tour);
    }

    #[test]
    fn line_graph() {
        let edges: Vec<(i32, i32)> = (0..10).map(|i| (i, (i + 1) % 10)).collect();
        assert!(
            PalmerHamiltonianCycle::new()
                .get_tour(&ug(10, &edges))
                .is_err()
        );
    }

    fn add_until_ore(graph: &mut Ug, n: i32, random: &mut StdRng) {
        let mut vertex_list: Vec<i32> = graph.vertices_cloned().collect();
        while !has_ore_property(graph) {
            java_shuffle(&mut vertex_list, random);
            'search: for i in 0..vertex_list.len() {
                for j in i + 1..vertex_list.len() {
                    let (u, v) = (vertex_list[i], vertex_list[j]);
                    if !graph.contains_edge_between(&u, &v)
                        && (graph.degree_of(&u) + graph.degree_of(&v)) < n as usize
                    {
                        graph.add_edge(&u, &v).unwrap();
                        break 'search;
                    }
                }
            }
        }
    }

    #[test]
    fn random_graphs() {
        let mut random = StdRng::seed_from_u64(0xC0FFEE);
        for _ in 0..100 {
            let n = 3 + random.random_range(0..40);
            let mut graph = ug(n, &[]);
            add_until_ore(&mut graph, n, &mut random);
            let tour = PalmerHamiltonianCycle::new()
                .get_tour(&graph)
                .unwrap()
                .unwrap();
            assert_hamiltonian(&graph, &tour);
        }
    }

    #[test]
    fn random_graphs2_fixed_seed() {
        let mut random = StdRng::seed_from_u64(0xBEEF);
        for _ in 0..100 {
            let n = 3 + random.random_range(0..40);
            let mut graph = ug(n, &[]);
            let mut vertex_list: Vec<i32> = graph.vertices_cloned().collect();
            loop {
                let mut changed = false;
                java_shuffle(&mut vertex_list, &mut random);
                'search: for &v in &vertex_list {
                    if graph.degree_of(&v) < ((n + 1) / 2) as usize {
                        for &u in &vertex_list {
                            if u != v && !graph.contains_edge_between(&u, &v) {
                                graph.add_edge(&u, &v).unwrap();
                                changed = true;
                                break 'search;
                            }
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
            let tour = PalmerHamiltonianCycle::new()
                .get_tour(&graph)
                .unwrap()
                .unwrap();
            assert_hamiltonian(&graph, &tour);
        }
    }

    #[test]
    fn big_graph() {
        let mut random = StdRng::seed_from_u64(0xC0FFEE);
        let n = 200;
        let mut big = ug(n, &[]);
        add_until_ore(&mut big, n, &mut random);
        assert!(has_ore_property(&big));
        let tour = PalmerHamiltonianCycle::new()
            .get_tour(&big)
            .unwrap()
            .unwrap();
        assert_hamiltonian(&big, &tour);
    }
}

// FarthestInsertionHeuristicTSPTest

mod farthest_insertion_heuristic_tsp_test {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = Ug::with_loops().weighted();
        assert!(FarthestInsertionHeuristicTSP::new().get_tour(&g).is_err());
    }

    #[test]
    fn no_complete_graph() {
        let mut g = Ug::with_loops().weighted();
        g.add_vertex(0);
        g.add_vertex(1);
        assert!(FarthestInsertionHeuristicTSP::new().get_tour(&g).is_err());
    }

    fn weighted_edges(edges: &[(i32, i32, f64)]) -> Ug {
        let mut g = Ug::simple().weighted();
        for (a, b, w) in edges {
            g.add_vertex(*a);
            g.add_vertex(*b);
            g.add_weighted_edge(a, b, *w).unwrap();
        }
        g
    }

    #[test]
    fn get_tour1() {
        let g = weighted_edges(&[(1, 2, 5.0)]);
        let tour = FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert_hamiltonian(&g, &tour);
        assert!((10.0 - tour.weight()).abs() <= 1e-9);
    }

    #[test]
    fn get_tour2() {
        let g = weighted_edges(&[(1, 2, 5.0), (1, 3, 5.0), (2, 3, 9.0)]);
        let tour = FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert_hamiltonian(&g, &tour);
        assert!((19.0 - tour.weight()).abs() <= 1e-9);
    }

    fn dist5() -> Ug {
        matrix_graph(&[
            &[0, 8, 10, 11, 15],
            &[8, 0, 2, 3, 7],
            &[10, 2, 0, 1, 5],
            &[11, 3, 1, 0, 4],
            &[15, 7, 5, 4, 0],
        ])
    }

    #[test]
    fn dummy_graph5() {
        let g = dist5();
        let tour = FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert!((30.0 - tour.weight()).abs() <= 1e-9);
        assert_eq!(&[3, 2, 1, 0, 4, 3], tour.vertex_list());
    }

    #[test]
    fn dummy_graph5_with_subtour() {
        let g = dist5();
        let subtour = GraphWalk::from_vertex_list(&g, vec![3, 2, 0, 4], -1.0).unwrap();
        let tour = FarthestInsertionHeuristicTSP::with_subtour(subtour)
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert!((30.0 - tour.weight()).abs() <= 1e-9);
        assert_eq!(&[3, 2, 1, 0, 4, 3], tour.vertex_list());
    }

    #[test]
    fn dummy_graph10() {
        let g = matrix_graph(&[
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            &[1, 0, 10, 11, 12, 13, 14, 15, 16, 17],
            &[2, 10, 0, 18, 19, 20, 21, 22, 23, 24],
            &[3, 11, 18, 0, 25, 26, 27, 28, 29, 30],
            &[4, 12, 19, 25, 0, 31, 32, 33, 34, 35],
            &[5, 13, 20, 26, 31, 0, 36, 37, 38, 39],
            &[6, 14, 21, 27, 32, 36, 0, 40, 41, 42],
            &[7, 15, 22, 28, 33, 37, 40, 0, 43, 44],
            &[8, 16, 23, 29, 34, 38, 41, 43, 0, 45],
            &[9, 17, 24, 30, 35, 39, 42, 44, 45, 0],
        ]);
        let tour = FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert!((210.0 - tour.weight()).abs() <= 1e-9);
        assert_eq!(&[4, 5, 1, 6, 0, 7, 3, 8, 2, 9, 4], tour.vertex_list());
    }

    #[test]
    fn negative_edge_weights() {
        let g = matrix_graph(&[
            &[0, 10, -3, -2],
            &[10, 0, -5, -4],
            &[-3, -5, 0, -1],
            &[-2, -4, -1, 0],
        ]);
        let tour = FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert_hamiltonian(&g, &tour);
    }

    #[test]
    fn negative_edge_weights_all_negative() {
        let g = matrix_graph(&[&[0, -3, -2], &[-3, 0, -1], &[-2, -1, 0]]);
        let tour = FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert_hamiltonian(&g, &tour);
    }

    #[test]
    fn default_subtour_persists_across_calls() {
        // The first graph selects 0--4 (weight 15) as the default initial sub-tour.
        let first = dist5();
        let second = matrix_graph(&[
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            &[1, 0, 10, 11, 12, 13, 14, 15, 16, 17],
            &[2, 10, 0, 18, 19, 20, 21, 22, 23, 24],
            &[3, 11, 18, 0, 25, 26, 27, 28, 29, 30],
            &[4, 12, 19, 25, 0, 31, 32, 33, 34, 35],
            &[5, 13, 20, 26, 31, 0, 36, 37, 38, 39],
            &[6, 14, 21, 27, 32, 36, 0, 40, 41, 42],
            &[7, 15, 22, 28, 33, 37, 40, 0, 43, 44],
            &[8, 16, 23, 29, 34, 38, 41, 43, 0, 45],
            &[9, 17, 24, 30, 35, 39, 42, 44, 45, 0],
        ]);

        let mut reused = FarthestInsertionHeuristicTSP::new();
        reused.get_tour(&first).unwrap();
        let reused_tour = reused.get_tour(&second).unwrap().unwrap();

        let retained = GraphWalk::from_vertex_list(&second, vec![0, 4], -1.0).unwrap();
        let expected = FarthestInsertionHeuristicTSP::with_subtour(retained)
            .get_tour(&second)
            .unwrap()
            .unwrap();
        let fresh = FarthestInsertionHeuristicTSP::new()
            .get_tour(&second)
            .unwrap()
            .unwrap();

        assert_eq!(reused_tour.vertex_list(), expected.vertex_list());
        assert_ne!(reused_tour.vertex_list(), fresh.vertex_list());
    }
}

// NearestNeighborHeuristicTSPTest and GeometricTSPTest

#[derive(Clone, Copy, Debug)]
struct Vector2D(f64, f64);

impl PartialEq for Vector2D {
    fn eq(&self, o: &Self) -> bool {
        self.0.to_bits() == o.0.to_bits() && self.1.to_bits() == o.1.to_bits()
    }
}
impl Eq for Vector2D {}
impl Hash for Vector2D {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.to_bits().hash(h);
        self.1.to_bits().hash(h);
    }
}
impl Vector2D {
    fn distance(&self, o: &Vector2D) -> f64 {
        ((self.0 - o.0).powi(2) + (self.1 - o.1).powi(2)).sqrt()
    }
}

fn geometric_graph(points: &[Vector2D]) -> UGraph<Vector2D> {
    let mut g = UGraph::<Vector2D>::simple().weighted();
    for p in points {
        g.add_vertex(*p);
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            g.add_weighted_edge(&points[i], &points[j], points[i].distance(&points[j]))
                .unwrap();
        }
    }
    g
}

mod nearest_neighbor_heuristic_tsp_test {
    use super::*;

    fn locations() -> Vec<Vector2D> {
        [
            (235, 170),
            (326, 212),
            (215, 430),
            (511, 693),
            (806, 463),
            (504, 62),
            (434, 742),
            (487, 614),
            (719, 147),
            (182, 449),
        ]
        .iter()
        .map(|&(x, y)| Vector2D(x as f64, y as f64))
        .collect()
    }

    #[test]
    fn constructor_with_random_number_generator() {
        let locs = locations();
        let g = geometric_graph(&locs);
        let tours = g.vertex_count();
        let mut test_rnd = StdRng::seed_from_u64(0);
        let mut alg = NearestNeighborHeuristicTSP::with_rng(StdRng::seed_from_u64(0));
        for _ in 0..tours {
            let expected_start = locs[test_rnd.random_range(0..tours)];
            let tour = alg.get_tour(&g).unwrap().unwrap();
            assert_eq!(Some(&expected_start), tour.start_vertex());
        }
    }

    #[test]
    fn constructor_with_first() {
        let locs = locations();
        let g = geometric_graph(&locs);
        let mut alg = NearestNeighborHeuristicTSP::with_first(locs[2]);
        let tour = alg.get_tour(&g).unwrap().unwrap();
        assert_eq!(Some(&locs[2]), tour.start_vertex());
    }

    #[test]
    fn constructor_with_initial_vertices() {
        let locs = locations();
        let g = geometric_graph(&locs);
        let mut initial = locs.clone();
        jgrapht::util::java::java_shuffle(
            &mut initial,
            &mut StdRng::seed_from_u64(0x004A_4772_6170_6854),
        );
        let mut alg = NearestNeighborHeuristicTSP::with_initial_vertices(initial.clone());
        for expected in &initial {
            let tour = alg.get_tour(&g).unwrap().unwrap();
            assert_eq!(Some(expected), tour.start_vertex());
        }
    }

    #[test]
    fn get_tour() {
        let locs = locations();
        let g = geometric_graph(&locs);
        let expected: [[usize; 10]; 10] = [
            [0, 1, 5, 8, 4, 7, 3, 6, 2, 9],
            [1, 0, 2, 9, 7, 3, 6, 4, 8, 5],
            [2, 9, 1, 0, 5, 8, 4, 7, 3, 6],
            [3, 7, 6, 2, 9, 1, 0, 5, 8, 4],
            [4, 8, 5, 1, 0, 2, 9, 7, 3, 6],
            [5, 8, 4, 7, 3, 6, 2, 9, 1, 0],
            [6, 3, 7, 2, 9, 1, 0, 5, 8, 4],
            [7, 3, 6, 2, 9, 1, 0, 5, 8, 4],
            [8, 5, 1, 0, 2, 9, 7, 3, 6, 4],
            [9, 2, 1, 0, 5, 8, 4, 7, 3, 6],
        ];
        for (i, start) in locs.iter().enumerate() {
            let tour = NearestNeighborHeuristicTSP::with_first(*start)
                .get_tour(&g)
                .unwrap()
                .unwrap();
            let mut vertices: Vec<Vector2D> = expected[i].iter().map(|&k| locs[k]).collect();
            vertices.push(vertices[0]);
            assert_eq!(vertices, tour.vertex_list());
        }
    }
}

mod geometric_tsp_test {
    use super::*;
    use rand::Rng;

    fn graphs() -> Vec<UGraph<Vector2D>> {
        let mut rng = StdRng::seed_from_u64(42);
        [1usize, 10, 100, 300]
            .iter()
            .map(|&size| {
                let points: Vec<Vector2D> = (0..size)
                    .map(|_| Vector2D(rng.random_range(0.0..100.0), rng.random_range(0.0..100.0)))
                    .collect();
                geometric_graph(&points)
            })
            .collect()
    }

    fn test_with(mut algorithm: impl HamiltonianCycleAlgorithm<UGraph<Vector2D>>) {
        for g in graphs() {
            let tour = algorithm.get_tour(&g).unwrap().unwrap();
            assert_hamiltonian(&g, &tour);
        }
    }

    #[test]
    fn greedy() {
        test_with(GreedyHeuristicTSP::new());
    }

    #[test]
    fn nearest_insertion_heuristic() {
        test_with(NearestInsertionHeuristicTSP::new());
    }

    #[test]
    fn nearest_neighbour_heuristic() {
        test_with(NearestNeighborHeuristicTSP::new());
    }

    #[test]
    fn random() {
        test_with(RandomTourTSP::new());
    }

    #[test]
    fn two_opt_nearest_neighbour() {
        test_with(TwoOptHeuristicTSP::with_initializer(
            NearestNeighborHeuristicTSP::new(),
        ));
    }

    #[test]
    fn two_opt1() {
        test_with(TwoOptHeuristicTSP::with_passes(1));
    }
}
