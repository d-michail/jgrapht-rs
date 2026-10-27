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

use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet};
use std::fmt;

pub const DEFAULT_EPSILON: f64 = 1e-9;

/// An exception used to notify about the detection of a negative cycle. The exception may carry the
/// negative cycle to the user.
///
/// # Rust notes
///
/// A detected negative-weight cycle, including the cycle itself.
#[derive(Debug, Clone)]
pub struct NegativeCycleDetected<V, E> {
    cycle: GraphWalk<V, E>,
}

impl<V, E> NegativeCycleDetected<V, E> {
    pub fn new(cycle: GraphWalk<V, E>) -> Self {
        Self { cycle }
    }

    pub fn cycle(&self) -> &GraphWalk<V, E> {
        &self.cycle
    }

    pub fn into_cycle(self) -> GraphWalk<V, E> {
        self.cycle
    }
}

impl<V, E> fmt::Display for NegativeCycleDetected<V, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("graph contains a negative-weight cycle")
    }
}

impl<V: fmt::Debug, E: fmt::Debug> std::error::Error for NegativeCycleDetected<V, E> {}

/// Errors produced by the Java-style Bellman–Ford entry points.
#[derive(Debug, Clone)]
pub enum BellmanFordError<V, E> {
    Graph(GraphError),
    NegativeCycle(NegativeCycleDetected<V, E>),
}

impl<V, E> BellmanFordError<V, E> {
    pub fn cycle(&self) -> Option<&GraphWalk<V, E>> {
        match self {
            Self::NegativeCycle(error) => Some(error.cycle()),
            Self::Graph(_) => None,
        }
    }
}

impl<V, E> fmt::Display for BellmanFordError<V, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => error.fmt(formatter),
            Self::NegativeCycle(error) => error.fmt(formatter),
        }
    }
}

impl<V: fmt::Debug, E: fmt::Debug> std::error::Error for BellmanFordError<V, E> {}

impl<V, E> From<GraphError> for BellmanFordError<V, E> {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}

/// The Bellman-Ford algorithm.
///
/// Computes shortest paths from a single source vertex to all other vertices in a weighted graph.
/// The Bellman-Ford algorithm supports negative edge weights.
///
/// Negative weight cycles are not allowed and will be reported by the algorithm. This implies that
/// negative edge weights are not allowed in undirected graphs. In such cases the code will throw an
/// exception of type `NegativeCycleDetectedException` which will contain the detected negative
/// weight cycle. Note that the algorithm will not report or find negative weight cycles which are
/// not reachable from the source vertex.
///
/// The running time is $O(|E||V|)$.
///
/// # Rust notes
///
/// Bellman–Ford single-source shortest paths in `O(|V||E|)` time.
///
/// [`BellmanFord::get_paths`] and [`BellmanFord::get_path`] preserve Java's negative-cycle
/// payload. The [`ShortestPathAlgorithm`] implementation maps that richer error to
/// [`GraphError::NegativeCycle`] for the common algorithm interface.
pub struct BellmanFord<'g, G> {
    graph: &'g G,
    epsilon: f64,
    max_hops: usize,
}

/// Java-compatible class name for [`BellmanFord`].
pub type BellmanFordShortestPath<'g, G> = BellmanFord<'g, G>;

impl<'g, G> BellmanFord<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            epsilon: DEFAULT_EPSILON,
            max_hops: usize::MAX,
        }
    }

    pub fn with_epsilon(graph: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_options(graph, epsilon, usize::MAX)
    }

    pub fn with_options(graph: &'g G, epsilon: f64, max_hops: usize) -> Result<Self> {
        if epsilon <= 0.0 {
            return Err(GraphError::InvalidArgument(
                "tolerance must be positive".into(),
            ));
        }
        if max_hops == 0 {
            return Err(GraphError::InvalidArgument(
                "number of hops must be positive".into(),
            ));
        }
        Ok(Self {
            graph,
            epsilon,
            max_hops,
        })
    }
}

