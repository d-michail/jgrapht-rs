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
use crate::graph::GraphExt;
use crate::graph::OutgoingGraph;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use indexmap::IndexSet;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

pub const DEFAULT_EPSILON: f64 = 1e-9;

/// Reported when a negative-weight cycle is reachable from the source of a shortest-path
/// computation, together with the cycle that was found.
#[derive(Debug, Clone, thiserror::Error)]
#[error("graph contains a negative-weight cycle")]
pub struct NegativeCycleDetected<V, E> {
    cycle: GraphWalk<V, E>,
}

impl<V, E> NegativeCycleDetected<V, E> {
    /// Wraps the negative cycle that was found.
    pub fn new(cycle: GraphWalk<V, E>) -> Self {
        Self { cycle }
    }

    /// The negative cycle, as a closed walk.
    pub fn cycle(&self) -> &GraphWalk<V, E> {
        &self.cycle
    }

    /// Consumes the error and returns the negative cycle.
    pub fn into_cycle(self) -> GraphWalk<V, E> {
        self.cycle
    }
}

/// Errors of [`BellmanFord::get_paths`] and [`BellmanFord::get_path`]: either a graph error or a
/// negative cycle, which carries the cycle itself.
///
/// The common [`ShortestPathAlgorithm`] interface reports a negative cycle as the payload-free
/// [`GraphError::NegativeCycle`]; call these methods directly to get the cycle.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BellmanFordError<V, E> {
    /// The computation failed for a reason unrelated to negative cycles.
    #[error(transparent)]
    Graph(#[from] GraphError),
    /// A negative-weight cycle is reachable from the source.
    #[error(transparent)]
    NegativeCycle(NegativeCycleDetected<V, E>),
}

impl<V, E> BellmanFordError<V, E> {
    /// The negative cycle, if that is what went wrong.
    pub fn cycle(&self) -> Option<&GraphWalk<V, E>> {
        match self {
            Self::NegativeCycle(error) => Some(error.cycle()),
            Self::Graph(_) => None,
        }
    }
}

/// The Bellman-Ford algorithm.
///
/// Computes shortest paths from a single source vertex to all other vertices in a weighted graph.
/// The Bellman-Ford algorithm supports negative edge weights.
///
/// Negative weight cycles are not allowed and will be reported by the algorithm. This implies that
/// negative edge weights are not allowed in undirected graphs. In such cases the call fails with a
/// [`NegativeCycleDetected`] error which contains the detected negative weight cycle. Note that the
/// algorithm will not report or find negative weight cycles which are not reachable from the source
/// vertex.
///
/// The running time is $O(|E||V|)$.
///
/// # Rust notes
///
/// Bellman–Ford single-source shortest paths in `O(|V||E|)` time.
///
/// [`BellmanFord::get_paths`] and [`BellmanFord::get_path`] return the negative cycle in their
/// [`BellmanFordError`]. The [`ShortestPathAlgorithm`] implementation maps that richer error to
/// [`GraphError::NegativeCycle`] for the common algorithm interface.
pub struct BellmanFord<'g, G> {
    graph: &'g G,
    epsilon: f64,
    max_hops: usize,
}

/// Java-compatible class name for [`BellmanFord`].
pub type BellmanFordShortestPath<'g, G> = BellmanFord<'g, G>;

impl<'g, G> BellmanFord<'g, G> {
    /// Construct a new instance.
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            epsilon: DEFAULT_EPSILON,
            max_hops: usize::MAX,
        }
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_epsilon(graph: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_options(graph, epsilon, usize::MAX)
    }

    /// Creates an instance with the given options.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_options(graph: &'g G, epsilon: f64, max_hops: usize) -> Result<Self> {
        if epsilon <= 0.0 {
            return Err(GraphError::must_be_positive("tolerance"));
        }
        if max_hops == 0 {
            return Err(GraphError::must_be_positive("number_of_hops"));
        }
        Ok(Self {
            graph,
            epsilon,
            max_hops,
        })
    }
}

