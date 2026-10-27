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
use crate::graph::{Graph, GraphExt, props};
use crate::util::ToleranceDoubleComparator;
use indexmap::IndexSet;
use std::cmp::Ordering;
use std::collections::HashSet;

fn ordering(c: i32) -> Ordering {
    c.cmp(&0)
}

/// `GreedyMaximumCardinalityMatching`: a maximal matching, found greedily. With `sort`, the edges
/// are taken in increasing order of the total degree of their endpoints.
pub struct GreedyMaximumCardinalityMatching<'g, G: Graph> {
    graph: &'g G,
    sort: bool,
}

impl<'g, G: Graph> GreedyMaximumCardinalityMatching<'g, G> {
    /// Fails if the graph is not undirected.
    pub fn new(graph: &'g G, sort: bool) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(GreedyMaximumCardinalityMatching { graph, sort })
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge> for GreedyMaximumCardinalityMatching<'_, G> {
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        let mut matched: HashSet<&G::Vertex> = HashSet::new();
        let mut edges: IndexSet<G::Edge> = IndexSet::new();
        let mut cost = 0.0;
        if self.sort {
            // sort edges in increasing order of the total degree of their endpoints
            let degree = |e: &G::Edge| {
                let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
                graph.degree_of(s).unwrap() + graph.degree_of(t).unwrap()
            };
            let mut all_edges: Vec<&G::Edge> = graph.edges().collect();
            all_edges.sort_by_key(|e| degree(e));
            for e in all_edges {
                let (v, w) = graph.edge_endpoints(e).expect("edge of the graph");
                if v != w && !matched.contains(v) && !matched.contains(w) {
                    edges.insert(e.clone());
                    matched.insert(v);
                    matched.insert(w);
                    cost += graph.edge_weight(e).unwrap_or(1.0);
                }
            }
        } else {
            for v in graph.vertices() {
                if matched.contains(v) {
                    continue;
                }
                for e in graph.edges_of(v).unwrap() {
                    let w = graph.opposite_vertex(e, v).expect("incident edge");
                    if v != w && !matched.contains(w) {
                        edges.insert(e.clone());
                        matched.insert(v);
                        matched.insert(w);
                        cost += graph.edge_weight(e).unwrap_or(1.0);
                        break;
                    }
                }
            }
        }
        Ok(Matching::new(graph, edges, cost))
    }
}

/// `GreedyWeightedMatching`: edges of positive weight, heaviest first (optionally the weight is
/// divided by the total degree of the endpoints), added whenever both endpoints are free.
pub struct GreedyWeightedMatching<'g, G: Graph> {
    graph: &'g G,
    comparator: ToleranceDoubleComparator,
    normalize_edge_costs: bool,
}

impl<'g, G: Graph> GreedyWeightedMatching<'g, G> {
    pub fn new(graph: &'g G, normalize_edge_costs: bool) -> Result<Self> {
        Self::with_epsilon(graph, normalize_edge_costs, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(graph: &'g G, normalize_edge_costs: bool, epsilon: f64) -> Result<Self> {
        Ok(GreedyWeightedMatching {
            graph,
            comparator: ToleranceDoubleComparator::new(epsilon)?,
            normalize_edge_costs,
        })
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge> for GreedyWeightedMatching<'_, G> {
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        let weight = |e: &G::Edge| graph.edge_weight(e).unwrap_or(1.0);
        // sort edges in non-increasing order of weight
        let mut all_edges: Vec<&G::Edge> = graph.edges().collect();
        if self.normalize_edge_costs {
            let degree = |e: &G::Edge| {
                let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
                (graph.degree_of(s).unwrap() + graph.degree_of(t).unwrap()) as f64
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
        let mut matched_vertices: HashSet<&G::Vertex> = HashSet::new();
        // find maximal matching
        for e in all_edges {
            let edge_weight = weight(e);
            let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
            if s != t
                && self.comparator.compare(edge_weight, 0.0) > 0
                && !matched_vertices.contains(s)
                && !matched_vertices.contains(t)
            {
                matching.insert(e.clone());
                matched_vertices.insert(s);
                matched_vertices.insert(t);
                matching_weight += edge_weight;
            }
        }
        Ok(Matching::new(graph, matching, matching_weight))
    }
}
