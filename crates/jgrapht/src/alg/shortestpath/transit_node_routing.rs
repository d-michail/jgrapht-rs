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
    ManyToManyPaths, parallel_chunks,
};
use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::util::heap::JavaPairingHeap;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use rheaps::{AddressableHeap, DecreaseKeyHeap};
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Special Voronoi diagram cell id to indicate, that a vertex does not belong to any cells. For
/// usual Voronoi cell the ids of contracted vertices are used. Once those ids are non-negative,
/// this value is guaranteed to be unique.
const NO_VORONOI_CELL: i64 = -1;

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
    forward: Vec<Vec<AccessVertex<V, E>>>,
    backward: Vec<Vec<AccessVertex<V, E>>>,
}

impl<V, E> AccessVertices<V, E> {
    /// Transit vertices reachable from `vertex_id` in the forward direction.
    pub fn forward(&self, vertex_id: usize) -> &[AccessVertex<V, E>] {
        self.forward.get(vertex_id).map_or(&[], Vec::as_slice)
    }

    /// Transit vertices from which `vertex_id` is reachable.
    pub fn backward(&self, vertex_id: usize) -> &[AccessVertex<V, E>] {
        self.backward.get(vertex_id).map_or(&[], Vec::as_slice)
    }
}

/// Voronoi diagram of the contraction graph with the transit vertices as cell centers. The id of
/// a cell is the hierarchy id of its center.
#[derive(Debug, Clone)]
pub struct VoronoiDiagram {
    cells: Vec<i64>,
}

impl VoronoiDiagram {
    /// Id of the Voronoi cell of `vertex_id`, `-1` if the vertex belongs to no cell.
    pub fn voronoi_cell_id(&self, vertex_id: usize) -> i64 {
        self.cells[vertex_id]
    }

    /// Id (hierarchy id of the center) of the cell of `vertex_id`, `None` if the vertex belongs
    /// to no cell.
    pub fn cell(&self, vertex_id: usize) -> Option<usize> {
        self.cells
            .get(vertex_id)
            .and_then(|&c| usize::try_from(c).ok())
    }
}

/// Locality filter of the transit node routing. It is constructed using Voronoi cells visited by
/// the forward and backward searches in the contraction hierarchy: `L(s, t)` is `true` when no
/// shortest path between `s` and `t` is guaranteed to contain a transit vertex. False positives
/// are possible.
#[derive(Debug, Clone)]
pub struct LocalityFilter<V> {
    mapping: HashMap<V, usize>,
    visited_forward_cells: Vec<HashSet<i64>>,
    visited_backward_cells: Vec<HashSet<i64>>,
}

impl<V: Eq + std::hash::Hash> LocalityFilter<V> {
    /// Whether the pair `(source, sink)` is local, i.e. the query has to be answered by the
    /// fallback algorithm. Vertices unknown to the filter are never local.
    pub fn is_local(&self, source: &V, sink: &V) -> bool {
        let (Some(&source), Some(&sink)) = (self.mapping.get(source), self.mapping.get(sink))
        else {
            return false;
        };
        let source_cells = &self.visited_forward_cells[source];
        let sink_cells = &self.visited_backward_cells[sink];
        if source_cells.contains(&NO_VORONOI_CELL) || sink_cells.contains(&NO_VORONOI_CELL) {
            return true;
        }
        let (smaller, larger) = if source_cells.len() <= sink_cells.len() {
            (source_cells, sink_cells)
        } else {
            (sink_cells, source_cells)
        };
        smaller.iter().any(|cell| larger.contains(cell))
    }
}

struct RoutingData<'g, G: Graph> {
    hierarchy: ContractionHierarchy<'g, G>,
    transit_vertex_ids: Vec<usize>,
    transit_vertices: HashSet<G::Vertex>,
    transit_paths: ManyToManyPaths<G::Vertex, G::Edge>,
    access_vertices: AccessVertices<G::Vertex, G::Edge>,
    locality_filter: LocalityFilter<G::Vertex>,
    voronoi_diagram: VoronoiDiagram,
}

/// Materialized data used by transit-node routing queries: the transit vertices, the paths between
/// them, the access vertices, the Voronoi diagram and the locality filter. Cloning is cheap.
pub struct TransitNodeRouting<'g, G: Graph> {
    data: Arc<RoutingData<'g, G>>,
}

impl<G: Graph> Clone for TransitNodeRouting<'_, G> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}

