// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Arena-backed implicit AVL sequences for Euler tours.
//!
//! There are no keys: in-order traversal is the sequence order. Multiple logical sequences share
//! the arena, so stable indices survive logarithmic split and concatenation.

#![cfg_attr(not(test), allow(dead_code))]

/// Stable handle to a node in a [`SequenceArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SequenceNode(pub(crate) usize);

/// Handle to one logical sequence in a [`SequenceArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SequenceId(pub(crate) usize);

#[derive(Debug, Clone)]
struct Node<T> {
    value: T,
    parent: Option<SequenceNode>,
    left: Option<SequenceNode>,
    right: Option<SequenceNode>,
    predecessor: Option<SequenceNode>,
    successor: Option<SequenceNode>,
    subtree_min: SequenceNode,
    subtree_max: SequenceNode,
    height: usize,
    size: usize,
    /// Present only on a logical tree root.
    tree: Option<SequenceId>,
}

/// Arena containing one or more implicit AVL sequences.
#[derive(Debug, Clone, Default)]
pub struct SequenceArena<T> {
    nodes: Vec<Node<T>>,
    roots: Vec<Option<SequenceNode>>,
}

impl<T> SequenceArena<T> {
    pub fn new() -> Self {
        SequenceArena {
            nodes: Vec::new(),
            roots: Vec::new(),
        }
    }

    /// Creates an empty logical tree.
    pub fn create_tree(&mut self) -> SequenceId {
        let id = SequenceId(self.roots.len());
        self.roots.push(None);
        id
    }

    pub fn value(&self, node: SequenceNode) -> &T {
        &self.nodes[node.0].value
    }

    pub fn root(&self, tree: SequenceId) -> Option<SequenceNode> {
        self.roots[tree.0]
    }

    pub fn min(&self, tree: SequenceId) -> Option<SequenceNode> {
        self.root(tree).map(|root| self.nodes[root.0].subtree_min)
    }

    pub fn max(&self, tree: SequenceId) -> Option<SequenceNode> {
        self.root(tree).map(|root| self.nodes[root.0].subtree_max)
    }

    pub fn successor(&self, node: SequenceNode) -> Option<SequenceNode> {
        self.nodes[node.0].successor
    }

    pub fn predecessor(&self, node: SequenceNode) -> Option<SequenceNode> {
        self.nodes[node.0].predecessor
    }

    pub fn parent(&self, node: SequenceNode) -> Option<SequenceNode> {
        self.nodes[node.0].parent
    }

    pub fn left(&self, node: SequenceNode) -> Option<SequenceNode> {
        self.nodes[node.0].left
    }

    pub fn right(&self, node: SequenceNode) -> Option<SequenceNode> {
        self.nodes[node.0].right
    }

    pub fn subtree_min(&self, node: SequenceNode) -> SequenceNode {
        self.nodes[node.0].subtree_min
    }

    pub fn subtree_max(&self, node: SequenceNode) -> SequenceNode {
        self.nodes[node.0].subtree_max
    }

    pub fn height(&self, node: SequenceNode) -> usize {
        self.nodes[node.0].height
    }

    pub fn subtree_size(&self, node: SequenceNode) -> usize {
        self.nodes[node.0].size
    }

    pub fn size(&self, tree: SequenceId) -> usize {
        self.root(tree).map_or(0, |root| self.nodes[root.0].size)
    }

    pub fn is_empty(&self, tree: SequenceId) -> bool {
        self.root(tree).is_none()
    }

    /// Finds the logical tree containing `node` in `O(log n)` time.
    pub fn tree_of(&self, node: SequenceNode) -> Option<SequenceId> {
        let root = self.root_of(node);
        self.nodes[root.0].tree
    }

    pub fn add_max(&mut self, tree: SequenceId, value: T) -> SequenceNode {
        let node = self.allocate(value);
        let root = self.root(tree);
        if let Some(max) = self.max(tree) {
            self.nodes[max.0].successor = Some(node);
            self.nodes[node.0].predecessor = Some(max);
        }
        let root = self.join_with(root, node, None);
        self.set_root(tree, root);
        node
    }

