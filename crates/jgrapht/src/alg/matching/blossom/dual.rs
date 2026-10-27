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

use super::options::DualUpdateStrategy;
use super::primal::BlossomVPrimalUpdater;
use super::state::BlossomVState;
use super::types::{EdgeId, TreeEdgeId, TreeId};
use crate::alg::matching::DEFAULT_EPSILON;
use crate::error::{GraphError, Result};
use crate::graph::Graph;
use std::time::Instant;

const INFINITY: f64 = 1e100;
const NO_PERFECT_MATCHING_THRESHOLD: f64 = 1e10;
const NO_PERFECT_MATCHING: &str = "There is no perfect matching in the specified graph";
const PROCESSED_TREE: usize = usize::MAX;

pub(crate) struct BlossomVDualUpdater;

impl BlossomVDualUpdater {
    pub(crate) fn update_duals<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        strategy: DualUpdateStrategy,
    ) -> Result<f64> {
        let start = Instant::now();
        let roots = Self::roots(state);
        for root in &roots {
            let tree = state.arena.nodes[*root].tree.expect("root tree");
            let eps = Self::get_eps(state, tree);
            state.arena.trees[tree].accumulated_eps = eps - state.arena.trees[tree].eps;
        }
        let augment_edge = match strategy {
            DualUpdateStrategy::MultipleTreeFixedDelta => Self::multiple_tree_fixed_delta(state)?,
            DualUpdateStrategy::MultipleTreeConnectedComponents => {
                Self::update_connected_components(state)?
            }
        };

        let mut dual_change = 0.0;
        for root in roots {
            let tree = state.arena.nodes[root].tree.expect("root tree");
            let delta = state.arena.trees[tree].accumulated_eps;
            if delta > DEFAULT_EPSILON {
                dual_change += delta;
                state.arena.trees[tree].eps += delta;
            }
        }
        state.statistics.dual_updates_time_ns += start.elapsed().as_nanos();
        if let Some(edge) = augment_edge {
            BlossomVPrimalUpdater::augment(state, edge);
        }
        Ok(dual_change)
    }

    pub(crate) fn update_duals_single<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        tree: TreeId,
    ) -> Result<bool> {
        let start = Instant::now();
        let mut eps = Self::get_eps(state, tree);
        let mut eps_augment = INFINITY;
        let mut augment_edge = None;
        for (tree_edge, direction) in state.arena.tree_edges(tree) {
            let opposite =
                state.arena.tree_edges[tree_edge].head[direction].expect("opposite tree");
            if let Some(edge) = Self::heap_min_plus_plus(state, tree_edge) {
                let candidate = state.arena.edges[edge].slack - state.arena.trees[opposite].eps;
                if candidate < eps_augment {
                    eps_augment = candidate;
                    augment_edge = Some(edge);
                }
            }
            let current_direction = state.arena.trees[opposite].current_direction;
            if let Some(edge) = Self::heap_min_plus_minus(state, tree_edge, current_direction) {
                let candidate = state.arena.edges[edge].slack + state.arena.trees[opposite].eps;
                if candidate < eps {
                    eps = candidate;
                }
            }
        }
        eps = eps.min(eps_augment);
        if eps > NO_PERFECT_MATCHING_THRESHOLD {
            return Err(GraphError::InvalidArgument(NO_PERFECT_MATCHING.into()));
        }
        let mut delta = 0.0;
        if eps > state.arena.trees[tree].eps {
            delta = eps - state.arena.trees[tree].eps;
            state.arena.trees[tree].eps = eps;
        }
        state.statistics.dual_updates_time_ns += start.elapsed().as_nanos();
        if let Some(augment_edge) = augment_edge
            && eps_augment <= state.arena.trees[tree].eps
        {
            BlossomVPrimalUpdater::augment(state, augment_edge);
            Ok(false)
        } else {
            Ok(delta > DEFAULT_EPSILON)
        }
    }

    fn get_eps<G: Graph>(state: &BlossomVState<'_, G>, tree: TreeId) -> f64 {
        let value = &state.arena.trees[tree];
        let mut eps = INFINITY;
        if let Some((_, _, &edge)) = value.plus_infinity_edges.peek_entry() {
            eps = eps.min(state.arena.edges[edge].slack);
        }
        if let Some((_, _, &node)) = value.minus_blossoms.peek_entry() {
            eps = eps.min(state.arena.nodes[node].dual);
        }
        if let Some((_, _, &edge)) = value.plus_plus_edges.peek_entry() {
            eps = eps.min(state.arena.edges[edge].slack / 2.0);
        }
        eps
    }

    fn multiple_tree_fixed_delta<G: Graph>(
        state: &mut BlossomVState<'_, G>,
    ) -> Result<Option<EdgeId>> {
        let mut eps = INFINITY;
        let mut augment_eps = INFINITY;
        let mut augment_edge = None;
        for root in Self::roots(state) {
            let tree = state.arena.nodes[root].tree.expect("root tree");
            eps = eps.min(state.arena.trees[tree].accumulated_eps);
            let tree_eps = state.arena.trees[tree].eps;
            let mut outgoing = state.arena.trees[tree].first[0];
            while let Some(tree_edge) = outgoing {
                if let Some(edge) = Self::heap_min_plus_plus(state, tree_edge) {
                    let opposite =
                        state.arena.tree_edges[tree_edge].head[0].expect("outgoing opposite tree");
                    let slack =
                        state.arena.edges[edge].slack - tree_eps - state.arena.trees[opposite].eps;
                    eps = eps.min(slack / 2.0);
                    if slack < augment_eps {
                        augment_eps = slack;
                        augment_edge = Some(edge);
                    }
                }
                outgoing = state.arena.tree_edges[tree_edge].next[0];
            }
        }
        Self::require_finite_progress(eps)?;
        for root in Self::roots(state) {
            let tree = state.arena.nodes[root].tree.expect("root tree");
            state.arena.trees[tree].accumulated_eps = eps;
        }
        Ok((augment_eps <= 2.0 * eps).then_some(augment_edge).flatten())
    }

    fn update_connected_components<G: Graph>(
        state: &mut BlossomVState<'_, G>,
    ) -> Result<Option<EdgeId>> {
        let roots = Self::roots(state);
        for root in &roots {
            let tree = state.arena.nodes[*root].tree.expect("root tree");
            state.arena.trees[tree].next_tree = None;
        }
        let mut augment_edge = None;
        let mut augment_eps = INFINITY;
        for root in roots {
            let start_tree = state.arena.nodes[root].tree.expect("root tree");
            if state.arena.trees[start_tree].next_tree.is_some() {
                continue;
            }
            let mut eps = state.arena.trees[start_tree].accumulated_eps;
            state.arena.trees[start_tree].next_tree = Some(start_tree);
            let mut component_last = start_tree;
            let mut current_tree = start_tree;
            loop {
                for (tree_edge, direction) in state.arena.tree_edges(current_tree) {
                    let opposite =
                        state.arena.tree_edges[tree_edge].head[direction].expect("opposite tree");
                    let reverse = 1 - direction;
                    let plus_plus_eps =
                        if let Some(edge) = Self::heap_min_plus_plus(state, tree_edge) {
                            let value = state.arena.edges[edge].slack
                                - state.arena.trees[current_tree].eps
                                - state.arena.trees[opposite].eps;
                            if value < augment_eps {
                                augment_eps = value;
                                augment_edge = Some(edge);
                            }
                            value
                        } else {
                            INFINITY
                        };
                    if state.arena.trees[opposite].next_tree.is_some()
                        && state.arena.trees[opposite].next_tree != Some(PROCESSED_TREE)
                    {
                        eps = eps.min(plus_plus_eps / 2.0);
                        continue;
                    }

                    let mut plus_minus_eps = [INFINITY; 2];
                    if let Some(edge) = Self::heap_min_plus_minus(state, tree_edge, direction) {
                        plus_minus_eps[direction] = state.arena.edges[edge].slack
                            - state.arena.trees[current_tree].eps
                            + state.arena.trees[opposite].eps;
                    }
                    if let Some(edge) = Self::heap_min_plus_minus(state, tree_edge, reverse) {
                        plus_minus_eps[reverse] = state.arena.edges[edge].slack
                            - state.arena.trees[opposite].eps
                            + state.arena.trees[current_tree].eps;
                    }
                    let opposite_eps =
                        if state.arena.trees[opposite].next_tree == Some(PROCESSED_TREE) {
                            state.arena.trees[opposite].accumulated_eps
                        } else if plus_minus_eps[0] > 0.0 && plus_minus_eps[1] > 0.0 {
                            0.0
                        } else {
                            state.arena.trees[component_last].next_tree = Some(opposite);
                            component_last = opposite;
                            state.arena.trees[opposite].next_tree = Some(opposite);
                            eps = eps.min(state.arena.trees[opposite].accumulated_eps);
                            continue;
                        };
                    eps = eps.min(plus_plus_eps - opposite_eps);
                    eps = eps.min(plus_minus_eps[direction] + opposite_eps);
                }
                let next = state.arena.trees[current_tree]
                    .next_tree
                    .expect("component link");
                if next == current_tree {
                    break;
                }
                current_tree = next;
            }
            Self::require_finite_progress(eps)?;

            let mut next_tree = start_tree;
            loop {
                current_tree = next_tree;
                next_tree = state.arena.trees[current_tree]
                    .next_tree
                    .expect("component link");
                state.arena.trees[current_tree].next_tree = Some(PROCESSED_TREE);
                state.arena.trees[current_tree].accumulated_eps = eps;
                if current_tree == next_tree {
                    break;
                }
            }
        }
        if let Some(edge) = augment_edge {
            let [Some(a), Some(b)] = state.arena.edges[edge].head else {
                unreachable!("active edge endpoints")
            };
            let tree_a = state.arena.nodes[a].tree.expect("plus node tree");
            let tree_b = state.arena.nodes[b].tree.expect("plus node tree");
            if augment_eps
                - state.arena.trees[tree_a].accumulated_eps
                - state.arena.trees[tree_b].accumulated_eps
                <= 0.0
            {
                return Ok(Some(edge));
            }
        }
        Ok(None)
    }

    fn heap_min_plus_plus<G: Graph>(
        state: &BlossomVState<'_, G>,
        tree_edge: TreeEdgeId,
    ) -> Option<EdgeId> {
        state.arena.tree_edges[tree_edge]
            .plus_plus_edges
            .peek_entry()
            .map(|(_, _, edge)| *edge)
    }

    fn heap_min_plus_minus<G: Graph>(
        state: &BlossomVState<'_, G>,
        tree_edge: TreeEdgeId,
        direction: usize,
    ) -> Option<EdgeId> {
        let heap = if direction == 0 {
            &state.arena.tree_edges[tree_edge].plus_minus_edges_1
        } else {
            &state.arena.tree_edges[tree_edge].plus_minus_edges_0
        };
        heap.peek_entry().map(|(_, _, edge)| *edge)
    }

    fn roots<G: Graph>(state: &BlossomVState<'_, G>) -> Vec<usize> {
        let first = state.arena.nodes[state.root_sentinel].tree_sibling_next;
        first
            .into_iter()
            .flat_map(|root| {
                std::iter::successors(Some(root), |&node| {
                    state.arena.nodes[node].tree_sibling_next
                })
            })
            .collect()
    }

    fn require_finite_progress(eps: f64) -> Result<()> {
        if eps > NO_PERFECT_MATCHING_THRESHOLD {
            Err(GraphError::InvalidArgument(NO_PERFECT_MATCHING.into()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alg::matching::blossom::initializer::BlossomVInitializer;
    use crate::alg::matching::blossom::{BlossomVOptions, DualUpdateStrategy, InitializationType};
    use crate::graph::{GraphMut, GraphMutExt, UGraph};

    fn graph(edges: &[(i32, i32, f64)]) -> UGraph<i32> {
        let mut graph = UGraph::<i32>::simple().weighted();
        for &(source, target, weight) in edges {
            graph.add_vertex(source);
            graph.add_vertex(target);
            graph.add_weighted_edge(&source, &target, weight).unwrap();
        }
        graph
    }

    fn edge<G: Graph<Vertex = i32>>(
        graph: &G,
        state: &BlossomVState<'_, G>,
        a: i32,
        b: i32,
    ) -> EdgeId {
        state
            .graph_edges
            .iter()
            .position(|edge| {
                let source = graph.edge_source(edge).unwrap();
                let target = graph.edge_target(edge).unwrap();
                (*source, *target) == (a, b) || (*source, *target) == (b, a)
            })
            .unwrap()
    }

    #[test]
    fn fixed_delta_matches_java_two_edge_case() {
        let graph = graph(&[(1, 2, 5.0), (3, 4, 2.0)]);
        let mut state = BlossomVInitializer::new(&graph)
            .initialize(BlossomVOptions::with_initialization(
                InitializationType::None,
            ))
            .unwrap();
        let e34 = edge(&graph, &state, 3, 4);
        BlossomVPrimalUpdater::augment(&mut state, e34);
        assert!(
            BlossomVDualUpdater::update_duals(
                &mut state,
                DualUpdateStrategy::MultipleTreeFixedDelta,
            )
            .unwrap()
                > 0.0
        );
        for root in BlossomVDualUpdater::roots(&state) {
            let tree = state.arena.nodes[root].tree.unwrap();
            assert_eq!(state.arena.trees[tree].eps, 1.5);
        }
    }

    #[test]
    fn single_tree_update_matches_java_path_case() {
        let graph = graph(&[(1, 2, 5.0), (2, 3, 0.0)]);
        let mut state = BlossomVInitializer::new(&graph)
            .initialize(BlossomVOptions::with_initialization(
                InitializationType::None,
            ))
            .unwrap();
        let node1 = state
            .graph_vertices
            .iter()
            .position(|vertex| *vertex == 1)
            .unwrap();
        let tree = state.arena.nodes[node1].tree.unwrap();
        BlossomVDualUpdater::update_duals_single(&mut state, tree).unwrap();
        assert_eq!(state.arena.trees[tree].eps, 5.0);
    }
}