impl<'g, G: Graph> TransitNodeRouting<'g, G> {
    /// Contraction hierarchy used by this routing data.
    pub fn contraction_hierarchy(&self) -> &ContractionHierarchy<'g, G> {
        &self.data.hierarchy
    }
    /// IDs of the selected hierarchy vertices, in decreasing contraction-level order.
    pub fn transit_vertex_ids(&self) -> &[usize] {
        &self.data.transit_vertex_ids
    }
    /// Selected original-graph vertices.
    pub fn transit_vertices(&self) -> &HashSet<G::Vertex> {
        &self.data.transit_vertices
    }
    /// Precomputed paths among all transit vertices.
    pub fn transit_paths(&self) -> &ManyToManyPaths<G::Vertex, G::Edge> {
        &self.data.transit_paths
    }
    /// Forward and backward access vertices.
    pub fn access_vertices(&self) -> &AccessVertices<G::Vertex, G::Edge> {
        &self.data.access_vertices
    }
    /// Locality filter used by queries.
    pub fn locality_filter(&self) -> &LocalityFilter<G::Vertex> {
        &self.data.locality_filter
    }
    /// Voronoi diagram with the transit vertices as cell centers.
    pub fn voronoi_diagram(&self) -> &VoronoiDiagram {
        &self.data.voronoi_diagram
    }
}

/// Parallel implementation of the [transit node routing route planning precomputation
/// technique](https://en.wikipedia.org/wiki/Transit_node_routing).
///
/// This implementation is based on the `ContractionHierarchyPrecomputation`.
///
/// The precomputation algorithm is described in the article: Arz, Julian & Luxen, Dennis &
/// Sanders, Peter. (2013). Transit Node Routing Reconsidered. 7933. 10.1007/978-3-642-38527-8_7.
///
/// As mentioned is the original paper, TNR in itself is not a complete algorithm, but a framework
/// which is used to speed up shortest paths computations. Formally the framework consists of the
/// following parts:
///
/// * set $T ⊆ V$ of transit vertices;
/// * distance table $D_{T} : T × T → {\rm I\!R}^{+}_{0}$ of shortest path distances between the
///   transit vertices;
/// * forward (backward) access vertex mapping $A^{↑} : V → 2^{T}$ ($A^{↓} : V → 2^{T}$). For any
///   shortest $s$–$t$-path $P$ containing transit vertices, $A^{↑}(s)$ ($A^{↓}(t)$) must contain the
///   first (last) transit vertex on $P$;
/// * locality filter $L : V × V → \{true, false\}$. $L(s, t)$ must be $true$ when no shortest path
///   between $s$ and $t$ contains a transit vertex. False positives are allowed, i.e., $L(s, t)$ may
///   sometimes be $true$ even when a shortest path contains a transit vertex.
///
/// To implement the TNR framework means to define how to select transit vertices and how to compute
/// distance table $D_{T}$, access vertices and locality filter. This implementation selects transit
/// vertices to be to $k$ vertices form the contraction hierarchy. For the details of how other parts
/// of this TNR work please refer to the original paper.
///
/// # Rust notes
///
/// Java's `ThreadPoolExecutor` becomes a worker-thread count. The Voronoi diagram, the transit
/// vertex paths, and the access vertices and locality filter of every vertex follow Java. The
/// forward and backward searches of the access vertex computation visit every vertex once
/// (Java's queue may contain a vertex several times, which does not change the result), and the
/// vertex sets are iterated in hierarchy id order (Java: `HashSet` order), which only decides
/// between access vertices pruning each other at equal weight.
pub struct TransitNodeRoutingPrecomputation<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
    transit_vertex_count: usize,
    parallelism: usize,
}

impl<'h, 'g, G: Graph> TransitNodeRoutingPrecomputation<'h, 'g, G> {
    /// Constructs an instance for the given contraction hierarchy, which selects `floor(sqrt(|V|))`
    /// transit vertices and uses all available processors.
    pub fn new(hierarchy: &'h ContractionHierarchy<'g, G>) -> Self {
        Self {
            hierarchy,
            transit_vertex_count: (hierarchy.vertices().len() as f64).sqrt() as usize,
            parallelism: std::thread::available_parallelism().map_or(1, |p| p.get()),
        }
    }

