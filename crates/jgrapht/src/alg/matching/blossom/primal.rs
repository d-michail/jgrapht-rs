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

use super::state::BlossomVState;
use super::types::{BlossomArena, EdgeId, Label, NodeId, TreeEdgeId, TreeId};
use crate::graph::Graph;
use crate::util::OrderedF64;
use std::mem;
use std::time::Instant;

/// This class is used by `KolmogorovWeightedPerfectMatching` for performing primal operations:
/// grow, augment, shrink and expand. This class operates on alternating trees, blossom structures,
/// and node states. It changes them after applying any primal operation. Also, this class can add
/// and subtract some values from nodes' dual variables; it never changes their actual dual
/// variables.
///
/// The augment operation is used to increase the cardinality of the matching. It is applied to a
/// tight (+, +) cross-tree edge. Its main purpose is to alter the matching on the simple path
/// between tree roots through the tight edge, destroy the previous tree structures, update the state
/// of the node, and change the presence of edges in the priority queues. This operation doesn't
/// destroy the tree structure; this technique is called *lazy tree structure destroying*. The
/// information of the nodes from the tree structure block is overwritten when a node is being added
/// to another tree. This operation doesn't change the matching in the contracted blossoms.
///
/// The grow operation is used to add new nodes to a given tree. This operation is applied only to
/// tight infinity edges. It always adds even number of nodes. This operation can grow the tree
/// recursively in the depth-first order. If it encounters a tight (+, +) cross-tree edge, it stops
/// growing and performs immediate augmentation.
///
/// The shrink operation contracts an odd node circuit and introduces a new pseudonode. It is applied
/// to tight (+, +) in-tree edges. It changes the state so than the contracted nodes don't appear in
/// the surface graph. If during the changing of the endpoints of boundary edge a tight (+, +)
/// cross-tree edge is encountered, an immediate augmentation is performed.
///
/// The expand operation brings the contracted blossom nodes to the surface graph. It is applied only
/// to a "-" blossom with zero dual variable. The operation determines the two branches of a blossom:
/// an even and an odd one. The formercontains an even number of edges and can be empty, the latter
/// contains an odd number of edges and necessarily contains at least one edge. An even branch is
/// inserted into the tree. The state of the algorithm is changes respectively (node duals, tree
/// structure, etc.). If some boundary edge in a tight (+, +) cross-tree edge, an immediate
/// augmentation is performed.
///
/// The immediate augmentations are used to speed up the algorithm. More detailed description of the
/// primal operations can be found in their Javadoc.
pub(crate) struct BlossomVPrimalUpdater;

