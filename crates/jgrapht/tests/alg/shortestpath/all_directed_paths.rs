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
use jgrapht::graph::OutgoingGraph;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

const I1: &str = "I1";
const I2: &str = "I2";
const A: &str = "A";
const B: &str = "B";
const C: &str = "C";
const D: &str = "D";
const E: &str = "E";
const F: &str = "F";
const O1: &str = "O1";
const O2: &str = "O2";

fn toy_graph() -> DiGraph<&'static str> {
    let mut graph = DiGraph::simple();
    graph.add_vertices([I1, I2, A, B, C, D, E, F, O1, O2]);
    for (source, target) in [
        (I1, A),
        (I1, B),
        (I2, B),
        (I2, C),
        (A, B),
        (A, D),
        (A, E),
        (B, E),
        (C, B),
        (C, F),
        (D, E),
        (E, O1),
        (F, O2),
    ] {
        graph.add_edge(&source, &target).unwrap();
    }
    graph
}

#[test]
fn small_example_graph() {
    let graph = toy_graph();
    let paths = AllDirectedPaths::new(&graph)
        .get_all_paths_between([&I1, &I2], [&O1, &O2], true, None)
        .unwrap();
    assert_eq!(paths.len(), 7);
}

#[test]
fn small_example_graph_with_path_validator() {
    let graph = toy_graph();
    let validator = |_: &GraphWalk<&str, DefaultEdge>, edge: &DefaultEdge| {
        graph.edge_target(edge).copied() != Some(B)
    };
    let paths = AllDirectedPaths::with_validator(&graph, validator)
        .get_all_paths_between([&I1, &I2], [&O1, &O2], true, None)
        .unwrap();
    assert_eq!(paths.len(), 3);
}

#[test]
fn trivial_paths() {
    let graph = toy_graph();
    let paths = AllDirectedPaths::new(&graph)
        .get_all_paths_between([&I1], [&I1, &A], true, Some(1))
        .unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].vertex_list(), [I1]);
    assert_eq!(paths[1].vertex_list(), [I1, A]);
}

#[test]
fn length_one_paths() {
    let mut graph = DiGraph::simple();
    graph.add_vertices([A, B]);
    graph.add_edge(&B, &A).unwrap();
    let paths = AllDirectedPaths::new(&graph)
        .get_all_paths_between(
            graph.vertices(),
            graph.vertices(),
            true,
            Some(graph.edge_count()),
        )
        .unwrap();
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].vertex_list(), [A]);
    assert_eq!(paths[1].vertex_list(), [B]);
    assert_eq!(paths[2].vertex_list(), [B, A]);
}

#[test]
fn length_one_paths_with_path_validator() {
    let mut graph = DiGraph::simple();
    graph.add_vertices([A, B, C]);
    graph.add_edge(&C, &A).unwrap();
    graph.add_edge(&C, &B).unwrap();
    let validator = |_: &GraphWalk<&str, DefaultEdge>, edge: &DefaultEdge| {
        graph.edge_target(edge).copied() != Some(B)
    };
    let paths = AllDirectedPaths::with_validator(&graph, validator)
        .get_all_paths_between(
            graph.vertices(),
            graph.vertices(),
            true,
            Some(graph.edge_count()),
        )
        .unwrap();
    assert_eq!(paths.len(), 4);
    assert_eq!(paths[0].vertex_list(), [A]);
    assert_eq!(paths[1].vertex_list(), [B]);
    assert_eq!(paths[2].vertex_list(), [C]);
    assert_eq!(paths[3].vertex_list(), [C, A]);
}

#[test]
fn path_weights() {
    let mut graph = DiGraph::simple().weighted();
    graph.add_vertices([A, B, C, D]);
    for (source, target, weight) in [
        (A, B, 1.2),
        (A, C, 0.0),
        (A, D, -1.0),
        (B, C, 2.0),
        (B, D, 1.0),
        (C, D, 0.5),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    let mut paths = AllDirectedPaths::new(&graph)
        .get_all_paths(&A, &D, true, Some(2))
        .unwrap();
    paths.sort_by(|left, right| left.weight().total_cmp(&right.weight()));
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].vertex_list(), [A, D]);
    assert_eq!(paths[0].weight(), -1.0);
    assert_eq!(paths[1].vertex_list(), [A, C, D]);
    assert_eq!(paths[1].weight(), 0.5);
    assert_eq!(paths[2].vertex_list(), [A, B, D]);
    assert_eq!(paths[2].weight(), 2.2);
}

