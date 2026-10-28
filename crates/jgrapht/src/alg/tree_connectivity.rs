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

//! Dynamic connectivity in a forest (`TreeDynamicConnectivity`).
//!
//! This uses the same Euler-tour representation and worst-case bounds as Java. Tours are stored in
//! private implicit AVL sequences, and each vertex keeps its outgoing arcs in a private intrusive
//! circular ring. Both Rust-native structures use stable arena indices.

mod arc_ring;
mod sequence;

#[cfg(test)]
mod arc_ring_test;
#[cfg(test)]
mod sequence_test;

use self::arc_ring::{ArcRingArena, RingId, RingNode};
use self::sequence::{SequenceArena, SequenceId, SequenceNode};
use crate::{GraphError, Result};
use std::collections::HashMap;
use std::hash::Hash;

type NodeId = usize;
type ArcId = usize;

#[derive(Debug, Clone)]
struct Node<T> {
    value: T,
    arcs: RingId,
    target_map: HashMap<NodeId, ArcId>,
    singleton_tree: Option<SequenceId>,
}

#[derive(Debug, Clone, Copy)]
struct Arc {
    target: NodeId,
    list_node: RingNode,
    tree_node: SequenceNode,
}

/// Data structure for storing dynamic trees and querying node connectivity
///
/// This data structure supports the following operations:
///
/// - Adding an element in $\mathcal{O}(\log 1)$
/// - Checking if an element in present in $\mathcal{O}(1)$
/// - Connecting two elements in $\mathcal{O}(\log n)$
/// - Checking if two elements are connected in $\mathcal{O}(\log n)$
/// - Removing connection between two nodes in $\mathcal{O}(\log n)$
/// - Removing an element in $\mathcal{O}(deg(element)\cdot\log n + 1)$
///
/// This data structure doesn't allow to store graphs with cycles. Also, the edges are considered to
/// be undirected. The memory complexity is linear in the number of inserted elements. The
/// implementation is based on the [Euler tour technique](https://en.wikipedia.org/wiki/Euler_tour_technique).
///
/// For the description of the Euler tour data structure, we refer to the *Monika Rauch Henzinger,
/// Valerie King: Randomized dynamic graph algorithms with polylogarithmic time per operation. STOC
/// 1995: 519-527*
///
/// # Rust notes
///
/// A forest whose trees can be linked and cut in worst-case logarithmic time.
#[derive(Debug, Clone)]
pub struct TreeDynamicConnectivity<T: Eq + Hash + Clone> {
    trees: SequenceArena<T>,
    arc_lists: ArcRingArena<ArcId>,
    node_map: HashMap<T, NodeId>,
    nodes: Vec<Node<T>>,
    arcs: Vec<Option<Arc>>,
}

impl<T: Eq + Hash + Clone> Default for TreeDynamicConnectivity<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> TreeDynamicConnectivity<T> {
    /// An empty forest.
    pub fn new() -> Self {
        TreeDynamicConnectivity {
            trees: SequenceArena::new(),
            arc_lists: ArcRingArena::new(),
            node_map: HashMap::new(),
            nodes: Vec::new(),
            arcs: Vec::new(),
        }
    }

    /// Whether `element` is in the forest.
    pub fn contains(&self, element: &T) -> bool {
        self.node_map.contains_key(element)
    }

    /// Adds an isolated element; `false` if it was already present.
    pub fn add(&mut self, element: T) -> bool {
        if self.contains(&element) {
            return false;
        }
        let id = self.nodes.len();
        let tree = self.trees.create_tree();
        let arcs = self.arc_lists.create_list();
        self.nodes.push(Node {
            value: element.clone(),
            arcs,
            target_map: HashMap::new(),
            singleton_tree: Some(tree),
        });
        self.node_map.insert(element, id);
        true
    }

    /// Removes an element, cutting all its incident edges first.
    pub fn remove(&mut self, element: &T) -> bool {
        let Some(&node) = self.node_map.get(element) else {
            return false;
        };
        while !self.is_singleton(node) {
            let last = self.arc_lists.last(self.nodes[node].arcs).unwrap();
            let arc = *self.arc_lists.value(last);
            let target = self.nodes[self.arc(arc).target].value.clone();
            self.cut(element, &target).unwrap();
        }
        self.node_map.remove(element);
        self.nodes[node].singleton_tree = None;
        true
    }

    /// Following JGraphT, isolated elements are not connected even to themselves.
    pub fn connected(&self, first: &T, second: &T) -> bool {
        let (Some(&first), Some(&second)) = (self.node_map.get(first), self.node_map.get(second))
        else {
            return false;
        };
        if self.is_singleton(first) || self.is_singleton(second) {
            return false;
        }
        self.get_tree(first) == self.get_tree(second)
    }

