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

//! Shared helpers mirroring `TestUtil`, `GraphTestsUtils` and `TestEdge` from the Java tests.
#![allow(dead_code)]

use jgrapht::graph::{Direction, GraphExt};
use jgrapht::*;
use std::collections::HashMap;
use std::hash::Hash;

/// Structural equality like `TestEdge`-based `assertEquals(graph1, graph2)`: same vertex set and
/// same multiset of `(source, target, weight)` edges (endpoints unordered when undirected).
pub fn same_graph<A, B>(a: &A, b: &B) -> bool
where
    A: Graph,
    B: Graph<Vertex = A::Vertex>,
{
    if a.graph_type().directed != b.graph_type().directed || a.vertex_count() != b.vertex_count() {
        return false;
    }
    if !a.vertices().all(|v| b.contains_vertex(v)) || a.edge_count() != b.edge_count() {
        return false;
    }
    fn key<G: Graph>(g: &G, e: &G::Edge) -> (G::Vertex, G::Vertex, u64) {
        let (s, t) = g.edge_endpoints(e).unwrap();
        (s.clone(), t.clone(), g.edge_weight(e).unwrap().to_bits())
    }
    let directed = a.graph_type().directed;
    let norm = |(s, t, w): (A::Vertex, A::Vertex, u64), idx: &HashMap<A::Vertex, usize>| {
        if directed || idx[&s] <= idx[&t] {
            (s, t, w)
        } else {
            (t, s, w)
        }
    };
    let idx: HashMap<A::Vertex, usize> = a
        .vertices()
        .cloned()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    let mut counts: HashMap<(A::Vertex, A::Vertex, u64), i64> = HashMap::new();
    for e in a.edges() {
        *counts.entry(norm(key(a, e), &idx)).or_default() += 1;
    }
    for e in b.edges() {
        *counts.entry(norm(key(b, e), &idx)).or_default() -= 1;
    }
    counts.values().all(|&c| c == 0)
}

/// Adds the edges (creating vertices) to a graph with `i32` vertices; a third column is the weight.
pub fn construct_graph<D: Direction>(g: &mut BaseGraph<i32, DefaultEdge, D>, edges: &[&[i32]]) {
    let weighted = edges.first().is_some_and(|e| e.len() > 2);
    for e in edges {
        g.add_vertex(e[0]);
        g.add_vertex(e[1]);
        let id = g.add_edge(&e[0], &e[1]).unwrap().expect("edge added");
        if weighted {
            g.set_edge_weight(&id, e[2] as f64).unwrap();
        }
    }
}

pub fn simple_graph_i32() -> UGraph<i32> {
    UGraph::<i32>::simple()
}
pub fn pseudograph_i32() -> UGraph<i32> {
    UGraph::<i32>::pseudo()
}

/// Asserts two collections hold the same elements regardless of order.
pub fn assert_same_elements<T: Eq + Hash + std::fmt::Debug>(
    a: impl IntoIterator<Item = T>,
    b: impl IntoIterator<Item = T>,
) {
    let (a, b): (std::collections::HashSet<T>, std::collections::HashSet<T>) =
        (a.into_iter().collect(), b.into_iter().collect());
    assert_eq!(a, b);
}

#[allow(unused)]
fn _unused<G: Graph>(g: &G) -> usize {
    g.vertex_count() + g.edge_count() + g.vertices().count()
}

pub mod fixtures;

pub use jgrapht::graph::GraphExt as _;