#[test]
fn cycle_behavior() {
    let mut graph = toy_graph();
    graph.add_edge(&D, &A).unwrap();
    let algorithm = AllDirectedPaths::new(&graph);
    let simple = algorithm
        .get_all_paths_between([&I1, &I2], [&O1, &O2], true, Some(8))
        .unwrap();
    let with_cycles = algorithm
        .get_all_paths_between([&I1, &I2], [&O1, &O2], false, Some(8))
        .unwrap();
    assert_eq!(with_cycles.len(), 13);
    assert_eq!(simple.len(), 7);
}

#[test]
fn must_bound_if_non_simple_paths() {
    let graph = toy_graph();
    assert!(
        AllDirectedPaths::new(&graph)
            .get_all_paths(&I1, &O1, false, None)
            .is_err()
    );
}

#[test]
fn target_reachable_but_source_unreachable_branch_is_ignored() {
    let mut graph = DiGraph::simple();
    graph.add_vertices(["S", A, "T", "U", "V"]);
    for (source, target) in [("S", A), (A, "T"), ("U", "T"), ("V", "U")] {
        graph.add_edge(&source, &target).unwrap();
    }
    for forward_pruning in [false, true] {
        let mut algorithm = AllDirectedPaths::new(&graph);
        algorithm.set_forward_pruning(forward_pruning);
        for simple in [false, true] {
            let paths = algorithm
                .get_all_paths(&"S", &"T", simple, Some(5))
                .unwrap();
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].vertex_list(), ["S", A, "T"]);
        }
    }
}

#[test]
fn zero_length_paths() {
    let mut graph = DiGraph::simple();
    graph.add_vertices(["a", "b"]);
    graph.add_edge(&"a", &"b").unwrap();
    let paths = AllDirectedPaths::new(&graph)
        .get_all_paths_between(graph.vertices(), graph.vertices(), false, Some(0))
        .unwrap();
    assert!(!paths.is_empty());
    assert!(paths.iter().all(|path| path.length() == 0));
}

#[test]
fn non_simple_mode_adjacent_vertices_with_self_loops() {
    let mut graph = DiGraph::with_loops();
    graph.add_vertices([0, 1]);
    graph.add_edge(&0, &0).unwrap();
    graph.add_edge(&1, &1).unwrap();
    graph.add_edge(&0, &1).unwrap();
    let paths = AllDirectedPaths::new(&graph)
        .get_all_paths(&0, &1, false, Some(3))
        .unwrap();
    assert_eq!(paths.len(), 6);
}

#[test]
fn non_simple_mode_cycle_without_self_loops() {
    let mut graph = DiGraph::simple();
    graph.add_vertices([0, 1, 2]);
    graph.add_edge(&0, &1).unwrap();
    graph.add_edge(&1, &2).unwrap();
    graph.add_edge(&2, &0).unwrap();
    let paths = AllDirectedPaths::new(&graph)
        .get_all_paths(&0, &1, false, Some(4))
        .unwrap();
    assert_eq!(paths.len(), 2);
}

#[test]
fn non_simple_mode_path_validator_still_respected() {
    let mut graph = DiGraph::with_loops();
    graph.add_vertices([0, 1, 2]);
    graph.add_edge(&0, &0).unwrap();
    graph.add_edge(&0, &1).unwrap();
    graph.add_edge(&1, &2).unwrap();
    graph.add_edge(&2, &2).unwrap();
    let validator = |_: &GraphWalk<i32, DefaultEdge>, edge: &DefaultEdge| {
        graph.edge_target(edge).copied() != Some(1)
    };
    let paths = AllDirectedPaths::with_validator(&graph, validator)
        .get_all_paths(&0, &2, false, Some(3))
        .unwrap();
    assert!(paths.is_empty());
}

#[test]
fn non_simple_mode_max_length_boundary() {
    let mut graph = DiGraph::with_loops();
    graph.add_vertices([0, 1]);
    graph.add_edge(&0, &0).unwrap();
    graph.add_edge(&0, &1).unwrap();
    let algorithm = AllDirectedPaths::new(&graph);
    let zero = algorithm
        .get_all_paths_between([&0], [&0], false, Some(0))
        .unwrap();
    assert_eq!(zero.len(), 1);
    assert_eq!(zero[0].length(), 0);
    let one = algorithm
        .get_all_paths_between([&0], [&0, &1], false, Some(1))
        .unwrap();
    assert_eq!(one.len(), 3);
}

