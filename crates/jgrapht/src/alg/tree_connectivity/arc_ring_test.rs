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

//! Invariant tests adapted from Java's `DoublyLinkedListTest` for the private arc-ring arena.
//!
//! Java collection-interface and fail-fast iterator checks do not have Rust counterparts; these
//! tests cover the node API, circular links, deque operations, splicing, inversion and traversal.

use super::arc_ring::{ArcRingArena, RingId, RingNode};

fn values<T: Clone>(arena: &ArcRingArena<T>, list: RingId) -> Vec<T> {
    arena
        .node_ids(list)
        .into_iter()
        .map(|node| arena.value(node).clone())
        .collect()
}

fn strings(arena: &mut ArcRingArena<String>, values: &[&str]) -> RingId {
    let list = arena.create_list();
    for value in values {
        arena.add_element_last(list, (*value).to_owned());
    }
    list
}

fn assert_links<T>(arena: &ArcRingArena<T>, list: RingId) {
    let nodes = arena.node_ids(list);
    assert_eq!(nodes.len(), arena.len(list));
    for (index, &node) in nodes.iter().enumerate() {
        assert_eq!(arena.list_of(node), Some(list));
        assert_eq!(arena.next(node), Some(nodes[(index + 1) % nodes.len()]));
        assert_eq!(
            arena.previous(node),
            Some(nodes[(index + nodes.len() - 1) % nodes.len()])
        );
    }
}

#[test]
fn empty_size_and_clear() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "c"]);
    assert!(!arena.is_empty(list));
    assert_eq!(arena.len(list), 3);
    arena.clear(list);
    assert!(arena.is_empty(list));
    assert_eq!(arena.first(list), None);
    assert_eq!(arena.last(list), None);
}

#[test]
fn add_node_first_last_and_at_index() {
    let mut arena = ArcRingArena::new();
    let list = arena.create_list();
    let a = arena.allocate_node("a");
    let b = arena.allocate_node("b");
    let c = arena.allocate_node("c");
    arena.add_node_first(list, b).unwrap();
    arena.add_node_first(list, a).unwrap();
    arena.add_node_last(list, c).unwrap();
    let x = arena.allocate_node("x");
    arena.add_node(list, 2, x).unwrap();
    assert_eq!(values(&arena, list), ["a", "b", "x", "c"]);
    assert_links(&arena, list);
}

#[test]
fn a_node_can_belong_to_only_one_list() {
    let mut arena = ArcRingArena::new();
    let first = arena.create_list();
    let second = arena.create_list();
    let node = arena.allocate_node(1);
    arena.add_node_last(first, node).unwrap();
    assert!(arena.add_node_last(first, node).is_err());
    assert!(arena.add_node_last(second, node).is_err());
    assert!(!arena.remove_node(second, node));
    assert!(arena.remove_node(first, node));
    arena.add_node_last(second, node).unwrap();
}

#[test]
fn add_node_before_checks_the_successor_list() {
    let mut arena = ArcRingArena::new();
    let first = arena.create_list();
    let second = arena.create_list();
    let successor = arena.add_element_last(first, 2);
    let other = arena.add_element_last(second, 3);
    let node = arena.allocate_node(1);
    assert!(arena.add_node_before(first, node, other).is_err());
    arena.add_node_before(first, node, successor).unwrap();
    assert_eq!(values(&arena, first), [1, 2]);
}

#[test]
fn get_first_last_and_indexed_nodes() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "c", "d"]);
    assert_eq!(arena.value(arena.first(list).unwrap()), "a");
    assert_eq!(arena.value(arena.last(list).unwrap()), "d");
    assert_eq!(arena.value(arena.node(list, 2).unwrap()), "c");
    assert_eq!(arena.node(list, 4), None);
}

#[test]
fn index_and_contains_are_by_node_identity() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["same", "same", "other"]);
    let nodes = arena.node_ids(list);
    assert_eq!(arena.index_of_node(list, nodes[1]), Some(1));
    assert!(arena.contains_node(list, nodes[1]));
    let free = arena.allocate_node("same".to_owned());
    assert_eq!(arena.index_of_node(list, free), None);
}