    /// Sets the number of highest-level vertices used as transit vertices.
    pub fn with_transit_vertex_count(mut self, count: usize) -> Result<Self> {
        if count > self.hierarchy.vertices().len() {
            return Err(GraphError::InvalidParameter {
                name: "transit_vertex_count",
                requirement: crate::ParameterRequirement::AtMost(self.hierarchy.vertices().len() as u64),
            });
        }
        self.transit_vertex_count = count;
        Ok(self)
    }

    /// Sets the number of worker threads (at least one).
    pub fn with_parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = parallelism.max(1);
        self
    }
}

/// Everything computed for one vertex of the contraction graph.
struct VertexResult<V, E> {
    vertex: usize,
    forward_access: Vec<AccessVertex<V, E>>,
    backward_access: Vec<AccessVertex<V, E>>,
    forward_cells: HashSet<i64>,
    backward_cells: HashSet<i64>,
}

impl<'h, 'g, G> TransitNodeRoutingPrecomputation<'h, 'g, G>
where
    G: Graph + Sync,
    G::Vertex: Send + Sync,
    G::Edge: Send + Sync,
{
    /// Computes transit node routing based on the contraction hierarchy.
    pub fn compute_transit_node_routing(&self) -> Result<TransitNodeRouting<'g, G>> {
        let h = self.hierarchy;
        let n = h.vertices().len();

        // select top k vertices in the contraction hierarchy as transit vertices
        let mut transit_vertex_ids: Vec<usize> = (0..n)
            .filter(|&v| h.vertices()[v].contraction_level + self.transit_vertex_count >= n)
            .collect();
        transit_vertex_ids.sort_by_key(|&v| std::cmp::Reverse(h.vertices()[v].contraction_level));
        let is_transit: Vec<bool> = {
            let mut flags = vec![false; n];
            for &v in &transit_vertex_ids {
                flags[v] = true;
            }
            flags
        };
        let transit_vertices: HashSet<G::Vertex> = transit_vertex_ids
            .iter()
            .map(|&v| h.vertices()[v].vertex.clone())
            .collect();

        let voronoi_diagram = compute_voronoi_diagram(h, &transit_vertex_ids);

        // paths between the transit vertices
        let transit_paths = CHManyToManyShortestPaths::new(h).many_to_many(
            transit_vertices.iter().cloned(),
            transit_vertices.iter().cloned(),
        )?;

        // access vertices and locality filter, computed in parallel
        let mut workers = vec![(); self.parallelism];
        let results = parallel_chunks(&mut workers, 0, n, |_, chunk| {
            chunk
                .map(|v| self.compute_vertex(v, &is_transit, &voronoi_diagram, &transit_paths))
                .collect()
        });
        let mut forward = vec![Vec::new(); n];
        let mut backward = vec![Vec::new(); n];
        let mut forward_cells = vec![HashSet::new(); n];
        let mut backward_cells = vec![HashSet::new(); n];
        for result in results {
            let result = result?;
            forward[result.vertex] = result.forward_access;
            backward[result.vertex] = result.backward_access;
            forward_cells[result.vertex] = result.forward_cells;
            backward_cells[result.vertex] = result.backward_cells;
        }

        Ok(TransitNodeRouting {
            data: Arc::new(RoutingData {
                hierarchy: h.clone(),
                transit_vertex_ids,
                transit_vertices,
                transit_paths,
                access_vertices: AccessVertices { forward, backward },
                locality_filter: LocalityFilter {
                    mapping: h.contraction_mapping().clone(),
                    visited_forward_cells: forward_cells,
                    visited_backward_cells: backward_cells,
                },
                voronoi_diagram,
            }),
        })
    }

    fn compute_vertex(
        &self,
        v: usize,
        is_transit: &[bool],
        voronoi: &VoronoiDiagram,
        transit_paths: &ManyToManyPaths<G::Vertex, G::Edge>,
    ) -> Result<VertexResult<G::Vertex, G::Edge>> {
        let h = self.hierarchy;
        let (forward_access, forward_cells) =
            hierarchy_bfs(h, v, is_transit, voronoi, |u| h.upward_outgoing(u));
        let (backward_access, backward_cells) =
            hierarchy_bfs(h, v, is_transit, voronoi, |u| h.downward_incoming(u));
        let vertex = h.vertices()[v].vertex.clone();
        let many_to_many = CHManyToManyShortestPaths::new(h);
        let access = |ids: &[usize]| -> Vec<G::Vertex> {
            ids.iter().map(|&u| h.vertices()[u].vertex.clone()).collect()
        };

        // forward access vertices
        let targets = access(&forward_access);
        let paths = many_to_many.many_to_many([vertex.clone()], targets.iter().cloned())?;
        let pruned = pruned_access_vertices(&targets, transit_paths, |a, b| {
            let (w1, w2) = (paths.weight(&vertex, a), paths.weight(&vertex, b));
            (w1, w2, true)
        });
        let forward_vertices = targets
            .iter()
            .filter(|u| !pruned.contains(*u))
            .filter_map(|u| {
                Some(AccessVertex {
                    vertex: u.clone(),
                    path: paths.path(&vertex, u)?.clone(),
                })
            })
            .collect();

        // backward access vertices
        let sources = access(&backward_access);
        let paths = many_to_many.many_to_many(sources.iter().cloned(), [vertex.clone()])?;
        let pruned = pruned_access_vertices(&sources, transit_paths, |a, b| {
            let (w1, w2) = (paths.weight(a, &vertex), paths.weight(b, &vertex));
            (w1, w2, false)
        });
        let backward_vertices = sources
            .iter()
            .filter(|u| !pruned.contains(*u))
            .filter_map(|u| {
                Some(AccessVertex {
                    vertex: u.clone(),
                    path: paths.path(u, &vertex)?.clone(),
                })
            })
            .collect();

        Ok(VertexResult {
            vertex: v,
            forward_access: forward_vertices,
            backward_access: backward_vertices,
            forward_cells,
            backward_cells,
        })
    }
}

