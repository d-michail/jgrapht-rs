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

//! Transit-node routing built on top of a contraction hierarchy.

use super::contraction_hierarchy::{
    CHManyToManyShortestPaths, ContractionHierarchy, ContractionHierarchyBidirectionalDijkstra,
    ManyToManyPaths,
};
use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::Graph;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet};

/// A transit vertex together with the path to or from it.
#[derive(Debug, Clone)]
pub struct AccessVertex<V, E> {
    /// The original-graph transit vertex.
    pub vertex: V,
    /// Forward path to the transit vertex, or backward path from it.
    pub path: GraphWalk<V, E>,
}

/// Forward and backward access vertices for every contracted vertex.
#[derive(Debug, Clone)]
pub struct AccessVertices<V, E> {
    forward: HashMap<usize, Vec<AccessVertex<V, E>>>,
    backward: HashMap<usize, Vec<AccessVertex<V, E>>>,
}

impl<V, E> AccessVertices<V, E> {
    /// Transit vertices reachable from `vertex_id` in the forward direction.
    pub fn forward(&self, vertex_id: usize) -> &[AccessVertex<V, E>] {
        self.forward.get(&vertex_id).map_or(&[], Vec::as_slice)
    }

    /// Transit vertices from which `vertex_id` is reachable.
    pub fn backward(&self, vertex_id: usize) -> &[AccessVertex<V, E>] {
        self.backward.get(&vertex_id).map_or(&[], Vec::as_slice)
    }
}

/// Assignment of hierarchy vertices to their nearest transit vertex.
#[derive(Debug, Clone)]
pub struct VoronoiDiagram {
    cells: Vec<Option<usize>>,
}

impl VoronoiDiagram {
    /// Transit-cell index, or `None` for a vertex with no reachable transit vertex.
    pub fn cell(&self, vertex_id: usize) -> Option<usize> {
        self.cells.get(vertex_id).copied().flatten()
    }
}

/// Conservative locality filter.
///
/// Queries incident to a transit vertex are guaranteed non-local. Other queries use the
/// contraction-hierarchy fallback. This is deliberately conservative: false positives cost query
/// time but cannot change a shortest-path result.
#[derive(Debug, Clone)]
pub struct LocalityFilter<V> {
    transit_vertices: HashSet<V>,
}

impl<V: Eq + std::hash::Hash> LocalityFilter<V> {
    /// Whether the fallback hierarchy query must be used.
    pub fn is_local(&self, source: &V, sink: &V) -> bool {
        !self.transit_vertices.contains(source) && !self.transit_vertices.contains(sink)
    }
}

/// Materialized data used by transit-node routing queries.
pub struct TransitNodeRouting<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
    transit_vertex_ids: Vec<usize>,
    transit_vertices: HashSet<G::Vertex>,
    transit_paths: ManyToManyPaths<G::Vertex, G::Edge>,
    access_vertices: AccessVertices<G::Vertex, G::Edge>,
    locality_filter: LocalityFilter<G::Vertex>,
    voronoi_diagram: VoronoiDiagram,
}

impl<'h, 'g, G: Graph> TransitNodeRouting<'h, 'g, G> {
    /// Contraction hierarchy used by this routing data.
    pub fn contraction_hierarchy(&self) -> &'h ContractionHierarchy<'g, G> {
        self.hierarchy
    }
    /// IDs of the selected hierarchy vertices, in decreasing contraction-level order.
    pub fn transit_vertex_ids(&self) -> &[usize] {
        &self.transit_vertex_ids
    }
    /// Selected original-graph vertices.
    pub fn transit_vertices(&self) -> &HashSet<G::Vertex> {
        &self.transit_vertices
    }
    /// Precomputed paths among all transit vertices.
    pub fn transit_paths(&self) -> &ManyToManyPaths<G::Vertex, G::Edge> {
        &self.transit_paths
    }
    /// Forward and backward access vertices.
    pub fn access_vertices(&self) -> &AccessVertices<G::Vertex, G::Edge> {
        &self.access_vertices
    }
    /// Locality filter used by queries.
    pub fn locality_filter(&self) -> &LocalityFilter<G::Vertex> {
        &self.locality_filter
    }
    /// Nearest-transit Voronoi assignment.
    pub fn voronoi_diagram(&self) -> &VoronoiDiagram {
        &self.voronoi_diagram
    }
}