    pub fn add_min(&mut self, tree: SequenceId, value: T) -> SequenceNode {
        let node = self.allocate(value);
        let root = self.root(tree);
        if let Some(min) = self.min(tree) {
            self.nodes[min.0].predecessor = Some(node);
            self.nodes[node.0].successor = Some(min);
        }
        let root = self.join_with(None, node, root);
        self.set_root(tree, root);
        node
    }

    pub fn remove_min(&mut self, tree: SequenceId) -> Option<SequenceNode> {
        let root = self.root(tree)?;
        let (removed, rest) = self.split_root(root, 1);
        let removed = removed.unwrap();
        let next = self.nodes[removed.0].successor;
        if let Some(next) = next {
            self.nodes[next.0].predecessor = None;
        }
        self.reset_detached(removed);
        self.set_root(tree, rest);
        Some(removed)
    }

    pub fn remove_max(&mut self, tree: SequenceId) -> Option<SequenceNode> {
        let root = self.root(tree)?;
        let count = self.nodes[root.0].size;
        let (rest, removed) = self.split_root(root, count - 1);
        let removed = removed.unwrap();
        let previous = self.nodes[removed.0].predecessor;
        if let Some(previous) = previous {
            self.nodes[previous.0].successor = None;
        }
        self.reset_detached(removed);
        self.set_root(tree, rest);
        Some(removed)
    }

    /// Leaves `node` in `tree` and returns a tree containing everything after it.
    pub fn split_after(&mut self, tree: SequenceId, node: SequenceNode) -> SequenceId {
        debug_assert_eq!(self.tree_of(node), Some(tree));
        let right_tree = self.create_tree();
        let rank = self.rank(node);
        let root = self.root(tree).unwrap();
        let (left, right) = self.split_root(root, rank + 1);
        self.sever_boundary(left, right);
        self.set_root(tree, left);
        self.set_root(right_tree, right);
        right_tree
    }

    /// Returns a tree containing `node` and everything after it.
    pub fn split_before(&mut self, tree: SequenceId, node: SequenceNode) -> SequenceId {
        debug_assert_eq!(self.tree_of(node), Some(tree));
        let right_tree = self.create_tree();
        let rank = self.rank(node);
        let root = self.root(tree).unwrap();
        let (left, right) = self.split_root(root, rank);
        self.sever_boundary(left, right);
        self.set_root(tree, left);
        self.set_root(right_tree, right);
        right_tree
    }

    /// Appends all nodes of `other` and empties `other`.
    pub fn merge_after(&mut self, tree: SequenceId, other: SequenceId) {
        assert_ne!(tree, other);
        let left = self.take_root(tree);
        let right = self.take_root(other);
        let root = self.concatenate(left, right);
        self.set_root(tree, root);
    }

    /// Prepends all nodes of `other` and empties `other`.
    pub fn merge_before(&mut self, tree: SequenceId, other: SequenceId) {
        assert_ne!(tree, other);
        let right = self.take_root(tree);
        let left = self.take_root(other);
        let root = self.concatenate(left, right);
        self.set_root(tree, root);
    }

    pub fn node_ids(&self, tree: SequenceId) -> Vec<SequenceNode> {
        let mut result = Vec::with_capacity(self.size(tree));
        let mut current = self.min(tree);
        while let Some(node) = current {
            result.push(node);
            current = self.successor(node);
        }
        result
    }

    fn allocate(&mut self, value: T) -> SequenceNode {
        let id = SequenceNode(self.nodes.len());
        self.nodes.push(Node {
            value,
            parent: None,
            left: None,
            right: None,
            predecessor: None,
            successor: None,
            subtree_min: id,
            subtree_max: id,
            height: 1,
            size: 1,
            tree: None,
        });
        id
    }

    fn root_of(&self, mut node: SequenceNode) -> SequenceNode {
        while let Some(parent) = self.nodes[node.0].parent {
            node = parent;
        }
        node
    }

