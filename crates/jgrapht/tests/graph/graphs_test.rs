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

//! Port of `org.jgrapht.GraphsTest`.

use crate::common::same_graph;
use jgrapht::graph::{DirectedExt, remove_vertex_and_preserve_connectivity};
use jgrapht::*;
use std::collections::HashSet;

fn abcd() -> DiGraph<&'static str> {
    // A -> B, B -> C, B -> D
    let mut g = DiGraph::<&str>::with_loops();
    g.add_vertices(["A", "B", "C", "D"]);
    g.add_edge(&"A", &"B").unwrap();
    g.add_edge(&"B", &"C").unwrap();
    g.add_edge(&"B", &"D").unwrap();
    g
}

#[test]
fn remove_vertex_vertex_not_found() {
    let mut graph = abcd();
    let expected = abcd();
    assert_eq!(
        remove_vertex_and_preserve_connectivity(&mut graph, &"X"),
        Ok(false)
    );
    assert!(same_graph(&expected, &graph));
}

#[test]
fn remove_vertex_00() {
    let mut graph = abcd();
    let mut expected = DiGraph::<&str>::with_loops();
    expected.add_vertices(["A", "C", "D"]);
    expected.add_edge(&"A", &"C").unwrap();
    expected.add_edge(&"A", &"D").unwrap();
    assert_eq!(
        remove_vertex_and_preserve_connectivity(&mut graph, &"B"),
        Ok(true)
    );
    assert!(same_graph(&expected, &graph));
}

#[test]
fn remove_vertex_01() {
    let mut graph = DiGraph::<&str>::with_loops();
    graph.add_vertices(["A", "B"]);
    graph.add_edge(&"A", &"B").unwrap();
    let mut expected = DiGraph::<&str>::with_loops();
    expected.add_vertex("B");
    assert_eq!(
        remove_vertex_and_preserve_connectivity(&mut graph, &"A"),
        Ok(true)
    );
    assert!(same_graph(&expected, &graph));
}

#[test]
fn remove_vertex_02() {
    let mut graph = DiGraph::<&str>::with_loops();
    graph.add_vertices(["A", "B"]);
    graph.add_edge(&"A", &"B").unwrap();
    let mut expected = DiGraph::<&str>::with_loops();
    expected.add_vertex("A");
    assert_eq!(
        remove_vertex_and_preserve_connectivity(&mut graph, &"B"),
        Ok(true)
    );
    assert!(same_graph(&expected, &graph));
}

#[test]
fn add_outgoing_edges() {
    let mut graph = DiGraph::<&str>::with_loops();
    graph.add_vertex("B");
    let mut expected = DiGraph::<&str>::with_loops();
    expected.add_vertices(["A", "B", "C"]);
    expected.add_edge(&"A", &"B").unwrap();
    expected.add_edge(&"A", &"C").unwrap();
    graph.add_outgoing_edges(&"A", ["B", "C"]).unwrap();
    assert!(same_graph(&expected, &graph));
}

#[test]
fn add_incoming_edges() {
    let mut graph = DiGraph::<&str>::with_loops();
    graph.add_vertex("B");
    let mut expected = DiGraph::<&str>::with_loops();
    expected.add_vertices(["A", "B", "C"]);
    expected.add_edge(&"B", &"A").unwrap();
    expected.add_edge(&"C", &"A").unwrap();
    graph.add_incoming_edges(&"A", ["B", "C"]).unwrap();
    assert!(same_graph(&expected, &graph));
}

#[test]
fn vertex_has_children_b() {
    assert!(abcd().vertex_has_successors(&"B"));
}

#[test]
fn vertex_has_children_c() {
    assert!(!abcd().vertex_has_successors(&"C"));
}

#[test]
fn vertex_has_parents_b() {
    assert!(abcd().vertex_has_predecessors(&"B"));
}

#[test]
fn vertex_has_parents_a() {
    assert!(!abcd().vertex_has_predecessors(&"A"));
}

#[test]
fn test_neighbor_set_of() {
    let mut graph = UGraph::<i32>::with_loops();
    graph.add_vertices([1, 2, 3, 4]);
    graph.add_edge(&1, &2).unwrap();
    graph.add_edge(&2, &3).unwrap();
    graph.add_edge(&3, &4).unwrap();
    graph.add_edge(&1, &4).unwrap();
    assert_eq!(graph.neighbor_set_of(&1), HashSet::from([2, 4]));
}
