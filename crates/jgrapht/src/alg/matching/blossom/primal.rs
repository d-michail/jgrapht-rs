use super::state::BlossomVState;
use super::types::{BlossomArena, EdgeId, Label, NodeId, TreeEdgeId, TreeId};
use crate::graph::Graph;
use crate::util::OrderedF64;
use std::mem;
use std::time::Instant;

pub(crate) struct BlossomVPrimalUpdater;

impl BlossomVPrimalUpdater {
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

    pub(crate) fn augment<G: Graph>(state: &mut BlossomVState<'_, G>, edge: EdgeId) {
        let start = Instant::now();
        for dir in 0..2 {
            let node = state.arena.edges[edge].head[dir].expect("augment edge endpoint");
            Self::augment_branch(state, node, edge);
            state.arena.nodes[node].matched = Some(edge);
        }
        state.statistics.augment_time_ns += start.elapsed().as_nanos();
    }

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
        assert!((0..state.node_num)
            .all(|node| { state.arena.nodes[node].blossom_parent == Some(blossom) }));
    }
}
