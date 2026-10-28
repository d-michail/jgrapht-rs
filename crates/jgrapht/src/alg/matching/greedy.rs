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

use super::base::{DEFAULT_EPSILON, Matching, MatchingAlgorithm};
use crate::error::Result;
use crate::graph::IncidenceGraph;
use crate::graph::UndirectedGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::ToleranceDoubleComparator;
use indexmap::IndexSet;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

fn ordering(c: i32) -> Ordering {
    c.cmp(&0)
}

/// The greedy algorithm for computing a maximum cardinality matching. The algorithm can run in two
/// modes: sorted or unsorted. When unsorted, the matching is obtained by iterating through the edges
/// and adding an edge if it doesn't conflict with the edges already in the matching. When sorted,
/// the edges are first sorted by the sum of degrees of their endpoints. After that, the algorithm
/// proceeds in the same manner. Running this algorithm in sorted mode can sometimes produce better
/// results, albeit at the cost of some additional computational overhead.
///
/// Independent of the mode, the resulting matching is maximal, and is therefore guaranteed to
/// contain at least half of the edges that a maximum cardinality matching has ($\frac{1}{2}$
/// approximation). Runtime complexity: $O(m)$ when the edges are not sorted, $O(m + m \log n)$
/// otherwise, where $n$ is the number of vertices, and $m$ the number of edges.
///
/// # Rust notes
///
/// `GreedyMaximumCardinalityMatching`: a maximal matching, found greedily. With `sort`, the edges
/// are taken in increasing order of the total degree of their endpoints.
pub struct GreedyMaximumCardinalityMatching<'g, G: Graph> {
    graph: &'g G,
    sort: bool,
}

impl<'g, G: UndirectedGraph> GreedyMaximumCardinalityMatching<'g, G> {
    /// Fails if the graph is not undirected.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(graph: &'g G, sort: bool) -> Result<Self> {
        Ok(GreedyMaximumCardinalityMatching { graph, sort })
    }
}

impl<G: IncidenceGraph> MatchingAlgorithm<G::Vertex, G::Edge>
    for GreedyMaximumCardinalityMatching<'_, G>
{
    /// Get a matching that is a $\frac{1}{2}$-approximation of the maximum cardinality matching.
    fn matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        let mut matched: HashSet<G::Vertex> = HashSet::new();
        let mut edges: IndexSet<G::Edge> = IndexSet::new();
        let mut cost = 0.0;
        if self.sort {
            // sort edges in increasing order of the total degree of their endpoints
            let degrees: HashMap<G::Vertex, usize> = graph
                .vertices_cloned()
                .map(|v| {
                    let degree = graph.degree_of(&v);
                    (v, degree)
                })
                .collect();
            let degree = |e: &G::Edge| {
                let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
                degrees[s] + degrees[t]
            };
            let mut all_edges: Vec<G::Edge> = graph.edges_cloned().collect();
            all_edges.sort_by_key(|e| degree(e));
            for e in &all_edges {
                let (v, w) = graph
                    .edge_endpoints(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?;
                if v != w && !matched.contains(v) && !matched.contains(w) {
                    edges.insert(e.clone());
                    matched.insert(v.clone());
                    matched.insert(w.clone());
                    cost += graph.weight_of(e);
                }
            }
        } else {
            for v in graph.vertices() {
                let v = v.borrow();
                if matched.contains(v) {
                    continue;
                }
                for e in graph.edges_of(v) {
                    let w = graph
                        .opposite_vertex(e, v)
                        .ok_or(crate::GraphError::EdgeNotFound)?;
                    if v != w && !matched.contains(w) {
                        edges.insert(e.clone());
                        matched.insert(v.clone());
                        matched.insert(w.clone());
                        cost += graph.weight_of(e);
                        break;
                    }
                }
            }
        }
        Ok(Matching::new(graph, edges, cost))
    }
}

