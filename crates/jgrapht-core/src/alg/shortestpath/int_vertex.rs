use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Integer types accepted by [`IntVertexDijkstraShortestPath`].
pub trait IntVertex: Copy + Eq + std::hash::Hash {
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

/// Array-backed Dijkstra specialized for integer vertices.
pub struct IntVertexDijkstraShortestPath<'g, G: Graph>
where
    G::Vertex: IntVertex,
{
    graph: &'g G,
}

impl<'g, G: Graph> IntVertexDijkstraShortestPath<'g, G>
where
    G::Vertex: IntVertex,
{
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
        let mut vertices: Vec<_> = self.graph.vertices().cloned().collect();
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
        let remap: HashMap<_, _> = (!directly_indexed)
            .then(|| {
                vertices
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, vertex)| (vertex, index))
                    .collect()
            })
            .unwrap_or_default();
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
                let weight = self.graph.edge_weight(edge).unwrap_or(1.0);
                if weight.is_nan() || weight < 0.0 {
                    return Err(GraphError::InvalidArgument(
                        "negative or NaN edge weight".into(),
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

impl<G: Graph> ShortestPathAlgorithm<G> for IntVertexDijkstraShortestPath<'_, G>
where
    G::Vertex: IntVertex,
{
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        self.run(source, None)
    }

    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        Ok(self.run(source, Some(sink))?.path(sink))
    }
}