fn random_cyclic_graph(
    seed: i64,
    n: i32,
    edge_probability: f64,
    loop_probability: f64,
) -> DiGraph<i32> {
    let mut rng = StdRng::seed_from_u64(seed as u64);
    let mut graph = DiGraph::with_loops();
    graph.add_vertices(0..n);
    for source in 0..n {
        for target in 0..n {
            let add = if source == target {
                rng.random::<f64>() < loop_probability
            } else {
                rng.random::<f64>() < edge_probability
            };
            if add {
                graph.add_edge(&source, &target).unwrap();
            }
        }
    }
    graph
}

fn brute_force_walks<G>(graph: &G, source: i32, target: i32, max_length: usize) -> Vec<Vec<i32>>
where
    G: DirectedGraph<Vertex = i32> + OutgoingGraph,
{
    fn visit<G>(
        graph: &G,
        target: i32,
        remaining: usize,
        stack: &mut Vec<i32>,
        walks: &mut Vec<Vec<i32>>,
    ) where
        G: DirectedGraph<Vertex = i32> + OutgoingGraph,
    {
        let current = *stack.last().unwrap();
        if current == target {
            walks.push(stack.clone());
        }
        if remaining == 0 {
            return;
        }
        for (_, next) in graph.outgoing_of(&current) {
            stack.push(*next);
            visit(graph, target, remaining - 1, stack, walks);
            stack.pop();
        }
    }
    let mut walks = Vec::new();
    visit(graph, target, max_length, &mut vec![source], &mut walks);
    walks
}

fn brute_force_simple_paths<G>(
    graph: &G,
    source: i32,
    target: i32,
    max_length: usize,
) -> Vec<Vec<i32>>
where
    G: DirectedGraph<Vertex = i32> + OutgoingGraph,
{
    fn visit<G>(
        graph: &G,
        target: i32,
        remaining: usize,
        path: &mut Vec<i32>,
        visited: &mut HashSet<i32>,
        paths: &mut Vec<Vec<i32>>,
    ) where
        G: DirectedGraph<Vertex = i32> + OutgoingGraph,
    {
        let current = *path.last().unwrap();
        if current == target {
            paths.push(path.clone());
        }
        if remaining == 0 {
            return;
        }
        for (_, next) in graph.outgoing_of(&current) {
            if !visited.insert(*next) {
                continue;
            }
            path.push(*next);
            visit(graph, target, remaining - 1, path, visited, paths);
            path.pop();
            visited.remove(next);
        }
    }
    let mut paths = Vec::new();
    let mut visited = HashSet::from([source]);
    visit(
        graph,
        target,
        max_length,
        &mut vec![source],
        &mut visited,
        &mut paths,
    );
    paths
}

fn path_set(paths: &[GraphWalk<i32, DefaultEdge>]) -> HashSet<Vec<i32>> {
    paths
        .iter()
        .map(|path| path.vertex_list().to_vec())
        .collect()
}

#[test]
fn non_simple_mode_matches_brute_force() {
    for seed in [1, 2, 3, 5, 7] {
        let graph = random_cyclic_graph(seed, 5, 0.4, 0.2);
        let expected: HashSet<_> = brute_force_walks(&graph, 0, 4, 4).into_iter().collect();
        let actual = AllDirectedPaths::new(&graph)
            .get_all_paths(&0, &4, false, Some(4))
            .unwrap();
        assert_eq!(actual.len(), expected.len(), "seed={seed}");
        assert_eq!(path_set(&actual), expected, "seed={seed}");
    }
}

#[test]
fn multiple_sources_partial_garden() {
    let mut graph = DiGraph::simple();
    graph.add_vertices(["S1", "S2", A, "T", "U"]);
    for (source, target) in [("S1", A), (A, "T"), ("U", "T")] {
        graph.add_edge(&source, &target).unwrap();
    }
    for forward_pruning in [false, true] {
        let mut algorithm = AllDirectedPaths::new(&graph);
        algorithm.set_forward_pruning(forward_pruning);
        let paths = algorithm
            .get_all_paths_between([&"S1", &"S2"], [&"T"], true, Some(5))
            .unwrap();
        assert_eq!(paths.len(), 1, "forward_pruning={forward_pruning}");
        assert_eq!(paths[0].vertex_list(), ["S1", A, "T"]);
    }
}

