use crate::util::OrderedF64;
use rheaps::tree::{PairingHeap, TreeHandle};

pub(crate) type NodeId = usize;
pub(crate) type EdgeId = usize;
pub(crate) type TreeId = usize;
pub(crate) type TreeEdgeId = usize;
pub(crate) type BlossomHeap = PairingHeap<OrderedF64, usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Label {
    Plus,
    Minus,
    Infinity,
}

pub(crate) struct BlossomVNode {
    pub(crate) handle: Option<TreeHandle>,
    pub(crate) is_tree_root: bool,
    pub(crate) is_blossom: bool,
    pub(crate) is_outer: bool,
    pub(crate) is_processed: bool,
    pub(crate) is_marked: bool,
    pub(crate) label: Label,
    pub(crate) first: [Option<EdgeId>; 2],
    pub(crate) dual: f64,
    pub(crate) matched: Option<EdgeId>,
    pub(crate) best_edge: Option<EdgeId>,
    pub(crate) tree: Option<TreeId>,
    pub(crate) parent_edge: Option<EdgeId>,
    pub(crate) first_tree_child: Option<NodeId>,
    pub(crate) tree_sibling_next: Option<NodeId>,
    pub(crate) tree_sibling_prev: Option<NodeId>,
    pub(crate) blossom_parent: Option<NodeId>,
    pub(crate) blossom_grandparent: Option<NodeId>,
    pub(crate) blossom_sibling: Option<EdgeId>,
    pub(crate) pos: usize,
}

impl BlossomVNode {
    pub(crate) fn new(pos: usize) -> Self {
        Self {
            handle: None,
            is_tree_root: false,
            is_blossom: false,
            is_outer: false,
            is_processed: false,
            is_marked: false,
            label: Label::Plus,
            first: [None; 2],
            dual: 0.0,
            matched: None,
            best_edge: None,
            tree: None,
            parent_edge: None,
            first_tree_child: None,
            tree_sibling_next: None,
            tree_sibling_prev: None,
            blossom_parent: None,
            blossom_grandparent: None,
            blossom_sibling: None,
            pos,
        }
    }

    pub(crate) fn is_plus(&self) -> bool {
        self.label == Label::Plus
    }

    pub(crate) fn is_minus(&self) -> bool {
        self.label == Label::Minus
    }

    pub(crate) fn is_infinity(&self) -> bool {
        self.label == Label::Infinity
    }
}

pub(crate) struct BlossomVEdge {
    pub(crate) pos: usize,
    pub(crate) handle: Option<TreeHandle>,
    pub(crate) slack: f64,
    pub(crate) head_original: [Option<NodeId>; 2],
    pub(crate) head: [Option<NodeId>; 2],
    pub(crate) prev: [Option<EdgeId>; 2],
    pub(crate) next: [Option<EdgeId>; 2],
}

impl BlossomVEdge {
    pub(crate) fn new(pos: usize) -> Self {
        Self {
            pos,
            handle: None,
            slack: 0.0,
            head_original: [None; 2],
            head: [None; 2],
            prev: [None; 2],
            next: [None; 2],
        }
    }
}

pub(crate) struct BlossomVTree {
    pub(crate) active: bool,
    pub(crate) first: [Option<TreeEdgeId>; 2],
    pub(crate) current_edge: Option<TreeEdgeId>,
    pub(crate) current_direction: usize,
    pub(crate) eps: f64,
    pub(crate) accumulated_eps: f64,
    pub(crate) root: NodeId,
    pub(crate) next_tree: Option<TreeId>,
    pub(crate) plus_plus_edges: BlossomHeap,
    pub(crate) plus_infinity_edges: BlossomHeap,
    pub(crate) minus_blossoms: BlossomHeap,
    pub(crate) id: usize,
}

impl BlossomVTree {
    fn new(root: NodeId, id: usize) -> Self {
        Self {
            active: true,
            first: [None; 2],
            current_edge: None,
            current_direction: 0,
            eps: 0.0,
            accumulated_eps: 0.0,
            root,
            next_tree: None,
            plus_plus_edges: BlossomHeap::new(),
            plus_infinity_edges: BlossomHeap::new(),
            minus_blossoms: BlossomHeap::new(),
            id,
        }
    }
}