impl BlossomVPrimalUpdater {
    /// Performs grow operation. This is invoked on the plus-infinity `growEdge`, which
    /// connects a "+" node in the tree and an infinity matched node. The `growEdge` and the
    /// matched free edge are added to the tree structure. Two new nodes are added to the tree: minus
    /// node and plus node. Let's call the node incident to the `growEdge` and opposite to the
    /// minusNode the "tree node".
    ///
    /// As the result, following actions are performed:
    ///
    /// - Add new child to the children of tree node and minus node
    /// - Set parent edges of minus and plus nodes
    /// - If minus node is a blossom, add it to the heap of "-" blossoms
    /// - Remove growEdge from the heap of infinity edges
    /// - Remove former infinity edges and add new (+, +) in-tree and cross-tree edges, (+, -)
    ///   cross tree edges to the appropriate heaps (due to the changes of the labels of the minus and
    ///   plus nodes)
    /// - Add new infinity edge from the plus node
    /// - Add new tree edges is necessary
    /// - Subtract tree.eps from the slacks of all edges incident to the minus node
    /// - Add tree.eps to the slacks of all edges incident to the plus node
    ///
    /// If the `manyGrows` flag is true, performs recursive growing of the tree.
    pub(crate) fn grow<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        grow_edge: EdgeId,
        recursive_grow: bool,
        immediate_augment: bool,
    ) {
        let start = Instant::now();
        let initial_tree_num = state.tree_num;
        let dir_to_minus = if state.arena.edges[grow_edge].head[0]
            .is_some_and(|node| state.arena.nodes[node].is_infinity())
        {
            0
        } else {
            1
        };
        let node_in_tree =
            state.arena.edges[grow_edge].head[1 - dir_to_minus].expect("grow edge tree endpoint");
        let mut minus =
            state.arena.edges[grow_edge].head[dir_to_minus].expect("grow edge infinity endpoint");
        let mut plus = state.arena.node_opposite_matched(minus);
        let matched = state.arena.nodes[minus]
            .matched
            .expect("matched infinity node");
        state.arena.add_child(node_in_tree, minus, grow_edge, true);
        state.arena.add_child(minus, plus, matched, true);
        let stop = plus;

        loop {
            state.arena.nodes[minus].label = Label::Minus;
            state.arena.nodes[plus].label = Label::Plus;
            state.arena.nodes[minus].is_marked = false;
            state.arena.nodes[plus].is_marked = false;
            Self::process_minus_node_grow(state, minus);
            Self::process_plus_node_grow(state, plus, recursive_grow, immediate_augment);
            if initial_tree_num != state.tree_num {
                break;
            }

            if let Some(child) = state.arena.nodes[plus].first_tree_child {
                minus = child;
                plus = state.arena.node_opposite_matched(minus);
            } else {
                while plus != stop && state.arena.nodes[plus].tree_sibling_next.is_none() {
                    plus = state.arena.node_tree_parent(plus).expect("tree parent");
                }
                if state.arena.nodes[plus].is_minus() {
                    minus = state.arena.nodes[plus]
                        .tree_sibling_next
                        .expect("unvisited sibling");
                    plus = state.arena.node_opposite_matched(minus);
                } else {
                    break;
                }
            }
        }
        state.statistics.grow_time_ns += start.elapsed().as_nanos();
    }

    /// Performs augment operation. This is invoked on a tight (+, +) cross-tree edge. It increases
    /// the matching by 1, converts the trees on both sides into the set of free matched edges, and
    /// applies lazy delta spreading.
    ///
    /// For each tree the following actions are performed:
    ///
    /// - Labels of all nodes change to INFINITY
    /// - tree.eps is subtracted from "-" nodes' duals and added to the "+" nodes' duals
    /// - tree.eps is subtracted from all edges incident to "+" nodes and added to all edges
    ///   incident to "-" nodes. Consecutively, the slacks of the (+, -) in-tree edges stay
    ///   unchanged
    /// - Former (-, +) and (+, +) are substituted with the (+, inf) edges (removed and added to
    ///   appropriate heaps).
    /// - The cardinality of the matching is increased by 1
    /// - Tree structure references are set to null
    /// - Tree roots are removed from the linked list of tree roots
    ///
    /// These actions change only the surface graph. They don't change the nodes and edges in the
    /// pseudonodes.
    pub(crate) fn augment<G: Graph>(state: &mut BlossomVState<'_, G>, edge: EdgeId) {
        let start = Instant::now();
        for dir in 0..2 {
            let node = state.arena.edges[edge].head[dir].expect("augment edge endpoint");
            Self::augment_branch(state, node, edge);
            state.arena.nodes[node].matched = Some(edge);
        }
        state.statistics.augment_time_ns += start.elapsed().as_nanos();
    }

    /// Performs shrink operation. This is invoked on a tight (+, +) in-tree edge. The result of this
    /// operation is the substitution of an odd circuit with a single node. This means that we
    /// consider the set of nodes of odd cardinality as a single node.
    ///
    /// In the shrink operation the following main actions are performed:
    ///
    /// - Lazy dual updates are applied to all inner edges and nodes on the circuit. Thus, the
    ///   inner edges and nodes in the pseudonodes have valid slacks and dual variables
    /// - The endpoints of the boundary edges are moved to the new blossom node, which has label
    ///   `BlossomVNode.Label#PLUS`
    /// - Lazy dual updates are applied to boundary edges and newly created blossom
    /// - Children of blossom nodes are moved to the blossom, their parent edges are changed
    ///   respectively
    /// - The blossomSibling references are set so that they form a circular linked list
    /// - If the blossom becomes a tree root, it substitutes the previous tree's root in the linked
    ///   list of tree roots
    /// - Since the newly created blossom with "+" label can change the classification of edges,
    ///   their presence in heaps is updated
    pub(crate) fn shrink<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        forming_edge: EdgeId,
        immediate_augment: bool,
    ) -> NodeId {
        let start = Instant::now();
        let blossom_root = Self::find_blossom_root(&mut state.arena, forming_edge);
        let tree = state.arena.nodes[blossom_root]
            .tree
            .expect("blossom root tree");
        let blossom = state.arena.add_node(state.node_num + state.blossom_num);
        state.arena.nodes[blossom].tree = Some(tree);
        state.arena.nodes[blossom].is_blossom = true;
        state.arena.nodes[blossom].is_outer = true;
        state.arena.nodes[blossom].is_tree_root = state.arena.nodes[blossom_root].is_tree_root;
        state.arena.nodes[blossom].dual = -state.arena.trees[tree].eps;
        if state.arena.nodes[blossom].is_tree_root {
            state.arena.trees[tree].root = blossom;
        } else {
            state.arena.nodes[blossom].matched = state.arena.nodes[blossom_root].matched;
        }

        for (node, _) in state.arena.blossom_nodes(blossom_root, forming_edge) {
            state.arena.nodes[node].is_marked = true;
        }
        let augment_edge = Self::update_tree_structure(state, blossom_root, forming_edge, blossom);
        Self::set_blossom_siblings(&mut state.arena, blossom_root, forming_edge);

        let mut current = blossom_root;
        loop {
            state.arena.nodes[current].is_marked = false;
            state.arena.nodes[current].is_processed = false;
            let sibling = state.arena.nodes[current]
                .blossom_sibling
                .expect("blossom sibling edge");
            current = state
                .arena
                .edge_opposite(sibling, current)
                .expect("blossom sibling endpoint");
            if current == blossom_root {
                break;
            }
        }
        state.arena.nodes[blossom_root].matched = None;
        state.statistics.shrink_num += 1;
        state.blossom_num += 1;
        state.statistics.shrink_time_ns += start.elapsed().as_nanos();
        if immediate_augment {
            if let Some(edge) = augment_edge {
                Self::augment(state, edge);
            }
        }
        blossom
    }

    /// Performs expand operation. This is invoked on a previously contracted pseudonode. The result
    /// of this operation is bringing the nodes in the blossom to the surface graph. An even branch
    /// of the blossom is inserted into the tree structure. Endpoints of the edges incident to the
    /// blossom are moved one layer down. The slack of the inner and boundary edges are update
    /// according to the lazy delta spreading technique.
    ///
    /// **Note:** only "-" blossoms can be expanded. At that moment their dual variables are
    /// always zero. This is the reason why they don't need to be stored to compute the dual
    /// solution.
    ///
    /// In the expand operation the following actions are performed:
    ///
    /// - Endpoints of the boundary edges are updated
    /// - The matching in the blossom is changed. **Note:** the resulting matching doesn't
    ///   depend on the previous matching
    /// - isOuter flags are updated
    /// - node.tree are updated
    /// - Tree structure is updated including parent edges and tree children of the nodes on the
    ///   even branch
    /// - The endpoints of some edges change their labels to "+" => their slacks are changed
    ///   according to the lazy delta spreading and their presence in heaps also changes
    pub(crate) fn expand<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        blossom: NodeId,
        immediate_augment: bool,
    ) {
        let start = Instant::now();
        let tree = state.arena.nodes[blossom].tree.expect("blossom tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[blossom].dual -= eps;
        Self::remove_tree_minus_blossom(&mut state.arena, tree, blossom);

        let parent_edge = state.arena.nodes[blossom]
            .parent_edge
            .expect("minus blossom parent edge");
        let branches_original = state
            .arena
            .edge_current_original(parent_edge, blossom)
            .expect("parent-edge original endpoint");
        let branches_endpoint = state.arena.get_penultimate_blossom(branches_original);
        let matched = state.arena.nodes[blossom]
            .matched
            .expect("minus blossom matched");
        let root_original = state
            .arena
            .edge_current_original(matched, blossom)
            .expect("matching-edge original endpoint");
        let blossom_root = state.arena.get_penultimate_blossom(root_original);

        let mut current = blossom_root;
        loop {
            state.arena.nodes[current].is_marked = true;
            current = Self::next_blossom_node(&state.arena, current);
            if current == blossom_root {
                break;
            }
        }

        state.arena.remove_from_child_list(blossom);
        for (edge, dir) in state.arena.incident_edges(blossom) {
            let original = state.arena.edges[edge].head_original[1 - dir]
                .expect("boundary edge original endpoint");
            let child = state
                .arena
                .get_penultimate_blossom_and_fix_grandparent(original);
            state.arena.move_edge_tail(edge, blossom, child);
        }

        if !Self::forward_direction(&state.arena, blossom_root, branches_endpoint) {
            Self::reverse_blossom_siblings(&mut state.arena, blossom_root);
        }
        Self::expand_odd_branch(state, blossom_root, branches_endpoint, tree);
        let augment_edge =
            Self::expand_even_branch(state, blossom_root, branches_endpoint, blossom);

        current = blossom_root;
        loop {
            state.arena.nodes[current].is_marked = false;
            state.arena.nodes[current].is_processed = false;
            current = Self::next_blossom_node(&state.arena, current);
            if current == blossom_root {
                break;
            }
        }
        state.statistics.expand_num += 1;
        state.removed_num += 1;
        state.statistics.expand_time_ns += start.elapsed().as_nanos();
        if immediate_augment {
            if let Some(edge) = augment_edge {
                Self::augment(state, edge);
            }
        }
    }

    /// Expands an even branch of the blossom. Here it is assumed that the blossomSiblings are
    /// directed in the way that the even branch goes from `blossomRoot` to
    /// `branchesEndpoint`.
    ///
    /// The method traverses the nodes twice: first it changes the tree structure, updates the
    /// labeling and flags, adds children, and changes the matching. After that it changes the slacks
    /// of the edges according to the lazy delta spreading and their presence in heaps. This
    /// operation is done in two steps because the later step requires correct labeling of the nodes
    /// on the branch.
    ///
    /// **Note:** this branch may consist of only one node. In this case `blossomRoot` and
    /// `branchesEndpoint` are the same nodes
    fn expand_even_branch<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        blossom_root: NodeId,
        branches_endpoint: NodeId,
        blossom: NodeId,
    ) -> Option<EdgeId> {
        let tree = state.arena.nodes[blossom]
            .tree
            .expect("expanded blossom tree");
        let blossom_matched = state.arena.nodes[blossom]
            .matched
            .expect("expanded blossom matched");
        state.arena.nodes[blossom_root].matched = Some(blossom_matched);
        state.arena.nodes[blossom_root].tree = Some(tree);
        let outside = state
            .arena
            .edge_opposite(blossom_matched, blossom_root)
            .expect("matching edge outside endpoint");
        state
            .arena
            .add_child(blossom_root, outside, blossom_matched, false);

        let mut current = blossom_root;
        let mut previous = current;
        state.arena.nodes[current].label = Label::Minus;
        state.arena.nodes[current].is_outer = true;
        state.arena.nodes[current].parent_edge = state.arena.nodes[blossom].parent_edge;
        while current != branches_endpoint {
            current = Self::next_blossom_node(&state.arena, current);
            state.arena.nodes[current].label = Label::Plus;
            state.arena.nodes[current].is_outer = true;
            state.arena.nodes[current].tree = Some(tree);
            let sibling = state.arena.nodes[current]
                .blossom_sibling
                .expect("blossom sibling edge");
            state.arena.nodes[current].matched = Some(sibling);
            let parent_edge = state.arena.nodes[previous]
                .blossom_sibling
                .expect("previous blossom sibling edge");
            state.arena.add_child(current, previous, parent_edge, false);
            previous = current;

            current = Self::next_blossom_node(&state.arena, current);
            state.arena.nodes[current].label = Label::Minus;
            state.arena.nodes[current].is_outer = true;
            state.arena.nodes[current].tree = Some(tree);
            state.arena.nodes[current].matched = Some(sibling);
            let parent_edge = state.arena.nodes[previous]
                .blossom_sibling
                .expect("previous blossom sibling edge");
            state.arena.add_child(current, previous, parent_edge, false);
            previous = current;
        }
        let parent_edge = state.arena.nodes[blossom]
            .parent_edge
            .expect("expanded blossom parent edge");
        let parent = state
            .arena
            .edge_opposite(parent_edge, branches_endpoint)
            .expect("expanded branch parent");
        state
            .arena
            .add_child(parent, branches_endpoint, parent_edge, false);

        let mut augment_edge = None;
        current = blossom_root;
        Self::expand_minus_node(state, current);
        while current != branches_endpoint {
            current = Self::next_blossom_node(&state.arena, current);
            if let Some(edge) = Self::expand_plus_node(state, current) {
                augment_edge = Some(edge);
            }
            state.arena.nodes[current].is_processed = true;
            current = Self::next_blossom_node(&state.arena, current);
            Self::expand_minus_node(state, current);
        }
        augment_edge
    }

    /// Expands the nodes on an odd branch. Here it is assumed that the blossomSiblings are directed
    /// in the way the odd branch goes from `branchesEndpoint` to `blossomRoot`.
    ///
    /// The method traverses the nodes only once setting the labels, flags, updating the matching,
    /// removing former (+, -) edges and creating new (+, inf) edges in the corresponding heaps. The
    /// method doesn't process the `blossomRoot` and `branchesEndpoint` as they belong to
    /// the even branch.
    fn expand_odd_branch<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        blossom_root: NodeId,
        branches_endpoint: NodeId,
        tree: TreeId,
    ) {
        let mut current = Self::next_blossom_node(&state.arena, branches_endpoint);
        while current != blossom_root {
            state.arena.nodes[current].label = Label::Infinity;
            state.arena.nodes[current].is_outer = true;
            state.arena.nodes[current].tree = None;
            let sibling = state.arena.nodes[current]
                .blossom_sibling
                .expect("blossom sibling edge");
            state.arena.nodes[current].matched = Some(sibling);
            Self::expand_infinity_node(state, current, tree);
            current = Self::next_blossom_node(&state.arena, current);

            state.arena.nodes[current].label = Label::Infinity;
            state.arena.nodes[current].is_outer = true;
            state.arena.nodes[current].tree = None;
            state.arena.nodes[current].matched = Some(sibling);
            Self::expand_infinity_node(state, current, tree);
            current = Self::next_blossom_node(&state.arena, current);
        }
    }

    /// Changes dual information of the `plusNode` and edge incident to it. This method relies
    /// on the labeling produced by the first traversal of the
    /// `BlossomVPrimalUpdater#expandEvenBranch(BlossomVNode, BlossomVNode, BlossomVNode)` and
    /// on the isProcessed flags of the nodes on the even branch that have been traversed already. It
    /// also assumes that all blossom nodes are marked.
    ///
    /// Since one of endpoints of the edges previously incident to the blossom changes its label, we
    /// have to update the slacks of the boundary edges incindent to the `plusNode`.
    fn expand_plus_node<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        plus: NodeId,
    ) -> Option<EdgeId> {
        let tree = state.arena.nodes[plus].tree.expect("plus node tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[plus].dual -= eps;
        let mut augment_edge = None;
        for (edge, dir) in state.arena.incident_edges(plus) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            if state.arena.nodes[opposite].is_marked && state.arena.nodes[opposite].is_plus() {
                if !state.arena.nodes[opposite].is_processed {
                    state.arena.edges[edge].slack += 2.0 * eps;
                }
            } else if !state.arena.nodes[opposite].is_marked {
                state.arena.edges[edge].slack += 2.0 * eps;
            } else if !state.arena.nodes[opposite].is_minus() {
                state.arena.edges[edge].slack += eps;
            }

            match state.arena.nodes[opposite].label {
                Label::Plus => {
                    let opposite_tree = state.arena.nodes[opposite].tree.expect("plus tree");
                    if opposite_tree == tree {
                        if !state.arena.nodes[opposite].is_processed {
                            state.arena.add_plus_plus_edge(tree, edge);
                        }
                    } else {
                        let tree_edge = state.arena.trees[opposite_tree]
                            .current_edge
                            .expect("cross-tree edge");
                        let direction = state.arena.trees[opposite_tree].current_direction;
                        Self::remove_tree_edge_minus_plus(
                            &mut state.arena,
                            tree_edge,
                            edge,
                            direction,
                        );
                        Self::add_tree_edge_plus_plus(&mut state.arena, tree_edge, edge);
                        if state.arena.edges[edge].slack
                            <= eps + state.arena.trees[opposite_tree].eps
                        {
                            augment_edge = Some(edge);
                        }
                    }
                }
                Label::Minus => {
                    let opposite_tree = state.arena.nodes[opposite].tree.expect("minus tree");
                    if opposite_tree != tree {
                        if state.arena.trees[opposite_tree].current_edge.is_none() {
                            state.arena.add_tree_edge(tree, opposite_tree);
                        }
                        let tree_edge = state.arena.trees[opposite_tree]
                            .current_edge
                            .expect("cross-tree edge");
                        let direction = state.arena.trees[opposite_tree].current_direction;
                        Self::add_tree_edge_plus_minus(
                            &mut state.arena,
                            tree_edge,
                            edge,
                            direction,
                        );
                    }
                }
                Label::Infinity => state.arena.add_plus_infinity_edge(tree, edge),
            }
        }
        augment_edge
    }

    /// Expands a minus node from the odd branch. Changes the slacks of inner (-,-) and (-, inf)
    /// edges.
    fn expand_minus_node<G: Graph>(state: &mut BlossomVState<'_, G>, minus: NodeId) {
        let tree = state.arena.nodes[minus].tree.expect("minus node tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[minus].dual += eps;
        if state.arena.nodes[minus].is_blossom {
            state.arena.add_minus_blossom(tree, minus);
        }
        for (edge, dir) in state.arena.incident_edges(minus) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            if state.arena.nodes[opposite].is_marked && !state.arena.nodes[opposite].is_plus() {
                state.arena.edges[edge].slack -= eps;
            }
        }
    }

    /// Expands an infinity node from the odd branch
    fn expand_infinity_node<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        node: NodeId,
        old_tree: TreeId,
    ) {
        let eps = state.arena.trees[old_tree].eps;
        for (edge, dir) in state.arena.incident_edges(node) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            if !state.arena.nodes[opposite].is_marked {
                state.arena.edges[edge].slack += eps;
                if state.arena.nodes[opposite].is_plus() {
                    let opposite_tree = state.arena.nodes[opposite].tree.expect("plus tree");
                    if opposite_tree != old_tree {
                        let tree_edge = state.arena.trees[opposite_tree]
                            .current_edge
                            .expect("cross-tree edge");
                        let direction = state.arena.trees[opposite_tree].current_direction;
                        Self::remove_tree_edge_minus_plus(
                            &mut state.arena,
                            tree_edge,
                            edge,
                            direction,
                        );
                    }
                    state.arena.add_plus_infinity_edge(opposite_tree, edge);
                }
            }
        }
    }

    fn next_blossom_node(arena: &BlossomArena, node: NodeId) -> NodeId {
        let edge = arena.nodes[node]
            .blossom_sibling
            .expect("blossom sibling edge");
        arena
            .edge_opposite(edge, node)
            .expect("blossom sibling endpoint")
    }

    /// Reverses the direction of blossomSibling references
    fn reverse_blossom_siblings(arena: &mut BlossomArena, blossom_node: NodeId) {
        let mut previous_edge = arena.nodes[blossom_node]
            .blossom_sibling
            .expect("blossom sibling edge");
        let mut current = blossom_node;
        loop {
            current = arena
                .edge_opposite(previous_edge, current)
                .expect("blossom sibling endpoint");
            let temporary = previous_edge;
            previous_edge = arena.nodes[current]
                .blossom_sibling
                .expect("blossom sibling edge");
            arena.nodes[current].blossom_sibling = Some(temporary);
            if current == blossom_node {
                break;
            }
        }
    }

    /// Checks whether the direction of blossomSibling references is suitable for the expand
    /// operation, i.e. an even branch goes from `blossomRoot` to `branchesEndpoint`.
    fn forward_direction(
        arena: &BlossomArena,
        blossom_root: NodeId,
        branches_endpoint: NodeId,
    ) -> bool {
        let mut hops = 0;
        let mut current = blossom_root;
        while current != branches_endpoint {
            hops += 1;
            current = Self::next_blossom_node(arena, current);
        }
        hops % 2 == 0
    }

    /// Updates the tree structure in the shrink operation. Moves the endpoints of the boundary edges
    /// to the `blossom`, moves the children of the nodes on the circuit to the blossom,
    /// updates edges's slacks and presence in heaps accordingly.
    fn update_tree_structure<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        blossom_root: NodeId,
        forming_edge: EdgeId,
        blossom: NodeId,
    ) -> Option<EdgeId> {
        let tree = state.arena.nodes[blossom_root].tree.expect("blossom tree");
        let mut augment_edge = None;
        for (node, _) in state.arena.blossom_nodes(blossom_root, forming_edge) {
            if node != blossom_root {
                if state.arena.nodes[node].is_plus() {
                    state.arena.remove_from_child_list(node);
                    state.arena.move_children_to(node, blossom);
                    if let Some(edge) = Self::shrink_plus_node(state, node, blossom) {
                        augment_edge = Some(edge);
                    }
                    state.arena.nodes[node].is_processed = true;
                } else {
                    if state.arena.nodes[node].is_blossom {
                        Self::remove_tree_minus_blossom(&mut state.arena, tree, node);
                    }
                    state.arena.remove_from_child_list(node);
                    Self::shrink_minus_node(state, node, blossom);
                }
            }
            state.arena.nodes[node].blossom_grandparent = Some(blossom);
            state.arena.nodes[node].blossom_parent = Some(blossom);
        }

        state.arena.remove_from_child_list(blossom_root);
        if !state.arena.nodes[blossom_root].is_tree_root {
            let parent = state
                .arena
                .node_tree_parent(blossom_root)
                .expect("non-root blossom root parent");
            let parent_edge = state.arena.nodes[blossom_root]
                .parent_edge
                .expect("non-root parent edge");
            state.arena.add_child(parent, blossom, parent_edge, false);
        } else {
            let next = state.arena.nodes[blossom_root].tree_sibling_next;
            let prev = state.arena.nodes[blossom_root]
                .tree_sibling_prev
                .expect("root list predecessor");
            state.arena.nodes[blossom].tree_sibling_next = next;
            state.arena.nodes[blossom].tree_sibling_prev = Some(prev);
            state.arena.nodes[prev].tree_sibling_next = Some(blossom);
            if let Some(next) = next {
                state.arena.nodes[next].tree_sibling_prev = Some(blossom);
            }
        }
        state.arena.move_children_to(blossom_root, blossom);
        if let Some(edge) = Self::shrink_plus_node(state, blossom_root, blossom) {
            augment_edge = Some(edge);
        }
        state.arena.nodes[blossom_root].is_tree_root = false;
        augment_edge
    }

    /// Processes a plus node on an odd circuit in the shrink operation. Moves endpoints of the
    /// boundary edges, updates slacks of incident edges.
    fn shrink_plus_node<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        plus: NodeId,
        blossom: NodeId,
    ) -> Option<EdgeId> {
        let tree = state.arena.nodes[plus].tree.expect("plus node tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[plus].dual += eps;
        let mut augment_edge = None;
        for (edge, dir) in state.arena.incident_edges(plus) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            if !state.arena.nodes[opposite].is_marked {
                state.arena.move_edge_tail(edge, plus, blossom);
                if state.arena.nodes[opposite].tree != Some(tree)
                    && state.arena.nodes[opposite].is_plus()
                    && state.arena.edges[edge].slack
                        <= eps
                            + state.arena.trees
                                [state.arena.nodes[opposite].tree.expect("plus node tree")]
                            .eps
                {
                    augment_edge = Some(edge);
                }
            } else if state.arena.nodes[opposite].is_plus() {
                if !state.arena.nodes[opposite].is_processed {
                    Self::remove_tree_plus_plus(&mut state.arena, tree, edge);
                }
                state.arena.edges[edge].slack -= eps;
            }
        }
        augment_edge
    }

    /// Processes a minus node from an odd circuit in the shrink operation. Moves the endpoints of
    /// the boundary edges, updates their slacks
    fn shrink_minus_node<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        minus: NodeId,
        blossom: NodeId,
    ) {
        let tree = state.arena.nodes[minus].tree.expect("minus node tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[minus].dual -= eps;
        for (edge, dir) in state.arena.incident_edges(minus) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            let opposite_tree = state.arena.nodes[opposite].tree;
            if !state.arena.nodes[opposite].is_marked {
                state.arena.move_edge_tail(edge, minus, blossom);
                state.arena.edges[edge].slack += 2.0 * eps;
                if opposite_tree == Some(tree) {
                    if state.arena.nodes[opposite].is_plus() {
                        state.arena.add_plus_plus_edge(tree, edge);
                    }
                } else {
                    match state.arena.nodes[opposite].label {
                        Label::Plus => {
                            let opposite_tree = opposite_tree.expect("plus node tree");
                            let tree_edge = state.arena.trees[opposite_tree]
                                .current_edge
                                .expect("current cross-tree edge");
                            let direction = state.arena.trees[opposite_tree].current_direction;
                            Self::remove_tree_edge_minus_plus(
                                &mut state.arena,
                                tree_edge,
                                edge,
                                direction,
                            );
                            Self::add_tree_edge_plus_plus(&mut state.arena, tree_edge, edge);
                        }
                        Label::Minus => {
                            let opposite_tree = opposite_tree.expect("minus node tree");
                            if state.arena.trees[opposite_tree].current_edge.is_none() {
                                state.arena.add_tree_edge(tree, opposite_tree);
                            }
                            let tree_edge = state.arena.trees[opposite_tree]
                                .current_edge
                                .expect("current cross-tree edge");
                            let direction = state.arena.trees[opposite_tree].current_direction;
                            Self::add_tree_edge_plus_minus(
                                &mut state.arena,
                                tree_edge,
                                edge,
                                direction,
                            );
                        }
                        Label::Infinity => state.arena.add_plus_infinity_edge(tree, edge),
                    }
                }
            } else if state.arena.nodes[opposite].is_minus() {
                state.arena.edges[edge].slack += eps;
            }
        }
    }

    /// Creates a circular linked list of blossom nodes.
    ///
    /// **Note:** this method heavily relies on the property of the
    /// `BlossomVEdge.BlossomNodesIterator` that it returns the blossomRoot while processing
    /// the first branch (with direction 0).
    fn set_blossom_siblings(arena: &mut BlossomArena, blossom_root: NodeId, forming_edge: EdgeId) {
        let mut previous_edge = forming_edge;
        for (current, direction) in arena.blossom_nodes(blossom_root, forming_edge) {
            if direction == 0 {
                arena.nodes[current].blossom_sibling = Some(previous_edge);
                if let Some(parent_edge) = arena.nodes[current].parent_edge {
                    previous_edge = parent_edge;
                }
            } else {
                arena.nodes[current].blossom_sibling = arena.nodes[current].parent_edge;
            }
        }
    }

    /// Finds a blossom root of the circuit created by the `edge`. More precisely, finds an lca
    /// of `edge.head[0]` and `edge.head[1]`.
    fn find_blossom_root(arena: &mut BlossomArena, forming_edge: EdgeId) -> NodeId {
        let mut endpoints = [
            arena.edges[forming_edge].head[0].expect("forming edge endpoint"),
            arena.edges[forming_edge].head[1].expect("forming edge endpoint"),
        ];
        let mut branch = 0;
        let (root, upper_bound) = loop {
            if arena.nodes[endpoints[branch]].is_marked {
                break (endpoints[branch], endpoints[1 - branch]);
            }
            arena.nodes[endpoints[branch]].is_marked = true;
            if arena.nodes[endpoints[branch]].is_tree_root {
                let upper_bound = endpoints[branch];
                let mut jump = endpoints[1 - branch];
                while !arena.nodes[jump].is_marked {
                    jump = arena.node_tree_grandparent(jump);
                }
                break (jump, upper_bound);
            }
            endpoints[branch] = arena.node_tree_grandparent(endpoints[branch]);
            branch = 1 - branch;
        };
        let mut jump = root;
        while jump != upper_bound {
            jump = arena.node_tree_grandparent(jump);
            arena.nodes[jump].is_marked = false;
        }
        let starts = [
            arena.edges[forming_edge].head[0].expect("forming edge endpoint"),
            arena.edges[forming_edge].head[1].expect("forming edge endpoint"),
        ];
        for mut start in starts {
            while start != root {
                arena.nodes[start].is_marked = false;
                arena.nodes[start].is_outer = false;
                start = arena.node_tree_parent(start).expect("minus parent");
                arena.nodes[start].is_outer = false;
                start = arena.node_tree_parent(start).expect("plus parent");
            }
        }
        arena.nodes[root].is_outer = false;
        arena.nodes[root].is_marked = false;
        root
    }

    /// Processes a minus node in the grow operation. Applies lazy delta spreading, adds new (-,+)
    /// cross-tree edges, removes former (+, inf) edges.
    fn process_minus_node_grow<G: Graph>(state: &mut BlossomVState<'_, G>, minus: NodeId) {
        let tree = state.arena.nodes[minus].tree.expect("minus node tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[minus].dual += eps;
        if state.arena.nodes[minus].is_blossom {
            state.arena.add_minus_blossom(tree, minus);
        }
        let matched_opposite = state.arena.node_opposite_matched(minus);
        for (edge, dir) in state.arena.incident_edges(minus) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            state.arena.edges[edge].slack -= eps;
            if state.arena.nodes[opposite].is_plus() {
                let opposite_tree = state.arena.nodes[opposite].tree.expect("plus node tree");
                if opposite_tree != tree {
                    if state.arena.trees[opposite_tree].current_edge.is_none() {
                        state.arena.add_tree_edge(tree, opposite_tree);
                    }
                    Self::remove_tree_plus_infinity(&mut state.arena, opposite_tree, edge);
                    let tree_edge = state.arena.trees[opposite_tree]
                        .current_edge
                        .expect("current tree edge");
                    let direction = state.arena.trees[opposite_tree].current_direction;
                    Self::add_tree_edge_minus_plus(&mut state.arena, tree_edge, edge, direction);
                } else if opposite != matched_opposite {
                    Self::remove_tree_plus_infinity(&mut state.arena, tree, edge);
                }
            }
        }
    }

    /// Processes a plus node during the grow operation. Applies lazy delta spreading, removes former
    /// (+, inf) edges, adds new (+, +) in-tree and cross-tree edges, new (+, -) cross-tree edges.
    /// When the `manyGrows` flag is on, collects the tight (+, inf) edges on grows them as
    /// well.
    ///
    /// **Note:** the recursive grows must be done ofter the grow operation on the current edge is
    /// over. This ensures correct state of the heaps and the edges' slacks.
    fn process_plus_node_grow<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        node: NodeId,
        recursive_grow: bool,
        immediate_augment: bool,
    ) {
        let tree = state.arena.nodes[node].tree.expect("plus node tree");
        let eps = state.arena.trees[tree].eps;
        state.arena.nodes[node].dual -= eps;
        let mut augment_edge = None;
        for (edge, dir) in state.arena.incident_edges(node) {
            let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
            state.arena.edges[edge].slack += eps;
            match state.arena.nodes[opposite].label {
                Label::Plus => {
                    let opposite_tree = state.arena.nodes[opposite].tree.expect("plus node tree");
                    if opposite_tree == tree {
                        Self::remove_tree_plus_infinity(&mut state.arena, tree, edge);
                        state.arena.add_plus_plus_edge(tree, edge);
                    } else {
                        if state.arena.trees[opposite_tree].current_edge.is_none() {
                            state.arena.add_tree_edge(tree, opposite_tree);
                        }
                        Self::remove_tree_plus_infinity(&mut state.arena, opposite_tree, edge);
                        let tree_edge = state.arena.trees[opposite_tree]
                            .current_edge
                            .expect("current tree edge");
                        Self::add_tree_edge_plus_plus(&mut state.arena, tree_edge, edge);
                        if state.arena.edges[edge].slack
                            <= eps + state.arena.trees[opposite_tree].eps
                        {
                            augment_edge = Some(edge);
                        }
                    }
                }
                Label::Minus => {
                    let opposite_tree = state.arena.nodes[opposite].tree.expect("minus node tree");
                    if opposite_tree != tree {
                        if state.arena.trees[opposite_tree].current_edge.is_none() {
                            state.arena.add_tree_edge(tree, opposite_tree);
                        }
                        let tree_edge = state.arena.trees[opposite_tree]
                            .current_edge
                            .expect("current tree edge");
                        let direction = state.arena.trees[opposite_tree].current_direction;
                        Self::add_tree_edge_plus_minus(
                            &mut state.arena,
                            tree_edge,
                            edge,
                            direction,
                        );
                    }
                }
                Label::Infinity => {
                    state.arena.add_plus_infinity_edge(tree, edge);
                    if recursive_grow
                        && state.arena.edges[edge].slack <= eps
                        && !state.arena.nodes[opposite].is_marked
                    {
                        let recursive_minus = opposite;
                        let recursive_plus = state.arena.node_opposite_matched(recursive_minus);
                        state.arena.nodes[recursive_minus].is_marked = true;
                        state.arena.nodes[recursive_plus].is_marked = true;
                        state.arena.add_child(node, recursive_minus, edge, true);
                        let matched = state.arena.nodes[recursive_minus]
                            .matched
                            .expect("matched infinity node");
                        state
                            .arena
                            .add_child(recursive_minus, recursive_plus, matched, true);
                    }
                }
            }
        }
        if immediate_augment {
            if let Some(edge) = augment_edge {
                Self::augment(state, edge);
            }
        }
        state.statistics.grow_num += 1;
    }

    /// Converts a tree into a set of free matched edges. Changes the matching starting from
    /// `firstNode` all the way up to the firstNode.tree.root. It changes the labeling of the
    /// nodes, applies lazy delta spreading, updates edges' presence in the heaps. This method also
    /// deletes unnecessary tree edges.
    ///
    /// This method doesn't change the nodes and edge contracted in the blossoms.
    fn augment_branch<G: Graph>(
        state: &mut BlossomVState<'_, G>,
        first_node: NodeId,
        augment_edge: EdgeId,
    ) {
        let tree = state.arena.nodes[first_node].tree.expect("augment tree");
        let eps = state.arena.trees[tree].eps;
        let root = state.arena.trees[tree].root;
        state.arena.set_current_edges(tree);

        let tree_nodes = state.arena.tree_nodes(root);
        for node in tree_nodes {
            if !state.arena.nodes[node].is_marked {
                if state.arena.nodes[node].is_plus() {
                    state.arena.nodes[node].dual += eps;
                } else {
                    state.arena.nodes[node].dual -= eps;
                }
                for (edge, dir) in state.arena.incident_edges(node) {
                    let opposite = state.arena.edges[edge].head[dir].expect("opposite endpoint");
                    let opposite_tree = state.arena.nodes[opposite].tree;
                    if state.arena.nodes[node].is_plus() {
                        state.arena.edges[edge].slack -= eps;
                        if let Some(opposite_tree) = opposite_tree.filter(|&other| other != tree) {
                            if state.arena.nodes[opposite].is_plus() {
                                let tree_edge = state.arena.trees[opposite_tree]
                                    .current_edge
                                    .expect("cross-tree edge");
                                Self::remove_tree_edge_plus_plus(&mut state.arena, tree_edge, edge);
                                state.arena.add_plus_infinity_edge(opposite_tree, edge);
                            } else if state.arena.nodes[opposite].is_minus() {
                                let tree_edge = state.arena.trees[opposite_tree]
                                    .current_edge
                                    .expect("cross-tree edge");
                                let direction = state.arena.trees[opposite_tree].current_direction;
                                Self::remove_tree_edge_plus_minus(
                                    &mut state.arena,
                                    tree_edge,
                                    edge,
                                    direction,
                                );
                            }
                        }
                    } else {
                        state.arena.edges[edge].slack += eps;
                        if let Some(opposite_tree) = opposite_tree.filter(|&other| other != tree) {
                            if state.arena.nodes[opposite].is_plus() {
                                let tree_edge = state.arena.trees[opposite_tree]
                                    .current_edge
                                    .expect("cross-tree edge");
                                let direction = state.arena.trees[opposite_tree].current_direction;
                                Self::remove_tree_edge_minus_plus(
                                    &mut state.arena,
                                    tree_edge,
                                    edge,
                                    direction,
                                );
                                state.arena.add_plus_infinity_edge(opposite_tree, edge);
                            }
                        }
                    }
                }
                state.arena.nodes[node].label = Label::Infinity;
            } else {
                state.arena.nodes[node].is_marked = false;
            }
        }

        for (tree_edge, direction) in state.arena.tree_edges(tree) {
            let opposite =
                state.arena.tree_edges[tree_edge].head[direction].expect("opposite tree");
            state.arena.trees[opposite].current_edge = None;
            let plus_plus = mem::take(&mut state.arena.tree_edges[tree_edge].plus_plus_edges);
            state.arena.trees[opposite]
                .plus_infinity_edges
                .meld(plus_plus);
            let minus_plus = if direction == 0 {
                mem::take(&mut state.arena.tree_edges[tree_edge].plus_minus_edges_0)
            } else {
                mem::take(&mut state.arena.tree_edges[tree_edge].plus_minus_edges_1)
            };
            state.arena.trees[opposite]
                .plus_infinity_edges
                .meld(minus_plus);
            state.arena.remove_tree_edge(tree_edge);
        }

        let mut matched_edge = augment_edge;
        let mut plus = first_node;
        let mut minus = state.arena.node_tree_parent(plus);
        while let Some(minus_node) = minus {
            state.arena.nodes[plus].matched = Some(matched_edge);
            matched_edge = state.arena.nodes[minus_node]
                .parent_edge
                .expect("minus parent edge");
            state.arena.nodes[minus_node].matched = Some(matched_edge);
            plus = state
                .arena
                .node_tree_parent(minus_node)
                .expect("minus node has plus parent");
            minus = state.arena.node_tree_parent(plus);
        }
        state.arena.nodes[root].matched = Some(matched_edge);
        state.arena.remove_from_child_list(root);
        state.arena.nodes[root].is_tree_root = false;
        state.arena.trees[tree].active = false;
        state.tree_num -= 1;
    }

    fn remove_tree_plus_plus(arena: &mut BlossomArena, tree: TreeId, edge: EdgeId) {
        let handle = arena.edges[edge]
            .handle
            .expect("edge has plus-plus heap handle");
        arena.trees[tree]
            .plus_plus_edges
            .delete(handle)
            .expect("valid plus-plus handle");
    }

    fn remove_tree_minus_blossom(arena: &mut BlossomArena, tree: TreeId, blossom: NodeId) {
        let handle = arena.nodes[blossom]
            .handle
            .expect("blossom has minus-blossom heap handle");
        arena.trees[tree]
            .minus_blossoms
            .delete(handle)
            .expect("valid minus-blossom handle");
    }

    fn remove_tree_plus_infinity(arena: &mut BlossomArena, tree: TreeId, edge: EdgeId) {
        let handle = arena.edges[edge]
            .handle
            .expect("edge has plus-infinity heap handle");
        arena.trees[tree]
            .plus_infinity_edges
            .delete(handle)
            .expect("valid plus-infinity handle");
    }

    fn add_tree_edge_plus_plus(arena: &mut BlossomArena, tree_edge: TreeEdgeId, edge: EdgeId) {
        let handle = arena.tree_edges[tree_edge]
            .plus_plus_edges
            .insert(OrderedF64(arena.edges[edge].slack), edge);
        arena.edges[edge].handle = Some(handle);
    }

    fn remove_tree_edge_plus_plus(arena: &mut BlossomArena, tree_edge: TreeEdgeId, edge: EdgeId) {
        let handle = arena.edges[edge].handle.expect("plus-plus heap handle");
        arena.tree_edges[tree_edge]
            .plus_plus_edges
            .delete(handle)
            .expect("valid plus-plus handle");
    }

    fn add_tree_edge_minus_plus(
        arena: &mut BlossomArena,
        tree_edge: TreeEdgeId,
        edge: EdgeId,
        direction: usize,
    ) {
        let heap = if direction == 0 {
            &mut arena.tree_edges[tree_edge].plus_minus_edges_0
        } else {
            &mut arena.tree_edges[tree_edge].plus_minus_edges_1
        };
        arena.edges[edge].handle = Some(heap.insert(OrderedF64(arena.edges[edge].slack), edge));
    }

    fn add_tree_edge_plus_minus(
        arena: &mut BlossomArena,
        tree_edge: TreeEdgeId,
        edge: EdgeId,
        direction: usize,
    ) {
        Self::add_tree_edge_minus_plus(arena, tree_edge, edge, 1 - direction);
    }

    fn remove_tree_edge_minus_plus(
        arena: &mut BlossomArena,
        tree_edge: TreeEdgeId,
        edge: EdgeId,
        direction: usize,
    ) {
        let handle = arena.edges[edge].handle.expect("minus-plus heap handle");
        let heap = if direction == 0 {
            &mut arena.tree_edges[tree_edge].plus_minus_edges_0
        } else {
            &mut arena.tree_edges[tree_edge].plus_minus_edges_1
        };
        heap.delete(handle).expect("valid minus-plus handle");
        arena.edges[edge].handle = None;
    }

    fn remove_tree_edge_plus_minus(
        arena: &mut BlossomArena,
        tree_edge: TreeEdgeId,
        edge: EdgeId,
        direction: usize,
    ) {
        Self::remove_tree_edge_minus_plus(arena, tree_edge, edge, 1 - direction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alg::matching::blossom::initializer::BlossomVInitializer;
    use crate::alg::matching::blossom::{BlossomVOptions, InitializationType};
    use crate::graph::{GraphMut, GraphMutExt, UGraph};

    fn path_graph() -> UGraph<i32> {
        let mut graph = UGraph::<i32>::simple().weighted();
        for vertex in 1..=3 {
            graph.add_vertex(vertex);
        }
        graph.add_weighted_edge(&1, &2, 0.0).unwrap();
        graph.add_weighted_edge(&2, &3, 0.0).unwrap();
        graph
    }

    fn internal_edge(
        graph: &UGraph<i32>,
        state: &BlossomVState<'_, UGraph<i32>>,
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
    fn augment_then_grow_builds_java_ancestor_chain() {
        let graph = path_graph();
        let mut state = BlossomVInitializer::new(&graph)
            .initialize(BlossomVOptions::with_initialization(
                InitializationType::None,
            ))
            .unwrap();
        let node = |state: &BlossomVState<'_, UGraph<i32>>, vertex| {
            state
                .graph_vertices
                .iter()
                .position(|candidate| *candidate == vertex)
                .unwrap()
        };
        let n1 = node(&state, 1);
        let n2 = node(&state, 2);
        let n3 = node(&state, 3);
        let e12 = internal_edge(&graph, &state, 1, 2);
        let e23 = internal_edge(&graph, &state, 2, 3);

        BlossomVPrimalUpdater::augment(&mut state, e23);
        BlossomVPrimalUpdater::grow(&mut state, e12, false, false);

        assert_eq!(state.arena.node_tree_parent(n2), Some(n1));
        assert_eq!(state.arena.node_tree_parent(n3), Some(n2));
        assert_eq!(state.arena.node_tree_grandparent(n3), n1);
        assert_eq!(state.tree_num, 1);
    }

    #[test]
    fn shrink_contracts_java_triangle_blossom() {
        let mut graph = path_graph();
        graph.add_weighted_edge(&1, &3, 0.0).unwrap();
        let mut state = BlossomVInitializer::new(&graph)
            .initialize(BlossomVOptions::with_initialization(
                InitializationType::None,
            ))
            .unwrap();
        let e12 = internal_edge(&graph, &state, 1, 2);
        let e23 = internal_edge(&graph, &state, 2, 3);
        let e13 = internal_edge(&graph, &state, 1, 3);

        BlossomVPrimalUpdater::augment(&mut state, e23);
        BlossomVPrimalUpdater::grow(&mut state, e12, false, false);
        let blossom = BlossomVPrimalUpdater::shrink(&mut state, e13, false);

        assert!(state.arena.nodes[blossom].is_blossom);
        assert!(state.arena.nodes[blossom].is_outer);
        assert!(state.arena.nodes[blossom].is_tree_root);
        assert_eq!(
            state.arena.trees[state.arena.nodes[blossom].tree.unwrap()].root,
            blossom
        );
        assert_eq!(state.blossom_num, 1);
        assert_eq!(state.statistics.shrink_count(), 1);
        assert!(
            (0..state.node_num)
                .all(|node| { state.arena.nodes[node].blossom_parent == Some(blossom) })
        );
    }
}