/// Precompute transit vertices, access vertices, transit paths and locality data.
pub struct TransitNodeRoutingPrecomputation<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
    transit_vertex_count: usize,
}

impl<'h, 'g, G: Graph> TransitNodeRoutingPrecomputation<'h, 'g, G> {
    /// Use JGraphT's default transit-set size, `floor(sqrt(|V|))`.
    pub fn new(hierarchy: &'h ContractionHierarchy<'g, G>) -> Self {
        Self {
            hierarchy,
            transit_vertex_count: (hierarchy.vertices().len() as f64).sqrt() as usize,
        }
    }

    /// Override the number of highest-level vertices used as transit vertices.
    pub fn with_transit_vertex_count(mut self, count: usize) -> Self {
        self.transit_vertex_count = count.min(self.hierarchy.vertices().len());
        self
    }

    /// Perform the transit-node preprocessing.
    pub fn compute_transit_node_routing(&self) -> Result<TransitNodeRouting<'h, 'g, G>> {
        let h = self.hierarchy;
        let mut transit_vertex_ids: Vec<_> = (0..h.vertices().len()).collect();
        transit_vertex_ids.sort_by_key(|&v| std::cmp::Reverse(h.vertices()[v].contraction_level));
        transit_vertex_ids.truncate(self.transit_vertex_count);
        let transit_vertices: HashSet<_> = transit_vertex_ids
            .iter()
            .map(|&v| h.vertices()[v].vertex.clone())
            .collect();

        let transit_paths = CHManyToManyShortestPaths::new(h).many_to_many(
            transit_vertices.iter().cloned(),
            transit_vertices.iter().cloned(),
        )?;
        let query = ContractionHierarchyBidirectionalDijkstra::new(h);
        let mut forward = HashMap::new();
        let mut backward = HashMap::new();
        let mut cells = vec![None; h.vertices().len()];

        for vertex in h.vertices() {
            let mut forward_access = Vec::new();
            let mut backward_access = Vec::new();
            let mut closest = (f64::INFINITY, None);
            for (cell, &transit_id) in transit_vertex_ids.iter().enumerate() {
                let transit = &h.vertices()[transit_id].vertex;
                if let Some(path) = query.path(&vertex.vertex, transit)? {
                    if path.weight < closest.0 {
                        closest = (path.weight, Some(cell));
                    }
                    forward_access.push(AccessVertex {
                        vertex: transit.clone(),
                        path,
                    });
                }
                if let Some(path) = query.path(transit, &vertex.vertex)? {
                    backward_access.push(AccessVertex {
                        vertex: transit.clone(),
                        path,
                    });
                }
            }
            // Stable weight order gives deterministic access-pair enumeration.
            forward_access.sort_by(|a, b| a.path.weight.total_cmp(&b.path.weight));
            backward_access.sort_by(|a, b| a.path.weight.total_cmp(&b.path.weight));
            forward.insert(vertex.vertex_id, forward_access);
            backward.insert(vertex.vertex_id, backward_access);
            cells[vertex.vertex_id] = closest.1;
        }

        let locality_filter = LocalityFilter {
            transit_vertices: transit_vertices.clone(),
        };
        Ok(TransitNodeRouting {
            hierarchy: h,
            transit_vertex_ids,
            transit_vertices,
            transit_paths,
            access_vertices: AccessVertices { forward, backward },
            locality_filter,
            voronoi_diagram: VoronoiDiagram { cells },
        })
    }
}

