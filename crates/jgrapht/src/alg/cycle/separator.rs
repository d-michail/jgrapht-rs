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

use super::chordality::{ChordalityInspector, IterationOrder};
use crate::graph::IncidenceGraph;
use crate::graph::{GraphExt, UndirectedGraph};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Allows obtaining a mapping of all
/// [minimal vertex separators](https://en.wikipedia.org/wiki/Chordal_graph#Minimal_separators) of a graph to their multiplicities
///
/// In the context of this implementation following definitions are used:
///
/// - A set of vertices $S$ of a graph $G=(V, E)$ is called a *u-v separator*, if vertices $u$
///   and $v$ in the induced graph on vertices $V(G) - S$ are in different connected components.
/// - A set $S$ is called a *minimal u-v separator* if it is a u-v separator and no proper
///   subset of $S$ is a u-v separator.
/// - A set $S$ is called a *minimal vertex separator* if it is minimal u-v separator for some
///   vertices $u$ and $v$ of the graph $G$.
/// - A set of vertices $S$ is called a *minimal separator* if no proper subset of $S$ is a
///   separator of the graph $G$.
///
/// Let $\sigma = (v_1, v_2, \dots, v_n)$ be some perfect elimination order (peo) of the graph $G =
/// (V, E)$. The induced graph on vertices $(v_1, v_2, \dots, v_i)$ with respect to peo $\sigma$ is
/// denoted as $G_i$. The predecessors set of vertex $v$ with respect to peo $\sigma$ is denoted as
/// $N(v, \sigma)$. A set $B$ is called a *base set* with respect to $\sigma$, is there exist
/// some vertex $v$ with $t = \sigma(v)$ such that $N(v, \sigma) = B$ and B is not a maximal clique
/// in $G_{t-1}$. The vertices which satisfy conditions described above are called *dependent
/// vertices* with respect to $\sigma$. The cardinality of the set of dependent vertices is called
/// a multiplicity of the base set $B$. The multiplicity of a minimal vertex separator indicates the
/// number of different pairs of vertices separated by it.The definitions of a base set and a minimal
/// vertex separator in the context of chordal graphs are equivalent.
///
/// For more information on the topic see: Kumar, P. Sreenivasa & Madhavan, C. E. Veni. (1998).
/// [Minimal vertex separators of chordal graphs](https://www.sciencedirect.com/science/article/pii/S0166218X98001231?via%3Dihub). Discrete Applied Mathematics. 89. 155-168.
/// 10.1016/S0166-218X(98)00123-1.
///
/// The running time of the algorithm is $\mathcal{O}(\omega(G)(|V| + |E|))$, where $\omega(G)$ is
/// the size of a maximum clique of the graph $G$.
///
/// # Rust notes
///
/// `MinimalVertexSeparatorFinder`: the minimal vertex separators of a chordal graph
/// with their multiplicities, read off a perfect elimination order.
///
/// A separator is a set of vertices; it is represented by the vector of its members sorted by
/// their position in the perfect elimination order, so equal sets have equal vectors.
pub struct MinimalVertexSeparatorFinder<'g, G: UndirectedGraph> {
    graph: &'g G,
    chordality_inspector: ChordalityInspector<'g, G>,
    computed: bool,
    minimal_separators_with_multiplicities: Option<IndexMap<Vec<G::Vertex>, usize>>,
}

impl<'g, G: UndirectedGraph + IncidenceGraph> MinimalVertexSeparatorFinder<'g, G> {
    /// Creates new `MinimalVertexSeparatorFinder` instance. The
    /// `ChordalityInspector` used in this implementation uses the
    /// `org.jgrapht.traverse.MaximumCardinalityIterator` iterator
    pub fn new(graph: &'g G) -> Self {
        MinimalVertexSeparatorFinder {
            graph,
            chordality_inspector: ChordalityInspector::with_order(graph, IterationOrder::Mcs),
            computed: false,
            minimal_separators_with_multiplicities: None,
        }
    }

    /// The minimal separators, or `None` if the graph is not chordal.
    pub fn minimal_separators(&mut self) -> Option<Vec<&Vec<G::Vertex>>> {
        self.lazy_compute();
        self.minimal_separators_with_multiplicities
            .as_ref()
            .map(|m| m.keys().collect())
    }

    /// The minimal separators with the number of times each occurs, or `None` if the graph is not
    /// chordal.
    pub fn minimal_separators_with_multiplicities(
        &mut self,
    ) -> Option<&IndexMap<Vec<G::Vertex>, usize>> {
        self.lazy_compute();
        self.minimal_separators_with_multiplicities.as_ref()
    }

    fn lazy_compute(&mut self) {
        if self.computed {
            return;
        }
        self.computed = true;
        if !self.chordality_inspector.is_chordal() {
            return;
        }
        let order: Vec<G::Vertex> = self
            .chordality_inspector
            .perfect_elimination_order()
            .expect("chordal graph")
            .to_vec();
        let vertex_in_order: HashMap<&G::Vertex, usize> =
            order.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let mut result: IndexMap<Vec<G::Vertex>, usize> = IndexMap::new();
        let mut current: Vec<G::Vertex> = Vec::new();
        for vertex in order.iter().skip(1) {
            let previous_len = current.len();
            current = self.predecessors(&vertex_in_order, vertex);
            if current.len() <= previous_len {
                *result.entry(current.clone()).or_insert(0) += 1;
            }
        }
        self.minimal_separators_with_multiplicities = Some(result);
    }

    /// The neighbours earlier in the order, sorted by their position.
    fn predecessors(
        &self,
        vertex_in_order: &HashMap<&G::Vertex, usize>,
        vertex: &G::Vertex,
    ) -> Vec<G::Vertex> {
        let position = vertex_in_order[vertex];
        let mut predecessors: Vec<(usize, G::Vertex)> = Vec::new();
        for edge in self.graph.edges_of(vertex) {
            let opposite = self
                .graph
                .opposite_vertex(edge, vertex)
                .expect("edge of the graph");
            let dest = vertex_in_order[opposite];
            if dest < position && !predecessors.iter().any(|(d, _)| *d == dest) {
                predecessors.push((dest, opposite.clone()));
            }
        }
        predecessors.sort_by_key(|(d, _)| *d);
        predecessors.into_iter().map(|(_, v)| v).collect()
    }
}