pub(crate) struct BlossomVTreeEdge {
    pub(crate) active: bool,
    pub(crate) head: [Option<TreeId>; 2],
    pub(crate) prev: [Option<TreeEdgeId>; 2],
    pub(crate) next: [Option<TreeEdgeId>; 2],
    pub(crate) plus_plus_edges: BlossomHeap,
    pub(crate) plus_minus_edges_0: BlossomHeap,
    pub(crate) plus_minus_edges_1: BlossomHeap,
}

impl BlossomVTreeEdge {
    fn new() -> Self {
        Self {
            active: true,
            head: [None; 2],
            prev: [None; 2],
            next: [None; 2],
            plus_plus_edges: BlossomHeap::new(),
            plus_minus_edges_0: BlossomHeap::new(),
            plus_minus_edges_1: BlossomHeap::new(),
        }
    }
}

#[derive(Default)]
pub(crate) struct BlossomArena {
    pub(crate) nodes: Vec<BlossomVNode>,
    pub(crate) edges: Vec<BlossomVEdge>,
    pub(crate) trees: Vec<BlossomVTree>,
    pub(crate) tree_edges: Vec<BlossomVTreeEdge>,
    next_tree_debug_id: usize,
}

impl BlossomArena {
    pub(crate) fn add_node(&mut self, pos: usize) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(BlossomVNode::new(pos));
        id
    }

    pub(crate) fn add_edge(&mut self, from: NodeId, to: NodeId, slack: f64, pos: usize) -> EdgeId {
        let edge = self.edges.len();
        let mut value = BlossomVEdge::new(pos);
        value.slack = slack;
        value.head_original = [Some(to), Some(from)];
        self.edges.push(value);
        self.node_add_edge(from, edge, 0);
        self.node_add_edge(to, edge, 1);
        edge
    }

    pub(crate) fn node_add_edge(&mut self, node: NodeId, edge: EdgeId, dir: usize) {
        if let Some(first) = self.nodes[node].first[dir] {
            let last = self.edges[first].prev[dir].expect("circular edge list");
            self.edges[edge].prev[dir] = Some(last);
            self.edges[edge].next[dir] = Some(first);
            self.edges[last].next[dir] = Some(edge);
            self.edges[first].prev[dir] = Some(edge);
        } else {
            self.nodes[node].first[dir] = Some(edge);
            self.edges[edge].next[dir] = Some(edge);
            self.edges[edge].prev[dir] = Some(edge);
        }
        self.edges[edge].head[1 - dir] = Some(node);
    }

    pub(crate) fn node_remove_edge(&mut self, node: NodeId, edge: EdgeId, dir: usize) {
        let prev = self.edges[edge].prev[dir].expect("edge in incident list");
        let next = self.edges[edge].next[dir].expect("edge in incident list");
        if prev == edge {
            self.nodes[node].first[dir] = None;
        } else {
            self.edges[prev].next[dir] = Some(next);
            self.edges[next].prev[dir] = Some(prev);
            if self.nodes[node].first[dir] == Some(edge) {
                self.nodes[node].first[dir] = Some(next);
            }
        }
    }

    pub(crate) fn edge_opposite(&self, edge: EdgeId, endpoint: NodeId) -> Option<NodeId> {
        let heads = self.edges[edge].head;
        if heads[0] == Some(endpoint) {
            heads[1]
        } else if heads[1] == Some(endpoint) {
            heads[0]
        } else {
            None
        }
    }

    pub(crate) fn edge_current_original(&self, edge: EdgeId, endpoint: NodeId) -> Option<NodeId> {
        if self.edges[edge].head[0] == Some(endpoint) {
            self.edges[edge].head_original[0]
        } else if self.edges[edge].head[1] == Some(endpoint) {
            self.edges[edge].head_original[1]
        } else {
            None
        }
    }

    pub(crate) fn edge_dir_from(&self, edge: EdgeId, current: NodeId) -> usize {
        usize::from(self.edges[edge].head[0] == Some(current))
    }

    pub(crate) fn move_edge_tail(&mut self, edge: EdgeId, from: NodeId, to: NodeId) {
        let dir = self.edge_dir_from(edge, from);
        self.node_remove_edge(from, edge, dir);
        self.node_add_edge(to, edge, dir);
    }

    /// Returns incident edges and their direction in Java iterator order.
    pub(crate) fn incident_edges(&self, node: NodeId) -> Vec<(EdgeId, usize)> {
        let mut result = Vec::new();
        for dir in 0..2 {
            let Some(first) = self.nodes[node].first[dir] else {
                continue;
            };
            let mut edge = first;
            loop {
                result.push((edge, dir));
                edge = self.edges[edge].next[dir].expect("circular edge list");
                if edge == first {
                    break;
                }
            }
        }
        result
    }

    pub(crate) fn edge_true_slack(&self, edge: EdgeId) -> f64 {
        let mut result = self.edges[edge].slack;
        for endpoint in self.edges[edge].head.into_iter().flatten() {
            if let Some(tree) = self.nodes[endpoint].tree {
                if self.nodes[endpoint].is_plus() {
                    result -= self.trees[tree].eps;
                } else {
                    result += self.trees[tree].eps;
                }
            }
        }
        result
    }

    pub(crate) fn node_true_dual(&self, node: NodeId) -> f64 {
        let value = &self.nodes[node];
        if value.is_infinity() || !value.is_outer {
            value.dual
        } else {
            let eps = self.trees[value.tree.expect("outer labelled node has a tree")].eps;
            if value.is_plus() {
                value.dual + eps
            } else {
                value.dual - eps
            }
        }
    }

    pub(crate) fn add_tree(&mut self, root: NodeId) -> TreeId {
        self.next_tree_debug_id += 1;
        let tree = self.trees.len();
        self.trees
            .push(BlossomVTree::new(root, self.next_tree_debug_id));
        self.nodes[root].tree = Some(tree);
        self.nodes[root].is_tree_root = true;
        tree
    }

    pub(crate) fn add_tree_edge(&mut self, from: TreeId, to: TreeId) -> TreeEdgeId {
        let edge = self.tree_edges.len();
        let mut value = BlossomVTreeEdge::new();
        value.head = [Some(to), Some(from)];
        value.next = [self.trees[from].first[0], self.trees[to].first[1]];
        self.tree_edges.push(value);

        if let Some(first) = self.trees[from].first[0] {
            self.tree_edges[first].prev[0] = Some(edge);
        }
        if let Some(first) = self.trees[to].first[1] {
            self.tree_edges[first].prev[1] = Some(edge);
        }
        self.trees[from].first[0] = Some(edge);
        self.trees[to].first[1] = Some(edge);
        self.trees[to].current_edge = Some(edge);
        self.trees[to].current_direction = 0;
        edge
    }

    pub(crate) fn tree_edges(&self, tree: TreeId) -> Vec<(TreeEdgeId, usize)> {
        let mut result = Vec::new();
        for dir in 0..2 {
            let mut edge = self.trees[tree].first[dir];
            while let Some(current) = edge {
                result.push((current, dir));
                edge = self.tree_edges[current].next[dir];
            }
        }
        result
    }

    pub(crate) fn remove_tree_edge(&mut self, edge: TreeEdgeId) {
        for dir in 0..2 {
            let prev = self.tree_edges[edge].prev[dir];
            let next = self.tree_edges[edge].next[dir];
            if let Some(prev) = prev {
                self.tree_edges[prev].next[dir] = next;
            } else {
                let owner = self.tree_edges[edge].head[1 - dir].expect("active tree edge");
                self.trees[owner].first[dir] = next;
            }
            if let Some(next) = next {
                self.tree_edges[next].prev[dir] = prev;
            }
        }
        self.tree_edges[edge].head = [None; 2];
        self.tree_edges[edge].active = false;
    }

    pub(crate) fn set_current_edges(&mut self, tree: TreeId) {
        for (edge, dir) in self.tree_edges(tree) {
            let opposite = self.tree_edges[edge].head[dir].expect("active tree edge");
            self.trees[opposite].current_edge = Some(edge);
            self.trees[opposite].current_direction = dir;
        }
    }

    pub(crate) fn clear_current_edges(&mut self, tree: TreeId) {
        self.trees[tree].current_edge = None;
        for (edge, dir) in self.tree_edges(tree) {
            let opposite = self.tree_edges[edge].head[dir].expect("active tree edge");
            self.trees[opposite].current_edge = None;
        }
    }

    pub(crate) fn add_plus_plus_edge(&mut self, tree: TreeId, edge: EdgeId) {
        let handle = self.trees[tree]
            .plus_plus_edges
            .insert(OrderedF64(self.edges[edge].slack), edge);
        self.edges[edge].handle = Some(handle);
    }

    pub(crate) fn add_plus_infinity_edge(&mut self, tree: TreeId, edge: EdgeId) {
        let handle = self.trees[tree]
            .plus_infinity_edges
            .insert(OrderedF64(self.edges[edge].slack), edge);
        self.edges[edge].handle = Some(handle);
    }

    pub(crate) fn add_minus_blossom(&mut self, tree: TreeId, blossom: NodeId) {
        let handle = self.trees[tree]
            .minus_blossoms
            .insert(OrderedF64(self.nodes[blossom].dual), blossom);
        self.nodes[blossom].handle = Some(handle);
    }

    pub(crate) fn tree_edge_minus_plus_heap(
        &self,
        edge: TreeEdgeId,
        direction: usize,
    ) -> &BlossomHeap {
        if direction == 0 {
            &self.tree_edges[edge].plus_minus_edges_0
        } else {
            &self.tree_edges[edge].plus_minus_edges_1
        }
    }

    pub(crate) fn tree_edge_plus_minus_heap(
        &self,
        edge: TreeEdgeId,
        direction: usize,
    ) -> &BlossomHeap {
        if direction == 0 {
            &self.tree_edges[edge].plus_minus_edges_1
        } else {
            &self.tree_edges[edge].plus_minus_edges_0
        }
    }

    pub(crate) fn node_tree_parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node]
            .parent_edge
            .and_then(|edge| self.edge_opposite(edge, node))
    }

    pub(crate) fn node_tree_grandparent(&self, node: NodeId) -> NodeId {
        let parent = self.node_tree_parent(node).expect("node has parent");
        self.node_tree_parent(parent).expect("node has grandparent")
    }

    pub(crate) fn node_opposite_matched(&self, node: NodeId) -> NodeId {
        self.edge_opposite(self.nodes[node].matched.expect("matched node"), node)
            .expect("matching edge endpoint")
    }

    pub(crate) fn add_child(
        &mut self,
        parent: NodeId,
        child: NodeId,
        parent_edge: EdgeId,
        grow: bool,
    ) {
        self.nodes[child].parent_edge = Some(parent_edge);
        self.nodes[child].tree = self.nodes[parent].tree;
        self.nodes[child].tree_sibling_next = self.nodes[parent].first_tree_child;
        if grow {
            self.nodes[child].first_tree_child = None;
        }
        if let Some(first) = self.nodes[parent].first_tree_child {
            self.nodes[child].tree_sibling_prev = self.nodes[first].tree_sibling_prev;
            self.nodes[first].tree_sibling_prev = Some(child);
        } else {
            self.nodes[child].tree_sibling_prev = Some(child);
        }
        self.nodes[parent].first_tree_child = Some(child);
    }

    pub(crate) fn remove_from_child_list(&mut self, node: NodeId) {
        let prev = self.nodes[node]
            .tree_sibling_prev
            .expect("node in child/root list");
        let next = self.nodes[node].tree_sibling_next;
        if self.nodes[node].is_tree_root {
            self.nodes[prev].tree_sibling_next = next;
            if let Some(next) = next {
                self.nodes[next].tree_sibling_prev = Some(prev);
            }
            return;
        }

        let parent = self
            .node_tree_parent(node)
            .expect("non-root node has parent");
        if self.nodes[prev].tree_sibling_next.is_none() {
            self.nodes[parent].first_tree_child = next;
        } else {
            self.nodes[prev].tree_sibling_next = next;
        }
        if let Some(next) = next {
            self.nodes[next].tree_sibling_prev = Some(prev);
        } else if let Some(first) = self.nodes[parent].first_tree_child {
            self.nodes[first].tree_sibling_prev = Some(prev);
        }
    }

    pub(crate) fn move_children_to(&mut self, from: NodeId, blossom: NodeId) {
        let Some(from_first) = self.nodes[from].first_tree_child else {
            return;
        };
        if let Some(blossom_first) = self.nodes[blossom].first_tree_child {
            let blossom_last = self.nodes[blossom_first]
                .tree_sibling_prev
                .expect("child list has last");
            let from_last = self.nodes[from_first]
                .tree_sibling_prev
                .expect("child list has last");
            self.nodes[from_last].tree_sibling_next = Some(blossom_first);
            self.nodes[blossom_first].tree_sibling_prev = Some(from_last);
            self.nodes[from_first].tree_sibling_prev = Some(blossom_last);
        }
        self.nodes[blossom].first_tree_child = Some(from_first);
        self.nodes[from].first_tree_child = None;
    }

    pub(crate) fn tree_nodes(&self, root: NodeId) -> Vec<NodeId> {
        let mut result = vec![root];
        let mut current = root;
        loop {
            if let Some(child) = self.nodes[current].first_tree_child {
                current = child;
                result.push(current);
                continue;
            }
            while current != root && self.nodes[current].tree_sibling_next.is_none() {
                current = self.node_tree_parent(current).expect("tree node parent");
            }
            let next = self.nodes[current].tree_sibling_next;
            if next == self.nodes[root].tree_sibling_next {
                break;
            }
            let Some(next) = next else {
                break;
            };
            current = next;
            result.push(current);
        }
        result
    }

    pub(crate) fn get_penultimate_blossom(&mut self, node: NodeId) -> NodeId {
        let mut current = node;
        loop {
            let grandparent = self.nodes[current]
                .blossom_grandparent
                .expect("contracted node has blossom grandparent");
            if !self.nodes[grandparent].is_outer {
                current = grandparent;
            } else if self.nodes[current].blossom_grandparent != self.nodes[current].blossom_parent
            {
                self.nodes[current].blossom_grandparent = self.nodes[current].blossom_parent;
            } else {
                break;
            }
        }
        let mut previous = node;
        while previous != current {
            let next = self.nodes[previous]
                .blossom_grandparent
                .expect("compressed blossom path");
            self.nodes[previous].blossom_grandparent = Some(current);
            previous = next;
        }
        current
    }

    pub(crate) fn get_penultimate_blossom_and_fix_grandparent(&mut self, node: NodeId) -> NodeId {
        let mut current = node;
        let mut previous_to_result = None;
        loop {
            let grandparent = self.nodes[current]
                .blossom_grandparent
                .expect("contracted node has blossom grandparent");
            if !self.nodes[grandparent].is_outer {
                previous_to_result = Some(current);
                current = grandparent;
            } else if self.nodes[current].blossom_grandparent != self.nodes[current].blossom_parent
            {
                self.nodes[current].blossom_grandparent = self.nodes[current].blossom_parent;
            } else {
                break;
            }
        }
        if let Some(previous_to_result) = previous_to_result {
            let mut previous = node;
            while previous != previous_to_result {
                let next = self.nodes[previous]
                    .blossom_grandparent
                    .expect("compressed blossom path");
                self.nodes[previous].blossom_grandparent = Some(previous_to_result);
                previous = next;
            }
        }
        current
    }

    pub(crate) fn blossom_nodes(&self, root: NodeId, forming_edge: EdgeId) -> Vec<(NodeId, usize)> {
        let mut result = Vec::new();
        let mut current = self.edges[forming_edge].head[0].expect("forming edge endpoint");
        loop {
            result.push((current, 0));
            if current == root {
                break;
            }
            current = self.node_tree_parent(current).expect("branch reaches root");
        }
        current = self.edges[forming_edge].head[1].expect("forming edge endpoint");
        while current != root {
            result.push((current, 1));
            let parent = self.node_tree_parent(current).expect("branch reaches root");
            if parent == root {
                break;
            }
            current = parent;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn add_tree_edge_preserves_java_direction() {
        let mut arena = BlossomArena::default();
        let n1 = arena.add_node(0);
        let n2 = arena.add_node(1);
        let t1 = arena.add_tree(n1);
        let t2 = arena.add_tree(n2);
        let edge = arena.add_tree_edge(t1, t2);
        let direction = arena.trees[t2].current_direction;
        assert_eq!(arena.tree_edges[edge].head[direction], Some(t2));
        assert_eq!(arena.tree_edges[edge].head[1 - direction], Some(t1));
    }

    #[test]
    fn add_remove_and_move_incident_edges() {
        let mut arena = BlossomArena::default();
        let n1 = arena.add_node(0);
        let n2 = arena.add_node(1);
        let n3 = arena.add_node(2);
        let e12 = arena.add_edge(n1, n2, 0.0, 0);
        let e13 = arena.add_edge(n1, n3, 0.0, 1);
        let e23 = arena.add_edge(n2, n3, 0.0, 2);

        arena.move_edge_tail(e12, n2, n3);
        assert_eq!(arena.edge_opposite(e12, n1), Some(n3));
        assert_eq!(edge_set(&arena, n1), HashSet::from([e12, e13]));
        assert_eq!(edge_set(&arena, n2), HashSet::from([e23]));
        assert_eq!(edge_set(&arena, n3), HashSet::from([e12, e13, e23]));

        arena.move_edge_tail(e23, n2, n1);
        assert_eq!(edge_set(&arena, n1), HashSet::from([e12, e13, e23]));
        assert!(edge_set(&arena, n2).is_empty());
    }

    #[test]
    fn incident_iterator_reports_direction() {
        let mut arena = BlossomArena::default();
        let from = arena.add_node(0);
        let to = arena.add_node(1);
        let edge = arena.add_edge(from, to, 0.0, 0);
        assert_eq!(arena.incident_edges(from), vec![(edge, 0)]);
        assert_eq!(arena.incident_edges(to), vec![(edge, 1)]);
        assert_eq!(arena.edges[edge].head[0], Some(to));
        assert_eq!(arena.edges[edge].head[1], Some(from));
    }

    #[test]
    fn tree_edge_lists_and_heap_orientation() {
        let mut arena = BlossomArena::default();
        let nodes: Vec<_> = (0..5).map(|i| arena.add_node(i)).collect();
        let trees: Vec<_> = nodes.iter().map(|&n| arena.add_tree(n)).collect();
        let e1 = arena.add_tree_edge(trees[0], trees[1]);
        let e2 = arena.add_tree_edge(trees[0], trees[2]);
        let e3 = arena.add_tree_edge(trees[3], trees[0]);
        let e4 = arena.add_tree_edge(trees[4], trees[0]);

        assert_eq!(
            arena
                .tree_edges(trees[0])
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([(e1, 0), (e2, 0), (e3, 1), (e4, 1)])
        );
        assert!(!std::ptr::eq(
            arena.tree_edge_minus_plus_heap(e1, 0),
            arena.tree_edge_plus_minus_heap(e1, 0)
        ));
        assert!(std::ptr::eq(
            arena.tree_edge_plus_minus_heap(e1, 0),
            arena.tree_edge_minus_plus_heap(e1, 1)
        ));

        arena.remove_tree_edge(e1);
        assert!(!arena.tree_edges(trees[0]).iter().any(|&(e, _)| e == e1));
        assert!(!arena.tree_edges(trees[1]).iter().any(|&(e, _)| e == e1));
    }

    #[test]
    fn minus_blossom_is_inserted_in_tree_heap() {
        let mut arena = BlossomArena::default();
        let root = arena.add_node(0);
        let blossom = arena.add_node(1);
        let tree = arena.add_tree(root);
        arena.nodes[blossom].label = Label::Minus;
        arena.nodes[blossom].is_outer = true;
        arena.nodes[blossom].is_blossom = true;
        arena.add_minus_blossom(tree, blossom);
        assert!(arena.nodes[blossom].handle.is_some());
        assert_eq!(
            arena.trees[tree]
                .minus_blossoms
                .peek_entry()
                .map(|(_, _, value)| *value),
            Some(blossom)
        );
    }

    fn edge_set(arena: &BlossomArena, node: NodeId) -> HashSet<EdgeId> {
        arena
            .incident_edges(node)
            .into_iter()
            .map(|(edge, _)| edge)
            .collect()
    }
}
