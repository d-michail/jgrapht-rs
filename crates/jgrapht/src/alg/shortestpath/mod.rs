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

//! Shortest path algorithms.
//!
//! Heap-based algorithms ([`Dijkstra`], [`AStar`], [`JohnsonShortestPaths`]) are generic over any
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

pub use crate::alg::metrics::GraphMeasurer;
pub use all_directed::{AllDirectedPaths, AllowAllPaths, PathValidator};
pub use alt::AltAdmissibleHeuristic;
pub use astar::AStar;
pub use bellman_ford::{BellmanFord, BellmanFordError, NegativeCycleDetected};
pub use bfs::BfsShortestPath;
pub use bidirectional::{
    BidirectionalAStarShortestPath, BidirectionalDijkstra, BidirectionalDijkstraShortestPath,
};
pub use closest_first::DijkstraClosestFirstIterator;
pub use contraction_hierarchy::{
    ContractionEdge, ContractionHierarchy, ContractionHierarchyBidirectionalDijkstra,
    ContractionHierarchyManyToManyShortestPaths, ContractionHierarchyPrecomputation,
    ContractionVertex, ManyToManyPaths,
};
pub use dag_all_paths::DagAllPathsCounter;
pub use delta_stepping::DeltaStepping;
pub use delta_stepping_parallel::ParallelDeltaSteppingShortestPath;
pub use dijkstra::Dijkstra;
pub use disjoint::{BhandariKDisjointShortestPaths, SuurballeKDisjointShortestPaths};
pub use eppstein::{EppsteinKShortestPath, EppsteinShortestPathIterator};
pub use floyd_warshall::FloydWarshallShortestPaths;
pub(crate) use floyd_warshall::floyd_warshall;
pub use int_vertex::{IntVertex, IntVertexDijkstraShortestPath};
pub use johnson::JohnsonShortestPaths;
pub use many_to_many::{DefaultManyToManyShortestPaths, DijkstraManyToManyShortestPaths};
pub use multi_objective::{
    LabelCorrectingMultiObjectiveShortestPath, MartinShortestPath, MultiObjectiveSingleSourcePaths,
};
pub use paths::{AllPairsPaths, ListSingleSourcePaths, SingleSourcePaths};
pub use transit_node_routing::{
    AccessVertex, AccessVertices, LocalityFilter, TransitNodeRouting,
    TransitNodeRoutingPrecomputation, TransitNodeRoutingShortestPath, VoronoiDiagram,
};
pub use tree_measurer::TreeMeasurer;
pub use yen::{
    AStarSpurEngine, BoundedPrunedYenKShortestPath, BoundedPrunedYenStats, DijkstraSpurEngine,
    SpurShortestPathEngine, YenKShortestPath, YenShortestPathIterator,
};

use crate::Result;
use crate::graph::Graph;
use crate::walk::GraphWalk;

/// A single-source shortest path algorithm bound to a graph.
pub trait ShortestPathAlgorithm<G: Graph> {
    /// Shortest paths from `source` to every reachable vertex.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`](crate::GraphError::VertexNotFound) when `source` is missing, an `InvalidWeight` error for a weight the algorithm cannot handle, and [`GraphError::NegativeCycle`](crate::GraphError::NegativeCycle) where negative cycles are detected.
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>>;

    /// Shortest path between two vertices, `Ok(None)` if unreachable.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`ShortestPathAlgorithm::single_source`] for either endpoint.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        Ok(self.single_source(source)?.path(sink))
    }

    /// Weight of the shortest path; `Ok(None)` if `sink` is unreachable from `source`.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`ShortestPathAlgorithm::path`].
    fn path_weight(&self, source: &G::Vertex, sink: &G::Vertex) -> Result<Option<f64>> {
        Ok(self.path(source, sink)?.map(|p| p.weight))
    }
}