    fn rank(&self, mut node: SequenceNode) -> usize {
        let mut rank = self.node_size(self.nodes[node.0].left);
        while let Some(parent) = self.nodes[node.0].parent {
            if self.nodes[parent.0].right == Some(node) {
                rank += self.node_size(self.nodes[parent.0].left) + 1;
            }
            node = parent;
        }
        rank
    }

    fn node_height(&self, node: Option<SequenceNode>) -> usize {
        node.map_or(0, |node| self.nodes[node.0].height)
    }

    fn node_size(&self, node: Option<SequenceNode>) -> usize {
        node.map_or(0, |node| self.nodes[node.0].size)
    }

    fn update(&mut self, node: SequenceNode) {
        let (left, right) = (self.nodes[node.0].left, self.nodes[node.0].right);
        self.nodes[node.0].height = 1 + self.node_height(left).max(self.node_height(right));
        self.nodes[node.0].size = 1 + self.node_size(left) + self.node_size(right);
        self.nodes[node.0].subtree_min = left.map_or(node, |left| self.nodes[left.0].subtree_min);
        self.nodes[node.0].subtree_max =
            right.map_or(node, |right| self.nodes[right.0].subtree_max);
        if let Some(left) = left {
            self.nodes[left.0].parent = Some(node);
        }
        if let Some(right) = right {
            self.nodes[right.0].parent = Some(node);
        }
    }

    fn reset_structure(&mut self, node: SequenceNode) {
        self.nodes[node.0].parent = None;
        self.nodes[node.0].left = None;
        self.nodes[node.0].right = None;
        self.nodes[node.0].subtree_min = node;
        self.nodes[node.0].subtree_max = node;
        self.nodes[node.0].height = 1;
        self.nodes[node.0].size = 1;
        self.nodes[node.0].tree = None;
    }

    fn reset_detached(&mut self, node: SequenceNode) {
        self.reset_structure(node);
        self.nodes[node.0].predecessor = None;
        self.nodes[node.0].successor = None;
    }

    fn set_root(&mut self, tree: SequenceId, root: Option<SequenceNode>) {
        self.roots[tree.0] = root;
        if let Some(root) = root {
            self.nodes[root.0].parent = None;
            self.nodes[root.0].tree = Some(tree);
        }
    }

    fn take_root(&mut self, tree: SequenceId) -> Option<SequenceNode> {
        let root = self.roots[tree.0].take();
        if let Some(root) = root {
            self.nodes[root.0].tree = None;
        }
        root
    }

    fn rotate_left(&mut self, node: SequenceNode) -> SequenceNode {
        let right = self.nodes[node.0].right.unwrap();
        let middle = self.nodes[right.0].left;
        self.nodes[node.0].right = middle;
        if let Some(middle) = middle {
            self.nodes[middle.0].parent = Some(node);
        }
        self.nodes[right.0].left = Some(node);
        self.nodes[node.0].parent = Some(right);
        self.nodes[right.0].parent = None;
        self.update(node);
        self.update(right);
        right
    }

    fn rotate_right(&mut self, node: SequenceNode) -> SequenceNode {
        let left = self.nodes[node.0].left.unwrap();
        let middle = self.nodes[left.0].right;
        self.nodes[node.0].left = middle;
        if let Some(middle) = middle {
            self.nodes[middle.0].parent = Some(node);
        }
        self.nodes[left.0].right = Some(node);
        self.nodes[node.0].parent = Some(left);
        self.nodes[left.0].parent = None;
        self.update(node);
        self.update(left);
        left
    }

    fn balance_node(&mut self, node: SequenceNode) -> SequenceNode {
        self.update(node);
        let left = self.nodes[node.0].left;
        let right = self.nodes[node.0].right;
        if self.node_height(left) > self.node_height(right) + 1 {
            let left = left.unwrap();
            if self.node_height(self.nodes[left.0].right)
                > self.node_height(self.nodes[left.0].left)
            {
                let new_left = self.rotate_left(left);
                self.nodes[node.0].left = Some(new_left);
                self.nodes[new_left.0].parent = Some(node);
            }
            self.rotate_right(node)
        } else if self.node_height(right) > self.node_height(left) + 1 {
            let right = right.unwrap();
            if self.node_height(self.nodes[right.0].left)
                > self.node_height(self.nodes[right.0].right)
            {
                let new_right = self.rotate_right(right);
                self.nodes[node.0].right = Some(new_right);
                self.nodes[new_right.0].parent = Some(node);
            }
            self.rotate_left(node)
        } else {
            self.nodes[node.0].parent = None;
            node
        }
    }

