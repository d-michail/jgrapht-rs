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

use super::*;

#[derive(Clone, Copy)]
enum Solver {
    Suurballe,
    Bhandari,
}

fn solve(
    solver: Solver,
    graph: &DiGraph<i32>,
    source: i32,
    target: i32,
    k: usize,
) -> Result<Vec<GraphWalk<i32, DefaultEdge>>> {
    match solver {
        Solver::Suurballe => {
            SuurballeKDisjointShortestPaths::new(graph).get_paths(&source, &target, k)
        }
        Solver::Bhandari => {
            BhandariKDisjointShortestPaths::new(graph).get_paths(&source, &target, k)
        }
    }
}

fn graph(vertices: impl IntoIterator<Item = i32>, edges: &[(i32, i32, f64)]) -> DiGraph<i32> {
    let mut graph = DiGraph::simple().weighted();
    graph.add_vertices(vertices);
    for &(source, target, weight) in edges {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn assert_paths(paths: &[GraphWalk<i32, DefaultEdge>], expected: &[(&[i32], f64)]) {
    assert_eq!(paths.len(), expected.len());
    for (path, (vertices, weight)) in paths.iter().zip(expected) {
        assert_eq!(path.vertex_list(), *vertices);
        assert_eq!(path.length(), vertices.len() - 1);
        assert_eq!(path.weight(), *weight);
    }
}

fn single_path(solver: Solver) {
    let graph = graph(1..=2, &[(1, 2, 8.0)]);
    let paths = solve(solver, &graph, 1, 2, 5).unwrap();
    assert_paths(&paths, &[(&[1, 2], 8.0)]);
}

fn two_disjoint_paths_joint_node(solver: Solver) {
    let graph = graph(
        1..=7,
        &[
            (1, 2, 1.0),
            (1, 7, 1.0),
            (2, 3, 1.0),
            (7, 3, 1.0),
            (3, 4, 1.0),
            (3, 6, 1.0),
            (4, 5, 1.0),
            (6, 5, 1.0),
        ],
    );
    let paths = solve(solver, &graph, 1, 5, 2).unwrap();
    assert_eq!(paths.len(), 2);
    assert!(
        paths
            .iter()
            .all(|path| path.length() == 4 && path.weight() == 4.0)
    );
    let choices = [
        vec![1, 2, 3, 4, 5],
        vec![1, 2, 3, 6, 5],
        vec![1, 7, 3, 4, 5],
        vec![1, 7, 3, 6, 5],
    ];
    assert!(choices.contains(&paths[0].vertex_list().to_vec()));
    assert!(choices.contains(&paths[1].vertex_list().to_vec()));
    assert_ne!(paths[0].edge_list(), paths[1].edge_list());
    assert!(
        paths[0]
            .edge_list()
            .iter()
            .all(|edge| !paths[1].edge_list().contains(edge))
    );
}

fn two_disjoint_paths(solver: Solver) {
    let graph = graph(1..=3, &[(1, 2, 1.0), (2, 3, 1.0), (1, 3, 1.0)]);
    let paths = solve(solver, &graph, 1, 3, 5).unwrap();
    assert_paths(&paths, &[(&[1, 3], 1.0), (&[1, 2, 3], 2.0)]);
}

fn disconnected_graph() -> DiGraph<i32> {
    graph(
        1..=6,
        &[
            (1, 2, 2.0),
            (2, 1, 2.0),
            (2, 3, 1.0),
            (3, 2, 1.0),
            (3, 1, 4.0),
            (1, 3, 4.0),
            (4, 5, 7.0),
            (5, 6, 8.0),
            (6, 4, 9.0),
        ],
    )
}

fn disconnected(solver: Solver) {
    let graph = disconnected_graph();
    let paths = solve(solver, &graph, 1, 3, 5).unwrap();
    assert_paths(&paths, &[(&[1, 2, 3], 3.0), (&[1, 3], 4.0)]);
}

fn merge_graph(weights: [f64; 7], reversed: bool) -> DiGraph<i32> {
    let endpoints = [(1, 2), (2, 3), (3, 4), (1, 5), (5, 3), (2, 6), (6, 4)];
    let mut edges: Vec<_> = endpoints
        .iter()
        .zip(weights)
        .map(|(&(source, target), weight)| (source, target, weight))
        .collect();
    if reversed {
        edges.extend(
            endpoints
                .iter()
                .zip(weights)
                .map(|(&(source, target), weight)| (target, source, weight)),
        );
    }
    graph(1..=6, &edges)
}

fn two_paths_no_merge(solver: Solver) {
    let graph = merge_graph([1.0, 3.0, 2.0, 2.0, 2.0, 1.0, 1.0], false);
    let paths = solve(solver, &graph, 1, 4, 5).unwrap();
    assert_paths(&paths, &[(&[1, 2, 6, 4], 3.0), (&[1, 5, 3, 4], 6.0)]);
}

fn two_paths_need_merge(solver: Solver, reversed: bool) {
    let graph = merge_graph([1.0, 1.0, 1.0, 3.0, 3.0, 2.0, 2.0], reversed);
    let paths = solve(solver, &graph, 1, 4, 5).unwrap();
    assert_paths(&paths, &[(&[1, 2, 6, 4], 5.0), (&[1, 5, 3, 4], 7.0)]);
}

fn three_paths_graph(bidirectional: bool) -> DiGraph<i32> {
    let base = [
        (1, 2, 1.0),
        (2, 5, 6.0),
        (1, 3, 4.0),
        (3, 5, 5.0),
        (1, 4, 4.0),
        (4, 5, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
    ];
    let mut edges = base.to_vec();
    if bidirectional {
        edges.extend(
            base.into_iter()
                .map(|(source, target, weight)| (target, source, weight)),
        );
    }
    graph(1..=5, &edges)
}

fn three_disjoint_paths(solver: Solver, bidirectional: bool) {
    let graph = three_paths_graph(bidirectional);
    let paths = solve(solver, &graph, 1, 5, 5).unwrap();
    assert_paths(
        &paths,
        &[(&[1, 4, 5], 5.0), (&[1, 2, 5], 7.0), (&[1, 3, 5], 9.0)],
    );
}

fn maximum_k_paths(solver: Solver) {
    let graph = three_paths_graph(false);
    for k in 1..=3 {
        assert_eq!(solve(solver, &graph, 1, 5, k).unwrap().len(), k);
    }
}

fn sequential_calls(solver: Solver) {
    let graph = graph(1..=2, &[(1, 2, 8.0)]);
    let first = solve(solver, &graph, 1, 2, 5).unwrap();
    let second = solve(solver, &graph, 1, 2, 5).unwrap();
    assert_eq!(first, second);
}

fn snapshot(graph: &DiGraph<i32>) -> Vec<(DefaultEdge, i32, i32, f64)> {
    graph
        .edges()
        .map(|edge| {
            (
                *edge,
                *graph.edge_source(edge).unwrap(),
                *graph.edge_target(edge).unwrap(),
                graph.edge_weight(edge).unwrap(),
            )
        })
        .collect()
}

fn graph_is_not_changed(solver: Solver, graph: DiGraph<i32>, source: i32, target: i32) {
    let before = snapshot(&graph);
    solve(solver, &graph, source, target, 5).unwrap();
    assert_eq!(snapshot(&graph), before);
}

fn unweighted_graph() -> DiGraph<i32> {
    graph(
        1..=8,
        &[
            (1, 2, 1.0),
            (2, 5, 1.0),
            (1, 3, 1.0),
            (3, 6, 1.0),
            (6, 5, 1.0),
            (1, 4, 1.0),
            (4, 7, 1.0),
            (7, 8, 1.0),
            (8, 5, 1.0),
            (2, 3, 1.0),
            (3, 4, 1.0),
        ],
    )
}

fn unweighted(solver: Solver) {
    let graph = unweighted_graph();
    let paths = solve(solver, &graph, 1, 5, 5).unwrap();
    assert_paths(
        &paths,
        &[
            (&[1, 2, 5], 2.0),
            (&[1, 3, 6, 5], 3.0),
            (&[1, 4, 7, 8, 5], 4.0),
        ],
    );
}

fn wikipedia(solver: Solver) {
    let graph = graph(
        1..=6,
        &[
            (1, 2, 1.0),
            (2, 1, 1.0),
            (1, 3, 2.0),
            (3, 1, 2.0),
            (2, 4, 1.0),
            (4, 2, 1.0),
            (2, 5, 2.0),
            (5, 2, 2.0),
            (4, 3, 2.0),
            (3, 4, 2.0),
            (4, 6, 1.0),
            (6, 4, 1.0),
            (5, 6, 2.0),
            (6, 5, 2.0),
        ],
    );
    let paths = solve(solver, &graph, 1, 6, 3).unwrap();
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|path| path.weight() == 5.0));
    let vertices: Vec<_> = paths.iter().map(|path| path.vertex_list().to_vec()).collect();
    assert!(vertices.contains(&vec![1, 3, 4, 6]));
    assert!(vertices.contains(&vec![1, 2, 5, 6]));
}

fn linear(solver: Solver) {
    let edges: Vec<_> = (1..20).map(|source| (source, source + 1, 1.0)).collect();
    let graph = graph(1..=20, &edges);
    let paths = solve(solver, &graph, 1, 20, 2).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].length(), 19);
    assert_eq!(paths[0].weight(), 19.0);
}

