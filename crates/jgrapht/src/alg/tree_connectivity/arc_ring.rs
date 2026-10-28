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

//! Arena-backed intrusive circular rings for the outgoing arcs of Euler-tour vertices.
//!
//! A node belongs to at most one logical ring and its index remains stable across removal and
//! splicing.

#![cfg_attr(not(test), allow(dead_code))]

use crate::{GraphError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RingId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RingNode(pub(crate) usize);

#[derive(Debug, Clone)]
struct Node<T> {
    value: T,
    next: Option<RingNode>,
    previous: Option<RingNode>,
    list: Option<RingId>,
}

#[derive(Debug, Clone, Default)]
struct List {
    head: Option<RingNode>,
    len: usize,
}

/// Arena containing circular doubly linked lists.
#[derive(Debug, Clone, Default)]
pub struct ArcRingArena<T> {
    nodes: Vec<Node<T>>,
    lists: Vec<List>,
}

impl<T> ArcRingArena<T> {
    pub fn new() -> Self {
        ArcRingArena {
            nodes: Vec::new(),
            lists: Vec::new(),
        }
    }

    pub fn create_list(&mut self) -> RingId {
        let id = RingId(self.lists.len());
        self.lists.push(List::default());
        id
    }

    pub fn allocate_node(&mut self, value: T) -> RingNode {
        let id = RingNode(self.nodes.len());
        self.nodes.push(Node {
            value,
            next: None,
            previous: None,
            list: None,
        });
        id
    }

    pub fn value(&self, node: RingNode) -> &T {
        &self.nodes[node.0].value
    }

    pub fn value_mut(&mut self, node: RingNode) -> &mut T {
        &mut self.nodes[node.0].value
    }

    pub fn list_of(&self, node: RingNode) -> Option<RingId> {
        self.nodes[node.0].list
    }

    /// The list is circular: the last node's successor is the first node.
    pub fn next(&self, node: RingNode) -> Option<RingNode> {
        self.nodes[node.0].next
    }

    /// The list is circular: the first node's predecessor is the last node.
    pub fn previous(&self, node: RingNode) -> Option<RingNode> {
        self.nodes[node.0].previous
    }

    pub fn len(&self, list: RingId) -> usize {
        self.lists[list.0].len
    }

    pub fn is_empty(&self, list: RingId) -> bool {
        self.len(list) == 0
    }

    pub fn first(&self, list: RingId) -> Option<RingNode> {
        self.lists[list.0].head
    }

    pub fn last(&self, list: RingId) -> Option<RingNode> {
        self.first(list)
            .and_then(|head| self.nodes[head.0].previous)
    }

    pub fn get_node(&self, list: RingId, index: usize) -> Option<RingNode> {
        let len = self.len(list);
        if index >= len {
            return None;
        }
        if index < len / 2 {
            let mut node = self.first(list).unwrap();
            for _ in 0..index {
                node = self.next(node).unwrap();
            }
            Some(node)
        } else {
            let mut node = self.last(list).unwrap();
            for _ in index + 1..len {
                node = self.previous(node).unwrap();
            }
            Some(node)
        }
    }

    pub fn contains_node(&self, list: RingId, node: RingNode) -> bool {
        self.list_of(node) == Some(list)
    }

    pub fn index_of_node(&self, list: RingId, node: RingNode) -> Option<usize> {
        if !self.contains_node(list, node) {
            return None;
        }
        let mut current = self.first(list).unwrap();
        for index in 0..self.len(list) {
            if current == node {
                return Some(index);
            }
            current = self.next(current).unwrap();
        }
        None
    }

    pub fn add_node_first(&mut self, list: RingId, node: RingNode) -> Result<()> {
        self.add_node(list, 0, node)
    }

    pub fn add_node_last(&mut self, list: RingId, node: RingNode) -> Result<()> {
        let len = self.len(list);
        self.add_node(list, len, node)
    }

    pub fn add_node(&mut self, list: RingId, index: usize, node: RingNode) -> Result<()> {
        if index > self.len(list) {
            return Err(GraphError::InvalidParameter {
                name: "index",
                requirement: crate::ParameterRequirement::InBounds,
            });
        }
        self.ensure_detached(node)?;
        if self.is_empty(list) {
            self.nodes[node.0].next = Some(node);
            self.nodes[node.0].previous = Some(node);
            self.nodes[node.0].list = Some(list);
            self.lists[list.0].head = Some(node);
            self.lists[list.0].len = 1;
            return Ok(());
        }
        let successor = if index == self.len(list) {
            self.first(list).unwrap()
        } else {
            self.get_node(list, index).unwrap()
        };
        self.link_before(list, node, successor);
        if index == 0 {
            self.lists[list.0].head = Some(node);
        }
        Ok(())
    }

    pub fn add_node_before(
        &mut self,
        list: RingId,
        node: RingNode,
        successor: RingNode,
    ) -> Result<()> {
        if !self.contains_node(list, successor) {
            return Err(GraphError::ElementNotFound);
        }
        self.ensure_detached(node)?;
        self.link_before(list, node, successor);
        if self.first(list) == Some(successor) {
            self.lists[list.0].head = Some(node);
        }
        Ok(())
    }

    pub fn add_element_first(&mut self, list: RingId, value: T) -> RingNode {
        let node = self.allocate_node(value);
        self.add_node_first(list, node).unwrap();
        node
    }

    pub fn add_element_last(&mut self, list: RingId, value: T) -> RingNode {
        let node = self.allocate_node(value);
        self.add_node_last(list, node).unwrap();
        node
    }

    pub fn add_element_before(
        &mut self,
        list: RingId,
        successor: RingNode,
        value: T,
    ) -> Result<RingNode> {
        let node = self.allocate_node(value);
        self.add_node_before(list, node, successor)?;
        Ok(node)
    }

    pub fn remove_node(&mut self, list: RingId, node: RingNode) -> bool {
        if !self.contains_node(list, node) {
            return false;
        }
        let next = self.nodes[node.0].next.unwrap();
        let previous = self.nodes[node.0].previous.unwrap();
        if self.len(list) == 1 {
            self.lists[list.0].head = None;
        } else {
            self.nodes[previous.0].next = Some(next);
            self.nodes[next.0].previous = Some(previous);
            if self.first(list) == Some(node) {
                self.lists[list.0].head = Some(next);
            }
        }
        self.lists[list.0].len -= 1;
        self.nodes[node.0].next = None;
        self.nodes[node.0].previous = None;
        self.nodes[node.0].list = None;
        true
    }

    pub fn clear(&mut self, list: RingId) {
        while let Some(node) = self.first(list) {
            self.remove_node(list, node);
        }
    }

    /// Reverses the list in place, including its circular links.
    pub fn invert(&mut self, list: RingId) {
        let nodes = self.node_ids(list);
        for node in nodes {
            let links = &mut self.nodes[node.0];
            std::mem::swap(&mut links.next, &mut links.previous);
        }
        if let Some(head) = self.first(list) {
            self.lists[list.0].head = self.nodes[head.0].next;
        }
    }

    pub fn append(&mut self, list: RingId, moved: RingId) -> Result<()> {
        let index = self.len(list);
        self.move_from(list, index, moved)
    }

    pub fn prepend(&mut self, list: RingId, moved: RingId) -> Result<()> {
        self.move_from(list, 0, moved)
    }

    /// Moves all nodes from `moved` before `index`, preserving their handles and order.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DuplicateElement`] if an element is already present;
    /// [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn move_from(&mut self, list: RingId, index: usize, moved: RingId) -> Result<()> {
        if list == moved {
            return Err(GraphError::DuplicateElement);
        }
        if index > self.len(list) {
            return Err(GraphError::InvalidParameter {
                name: "index",
                requirement: crate::ParameterRequirement::InBounds,
            });
        }
        if self.is_empty(moved) {
            return Ok(());
        }

        let moved_nodes = self.node_ids(moved);
        for &node in &moved_nodes {
            self.nodes[node.0].list = Some(list);
        }
        let moved_head = self.first(moved).unwrap();
        let moved_tail = self.last(moved).unwrap();
        let moved_len = self.len(moved);
        if self.is_empty(list) {
            self.lists[list.0].head = Some(moved_head);
        } else {
            let successor = if index == self.len(list) {
                self.first(list).unwrap()
            } else {
                self.get_node(list, index).unwrap()
            };
            let predecessor = self.previous(successor).unwrap();
            self.nodes[predecessor.0].next = Some(moved_head);
            self.nodes[moved_head.0].previous = Some(predecessor);
            self.nodes[moved_tail.0].next = Some(successor);
            self.nodes[successor.0].previous = Some(moved_tail);
            if index == 0 {
                self.lists[list.0].head = Some(moved_head);
            }
        }
        self.lists[list.0].len += moved_len;
        self.lists[moved.0] = List::default();
        Ok(())
    }

    pub fn node_ids(&self, list: RingId) -> Vec<RingNode> {
        let mut result = Vec::with_capacity(self.len(list));
        let Some(mut node) = self.first(list) else {
            return result;
        };
        for _ in 0..self.len(list) {
            result.push(node);
            node = self.next(node).unwrap();
        }
        result
    }

    pub fn reverse_node_ids(&self, list: RingId) -> Vec<RingNode> {
        let mut result = Vec::with_capacity(self.len(list));
        let Some(mut node) = self.last(list) else {
            return result;
        };
        for _ in 0..self.len(list) {
            result.push(node);
            node = self.previous(node).unwrap();
        }
        result
    }

    pub fn circular_node_ids(&self, list: RingId, first: RingNode) -> Option<Vec<RingNode>> {
        if !self.contains_node(list, first) {
            return None;
        }
        let mut result = Vec::with_capacity(self.len(list));
        let mut node = first;
        for _ in 0..self.len(list) {
            result.push(node);
            node = self.next(node).unwrap();
        }
        Some(result)
    }

    pub fn reverse_circular_node_ids(
        &self,
        list: RingId,
        first: RingNode,
    ) -> Option<Vec<RingNode>> {
        if !self.contains_node(list, first) {
            return None;
        }
        let mut result = Vec::with_capacity(self.len(list));
        let mut node = first;
        for _ in 0..self.len(list) {
            result.push(node);
            node = self.previous(node).unwrap();
        }
        Some(result)
    }

    fn ensure_detached(&self, node: RingNode) -> Result<()> {
        if self.list_of(node).is_some() {
            Err(GraphError::DuplicateElement)
        } else {
            Ok(())
        }
    }

    fn link_before(&mut self, list: RingId, node: RingNode, successor: RingNode) {
        let previous = self.nodes[successor.0].previous.unwrap();
        self.nodes[node.0].list = Some(list);
        self.nodes[node.0].previous = Some(previous);
        self.nodes[node.0].next = Some(successor);
        self.nodes[previous.0].next = Some(node);
        self.nodes[successor.0].previous = Some(node);
        self.lists[list.0].len += 1;
    }
}

impl<T: PartialEq> ArcRingArena<T> {
    pub fn node_of(&self, list: RingId, value: &T) -> Option<RingNode> {
        self.node_ids(list)
            .into_iter()
            .find(|&node| self.value(node) == value)
    }

    pub fn last_node_of(&self, list: RingId, value: &T) -> Option<RingNode> {
        self.reverse_node_ids(list)
            .into_iter()
            .find(|&node| self.value(node) == value)
    }

    pub fn remove_first_occurrence(&mut self, list: RingId, value: &T) -> bool {
        self.node_of(list, value)
            .is_some_and(|node| self.remove_node(list, node))
    }

    pub fn remove_last_occurrence(&mut self, list: RingId, value: &T) -> bool {
        self.last_node_of(list, value)
            .is_some_and(|node| self.remove_node(list, node))
    }
}
