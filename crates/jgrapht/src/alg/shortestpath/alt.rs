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

use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// An admissible heuristic for the A* algorithm using a set of landmarks and the triangle
/// inequality. Assumes that the graph contains non-negative edge weights.
///
/// The heuristic requires a set of input nodes from the graph, which are used as landmarks. During a
/// pre-processing phase, which requires two shortest path computations per landmark using Dijkstra's
/// algorithm, all distances to and from these landmark nodes are computed and stored. Afterwards,
/// the heuristic estimates the distance from a vertex to another vertex using the already computed
/// distances to and from the landmarks and the fact that shortest path distances obey the
/// triangle-inequality. The heuristic's space requirement is $O(n)$ per landmark where n is the
/// number of vertices of the graph. In case of undirected graphs only one Dijkstra's algorithm
/// execution is performed per landmark.
///
/// The method generally abbreviated as ALT (from A*, Landmarks and Triangle inequality) is described
/// in detail in the following [paper](https://www.microsoft.com/en-us/research/publication/computing-the-shortest-path-a-search-meets-graph-theory) which also contains a discussion on landmark selection strategies.
///
/// - Andrew Goldberg and Chris Harrelson. Computing the shortest path: A* Search Meets Graph
/// Theory. In Proceedings of the sixteenth annual ACM-SIAM symposium on Discrete algorithms (SODA'
/// 05), 156--165, 2005.
///
/// Note that using this heuristic does not require the edge weights to satisfy the
/// triangle-inequality. The method depends on the triangle inequality with respect to the shortest
/// path distances in the graph, not an embedding in Euclidean space or some other metric, which need
/// not be present.
///
/// In general more landmarks will speed up A* but will need more space. Given an A* query with
/// vertices source and target, a good landmark appears "before" source or "after" target where
/// before and after are relative to the "direction" from source to target.
///
/// # Rust notes
///
/// A* heuristic based on landmarks and the triangle inequality.
///
/// Construction performs one Dijkstra search per landmark in undirected
/// graphs and a forward plus reverse search in directed graphs. Query time is
/// linear in the number of landmarks.
pub struct ALTAdmissibleHeuristic<'g, G: Graph> {
    graph: &'g G,
    from_landmark: HashMap<G::Vertex, HashMap<G::Vertex, f64>>,
    to_landmark: HashMap<G::Vertex, HashMap<G::Vertex, f64>>,
    directed: bool,
}

impl<'g, G: IncidenceGraph> ALTAdmissibleHeuristic<'g, G> {
    /// Constructs a new `AStarAdmissibleHeuristic` using a set of landmarks.
    pub fn new<I>(graph: &'g G, landmarks: I) -> Result<Self>
    where
        I: IntoIterator<Item = G::Vertex>,
    {
        let landmarks: Vec<_> = landmarks.into_iter().collect();
        if landmarks.is_empty() {
            return Err(GraphError::InvalidArgument(
                "at least one landmark must be provided".into(),
            ));
        }
        for landmark in &landmarks {
            if !graph.contains_vertex(landmark) {
                return Err(GraphError::VertexNotFound);
            }
        }
        if graph.edges().any(|edge| {
            let weight = graph.edge_weight(edge).unwrap_or(1.0);
            weight.is_nan() || weight < 0.0
        }) {
            return Err(GraphError::InvalidArgument(
                "graph edge weights cannot be negative or NaN".into(),
            ));
        }

        let directed = graph.graph_type().directed;
        let mut from_landmark = HashMap::new();
        let mut to_landmark = HashMap::new();
        for landmark in landmarks {
            let from = distances(graph, &landmark, false)?;
            let to = if directed {
                distances(graph, &landmark, true)?
            } else {
                from.clone()
            };
            from_landmark.insert(landmark.clone(), from);
            to_landmark.insert(landmark, to);
        }
        Ok(Self {
            graph,
            from_landmark,
            to_landmark,
            directed,
        })
    }

    pub fn graph(&self) -> &G {
        self.graph
    }

    /// An admissible heuristic estimate from a source vertex to a target vertex. The estimate is
    /// always non-negative and never overestimates the true distance.
    pub fn cost_estimate(&self, source: &G::Vertex, target: &G::Vertex) -> f64 {
        if source == target {
            return 0.0;
        }
        if let Some(from) = self.from_landmark.get(source) {
            return from.get(target).copied().unwrap_or(f64::INFINITY);
        }
        if let Some(to) = self.to_landmark.get(target) {
            return to.get(source).copied().unwrap_or(f64::INFINITY);
        }

        let mut estimate = 0.0f64;
        for (landmark, from) in &self.from_landmark {
            let candidate = if self.directed {
                let to = &self.to_landmark[landmark];
                let before = to.get(source).copied().unwrap_or(f64::INFINITY)
                    - to.get(target).copied().unwrap_or(f64::INFINITY);
                let after = from.get(target).copied().unwrap_or(f64::INFINITY)
                    - from.get(source).copied().unwrap_or(f64::INFINITY);
                before.max(after)
            } else {
                (from.get(source).copied().unwrap_or(f64::INFINITY)
                    - from.get(target).copied().unwrap_or(f64::INFINITY))
                .abs()
            };
            if candidate.is_finite() {
                estimate = estimate.max(candidate);
            }
        }
        estimate
    }

    /// Returns true if the heuristic is a *consistent* or *monotone* heuristic wrt the
    /// provided `graph`. A heuristic is monotonic if its estimate is always less than or equal
    /// to the estimated distance from any neighboring vertex to the goal, plus the step cost of
    /// reaching that neighbor. For details, refer to [https://en.wikipedia.org/wiki/Consistent_heuristic](https://en.wikipedia.org/wiki/Consistent_heuristic).
    /// In short, a heuristic is consistent iff $h(u) \le d(u,v)+h(v)$, for every edge $(u,v)$, where
    /// $d(u,v)$ is the weight of edge $(u,v)$ and $h(u)$ is the estimated cost to reach the target
    /// node from vertex u. Most natural admissible heuristics, such as Manhattan or Euclidean
    /// distance, are consistent heuristics.
    pub fn is_consistent(&self) -> bool {
        true
    }
}

fn distances<G: IncidenceGraph>(
    graph: &G,
    source: &G::Vertex,
    reverse: bool,
) -> Result<HashMap<G::Vertex, f64>> {
    let source = graph
        .vertices()
        .find(|vertex| *vertex == source)
        .ok_or(GraphError::VertexNotFound)?;
    let mut distance = HashMap::from([(source, 0.0)]);
    let mut settled = HashSet::new();
    let mut nodes = vec![source];
    let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), 0usize))]);

    while let Some(Reverse((OrderedF64(value), index))) = heap.pop() {
        let vertex = nodes[index];
        if settled.contains(vertex) || value > distance[vertex] {
            continue;
        }
        settled.insert(vertex);
        let adjacent: Vec<_> = if reverse {
            graph.incoming_of(vertex).collect()
        } else {
            graph.outgoing_of(vertex).collect()
        };
        for (edge, target) in adjacent {
            let candidate = value + graph.edge_weight(edge).unwrap_or(1.0);
            if candidate < distance.get(target).copied().unwrap_or(f64::INFINITY) {
                distance.insert(target, candidate);
                nodes.push(target);
                heap.push(Reverse((OrderedF64(candidate), nodes.len() - 1)));
            }
        }
    }

    Ok(distance
        .into_iter()
        .map(|(vertex, value)| (vertex.clone(), value))
        .collect())
}
