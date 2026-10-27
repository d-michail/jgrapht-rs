//! Arena/index based port of JGraphT's circular `DoublyLinkedList`.
//!
//! A node belongs to at most one logical list. Node handles remain stable across removal and the
//! bulk list-splicing operations, matching the property for which the Java utility exists.

use crate::{GraphError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListNodeId(pub(crate) usize);

#[derive(Debug, Clone)]
struct Node<T> {
    value: T,
    next: Option<ListNodeId>,
    previous: Option<ListNodeId>,
    list: Option<ListId>,
}

#[derive(Debug, Clone, Default)]
struct List {
    head: Option<ListNodeId>,
    len: usize,
}

/// Arena containing circular doubly linked lists.
#[derive(Debug, Clone, Default)]
pub struct DoublyLinkedList<T> {
    nodes: Vec<Node<T>>,
    lists: Vec<List>,
}

impl<T> DoublyLinkedList<T> {
    pub fn new() -> Self {
        DoublyLinkedList {
            nodes: Vec::new(),
            lists: Vec::new(),
        }
    }

    pub fn create_list(&mut self) -> ListId {
        let id = ListId(self.lists.len());
        self.lists.push(List::default());
        id
    }

    pub fn allocate_node(&mut self, value: T) -> ListNodeId {
        let id = ListNodeId(self.nodes.len());
        self.nodes.push(Node {
            value,
            next: None,
            previous: None,
            list: None,
        });
        id
    }

    pub fn value(&self, node: ListNodeId) -> &T {
        &self.nodes[node.0].value
    }

    pub fn value_mut(&mut self, node: ListNodeId) -> &mut T {
        &mut self.nodes[node.0].value
    }

    pub fn list_of(&self, node: ListNodeId) -> Option<ListId> {
        self.nodes[node.0].list
    }

    /// The list is circular: the last node's successor is the first node.
    pub fn next(&self, node: ListNodeId) -> Option<ListNodeId> {
        self.nodes[node.0].next
    }

    /// The list is circular: the first node's predecessor is the last node.
    pub fn previous(&self, node: ListNodeId) -> Option<ListNodeId> {
        self.nodes[node.0].previous
    }

    pub fn len(&self, list: ListId) -> usize {
        self.lists[list.0].len
    }

    pub fn is_empty(&self, list: ListId) -> bool {
        self.len(list) == 0
    }

    pub fn first(&self, list: ListId) -> Option<ListNodeId> {
        self.lists[list.0].head
    }

    pub fn last(&self, list: ListId) -> Option<ListNodeId> {
        self.first(list)
            .and_then(|head| self.nodes[head.0].previous)
    }

    pub fn get_node(&self, list: ListId, index: usize) -> Option<ListNodeId> {
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

    pub fn contains_node(&self, list: ListId, node: ListNodeId) -> bool {
        self.list_of(node) == Some(list)
    }

    pub fn index_of_node(&self, list: ListId, node: ListNodeId) -> Option<usize> {
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

    pub fn add_node_first(&mut self, list: ListId, node: ListNodeId) -> Result<()> {
        self.add_node(list, 0, node)
    }

    pub fn add_node_last(&mut self, list: ListId, node: ListNodeId) -> Result<()> {
        let len = self.len(list);
        self.add_node(list, len, node)
    }

    pub fn add_node(&mut self, list: ListId, index: usize, node: ListNodeId) -> Result<()> {
        if index > self.len(list) {
            return Err(GraphError::InvalidArgument("index out of bounds".into()));
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
        list: ListId,
        node: ListNodeId,
        successor: ListNodeId,
    ) -> Result<()> {
        if !self.contains_node(list, successor) {
            return Err(GraphError::InvalidArgument(
                "successor is not in this list".into(),
            ));
        }
        self.ensure_detached(node)?;
        self.link_before(list, node, successor);
        if self.first(list) == Some(successor) {
            self.lists[list.0].head = Some(node);
        }
        Ok(())
    }

    pub fn add_element_first(&mut self, list: ListId, value: T) -> ListNodeId {
        let node = self.allocate_node(value);
        self.add_node_first(list, node).unwrap();
        node
    }

    pub fn add_element_last(&mut self, list: ListId, value: T) -> ListNodeId {
        let node = self.allocate_node(value);
        self.add_node_last(list, node).unwrap();
        node
    }

    pub fn add_element_before(
        &mut self,
        list: ListId,
        successor: ListNodeId,
        value: T,
    ) -> Result<ListNodeId> {
        let node = self.allocate_node(value);
        self.add_node_before(list, node, successor)?;
        Ok(node)
    }

    pub fn remove_node(&mut self, list: ListId, node: ListNodeId) -> bool {
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

    pub fn remove_at(&mut self, list: ListId, index: usize) -> Option<ListNodeId> {
        let node = self.get_node(list, index)?;
        self.remove_node(list, node);
        Some(node)
    }

    pub fn clear(&mut self, list: ListId) {
        while let Some(node) = self.first(list) {
            self.remove_node(list, node);
        }
    }

    /// Reverses the list in place, including its circular links.
    pub fn invert(&mut self, list: ListId) {
        let nodes = self.node_ids(list);
        for node in nodes {
            let next = self.nodes[node.0].next;
            self.nodes[node.0].next = self.nodes[node.0].previous;
            self.nodes[node.0].previous = next;
        }
        if let Some(head) = self.first(list) {
            self.lists[list.0].head = self.nodes[head.0].next;
        }
    }

    pub fn append(&mut self, list: ListId, moved: ListId) -> Result<()> {
        let index = self.len(list);
        self.move_from(list, index, moved)
    }

    pub fn prepend(&mut self, list: ListId, moved: ListId) -> Result<()> {
        self.move_from(list, 0, moved)
    }

    /// Moves all nodes from `moved` before `index`, preserving their handles and order.
    pub fn move_from(&mut self, list: ListId, index: usize, moved: ListId) -> Result<()> {
        if list == moved {
            return Err(GraphError::InvalidArgument(
                "cannot move a list into itself".into(),
            ));
        }
        if index > self.len(list) {
            return Err(GraphError::InvalidArgument("index out of bounds".into()));
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

    pub fn node_ids(&self, list: ListId) -> Vec<ListNodeId> {
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

    pub fn reverse_node_ids(&self, list: ListId) -> Vec<ListNodeId> {
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

    pub fn circular_node_ids(&self, list: ListId, first: ListNodeId) -> Option<Vec<ListNodeId>> {
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
        list: ListId,
        first: ListNodeId,
    ) -> Option<Vec<ListNodeId>> {
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

    fn ensure_detached(&self, node: ListNodeId) -> Result<()> {
        if self.list_of(node).is_some() {
            Err(GraphError::InvalidArgument(
                "node is already contained in a list".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn link_before(&mut self, list: ListId, node: ListNodeId, successor: ListNodeId) {
        let previous = self.nodes[successor.0].previous.unwrap();
        self.nodes[node.0].list = Some(list);
        self.nodes[node.0].previous = Some(previous);
        self.nodes[node.0].next = Some(successor);
        self.nodes[previous.0].next = Some(node);
        self.nodes[successor.0].previous = Some(node);
        self.lists[list.0].len += 1;
    }
}

impl<T: PartialEq> DoublyLinkedList<T> {
    pub fn node_of(&self, list: ListId, value: &T) -> Option<ListNodeId> {
        self.node_ids(list)
            .into_iter()
            .find(|&node| self.value(node) == value)
    }

    pub fn last_node_of(&self, list: ListId, value: &T) -> Option<ListNodeId> {
        self.reverse_node_ids(list)
            .into_iter()
            .find(|&node| self.value(node) == value)
    }

    pub fn remove_first_occurrence(&mut self, list: ListId, value: &T) -> bool {
        self.node_of(list, value)
            .is_some_and(|node| self.remove_node(list, node))
    }

    pub fn remove_last_occurrence(&mut self, list: ListId, value: &T) -> bool {
        self.last_node_of(list, value)
            .is_some_and(|node| self.remove_node(list, node))
    }
}