/// Searches the contraction hierarchy from `start` over the edges given by `edges_of` without
/// traversing the edges leaving transit vertices. Returns the traversed transit vertices (access
/// vertices) and the Voronoi cells of the traversed non-transit vertices.
fn hierarchy_bfs<G: Graph>(
    h: &ContractionHierarchy<'_, G>,
    start: usize,
    is_transit: &[bool],
    voronoi: &VoronoiDiagram,
    edges_of: impl Fn(usize) -> Vec<(usize, usize)>,
) -> (Vec<usize>, HashSet<i64>) {
    let mut access_vertices = Vec::new();
    let mut visited_cells = HashSet::new();
    let mut seen = vec![false; h.vertices().len()];
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    while let Some(v) = queue.pop_front() {
        if is_transit[v] {
            access_vertices.push(v);
        } else {
            visited_cells.insert(voronoi.voronoi_cell_id(v));
            for (_, successor) in edges_of(v) {
                if !seen[successor] {
                    seen[successor] = true;
                    queue.push_back(successor);
                }
            }
        }
    }
    access_vertices.sort_unstable();
    (access_vertices, visited_cells)
}

/// Access vertices that are not needed because the path through another access vertex is not
/// longer. `weights(a, b)` returns the weights of the paths from (to) the vertex to (from) `a`
/// and `b`, and whether the paths are forward.
fn pruned_access_vertices<V: Clone + Eq + std::hash::Hash, E: Clone>(
    vertices: &[V],
    transit_paths: &ManyToManyPaths<V, E>,
    weights: impl Fn(&V, &V) -> (f64, f64, bool),
) -> HashSet<V> {
    let mut result = HashSet::new();
    for v1 in vertices {
        if result.contains(v1) {
            continue;
        }
        for v2 in vertices {
            if v1 != v2 && !result.contains(v2) {
                let (w1, w2, forward) = weights(v1, v2);
                let prune = if forward {
                    w1 + transit_paths.weight(v1, v2) <= w2
                } else {
                    transit_paths.weight(v2, v1) + w1 <= w2
                };
                if prune {
                    result.insert(v2.clone());
                }
            }
        }
    }
    result
}

