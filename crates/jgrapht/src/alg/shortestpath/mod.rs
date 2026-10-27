//! Shortest path algorithms.
//!
//! Heap-based algorithms ([`Dijkstra`], [`AStar`], [`johnson`]) are generic over any
//! [`rheaps::DecreaseKeyHeap`]; the pairing heap is the default.

mod all_directed;
mod alt;
mod astar;
mod bellman_ford;
mod bfs;
mod bidirectional;
mod closest_first;
mod contraction_hierarchy;
mod dag_all_paths;
mod delta_stepping;
mod delta_stepping_parallel;
mod dijkstra;
pub(crate) mod disjoint;
mod eppstein;
mod floyd_warshall;
mod int_vertex;
mod johnson;
mod many_to_many;
mod multi_objective;
mod paths;
mod search;
mod transit_node_routing;
mod tree_measurer;
mod yen;

pub use all_directed::{AllDirectedPaths, AllowAllPaths, PathValidator};
pub use alt::ALTAdmissibleHeuristic;
pub use astar::{AStar, AStarShortestPath};
pub use bellman_ford::{
    BellmanFord, BellmanFordError, BellmanFordShortestPath, NegativeCycleDetected,
};
pub use bfs::{BFSShortestPath, BfsShortestPath};
pub use bidirectional::{
    BidirectionalAStarShortestPath, BidirectionalDijkstra, BidirectionalDijkstraShortestPath,
};
pub use closest_first::DijkstraClosestFirstIterator;
pub use contraction_hierarchy::{
    CHManyToManyShortestPaths, ContractionEdge, ContractionHierarchy,
    ContractionHierarchyBidirectionalDijkstra, ContractionHierarchyPrecomputation,
    ContractionVertex, ManyToManyPaths,
};
pub use dag_all_paths::DagAllPathsCounter;
pub use delta_stepping::DeltaStepping;
pub use delta_stepping_parallel::ParallelDeltaSteppingShortestPath;
pub type DeltaSteppingShortestPath<'g, G> = DeltaStepping<'g, G>;
pub use crate::alg::metrics::GraphMeasurer;
pub use dijkstra::{Dijkstra, DijkstraShortestPath};
pub use disjoint::{BhandariKDisjointShortestPaths, SuurballeKDisjointShortestPaths};
pub use eppstein::{EppsteinKShortestPath, EppsteinShortestPathIterator};
pub use floyd_warshall::{FloydWarshallShortestPaths, floyd_warshall};
pub use int_vertex::{IntVertex, IntVertexDijkstraShortestPath};
pub use johnson::{JohnsonShortestPaths, johnson};
pub use many_to_many::{DefaultManyToManyShortestPaths, DijkstraManyToManyShortestPaths};
pub use multi_objective::{
    LabelCorrectingMultiObjectiveShortestPath, MartinShortestPath, MultiObjectiveSingleSourcePaths,
};
pub use paths::{AllPairsPaths, ListSingleSourcePaths, SingleSourcePaths, TreeSingleSourcePaths};
pub use transit_node_routing::{
    AccessVertex, AccessVertices, LocalityFilter, TransitNodeRouting,
    TransitNodeRoutingPrecomputation, TransitNodeRoutingShortestPath, VoronoiDiagram,
};
pub use tree_measurer::TreeMeasurer;
pub use yen::{
    BoundedPrunedYenKShortestPath, BoundedPrunedYenStats, YenKShortestPath,
    YenShortestPathIterator, yen_k_shortest_paths,
};

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
