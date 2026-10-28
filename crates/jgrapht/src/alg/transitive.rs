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

//! `TransitiveClosure` and `TransitiveReduction` (JGraphT `org.jgrapht.alg`).

use crate::error::{GraphError, Result};
use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use crate::graph::{DirectedExt, DirectedGraph, Graph, GraphExt, GraphMut, props};
use crate::traverse::TopologicalOrderIterator;
use indexmap::IndexSet;
use std::borrow::Borrow;
use std::collections::VecDeque;

/// Constructs the transitive closure of the input graph.
///
/// # Rust notes
///
/// Constructs the transitive closure of the input graph.
#[derive(Debug, Clone, Copy)]
pub struct TransitiveClosure;

impl TransitiveClosure {
    /// The singleton instance.
    pub const INSTANCE: TransitiveClosure = TransitiveClosure;

    /// Computes the transitive closure of a simple directed graph, in place.
    ///
    /// Every iteration of the outer loop adds an edge between vertices that had a path of length
    /// two; at most `floor(log2 |V|) + 1` iterations are needed, and the loop stops earlier if
    /// nothing changes. Fails if the graph is not directed or allows loops or multiple edges.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn close_simple_directed_graph<G>(&self, graph: &mut G) -> Result<()>
    where
        G: DirectedGraph + GraphMut + OutgoingGraph,
    {
        props::require_directed(graph)?;
        let t = graph.graph_type();
        if t.self_loops || t.multiple_edges {
            return Err(GraphError::InvalidGraph(
                crate::GraphProperty::SimpleDirected,
            ));
        }
        let vertex_set: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let mut new_edge_targets: IndexSet<G::Vertex> = IndexSet::new();

        let bound = compute_binary_log(vertex_set.len());
        let mut done = false;
        let mut i = 0;
        while !done && i < bound {
            done = true;
            for v1 in &vertex_set {
                new_edge_targets.clear();

                for v1_out_edge in graph.out_edges_of(v1) {
                    let v2 = graph.edge_target(v1_out_edge).expect("edge target");
                    for v2_out_edge in graph.out_edges_of(v2) {
                        let v3 = graph.edge_target(v2_out_edge).expect("edge target");

                        if v1 == v3 {
                            // Its a simple graph, so no self loops.
                            continue;
                        }
                        if graph.get_edge(v1, v3).is_some() {
                            // There is already an edge from v1 ---> v3, skip.
                            continue;
                        }
                        new_edge_targets.insert(v3.clone());
                        done = false;
                    }
                }

                for v3 in &new_edge_targets {
                    graph.add_edge(v1, v3)?;
                }
            }
            i += 1;
        }
        Ok(())
    }

    /// Computes the transitive closure of a directed acyclic graph in `O(nm)`, in place.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn close_directed_acyclic_graph<G>(&self, graph: &mut G) -> Result<()>
    where
        G: DirectedGraph + GraphMut + IncidenceGraph,
    {
        let mut ordered_vertices: VecDeque<G::Vertex> =
            VecDeque::with_capacity(graph.vertex_count());
        for v in TopologicalOrderIterator::new(&*graph)? {
            ordered_vertices.push_front(v?.clone());
        }

        for vertex in &ordered_vertices {
            for successor in graph.successor_list_of(vertex) {
                for closure_vertex in graph.successor_list_of(&successor) {
                    graph.add_edge(vertex, &closure_vertex)?;
                }
            }
        }
        Ok(())
    }
}

/// `floor(log2(n)) + 1`.
fn compute_binary_log(mut n: usize) -> usize {
    let mut result = 0;
    while n > 0 {
        n >>= 1;
        result += 1;
    }
    result
}

