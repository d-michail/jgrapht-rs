// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

use super::base::{
    HamiltonianCycleAlgorithm, HamiltonianCycleImprovementAlgorithm, check_graph,
    closed_vertex_list_to_tour, singleton_tour,
};
use super::simple::{RandomTourTsp, reverse};
use crate::error::{GraphError, Result};
use crate::graph::OutgoingGraph;
use crate::graph::UndirectedGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::java::RandomSource;
use crate::walk::GraphWalk;
use std::borrow::Borrow;
use std::collections::HashMap;

type Tour<G> = Result<Option<GraphWalk<<G as Graph>::Vertex, <G as Graph>::Edge>>>;

/// The 2-opt heuristic algorithm for the TSP problem.
///
/// The travelling salesman problem (TSP) asks the following question: "Given a list of cities and
/// the distances between each pair of cities, what is the shortest possible route that visits each
/// city exactly once and returns to the origin city?".
///
/// This is an implementation of the 2-opt improvement heuristic algorithm. The algorithm generates
/// *passes* initial tours and then iteratively improves the tours until a local minimum is
/// reached. In each iteration it applies the best possible 2-opt move which means to find the best
/// pair of edges $(i,i+1)$ and $(j,j+1)$ such that replacing them with $(i,j)$ and $(i+1,j+1)$
/// minimizes the tour length. The default initial tours use RandomTour, however an alternative
/// algorithm can be provided to create the initial tour. Initial tours generated using
/// NearestNeighborHeuristicTsp give good results and performance.
///
/// See [wikipedia](https://en.wikipedia.org/wiki/2-opt) for more details.
///
/// This implementation can also be used in order to try to improve an existing tour. See method
/// `improveTour(GraphPath)`.
///
/// # Rust notes
///
/// `TwoOptHeuristicTsp`: repeatedly applies the best improving 2-opt move to a tour produced by an
/// initializer (a random tour by default), `passes` times, and keeps the best result.
pub struct TwoOptHeuristicTsp<'a, G: Graph> {
    passes: usize,
    initializer: Box<dyn HamiltonianCycleAlgorithm<G> + 'a>,
    min_cost_improvement: f64,
}

struct State<V> {
    n: usize,
    dist: Vec<Vec<f64>>,
    index: HashMap<V, usize>,
    rev_index: Vec<V>,
}

