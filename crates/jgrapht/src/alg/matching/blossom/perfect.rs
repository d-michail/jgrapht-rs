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

use super::dual::BlossomVDualUpdater;
use super::initializer::BlossomVInitializer;
use super::options::{BlossomVOptions, DualUpdateStrategy, ObjectiveSense};
use super::primal::BlossomVPrimalUpdater;
use super::state::{BlossomVState, BlossomVStatistics};
use super::types::{NodeId, TreeId};
use crate::alg::matching::{Matching, MatchingAlgorithm, DEFAULT_EPSILON};
use crate::error::{GraphError, Result};
use crate::graph::{props, Graph};
use crate::util::HashableSet;
use indexmap::{IndexMap, IndexSet};
use std::collections::{HashMap, HashSet};

/// Default numerical tolerance used by Blossom V.
pub const BLOSSOM_V_EPSILON: f64 = DEFAULT_EPSILON;
/// Internal finite infinity used by the Java implementation.
pub const BLOSSOM_V_INFINITY: f64 = 1e100;
const NO_PERFECT_MATCHING: &str = "There is no perfect matching in the specified graph";

/// A feasible dual solution for the weighted perfect-matching linear program.
#[derive(Debug, Clone)]
pub struct BlossomVDualSolution<'g, G: Graph> {
    graph: &'g G,
    dual_variables: IndexMap<HashableSet<G::Vertex>, f64>,
}

impl<'g, G: Graph> BlossomVDualSolution<'g, G> {
    /// Graph on which the primal and dual programs are formulated.
    pub const fn graph(&self) -> &'g G {
        self.graph
    }

    /// Non-zero dual variables, keyed by odd-cardinality vertex sets.
    pub fn dual_variables(&self) -> &IndexMap<HashableSet<G::Vertex>, f64> {
        &self.dual_variables
    }
}

/// Kolmogorov's Blossom V algorithm for minimum- or maximum-weight perfect matchings.
pub struct KolmogorovWeightedPerfectMatching<'g, G: Graph> {
    graph: &'g G,
    options: BlossomVOptions,
    objective_sense: ObjectiveSense,
    matching: Option<Matching<G::Vertex, G::Edge>>,
    state: Option<BlossomVState<'g, G>>,
}

impl<'g, G: Graph> KolmogorovWeightedPerfectMatching<'g, G> {
    /// Constructs a minimum-weight perfect-matching algorithm with default options.
    pub fn new(graph: &'g G) -> Result<Self> {
        Self::with_options_and_objective(
            graph,
            BlossomVOptions::default(),
            ObjectiveSense::Minimize,
        )
    }

    /// Constructs an algorithm with default options and the requested objective.
    pub fn with_objective(graph: &'g G, objective_sense: ObjectiveSense) -> Result<Self> {
        Self::with_options_and_objective(graph, BlossomVOptions::default(), objective_sense)
    }

    /// Constructs a minimum-weight algorithm with custom options.
    pub fn with_options(graph: &'g G, options: BlossomVOptions) -> Result<Self> {
        Self::with_options_and_objective(graph, options, ObjectiveSense::Minimize)
    }

    /// Constructs an algorithm with custom initialization, dual updates, and objective.
    pub fn with_options_and_objective(
        graph: &'g G,
        options: BlossomVOptions,
        objective_sense: ObjectiveSense,
    ) -> Result<Self> {
        props::require_undirected(graph)?;
        if graph.vertex_count() % 2 == 1 {
            return Err(GraphError::InvalidArgument(NO_PERFECT_MATCHING.into()));
        }
        Ok(Self {
            graph,
            options,
            objective_sense,
            matching: None,
            state: None,
        })
    }

    /// Returns counters and timings from the completed run.
    pub fn statistics(&self) -> BlossomVStatistics {
        self.state
            .as_ref()
            .map(|state| state.statistics)
            .unwrap_or_default()
    }