/// A fixed-size set of non-negative integers (`java.util.BitSet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    /// A set that can hold `n` bits, all clear.
    pub fn new(n: usize) -> Self {
        Self {
            words: vec![0; n.div_ceil(64)],
        }
    }

    /// Returns the get.
    pub fn get(&self, i: usize) -> bool {
        self.words
            .get(i / 64)
            .is_some_and(|w| w & (1 << (i % 64)) != 0)
    }

    /// Updates the state for `set`.
    pub fn set(&mut self, i: usize, value: bool) {
        if value {
            self.words[i / 64] |= 1 << (i % 64);
        } else {
            self.words[i / 64] &= !(1 << (i % 64));
        }
    }

    /// `this |= other`.
    pub fn or(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            *a |= b;
        }
    }

    /// `this &= !other`.
    pub fn and_not(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            *a &= !b;
        }
    }

    /// The index of the first set bit at or after `from`.
    pub fn next_set_bit(&self, from: usize) -> Option<usize> {
        let mut wi = from / 64;
        if wi >= self.words.len() {
            return None;
        }
        let mut word = self.words[wi] & (!0u64 << (from % 64));
        loop {
            if word != 0 {
                return Some(wi * 64 + word.trailing_zeros() as usize);
            }
            wi += 1;
            if wi >= self.words.len() {
                return None;
            }
            word = self.words[wi];
        }
    }
}

/// An implementation of Harry Hsu's
/// [transitive reduction algorithm](https://en.wikipedia.org/wiki/Transitive_reduction).
///
/// cf. [Harry Hsu. "An algorithm for finding a minimal equivalent graph of a digraph.", Journal of the ACM, 22(1):11-16, January 1975.](http://projects.csail.mit.edu/jacm/References/hsu1975:11.html)
///
/// This is a port from a python example by Michael Clerx, posted as an answer to a question about
/// [transitive reduction algorithm pseudocode](http://stackoverflow.com/questions/1690953/transitive-reduction-algorithm-pseudocode) on [Stack Overflow](http://stackoverflow.com)
///
/// # Rust notes
///
/// Harry Hsu's transitive reduction algorithm (JACM 22(1), 1975).
#[derive(Debug, Clone, Copy)]
pub struct TransitiveReduction;

impl TransitiveReduction {
    /// The singleton instance.
    pub const INSTANCE: TransitiveReduction = TransitiveReduction;

    /// Transforms `matrix` into a path matrix.
    ///
    /// Meant as a private method; public for the unit tests.
    #[doc(hidden)]
    pub fn transform_to_path_matrix(matrix: &mut [BitSet]) {
        for i in 0..matrix.len() {
            for j in 0..matrix.len() {
                if i == j {
                    continue;
                }
                if matrix[j].get(i) {
                    let row = matrix[i].clone();
                    matrix[j].or(&row);
                }
            }
        }
    }

    /// Transforms a path matrix into a transitively reduced matrix.
    ///
    /// Meant as a private method; public for the unit tests.
    #[doc(hidden)]
    pub fn transitive_reduction(path_matrix: &mut [BitSet]) {
        for j in 0..path_matrix.len() {
            for i in 0..path_matrix.len() {
                if path_matrix[i].get(j) {
                    let row = path_matrix[j].clone();
                    path_matrix[i].and_not(&row);
                }
            }
        }
    }

    /// Removes all transitive edges from the directed graph, in place.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn reduce<G: Graph + GraphMut>(&self, directed_graph: &mut G) -> Result<()> {
        props::require_directed(directed_graph)?;

        let vertices: Vec<G::Vertex> = directed_graph.vertices_cloned().collect();
        let n = vertices.len();
        let index_of = |v: &G::Vertex| vertices.iter().position(|x| x == v).expect("vertex");

        let mut original_matrix: Vec<BitSet> = (0..n).map(|_| BitSet::new(n)).collect();
        for edge in directed_graph.edges() {
            let edge = edge.borrow();
            let (v1, v2) = directed_graph.edge_endpoints(edge).expect("edge endpoints");
            original_matrix[index_of(v1)].set(index_of(v2), true);
        }

        // Keep a copy of the original adjacency, so the final removal loop can skip pairs that
        // were never edges in the first place.
        let original_matrix_copy = original_matrix.clone();

        let mut matrix = original_matrix;
        Self::transform_to_path_matrix(&mut matrix);
        Self::transitive_reduction(&mut matrix);

        // Remove edges that existed in the original graph but were eliminated by the reduction.
        for i in 0..n {
            let mut next = original_matrix_copy[i].next_set_bit(0);
            while let Some(j) = next {
                if !matrix[i].get(j) {
                    let e = directed_graph
                        .get_edge(&vertices[i], &vertices[j])
                        .cloned()
                        .expect("original edge");
                    directed_graph.remove_edge(&e);
                }
                next = original_matrix_copy[i].next_set_bit(j + 1);
            }
        }
        Ok(())
    }
}
