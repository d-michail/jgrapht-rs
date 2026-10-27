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

use crate::graph::GraphExt;
use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use crate::error::{GraphError, Result};
use crate::graph::Graph;
use crate::graph::props::{is_complete, require_undirected};
use crate::traverse::DepthFirstIterator;
use crate::walk::GraphWalk;
use std::collections::HashSet;

/// An algorithm solving the [Hamiltonian cycle problem](https://en.wikipedia.org/wiki/Hamiltonian_path).
///
/// A Hamiltonian cycle, also called a Hamiltonian circuit, Hamilton cycle, or Hamilton circuit, is a
/// graph cycle (i.e., closed loop) through a graph that visits each node exactly once (Skiena 1990,
/// p. 196).
///
/// # Rust notes
///
/// `HamiltonianCycleAlgorithm`: computes a tour (a Hamiltonian cycle) of a graph.
pub trait HamiltonianCycleAlgorithm<G: Graph> {
    /// The tour, or `None` when the algorithm proves that there is none (Held–Karp only).
    fn get_tour(&mut self, graph: &G) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>>;
}

/// An algorithm improving the result of solving the
/// [Hamiltonian cycle problem](https://en.wikipedia.org/wiki/Hamiltonian_path).
///
/// A Hamiltonian cycle, also called a Hamiltonian circuit, Hamilton cycle, or Hamilton circuit, is a
/// graph cycle (i.e., closed loop) through a graph that visits each node exactly once (Skiena 1990,
/// p. 196).
///
/// An improvement algorithm could be one that optimises the cycle for lower cost, or that updates
/// the cycle to match changes in the graph.
///
/// # Rust notes
///
/// `HamiltonianCycleImprovementAlgorithm`: improves an existing tour.
pub trait HamiltonianCycleImprovementAlgorithm<G: Graph> {
    fn improve_tour(
        &mut self,
        graph: &G,
        tour: &GraphWalk<G::Vertex, G::Edge>,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>>;
}

/// `vertexListToTour`: closes `tour` by repeating its first vertex.
pub(crate) fn vertex_list_to_tour<G: Graph>(
    mut tour: Vec<G::Vertex>,
    graph: &G,
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    tour.push(tour[0].clone());
    closed_vertex_list_to_tour(tour, graph)
}

/// `closedVertexListToTour`: `tour` already ends where it starts.
pub(crate) fn closed_vertex_list_to_tour<G: Graph>(
    tour: Vec<G::Vertex>,
    graph: &G,
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    debug_assert!(tour.first() == tour.last());
    let mut edges = Vec::with_capacity(tour.len() - 1);
    let mut tour_weight = 0.0;
    for pair in tour.windows(2) {
        let e = graph
            .get_edge(&pair[0], &pair[1])
            .ok_or(GraphError::EdgeNotFound)?;
        tour_weight += graph.weight_of(e);
        edges.push(e.clone());
    }
    Ok(GraphWalk::new(tour, edges, tour_weight))
}

/// `edgeSetToTour`: walks the edges of `tour` depth first.
pub(crate) fn edge_set_to_tour<G: IncidenceGraph>(
    tour: &HashSet<G::Edge>,
    graph: &G,
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let tour_graph =
        crate::view::MaskSubgraph::new(graph, |_| false, |e: &G::Edge| !tour.contains(e));
    let vertices: Vec<G::Vertex> = DepthFirstIterator::new(&tour_graph).cloned().collect();
    vertex_list_to_tour(vertices, graph)
}

/// `getSingletonTour`.
pub(crate) fn singleton_tour<G: Graph>(graph: &G) -> GraphWalk<G::Vertex, G::Edge> {
    let start = graph.vertices().next().expect("one vertex").clone();
    GraphWalk::new(vec![start], Vec::new(), 0.0)
}

pub(crate) fn require_not_empty<G: Graph>(graph: &G) -> Result<()> {
    if graph.vertex_count() == 0 {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::NonEmpty));
    }
    Ok(())
}

/// `checkGraph`: undirected, non-empty and complete.
pub(crate) fn check_graph<G: OutgoingGraph>(graph: &G) -> Result<()> {
    require_undirected(graph)?;
    require_not_empty(graph)?;
    if !is_complete(graph) {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::Complete));
    }
    Ok(())
}

/// The weight of the edge between `u` and `v` (the graph must be complete).
pub(crate) fn weight_between<G: Graph>(graph: &G, u: &G::Vertex, v: &G::Vertex) -> f64 {
    graph.weight_of(graph.get_edge(u, v).expect("complete graph"))
}