/// The greedy algorithm for computing a maximum weight matching in an arbitrary graph. The algorithm
/// runs in $O(m + m \log n)$ where $n$ is the number of vertices and $m$ is the number of edges of
/// the graph. This implementation accepts directed and undirected graphs which may contain
/// self-loops and multiple (parallel) edges. There is no assumption on the edge weights, i.e. they
/// can also be negative or zero.
///
/// This algorithm can be run in two modes: with and without edge cost normalization. Without
/// normalization, the algorithm first orders the edge set in non-increasing order of weights and
/// then greedily constructs a maximal cardinality matching out of the edges with positive weight. A
/// maximal cardinality matching (not to be confused with maximum cardinality) is a matching that
/// cannot be increased in cardinality without removing an edge first. The resulting matching is
/// guaranteed to be a $\frac{1}{2}$-Approximation.
/// With normalization, the edges are sorted in non-increasing order of their normalized costs
/// $\frac{c(u,v)}{d(u)+d(v)}$ instead, after which the algorithm proceeds in the same manner. Here,
/// $c(u,v)$ is the cost of edge $(u,v)$, and $d(u)$ resp $d(v)$ are the degrees of vertices $u$ resp
/// $v$. Running this algorithm in normalized mode often (but not always!) produces a better result
/// than running the algorithm without normalization. *Note however that the normalized version
/// does NOT produce a $\frac{1}{2}$-approximation*. See [this proof for details.](https://mathoverflow.net/questions/269526/is-greedy-matching-algorithm-with-normalized-edge-weights-a-2-approximation/269760#269760) The runtime complexity remains the same, independent of whether
/// normalization is used.
///
/// For more information about approximation algorithms for the maximum weight matching problem in
/// arbitrary graphs see:
///
/// - R. Preis, Linear Time $\frac{1}{2}$-Approximation Algorithm for Maximum Weighted Matching in
///   General Graphs. Symposium on Theoretical Aspects of Computer Science, 259-269, 1999.
/// - D.E. Drake, S. Hougardy, A Simple Approximation Algorithm for the Weighted Matching Problem,
///   Information Processing Letters 85, 211-213, 2003.
///
/// # Rust notes
///
/// `GreedyWeightedMatching`: edges of positive weight, heaviest first (optionally the weight is
/// divided by the total degree of the endpoints), added whenever both endpoints are free.
pub struct GreedyWeightedMatching<'g, G: Graph> {
    graph: &'g G,
    comparator: ToleranceDoubleComparator,
    normalize_edge_costs: bool,
}

impl<'g, G: Graph> GreedyWeightedMatching<'g, G> {
    /// Create and execute a new instance of the greedy maximum weight matching algorithm. Floating
    /// point values are compared using `DEFAULT_EPSILON` tolerance.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(graph: &'g G, normalize_edge_costs: bool) -> Result<Self> {
        Self::with_epsilon(graph, normalize_edge_costs, DEFAULT_EPSILON)
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_epsilon(graph: &'g G, normalize_edge_costs: bool, epsilon: f64) -> Result<Self> {
        Ok(GreedyWeightedMatching {
            graph,
            comparator: ToleranceDoubleComparator::new(epsilon)?,
            normalize_edge_costs,
        })
    }
}

impl<G: IncidenceGraph> MatchingAlgorithm<G::Vertex, G::Edge> for GreedyWeightedMatching<'_, G> {
    /// Get a matching that is a $\frac{1}{2}$-approximation of the maximum weighted matching.
    fn matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        let weight = |e: &G::Edge| graph.weight_of(e);
        // sort edges in non-increasing order of weight
        let mut all_edges: Vec<G::Edge> = graph.edges_cloned().collect();
        if self.normalize_edge_costs {
            let degrees: HashMap<G::Vertex, usize> = graph
                .vertices_cloned()
                .map(|v| {
                    let degree = graph.degree_of(&v);
                    (v, degree)
                })
                .collect();
            let degree = |e: &G::Edge| {
                let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
                (degrees[s] + degrees[t]) as f64
            };
            all_edges.sort_by(|e1, e2| {
                ordering(
                    self.comparator
                        .compare(weight(e2) / degree(e2), weight(e1) / degree(e1)),
                )
            });
        } else {
            all_edges.sort_by(|e1, e2| ordering(self.comparator.compare(weight(e2), weight(e1))));
        }

        let mut matching_weight = 0.0;
        let mut matching: IndexSet<G::Edge> = IndexSet::new();
        let mut matched_vertices: HashSet<G::Vertex> = HashSet::new();
        // find maximal matching
        for e in &all_edges {
            let edge_weight = weight(e);
            let (s, t) = graph
                .edge_endpoints(e)
                .ok_or(crate::GraphError::EdgeNotFound)?;
            if s != t
                && self.comparator.compare(edge_weight, 0.0) > 0
                && !matched_vertices.contains(s)
                && !matched_vertices.contains(t)
            {
                matching.insert(e.clone());
                matched_vertices.insert(s.clone());
                matched_vertices.insert(t.clone());
                matching_weight += edge_weight;
            }
        }
        Ok(Matching::new(graph, matching, matching_weight))
    }
}
