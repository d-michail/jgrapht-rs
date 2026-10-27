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

use crate::graph::DirectedGraph;
use crate::traverse::TopologicalOrder;
use crate::{GraphError, Result};
use dashu_int::UBig;
use std::collections::HashMap;

/// Counts directed paths in a DAG using dynamic programming in topological order.
pub struct DagAllPathsCounter;

impl DagAllPathsCounter {
    pub fn count_all_paths<G: DirectedGraph>(
        graph: &G,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<UBig> {
        if !graph.contains_vertex(source) || !graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        let mut order = TopologicalOrder::new(graph);
        let vertices: Vec<_> = order.by_ref().collect();
        if !order.is_complete() {
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
            for edge in graph.out_edges_of(vertices[index]).unwrap() {
                let child = graph.edge_target(edge).expect("edge in graph");
                ways[positions[child]] += &count;
            }
        }
        Ok(ways[target_index].clone())
    }
}
