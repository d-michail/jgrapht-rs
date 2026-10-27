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

//! Contraction hierarchies and hierarchy-backed shortest-path queries.
//!
//! The representation deliberately keeps shortcut edges separate from the input graph.  Every
//! shortcut stores the two hierarchy edges it bypasses, which makes query results unpackable into
//! the original edge and vertex types without requiring a mutable or cloneable graph.

use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A vertex in a contraction hierarchy.
#[derive(Debug, Clone)]
pub struct ContractionVertex<V> {
    /// Position in the hierarchy's vertex array.
    pub vertex_id: usize,
    /// The corresponding input vertex.
    pub vertex: V,
    /// Contraction order; larger levels are higher in the hierarchy.
    pub contraction_level: usize,
}

/// An edge in a contraction hierarchy.
#[derive(Debug, Clone)]
pub struct ContractionEdge<E> {
    /// Source hierarchy-vertex index.
    pub source: usize,
    /// Target hierarchy-vertex index.
    pub target: usize,
    /// Original edge, or `None` for a shortcut.
    pub edge: Option<E>,
    /// The two hierarchy edges bypassed by a shortcut.
    pub bypassed_edges: Option<(usize, usize)>,
    /// Whether this edge goes from a lower to a higher contraction level.
    pub is_upward: bool,
    /// Number of original edges represented by this edge.
    pub original_edges: usize,
    /// Edge weight in the overlay graph.
    pub weight: f64,
}

/// The result of contraction-hierarchy preprocessing.
#[derive(Debug)]
pub struct ContractionHierarchy<'g, G: Graph> {
    graph: &'g G,
    vertices: Vec<ContractionVertex<G::Vertex>>,
    mapping: HashMap<G::Vertex, usize>,
    edges: Vec<ContractionEdge<G::Edge>>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
}

impl<'g, G: Graph> ContractionHierarchy<'g, G> {
    /// The input graph.
    pub fn graph(&self) -> &'g G {
        self.graph
    }

    /// Vertices of the hierarchy, in input iteration order.
    pub fn vertices(&self) -> &[ContractionVertex<G::Vertex>] {
        &self.vertices
    }

    /// Edges of the directed, simple overlay graph (original arcs and shortcuts).
    pub fn edges(&self) -> &[ContractionEdge<G::Edge>] {
        &self.edges
    }

    /// Find the contracted representation of an input vertex.
    pub fn contraction_vertex(&self, vertex: &G::Vertex) -> Option<&ContractionVertex<G::Vertex>> {
        self.mapping.get(vertex).map(|&i| &self.vertices[i])
    }

    /// Mapping from input vertices to hierarchy-vertex indices.
    pub fn contraction_mapping(&self) -> &HashMap<G::Vertex, usize> {
        &self.mapping
    }

    /// Outgoing hierarchy-edge indices.
    pub fn outgoing_edges(&self, vertex_id: usize) -> &[usize] {
        &self.outgoing[vertex_id]
    }

    /// Incoming hierarchy-edge indices.
    pub fn incoming_edges(&self, vertex_id: usize) -> &[usize] {
        &self.incoming[vertex_id]
    }

    fn unpack_forward(
        &self,
        edge_id: usize,
        vertices: &mut Vec<G::Vertex>,
        edges: &mut Vec<G::Edge>,
    ) {
        let edge = &self.edges[edge_id];
        if let Some((first, second)) = edge.bypassed_edges {
            self.unpack_forward(first, vertices, edges);
            self.unpack_forward(second, vertices, edges);
        } else if let Some(original) = &edge.edge {
            edges.push(original.clone());
            vertices.push(self.vertices[edge.target].vertex.clone());
        }
    }

    fn unpack_path(
        &self,
        source: usize,
        hierarchy_edges: &[usize],
        weight: f64,
    ) -> GraphWalk<G::Vertex, G::Edge> {
        let mut vertices = vec![self.vertices[source].vertex.clone()];
        let mut edges = Vec::new();
        for &edge in hierarchy_edges {
            self.unpack_forward(edge, &mut vertices, &mut edges);
        }
        GraphWalk {
            vertices,
            edges,
            weight,
        }
    }
}

