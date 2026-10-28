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

use super::SingleSourcePaths;
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::borrow::Borrow;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Lightweight closest-first Dijkstra iterator.
///
/// Vertices are yielded when their distance becomes final. The predecessor
/// tree also contains tentative vertices discovered inside the radius, just
/// like JGraphT's internal `DijkstraClosestFirstIterator`.
pub struct DijkstraClosestFirstIterator<'g, G: Graph> {
    graph: &'g G,
    source: G::Vertex,
    radius: f64,
    distance: HashMap<G::Vertex, f64>,
    predecessor: HashMap<G::Vertex, (G::Edge, G::Vertex)>,
    settled: HashSet<G::Vertex>,
    nodes: Vec<G::Vertex>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl<'g, G: Graph> DijkstraClosestFirstIterator<'g, G> {
    /// Creates a new iterator for the specified graph. Iteration will start at the specified start
    /// vertex and will be limited to the connected component that includes that vertex. This
    /// iterator will use pairing heap as a default heap implementation.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(graph: &'g G, source: &G::Vertex) -> Result<Self> {
        Self::with_radius(graph, source, f64::INFINITY)
    }

    /// Creates an instance with the given radius.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph;
    /// [`GraphError::InvalidWeight`] if an edge weight is negative, NaN or not finite;
    /// [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_radius(graph: &'g G, source: &G::Vertex, radius: f64) -> Result<Self> {
        if radius.is_nan() || radius < 0.0 {
            return Err(GraphError::must_be_non_negative("radius"));
        }
        if !graph.contains_vertex(source) {
            return Err(GraphError::VertexNotFound);
        }
        let source = source.clone();
        // The Java iterator detects this while relaxing. Eager validation lets
        // Rust's `Iterator::next` stay infallible without changing semantics
        // for any successful iteration.
        if graph.edges().any(|edge| {
            let weight = graph.weight_of(edge.borrow());
            weight.is_nan() || weight < 0.0
        }) {
            return Err(GraphError::InvalidWeight(
                crate::WeightProblem::NegativeOrNaN,
            ));
        }
        Ok(Self {
            graph,
            radius,
            distance: HashMap::from([(source.clone(), 0.0)]),
            predecessor: HashMap::new(),
            settled: HashSet::new(),
            nodes: vec![source.clone()],
            source,
            heap: BinaryHeap::from([Reverse((OrderedF64(0.0), 0))]),
        })
    }

    /// Return the current shortest-path tree. Paths to vertices which have
    /// not yet been yielded are tentative upper bounds, as in JGraphT.
    pub fn paths(&self) -> SingleSourcePaths<G::Vertex, G::Edge> {
        SingleSourcePaths {
            source: self.source.clone(),
            dist: self
                .distance
                .iter()
                .filter(|(_, distance)| **distance <= self.radius)
                .map(|(vertex, distance)| (vertex.clone(), *distance))
                .collect(),
            pred: self
                .predecessor
                .iter()
                .filter(|(vertex, _)| self.distance[*vertex] <= self.radius)
                .map(|(vertex, (edge, parent))| (vertex.clone(), (edge.clone(), parent.clone())))
                .collect(),
        }
    }

    /// Return all paths using the traditional representation of the shortest path tree, which stores
    /// for each vertex (a) the distance of the path from the source vertex and (b) the last edge
    /// used to reach the vertex from the source vertex.
    ///
    /// Only the paths to vertices which are already returned by the iterator will be shortest paths.
    /// Additional paths to vertices which are not yet returned (settled) by the iterator might be
    /// included with the following properties: the distance will be an upper bound on the actual
    /// shortest path and the distance will be inside the radius of the search.
    pub fn distance_and_predecessor_map(&self) -> HashMap<G::Vertex, (f64, Option<G::Edge>)> {
        self.distance
            .iter()
            .filter(|(_, distance)| **distance <= self.radius)
            .map(|(vertex, distance)| {
                (
                    vertex.clone(),
                    (
                        *distance,
                        self.predecessor.get(vertex).map(|(edge, _)| edge.clone()),
                    ),
                )
            })
            .collect()
    }

    fn discard_stale(&mut self) {
        while let Some(Reverse((OrderedF64(distance), index))) = self.heap.peek().copied() {
            let vertex = &self.nodes[index];
            if self.settled.contains(vertex) || distance > self.distance[vertex] {
                self.heap.pop();
            } else {
                break;
            }
        }
        if self
            .heap
            .peek()
            .is_some_and(|Reverse((OrderedF64(distance), _))| *distance > self.radius)
        {
            self.heap.clear();
        }
    }
}

impl<G: OutgoingGraph> Iterator for DijkstraClosestFirstIterator<'_, G> {
    type Item = G::Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        self.discard_stale();
        let Reverse((OrderedF64(distance), index)) = self.heap.pop()?;
        let vertex = self.nodes[index].clone();
        self.settled.insert(vertex.clone());

        for (edge, target) in self.graph.outgoing_of(&vertex) {
            if self.settled.contains(target) {
                continue;
            }
            let candidate = distance + self.graph.weight_of(edge);
            if candidate < self.distance.get(target).copied().unwrap_or(f64::INFINITY) {
                self.distance.insert(target.clone(), candidate);
                self.predecessor
                    .insert(target.clone(), (edge.clone(), vertex.clone()));
                self.nodes.push(target.clone());
                self.heap
                    .push(Reverse((OrderedF64(candidate), self.nodes.len() - 1)));
            }
        }

        Some(vertex)
    }
}
