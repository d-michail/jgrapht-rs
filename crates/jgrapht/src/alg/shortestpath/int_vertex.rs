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

use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Integer types accepted by [`IntVertexDijkstraShortestPath`].
pub trait IntVertex: Copy + Eq + std::hash::Hash {
    /// The index of this vertex in `0..vertex_count`, or `None` if it is outside that range.
    fn dense_index(self, vertex_count: usize) -> Option<usize>;
}

macro_rules! impl_int_vertex {
    ($($t:ty),* $(,)?) => {$ (
        impl IntVertex for $t {
            fn dense_index(self, vertex_count: usize) -> Option<usize> {
                usize::try_from(self).ok().filter(|&index| index < vertex_count)
            }
        }
    )* };
}
impl_int_vertex!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

/// Dijkstra Shortest Path implementation specialized for graphs with integer vertices.
///
/// This class avoids using hash tables when the vertices are numbered from $0$ to $n-1$ where $n$ is
/// the number of vertices of the graph. If vertices are not in this range, then they are mapped in
/// this range using an open addressing hash table (linear probing).
///
/// This implementation should be faster than our more generic one which is
/// `Dijkstra` since it avoids the extensive use of hash tables.
///
/// By default a 4-ary heap is used. The user is free to use a custom heap implementation during
/// construction time. Regarding the choice of heap, it is generally known that it depends on the
/// particular workload. For more details and experiments which can help someone make the choice,
/// read the following paper: Larkin, Daniel H., Siddhartha Sen, and Robert E. Tarjan. "A
/// back-to-basics empirical study of priority queues." 2014 Proceedings of the Sixteenth Workshop on
/// Algorithm Engineering and Experiments (ALENEX). Society for Industrial and Applied Mathematics,
/// 2014.
///
/// # Rust notes
///
/// Array-backed Dijkstra specialized for integer vertices.
pub struct IntVertexDijkstraShortestPath<'g, G: Graph>
where
    G::Vertex: IntVertex,
{
    graph: &'g G,
}

impl<'g, G: OutgoingGraph> IntVertexDijkstraShortestPath<'g, G>
where
    G::Vertex: IntVertex,
{
    /// Constructs a new instance of the algorithm for a given graph.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    fn run(
        &self,
        source: &G::Vertex,
        target: Option<&G::Vertex>,
    ) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        if !self.graph.contains_vertex(source)
            || target.is_some_and(|vertex| !self.graph.contains_vertex(vertex))
        {
            return Err(GraphError::VertexNotFound);
        }
        let mut vertices: Vec<_> = self.graph.vertices_cloned().collect();
        let n = vertices.len();
        let directly_indexed = {
            let mut seen = vec![false; n];
            vertices.iter().all(|&vertex| {
                vertex.dense_index(n).is_some_and(|index| {
                    let fresh = !seen[index];
                    seen[index] = true;
                    fresh
                })
            })
        };
        let remap: HashMap<_, _> = if directly_indexed {
            HashMap::new()
        } else {
            vertices
                .iter()
                .copied()
                .enumerate()
                .map(|(index, vertex)| (vertex, index))
                .collect()
        };
        if directly_indexed {
            vertices.sort_by_key(|vertex| vertex.dense_index(n).expect("dense integer vertex"));
        }
        let index_of = |vertex: G::Vertex| {
            if directly_indexed {
                vertex.dense_index(n).expect("dense integer vertex")
            } else {
                remap[&vertex]
            }
        };
        let source_index = index_of(*source);
        let target_index = target.map(|vertex| index_of(*vertex));
        let mut distance = vec![f64::INFINITY; n];
        let mut predecessor: Vec<Option<(G::Edge, usize)>> = vec![None; n];
        let mut settled = vec![false; n];
        let mut heap = BinaryHeap::new();
        distance[source_index] = 0.0;
        heap.push(Reverse((OrderedF64(0.0), source_index)));
        while let Some(Reverse((OrderedF64(current_distance), vertex_index))) = heap.pop() {
            if settled[vertex_index] || current_distance > distance[vertex_index] {
                continue;
            }
            settled[vertex_index] = true;
            if Some(vertex_index) == target_index {
                break;
            }
            let vertex = vertices[vertex_index];
            for (edge, neighbor) in self.graph.outgoing_of(&vertex) {
                let weight = self.graph.weight_of(edge);
                if weight.is_nan() || weight < 0.0 {
                    return Err(GraphError::InvalidWeight(
                        crate::WeightProblem::NegativeOrNaN,
                    ));
                }
                let neighbor_index = index_of(*neighbor);
                let candidate = current_distance + weight;
                if candidate < distance[neighbor_index] {
                    distance[neighbor_index] = candidate;
                    predecessor[neighbor_index] = Some((edge.clone(), vertex_index));
                    heap.push(Reverse((OrderedF64(candidate), neighbor_index)));
                }
            }
        }
        let mut paths = SingleSourcePaths {
            source: *source,
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for (index, &vertex) in vertices.iter().enumerate() {
            if distance[index].is_finite() {
                paths.dist.insert(vertex, distance[index]);
                if let Some((edge, parent)) = &predecessor[index] {
                    paths.pred.insert(vertex, (edge.clone(), vertices[*parent]));
                }
            }
        }
        Ok(paths)
    }
}

impl<G: OutgoingGraph> ShortestPathAlgorithm<G> for IntVertexDijkstraShortestPath<'_, G>
where
    G::Vertex: IntVertex,
{
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        self.run(source, None)
    }

    /// Get a shortest path from a source vertex to a sink vertex.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        Ok(self.run(source, Some(sink))?.path(sink))
    }
}