/// Implementation of the [contraction hierarchy route planning precomputation technique](https://en.wikipedia.org/wiki/Contraction_hierarchies).
///
/// The original algorithm is described the article: Robert Geisberger, Peter Sanders, Dominik
/// Schultes, and Daniel Delling. 2008. Contraction hierarchies: faster and simpler hierarchical
/// routing in road networks. In Proceedings of the 7th international conference on Experimental
/// algorithms (WEA'08), Catherine C. McGeoch (Ed.). Springer-Verlag, Berlin, Heidelberg, 319-333.
///
/// Parallel version of the algorithm is described in the article: Vetter, Christian. "Parallel
/// Time-Dependent Contraction Hierarchies." (2009).
///
/// This algorithm speeds up shortest paths computation by contracting graph vertices. To contract a
/// vertex means to remove it from the graph in such a way that shortest paths in the remaining
/// overlay graph are preserved. This property is achieved by replacing paths of the form $\langle u,
/// v, w\rangle$ by a shortcut edge $(u, w)$. Note that the shortcut $(u, w)$ is only required if
/// $\langle u, v, w\rangle$ is the only shortest path from $u$ to $w$.
///
/// Contraction is performed as follows. First a priority is computed for each vertex in the graph.
/// This implementation uses edge quotient, complexity quotient and hierarchical depth metrics for
/// computing priority. A hierarchy is then generated by iteratively contracting independent sets of
/// vertices. A vertex is independent iff it's priority is less than the priority of every vertex in
/// its 2-neighbourhood. A 2-neighbourhood of a vertex $v$ is defined as a set of vertices that are
/// reachable from $v$ using at most 2 hops. After contraction each vertex gets its unique
/// contraction level - its position in the computed hierarchy. Finally, after all vertices are
/// contracted each edge is set to be either upward if its source has lower level that its target, or
/// downward if vice versa.
///
/// Computing initial priorities, independent sets and shortcuts, updating neighbours priorities and
/// marking upward edges is performed in parallel what gives this implementation performance speedup
/// comparing to the sequential approach.
///
/// # Rust notes
///
/// Sequential contraction-hierarchy preprocessing.
///
/// JGraphT parallelizes independent parts of this procedure.  The Rust implementation uses the
/// same witness-search and shortcut rules sequentially, avoiding a mandatory thread-pool in the
/// public API while preserving the algorithm and its query complexity.
pub struct ContractionHierarchyPrecomputation<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> ContractionHierarchyPrecomputation<'g, G> {
    /// Prepare a hierarchy computation for `graph`.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    /// Build the hierarchy. Negative and NaN weights are rejected.
    pub fn compute_contraction_hierarchy(&self) -> Result<ContractionHierarchy<'g, G>> {
        Builder::new(self.graph)?.contract()
    }
}

struct Builder<'g, G: Graph> {
    graph: &'g G,
    vertices: Vec<ContractionVertex<G::Vertex>>,
    mapping: HashMap<G::Vertex, usize>,
    edges: Vec<ContractionEdge<G::Edge>>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    by_endpoints: HashMap<(usize, usize), usize>,
    active: Vec<bool>,
    depth: Vec<usize>,
}

