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