#[test]
fn remove_nodes_from_front_middle_and_back() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "c", "d"]);
    let nodes = arena.node_ids(list);
    assert!(arena.remove_node(list, nodes[0]));
    assert!(arena.remove_node(list, nodes[2]));
    assert!(arena.remove_node(list, nodes[3]));
    assert_eq!(values(&arena, list), ["b"]);
    assert_eq!(arena.list_of(nodes[0]), None);
    assert_eq!(arena.next(nodes[0]), None);
    assert_links(&arena, list);
}

#[test]
fn node_of_and_last_node_of() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "a"]);
    let nodes = arena.node_ids(list);
    assert_eq!(arena.node_of(list, &"a".to_owned()), Some(nodes[0]));
    assert_eq!(arena.last_node_of(list, &"a".to_owned()), Some(nodes[2]));
    assert_eq!(arena.node_of(list, &"z".to_owned()), None);
}

#[test]
fn add_elements_at_both_ends_and_before_node() {
    let mut arena = ArcRingArena::new();
    let list = arena.create_list();
    let c = arena.add_element_last(list, "c");
    arena.add_element_first(list, "a");
    arena.add_element_before(list, c, "b").unwrap();
    assert_eq!(values(&arena, list), ["a", "b", "c"]);
    assert_links(&arena, list);
}

#[test]
fn remove_first_and_last_occurrences() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "a", "c"]);
    assert!(arena.remove_first_occurrence(list, &"a".to_owned()));
    assert!(arena.remove_last_occurrence(list, &"a".to_owned()));
    assert!(!arena.remove_first_occurrence(list, &"z".to_owned()));
    assert_eq!(values(&arena, list), ["b", "c"]);
}

#[test]
fn invert_reverses_content_and_links() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "c", "d"]);
    arena.invert(list);
    assert_eq!(values(&arena, list), ["d", "c", "b", "a"]);
    assert_links(&arena, list);
}

#[test]
fn move_from_splices_without_changing_node_handles() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "d"]);
    let moved = strings(&mut arena, &["b", "c"]);
    let moved_nodes = arena.node_ids(moved);
    arena.move_from(list, 1, moved).unwrap();
    assert_eq!(values(&arena, list), ["a", "b", "c", "d"]);
    assert!(arena.is_empty(moved));
    assert!(
        moved_nodes
            .into_iter()
            .all(|node| arena.list_of(node) == Some(list))
    );
    assert_links(&arena, list);
}

#[test]
fn append_and_prepend() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["c"]);
    let prefix = strings(&mut arena, &["a", "b"]);
    let suffix = strings(&mut arena, &["d", "e"]);
    arena.prepend(list, prefix).unwrap();
    arena.append(list, suffix).unwrap();
    assert_eq!(values(&arena, list), ["a", "b", "c", "d", "e"]);
    assert_links(&arena, list);
}

#[test]
fn forward_reverse_and_circular_traversal() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "c", "d"]);
    let nodes = arena.node_ids(list);
    let reverse: Vec<_> = arena
        .reverse_node_ids(list)
        .into_iter()
        .map(|node| arena.value(node).as_str())
        .collect();
    assert_eq!(reverse, ["d", "c", "b", "a"]);
    let circular: Vec<_> = arena
        .circular_node_ids(list, nodes[2])
        .unwrap()
        .into_iter()
        .map(|node| arena.value(node).as_str())
        .collect();
    assert_eq!(circular, ["c", "d", "a", "b"]);
    let reverse_circular: Vec<_> = arena
        .reverse_circular_node_ids(list, nodes[1])
        .unwrap()
        .into_iter()
        .map(|node| arena.value(node).as_str())
        .collect();
    assert_eq!(reverse_circular, ["b", "a", "d", "c"]);
}

#[test]
fn node_value_can_be_replaced_like_list_iterator_set() {
    let mut arena = ArcRingArena::new();
    let list = strings(&mut arena, &["a", "b", "c"]);
    let middle: RingNode = arena.node(list, 1).unwrap();
    *arena.value_mut(middle) = "x".to_owned();
    assert_eq!(values(&arena, list), ["a", "x", "c"]);
}