fn ring(solver: Solver) {
    let mut edges: Vec<_> = (1..20).map(|source| (source, source + 1, 1.0)).collect();
    edges.push((20, 1, 1.0));
    let graph = graph(1..=20, &edges);
    let paths = solve(solver, &graph, 1, 10, 2).unwrap();
    assert_paths(&paths, &[(&(1..=10).collect::<Vec<_>>(), 9.0)]);
}

fn clique(solver: Solver) {
    let mut edges = Vec::new();
    for source in 1..=20 {
        for target in 1..=20 {
            if source != target {
                edges.push((source, target, 1.0));
            }
        }
    }
    let graph = graph(1..=20, &edges);
    for target in 2..20 {
        assert_eq!(solve(solver, &graph, 1, target, 2).unwrap().len(), 2);
    }
}

fn star(solver: Solver) {
    let edges: Vec<_> = (2..=20).map(|leaf| (leaf, 1, 1.0)).collect();
    let graph = graph(1..=20, &edges);
    for source in 2..20 {
        assert_eq!(solve(solver, &graph, source, 1, 2).unwrap().len(), 1);
    }
}

fn multi_hits_on_edge(solver: Solver) {
    let graph = graph(
        1..=8,
        &[
            (1, 3, 1.0),
            (1, 4, 1.0),
            (1, 5, 1.0),
            (3, 6, 2.0),
            (3, 7, 1.0),
            (4, 7, 6.0),
            (4, 6, 5.0),
            (5, 7, 3.0),
            (5, 8, 6.0),
            (6, 2, 1.0),
            (7, 2, 1.0),
            (8, 2, 1.0),
        ],
    );
    let paths = solve(solver, &graph, 1, 2, 3).unwrap();
    let vertices: Vec<_> = paths.iter().map(|path| path.vertex_list().to_vec()).collect();
    assert_eq!(paths.len(), 3);
    assert!(vertices.contains(&vec![1, 3, 7, 2]));
    assert!(vertices.contains(&vec![1, 4, 6, 2]));
    assert!(vertices.contains(&vec![1, 5, 8, 2]));
}