/// Implementation of the shortest paths algorithm based on `TransitNodeRoutingPrecomputation`.
///
/// The algorithm is originally described the article: Arz, Julian & Luxen, Dennis & Sanders,
/// Peter. (2013). Transit Node Routing Reconsidered. 7933. 10.1007/978-3-642-38527-8_7.
///
/// The shortest paths between vertices $u$ and $v$ is computed in the following way. First, a
/// locality filter is used to determine if the vertices are local to each other. If so, a fallback
/// shortest path algorithm is used to compute the path. Otherwise, there is a shortest path between
/// the vertices which contains a transit vertex. Therefore the forward access vertices of $u$ and
/// backward access vertices of $v$ are inspected to find a pair of such access vertices $(a_u, a_v)$
/// so that the value of $d(u,a_u) + d(a_u, a_v) + d(a_u, v)$ is minimum over all such pairs. Here
/// $d(s,t)$ is the distance from vertex $s$ to vertex $t$.
///
/// The algorithm is designed to operate on sparse graphs with low average outdegree. Comparing to
/// `ContractionHierarchyBidirectionalDijkstra` it uses significantly more time on the
/// precomputation stage. Because of that it makes sense to use this algorithm on large instances
/// (i.e. with more than 10.000 vertices), where it shows substantially better performance results
/// than `ContractionHierarchyBidirectionalDijkstra`. Typically this algorithm is used as the
/// backend for large scale shortest path search engines, e.g.
/// [OpenStreetMap](https://www.openstreetmap.org).
///
/// The precomputation in this algorithm is performed in a lazy fashion. It can be performed by
/// directly calling the `#performPrecomputation()` method. Otherwise, this method is called
/// during the first call to either the `#getPath()` or `#getPathWeight()` methods.
///
/// # Rust notes
///
/// Shortest-path queries using precomputed transit-node routing data.
pub struct TransitNodeRoutingShortestPath<'r, 'h, 'g, G: Graph> {
    routing: &'r TransitNodeRouting<'h, 'g, G>,
}

impl<'r, 'h, 'g, G: Graph> TransitNodeRoutingShortestPath<'r, 'h, 'g, G> {
    /// Query existing routing data.
    pub fn new(routing: &'r TransitNodeRouting<'h, 'g, G>) -> Self {
        Self { routing }
    }

    /// Find a shortest path, falling back to a hierarchy query for local pairs.
    pub fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let h = self.routing.hierarchy;
        let &source_id = h
            .contraction_mapping()
            .get(source)
            .ok_or(GraphError::VertexNotFound)?;
        let &sink_id = h
            .contraction_mapping()
            .get(sink)
            .ok_or(GraphError::VertexNotFound)?;
        if self.routing.locality_filter.is_local(source, sink) {
            return ContractionHierarchyBidirectionalDijkstra::new(h).path(source, sink);
        }

        let mut best: Option<GraphWalk<G::Vertex, G::Edge>> = None;
        for first in self.routing.access_vertices.forward(source_id) {
            for third in self.routing.access_vertices.backward(sink_id) {
                let Some(second) = self
                    .routing
                    .transit_paths
                    .path(&first.vertex, &third.vertex)
                else {
                    continue;
                };
                let weight = first.path.weight + second.weight + third.path.weight;
                if best.as_ref().is_none_or(|path| weight < path.weight) {
                    best = Some(merge_paths(&first.path, second, &third.path, weight));
                }
            }
        }
        Ok(best)
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for TransitNodeRoutingShortestPath<'_, '_, '_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        if !self.routing.hierarchy.graph().contains_vertex(source) {
            return Err(GraphError::VertexNotFound);
        }
        let mut result = SingleSourcePaths {
            source: source.clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for target in self.routing.hierarchy.graph().vertices() {
            if let Some(path) = self.path(source, target)? {
                result.dist.insert(target.clone(), path.weight);
                for (vertices, edge) in path.vertices.windows(2).zip(path.edges) {
                    result
                        .pred
                        .insert(vertices[1].clone(), (edge, vertices[0].clone()));
                }
            }
        }
        Ok(result)
    }

    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        TransitNodeRoutingShortestPath::path(self, source, sink)
    }
}

fn merge_paths<V: Clone, E: Clone>(
    first: &GraphWalk<V, E>,
    second: &GraphWalk<V, E>,
    third: &GraphWalk<V, E>,
    weight: f64,
) -> GraphWalk<V, E> {
    let mut vertices = first.vertices.clone();
    vertices.extend(second.vertices.iter().skip(1).cloned());
    vertices.extend(third.vertices.iter().skip(1).cloned());
    let mut edges = first.edges.clone();
    edges.extend(second.edges.iter().cloned());
    edges.extend(third.edges.iter().cloned());
    GraphWalk {
        vertices,
        edges,
        weight,
    }
}
