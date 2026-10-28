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
use crate::alg::matching::{DEFAULT_EPSILON, Matching, MatchingAlgorithm};
use crate::error::{GraphError, Result};
use crate::graph::UndirectedGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::HashableSet;
use indexmap::{IndexMap, IndexSet};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

/// Default numerical tolerance used by Blossom V.
pub const BLOSSOM_V_EPSILON: f64 = DEFAULT_EPSILON;
/// Internal finite infinity used by the Java implementation.
pub const BLOSSOM_V_INFINITY: f64 = 1e100;

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

/// This class computes weighted perfect matchings in general graphs using the Blossom V algorithm.
/// If maximum or minimum weight matching algorithms is needed, see
/// `KolmogorovWeightedMatching`
///
/// Let $G = (V, E, c)$ be an undirected graph with a real-valued cost function defined on it. A
/// matching is an edge-disjoint subset of edges $M \subseteq E$. A matching is perfect if $2|M| =
/// |V|$. In the weighted perfect matching problem the goal is to maximize or minimize the weighted
/// sum of the edges in the matching. This class supports pseudographs, but a problem on a
/// pseudograph can be easily reduced to a problem on a simple graph. Moreover, this reduction can
/// heavily influence the running time since only an edge with a maximum or minimum weight between
/// two vertices can belong to the matching in the corresponding optimization problems. Currently,
/// users are responsible for doing this reduction themselves before invoking the algorithm.
///
/// Note that if the graph is unweighted and dense, `SparseEdmondsMaximumCardinalityMatching`
/// may be a better choice.
///
/// For more information about the algorithm see the following paper: *Kolmogorov, V. Math. Prog.
/// Comp. (2009) 1: 43. <https://doi.org/10.1007/s12532-009-0002-8>*, and the original
/// implementation: *<http://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz>*
///
/// The algorithm can be divided into two phases: initialization and the main algorithm. The
/// initialization phase is responsible for converting the specified graph into the form convenient
/// for the algorithm and for finding an initial matching to speed up the main part. Furthermore, the
/// main part of the algorithm can be further divided into primal and dual updates. The primal phases
/// are aimed at augmenting the matching so that the value of the objective function of the primal
/// linear program increases. Dual updates are aimed at increasing the objective function of the dual
/// linear program. The algorithm iteratively performs these primal and dual operations to build
/// alternating trees of tight edges and augment the matching. Thus, at any stage of the algorithm
/// the matching consists of tight edges. This means that the resulting perfect matching meets
/// complementary slackness conditions, and is therefore optimal.
///
/// At construction time the set of options can be specified to define the strategies used by the
/// algorithm to perform initialization, dual updates, etc. This can be done with the
/// `BlossomVOptions`. During the construction time the objective sense of the optimization
/// problem can be specified, i.e. whether to maximize of minimize the weight of the resulting
/// perfect matching. Default objective sense of the algorithm is to minimize the weight of the
/// resulting perfect matching. If the objective sense of the algorithm is to maximize the weight of
/// the matching, the problem is reduced to minimum weight perfect matching problem by multiplying
/// all edge weights by $-1$. This class supports retrieving statistics for the algorithm
/// performance, see `KolmogorovWeightedPerfectMatching#getStatistics()`. It provides the time
/// elapsed during primal operations and dual updates, as well as the number of these primal
/// operations performed.
///
/// The solution to a weighted perfect matching problem instance comes with a certificate of
/// optimality, which is represented by a solution to a dual linear program; see
/// `DualSolution`. This class encapsulates a mapping from the node sets of odd cardinality to
/// the corresponding dual variables. This mapping doesn't contain the sets whose dual variables are
/// $0$. The computation of the dual solution is performed lazily and doesn't affect the running time
/// of finding a weighted perfect matching.
///
/// Here we describe the certificates of optimality more precisely. Let the graph $G = (V, E)$ be an
/// undirected graph with cost function $c: V \mapsto \mathbb{R}$ defined on it. Let $\mathcal{O}$ be
/// the set of all subsets of $V$ of odd cardinality containing at least 3 vertices, and $\delta(S),
/// S \subset V$ be the set of boundary edges of $V$. Then **minimum** weight perfect matching
/// problem has the following linear programming formulation:
///
/// \[ \begin{align} \mbox{minimize} \qquad & \sum_{e\in E}c_e \cdot x_e &\\ s.t. \qquad
/// & \sum_{e\in \delta^(i)} x_e = 1 & \forall i\in V\\ & \sum_{e\in \delta(S)}x_e \ge 1
/// & \forall S\in \mathcal{O} \\ & x_e \ge 0 & \forall e\in E \end{align}\] The
/// corresponding dual linear program has the following form:
///
/// \[ \begin{align} \mbox{maximize} \qquad & \sum_{x \in V}y_e &\\ s.t. \qquad & y_u +
/// y_v + \sum_{S\in \mathcal{O}: e \in \delta(S)}y_S \le c_e & \forall\ e = \{u, v\}\in E\\
/// & x_S \ge 0 & \forall S\in \mathcal{O} \end{align} \] Let's use the following notation:
/// $slack(e) = c_e - y_u - y_v - \sum_{S\in \mathcal{O}: e \in \delta(S)}y_S$. Complementary
/// slackness conditions have the following form:
///
/// \[ \begin{align} slack(e) > 0 &\Rightarrow x_e = 0 \\ y_S > 0 &\Rightarrow
/// \sum_{e\in \delta(S)}x_e = 1 \end{align} \] Therefore, the slacks of all edges will be
/// non-negative and the slacks of matched edges will be $0$.
///
/// The **maximum** weight perfect matching problem has the following linear programming
/// formulation:
///
/// \[ \begin{align} \mbox{maximize} \qquad & \sum_{e\in E}c_e \cdot x_e &\\ s.t. \qquad
/// &\sum_{e\in \delta^(i)} x_e = 1 & \forall i\in V\\ & \sum_{e\in \delta(S)}x_e \ge 1
/// & \forall S\in \mathcal{O} \\ & x_e \ge 0 & \forall e\in E \end{align} \]
///
/// The corresponding dual linear program has the following form:
///
/// \[ \begin{align} \mbox{minimize} \qquad & \sum_{x \in V}y_e &\\ s.t. \qquad & y_u +
/// y_v + \sum_{S\in \mathcal{O}: e \in \delta(S)}y_S \ge c_e & \forall\ e = \{u, v\}\in E\\
/// & x_S \le 0 & \forall S\in \mathcal{O} \end{align} \]
///
/// Complementary slackness conditions have the following form:
///
/// \[ \begin{align} slack(e) < 0 &\Rightarrow x_e = 0 \\ y_S < 0 &\Rightarrow
/// \sum_{e\in \delta(S)}x_e = 1 \end{align} \]
///
/// Therefore, the slacks of all edges will be non-positive and the slacks of matched edges will be
/// $0$.
///
/// This class supports testing the optimality of the solution via
/// `KolmogorovWeightedPerfectMatching#testOptimality()`. It also supports retrieval of the
/// computation error when the edge weights are real values via
/// `KolmogorovWeightedPerfectMatching#getError()`. Both optimality test and error computation
/// are performed lazily and don't affect the running time of the main algorithm. If the problem
/// instance doesn't contain a perfect matching at all, the algorithm doesn't find a minimum weight
/// maximum matching; instead, it throws an exception.
///
/// # Rust notes
///
/// Kolmogorov's Blossom V algorithm for minimum- or maximum-weight perfect matchings.
pub struct KolmogorovWeightedPerfectMatching<'g, G: Graph> {
    graph: &'g G,
    options: BlossomVOptions,
    objective_sense: ObjectiveSense,
    matching: Option<Matching<G::Vertex, G::Edge>>,
    state: Option<BlossomVState<'g, G>>,
}

