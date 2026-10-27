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
use crate::error::{GraphError, Result};
use crate::graph::{Graph, GraphExt};
use crate::util::ToleranceDoubleComparator;
use indexmap::IndexSet;
use std::collections::HashSet;

/// `PathGrowingWeightedMatching.DEFAULT_USE_HEURISTICS`.
pub const DEFAULT_USE_HEURISTICS: bool = true;

/// A linear time $\frac{1}{2}$-approximation algorithm for finding a maximum weight matching in an
/// arbitrary graph. Linear time here means $O(m)$ where m is the cardinality of the edge set, even
/// if the graph contains isolated vertices. $\frac{1}{2}$-approximation means that for any graph
/// instance, the algorithm computes a matching whose weight is at least half of the weight of a
/// maximum weight matching. The implementation accepts directed and undirected graphs which may
/// contain self-loops and multiple edges. There is no assumption on the edge weights, i.e. they can
/// also be negative or zero.
///
/// The algorithm is due to Drake and Hougardy, described in detail in the following paper:
///
/// - D.E. Drake, S. Hougardy, A Simple Approximation Algorithm for the Weighted Matching Problem,
/// Information Processing Letters 85, 211-213, 2003.
///
/// This particular implementation uses by default two additional heuristics discussed by the authors
/// which also take linear time but improve the quality of the matchings. These heuristics can be
/// disabled by calling the constructor `PathGrowingWeightedMatching(Graph, boolean)`.
/// Disabling the heuristics has the effect of fewer passes over the edge set of the input graph,
/// probably at the expense of the total weight of the matching.
///
/// For a discussion on engineering approximate weighted matching algorithms see the following paper:
///
/// - Jens Maue and Peter Sanders. Engineering algorithms for approximate weighted matching.
/// International Workshop on Experimental and Efficient Algorithms, Springer, 2007.
///
/// # Rust notes
///
/// `PathGrowingWeightedMatching`: Drake and Hougardy's linear time 1/2-approximation of the
/// maximum weight matching. With the heuristics, every grown path is solved optimally by dynamic
/// programming and the matching is extended to a maximal one.
pub struct PathGrowingWeightedMatching<'g, G: Graph> {
    graph: &'g G,
    comparator: ToleranceDoubleComparator,
    use_heuristics: bool,
}

impl<'g, G: Graph> PathGrowingWeightedMatching<'g, G> {
    /// Construct a new instance of the path growing algorithm. Floating point values are compared
    /// using `DEFAULT_EPSILON` tolerance. By default two additional linear time heuristics
    /// are used in order to improve the quality of the matchings.
    pub fn new(graph: &'g G) -> Result<Self> {
        Self::with_options(graph, DEFAULT_USE_HEURISTICS, DEFAULT_EPSILON)
    }

    pub fn with_heuristics(graph: &'g G, use_heuristics: bool) -> Result<Self> {
        Self::with_options(graph, use_heuristics, DEFAULT_EPSILON)
    }

    pub fn with_options(graph: &'g G, use_heuristics: bool, epsilon: f64) -> Result<Self> {
        Ok(PathGrowingWeightedMatching {
            graph,
            comparator: ToleranceDoubleComparator::new(epsilon)?,
            use_heuristics,
        })
    }

    /// The `DynamicProgrammingPathSolver` of this algorithm.
    pub fn path_solver(&self) -> DynamicProgrammingPathSolver {
        DynamicProgrammingPathSolver {
            comparator: self.comparator,
            a: Vec::new(),
        }
    }

    /// Compute all vertices that have positive degree by iterating over the edges on purpose. This
    /// keeps the complexity to $O(m)$ where $m$ is the number of edges.
    fn init_visible_vertices(&self) -> IndexSet<G::Vertex> {
        let mut visible_vertex = IndexSet::new();
        for e in self.graph.edges() {
            let (s, t) = self.graph.edge_endpoints(e).expect("edge of the graph");
            if s != t {
                visible_vertex.insert(s.clone());
                visible_vertex.insert(t.clone());
            }
        }
        visible_vertex
    }

    /// The first heaviest edge incident to `x` (among visible neighbours), with its weight and
    /// neighbour.
    fn heaviest_visible_edge(
        &self,
        x: &G::Vertex,
        visible_vertex: &IndexSet<G::Vertex>,
    ) -> Result<Option<(G::Edge, f64, G::Vertex)>> {
        let graph = self.graph;
        let mut best: Option<(G::Edge, f64, G::Vertex)> = None;
        for e in graph.edges_of(x)? {
            let other = graph.opposite_vertex(e, x).expect("incident edge");
            if visible_vertex.contains(other) && other != x {
                let cur_weight = graph.edge_weight(e).unwrap_or(1.0);
                if self.comparator.compare(cur_weight, 0.0) > 0
                    && best
                        .as_ref()
                        .is_none_or(|(_, w, _)| self.comparator.compare(cur_weight, *w) > 0)
                {
                    best = Some((e.clone(), cur_weight, other.clone()));
                }
            }
        }
        Ok(best)
    }

