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
use crate::graph::DirectedGraph;
use crate::{GraphError, Result};
use dashu_int::UBig;
use std::collections::{HashMap, VecDeque};

/// Counts the number of simple directed paths in a Directed Acyclic Graph (DAG) between a given
/// source and target using dynamic programming in topological order.
///
/// For large graphs the number of paths can be extremely large; values are returned as
/// `java.math.BigInteger`.
///
/// Usage constraints: the provided graph must be directed and acyclic. An
/// `IllegalArgumentException` is thrown otherwise.
///
/// # Rust notes
///
/// Counts directed paths in a DAG using dynamic programming in topological order.
pub struct DagAllPathsCounter;

impl DagAllPathsCounter {
    /// Count the number of simple directed paths from `source` to `target` in the given
    /// directed acyclic graph.
    ///
    /// Edge case semantics:
    ///
    /// - If `source` or `target` is not contained in the graph, throws
    /// `IllegalArgumentException`.
    /// - If `source.equals(target)`, the empty path (length 0) is counted as one path and
    /// paths that leave `source` and return to it cannot exist in a DAG, thus the result is
    /// 1.
    /// - If there is no path from `source` to `target`, returns
    /// `BigInteger#ZERO`.
    pub fn count_all_paths<G: DirectedGraph + IncidenceGraph>(
        graph: &G,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<UBig> {
        if !graph.contains_vertex(source) || !graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        let mut in_degree: HashMap<_, _> = graph
            .vertices()
            .map(|vertex| Ok((vertex, graph.in_degree_of(vertex))))
            .collect::<Result<_>>()?;
        let mut queue: VecDeque<_> = graph
            .vertices()
            .filter(|vertex| in_degree[vertex] == 0)
            .collect();
        let mut vertices = Vec::with_capacity(graph.vertex_count());
        while let Some(vertex) = queue.pop_front() {
            vertices.push(vertex);
            for edge in graph.out_edges_of(vertex) {
                let target = graph.edge_target(edge).expect("edge in graph");
                let degree = in_degree.get_mut(target).expect("vertex in graph");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(target);
                }
            }
        }
        if vertices.len() != graph.vertex_count() {
            return Err(GraphError::InvalidArgument(
                "graph must be acyclic (DAG)".into(),
            ));
        }
        let positions: HashMap<_, _> = vertices
            .iter()
            .enumerate()
            .map(|(index, &vertex)| (vertex, index))
            .collect();
        let source_index = positions[source];
        let target_index = positions[target];
        if source_index > target_index {
            return Ok(UBig::ZERO);
        }
        let mut ways = vec![UBig::ZERO; vertices.len()];
        ways[source_index] = UBig::ONE;
        for index in source_index..=target_index {
            let count = ways[index].clone();
            if count == UBig::ZERO {
                continue;
            }
            for edge in graph.out_edges_of(vertices[index]) {
                let child = graph.edge_target(edge).expect("edge in graph");
                ways[positions[child]] += &count;
            }
        }
        Ok(ways[target_index].clone())
    }
}
