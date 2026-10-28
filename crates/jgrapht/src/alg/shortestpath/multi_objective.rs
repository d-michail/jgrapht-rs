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

use crate::alg::util::Indexed;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

#[derive(Debug, Clone)]
/// Pareto-optimal paths from one source to every other vertex.
pub struct MultiObjectiveSingleSourcePaths<V, E> {
    source: V,
    paths: HashMap<V, Vec<GraphWalk<V, E>>>,
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone> MultiObjectiveSingleSourcePaths<V, E> {
    /// The vertex all paths start at.
    pub fn source(&self) -> &V {
        &self.source
    }

    /// The Pareto-optimal paths from the source to `target`; empty if there are none.
    pub fn paths(&self, target: &V) -> &[GraphWalk<V, E>] {
        self.paths.get(target).map_or(&[], Vec::as_slice)
    }
}

struct Label {
    vertex: usize,
    value: Vec<f64>,
    previous: Option<usize>,
    edge: Option<usize>,
}

fn dominates(left: &[f64], right: &[f64]) -> bool {
    let mut strict = false;
    for (&a, &b) in left.iter().zip(right) {
        if a > b {
            return false;
        }
        strict |= a < b;
    }
    strict
}

fn close(left: &[f64], right: &[f64], epsilon: f64) -> bool {
    left.iter()
        .zip(right)
        .all(|(&a, &b)| (a - b).abs() <= a.abs() * epsilon)
}

fn validate_weights<G: Graph, F: Fn(&G::Edge) -> Vec<f64>>(
    indexed: &Indexed<'_, G>,
    weights: &F,
) -> Result<usize> {
    let mut dimensions = None;
    for edge in &indexed.es {
        let value = weights(edge);
        if dimensions.is_some_and(|d| d != value.len())
            || value.iter().any(|x| x.is_nan() || *x < 0.0)
        {
            return Err(GraphError::InvalidParameter {
                name: "weight_function",
                requirement: crate::ParameterRequirement::Consistent,
            });
        }
        dimensions.get_or_insert(value.len());
    }
    Ok(dimensions.unwrap_or(0))
}

fn build_paths<G: Graph>(
    indexed: &Indexed<'_, G>,
    source: usize,
    labels: &[Label],
    node_labels: &[Vec<usize>],
) -> MultiObjectiveSingleSourcePaths<G::Vertex, G::Edge> {
    let mut paths = HashMap::new();
    for sink in 0..indexed.n() {
        let mut sink_paths = Vec::new();
        if sink == source {
            sink_paths.push(GraphWalk::single(indexed.vertex(source)));
        } else {
            for &label_index in &node_labels[sink] {
                let mut edges = Vec::new();
                let mut vertices = Vec::new();
                let mut cursor = Some(label_index);
                while let Some(index) = cursor {
                    let label = &labels[index];
                    vertices.push(indexed.vertex(label.vertex));
                    if let Some(edge) = label.edge {
                        edges.push(edge);
                    }
                    cursor = label.previous;
                }
                vertices.reverse();
                edges.reverse();
                let weight = edges.iter().map(|&edge| indexed.w[edge]).sum();
                sink_paths.push(GraphWalk::new_unchecked(
                    vertices,
                    edges.iter().map(|&edge| indexed.es[edge].clone()).collect(),
                    weight,
                ));
            }
        }
        paths.insert(indexed.vertex(sink), sink_paths);
    }
    MultiObjectiveSingleSourcePaths {
        source: indexed.vertex(source),
        paths,
    }
}

/// Martin's algorithm for the multi-objective shortest paths problem.
///
/// Martin's label setting algorithm is a multiple objective extension of Dijkstra's algorithm, where
/// the minimum operator is replaced by a dominance test. It computes a maximal complete set of
/// efficient paths when all the cost values are non-negative.
///
/// Note that the multi-objective shortest path problem is a well-known NP-hard problem.
///
/// # Rust notes
///
/// Martin's label-setting multi-objective shortest-path algorithm.
pub struct MartinShortestPath<'g, G: Graph, F> {
    graph: &'g G,
    edge_weight: F,
}

impl<'g, G: Graph, F: Fn(&G::Edge) -> Vec<f64>> MartinShortestPath<'g, G, F> {
    /// Martin's algorithm on `graph`, where `edge_weight` gives the vector of objective values of an
    /// edge.
    pub fn new(graph: &'g G, edge_weight: F) -> Self {
        Self { graph, edge_weight }
    }

