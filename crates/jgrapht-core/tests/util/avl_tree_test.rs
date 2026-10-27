//! Port of `org.jgrapht.util.AVLTreeTest` to the shared-arena Rust API.

use jgrapht::util::avl_tree::{AvlNodeId, AvlTree, AvlTreeId};

fn values(tree: &AvlTree<i32>, id: AvlTreeId) -> Vec<i32> {
    tree.node_ids(id)
        .into_iter()
        .map(|node| *tree.value(node))
        .collect()
}

fn diagnostic(tree: &AvlTree<i32>, id: AvlTreeId) {
    fn visit(tree: &AvlTree<i32>, node: AvlNodeId) -> (AvlNodeId, AvlNodeId, usize, usize) {
        let left = tree.left(node).map(|child| visit(tree, child));
        let right = tree.right(node).map(|child| visit(tree, child));
        let left_height = left.map_or(0, |v| v.2);
        let right_height = right.map_or(0, |v| v.2);
        let left_size = left.map_or(0, |v| v.3);
        let right_size = right.map_or(0, |v| v.3);
        assert_eq!(tree.height(node), left_height.max(right_height) + 1);
        assert_eq!(tree.subtree_size(node), left_size + right_size + 1);
        assert!(left_height.abs_diff(right_height) < 2);

        let min = if let Some(left) = left {
            let child = tree.left(node).unwrap();
            assert_eq!(tree.parent(child), Some(node));
            assert_eq!(tree.predecessor(node), Some(left.1));
            assert_eq!(tree.successor(left.1), Some(node));
            left.0
        } else {
            node
        };
        let max = if let Some(right) = right {
            let child = tree.right(node).unwrap();
            assert_eq!(tree.parent(child), Some(node));
            assert_eq!(tree.successor(node), Some(right.0));
            assert_eq!(tree.predecessor(right.0), Some(node));
            right.1
        } else {
            node
        };
        assert_eq!(tree.subtree_min(node), min);
        assert_eq!(tree.subtree_max(node), max);
        (min, max, tree.height(node), tree.subtree_size(node))
    }

    if let Some(root) = tree.root(id) {
        assert_eq!(tree.parent(root), None);
        let (_, _, _, size) = visit(tree, root);
        assert_eq!(size, tree.size(id));
    }
}

fn fill(tree: &mut AvlTree<i32>, id: AvlTreeId, from: i32, to: i32) -> Vec<AvlNodeId> {
    let middle = (from + to) / 2;
    let mut result = Vec::new();
    for value in (from..middle).rev() {
        result.insert(0, tree.add_min(id, value));
        diagnostic(tree, id);
    }
    for value in middle..to {
        result.push(tree.add_max(id, value));
        diagnostic(tree, id);
    }
    result
}

#[test]
fn test_empty() {
    let mut arena = AvlTree::<i32>::new();
    let tree = arena.create_tree();
    assert_eq!(arena.size(tree), 0);
    assert_eq!(arena.root(tree), None);
}

#[test]
fn test_one_node() {
    let mut arena = AvlTree::new();
    let tree = arena.create_tree();
    let node = arena.add_max(tree, 1);
    assert_eq!(*arena.value(node), 1);
    assert_eq!(arena.root(tree), Some(node));
    assert_eq!(arena.min(tree), Some(node));
    assert_eq!(arena.max(tree), Some(node));
    assert_eq!(arena.height(node), 1);
    assert_eq!(arena.size(tree), 1);
}

#[test]
fn test_add_max() {
    for count in 0..50 {
        let mut arena = AvlTree::new();
        let tree = arena.create_tree();
        for value in 0..count {
            arena.add_max(tree, value);
            diagnostic(&arena, tree);
        }
        assert_eq!(arena.size(tree), count as usize);
        assert_eq!(values(&arena, tree), (0..count).collect::<Vec<_>>());
    }
}

#[test]
fn test_add_min() {
    for count in 0..50 {
        let mut arena = AvlTree::new();
        let tree = arena.create_tree();
        for value in (0..count).rev() {
            arena.add_min(tree, value);
            diagnostic(&arena, tree);
        }
        assert_eq!(values(&arena, tree), (0..count).collect::<Vec<_>>());
    }
}

#[test]
fn test_merge_after_and_before() {
    for left_size in 0..50 {
        for right_size in 0..50 {
            let mut arena = AvlTree::new();
            let left = arena.create_tree();
            let right = arena.create_tree();
            fill(&mut arena, left, 0, left_size);
            fill(&mut arena, right, left_size, left_size + right_size);
            arena.merge_after(left, right);
            assert_eq!(
                values(&arena, left),
                (0..left_size + right_size).collect::<Vec<_>>()
            );
            assert!(arena.is_empty(right));
            diagnostic(&arena, left);

            let left2 = arena.create_tree();
            let right2 = arena.create_tree();
            fill(&mut arena, left2, 0, left_size);
            fill(&mut arena, right2, left_size, left_size + right_size);
            arena.merge_before(right2, left2);
            assert_eq!(
                values(&arena, right2),
                (0..left_size + right_size).collect::<Vec<_>>()
            );
            assert!(arena.is_empty(left2));
            diagnostic(&arena, right2);
        }
    }
}

#[test]
fn test_split_after() {
    for size in 1..50 {
        for split in 0..size {
            let mut arena = AvlTree::new();
            let tree = arena.create_tree();
            let nodes = fill(&mut arena, tree, 0, size);
            let right = arena.split_after(tree, nodes[split as usize]);
            assert_eq!(values(&arena, tree), (0..=split).collect::<Vec<_>>());
            assert_eq!(values(&arena, right), (split + 1..size).collect::<Vec<_>>());
            diagnostic(&arena, tree);
            diagnostic(&arena, right);
        }
    }
}

#[test]
fn test_split_before() {
    for size in 1..50 {
        for split in 0..size {
            let mut arena = AvlTree::new();
            let tree = arena.create_tree();
            let nodes = fill(&mut arena, tree, 0, size);
            let right = arena.split_before(tree, nodes[split as usize]);
            assert_eq!(values(&arena, tree), (0..split).collect::<Vec<_>>());
            assert_eq!(values(&arena, right), (split..size).collect::<Vec<_>>());
            diagnostic(&arena, tree);
            diagnostic(&arena, right);
        }
    }
}

#[test]
fn test_remove_and_iteration() {
    for size in 1..50 {
        let mut arena = AvlTree::new();
        let tree = arena.create_tree();
        let nodes = fill(&mut arena, tree, 0, size);
        assert_eq!(arena.node_ids(tree), nodes);
        let minimum = arena.remove_min(tree).unwrap();
        assert_eq!(*arena.value(minimum), 0);
        if size > 1 {
            let maximum = arena.remove_max(tree).unwrap();
            assert_eq!(*arena.value(maximum), size - 1);
        }
        diagnostic(&arena, tree);
    }
}

#[test]
fn worst_case_height_scaling_guard() {
    let mut arena = AvlTree::new();
    let tree = arena.create_tree();
    for value in 0..100_000 {
        arena.add_max(tree, value);
    }
    // The exact AVL bound is below 24 here. A randomized treap is not worst-case bounded and its
    // typical height at this size is substantially larger.
    assert!(arena.height(arena.root(tree).unwrap()) <= 24);
    assert_eq!(arena.size(tree), 100_000);
}