    /// Returns whether dual feasibility and complementary slackness hold within the tolerance.
    pub fn test_optimality(&mut self) -> Result<bool> {
        Ok(self.get_error()? < BLOSSOM_V_EPSILON)
    }

    /// Returns the accumulated numerical error in the dual and complementary-slackness checks.
    pub fn get_error(&mut self) -> Result<f64> {
        self.compute()?;
        let state = self.state.as_ref().expect("computed state");
        let matching = self.matching.as_ref().expect("computed matching");
        let mut error = Self::nonnegativity_error(state);
        let matched: HashSet<_> = matching.edges().iter().cloned().collect();
        let scale = if self.objective_sense == ObjectiveSense::Maximize {
            -1.0
        } else {
            1.0
        };

        for edge in 0..state.edge_num {
            let graph_edge = &state.graph_edges[edge];
            let mut slack =
                scale * state.graph.edge_weight(graph_edge).unwrap_or(1.0) - state.min_edge_weight;
            let [Some(a), Some(b)] = state.arena.edges[edge].head_original else {
                unreachable!("original edge endpoints")
            };
            let (ancestor_a, ancestor_b) = Self::lca_or_outermost(state, a, b);
            slack -= Self::total_dual(state, a, ancestor_a);
            slack -= Self::total_dual(state, b, ancestor_b);
            if ancestor_a == ancestor_b {
                slack += 2.0 * state.arena.node_true_dual(ancestor_a);
            }
            if slack < 0.0 || matched.contains(graph_edge) {
                error += slack.abs();
            }
        }
        Ok(error)
    }

    /// Returns the non-zero dual variables of the computed solution.
    pub fn get_dual_solution(&mut self) -> Result<BlossomVDualSolution<'g, G>> {
        self.compute()?;
        let state = self.state.as_ref().expect("computed state");
        let mut blossom_members: HashMap<NodeId, HashableSet<G::Vertex>> = HashMap::new();
        for node in 0..state.node_num {
            let mut parent = state.arena.nodes[node].blossom_parent;
            while let Some(blossom) = parent {
                blossom_members
                    .entry(blossom)
                    .or_insert_with(HashableSet::new)
                    .insert(state.graph_vertices[node].clone());
                if state.arena.nodes[blossom].is_outer {
                    break;
                }
                parent = state.arena.nodes[blossom].blossom_parent;
            }
        }