impl<'g, G: UndirectedGraph> KolmogorovWeightedPerfectMatching<'g, G> {
    /// Constructs a minimum-weight perfect-matching algorithm with default options.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(graph: &'g G) -> Result<Self> {
        Self::with_options_and_objective(
            graph,
            BlossomVOptions::default(),
            ObjectiveSense::Minimize,
        )
    }

    /// Constructs an algorithm with default options and the requested objective.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_objective(graph: &'g G, objective_sense: ObjectiveSense) -> Result<Self> {
        Self::with_options_and_objective(graph, BlossomVOptions::default(), objective_sense)
    }

    /// Constructs a minimum-weight algorithm with custom options.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_options(graph: &'g G, options: BlossomVOptions) -> Result<Self> {
        Self::with_options_and_objective(graph, options, ObjectiveSense::Minimize)
    }

    /// Constructs an algorithm with custom initialization, dual updates, and objective.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Infeasible`] if the problem has no feasible solution. Errors of the
    /// graph operations it performs are propagated.
    pub fn with_options_and_objective(
        graph: &'g G,
        options: BlossomVOptions,
        objective_sense: ObjectiveSense,
    ) -> Result<Self> {
        if graph.vertex_count() % 2 == 1 {
            return Err(GraphError::Infeasible);
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
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn test_optimality(&mut self) -> Result<bool> {
        Ok(self.get_error()? < BLOSSOM_V_EPSILON)
    }

    /// Returns the accumulated numerical error in the dual and complementary-slackness checks.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Internal`] if an internal invariant of the library is broken (a bug).
    /// Errors of the graph operations it performs are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn get_error(&mut self) -> Result<f64> {
        self.compute()?;
        let state = self
            .state
            .as_ref()
            .ok_or(crate::GraphError::Internal("the computed state is missing"))?;
        let matching = self.matching.as_ref().ok_or(crate::GraphError::Internal(
            "the computed matching is missing",
        ))?;
        let mut error = Self::nonnegativity_error(state);
        let matched: HashSet<_> = matching
            .edges()
            .iter()
            .map(|x| {
                let x = x.borrow();
                x.clone()
            })
            .collect();
        let scale = if self.objective_sense == ObjectiveSense::Maximize {
            -1.0
        } else {
            1.0
        };

        for edge in 0..state.edge_num {
            let graph_edge = &state.graph_edges[edge];
            let mut slack = scale * state.graph.weight_of(graph_edge) - state.min_edge_weight;
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
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Internal`] if an internal invariant of the library is broken (a bug).
    /// Errors of the graph operations it performs are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn get_dual_solution(&mut self) -> Result<BlossomVDualSolution<'g, G>> {
        self.compute()?;
        let state = self
            .state
            .as_ref()
            .ok_or(crate::GraphError::Internal("the computed state is missing"))?;
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

    /// Sets the currentEdge and currentDirection variables for all trees adjacent to the
    /// `tree`
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

    /// Finishes the algorithm after all nodes are matched. The main problem it solves is that the
    /// matching after the end of primal and dual operations may not be valid in the contracted
    /// blossoms.
    ///
    /// Property: if a matching is changed in the parent blossom, the matching in all lower blossoms
    /// can become invalid. Therefore, we traverse all nodes, find an unmatched node (it is
    /// necessarily contracted), go up to the first blossom whose matching hasn't been fixed (we set
    /// blossomGrandparent references to point to the previous nodes on the path). Then we start to
    /// change the matching accordingly all the way down to the initial node.
    ///
    /// Let's call an edge that is matched to a blossom root a "blossom edge". To make the matching
    /// valid we move the blossom edge one layer down at a time so that in the end its endpoints are
    /// valid initial nodes of the graph. After this transformation we can't traverse the
    /// blossomSibling references any more. That is why we initially compute a mapping of every
    /// pseudonode to the set of nodes that are contracted in it. This map is needed to construct a
    /// dual solution after the matching in the graph becomes valid.
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
                .ok_or(GraphError::Infeasible)?;
            let graph_edge = state.graph_edges[state.arena.edges[matched].pos].clone();
            if edges.insert(graph_edge.clone()) {
                weight += graph.weight_of(&graph_edge);
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

    /// Computes the sum of all duals from `start` inclusive to `end` inclusive
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

impl<G: UndirectedGraph> MatchingAlgorithm<G::Vertex, G::Edge>
    for KolmogorovWeightedPerfectMatching<'_, G>
{
    /// Computes and returns a weighted perfect matching in the `graph`. See the class
    /// description for the relative definitions and algorithm description.
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        self.compute()?;
        self.matching.clone().ok_or(crate::GraphError::Internal(
            "the computed matching is missing",
        ))
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
            let edge = edge.borrow();
            let source = graph.edge_source(edge).unwrap();
            let target = graph.edge_target(edge).unwrap();
            if source == target {
                continue;
            }
            let mut slack = graph.weight_of(edge);
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
        let graph = weighted_graph(4, &[(0, 1, 7.0), (1, 2, 4.0), (2, 3, 3.0), (3, 0, 4.0)]);
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
                    assert!(
                        dual.dual_variables()
                            .keys()
                            .all(|vertices| vertices.len() % 2 == 1)
                    );
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