    /// The algorithm (no heuristics).
    fn run(&self) -> Result<Matching<G::Vertex, G::Edge>> {
        let mut visible_vertex = self.init_visible_vertices();
        let mut m1: IndexSet<G::Edge> = IndexSet::new();
        let mut m2: IndexSet<G::Edge> = IndexSet::new();
        let (mut m1_weight, mut m2_weight) = (0.0, 0.0);
        let mut i = 1;
        while let Some(first) = visible_vertex.first().cloned() {
            // grow path from x
            let mut x = Some(first);
            while let Some(cur) = x {
                let best = self.heaviest_visible_edge(&cur, &visible_vertex)?;
                // add it to either m1 or m2, alternating between them
                if let Some((edge, weight, _)) = &best {
                    match i {
                        1 => {
                            m1.insert(edge.clone());
                            m1_weight += weight;
                        }
                        2 => {
                            m2.insert(edge.clone());
                            m2_weight += weight;
                        }
                        _ => {
                            return Err(GraphError::InvalidArgument(
                                "Failed to figure out matching, seems to be a bug".into(),
                            ));
                        }
                    }
                    i = 3 - i;
                }
                // remove x and incident edges
                visible_vertex.shift_remove(&cur);
                // go to next vertex
                x = best.map(|(_, _, neighbor)| neighbor);
            }
        }
        // return best matching
        if self.comparator.compare(m1_weight, m2_weight) > 0 {
            Ok(Matching::new(self.graph, m1, m1_weight))
        } else {
            Ok(Matching::new(self.graph, m2, m2_weight))
        }
    }

    /// The algorithm (improved with additional heuristics).
    fn run_with_heuristics(&self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        let mut visible_vertex = self.init_visible_vertices();
        let mut path_solver = self.path_solver();
        let mut matching: IndexSet<G::Edge> = IndexSet::new();
        let mut matching_weight = 0.0;
        let mut matched_vertices: HashSet<G::Vertex> = HashSet::new();

        while let Some(first) = visible_vertex.first().cloned() {
            // grow path from x
            let mut path: Vec<G::Edge> = Vec::new();
            let mut x = Some(first);
            while let Some(cur) = x {
                let best = self.heaviest_visible_edge(&cur, &visible_vertex)?;
                // add edge to path and remove x
                if let Some((edge, _, _)) = &best {
                    path.push(edge.clone());
                }
                visible_vertex.shift_remove(&cur);
                // go to next vertex
                x = best.map(|(_, _, neighbor)| neighbor);
            }

            // find maximum weight matching of path using dynamic programming
            let (path_weight, path_matching) =
                path_solver.get_maximum_weight_matching(graph, &path);
            // add it to result while keeping track of matched vertices
            matching_weight += path_weight;
            for e in path_matching {
                let (s, t) = graph.edge_endpoints(&e).expect("edge of the graph");
                if !matched_vertices.insert(s.clone()) || !matched_vertices.insert(t.clone()) {
                    return Err(GraphError::InvalidArgument(
                        "Set is not a valid matching, please submit a bug report".into(),
                    ));
                }
                matching.insert(e);
            }
        }

        // extend matching to maximal cardinality (out of edges with positive weight)
        for e in graph.edges() {
            let edge_weight = graph.edge_weight(e).unwrap_or(1.0);
            if self.comparator.compare(edge_weight, 0.0) <= 0 {
                // ignore zero or negative weight
                continue;
            }
            let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
            if matched_vertices.contains(s) || matched_vertices.contains(t) {
                continue;
            }
            matching.insert(e.clone());
            matched_vertices.insert(s.clone());
            matched_vertices.insert(t.clone());
            matching_weight += edge_weight;
        }
        Ok(Matching::new(graph, matching, matching_weight))
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge> for PathGrowingWeightedMatching<'_, G> {
    /// Get a matching that is a $\frac{1}{2}$-approximation of the maximum weighted matching.
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        if self.use_heuristics {
            self.run_with_heuristics()
        } else {
            self.run()
        }
    }
}

/// `PathGrowingWeightedMatching.DynamicProgrammingPathSolver`: the maximum weight matching of a
/// path, in time linear in its length.
pub struct DynamicProgrammingPathSolver {
    comparator: ToleranceDoubleComparator,
    /// work array
    a: Vec<f64>,
}

impl DynamicProgrammingPathSolver {
    const WORK_ARRAY_INITIAL_SIZE: usize = 256;

    pub fn get_maximum_weight_matching<G: Graph>(
        &mut self,
        g: &G,
        path: &[G::Edge],
    ) -> (f64, IndexSet<G::Edge>) {
        let weight = |e: &G::Edge| g.edge_weight(e).unwrap_or(1.0);
        let path_length = path.len();
        // special cases
        match path_length {
            0 => return (0.0, IndexSet::new()),
            1 => {
                let e = &path[0];
                let e_weight = weight(e);
                return if self.comparator.compare(e_weight, 0.0) > 0 {
                    (e_weight, IndexSet::from([e.clone()]))
                } else {
                    (0.0, IndexSet::new())
                };
            }
            _ => {}
        }

        // make sure work array has enough space
        if self.a.len() < path_length + 1 {
            self.a = vec![0.0; (path_length + 1).max(Self::WORK_ARRAY_INITIAL_SIZE)];
        }
        let a = &mut self.a;

        // first pass to find solution
        let e_weight = weight(&path[0]);
        a[0] = 0.0;
        a[1] = if self.comparator.compare(e_weight, 0.0) > 0 {
            e_weight
        } else {
            0.0
        };
        for i in 2..=path_length {
            let e_weight = weight(&path[i - 1]);
            if self.comparator.compare(a[i - 1], a[i - 2] + e_weight) > 0 {
                a[i] = a[i - 1];
            } else {
                a[i] = a[i - 2] + e_weight;
            }
        }

        // reverse second pass to build solution
        let mut matching = IndexSet::new();
        let mut it = path.iter().rev();
        let mut i = path_length as isize;
        while i >= 1 {
            let e = it.next().expect("path edge");
            if self.comparator.compare(a[i as usize], a[i as usize - 1]) > 0 {
                matching.insert(e.clone());
                // skip next edge
                if i > 1 {
                    it.next();
                }
                i -= 1;
            }
            i -= 1;
        }
        (a[path_length], matching)
    }
}
