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

use crate::error::Result;
use crate::graph::{Graph, GraphExt};
use indexmap::IndexSet;
use std::collections::HashSet;
use std::hash::Hash;

/// `MatchingAlgorithm.DEFAULT_EPSILON`.
pub const DEFAULT_EPSILON: f64 = 1e-9;

/// `MatchingAlgorithm.Matching` / `MatchingImpl`: a set of edges, no two sharing an endpoint, with
/// its total weight. It remembers which vertices are matched and how many vertices the graph has,
/// so [`is_matched`](Self::is_matched) and [`is_perfect`](Self::is_perfect) need no graph.
#[derive(Debug, Clone)]
pub struct Matching<V, E> {
    edges: IndexSet<E>,
    weight: f64,
    matched_vertices: HashSet<V>,
    vertex_count: usize,
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> Matching<V, E> {
    pub fn new<G: Graph<Vertex = V, Edge = E>>(graph: &G, edges: IndexSet<E>, weight: f64) -> Self {
        let mut matched_vertices = HashSet::with_capacity(2 * edges.len());
        for e in &edges {
            let (s, t) = graph.edge_endpoints(e).expect("matching edge of the graph");
            matched_vertices.insert(s.clone());
            matched_vertices.insert(t.clone());
        }
        Matching {
            edges,
            weight,
            matched_vertices,
            vertex_count: graph.vertex_count(),
        }
    }

    /// The edges of the matching.
    pub fn edges(&self) -> &IndexSet<E> {
        &self.edges
    }

    /// The total weight of the matching.
    pub fn weight(&self) -> f64 {
        self.weight
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn iter(&self) -> indexmap::set::Iter<'_, E> {
        self.edges.iter()
    }

    /// Whether `v` is an endpoint of an edge of the matching.
    pub fn is_matched(&self, v: &V) -> bool {
        self.matched_vertices.contains(v)
    }

    /// Whether the matching covers every vertex (`edges = |V| / 2`).
    pub fn is_perfect(&self) -> bool {
        self.edges.len() as f64 == self.vertex_count as f64 / 2.0
    }
}

impl<'a, V, E> IntoIterator for &'a Matching<V, E> {
    type Item = &'a E;
    type IntoIter = indexmap::set::Iter<'a, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.edges.iter()
    }
}

/// Allows to derive a [matching](http://en.wikipedia.org/wiki/Matching_(graph_theory)) of
/// a given graph.
///
/// # Rust notes
///
/// `MatchingAlgorithm`.
pub trait MatchingAlgorithm<V, E> {
    /// Computes a matching.
    fn get_matching(&mut self) -> Result<Matching<V, E>>;
}