impl<'a, G: 'a + OutgoingGraph + UndirectedGraph> TwoOptHeuristicTsp<'a, G> {
    /// One pass from a random tour.
    pub fn new() -> Self {
        Self::with_passes(1)
    }

    /// Creates an instance with the given passes.
    pub fn with_passes(passes: usize) -> Self {
        Self::with_all(passes, Box::new(RandomTourTsp::new()), 1e-8).expect("one pass at least")
    }

    /// Creates an instance with the given seed.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_seed(passes: usize, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::with_rng(passes, rand::rngs::StdRng::seed_from_u64(seed))
    }

    /// Creates an instance with the given rng.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_rng(passes: usize, rng: impl RandomSource + 'a) -> Result<Self> {
        Self::with_all(passes, Box::new(RandomTourTsp::with_rng(rng)), 1e-8)
    }

    /// Creates an instance with the given initializer.
    pub fn with_initializer(initializer: impl HamiltonianCycleAlgorithm<G> + 'a) -> Self {
        Self::with_all(1, Box::new(initializer), 1e-8).expect("one pass at least")
    }

    /// `minCostImprovement`: a move must shorten the tour by more than this (absolute value).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_all(
        passes: usize,
        initializer: Box<dyn HamiltonianCycleAlgorithm<G> + 'a>,
        min_cost_improvement: f64,
    ) -> Result<Self> {
        if passes < 1 {
            return Err(GraphError::InvalidParameter {
                name: "passes",
                requirement: crate::ParameterRequirement::AtLeast(1),
            });
        }
        Ok(TwoOptHeuristicTsp {
            passes,
            initializer,
            min_cost_improvement: min_cost_improvement.abs(),
        })
    }

    /// Initialize graph and mapping to integer vertices.
    fn init(graph: &G) -> State<G::Vertex> {
        let n = graph.vertex_count();
        let rev_index: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let index: HashMap<G::Vertex, usize> = rev_index
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let mut dist = vec![vec![0.0; n]; n];
        for e in graph.edges() {
            let e = e.borrow();
            let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
            let weight = graph.weight_of(e);
            let (si, ti) = (index[s], index[t]);
            dist[si][ti] = weight;
            dist[ti][si] = weight;
        }
        State {
            n,
            dist,
            index,
            rev_index,
        }
    }

    /// Create an initial tour
    fn create_initial_tour(&mut self, graph: &G, state: &State<G::Vertex>) -> Result<Vec<usize>> {
        let tour = self
            .initializer
            .tour(graph)?
            .ok_or(GraphError::InvalidTour)?;
        Self::path_to_tour(&tour, state)
    }

    /// Improve the tour using the 2-opt heuristic. In each iteration it applies the best possible
    /// 2-opt move which means to find the best pair of edges $(i,i+1)$ and $(j,j+1)$ such that
    /// replacing them with $(i,j)$ and $(i+1,j+1)$ minimizes the tour length.
    ///
    /// The returned array instance might or might not be the input array.
    fn improve(&self, mut tour: Vec<usize>, state: &State<G::Vertex>) -> Vec<usize> {
        let (n, dist) = (state.n, &state.dist);
        loop {
            let mut min_change = -self.min_cost_improvement;
            let (mut mini, mut minj) = (usize::MAX, usize::MAX);
            for i in 0..n.saturating_sub(2) {
                for j in i + 2..n {
                    let (ci, ci1, cj, cj1) = (tour[i], tour[i + 1], tour[j], tour[j + 1]);
                    let change = (dist[ci][cj] + dist[ci1][cj1]) - (dist[ci][ci1] + dist[cj][cj1]);
                    if change < min_change {
                        min_change = change;
                        mini = i;
                        minj = j;
                    }
                }
            }
            if mini != usize::MAX && minj != usize::MAX {
                reverse(&mut tour, mini as isize + 1, minj as isize);
            } else {
                return tour;
            }
        }
    }

    /// Transform from an array representation to a graph path.
    fn tour_to_path(
        graph: &G,
        tour: &[usize],
        state: &State<G::Vertex>,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        let vertices = tour.iter().map(|&vi| state.rev_index[vi].clone()).collect();
        closed_vertex_list_to_tour(vertices, graph)
    }

    /// Transform from a path representation to an array representation.
    fn path_to_tour(
        path: &GraphWalk<G::Vertex, G::Edge>,
        state: &State<G::Vertex>,
    ) -> Result<Vec<usize>> {
        let n = state.n;
        let vertex_list = path.vertex_list();
        if vertex_list.len() != n + 1 {
            return Err(GraphError::InvalidTour);
        }
        let mut visited = vec![false; n];
        let mut tour = vec![0usize; n + 1];
        for i in 0..n {
            let vi = *state
                .index
                .get(&vertex_list[i])
                .ok_or(GraphError::InvalidTour)?;
            if visited[vi] {
                return Err(GraphError::InvalidTour);
            }
            visited[vi] = true;
            tour[i] = vi;
        }
        tour[n] = tour[0];
        Ok(tour)
    }
}

impl<'a, G: UndirectedGraph + OutgoingGraph + 'a> Default for TwoOptHeuristicTsp<'a, G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, G: 'a + OutgoingGraph + UndirectedGraph> HamiltonianCycleAlgorithm<G>
    for TwoOptHeuristicTsp<'a, G>
{
    /// Computes a 2-approximate tour.
    fn tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        if graph.vertex_count() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        let state = Self::init(graph);
        let initial = self.create_initial_tour(graph, &state)?;
        let mut best = Self::tour_to_path(graph, &self.improve(initial, &state), &state)?;
        for _ in 1..self.passes {
            let initial = self.create_initial_tour(graph, &state)?;
            let other = Self::tour_to_path(graph, &self.improve(initial, &state), &state)?;
            if other.weight() < best.weight() {
                best = other;
            }
        }
        Ok(Some(best))
    }
}

impl<'a, G: UndirectedGraph + OutgoingGraph + 'a> HamiltonianCycleImprovementAlgorithm<G>
    for TwoOptHeuristicTsp<'a, G>
{
    /// Try to improve a tour by running the 2-opt heuristic.
    fn improve_tour(
        &mut self,
        graph: &G,
        tour: &GraphWalk<G::Vertex, G::Edge>,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        let state = Self::init(graph);
        let initial = Self::path_to_tour(tour, &state)?;
        Self::tour_to_path(graph, &self.improve(initial, &state), &state)
    }
}