    /// Knuth's height-guided AVL concatenation, used by Java's implementation as well.
    fn join_with(
        &mut self,
        left: Option<SequenceNode>,
        junction: SequenceNode,
        right: Option<SequenceNode>,
    ) -> Option<SequenceNode> {
        if self.node_height(left) > self.node_height(right) + 1 {
            let left_root = left.unwrap();
            let left_right = self.nodes[left_root.0].right;
            if let Some(left_right) = left_right {
                self.nodes[left_right.0].parent = None;
            }
            let joined = self.join_with(left_right, junction, right);
            self.nodes[left_root.0].right = joined;
            if let Some(joined) = joined {
                self.nodes[joined.0].parent = Some(left_root);
            }
            Some(self.balance_node(left_root))
        } else if self.node_height(right) > self.node_height(left) + 1 {
            let right_root = right.unwrap();
            let right_left = self.nodes[right_root.0].left;
            if let Some(right_left) = right_left {
                self.nodes[right_left.0].parent = None;
            }
            let joined = self.join_with(left, junction, right_left);
            self.nodes[right_root.0].left = joined;
            if let Some(joined) = joined {
                self.nodes[joined.0].parent = Some(right_root);
            }
            Some(self.balance_node(right_root))
        } else {
            self.reset_structure(junction);
            self.nodes[junction.0].left = left;
            self.nodes[junction.0].right = right;
            self.update(junction);
            self.nodes[junction.0].parent = None;
            Some(junction)
        }
    }

    fn split_root(
        &mut self,
        root: SequenceNode,
        count: usize,
    ) -> (Option<SequenceNode>, Option<SequenceNode>) {
        if count == 0 {
            self.nodes[root.0].parent = None;
            return (None, Some(root));
        }
        if count == self.nodes[root.0].size {
            self.nodes[root.0].parent = None;
            return (Some(root), None);
        }
        let left = self.nodes[root.0].left;
        let right = self.nodes[root.0].right;
        let left_size = self.node_size(left);
        if let Some(left) = left {
            self.nodes[left.0].parent = None;
        }
        if let Some(right) = right {
            self.nodes[right.0].parent = None;
        }
        self.reset_structure(root);

        if count <= left_size {
            let (first, middle) = self.split_root(left.unwrap(), count);
            let second = self.join_with(middle, root, right);
            (first, second)
        } else if count == left_size + 1 {
            let first = self.join_with(left, root, None);
            (first, right)
        } else {
            let (middle, second) = self.split_root(right.unwrap(), count - left_size - 1);
            let first = self.join_with(left, root, middle);
            (first, second)
        }
    }

    fn concatenate(
        &mut self,
        left: Option<SequenceNode>,
        right: Option<SequenceNode>,
    ) -> Option<SequenceNode> {
        let (Some(left), Some(right)) = (left, right) else {
            return left.or(right);
        };
        let left_max = self.nodes[left.0].subtree_max;
        let right_min = self.nodes[right.0].subtree_min;
        self.nodes[left_max.0].successor = Some(right_min);
        self.nodes[right_min.0].predecessor = Some(left_max);

        let (junction, right) = self.split_root(right, 1);
        let junction = junction.unwrap();
        self.join_with(Some(left), junction, right)
    }

    fn sever_boundary(&mut self, left: Option<SequenceNode>, right: Option<SequenceNode>) {
        if let Some(left) = left {
            let max = self.nodes[left.0].subtree_max;
            self.nodes[max.0].successor = None;
        }
        if let Some(right) = right {
            let min = self.nodes[right.0].subtree_min;
            self.nodes[min.0].predecessor = None;
        }
    }
}
