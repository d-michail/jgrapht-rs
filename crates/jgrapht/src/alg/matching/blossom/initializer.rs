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

use super::options::{BlossomVOptions, InitializationType};
use super::state::{BlossomVState, BlossomVStatistics};
use super::types::{BlossomArena, BlossomHeap, EdgeId, Label, NodeId};
use crate::error::{GraphError, Result};
use crate::graph::Graph;
use crate::graph::GraphExt;
use crate::util::OrderedF64;
use std::borrow::Borrow;
use std::collections::HashMap;

const INFINITY: f64 = 1e100;
const NO_PERFECT_MATCHING_THRESHOLD: f64 = 1e10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FractionalAction {
    None,
    Shrink,
    Augment,
}

/// Is used to start the Kolmogorov's Blossom V algorithm. Performs initialization of the algorithm's
/// internal data structures and finds an initial matching according to the strategy specified in
/// `options`.
///
/// The initialization process involves converting the graph into internal representation, allocating
/// trees for unmatched vertices, and creating an auxiliary graph whose nodes correspond to
/// alternating trees. The only part that varies is the strategy to find an initial matching to speed
/// up the main part of the algorithm.
///
/// The simple initialization (option `BlossomVOptions.InitializationType#NONE`) doesn't find
/// any matching and initializes the data structures by allocating $|V|$ single vertex trees. This is
/// the fastest initialization strategy; however, it slows the main algorithm down.
///
/// The greedy initialization (option `BlossomVOptions.InitializationType#GREEDY` runs in two
/// phases. First, for every node it determines an edge of minimum weight and assigns half of that
/// weight to the node's dual variable. This ensures that the slacks of all edges are non-negative.
/// After that it goes through all nodes again, greedily increases its dual variable and chooses an
/// incident matching edge if it is possible. After that every node is incident to at least one tight
/// edge. The resulting matching is an output of this initialization strategy.
///
/// The fractional matching initialization (option
/// `BlossomVOptions.InitializationType#FRACTIONAL`) is both the most complicated and the most
/// efficient type of initialization. The linear programming formulation of the fractional matching
/// problem is identical to the one used for bipartite graphs. More precisely: <oi>
/// - Minimize the $sum_{e\in E}x_e\times c_e$ subject to:
/// - For all nodes: $\sum_{e is incident to v}x_e = 1$
/// - For all edges: $x_e \ge 0$ </oi> **Note:** for an optimal solution in general graphs
///   we have to require the variables $x_e$ to be $0$ or $1$. For more information on this type of
///   initialization, see: *David Applegate and William J. Cook. \Solving Large-Scale Matching
///   Problems". In: Network Flows And Matching. 1991.*
pub(crate) struct BlossomVInitializer<'g, G: Graph> {
    graph: &'g G,
    weight_scale: f64,
}

impl<'g, G: Graph> BlossomVInitializer<'g, G> {
    #[cfg(test)]
    pub(crate) fn new(graph: &'g G) -> Self {
        Self {
            graph,
            weight_scale: 1.0,
        }
    }

    pub(crate) fn with_weight_scale(graph: &'g G, weight_scale: f64) -> Self {
        Self {
            graph,
            weight_scale,
        }
    }