impl<'g, G: Graph> Builder<'g, G> {
    fn new(graph: &'g G) -> Result<Self> {
        let vertices: Vec<_> = graph
            .vertices()
            .enumerate()
            .map(|(vertex_id, vertex)| ContractionVertex {
                vertex_id,
                vertex: vertex.clone(),
                contraction_level: usize::MAX,
            })
            .collect();
        let mapping = vertices
            .iter()
            .map(|v| (v.vertex.clone(), v.vertex_id))
            .collect::<HashMap<_, _>>();
        let n = vertices.len();
        let mut this = Self {
            graph,
            vertices,
            mapping,
            edges: Vec::new(),
            outgoing: vec![Vec::new(); n],
            incoming: vec![Vec::new(); n],
            by_endpoints: HashMap::new(),
            active: vec![true; n],
            depth: vec![0; n],
        };
        let undirected = !graph.graph_type().directed;
        for edge in graph.edges() {
            let source = graph.edge_source(edge).ok_or(GraphError::EdgeNotFound)?;
            let target = graph.edge_target(edge).ok_or(GraphError::EdgeNotFound)?;
            let source = this.mapping[source];
            let target = this.mapping[target];
            if source == target {
                continue;
            }
            let weight = graph.edge_weight(edge).unwrap_or(1.0);
            if weight.is_nan() || weight < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "negative or NaN edge weight".into(),
                ));
            }
            this.add_original(source, target, edge.clone(), weight);
            if undirected {
                this.add_original(target, source, edge.clone(), weight);
            }
        }
        Ok(this)
    }

    fn add_original(&mut self, source: usize, target: usize, original: G::Edge, weight: f64) {
        if let Some(&id) = self.by_endpoints.get(&(source, target)) {
            if weight < self.edges[id].weight {
                self.edges[id].weight = weight;
                self.edges[id].edge = Some(original);
                self.edges[id].bypassed_edges = None;
                self.edges[id].original_edges = 1;
            }
            return;
        }
        let id = self.edges.len();
        self.edges.push(ContractionEdge {
            source,
            target,
            edge: Some(original),
            bypassed_edges: None,
            is_upward: false,
            original_edges: 1,
            weight,
        });
        self.outgoing[source].push(id);
        self.incoming[target].push(id);
        self.by_endpoints.insert((source, target), id);
    }

    fn add_shortcut(&mut self, source: usize, target: usize, first: usize, second: usize) {
        let weight = self.edges[first].weight + self.edges[second].weight;
        let original_edges = self.edges[first].original_edges + self.edges[second].original_edges;
        if let Some(&id) = self.by_endpoints.get(&(source, target)) {
            if weight < self.edges[id].weight {
                self.edges[id].weight = weight;
                self.edges[id].edge = None;
                self.edges[id].bypassed_edges = Some((first, second));
                self.edges[id].original_edges = original_edges;
            }
            return;
        }
        let id = self.edges.len();
        self.edges.push(ContractionEdge {
            source,
            target,
            edge: None,
            bypassed_edges: Some((first, second)),
            is_upward: false,
            original_edges,
            weight,
        });
        self.outgoing[source].push(id);
        self.incoming[target].push(id);
        self.by_endpoints.insert((source, target), id);
    }

    fn active_incoming(&self, vertex: usize) -> Vec<usize> {
        self.incoming[vertex]
            .iter()
            .copied()
            .filter(|&e| self.active[self.edges[e].source])
            .collect()
    }

    fn active_outgoing(&self, vertex: usize) -> Vec<usize> {
        self.outgoing[vertex]
            .iter()
            .copied()
            .filter(|&e| self.active[self.edges[e].target])
            .collect()
    }

    /// Shortcuts required when `vertex` is removed from the current overlay.
    fn shortcuts(&self, vertex: usize) -> Vec<(usize, usize)> {
        let incoming = self.active_incoming(vertex);
        let outgoing = self.active_outgoing(vertex);
        let mut result = Vec::new();
        for &first in &incoming {
            let source = self.edges[first].source;
            for &second in &outgoing {
                let target = self.edges[second].target;
                if source == target {
                    continue;
                }
                let candidate = self.edges[first].weight + self.edges[second].weight;
                if self.witness_distance(source, target, vertex, candidate) > candidate {
                    result.push((first, second));
                }
            }
        }
        result
    }

    /// Bounded Dijkstra in the uncontracted overlay, excluding the vertex being considered.
    fn witness_distance(&self, source: usize, target: usize, excluded: usize, radius: f64) -> f64 {
        let mut dist = vec![f64::INFINITY; self.vertices.len()];
        let mut heap = BinaryHeap::new();
        dist[source] = 0.0;
        heap.push(Reverse((OrderedF64(0.0), source)));
        while let Some(Reverse((OrderedF64(distance), vertex))) = heap.pop() {
            if distance != dist[vertex] {
                continue;
            }
            if distance > radius || vertex == target {
                return distance;
            }
            for &edge in &self.outgoing[vertex] {
                let next = self.edges[edge].target;
                if next == excluded || !self.active[next] {
                    continue;
                }
                let updated = distance + self.edges[edge].weight;
                if updated < dist[next] && updated <= radius {
                    dist[next] = updated;
                    heap.push(Reverse((OrderedF64(updated), next)));
                }
            }
        }
        dist[target]
    }

    fn priority(&self, vertex: usize) -> (usize, usize, usize) {
        let removed = self.active_incoming(vertex).len() + self.active_outgoing(vertex).len();
        // Edge difference and hierarchy depth are the dominant terms in JGraphT's priority.
        let shortcuts = self.shortcuts(vertex).len();
        (
            shortcuts.saturating_mul(4).saturating_sub(removed),
            self.depth[vertex],
            vertex,
        )
    }

    fn contract(mut self) -> Result<ContractionHierarchy<'g, G>> {
        for level in 0..self.vertices.len() {
            let vertex = (0..self.vertices.len())
                .filter(|&v| self.active[v])
                .min_by_key(|&v| self.priority(v))
                .expect("one active vertex per contraction level");
            let shortcuts = self.shortcuts(vertex);
            for (first, second) in shortcuts {
                let source = self.edges[first].source;
                let target = self.edges[second].target;
                self.add_shortcut(source, target, first, second);
            }
            self.vertices[vertex].contraction_level = level;
            self.active[vertex] = false;

            let next_depth = self.depth[vertex] + 1;
            let neighbors: HashSet<_> = self.incoming[vertex]
                .iter()
                .map(|&e| self.edges[e].source)
                .chain(self.outgoing[vertex].iter().map(|&e| self.edges[e].target))
                .filter(|&v| self.active[v])
                .collect();
            for neighbor in neighbors {
                self.depth[neighbor] = self.depth[neighbor].max(next_depth);
            }
        }
        for edge in &mut self.edges {
            edge.is_upward = self.vertices[edge.source].contraction_level
                < self.vertices[edge.target].contraction_level;
        }
        Ok(ContractionHierarchy {
            graph: self.graph,
            vertices: self.vertices,
            mapping: self.mapping,
            edges: self.edges,
            outgoing: self.outgoing,
            incoming: self.incoming,
        })
    }
}