        let sign = if self.objective_sense == ObjectiveSense::Maximize {
            -1.0
        } else {
            1.0
        };
        let shift = state.min_edge_weight / 2.0;
        let mut dual_variables = IndexMap::new();
        for node in 0..state.node_num {
            let dual = sign * (state.arena.node_true_dual(node) + shift);
            if dual.abs() > BLOSSOM_V_EPSILON {
                dual_variables.insert(
                    HashableSet::from([state.graph_vertices[node].clone()]),
                    dual,
                );
            }
        }
        let mut blossoms: Vec<_> = blossom_members.into_iter().collect();
        blossoms.sort_unstable_by_key(|(node, _)| *node);
        for (blossom, members) in blossoms {
            let dual = sign * state.arena.node_true_dual(blossom);
            if dual.abs() > BLOSSOM_V_EPSILON {
                dual_variables.insert(members, dual);
            }
        }
        Ok(BlossomVDualSolution {
            graph: self.graph,
            dual_variables,
        })
    }

    fn compute(&mut self) -> Result<()> {
        if self.matching.is_some() {
            return Ok(());
        }
        let scale = if self.objective_sense == ObjectiveSense::Maximize {
            -1.0
        } else {
            1.0
        };
        let mut state =
            BlossomVInitializer::with_weight_scale(self.graph, scale).initialize(self.options)?;
        debug_assert_eq!(state.options, self.options);

        loop {
            let cycle_tree_num = state.tree_num;
            let mut current_root = state.arena.nodes[state.root_sentinel].tree_sibling_next;
            while let Some(root) = current_root {
                let next_root = state.arena.nodes[root].tree_sibling_next;
                let next_next_root =
                    next_root.and_then(|node| state.arena.nodes[node].tree_sibling_next);
                let tree = state.arena.nodes[root].tree.expect("tree root");
                let iteration_tree_num = state.tree_num;

                Self::set_current_edges_and_try_to_augment(&mut state, tree);
                if iteration_tree_num == state.tree_num && self.options.update_duals_before() {
                    BlossomVDualUpdater::update_duals_single(&mut state, tree)?;
                }

                while iteration_tree_num == state.tree_num {
                    if let Some(edge) = state.arena.trees[tree]
                        .plus_infinity_edges
                        .peek_entry()
                        .map(|(_, _, edge)| *edge)
                    {
                        if state.arena.edges[edge].slack <= state.arena.trees[tree].eps {
                            BlossomVPrimalUpdater::grow(&mut state, edge, true, true);
                            continue;
                        }
                    }
                    if let Some(edge) = state.arena.trees[tree]
                        .plus_plus_edges
                        .peek_entry()
                        .map(|(_, _, edge)| *edge)
                    {
                        if state.arena.edges[edge].slack <= 2.0 * state.arena.trees[tree].eps {
                            BlossomVPrimalUpdater::shrink(&mut state, edge, true);
                            continue;
                        }
                    }
                    if let Some(blossom) = state.arena.trees[tree]
                        .minus_blossoms
                        .peek_entry()
                        .map(|(_, _, node)| *node)
                    {
                        if state.arena.nodes[blossom].dual <= state.arena.trees[tree].eps {
                            BlossomVPrimalUpdater::expand(&mut state, blossom, true);
                            continue;
                        }
                    }
                    break;
                }

                if state.tree_num == iteration_tree_num {
                    state.arena.trees[tree].current_edge = None;
                    if self.options.update_duals_after()
                        && BlossomVDualUpdater::update_duals_single(&mut state, tree)?
                    {
                        current_root = Some(root);
                        continue;
                    }
                    state.arena.clear_current_edges(tree);
                }
                current_root = next_root;
                if next_root.is_some_and(|node| state.arena.nodes[node].is_infinity()) {
                    current_root = next_next_root;
                }
            }

            if state.tree_num == 0 {
                break;
            }
            if cycle_tree_num == state.tree_num
                && BlossomVDualUpdater::update_duals(
                    &mut state,
                    self.options.dual_update_strategy(),
                )? <= 0.0
            {
                BlossomVDualUpdater::update_duals(
                    &mut state,
                    DualUpdateStrategy::MultipleTreeConnectedComponents,
                )?;
            }
        }

        self.matching = Some(Self::finish(&mut state, self.graph)?);
        self.state = Some(state);
        Ok(())
    }

    fn set_current_edges_and_try_to_augment(state: &mut BlossomVState<'_, G>, tree: TreeId) {
        for (tree_edge, direction) in state.arena.tree_edges(tree) {
            let opposite =
                state.arena.tree_edges[tree_edge].head[direction].expect("opposite tree");
            if let Some(edge) = state.arena.tree_edges[tree_edge]
                .plus_plus_edges
                .peek_entry()
                .map(|(_, _, edge)| *edge)
            {
                if state.arena.edges[edge].slack
                    <= state.arena.trees[tree].eps + state.arena.trees[opposite].eps
                {
                    BlossomVPrimalUpdater::augment(state, edge);
                    break;
                }
            }
            state.arena.trees[opposite].current_edge = Some(tree_edge);
            state.arena.trees[opposite].current_direction = direction;
        }
    }

    fn finish(state: &mut BlossomVState<'_, G>, graph: &G) -> Result<Matching<G::Vertex, G::Edge>> {
        let mut processed = Vec::new();
        for node in 0..state.node_num {
            if state.arena.nodes[node].matched.is_some() {
                continue;
            }
            let mut blossom_previous = None;
            let mut blossom = node;
            loop {
                state.arena.nodes[blossom].blossom_grandparent = blossom_previous;
                blossom_previous = Some(blossom);
                blossom = state.arena.nodes[blossom]
                    .blossom_parent
                    .expect("unmatched contracted node has parent blossom");
                if state.arena.nodes[blossom].is_outer {
                    break;
                }
            }

            loop {
                let blossom_matched = state.arena.nodes[blossom]
                    .matched
                    .expect("outer blossom matched");
                let mut blossom_root = state
                    .arena
                    .edge_current_original(blossom_matched, blossom)
                    .unwrap_or_else(|| {
                        let [Some(a), Some(_b)] = state.arena.edges[blossom_matched].head else {
                            unreachable!("matched edge endpoints")
                        };
                        if state.arena.nodes[a].is_processed {
                            state.arena.edges[blossom_matched].head_original[1]
                                .expect("original endpoint")
                        } else {
                            state.arena.edges[blossom_matched].head_original[0]
                                .expect("original endpoint")
                        }
                    });
                while state.arena.nodes[blossom_root].blossom_parent != Some(blossom) {
                    blossom_root = state.arena.nodes[blossom_root]
                        .blossom_parent
                        .expect("blossom ancestor");
                }
                state.arena.nodes[blossom_root].matched = Some(blossom_matched);
                if let Some(opposite) = state.arena.edge_opposite(blossom_matched, blossom) {
                    state.arena.nodes[opposite].is_processed = true;
                    processed.push(opposite);
                }
                let mut current = Self::next_blossom_node(state, blossom_root);
                while current != blossom_root {
                    let sibling = state.arena.nodes[current]
                        .blossom_sibling
                        .expect("blossom sibling edge");
                    state.arena.nodes[current].matched = Some(sibling);
                    let next = state
                        .arena
                        .edge_opposite(sibling, current)
                        .expect("sibling endpoint");
                    state.arena.nodes[next].matched = Some(sibling);
                    current = Self::next_blossom_node(state, next);
                }
                let previous = blossom_previous.expect("previous blossom layer");
                if !state.arena.nodes[previous].is_blossom {
                    break;
                }
                blossom = previous;
                blossom_previous = state.arena.nodes[blossom].blossom_grandparent;
            }
            for processed_node in processed.drain(..) {
                state.arena.nodes[processed_node].is_processed = false;
            }
        }

        let mut edges = IndexSet::new();
        let mut weight = 0.0;
        for node in 0..state.node_num {
            let matched = state.arena.nodes[node]
                .matched
                .ok_or_else(|| GraphError::InvalidArgument(NO_PERFECT_MATCHING.into()))?;
            let graph_edge = state.graph_edges[state.arena.edges[matched].pos].clone();
            if edges.insert(graph_edge.clone()) {
                weight += graph.edge_weight(&graph_edge).unwrap_or(1.0);
            }
        }
        Ok(Matching::new(graph, edges, weight))
    }

    fn next_blossom_node(state: &BlossomVState<'_, G>, node: NodeId) -> NodeId {
        let edge = state.arena.nodes[node]
            .blossom_sibling
            .expect("blossom sibling edge");
        state
            .arena
            .edge_opposite(edge, node)
            .expect("blossom sibling endpoint")
    }

    fn nonnegativity_error(state: &BlossomVState<'_, G>) -> f64 {
        let mut seen = HashSet::new();
        let mut error = 0.0;
        for node in 0..state.node_num {
            let mut parent = state.arena.nodes[node].blossom_parent;
            while let Some(blossom) = parent {
                if !seen.insert(blossom) {
                    break;
                }
                let dual = state.arena.nodes[blossom].dual;
                if dual < 0.0 {
                    error += dual.abs();
                    break;
                }
                parent = state.arena.nodes[blossom].blossom_parent;
            }
        }
        error
    }

    fn total_dual(state: &BlossomVState<'_, G>, start: NodeId, end: NodeId) -> f64 {
        let mut result = 0.0;
        let mut current = start;
        loop {
            result += state.arena.node_true_dual(current);
            if current == end {
                return result;
            }
            current = state.arena.nodes[current]
                .blossom_parent
                .expect("end belongs to ancestor chain");
        }
    }

    fn lca_or_outermost(state: &BlossomVState<'_, G>, a: NodeId, b: NodeId) -> (NodeId, NodeId) {
        let chain = |start: NodeId| {
            let mut result = vec![start];
            let mut current = start;
            if state.arena.nodes[current].is_outer {
                return result;
            }
            while let Some(parent) = state.arena.nodes[current].blossom_parent {
                result.push(parent);
                current = parent;
                if state.arena.nodes[current].is_outer {
                    break;
                }
            }
            result
        };
        let chain_a = chain(a);
        let chain_b = chain(b);
        let ancestors_a: HashSet<_> = chain_a.iter().copied().collect();
        if let Some(common) = chain_b
            .iter()
            .copied()
            .find(|node| ancestors_a.contains(node))
        {
            return (common, common);
        }
        (
            *chain_a.last().expect("non-empty ancestor chain"),
            *chain_b.last().expect("non-empty ancestor chain"),
        )
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge> for KolmogorovWeightedPerfectMatching<'_, G> {
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        self.compute()?;
        Ok(self.matching.clone().expect("computed matching"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphMut, GraphMutExt, UGraph};

    fn assert_dual_solution<G: Graph>(
        graph: &G,
        matching: &Matching<G::Vertex, G::Edge>,
        solution: &BlossomVDualSolution<'_, G>,
        objective: ObjectiveSense,
    ) {
        assert_eq!(matching.len() * 2, graph.vertex_count());
        for (vertices, &dual) in solution.dual_variables() {
            assert_eq!(vertices.len() % 2, 1);
            if vertices.len() > 1 {
                match objective {
                    ObjectiveSense::Minimize => assert!(dual >= -BLOSSOM_V_EPSILON),
                    ObjectiveSense::Maximize => assert!(dual <= BLOSSOM_V_EPSILON),
                }
            }
        }
        for edge in graph.edges() {
            let source = graph.edge_source(edge).unwrap();
            let target = graph.edge_target(edge).unwrap();
            if source == target {
                continue;
            }
            let mut slack = graph.edge_weight(edge).unwrap_or(1.0);
            for (vertices, &dual) in solution.dual_variables() {
                if vertices.contains(source) != vertices.contains(target) {
                    slack -= dual;
                }
            }
            if matching.edges().contains(edge) {
                assert!(slack.abs() < BLOSSOM_V_EPSILON, "matched slack={slack}");
            } else {
                match objective {
                    ObjectiveSense::Minimize => assert!(slack >= -BLOSSOM_V_EPSILON),
                    ObjectiveSense::Maximize => assert!(slack <= BLOSSOM_V_EPSILON),
                }
            }
        }
    }

    fn exact_perfect_weight(weights: &[Vec<f64>], maximize: bool) -> f64 {
        fn solve(
            weights: &[Vec<f64>],
            mask: usize,
            maximize: bool,
            memo: &mut [Option<f64>],
        ) -> f64 {
            if mask == 0 {
                return 0.0;
            }
            if let Some(value) = memo[mask] {
                return value;
            }
            let first = mask.trailing_zeros() as usize;
            let rest = mask & !(1 << first);
            let mut best = if maximize {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
            let mut candidates = rest;
            while candidates != 0 {
                let second = candidates.trailing_zeros() as usize;
                candidates &= !(1 << second);
                let value =
                    weights[first][second] + solve(weights, rest & !(1 << second), maximize, memo);
                best = if maximize {
                    best.max(value)
                } else {
                    best.min(value)
                };
            }
            memo[mask] = Some(best);
            best
        }

        let mut memo = vec![None; 1 << weights.len()];
        solve(weights, (1 << weights.len()) - 1, maximize, &mut memo)
    }

    fn exact_sparse_perfect_weight(weights: &[Vec<Option<f64>>], maximize: bool) -> Option<f64> {
        fn solve(
            weights: &[Vec<Option<f64>>],
            mask: usize,
            maximize: bool,
            memo: &mut [Option<Option<f64>>],
        ) -> Option<f64> {
            if mask == 0 {
                return Some(0.0);
            }
            if let Some(value) = memo[mask] {
                return value;
            }
            let first = mask.trailing_zeros() as usize;
            let rest = mask & !(1 << first);
            let mut best: Option<f64> = None;
            let mut candidates = rest;
            while candidates != 0 {
                let second = candidates.trailing_zeros() as usize;
                candidates &= !(1 << second);
                let Some(edge_weight) = weights[first][second] else {
                    continue;
                };
                let Some(suffix) = solve(weights, rest & !(1 << second), maximize, memo) else {
                    continue;
                };
                let value = edge_weight + suffix;
                best = Some(best.map_or(value, |current| {
                    if maximize {
                        current.max(value)
                    } else {
                        current.min(value)
                    }
                }));
            }
            memo[mask] = Some(best);
            best
        }

        let mut memo = vec![None; 1 << weights.len()];
        solve(weights, (1 << weights.len()) - 1, maximize, &mut memo)
    }

    fn weighted_graph(vertices: usize, edges: &[(usize, usize, f64)]) -> UGraph<usize> {
        let mut graph = UGraph::<usize>::simple().weighted();
        for vertex in 0..vertices {
            graph.add_vertex(vertex);
        }
        for &(a, b, weight) in edges {
            graph.add_weighted_edge(&a, &b, weight).unwrap();
        }
        graph
    }

    #[test]
    fn perfect_matching_on_two_disjoint_edges() {
        let graph = weighted_graph(4, &[(0, 1, 5.0), (2, 3, 2.0)]);
        let options = BlossomVOptions::with_initialization(super::super::InitializationType::None);
        let mut algorithm =
            KolmogorovWeightedPerfectMatching::with_options(&graph, options).unwrap();
        let matching = algorithm.get_matching().unwrap();
        assert!(matching.is_perfect());
        assert_eq!(matching.weight(), 7.0);
    }

    #[test]
    fn invalid_dual_solution_is_rejected() {
        let graph = weighted_graph(
            4,
            &[(0, 1, 7.0), (1, 2, 4.0), (2, 3, 3.0), (3, 0, 4.0)],
        );
        let mut algorithm = KolmogorovWeightedPerfectMatching::new(&graph).unwrap();
        algorithm.get_matching().unwrap();
        algorithm.state.as_mut().unwrap().arena.nodes[0].dual += 1.0;
        assert!(!algorithm.test_optimality().unwrap());
    }

    #[test]
    fn minimum_and_maximum_perfect_matching_on_k4() {
        let graph = weighted_graph(
            4,
            &[
                (0, 1, 1.0),
                (0, 2, 2.0),
                (0, 3, 8.0),
                (1, 2, 7.0),
                (1, 3, 3.0),
                (2, 3, 4.0),
            ],
        );
        let options = BlossomVOptions::with_initialization(super::super::InitializationType::None);
        let mut minimum = KolmogorovWeightedPerfectMatching::with_options_and_objective(
            &graph,
            options,
            ObjectiveSense::Minimize,
        )
        .unwrap();
        let mut maximum = KolmogorovWeightedPerfectMatching::with_options_and_objective(
            &graph,
            options,
            ObjectiveSense::Maximize,
        )
        .unwrap();
        assert_eq!(minimum.get_matching().unwrap().weight(), 5.0);
        assert_eq!(maximum.get_matching().unwrap().weight(), 15.0);
    }

    #[test]
    fn parallel_edges_are_distinct_and_self_loops_are_ignored() {
        let mut graph = UGraph::<usize>::pseudo().weighted();
        graph.add_vertex(0);
        graph.add_vertex(1);
        graph.add_weighted_edge(&0, &0, -100.0).unwrap();
        graph.add_weighted_edge(&0, &1, 7.0).unwrap();
        graph.add_weighted_edge(&0, &1, -3.0).unwrap();

        let mut minimum = KolmogorovWeightedPerfectMatching::new(&graph).unwrap();
        let mut maximum =
            KolmogorovWeightedPerfectMatching::with_objective(&graph, ObjectiveSense::Maximize)
                .unwrap();
        assert_eq!(minimum.get_matching().unwrap().weight(), -3.0);
        assert_eq!(maximum.get_matching().unwrap().weight(), 7.0);
        assert!(minimum.test_optimality().unwrap());
        assert!(maximum.test_optimality().unwrap());
    }

    #[test]
    fn all_options_match_exact_solver_on_small_complete_graphs() {
        for seed in 0..8 {
            let vertex_count = if seed < 2 { 4 } else { 6 };
            let mut weights = vec![vec![0.0; vertex_count]; vertex_count];
            let mut edges = Vec::new();
            for source in 0..vertex_count {
                for target in source + 1..vertex_count {
                    let value =
                        (((source + 3) * 17 + (target + 5) * 29 + seed * 31) % 37) as f64 - 18.0;
                    weights[source][target] = value;
                    weights[target][source] = value;
                    edges.push((source, target, value));
                }
            }
            let graph = weighted_graph(vertex_count, &edges);
            for options in BlossomVOptions::all_options() {
                for (objective, maximize) in [
                    (ObjectiveSense::Minimize, false),
                    (ObjectiveSense::Maximize, true),
                ] {
                    let expected = exact_perfect_weight(&weights, maximize);
                    let mut algorithm =
                        KolmogorovWeightedPerfectMatching::with_options_and_objective(
                            &graph, options, objective,
                        )
                        .unwrap();
                    let actual = algorithm.get_matching().unwrap().weight();
                    assert_eq!(
                        actual, expected,
                        "seed={seed}, options={options}, objective={objective:?}"
                    );
                    let error = algorithm.get_error().unwrap();
                    assert!(
                        error < BLOSSOM_V_EPSILON,
                        "error={error}, seed={seed}, options={options}, objective={objective:?}"
                    );
                    let dual = algorithm.get_dual_solution().unwrap();
                    assert!(dual
                        .dual_variables()
                        .keys()
                        .all(|vertices| vertices.len() % 2 == 1));
                    let matching = algorithm.get_matching().unwrap();
                    assert_dual_solution(&graph, &matching, &dual, objective);
                }
            }
        }
    }

    #[test]
    fn all_options_match_exact_solver_on_sparse_graphs() {
        for seed in 0..16 {
            let vertex_count = if seed < 2 { 4 } else { 8 };
            let mut weights = vec![vec![None; vertex_count]; vertex_count];
            let mut edges = Vec::new();
            for source in 0..vertex_count {
                for target in source + 1..vertex_count {
                    let hash = (source + 5) * 41 + (target + 7) * 67 + seed * 73;
                    if hash % 5 < 2 {
                        continue;
                    }
                    let value = (hash % 43) as f64 - 21.0;
                    weights[source][target] = Some(value);
                    weights[target][source] = Some(value);
                    edges.push((source, target, value));
                }
            }
            let graph = weighted_graph(vertex_count, &edges);
            for options in BlossomVOptions::all_options() {
                for (objective, maximize) in [
                    (ObjectiveSense::Minimize, false),
                    (ObjectiveSense::Maximize, true),
                ] {
                    let expected = exact_sparse_perfect_weight(&weights, maximize);
                    let mut algorithm =
                        KolmogorovWeightedPerfectMatching::with_options_and_objective(
                            &graph, options, objective,
                        )
                        .unwrap();
                    let result = algorithm.get_matching();
                    match expected {
                        Some(expected) => {
                            let matching = result.unwrap();
                            assert_eq!(
                                matching.weight(),
                                expected,
                                "seed={seed}, options={options}, objective={objective:?}"
                            );
                            assert!(algorithm.test_optimality().unwrap());
                        }
                        None => assert!(
                            result.is_err(),
                            "seed={seed}, options={options}, objective={objective:?}"
                        ),
                    }
                }
            }
        }
    }
}