impl<G: Graph> BellmanFord<'_, G> {
    fn improves(&self, candidate: f64, current: f64) -> bool {
        (candidate - current).abs() >= self.epsilon && candidate < current
    }

    fn negative_cycle(
        &self,
        edge: &G::Edge,
        predecessor: &HashMap<&G::Vertex, (&G::Edge, &G::Vertex)>,
    ) -> GraphWalk<G::Vertex, G::Edge> {
        let mut visited = HashSet::new();
        let mut current = self.graph.edge_target(edge).expect("edge belongs to graph");
        visited.insert(current);
        current = self
            .graph
            .opposite_vertex(edge, current)
            .expect("edge endpoint");

        while visited.insert(current) {
            let (edge, previous) = predecessor
                .get(current)
                .expect("a detected negative cycle has predecessors");
            debug_assert!(self.graph.opposite_vertex(edge, current) == Some(*previous));
            current = previous;
        }

        let start = current;
        let mut edges = Vec::new();
        let mut weight = 0.0;
        loop {
            let (edge, previous) = predecessor
                .get(current)
                .expect("a detected negative cycle has predecessors");
            edges.push((*edge).clone());
            weight += self.graph.edge_weight(edge).unwrap_or(1.0);
            current = previous;
            if current == start {
                break;
            }
        }
        edges.reverse();
        GraphWalk::from_edges(self.graph, start.clone(), edges, weight)
            .expect("predecessors form a cycle")
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
    ) -> std::result::Result<
        SingleSourcePaths<G::Vertex, G::Edge>,
        BellmanFordError<G::Vertex, G::Edge>,
    > {
        let source = self
            .graph
            .vertices()
            .find(|vertex| *vertex == source)
            .ok_or(GraphError::VertexNotFound)?;
        let vertex_count = self.graph.vertex_count();
        let mut distance = HashMap::from([(source, 0.0)]);
        let mut predecessor = HashMap::new();
        let mut current = IndexSet::from([source]);
        let mut next = IndexSet::new();

        for _ in 0..vertex_count.saturating_sub(1).min(self.max_hops) {
            for vertex in &current {
                let vertex = *vertex;
                let vertex_distance = distance[vertex];
                for (edge, target) in self.graph.outgoing_of(vertex) {
                    let candidate = vertex_distance + self.graph.edge_weight(edge).unwrap_or(1.0);
                    if self.improves(
                        candidate,
                        distance.get(target).copied().unwrap_or(f64::INFINITY),
                    ) {
                        distance.insert(target, candidate);
                        predecessor.insert(target, (edge, vertex));
                        next.insert(target);
                    }
                }
            }
            current.clear();
            std::mem::swap(&mut current, &mut next);
            if current.is_empty() {
                break;
            }
        }

        if self.max_hops >= vertex_count {
            for vertex in &current {
                let vertex = *vertex;
                let vertex_distance = distance[vertex];
                for (edge, target) in self.graph.outgoing_of(vertex) {
                    let candidate = vertex_distance + self.graph.edge_weight(edge).unwrap_or(1.0);
                    if self.improves(
                        candidate,
                        distance.get(target).copied().unwrap_or(f64::INFINITY),
                    ) {
                        predecessor.insert(target, (edge, vertex));
                        let cycle = self.negative_cycle(edge, &predecessor);
                        return Err(BellmanFordError::NegativeCycle(NegativeCycleDetected::new(
                            cycle,
                        )));
                    }
                }
            }
        }

        Ok(SingleSourcePaths {
            source: source.clone(),
            dist: distance
                .into_iter()
                .map(|(vertex, value)| (vertex.clone(), value))
                .collect(),
            pred: predecessor
                .into_iter()
                .map(|(vertex, (edge, previous))| {
                    (vertex.clone(), (edge.clone(), previous.clone()))
                })
                .collect(),
        })
    }

    pub fn get_path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> std::result::Result<
        Option<GraphWalk<G::Vertex, G::Edge>>,
        BellmanFordError<G::Vertex, G::Edge>,
    > {
        if !self.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound.into());
        }
        Ok(self.get_paths(source)?.path(sink))
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for BellmanFord<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        self.get_paths(source).map_err(|error| match error {
            BellmanFordError::Graph(error) => error,
            BellmanFordError::NegativeCycle(_) => GraphError::NegativeCycle,
        })
    }
}

/// Relaxes all edges up to `|V|` rounds from the given initial distances.
/// Returns `Err(NegativeCycle)` if still improving afterwards.
pub(crate) fn relax_all<'g, G: Graph>(
    graph: &'g G,
    distance: &mut HashMap<&'g G::Vertex, f64>,
    predecessor: &mut HashMap<&'g G::Vertex, (&'g G::Edge, &'g G::Vertex)>,
) -> Result<()> {
    let vertex_count = graph.vertex_count();
    for round in 0..=vertex_count {
        let mut changed = false;
        for vertex in graph.vertices() {
            let Some(&vertex_distance) = distance.get(vertex) else {
                continue;
            };
            for (edge, target) in graph.outgoing_of(vertex) {
                let candidate = vertex_distance + graph.edge_weight(edge).unwrap_or(1.0);
                if candidate < distance.get(target).copied().unwrap_or(f64::INFINITY) {
                    distance.insert(target, candidate);
                    predecessor.insert(target, (edge, vertex));
                    changed = true;
                }
            }
        }
        if !changed {
            return Ok(());
        }
        if round == vertex_count {
            return Err(GraphError::NegativeCycle);
        }
    }
    Ok(())
}