    /// Returns the paths.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn paths_from(
        &self,
        source: &G::Vertex,
    ) -> Result<MultiObjectiveSingleSourcePaths<G::Vertex, G::Edge>> {
        let indexed = Indexed::new(self.graph);
        let source = *indexed.idx.get(source).ok_or(GraphError::VertexNotFound)?;
        let dimensions = validate_weights(&indexed, &self.edge_weight)?;
        let mut labels = vec![Label {
            vertex: source,
            value: vec![0.0; dimensions],
            previous: None,
            edge: None,
        }];
        let mut node_labels = vec![Vec::new(); indexed.n()];
        node_labels[source].push(0);
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((vec![OrderedF64(0.0); dimensions], 0usize)));
        while let Some(Reverse((_, label_index))) = heap.pop() {
            let vertex = labels[label_index].vertex;
            for &(target, edge) in &indexed.out[vertex] {
                let edge_value = (self.edge_weight)(&indexed.es[edge]);
                let value: Vec<_> = labels[label_index]
                    .value
                    .iter()
                    .zip(edge_value)
                    .map(|(&a, b)| a + b)
                    .collect();
                if node_labels[target]
                    .iter()
                    .any(|&old| dominates(&labels[old].value, &value))
                {
                    continue;
                }
                node_labels[target].retain(|&old| !dominates(&value, &labels[old].value));
                let next = labels.len();
                labels.push(Label {
                    vertex: target,
                    value: value.clone(),
                    previous: Some(label_index),
                    edge: Some(edge),
                });
                node_labels[target].push(next);
                heap.push(Reverse((value.into_iter().map(OrderedF64).collect(), next)));
            }
        }
        Ok(build_paths(&indexed, source, &labels, &node_labels))
    }

    /// Returns the paths between.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn paths_between(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths_from(source)?.paths(sink).to_vec())
    }
}

/// A label-correcting algorithm for the multi-objective shortest paths problem.
///
/// The algorithm is a multiple objective extension of the Bellman-Ford relaxation process. It
/// maintains a set of non-dominated labels for each vertex and iteratively propagates labels through
/// outgoing edges. A newly generated label is discarded if it is dominated by an existing label at
/// the same vertex. Existing labels dominated by the new label are removed.
///
/// Optionally, an approximation tolerance $\varepsilon$ may be provided. In that case labels which
/// are sufficiently close according to the tolerance are also discarded in order to reduce the
/// number of stored labels, and the algorithm computes an approximation of the Pareto set. With a
/// tolerance equal to zero the complete Pareto set is computed.
///
/// All objective values must be non-negative.
///
/// The label-correcting strategy is described in: A. J. V. Skriver and K. A. Andersen. (2000). A
/// label correcting approach for solving bicriterion shortest-path problems. Computers &
/// Operations Research. 27. 507-524. 10.1016/S0305-0548(99)00037-4.
///
/// The approximation of the Pareto set is described in: A. Warburton. (1987). Approximation of
/// Pareto Optima in Multiple-Objective, Shortest-Path Problems. Operations Research. 35. 70-79.
/// 10.1287/opre.35.1.70.
///
/// Note that the multi-objective shortest path problem is a well-known NP-hard problem.
///
/// # Rust notes
///
/// Label-correcting multi-objective shortest paths, with optional epsilon approximation.
pub struct LabelCorrectingMultiObjectiveShortestPath<'g, G: Graph, F> {
    graph: &'g G,
    edge_weight: F,
    epsilon: f64,
}

impl<'g, G: Graph, F: Fn(&G::Edge) -> Vec<f64>>
    LabelCorrectingMultiObjectiveShortestPath<'g, G, F>
{
    /// The label-correcting algorithm on `graph`, where `edge_weight` gives the vector of objective
    /// values of an edge.
    pub fn new(graph: &'g G, edge_weight: F) -> Self {
        Self {
            graph,
            edge_weight,
            epsilon: 0.0,
        }
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_epsilon(graph: &'g G, edge_weight: F, epsilon: f64) -> Result<Self> {
        if epsilon.is_nan() || epsilon < 0.0 {
            return Err(GraphError::must_be_non_negative("epsilon"));
        }
        Ok(Self {
            graph,
            edge_weight,
            epsilon,
        })
    }

    /// Returns the paths.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn paths_from(
        &self,
        source: &G::Vertex,
    ) -> Result<MultiObjectiveSingleSourcePaths<G::Vertex, G::Edge>> {
        let indexed = Indexed::new(self.graph);
        let source = *indexed.idx.get(source).ok_or(GraphError::VertexNotFound)?;
        let dimensions = validate_weights(&indexed, &self.edge_weight)?;
        let mut labels = vec![Label {
            vertex: source,
            value: vec![0.0; dimensions],
            previous: None,
            edge: None,
        }];
        let mut node_labels = vec![Vec::new(); indexed.n()];
        node_labels[source].push(0);
        let mut queue = VecDeque::from([0usize]);
        while let Some(label_index) = queue.pop_front() {
            let vertex = labels[label_index].vertex;
            for &(target, edge) in &indexed.out[vertex] {
                let edge_value = (self.edge_weight)(&indexed.es[edge]);
                let value: Vec<_> = labels[label_index]
                    .value
                    .iter()
                    .zip(edge_value)
                    .map(|(&a, b)| a + b)
                    .collect();
                if node_labels[target].iter().any(|&old| {
                    dominates(&labels[old].value, &value)
                        || (self.epsilon > 0.0 && close(&labels[old].value, &value, self.epsilon))
                }) {
                    continue;
                }
                node_labels[target].retain(|&old| !dominates(&value, &labels[old].value));
                let next = labels.len();
                labels.push(Label {
                    vertex: target,
                    value,
                    previous: Some(label_index),
                    edge: Some(edge),
                });
                node_labels[target].push(next);
                queue.push_back(next);
            }
        }
        Ok(build_paths(&indexed, source, &labels, &node_labels))
    }

    /// Returns the paths between.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn paths_between(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths_from(source)?.paths(sink).to_vec())
    }
}