/// Computes the Voronoi diagram of the contraction graph with the transit vertices as cell
/// centers. Runs Dijkstra's algorithm with multiple sources on the reversed graph, without the
/// shortcuts, starting at the cell centers. For each vertex it maintains the distance to the
/// closest cell center and the id of this center.
fn compute_voronoi_diagram<G: Graph>(
    h: &ContractionHierarchy<'_, G>,
    transit_vertex_ids: &[usize],
) -> VoronoiDiagram {
    type Handle = <JavaPairingHeap<OrderedF64, usize> as AddressableHeap<OrderedF64, usize>>::Handle;
    let n = h.vertices().len();
    let mut cells = vec![NO_VORONOI_CELL; n];
    let mut distance_to_center = vec![f64::INFINITY; n];
    let mut heap: JavaPairingHeap<OrderedF64, usize> = JavaPairingHeap::default();
    let mut seen: HashMap<usize, (f64, Handle)> = HashMap::new();

    let mut update_distance = |heap: &mut JavaPairingHeap<OrderedF64, usize>,
                               cells: &mut Vec<i64>,
                               distance_to_center: &mut Vec<f64>,
                               vertex: usize,
                               predecessor: usize,
                               distance: f64| {
        let visit = |cells: &mut Vec<i64>, distance_to_center: &mut Vec<f64>| {
            cells[vertex] = if vertex == predecessor {
                vertex as i64
            } else {
                cells[predecessor]
            };
            distance_to_center[vertex] = distance;
        };
        match seen.get_mut(&vertex) {
            None => {
                let handle = heap.insert(OrderedF64(distance), vertex);
                seen.insert(vertex, (distance, handle));
                visit(cells, distance_to_center);
            }
            Some((key, handle)) if distance < *key => {
                *key = distance;
                let _ = heap.decrease_key(*handle, OrderedF64(distance));
                visit(cells, distance_to_center);
            }
            Some(_) => {}
        }
    };

    for &transit in transit_vertex_ids {
        update_distance(&mut heap, &mut cells, &mut distance_to_center, transit, transit, 0.0);
    }
    while let Some((OrderedF64(distance), v)) = heap.pop() {
        // the reversed graph without shortcuts: original edges entering v
        for &e in h.incoming_edges(v) {
            let edge = &h.edges()[e];
            if edge.edge.is_none() {
                continue;
            }
            let updated = distance + edge.weight;
            if updated < distance_to_center[edge.source] {
                update_distance(
                    &mut heap,
                    &mut cells,
                    &mut distance_to_center,
                    edge.source,
                    v,
                    updated,
                );
            }
        }
    }
    VoronoiDiagram { cells }
}

enum Source<'g, G: Graph> {
    Precomputed(TransitNodeRouting<'g, G>),
    Lazy {
        graph: &'g G,
        parallelism: usize,
        routing: OnceCell<TransitNodeRouting<'g, G>>,
    },
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
/// [`new`](Self::new) builds the algorithm for a graph and precomputes lazily, as Java's graph
/// constructor does; [`from_routing`](Self::from_routing) uses existing routing data.
pub struct TransitNodeRoutingShortestPath<'g, G: Graph> {
    source: Source<'g, G>,
}

impl<'g, G: Graph> TransitNodeRoutingShortestPath<'g, G> {
    /// Constructs a new instance of the algorithm for a given `graph`. The precomputation is
    /// performed when the first query is made or when
    /// [`perform_precomputation`](Self::perform_precomputation) is called.
    pub fn new(graph: &'g G) -> Self {
        Self::with_parallelism(
            graph,
            std::thread::available_parallelism().map_or(1, |p| p.get()),
        )
    }

    /// Like [`new`](Self::new), with `parallelism` worker threads for the precomputation.
    pub fn with_parallelism(graph: &'g G, parallelism: usize) -> Self {
        Self {
            source: Source::Lazy {
                graph,
                parallelism: parallelism.max(1),
                routing: OnceCell::new(),
            },
        }
    }

    /// Constructs an instance which answers queries from existing routing data.
    pub fn from_routing(routing: TransitNodeRouting<'g, G>) -> Self {
        Self {
            source: Source::Precomputed(routing),
        }
    }
}