fn first_path_edges_first(solver: Solver) {
    let pairs = [
        (1, 2),
        (2, 3),
        (1, 4),
        (1, 5),
        (2, 5),
        (2, 6),
        (3, 6),
        (3, 7),
        (4, 5),
        (6, 7),
        (5, 8),
        (6, 9),
        (8, 9),
    ];
    let mut edges = Vec::new();
    for (source, target) in pairs {
        edges.push((source, target, 1.0));
        edges.push((target, source, 1.0));
    }
    let graph = graph(1..=9, &edges);
    let paths = solve(solver, &graph, 1, 3, 3).unwrap();
    assert_paths(
        &paths,
        &[
            (&[1, 2, 3], 2.0),
            (&[1, 5, 2, 6, 3], 4.0),
            (&[1, 4, 5, 8, 9, 6, 7, 3], 7.0),
        ],
    );
}

macro_rules! shared_suite {
    ($module:ident, $solver:expr) => {
        mod $module {
            use super::*;
            const SOLVER: Solver = $solver;

            #[test]
            fn single_path() {
                super::single_path(SOLVER);
            }
            #[test]
            fn two_disjoint_paths_joint_node() {
                super::two_disjoint_paths_joint_node(SOLVER);
            }
            #[test]
            fn two_disjoint_paths() {
                super::two_disjoint_paths(SOLVER);
            }
            #[test]
            fn disconnected_graph() {
                super::disconnected(SOLVER);
            }
            #[test]
            fn two_disjoint_paths_no_need_to_merge() {
                super::two_paths_no_merge(SOLVER);
            }
            #[test]
            fn two_disjoint_paths_need_to_merge() {
                super::two_paths_need_merge(SOLVER, false);
            }
            #[test]
            fn two_disjoint_paths_with_reversed_edges_exist() {
                super::two_paths_need_merge(SOLVER, true);
            }
            #[test]
            fn three_disjoint_paths() {
                super::three_disjoint_paths(SOLVER, false);
            }
            #[test]
            fn three_disjoint_paths_reverse_edge_exists() {
                super::three_disjoint_paths(SOLVER, true);
            }
            #[test]
            fn maximum_k_paths_are_returned() {
                super::maximum_k_paths(SOLVER);
            }
            #[test]
            fn sequential_calls_sanity() {
                super::sequential_calls(SOLVER);
            }
            #[test]
            fn three_disjoint_paths_graph_is_not_changed() {
                super::graph_is_not_changed(SOLVER, super::three_paths_graph(false), 1, 5);
            }
            #[test]
            fn disconnected_graph_is_not_changed() {
                super::graph_is_not_changed(SOLVER, super::disconnected_graph(), 1, 5);
            }
            #[test]
            fn unweighted_graph_is_not_changed() {
                super::graph_is_not_changed(SOLVER, super::unweighted_graph(), 1, 5);
            }
            #[test]
            fn unweighted_graph() {
                super::unweighted(SOLVER);
            }
            #[test]
            fn wikipedia_graph() {
                super::wikipedia(SOLVER);
            }
            #[test]
            fn linear() {
                super::linear(SOLVER);
            }
            #[test]
            fn ring() {
                super::ring(SOLVER);
            }
            #[test]
            fn clique() {
                super::clique(SOLVER);
            }
            #[test]
            fn star() {
                super::star(SOLVER);
            }
            #[test]
            fn three_disjoint_paths_with_multi_hits_on_edge() {
                super::multi_hits_on_edge(SOLVER);
            }
            #[test]
            fn first_path_edges_first() {
                super::first_path_edges_first(SOLVER);
            }
        }
    };
}

shared_suite!(suurballe, Solver::Suurballe);
shared_suite!(bhandari, Solver::Bhandari);

#[test]
fn suurballe_fractional_edge_weight() {
    let graph = graph(1..=3, &[(1, 2, 1.0), (1, 3, 2.0), (2, 3, 0.9)]);
    let paths = solve(Solver::Suurballe, &graph, 1, 3, 2).unwrap();
    assert_paths(&paths, &[(&[1, 2, 3], 1.9), (&[1, 3], 2.0)]);
}

#[test]
fn bhandari_negative_edges() {
    let graph = graph(
        1..=6,
        &[
            (1, 2, -20.0),
            (2, 3, -1.0),
            (3, 4, -10.0),
            (1, 5, -2.0),
            (5, 3, 2.0),
            (2, 6, -3.0),
            (6, 4, 3.0),
        ],
    );
    let paths = solve(Solver::Bhandari, &graph, 1, 4, 5).unwrap();
    assert_paths(&paths, &[(&[1, 2, 6, 4], -20.0), (&[1, 5, 3, 4], -10.0)]);
}