    /// Joins two trees. Missing elements are inserted; an edge that would create a cycle is not.
    pub fn link(&mut self, first: T, second: T) -> bool {
        // Preserve the existing Rust public behaviour for a self-link.
        if first == second {
            return false;
        }
        self.add(first.clone());
        self.add(second.clone());
        if self.connected(&first, &second) {
            return false;
        }

        let first_node = self.node_map[&first];
        let second_node = self.node_map[&second];
        let first_tree = self.get_tree(first_node);
        let second_tree = self.get_tree(second_node);
        self.make_root(first_tree, first_node);
        self.make_root(second_tree, second_node);

        let first_occurrence = self.trees.add_min(second_tree, first.clone());
        let first_arc = self.create_arc(second_node, first_occurrence);
        if self.is_singleton(first_node) {
            self.nodes[first_node].singleton_tree = None;
            self.add_arc_last(first_node, first_arc);
        } else {
            let last_child_value = self
                .trees
                .value(self.trees.max(first_tree).unwrap())
                .clone();
            let last_child = self.node_map[&last_child_value];
            let previous_arc = self.nodes[first_node].target_map[&last_child];
            self.add_arc_after(first_node, previous_arc, first_arc);
        }

        let second_occurrence = self.trees.add_max(second_tree, second.clone());
        let second_arc = self.create_arc(first_node, second_occurrence);
        if self.is_singleton(second_node) {
            self.nodes[second_node].singleton_tree = None;
            self.add_arc_last(second_node, second_arc);
        } else {
            let maximum = self.trees.max(second_tree).unwrap();
            let previous = self.trees.predecessor(maximum).unwrap();
            let last_child_value = self.trees.value(previous).clone();
            let last_child = self.node_map[&last_child_value];
            let previous_arc = self.nodes[second_node].target_map[&last_child];
            self.add_arc_after(second_node, previous_arc, second_arc);
        }

        self.trees.merge_after(first_tree, second_tree);
        true
    }

    /// Removes an edge. A connected but non-adjacent pair is an invalid argument, as before.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::ElementNotFound`] if an element is not present. Errors of the graph
    /// operations it performs are propagated.
    pub fn cut(&mut self, first: &T, second: &T) -> Result<bool> {
        if !self.connected(first, second) {
            return Ok(false);
        }
        let first_node = self.node_map[first];
        let second_node = self.node_map[second];
        let Some(&arc_to_second) = self.nodes[first_node].target_map.get(&second_node) else {
            return Err(GraphError::ElementNotFound);
        };

        let tree = self.get_tree(first_node);
        self.make_last_arc(tree, first_node, arc_to_second);
        let right = self
            .trees
            .split_after(tree, self.arc(arc_to_second).tree_node);

        self.trees.remove_max(tree);
        self.remove_arc(first_node, arc_to_second);
        if self.is_singleton(first_node) {
            self.nodes[first_node].singleton_tree = Some(tree);
        }

        let second_to_first = self.nodes[second_node].target_map[&first_node];
        self.trees.remove_max(right);
        self.remove_arc(second_node, second_to_first);
        if self.is_singleton(second_node) {
            self.nodes[second_node].singleton_tree = Some(right);
        }
        Ok(true)
    }

    fn arc(&self, arc: ArcId) -> Arc {
        self.arcs[arc].unwrap()
    }

    fn is_singleton(&self, node: NodeId) -> bool {
        self.arc_lists.is_empty(self.nodes[node].arcs)
    }

    fn get_tree(&self, node: NodeId) -> SequenceId {
        if self.is_singleton(node) {
            self.nodes[node].singleton_tree.unwrap()
        } else {
            let first = self.arc_lists.first(self.nodes[node].arcs).unwrap();
            let arc = *self.arc_lists.value(first);
            self.trees.tree_of(self.arc(arc).tree_node).unwrap()
        }
    }

    fn create_arc(&mut self, target: NodeId, tree_node: SequenceNode) -> ArcId {
        let id = self.arcs.len();
        self.arcs.push(None);
        let detached = self.arc_lists.allocate_node(id);
        self.arcs[id] = Some(Arc {
            target,
            list_node: detached,
            tree_node,
        });
        id
    }

    fn add_arc_last(&mut self, source: NodeId, arc: ArcId) {
        let list = self.nodes[source].arcs;
        let target = self.arc(arc).target;
        self.arc_lists
            .add_node_last(list, self.arc(arc).list_node)
            .unwrap();
        self.nodes[source].target_map.insert(target, arc);
    }

    fn add_arc_after(&mut self, source: NodeId, previous_arc: ArcId, arc: ArcId) {
        let list = self.nodes[source].arcs;
        let target = self.arc(arc).target;
        let successor = self
            .arc_lists
            .next(self.arc(previous_arc).list_node)
            .unwrap();
        self.arc_lists
            .add_node_before(list, self.arc(arc).list_node, successor)
            .unwrap();
        self.nodes[source].target_map.insert(target, arc);
    }

    fn remove_arc(&mut self, source: NodeId, arc: ArcId) {
        let value = self.arc(arc);
        self.arc_lists
            .remove_node(self.nodes[source].arcs, value.list_node);
        self.nodes[source].target_map.remove(&value.target);
        self.arcs[arc] = None;
    }

    fn make_root(&mut self, tree: SequenceId, node: NodeId) {
        if let Some(first) = self.arc_lists.first(self.nodes[node].arcs) {
            let arc = *self.arc_lists.value(first);
            self.make_first_arc(tree, arc);
        }
    }

    fn make_first_arc(&mut self, tree: SequenceId, arc: ArcId) {
        let right = self.trees.split_before(tree, self.arc(arc).tree_node);
        self.trees.merge_before(tree, right);
    }

    fn make_last_arc(&mut self, tree: SequenceId, node: NodeId, arc: ArcId) {
        if self.arc_lists.len(self.nodes[node].arcs) == 1 {
            self.make_root(tree, node);
        } else {
            let next_node = self.arc_lists.next(self.arc(arc).list_node).unwrap();
            let next_arc = *self.arc_lists.value(next_node);
            self.make_first_arc(tree, next_arc);
        }
    }
}