#[test]
fn unbounded_simple_paths_with_unreachable_garden() {
    let mut graph = DiGraph::simple();
    graph.add_vertices(["S", A, "T", "U", "V"]);
    for (source, target) in [("S", A), (A, "T"), ("U", "T"), ("V", "U")] {
        graph.add_edge(&source, &target).unwrap();
    }
    for forward_pruning in [false, true] {
        let mut algorithm = AllDirectedPaths::new(&graph);
        algorithm.set_forward_pruning(forward_pruning);
        let paths = algorithm.get_all_paths(&"S", &"T", true, None).unwrap();
        assert_eq!(paths.len(), 1, "forward_pruning={forward_pruning}");
        assert_eq!(paths[0].vertex_list(), ["S", A, "T"]);
    }
}

#[test]
fn source_equals_target_in_disconnected_graph() {
    let mut graph = DiGraph::simple();
    graph.add_vertices(["S", "U"]);
    graph.add_edge(&"U", &"S").unwrap();
    for forward_pruning in [false, true] {
        let mut algorithm = AllDirectedPaths::new(&graph);
        algorithm.set_forward_pruning(forward_pruning);
        let paths = algorithm.get_all_paths(&"S", &"S", true, Some(5)).unwrap();
        assert_eq!(paths.len(), 1, "forward_pruning={forward_pruning}");
        assert_eq!(paths[0].length(), 0);
    }
}

#[test]
fn dense_strongly_connected_loss_case() {
    let n = 6;
    let mut graph = DiGraph::simple();
    graph.add_vertices(0..n);
    for source in 0..n {
        for target in 0..n {
            if source != target {
                graph.add_edge(&source, &target).unwrap();
            }
        }
    }
    let expected: HashSet<_> = brute_force_simple_paths(&graph, 0, n - 1, 3)
        .into_iter()
        .collect();
    for forward_pruning in [false, true] {
        let mut algorithm = AllDirectedPaths::new(&graph);
        algorithm.set_forward_pruning(forward_pruning);
        let actual = algorithm
            .get_all_paths(&0, &(n - 1), true, Some(3))
            .unwrap();
        assert_eq!(
            path_set(&actual),
            expected,
            "forward_pruning={forward_pruning}"
        );
    }
}

fn random_graph_with_isolated_pocket(
    seed: i64,
    main_size: i32,
    pocket_size: i32,
    edge_probability: f64,
) -> DiGraph<i32> {
    let mut rng = StdRng::seed_from_u64(seed as u64);
    let mut graph = DiGraph::simple();
    graph.add_vertices(0..main_size + pocket_size);
    for source in 0..main_size {
        for target in 0..main_size {
            if source != target && rng.random::<f64>() < edge_probability {
                graph.add_edge(&source, &target).unwrap();
            }
        }
    }
    for source in 0..main_size - 1 {
        if !graph.contains_edge_between(&source, &(source + 1)) {
            graph.add_edge(&source, &(source + 1)).unwrap();
        }
    }
    for offset in 0..pocket_size {
        let source = main_size + offset;
        graph.add_edge(&source, &(main_size - 1)).unwrap();
        for other_offset in 0..pocket_size {
            let target = main_size + other_offset;
            if source != target && rng.random::<f64>() < edge_probability {
                graph.add_edge(&source, &target).unwrap();
            }
        }
    }
    graph
}

#[test]
fn fuzz_against_brute_force() {
    for seed in [1, 2, 3, 5, 7, 11, 13, 17] {
        let graph = random_graph_with_isolated_pocket(seed, 4, 4, 0.4);
        let expected_simple: HashSet<_> = brute_force_simple_paths(&graph, 0, 3, 4)
            .into_iter()
            .collect();
        let expected_non_simple: HashSet<_> =
            brute_force_walks(&graph, 0, 3, 4).into_iter().collect();
        for forward_pruning in [false, true] {
            let mut algorithm = AllDirectedPaths::new(&graph);
            algorithm.set_forward_pruning(forward_pruning);
            let simple = algorithm.get_all_paths(&0, &3, true, Some(4)).unwrap();
            assert_eq!(
                path_set(&simple),
                expected_simple,
                "simple seed={seed} forward_pruning={forward_pruning}"
            );
            let non_simple = algorithm.get_all_paths(&0, &3, false, Some(4)).unwrap();
            assert_eq!(
                path_set(&non_simple),
                expected_non_simple,
                "non-simple seed={seed} forward_pruning={forward_pruning}"
            );
        }
    }
}