    pub(crate) fn initialize(&self, options: BlossomVOptions) -> Result<BlossomVState<'g, G>> {
        match options.initialization_type() {
            InitializationType::None => Ok(self.simple_initialization(options)),
            InitializationType::Greedy => Ok(self.greedy_initialization(options)),
            InitializationType::Fractional => self.fractional_matching_initialization(options),
        }
    }

    fn fractional_matching_initialization(
        &self,
        options: BlossomVOptions,
    ) -> Result<BlossomVState<'g, G>> {
        let mut state = self.init_graph(options);
        self.init_greedy(&mut state);
        self.allocate_trees(&mut state);
        state.tree_num = self.init_fractional(&mut state)?;
        self.init_auxiliary_graph(&mut state);
        Ok(state)
    }

    fn simple_initialization(&self, options: BlossomVOptions) -> BlossomVState<'g, G> {
        let mut state = self.init_graph(options);
        for node in &mut state.arena.nodes {
            node.is_outer = true;
        }
        self.allocate_trees(&mut state);
        self.init_auxiliary_graph(&mut state);
        state.tree_num = state.node_num;
        state
    }

    fn greedy_initialization(&self, options: BlossomVOptions) -> BlossomVState<'g, G> {
        let mut state = self.init_graph(options);
        state.tree_num = self.init_greedy(&mut state);
        self.allocate_trees(&mut state);
        self.init_auxiliary_graph(&mut state);
        state
    }

    fn init_graph(&self, options: BlossomVOptions) -> BlossomVState<'g, G> {
        let node_num = self.graph.vertex_count();
        let graph_vertices: Vec<_> = self.graph.vertices_cloned().collect();
        let mut vertex_map = HashMap::with_capacity(node_num);
        let mut arena = BlossomArena::default();
        for (position, vertex) in graph_vertices.iter().enumerate() {
            let node = arena.add_node(position);
            vertex_map.insert(vertex.clone(), node);
        }
        let root_sentinel = arena.add_node(node_num);

        let min_edge_weight = self
            .graph
            .edges()
            .map(|edge| {
                let edge = edge.borrow();
                self.weight_scale * self.graph.weight_of(edge)
            })
            .min_by(f64::total_cmp)
            .unwrap_or(0.0);
        let mut graph_edges = Vec::with_capacity(self.graph.edge_count());
        for edge in self.graph.edges() {
            let edge = edge.borrow();
            let source_vertex = self.graph.edge_source(edge).expect("graph edge source");
            let target_vertex = self.graph.edge_target(edge).expect("graph edge target");
            let source = vertex_map[source_vertex];
            let target = vertex_map[target_vertex];
            if source == target {
                continue;
            }
            let position = graph_edges.len();
            arena.add_edge(
                source,
                target,
                self.weight_scale * self.graph.weight_of(edge) - min_edge_weight,
                position,
            );
            graph_edges.push(edge.clone());
        }
        let edge_num = graph_edges.len();

        BlossomVState {
            graph: self.graph,
            arena,
            node_num,
            edge_num,
            tree_num: 0,
            removed_num: 0,
            blossom_num: 0,
            statistics: BlossomVStatistics::default(),
            options,
            graph_vertices,
            graph_edges,
            min_edge_weight,
            root_sentinel,
        }
    }

    fn init_greedy(&self, state: &mut BlossomVState<'g, G>) -> usize {
        for node in &mut state.arena.nodes[..state.node_num] {
            node.dual = f64::INFINITY;
        }
        for edge in 0..state.edge_num {
            let slack = state.arena.edges[edge].slack;
            for node in state.arena.edges[edge].head.into_iter().flatten() {
                state.arena.nodes[node].dual = state.arena.nodes[node].dual.min(slack);
            }
        }

        for edge in 0..state.edge_num {
            let [Some(source), Some(target)] = state.arena.edges[edge].head else {
                unreachable!("initialized edge has endpoints")
            };
            if !state.arena.nodes[source].is_outer {
                state.arena.nodes[source].is_outer = true;
                state.arena.nodes[source].dual /= 2.0;
            }
            state.arena.edges[edge].slack -= state.arena.nodes[source].dual;
            if !state.arena.nodes[target].is_outer {
                state.arena.nodes[target].is_outer = true;
                state.arena.nodes[target].dual /= 2.0;
            }
            state.arena.edges[edge].slack -= state.arena.nodes[target].dual;
        }

        let mut tree_num = state.node_num;
        for node in 0..state.node_num {
            if state.arena.nodes[node].is_infinity() {
                continue;
            }
            let incident = state.arena.incident_edges(node);
            let min_slack = incident
                .iter()
                .map(|&(edge, _)| state.arena.edges[edge].slack)
                .min_by(f64::total_cmp)
                .unwrap_or(f64::INFINITY);
            state.arena.nodes[node].dual += min_slack;
            for (edge, dir) in incident {
                let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
                if state.arena.edges[edge].slack <= min_slack
                    && state.arena.nodes[node].is_plus()
                    && state.arena.nodes[opposite].is_plus()
                {
                    state.arena.nodes[node].label = Label::Infinity;
                    state.arena.nodes[opposite].label = Label::Infinity;
                    state.arena.nodes[node].matched = Some(edge);
                    state.arena.nodes[opposite].matched = Some(edge);
                    tree_num -= 2;
                }
                state.arena.edges[edge].slack -= min_slack;
            }
        }
        tree_num
    }

    fn add_best_edge(
        &self,
        state: &mut BlossomVState<'g, G>,
        heap: &mut BlossomHeap,
        node: NodeId,
        edge: EdgeId,
    ) {
        let handle = heap.insert(OrderedF64(state.arena.edges[edge].slack), edge);
        state.arena.edges[edge].handle = Some(handle);
        state.arena.nodes[node].best_edge = Some(edge);
    }

    fn remove_best_edge(
        &self,
        state: &mut BlossomVState<'g, G>,
        heap: &mut BlossomHeap,
        node: NodeId,
    ) {
        let edge = state.arena.nodes[node]
            .best_edge
            .expect("infinity node has a best edge");
        let handle = state.arena.edges[edge]
            .handle
            .take()
            .expect("best edge has a heap handle");
        heap.delete(handle).expect("valid best-edge heap handle");
        state.arena.nodes[node].best_edge = None;
    }

    fn update_fractional_duals(
        &self,
        state: &mut BlossomVState<'g, G>,
        heap: &mut BlossomHeap,
        root: NodeId,
        eps: f64,
    ) {
        for plus in state.arena.tree_nodes(root) {
            if !state.arena.nodes[plus].is_processed {
                continue;
            }
            state.arena.nodes[plus].dual += eps;
            if !state.arena.nodes[plus].is_tree_root {
                let minus = state.arena.node_opposite_matched(plus);
                state.arena.nodes[minus].dual -= eps;
                let matched = state.arena.nodes[plus].matched.expect("matched plus node");
                let delta = eps - state.arena.edges[matched].slack;
                for (edge, _) in state.arena.incident_edges(minus) {
                    state.arena.edges[edge].slack += delta;
                }
            }
            for (edge, _) in state.arena.incident_edges(plus) {
                state.arena.edges[edge].slack -= eps;
            }
            state.arena.nodes[plus].is_processed = false;
        }
        while let Some(edge) = heap.peek_entry().map(|(_, _, edge)| *edge) {
            let [Some(a), Some(b)] = state.arena.edges[edge].head else {
                unreachable!("best edge endpoints")
            };
            let node = if state.arena.nodes[a].is_infinity() {
                a
            } else {
                b
            };
            self.remove_best_edge(state, heap, node);
        }
    }

    fn expand_fractional_cycle(
        &self,
        state: &mut BlossomVState<'g, G>,
        node: NodeId,
        matched: Option<EdgeId>,
    ) {
        let sibling = state.arena.nodes[node]
            .blossom_sibling
            .expect("fractional blossom sibling");
        let mut current = state
            .arena
            .edge_opposite(sibling, node)
            .expect("sibling endpoint");
        state.arena.nodes[node].is_outer = true;
        state.arena.nodes[node].label = Label::Infinity;
        state.arena.nodes[node].matched = matched;
        while current != node {
            let edge = state.arena.nodes[current]
                .blossom_sibling
                .expect("fractional blossom sibling");
            state.arena.nodes[current].matched = Some(edge);
            state.arena.nodes[current].is_outer = true;
            state.arena.nodes[current].label = Label::Infinity;
            current = state
                .arena
                .edge_opposite(edge, current)
                .expect("sibling endpoint");

            state.arena.nodes[current].matched = Some(edge);
            state.arena.nodes[current].is_outer = true;
            state.arena.nodes[current].label = Label::Infinity;
            let next = state.arena.nodes[current]
                .blossom_sibling
                .expect("fractional blossom sibling");
            current = state
                .arena
                .edge_opposite(next, current)
                .expect("sibling endpoint");
        }
    }

    fn finish_fractional(&self, state: &mut BlossomVState<'g, G>) -> usize {
        let mut previous_root = state.root_sentinel;
        state.arena.nodes[state.root_sentinel].tree_sibling_next = None;
        let mut tree_num = 0;
        for node in 0..state.node_num {
            state.arena.nodes[node].first_tree_child = None;
            state.arena.nodes[node].tree_sibling_next = None;
            state.arena.nodes[node].tree_sibling_prev = None;
            if !state.arena.nodes[node].is_outer {
                self.expand_fractional_cycle(state, node, None);
                state.arena.nodes[node].parent_edge = None;
                state.arena.nodes[node].label = Label::Plus;
                state.arena.add_tree(node);
                state.arena.nodes[previous_root].tree_sibling_next = Some(node);
                state.arena.nodes[node].tree_sibling_prev = Some(previous_root);
                previous_root = node;
                tree_num += 1;
            }
        }
        tree_num
    }

    fn handle_fractional_infinity_edge(
        &self,
        state: &mut BlossomVState<'g, G>,
        heap: &mut BlossomHeap,
        edge: EdgeId,
        direction: usize,
        eps: f64,
        critical_eps: f64,
    ) {
        let in_tree = state.arena.edges[edge].head[1 - direction].expect("tree endpoint");
        let opposite = state.arena.edges[edge].head[direction].expect("infinity endpoint");
        if state.arena.edges[edge].slack > eps {
            if state.arena.edges[edge].slack < critical_eps {
                if let Some(best) = state.arena.nodes[opposite].best_edge {
                    if state.arena.edges[edge].slack < state.arena.edges[best].slack {
                        self.remove_best_edge(state, heap, opposite);
                        self.add_best_edge(state, heap, opposite, edge);
                    }
                } else {
                    self.add_best_edge(state, heap, opposite, edge);
                }
            }
            return;
        }

        if state.arena.nodes[opposite].best_edge.is_some() {
            self.remove_best_edge(state, heap, opposite);
        }
        state.arena.nodes[opposite].label = Label::Minus;
        state.arena.add_child(in_tree, opposite, edge, true);
        let plus = state.arena.node_opposite_matched(opposite);
        if state.arena.nodes[plus].best_edge.is_some() {
            self.remove_best_edge(state, heap, plus);
        }
        state.arena.nodes[plus].label = Label::Plus;
        let matched = state.arena.nodes[plus].matched.expect("matched plus node");
        state.arena.add_child(opposite, plus, matched, true);
    }

    fn find_fractional_blossom_root(
        &self,
        state: &mut BlossomVState<'g, G>,
        forming_edge: EdgeId,
    ) -> NodeId {
        let [Some(a), Some(b)] = state.arena.edges[forming_edge].head else {
            unreachable!("forming edge endpoints")
        };
        let mut branches = [a, b];
        let mut direction = 0;
        let (root, upper_bound) = loop {
            if !state.arena.nodes[branches[direction]].is_outer {
                break (branches[direction], branches[1 - direction]);
            }
            state.arena.nodes[branches[direction]].is_outer = false;
            if state.arena.nodes[branches[direction]].is_tree_root {
                let upper_bound = branches[direction];
                let mut jump = branches[1 - direction];
                while state.arena.nodes[jump].is_outer {
                    state.arena.nodes[jump].is_outer = false;
                    jump = state.arena.node_tree_parent(jump).expect("minus parent");
                    state.arena.nodes[jump].is_outer = false;
                    jump = state.arena.node_tree_parent(jump).expect("plus parent");
                }
                break (jump, upper_bound);
            }
            let parent = state
                .arena
                .node_tree_parent(branches[direction])
                .expect("minus parent");
            state.arena.nodes[parent].is_outer = false;
            branches[direction] = state.arena.node_tree_parent(parent).expect("plus parent");
            direction = 1 - direction;
        };

        let mut jump = root;
        while jump != upper_bound {
            jump = state.arena.node_tree_parent(jump).expect("minus parent");
            state.arena.nodes[jump].is_outer = true;
            jump = state.arena.node_tree_parent(jump).expect("plus parent");
            state.arena.nodes[jump].is_outer = true;
        }
        root
    }

    fn augment_fractional_branch(
        &self,
        state: &mut BlossomVState<'g, G>,
        root: NodeId,
        branch_start: NodeId,
        augment_edge: EdgeId,
    ) {
        for node in state.arena.tree_nodes(root) {
            state.arena.nodes[node].label = Label::Infinity;
        }
        let mut plus = branch_start;
        let mut minus = state.arena.node_tree_parent(branch_start);
        let mut matched = augment_edge;
        while let Some(minus_node) = minus {
            state.arena.nodes[plus].matched = Some(matched);
            matched = state.arena.nodes[minus_node]
                .parent_edge
                .expect("minus parent edge");
            state.arena.nodes[minus_node].matched = Some(matched);
            plus = state
                .arena
                .node_tree_parent(minus_node)
                .expect("plus parent");
            minus = state.arena.node_tree_parent(plus);
        }
        state.arena.nodes[root].matched = Some(matched);
        state.arena.remove_from_child_list(root);
        state.arena.nodes[root].is_tree_root = false;
        if let Some(tree) = state.arena.nodes[root].tree {
            state.arena.trees[tree].active = false;
        }
    }

    fn shrink_fractional_cycle(
        &self,
        state: &mut BlossomVState<'g, G>,
        forming_edge: EdgeId,
        tree_root: NodeId,
    ) {
        for node in state.arena.tree_nodes(tree_root) {
            state.arena.nodes[node].label = Label::Infinity;
        }
        let blossom_root = self.find_fractional_blossom_root(state, forming_edge);
        if !state.arena.nodes[blossom_root].is_tree_root {
            let mut minus = state
                .arena
                .node_tree_parent(blossom_root)
                .expect("blossom root parent");
            let mut previous = state.arena.nodes[minus]
                .parent_edge
                .expect("minus parent edge");
            state.arena.nodes[minus].matched = Some(previous);
            let mut plus = state.arena.node_tree_parent(minus).expect("plus parent");
            while plus != tree_root {
                minus = state.arena.node_tree_parent(plus).expect("minus parent");
                state.arena.nodes[plus].matched = Some(previous);
                previous = state.arena.nodes[minus]
                    .parent_edge
                    .expect("minus parent edge");
                state.arena.nodes[minus].matched = Some(previous);
                plus = state.arena.node_tree_parent(minus).expect("plus parent");
            }
            state.arena.nodes[plus].matched = Some(previous);
        }

        let mut previous = forming_edge;
        for (current, direction) in state.arena.blossom_nodes(blossom_root, forming_edge) {
            state.arena.nodes[current].label = Label::Plus;
            if direction == 0 {
                state.arena.nodes[current].blossom_sibling = Some(previous);
                if let Some(parent_edge) = state.arena.nodes[current].parent_edge {
                    previous = parent_edge;
                }
            } else {
                state.arena.nodes[current].blossom_sibling = state.arena.nodes[current].parent_edge;
            }
        }
        state.arena.remove_from_child_list(tree_root);
        state.arena.nodes[tree_root].is_tree_root = false;
        if let Some(tree) = state.arena.nodes[tree_root].tree {
            state.arena.trees[tree].active = false;
        }
    }

    fn init_fractional(&self, state: &mut BlossomVState<'g, G>) -> Result<usize> {
        let mut heap = BlossomHeap::new();
        let mut root = state.arena.nodes[state.root_sentinel].tree_sibling_next;
        while let Some(tree_root) = root {
            let next_root = state.arena.nodes[tree_root].tree_sibling_next;
            let next_next_root =
                next_root.and_then(|node| state.arena.nodes[node].tree_sibling_next);
            let tree = state.arena.nodes[tree_root].tree.expect("fractional tree");
            heap.clear();

            let mut current = tree_root;
            let mut branch_root = current;
            let mut branch_eps = 0.0;
            let mut action = FractionalAction::None;
            let mut critical_edge = None;
            let mut critical_eps = INFINITY;
            let mut critical_direction = 0;
            let mut primal_operation = false;

            loop {
                state.arena.nodes[current].is_processed = true;
                state.arena.nodes[current].dual -= branch_eps;
                if !state.arena.nodes[current].is_tree_root {
                    let minus = state.arena.node_opposite_matched(current);
                    state.arena.nodes[minus].dual += branch_eps;
                }

                let incident = state.arena.incident_edges(current);
                let mut processed = 0;
                for &(edge, direction) in &incident {
                    processed += 1;
                    state.arena.edges[edge].slack += branch_eps;
                    let opposite = state.arena.edges[edge].head[direction]
                        .expect("fractional edge opposite endpoint");
                    if state.arena.nodes[opposite].tree == Some(tree) {
                        if state.arena.nodes[opposite].is_plus() {
                            let mut slack = state.arena.edges[edge].slack;
                            if !state.arena.nodes[opposite].is_processed {
                                slack += branch_eps;
                            }
                            if 2.0 * critical_eps > slack || critical_edge.is_none() {
                                action = FractionalAction::Shrink;
                                critical_eps = slack / 2.0;
                                critical_edge = Some(edge);
                                critical_direction = direction;
                                if critical_eps <= branch_eps {
                                    primal_operation = true;
                                    break;
                                }
                            }
                        }
                    } else if state.arena.nodes[opposite].is_plus() {
                        if critical_eps >= state.arena.edges[edge].slack || critical_edge.is_none()
                        {
                            action = FractionalAction::Augment;
                            critical_eps = state.arena.edges[edge].slack;
                            critical_edge = Some(edge);
                            critical_direction = direction;
                            if critical_eps <= branch_eps {
                                primal_operation = true;
                                break;
                            }
                        }
                    } else {
                        self.handle_fractional_infinity_edge(
                            state,
                            &mut heap,
                            edge,
                            direction,
                            branch_eps,
                            critical_eps,
                        );
                    }
                }

                if primal_operation {
                    for &(edge, _) in &incident[processed..] {
                        state.arena.edges[edge].slack += branch_eps;
                    }
                    break;
                }

                if let Some(child) = state.arena.nodes[current].first_tree_child {
                    current = state.arena.node_opposite_matched(child);
                    continue;
                }
                while current != branch_root
                    && state.arena.nodes[current].tree_sibling_next.is_none()
                {
                    current = state
                        .arena
                        .node_tree_parent(current)
                        .expect("fractional tree parent");
                }
                if state.arena.nodes[current].is_minus() {
                    let sibling = state.arena.nodes[current]
                        .tree_sibling_next
                        .expect("unprocessed sibling");
                    current = state.arena.node_opposite_matched(sibling);
                } else if current == branch_root {
                    let min_edge = heap.peek_entry().map(|(_, _, edge)| *edge);
                    if min_edge.is_none_or(|edge| state.arena.edges[edge].slack >= critical_eps) {
                        if critical_eps > NO_PERFECT_MATCHING_THRESHOLD {
                            return Err(GraphError::Infeasible);
                        }
                        branch_eps = critical_eps;
                        break;
                    }

                    let edge = min_edge.expect("minimum best edge");
                    let [Some(a), Some(_b)] = state.arena.edges[edge].head else {
                        unreachable!("best edge endpoints")
                    };
                    let direction_to_free = usize::from(!state.arena.nodes[a].is_infinity());
                    current = state.arena.edges[edge].head[1 - direction_to_free]
                        .expect("in-tree endpoint");
                    let minus =
                        state.arena.edges[edge].head[direction_to_free].expect("free endpoint");
                    self.remove_best_edge(state, &mut heap, minus);
                    state.arena.nodes[minus].label = Label::Minus;
                    state.arena.add_child(current, minus, edge, true);
                    branch_eps = state.arena.edges[edge].slack;
                    let plus = state.arena.node_opposite_matched(minus);
                    if state.arena.nodes[plus].best_edge.is_some() {
                        self.remove_best_edge(state, &mut heap, plus);
                    }
                    state.arena.nodes[plus].label = Label::Plus;
                    let matched = state.arena.nodes[minus].matched.expect("matched free node");
                    state.arena.add_child(minus, plus, matched, true);
                    current = plus;
                    branch_root = plus;
                }
            }

            self.update_fractional_duals(state, &mut heap, tree_root, branch_eps);
            let edge = critical_edge.ok_or(GraphError::Infeasible)?;
            let from = state.arena.edges[edge].head[1 - critical_direction]
                .expect("critical edge tree endpoint");
            let to = state.arena.edges[edge].head[critical_direction]
                .expect("critical edge opposite endpoint");
            match action {
                FractionalAction::Shrink => {
                    self.shrink_fractional_cycle(state, edge, tree_root);
                }
                FractionalAction::Augment => {
                    self.augment_fractional_branch(state, tree_root, from, edge);
                    if state.arena.nodes[to].is_outer {
                        self.augment_fractional_branch(state, to, to, edge);
                    } else {
                        self.expand_fractional_cycle(state, to, Some(edge));
                    }
                }
                FractionalAction::None => {
                    return Err(GraphError::Infeasible);
                }
            }

            root = next_root;
            if root.is_some_and(|node| !state.arena.nodes[node].is_tree_root) {
                root = next_next_root;
            }
        }
        Ok(self.finish_fractional(state))
    }

    fn allocate_trees(&self, state: &mut BlossomVState<'g, G>) {
        let mut last_root = state.root_sentinel;
        for node in 0..state.node_num {
            if state.arena.nodes[node].is_plus() {
                state.arena.nodes[node].tree_sibling_prev = Some(last_root);
                state.arena.nodes[last_root].tree_sibling_next = Some(node);
                last_root = node;
                state.arena.add_tree(node);
            }
        }
        state.arena.nodes[last_root].tree_sibling_next = None;
    }

    fn init_auxiliary_graph(&self, state: &mut BlossomVState<'g, G>) {
        let mut root = state.arena.nodes[state.root_sentinel].tree_sibling_next;
        while let Some(current_root) = root {
            let tree = state.arena.nodes[current_root].tree.expect("root tree");
            for (edge, dir) in state.arena.incident_edges(current_root) {
                let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
                if state.arena.nodes[opposite].is_infinity() {
                    state.arena.add_plus_infinity_edge(tree, edge);
                } else if !state.arena.nodes[opposite].is_processed {
                    let opposite_tree = state.arena.nodes[opposite].tree.expect("opposite tree");
                    if state.arena.trees[opposite_tree].current_edge.is_none() {
                        state.arena.add_tree_edge(tree, opposite_tree);
                    }
                    let tree_edge = state.arena.trees[opposite_tree]
                        .current_edge
                        .expect("tree edge was assigned");
                    self.add_tree_edge_plus_plus(&mut state.arena, tree_edge, edge);
                }
            }
            state.arena.nodes[current_root].is_processed = true;
            for (tree_edge, direction) in state.arena.tree_edges(tree) {
                let opposite_tree =
                    state.arena.tree_edges[tree_edge].head[direction].expect("active tree edge");
                state.arena.trees[opposite_tree].current_edge = None;
            }
            root = state.arena.nodes[current_root].tree_sibling_next;
        }

        let mut root = state.arena.nodes[state.root_sentinel].tree_sibling_next;
        while let Some(current_root) = root {
            state.arena.nodes[current_root].is_processed = false;
            root = state.arena.nodes[current_root].tree_sibling_next;
        }
    }

    fn add_tree_edge_plus_plus(&self, arena: &mut BlossomArena, tree_edge: usize, edge: EdgeId) {
        let handle = arena.tree_edges[tree_edge]
            .plus_plus_edges
            .insert(crate::util::OrderedF64(arena.edges[edge].slack), edge);
        arena.edges[edge].handle = Some(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alg::matching::MatchingAlgorithm;
    use crate::alg::matching::blossom::KolmogorovWeightedPerfectMatching;
    use crate::graph::{GraphMut, GraphMutExt, UGraph};
    use std::collections::HashSet;

    fn graph(edges: &[(i32, i32, f64)], isolated: &[i32]) -> UGraph<i32> {
        let mut graph = UGraph::<i32>::simple().weighted();
        for &vertex in isolated {
            graph.add_vertex(vertex);
        }
        for &(source, target, weight) in edges {
            graph.add_vertex(source);
            graph.add_vertex(target);
            graph
                .add_weighted_edge(&source, &target, weight)
                .expect("add weighted edge");
        }
        graph
    }

    #[test]
    fn simple_initialization_matches_java_state() {
        let graph = graph(
            &[
                (1, 2, 1.0),
                (2, 3, 2.0),
                (2, 5, 3.0),
                (4, 5, 4.0),
                (5, 6, 5.0),
                (8, 9, 0.0),
            ],
            &[7],
        );
        let state = BlossomVInitializer::new(&graph)
            .initialize(BlossomVOptions::with_initialization(
                InitializationType::None,
            ))
            .unwrap();
        assert_eq!(state.node_num, 9);
        assert_eq!(state.tree_num, 9);
        assert_eq!(state.edge_num, 6);
        assert!(
            state.arena.nodes[..state.node_num]
                .iter()
                .all(|node| node.is_outer && node.is_tree_root && node.dual == 0.0)
        );
        assert_eq!(
            state
                .arena
                .edges
                .iter()
                .map(|edge| edge.slack)
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 0.0]
        );
        let roots: HashSet<_> = state.arena.nodes[state.root_sentinel]
            .tree_sibling_next
            .into_iter()
            .flat_map(|first| {
                std::iter::successors(Some(first), |&node| {
                    state.arena.nodes[node].tree_sibling_next
                })
            })
            .collect();
        assert_eq!(roots.len(), 9);
        let isolated = state
            .graph_vertices
            .iter()
            .position(|vertex| *vertex == 7)
            .expect("isolated vertex");
        let tree = state.arena.nodes[isolated].tree.expect("isolated tree");
        assert!(state.arena.tree_edges(tree).is_empty());
        assert!(state.arena.incident_edges(isolated).is_empty());
    }

    #[test]
    fn greedy_initialization_matches_java_state() {
        let graph = graph(&[(1, 2, 5.0), (3, 4, 0.0)], &[]);
        let state = BlossomVInitializer::new(&graph)
            .initialize(BlossomVOptions::with_initialization(
                InitializationType::Greedy,
            ))
            .unwrap();
        assert_eq!(state.node_num, 4);
        assert_eq!(state.edge_num, 2);
        assert_eq!(state.tree_num, 0);
        assert!(
            state.arena.nodes[..state.node_num]
                .iter()
                .all(|node| node.is_outer && !node.is_tree_root && node.matched.is_some())
        );
        for edge in 0..state.edge_num {
            let [Some(a), Some(b)] = state.arena.edges[edge].head else {
                unreachable!()
            };
            let original_weight = graph.edge_weight(&state.graph_edges[edge]).unwrap();
            assert_eq!(
                state.arena.nodes[a].dual + state.arena.nodes[b].dual,
                original_weight
            );
            assert_eq!(state.arena.edges[edge].slack, 0.0);
        }
        assert!(
            state.arena.nodes[state.root_sentinel]
                .tree_sibling_next
                .is_none()
        );
    }

    #[test]
    fn fractional_initialization_matches_java_cases() {
        let cases: &[(&[(i32, i32, f64)], f64, bool)] = &[
            (
                &[
                    (0, 3, 8.),
                    (0, 4, 3.),
                    (0, 5, 3.),
                    (1, 3, 2.),
                    (1, 4, 5.),
                    (1, 5, 2.),
                    (2, 3, 7.),
                    (2, 4, 3.),
                    (2, 5, 4.),
                ],
                8.,
                true,
            ),
            (
                &[
                    (0, 3, 4.),
                    (0, 4, 4.),
                    (0, 5, 4.),
                    (1, 3, 5.),
                    (1, 4, 8.),
                    (1, 5, 10.),
                    (2, 3, 4.),
                    (2, 4, 6.),
                    (2, 5, 5.),
                ],
                14.,
                true,
            ),
            (
                &[
                    (0, 5, 6.),
                    (0, 6, 8.),
                    (1, 5, 5.),
                    (1, 6, 5.),
                    (1, 7, 3.),
                    (2, 4, 2.),
                    (2, 5, 1.),
                    (2, 6, 8.),
                    (3, 5, 5.),
                    (3, 7, 9.),
                ],
                18.,
                true,
            ),
            (
                &[
                    (0, 5, 2.),
                    (0, 6, 2.),
                    (0, 7, 1.),
                    (1, 4, 6.),
                    (1, 7, 10.),
                    (2, 4, 7.),
                    (2, 6, 8.),
                    (2, 7, 10.),
                    (3, 4, 5.),
                    (3, 5, 9.),
                ],
                24.,
                true,
            ),
            (
                &[
                    (1, 0, 2.),
                    (1, 2, 5.),
                    (0, 2, 4.),
                    (1, 4, 5.),
                    (2, 4, 2.),
                    (1, 3, 2.),
                    (1, 5, 4.),
                    (3, 5, 3.),
                    (4, 5, 5.),
                    (3, 6, 4.),
                    (5, 6, 2.),
                    (5, 7, 3.),
                    (6, 7, 4.),
                    (4, 7, 4.),
                ],
                11.,
                false,
            ),
            (
                &[
                    (0, 1, 5.),
                    (0, 2, 9.),
                    (1, 2, 6.),
                    (2, 3, 4.),
                    (2, 4, 5.),
                    (3, 4, 3.),
                    (1, 4, 8.),
                    (1, 5, 8.),
                    (0, 5, 11.),
                    (4, 5, 7.),
                    (4, 6, 3.),
                    (5, 6, 5.),
                    (6, 7, 3.),
                    (5, 7, 6.),
                    (4, 7, 6.),
                    (3, 7, 9.),
                ],
                18.,
                false,
            ),
            (
                &[
                    (0, 1, 2.),
                    (0, 2, 8.),
                    (1, 2, 7.),
                    (0, 4, 8.),
                    (1, 4, 7.),
                    (2, 4, 6.),
                    (2, 3, 9.),
                    (2, 5, 6.),
                    (3, 5, 6.),
                    (2, 6, 6.),
                    (5, 6, 5.),
                    (4, 6, 2.),
                    (5, 7, 9.),
                    (6, 7, 7.),
                    (3, 7, 14.),
                    (4, 7, 7.),
                    (0, 7, 15.),
                ],
                21.,
                false,
            ),
            (
                &[
                    (0, 1, 7.),
                    (0, 2, 8.),
                    (0, 3, 8.),
                    (1, 3, 4.),
                    (1, 5, 9.),
                    (1, 6, 13.),
                    (2, 4, 6.),
                    (2, 3, 11.),
                    (3, 4, 10.),
                    (3, 5, 6.),
                    (4, 5, 8.),
                    (4, 7, 7.),
                    (5, 6, 4.),
                    (5, 7, 4.),
                    (6, 7, 1.),
                ],
                20.,
                false,
            ),
            (
                &[
                    (0, 1, 4.),
                    (0, 2, 4.),
                    (0, 5, 14.),
                    (1, 2, 3.),
                    (1, 3, 1.),
                    (1, 5, 11.),
                    (2, 3, 4.),
                    (2, 4, 4.),
                    (2, 7, 11.),
                    (3, 4, 1.),
                    (3, 5, 10.),
                    (4, 5, 10.),
                    (4, 6, 10.),
                    (4, 7, 9.),
                    (5, 6, 3.),
                    (6, 7, 8.),
                ],
                17.,
                false,
            ),
        ];

        for (index, &(edges, expected, no_main_phase_operations)) in cases.iter().enumerate() {
            let graph = graph(edges, &[]);
            let mut algorithm = KolmogorovWeightedPerfectMatching::with_options(
                &graph,
                BlossomVOptions::with_initialization(InitializationType::Fractional),
            )
            .unwrap();
            assert_eq!(
                algorithm.matching().unwrap().weight(),
                expected,
                "Java fractional case {}",
                index + 1
            );
            assert!(algorithm.test_optimality().unwrap());
            if no_main_phase_operations {
                let statistics = algorithm.statistics();
                assert_eq!(statistics.grow_count(), 0);
                assert_eq!(statistics.shrink_count(), 0);
                assert_eq!(statistics.expand_count(), 0);
            }
        }
    }
}