impl<'g, G> TransitNodeRoutingShortestPath<'g, G>
where
    G: Graph + Sync,
    G::Vertex: Send + Sync,
    G::Edge: Send + Sync,
{
    /// Performs the precomputation if it has not been done yet.
    pub fn perform_precomputation(&self) -> Result<()> {
        self.routing().map(|_| ())
    }

    fn routing(&self) -> Result<&TransitNodeRouting<'g, G>> {
        match &self.source {
            Source::Precomputed(routing) => Ok(routing),
            Source::Lazy {
                graph,
                parallelism,
                routing,
            } => {
                if let Some(routing) = routing.get() {
                    return Ok(routing);
                }
                let hierarchy = super::ContractionHierarchyPrecomputation::with_parallelism(
                    *graph,
                    *parallelism,
                )
                .compute_contraction_hierarchy()?;
                let computed = TransitNodeRoutingPrecomputation::new(&hierarchy)
                    .with_parallelism(*parallelism)
                    .compute_transit_node_routing()?;
                Ok(routing.get_or_init(|| computed))
            }
        }
    }

    /// Computes a shortest path between `source` and `sink`.
    pub fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let routing = self.routing()?;
        let h = routing.contraction_hierarchy();
        let (source_id, sink_id) = Self::ids(routing, source, sink)?;
        if routing.locality_filter().is_local(source, sink) {
            return ContractionHierarchyBidirectionalDijkstra::new(h).path(source, sink);
        }
        let Some((first, third)) = Self::min_weight_access_vertices(routing, source_id, sink_id)
        else {
            return Ok(if source == sink {
                Some(GraphWalk::single(source.clone()))
            } else {
                None
            });
        };
        let second = routing
            .transit_paths()
            .path(&first.vertex, &third.vertex)
            .expect("access vertices are connected by a transit path");
        Ok(Some(merge_paths(&first.path, second, &third.path)))
    }

    /// Computes the weight of a shortest path between `source` and `sink`, infinity if there is
    /// none.
    pub fn path_weight(&self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let routing = self.routing()?;
        let h = routing.contraction_hierarchy();
        let (source_id, sink_id) = Self::ids(routing, source, sink)?;
        if routing.locality_filter().is_local(source, sink) {
            return Ok(ContractionHierarchyBidirectionalDijkstra::new(h)
                .path(source, sink)?
                .map_or(f64::INFINITY, |p| p.weight));
        }
        Ok(
            Self::min_weight_access_vertices(routing, source_id, sink_id).map_or(
                f64::INFINITY,
                |(first, third)| {
                    first.path.weight
                        + routing.transit_paths().weight(&first.vertex, &third.vertex)
                        + third.path.weight
                },
            ),
        )
    }

    fn ids(
        routing: &TransitNodeRouting<'g, G>,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<(usize, usize)> {
        let mapping = routing.contraction_hierarchy().contraction_mapping();
        Ok((
            *mapping.get(source).ok_or(GraphError::VertexNotFound)?,
            *mapping.get(sink).ok_or(GraphError::VertexNotFound)?,
        ))
    }

    /// The pair of forward access vertex of `source` and backward access vertex of `sink` with
    /// the minimum total weight.
    fn min_weight_access_vertices<'r>(
        routing: &'r TransitNodeRouting<'g, G>,
        source: usize,
        sink: usize,
    ) -> Option<(
        &'r AccessVertex<G::Vertex, G::Edge>,
        &'r AccessVertex<G::Vertex, G::Edge>,
    )> {
        let mut best = None;
        let mut minimum_weight = f64::INFINITY;
        for first in routing.access_vertices().forward(source) {
            for third in routing.access_vertices().backward(sink) {
                let weight = first.path.weight
                    + routing.transit_paths().weight(&first.vertex, &third.vertex)
                    + third.path.weight;
                if weight < minimum_weight {
                    minimum_weight = weight;
                    best = Some((first, third));
                }
            }
        }
        best
    }
}

impl<'g, G> ShortestPathAlgorithm<G> for TransitNodeRoutingShortestPath<'g, G>
where
    G: Graph + Sync,
    G::Vertex: Send + Sync,
    G::Edge: Send + Sync,
{
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let routing = self.routing()?;
        let graph = routing.contraction_hierarchy().graph();
        if !graph.contains_vertex(source) {
            return Err(GraphError::VertexNotFound);
        }
        let mut result = SingleSourcePaths {
            source: source.clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for target in graph.vertices() {
            if let Some(path) = TransitNodeRoutingShortestPath::path(self, source, target)? {
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

    /// Get a shortest path from a source vertex to a sink vertex.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        TransitNodeRoutingShortestPath::path(self, source, sink)
    }

    fn path_weight(&self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        TransitNodeRoutingShortestPath::path_weight(self, source, sink)
    }
}

/// Computes a path which consists of `first`, `second` and `third` paths.
fn merge_paths<V: Clone, E: Clone>(
    first: &GraphWalk<V, E>,
    second: &GraphWalk<V, E>,
    third: &GraphWalk<V, E>,
) -> GraphWalk<V, E> {
    let mut vertices: Vec<V> = first.vertices[..first.vertices.len() - 1].to_vec();
    vertices.extend(second.vertices.iter().cloned());
    vertices.extend(third.vertices.iter().skip(1).cloned());
    let mut edges = first.edges.clone();
    edges.extend(second.edges.iter().cloned());
    edges.extend(third.edges.iter().cloned());
    GraphWalk {
        vertices,
        edges,
        weight: first.weight + second.weight + third.weight,
    }
}
