//! Shortest path algorithms.
//!
//! Heap-based algorithms ([`Dijkstra`], [`AStar`], [`Johnson`]) are generic over any
//! [`rheaps::DecreaseKeyHeap`]; the pairing heap is the default.

mod astar;
mod bellman_ford;
mod bfs;
mod dijkstra;
mod floyd_warshall;
mod johnson;
mod paths;
mod search;
mod yen;

pub use astar::AStar;
pub use bellman_ford::BellmanFord;
pub use bfs::BfsShortestPath;
pub use dijkstra::Dijkstra;
pub use floyd_warshall::floyd_warshall;
pub use johnson::johnson;
pub use paths::{AllPairsPaths, SingleSourcePaths};
pub use yen::yen_k_shortest_paths;

use crate::Result;
use crate::graph::Graph;
use crate::walk::GraphWalk;

/// A single-source shortest path algorithm bound to a graph.
pub trait ShortestPathAlgorithm<G: Graph> {
    /// Shortest paths from `source` to every reachable vertex.
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>>;

    /// Shortest path between two vertices, `Ok(None)` if unreachable.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        Ok(self.single_source(source)?.path(sink))
    }

    /// Weight of the shortest path, `f64::INFINITY` if unreachable.
    fn path_weight(&self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        Ok(self.path(source, sink)?.map_or(f64::INFINITY, |p| p.weight))
    }
}