struct Frontier {
    dist: Vec<f64>,
    pred: Vec<Option<(usize, usize)>>,
    settled: Vec<bool>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl Frontier {
    fn new(size: usize, start: usize) -> Self {
        let mut dist = vec![f64::INFINITY; size];
        dist[start] = 0.0;
        Self {
            dist,
            pred: vec![None; size],
            settled: vec![false; size],
            heap: BinaryHeap::from([Reverse((OrderedF64(0.0), start))]),
        }
    }

    fn peek(&mut self) -> f64 {
        while let Some(&Reverse((OrderedF64(distance), vertex))) = self.heap.peek() {
            if self.settled[vertex] || distance != self.dist[vertex] {
                self.heap.pop();
            } else {
                return distance;
            }
        }
        f64::INFINITY
    }

    fn pop(&mut self) -> Option<(f64, usize)> {
        self.peek();
        let Reverse((OrderedF64(distance), vertex)) = self.heap.pop()?;
        self.settled[vertex] = true;
        Some((distance, vertex))
    }

    fn relax(&mut self, vertex: usize, distance: f64, edge: usize, parent: usize) {
        if distance < self.dist[vertex] {
            self.dist[vertex] = distance;
            self.pred[vertex] = Some((edge, parent));
            self.heap.push(Reverse((OrderedF64(distance), vertex)));
        }
    }
}

/// Implementation of the hierarchical query algorithm based on the bidirectional Dijkstra search.
/// This algorithm is designed to contracted graphs. The best speedup is achieved on sparse graphs
/// with low average outdegree.
///
/// The query algorithm is originally described the article: Robert Geisberger, Peter Sanders,
/// Dominik Schultes, and Daniel Delling. 2008. Contraction hierarchies: faster and simpler
/// hierarchical routing in road networks. In Proceedings of the 7th international conference on
/// Experimental algorithms (WEA'08), Catherine C. McGeoch (Ed.). Springer-Verlag, Berlin,
/// Heidelberg, 319-333.
///
/// During contraction graph is divided into 2 parts which are called upward and downward graphs.
/// Both parts have all vertices of the original graph. The upward graph ($G_{\uparrow}$)
/// contains only those edges which source has lower level than the target and vice versa for the
/// downward graph ($G_{\downarrow}$).
///
/// For the shortest path query from $s$ to $t$, a modified bidirectional Dijkstra shortest path
/// search is performed. The forward search from $s$ operates on $G_{\uparrow}$ and the backward
/// search from $t$ - on the $G_{\downarrow}$. In each direction only the edges of the corresponding
/// part of the graph are considered. Both searches eventually meet at the vertex $v$, which has the
/// highest level in the shortest path from $s$ to $t$. Whenever a search in one direction reaches a
/// vertex that has already been processed in other direction, a new candidate for a shortest path is
/// found. Search is aborted in one direction if the smallest element in the corresponding priority
/// queue is at least as large as the best candidate path found so far.
///
/// After computing a contracted path, the algorithm unpacks it recursively into the actual shortest
/// path using the bypassed edges stored in the contraction hierarchy graph.
///
/// There is a possibility to provide an already computed contraction for the graph. For now there is
/// no means to ensure that the specified contraction is correct, nor to fail-fast. If algorithm uses
/// an incorrect contraction, the results of the search are unpredictable.
///
/// Comparing to usual shortest path algorithm, as `DijkstraShortestPath`,
/// `AStarShortestPath`, etc., this algorithm spends time for computing contraction hierarchy
/// but offers significant speedup in shortest path query performance. Therefore it is efficient to
/// use it in order to compute many shortest path on a single graph. Furthermore, on small graphs
/// (i.e with less than 1.000 vertices) the overhead of precomputation is higher than the speed at
/// the stage of computing shortest paths. Typically this algorithm is used to gain speedup for
/// shortest path queries on graphs of middle and large size (i.e. starting at 1.000 vertices). If a
/// further query performance improvement is needed take a look at
/// `TransitNodeRoutingShortestPath`.
///
/// # Rust notes
///
/// Bidirectional Dijkstra restricted to the upward and downward hierarchy graphs.
pub struct ContractionHierarchyBidirectionalDijkstra<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
    radius: f64,
}