impl<G: OutgoingGraph> BellmanFord<'_, G> {
    fn improves(&self, candidate: f64, current: f64) -> bool {
        (candidate - current).abs() >= self.epsilon && candidate < current
    }

    /// Computes a negative weight cycle assuming that the algorithm has already determined that it
    /// exists.
    fn negative_cycle(
        &self,
        edge: &G::Edge,
        predecessor: &HashMap<G::Vertex, (G::Edge, G::Vertex)>,
    ) -> GraphWalk<G::Vertex, G::Edge> {
        let mut visited = HashSet::new();
        let mut current = self
            .graph
            .edge_target(edge)
            .expect("edge belongs to graph")
            .clone();
        visited.insert(current.clone());
        current = self
            .graph
            .opposite_vertex(edge, &current)
            .expect("edge endpoint")
            .clone();

        while visited.insert(current.clone()) {
            let (edge, previous) = predecessor
                .get(&current)
                .expect("a detected negative cycle has predecessors");
            debug_assert!(self.graph.opposite_vertex(edge, &current) == Some(previous));
            current = previous.clone();
        }

        let start = current.clone();
        let mut edges = Vec::new();
        let mut weight = 0.0;
        loop {
            let (edge, previous) = predecessor
                .get(&current)
                .expect("a detected negative cycle has predecessors");
            edges.push(edge.clone());
            weight += self.graph.weight_of(edge);
            current = previous.clone();
            if current == start {
                break;
            }
        }
        edges.reverse();
        GraphWalk::from_edges(self.graph, start, edges, weight).expect("predecessors form a cycle")
    }

    /// Compute all shortest paths starting from a single source vertex.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph;
    /// [`GraphError::InvalidWeight`] if an edge weight is negative, NaN or not finite. Errors of
    /// the graph operations it performs are propagated.
    pub fn get_paths(
        &self,
        source: &G::Vertex,
    ) -> std::result::Result<
        SingleSourcePaths<G::Vertex, G::Edge>,
        BellmanFordError<G::Vertex, G::Edge>,
    > {
        if !self.graph.contains_vertex(source) {
            return Err(GraphError::VertexNotFound.into());
        }
        for edge in self.graph.edges() {
            let edge = edge.borrow();
            let weight = self.graph.weight_of(edge);
            if weight.is_nan() {
                return Err(GraphError::InvalidWeight(crate::WeightProblem::NaN).into());
            }
            if weight.is_infinite() {
                return Err(GraphError::InvalidWeight(crate::WeightProblem::NotFinite).into());
            }
        }
        let vertex_count = self.graph.vertex_count();
        let mut distance: HashMap<G::Vertex, f64> = HashMap::from([(source.clone(), 0.0)]);
        let mut predecessor: HashMap<G::Vertex, (G::Edge, G::Vertex)> = HashMap::new();
        let mut current: IndexSet<G::Vertex> = IndexSet::from([source.clone()]);
        let mut next: IndexSet<G::Vertex> = IndexSet::new();

        for _ in 0..vertex_count.saturating_sub(1).min(self.max_hops) {
            for vertex in &current {
                let vertex_distance = distance[vertex];
                for (edge, target) in self.graph.outgoing_of(vertex) {
                    let candidate = vertex_distance + self.graph.weight_of(edge);
                    if self.improves(
                        candidate,
                        distance.get(target).copied().unwrap_or(f64::INFINITY),
                    ) {
                        distance.insert(target.clone(), candidate);
                        predecessor.insert(target.clone(), (edge.clone(), vertex.clone()));
                        next.insert(target.clone());
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
                let vertex_distance = distance[vertex];
                for (edge, target) in self.graph.outgoing_of(vertex) {
                    let candidate = vertex_distance + self.graph.weight_of(edge);
                    if self.improves(
                        candidate,
                        distance.get(target).copied().unwrap_or(f64::INFINITY),
                    ) {
                        predecessor.insert(target.clone(), (edge.clone(), vertex.clone()));
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
            dist: distance,
            pred: predecessor,
        })
    }

    /// Get a shortest path from a source vertex to a sink vertex.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
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

impl<G: OutgoingGraph> ShortestPathAlgorithm<G> for BellmanFord<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        self.get_paths(source).map_err(|error| match error {
            BellmanFordError::Graph(error) => error,
            BellmanFordError::NegativeCycle(_) => GraphError::NegativeCycle,
        })
    }
}

/// Relaxes all edges up to `|V|` rounds from the given initial distances.
/// Returns `Err(NegativeCycle)` if still improving afterwards.
pub(crate) fn relax_all<G: OutgoingGraph>(
    graph: &G,
    distance: &mut HashMap<G::Vertex, f64>,
    predecessor: &mut HashMap<G::Vertex, (G::Edge, G::Vertex)>,
) -> Result<()> {
    let vertex_count = graph.vertex_count();
    for round in 0..=vertex_count {
        let mut changed = false;
        for vertex in graph.vertices() {
            let vertex = vertex.borrow();
            let Some(&vertex_distance) = distance.get(vertex) else {
                continue;
            };
            for (edge, target) in graph.outgoing_of(vertex) {
                let candidate = vertex_distance + graph.weight_of(edge);
                if candidate < distance.get(target).copied().unwrap_or(f64::INFINITY) {
                    distance.insert(target.clone(), candidate);
                    predecessor.insert(target.clone(), (edge.clone(), vertex.clone()));
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