impl<'h, 'g, G: Graph> ContractionHierarchyBidirectionalDijkstra<'h, 'g, G> {
    /// Query an already-computed hierarchy.
    pub fn new(hierarchy: &'h ContractionHierarchy<'g, G>) -> Self {
        Self {
            hierarchy,
            radius: f64::INFINITY,
        }
    }

    /// Limit returned paths to the given radius.
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    /// Find an unpacked path in the input graph.
    pub fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let h = self.hierarchy;
        let &source = h.mapping.get(source).ok_or(GraphError::VertexNotFound)?;
        let &sink = h.mapping.get(sink).ok_or(GraphError::VertexNotFound)?;
        if source == sink {
            return Ok(Some(GraphWalk::single(h.vertices[source].vertex.clone())));
        }

        let n = h.vertices.len();
        let mut forward = Frontier::new(n, source);
        let mut backward = Frontier::new(n, sink);
        let mut best = f64::INFINITY;
        let mut meeting = None;

        loop {
            let f_min = forward.peek();
            let b_min = backward.peek();
            if f_min.is_infinite() && b_min.is_infinite() {
                break;
            }
            if f_min.min(b_min) >= best {
                break;
            }
            let use_forward = f_min <= b_min;
            if use_forward {
                let Some((distance, vertex)) = forward.pop() else {
                    continue;
                };
                if backward.dist[vertex].is_finite() && distance + backward.dist[vertex] < best {
                    best = distance + backward.dist[vertex];
                    meeting = Some(vertex);
                }
                for &edge in &h.outgoing[vertex] {
                    let e = &h.edges[edge];
                    if !e.is_upward {
                        continue;
                    }
                    let updated = distance + e.weight;
                    forward.relax(e.target, updated, edge, vertex);
                    if backward.dist[e.target].is_finite()
                        && updated + backward.dist[e.target] < best
                    {
                        best = updated + backward.dist[e.target];
                        meeting = Some(e.target);
                    }
                }
            } else {
                let Some((distance, vertex)) = backward.pop() else {
                    continue;
                };
                if forward.dist[vertex].is_finite() && distance + forward.dist[vertex] < best {
                    best = distance + forward.dist[vertex];
                    meeting = Some(vertex);
                }
                // Reverse traversal of original downward edges climbs the hierarchy.
                for &edge in &h.incoming[vertex] {
                    let e = &h.edges[edge];
                    if e.is_upward {
                        continue;
                    }
                    let updated = distance + e.weight;
                    backward.relax(e.source, updated, edge, vertex);
                    if forward.dist[e.source].is_finite() && updated + forward.dist[e.source] < best
                    {
                        best = updated + forward.dist[e.source];
                        meeting = Some(e.source);
                    }
                }
            }
        }
        let Some(meeting) = meeting else {
            return Ok(None);
        };
        if best > self.radius {
            return Ok(None);
        }
        let mut first = Vec::new();
        let mut vertex = meeting;
        while let Some((edge, parent)) = forward.pred[vertex] {
            first.push(edge);
            vertex = parent;
        }
        first.reverse();
        let mut hierarchy_edges = first;
        vertex = meeting;
        while let Some((edge, next)) = backward.pred[vertex] {
            hierarchy_edges.push(edge);
            vertex = next;
        }
        Ok(Some(h.unpack_path(source, &hierarchy_edges, best)))
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for ContractionHierarchyBidirectionalDijkstra<'_, '_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        if !self.hierarchy.graph.contains_vertex(source) {
            return Err(GraphError::VertexNotFound);
        }
        let mut result = SingleSourcePaths {
            source: source.clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for target in self.hierarchy.graph.vertices() {
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

    /// Get a shortest path from a source vertex to a sink vertex.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        ContractionHierarchyBidirectionalDijkstra::path(self, source, sink)
    }
}

/// Materialized shortest paths for a source/target subset.
#[derive(Debug, Clone)]
pub struct ManyToManyPaths<V, E> {
    sources: HashSet<V>,
    targets: HashSet<V>,
    paths: HashMap<(V, V), GraphWalk<V, E>>,
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone> ManyToManyPaths<V, E> {
    pub(crate) fn from_parts(
        sources: HashSet<V>,
        targets: HashSet<V>,
        paths: HashMap<(V, V), GraphWalk<V, E>>,
    ) -> Self {
        Self {
            sources,
            targets,
            paths,
        }
    }

    /// Sources used for this computation.
    pub fn sources(&self) -> &HashSet<V> {
        &self.sources
    }
    /// Targets used for this computation.
    pub fn targets(&self) -> &HashSet<V> {
        &self.targets
    }
    /// Path, or `None` when the target is unreachable.
    pub fn path(&self, source: &V, target: &V) -> Option<&GraphWalk<V, E>> {
        self.paths.get(&(source.clone(), target.clone()))
    }
    /// Path weight, or infinity when unreachable.
    pub fn weight(&self, source: &V, target: &V) -> f64 {
        self.path(source, target)
            .map_or(f64::INFINITY, |p| p.weight)
    }
}

/// Efficient algorithm for the many-to-many shortest paths problem based on contraction hierarchy.
///
/// The algorithm is originally described in the article: Sebastian Knopp, Peter Sanders, Dominik
/// Schultes, Frank Schulz, and Dorothea Wagner. 2007. Computing many-to-many shortest paths using
/// highway hierarchies. In Proceedings of the Meeting on Algorithm Engineering & Expermiments.
/// Society for Industrial and Applied Mathematics, Philadelphia, PA, USA, 36-45.
///
/// First contraction hierarchy is constructed. Then for each target vertex a backward single source
/// shortest paths search is performed on the contracted graph. During the searches a bucket $b(v)$
/// is associated with each vertex $v$ in the graph. A bucket stores a set of pairs $(t,d)$, where
/// $t$ is a target vertex current search is performed from and $d$ is the computed distance from $v$
/// to this target. Then a forward single source shortest paths search is performed from every source
/// vertex. When a search settles a vertex $v$ with distance $d(s,v)$, where $s$ is current source
/// vertex, its bucket is scanned. For each entry $(t,d)$ if $d(s,t) > d(s,v) + d$ values of paths
/// weight between $s$ and $t$ and its middle vertex is updated. The middle vertices are then used to
/// restored actual path from the information in the shortest paths trees.
///
/// Additionally if $|S| > |T|$ the algorithm is executed on the reversed graph. This allows to
/// reduce the number of buckets and optimize memory usage of the algorithm.
///
/// The efficiency of this algorithm is derived from the fact that contraction hierarchy produces
/// fairly small shortest paths trees. This allows to both speedup the computations and decrease
/// memory usage to store the paths. The bottleneck of the algorithm is the contraction hierarchy
/// computation, which can lead to significant overhead for dense graphs both in terms of running
/// time and space complexity. Therefore the ideal use cases for this algorithm are sparse graphs of
/// any size with low average out-degree of vertices.
///
/// # Rust notes
///
/// Many-to-many shortest paths over a precomputed contraction hierarchy.
pub struct CHManyToManyShortestPaths<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
}

impl<'h, 'g, G: Graph> CHManyToManyShortestPaths<'h, 'g, G> {
    /// Create a many-to-many query object.
    pub fn new(hierarchy: &'h ContractionHierarchy<'g, G>) -> Self {
        Self { hierarchy }
    }

    /// Compute paths for the Cartesian product of `sources` and `targets`.
    pub fn many_to_many<I, J>(
        &self,
        sources: I,
        targets: J,
    ) -> Result<ManyToManyPaths<G::Vertex, G::Edge>>
    where
        I: IntoIterator<Item = G::Vertex>,
        J: IntoIterator<Item = G::Vertex>,
    {
        let sources: HashSet<_> = sources.into_iter().collect();
        let targets: HashSet<_> = targets.into_iter().collect();
        if sources
            .iter()
            .any(|v| !self.hierarchy.graph.contains_vertex(v))
            || targets
                .iter()
                .any(|v| !self.hierarchy.graph.contains_vertex(v))
        {
            return Err(GraphError::VertexNotFound);
        }
        let query = ContractionHierarchyBidirectionalDijkstra::new(self.hierarchy);
        let mut paths = HashMap::new();
        for source in &sources {
            for target in &targets {
                if let Some(path) = query.path(source, target)? {
                    paths.insert((source.clone(), target.clone()), path);
                }
            }
        }
        Ok(ManyToManyPaths {
            sources,
            targets,
            paths,
        })
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for CHManyToManyShortestPaths<'_, '_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        ContractionHierarchyBidirectionalDijkstra::new(self.hierarchy).single_source(source)
    }

    /// Return the path from the `source` vertex to the `target` vertex. If no such
    /// path exists, null is returned.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        ContractionHierarchyBidirectionalDijkstra::new(self.hierarchy).path(source, sink)
    }
}
