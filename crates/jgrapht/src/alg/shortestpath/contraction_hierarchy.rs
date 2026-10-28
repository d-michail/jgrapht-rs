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
use crate::graph::GraphExt;
use crate::util::OrderedF64;
use crate::util::heap::JavaPairingHeap;
use crate::util::java::RandomSource;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use indexmap::IndexSet;
use rand::{SeedableRng, rngs::StdRng};
use rheaps::{AddressableHeap, DecreaseKeyHeap};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Deref;
use std::sync::Arc;

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

/// The data of a contraction hierarchy, shared by all clones of a [`ContractionHierarchy`].
#[derive(Debug)]
pub struct HierarchyData<G: Graph> {
    vertices: Vec<ContractionVertex<G::Vertex>>,
    mapping: HashMap<G::Vertex, usize>,
    edges: Vec<ContractionEdge<G::Edge>>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
}

/// The result of contraction-hierarchy preprocessing.
///
/// Cloning is cheap: clones share the hierarchy data, so routing objects can own the hierarchy
/// they were computed from.
pub struct ContractionHierarchy<'g, G: Graph> {
    graph: &'g G,
    data: Arc<HierarchyData<G>>,
}

impl<G: Graph> std::fmt::Debug for ContractionHierarchy<'_, G>
where
    HierarchyData<G>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractionHierarchy")
            .field("data", &self.data)
            .finish()
    }
}

impl<G: Graph> Clone for ContractionHierarchy<'_, G> {
    fn clone(&self) -> Self {
        Self {
            graph: self.graph,
            data: Arc::clone(&self.data),
        }
    }
}

impl<G: Graph> Deref for ContractionHierarchy<'_, G> {
    type Target = HierarchyData<G>;

    fn deref(&self) -> &HierarchyData<G> {
        &self.data
    }
}

impl<'g, G: Graph> ContractionHierarchy<'g, G> {
    /// The graph the hierarchy was built from.
    pub fn graph(&self) -> &'g G {
        self.graph
    }

    /// Vertices of the hierarchy; the position of a vertex is its `vertex_id`.
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

    /// Unpacks `edge` by recursively going from target to source, prepending to the path.
    pub fn unpack_backward(
        &self,
        edge: usize,
        vertices: &mut VecDeque<G::Vertex>,
        edges: &mut VecDeque<G::Edge>,
    ) {
        let e = &self.edges[edge];
        match (&e.bypassed_edges, &e.edge) {
            (Some((first, second)), _) => {
                self.unpack_backward(*second, vertices, edges);
                self.unpack_backward(*first, vertices, edges);
            }
            (None, Some(original)) => {
                vertices.push_front(self.vertices[e.source].vertex.clone());
                edges.push_front(original.clone());
            }
            (None, None) => {}
        }
    }

    /// Unpacks `edge` by recursively going from source to target, appending to the path.
    pub fn unpack_forward(
        &self,
        edge: usize,
        vertices: &mut VecDeque<G::Vertex>,
        edges: &mut VecDeque<G::Edge>,
    ) {
        let e = &self.edges[edge];
        match (&e.bypassed_edges, &e.edge) {
            (Some((first, second)), _) => {
                self.unpack_forward(*first, vertices, edges);
                self.unpack_forward(*second, vertices, edges);
            }
            (None, Some(original)) => {
                vertices.push_back(self.vertices[e.target].vertex.clone());
                edges.push_back(original.clone());
            }
            (None, None) => {}
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
/// Java's `ThreadPoolExecutor` becomes a worker-thread count: every parallel phase splits its
/// segment of vertices into the same `parallelism` chunks as Java and runs them on scoped threads.
/// Each chunk reads shared state and returns its results, which are applied sequentially in segment
/// order, so no locking is needed and the result does not depend on thread scheduling. As in
/// Java, every worker owns one random generator (created by the supplier, in worker order) that
/// supplies the tie-breaking values of the vertices in its chunk of the initial segment, so a
/// seeded supplier such as `|| StdRng::seed_from_u64(seed)` makes the hierarchy reproducible for
/// the same `parallelism`. The set of vertices that a vertex shortcut candidate is checked against is
/// iterated in insertion order (Java: `HashSet` order), which only changes the order in which the
/// shortcut edges of one vertex are added to the hierarchy.
pub struct ContractionHierarchyPrecomputation<'g, G: Graph> {
    graph: &'g G,
    parallelism: usize,
    random_supplier: RandomSupplier,
}

type RandomSupplier = Box<dyn Fn() -> Box<dyn RandomSource + Send>>;

impl<'g, G: Graph> ContractionHierarchyPrecomputation<'g, G> {
    /// Constructs a new instance of the algorithm for a given `graph`. The number of workers is
    /// the number of available processors and the tie-breaking random numbers are seeded from the
    /// clock.
    pub fn new(graph: &'g G) -> Self {
        let parallelism = std::thread::available_parallelism().map_or(1, |p| p.get());
        Self::with_parallelism(graph, parallelism)
    }

    /// Constructs a new instance of the algorithm for a given `graph` which uses `parallelism`
    /// worker threads (at least one).
    pub fn with_parallelism(graph: &'g G, parallelism: usize) -> Self {
        Self::with_random_supplier(graph, parallelism, StdRng::from_os_rng)
    }

    /// Constructs a new instance of the algorithm for a given `graph`, `parallelism` and
    /// `random_supplier`. The supplier is called once per worker and should return different
    /// generators, because they are used by different threads.
    pub fn with_random_supplier<R, F>(graph: &'g G, parallelism: usize, random_supplier: F) -> Self
    where
        R: RandomSource + Send + 'static,
        F: Fn() -> R + 'static,
    {
        Self {
            graph,
            parallelism: parallelism.max(1),
            random_supplier: Box::new(move || Box::new(random_supplier())),
        }
    }

    /// Computes contraction hierarchy for `graph`. Negative and NaN weights are rejected.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn contraction_hierarchy(&self) -> Result<ContractionHierarchy<'g, G>> {
        let (mut core, originals) = Core::fill(self.graph)?;
        let mut randoms: Vec<_> = (0..self.parallelism)
            .map(|_| (self.random_supplier)())
            .collect();
        core.contract_all(&mut randoms);
        Ok(core.into_hierarchy(self.graph, originals))
    }
}

/// An edge of the contraction graph; `original` indexes the input edge of a non-shortcut.
struct CoreEdge {
    source: usize,
    target: usize,
    weight: f64,
    original_edges: usize,
    original: Option<usize>,
    bypassed: Option<(usize, usize)>,
}

/// Contraction data of one vertex (Java: `VertexData`).
#[derive(Clone, Default)]
struct VertexData {
    depth: usize,
    random: i32,
    priority: f64,
    contracted: bool,
    independent: bool,
}

#[derive(Default)]
struct Statistics {
    added_contraction_edges: usize,
    removed_contraction_edges: usize,
    added_original_edges: usize,
    removed_original_edges: usize,
}

/// The contraction graph and the state of the contraction. Vertices are numbered in input
/// order; `order` is Java's `vertices` array (the position of a vertex in it is its Java
/// `vertexId`), partitioned into contracted and not yet contracted vertices.
struct Core {
    undirected: bool,
    edges: Vec<CoreEdge>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    by_endpoints: HashMap<(usize, usize), usize>,
    data: Vec<VertexData>,
    order: Vec<usize>,
    position: Vec<usize>,
    shortcuts: Vec<Vec<(usize, usize)>>,
    level: Vec<usize>,
}

/// Runs `work` on `parallelism` contiguous chunks of `[start, end)` and concatenates the results
/// in chunk order. Chunk `i` is `[start + len * i / p, start + len * (i + 1) / p)`, as in Java.
pub(super) fn parallel_chunks<S: Send, T: Send>(
    states: &mut [S],
    start: usize,
    end: usize,
    work: impl Fn(&mut S, std::ops::Range<usize>) -> Vec<T> + Sync,
) -> Vec<T> {
    let p = states.len();
    let len = end - start;
    let range = |i: usize| start + len * i / p..start + len * (i + 1) / p;
    if p == 1 {
        return work(&mut states[0], range(0));
    }
    let work = &work;
    std::thread::scope(|scope| {
        let handles: Vec<_> = states
            .iter_mut()
            .enumerate()
            .map(|(i, state)| {
                let chunk = range(i);
                scope.spawn(move || work(state, chunk))
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("contraction worker panicked"))
            .collect()
    })
}

impl Core {
    /// Fills the contraction graph. If there exist multiple edges between two vertices in the
    /// original graph, the shortest is added. Self loops are ignored. If the original graph is
    /// undirected, each edge becomes two directed edges.
    fn fill<G: Graph>(graph: &G) -> Result<(Self, Vec<G::Edge>)> {
        let all: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let ids: HashMap<&G::Vertex, usize> = all.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let n = ids.len();
        let mut core = Core {
            undirected: !graph.graph_type().directed,
            edges: Vec::new(),
            outgoing: vec![Vec::new(); n],
            incoming: vec![Vec::new(); n],
            by_endpoints: HashMap::new(),
            data: vec![VertexData::default(); n],
            order: (0..n).collect(),
            position: (0..n).collect(),
            shortcuts: vec![Vec::new(); n],
            level: vec![0; n],
        };
        let mut originals = Vec::new();
        for edge in graph.edges() {
            let edge = edge.borrow();
            let source = ids[graph.edge_source(edge).ok_or(GraphError::EdgeNotFound)?];
            let target = ids[graph.edge_target(edge).ok_or(GraphError::EdgeNotFound)?];
            if source == target {
                continue;
            }
            let weight = graph.weight_of(edge);
            if weight.is_nan() || weight < 0.0 {
                return Err(GraphError::InvalidWeight(
                    crate::WeightProblem::NegativeOrNaN,
                ));
            }
            let index = originals.len();
            originals.push(edge.clone());
            core.add_original(source, target, index, weight);
            if core.undirected {
                core.add_original(target, source, index, weight);
            }
        }
        Ok((core, originals))
    }

    fn add_original(&mut self, source: usize, target: usize, original: usize, weight: f64) {
        if let Some(&id) = self.by_endpoints.get(&(source, target)) {
            if weight < self.edges[id].weight {
                self.edges[id].weight = weight;
                self.edges[id].original = Some(original);
            }
            return;
        }
        self.push_edge(CoreEdge {
            source,
            target,
            weight,
            original_edges: 1,
            original: Some(original),
            bypassed: None,
        });
    }

    fn push_edge(&mut self, edge: CoreEdge) {
        let id = self.edges.len();
        self.outgoing[edge.source].push(id);
        self.incoming[edge.target].push(id);
        self.by_endpoints.insert((edge.source, edge.target), id);
        self.edges.push(edge);
    }

    /// Performs the contraction of all vertices (Java: `computeContractionHierarchy`).
    fn contract_all(&mut self, randoms: &mut [Box<dyn RandomSource + Send>]) {
        let n = self.order.len();
        // compute initial priorities in parallel
        let initial = parallel_chunks(randoms, 0, n, |random, chunk| {
            chunk
                .map(|pos| {
                    let vertex = self.order[pos];
                    let mut data = VertexData {
                        random: random.next_int(),
                        ..VertexData::default()
                    };
                    data.priority = self.priority(vertex, data.depth);
                    (vertex, data)
                })
                .collect()
        });
        for (vertex, data) in initial {
            self.data[vertex] = data;
        }

        let mut workers = vec![(); randoms.len()];
        let mut level = 0;
        let mut set_end = n;
        while set_end != 0 {
            // compute independent set in parallel
            let flags = parallel_chunks(&mut workers, 0, set_end, |_, chunk| {
                chunk
                    .map(|pos| {
                        let vertex = self.order[pos];
                        (vertex, self.is_independent(vertex))
                    })
                    .collect()
            });
            for (vertex, independent) in flags {
                self.data[vertex].independent = independent;
            }
            let set_start = self.partition_independent_set(set_end);

            // compute shortcuts for independent vertices in parallel
            let shortcuts = parallel_chunks(&mut workers, set_start, set_end, |_, chunk| {
                chunk
                    .map(|pos| {
                        let vertex = self.order[pos];
                        (vertex, self.get_shortcuts(vertex))
                    })
                    .collect()
            });
            for (vertex, found) in shortcuts {
                self.shortcuts[vertex] = found;
            }
            for pos in set_start..set_end {
                self.contract_vertex(self.order[pos], level);
                level += 1;
            }

            // update neighbours priorities in parallel
            let updates = parallel_chunks(&mut workers, set_start, set_end, |_, chunk| {
                chunk
                    .flat_map(|pos| self.updated_neighbours(self.order[pos]))
                    .collect()
            });
            for (neighbour, depth, priority) in updates {
                self.data[neighbour].depth = depth;
                self.data[neighbour].priority = priority;
            }
            for pos in set_start..set_end {
                self.data[self.order[pos]].contracted = true;
            }
            set_end = set_start;
        }
    }

    fn visible(&self, vertex: usize) -> bool {
        !self.data[vertex].contracted
    }

    /// Neighbours of `vertex` in the overlay graph (Java: `Graphs.neighborSetOf` on the masked
    /// contraction graph).
    fn neighbours(&self, vertex: usize) -> Vec<usize> {
        let mut result: IndexSet<usize> = IndexSet::new();
        for &e in &self.incoming[vertex] {
            let other = self.edges[e].source;
            if self.visible(other) {
                result.insert(other);
            }
        }
        for &e in &self.outgoing[vertex] {
            let other = self.edges[e].target;
            if self.visible(other) {
                result.insert(other);
            }
        }
        result.into_iter().collect()
    }

    /// Determines if a `vertex` is independent wrt the overlay graph.
    fn is_independent(&self, vertex: usize) -> bool {
        for first in self.neighbours(vertex) {
            if self.is_greater(vertex, first) {
                return false;
            }
            for second in self.neighbours(first) {
                if second != vertex && self.is_greater(vertex, second) {
                    return false;
                }
            }
        }
        true
    }

    /// Determines if the priority of `a` is greater than the priority of `b`. Ties are broken by
    /// the random values and then by the current position, which guarantees that every round
    /// has at least one independent vertex.
    fn is_greater(&self, a: usize, b: usize) -> bool {
        let (da, db) = (&self.data[a], &self.data[b]);
        if da.priority != db.priority {
            return da.priority > db.priority;
        }
        if da.random != db.random {
            return da.random > db.random;
        }
        self.position[a] > self.position[b]
    }

    /// Partitions `order[..end]` into not independent and independent vertices and returns the
    /// position of the first independent one.
    fn partition_independent_set(&mut self, end: usize) -> usize {
        let mut left = 0;
        let mut right = end as isize - 1;
        while left as isize <= right {
            while left < end && !self.data[self.order[left]].independent {
                left += 1;
            }
            while right >= 0 && self.data[self.order[right as usize]].independent {
                right -= 1;
            }
            if left as isize <= right {
                self.order.swap(left, right as usize);
                self.position[self.order[left]] = left;
                self.position[self.order[right as usize]] = right as usize;
            }
        }
        left
    }

    /// Adds the shortcuts of `vertex` to the contraction graph and assigns its level.
    fn contract_vertex(&mut self, vertex: usize, level: usize) {
        for (first, second) in std::mem::take(&mut self.shortcuts[vertex]) {
            let source = self.edges[first].source;
            let target = self.edges[second].target;
            let weight = self.edges[first].weight + self.edges[second].weight;
            let original_edges =
                self.edges[first].original_edges + self.edges[second].original_edges;
            if let Some(&id) = self.by_endpoints.get(&(source, target)) {
                // update the already existing edge
                let edge = &mut self.edges[id];
                edge.original = None;
                edge.bypassed = Some((first, second));
                edge.original_edges = original_edges;
                edge.weight = weight;
            } else {
                self.push_edge(CoreEdge {
                    source,
                    target,
                    weight,
                    original_edges,
                    original: None,
                    bypassed: Some((first, second)),
                });
            }
        }
        self.level[vertex] = level;
    }

    /// New depth and priority of every neighbour of the just contracted `vertex`.
    fn updated_neighbours(&self, vertex: usize) -> Vec<(usize, usize, f64)> {
        let depth = self.data[vertex].depth + 1;
        self.neighbours(vertex)
            .into_iter()
            .map(|neighbour| {
                let depth = self.data[neighbour].depth.max(depth);
                (neighbour, depth, self.priority(neighbour, depth))
            })
            .collect()
    }

    /// Priority of `vertex`: edge quotient, complexity quotient and hierarchical depth.
    fn priority(&self, vertex: usize, depth: usize) -> f64 {
        let mut stats = Statistics::default();
        self.iterate_shortcut_edges(vertex, &mut |first, second| {
            stats.added_contraction_edges += 1;
            stats.added_original_edges +=
                self.edges[first].original_edges + self.edges[second].original_edges;
        });
        for &e in &self.incoming[vertex] {
            if self.visible(self.edges[e].source) {
                stats.removed_contraction_edges += 1;
                stats.removed_original_edges += self.edges[e].original_edges;
            }
        }
        for &e in &self.outgoing[vertex] {
            if self.visible(self.edges[e].target) {
                stats.removed_contraction_edges += 1;
                stats.removed_original_edges += self.edges[e].original_edges;
            }
        }
        if stats.removed_contraction_edges * stats.removed_original_edges == 0 {
            depth as f64
        } else {
            4.0 * stats.added_contraction_edges as f64 / stats.removed_contraction_edges as f64
                + 2.0 * stats.added_original_edges as f64 / stats.removed_original_edges as f64
                + depth as f64
        }
    }

    fn get_shortcuts(&self, vertex: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        self.iterate_shortcut_edges(vertex, &mut |first, second| result.push((first, second)));
        result
    }

    fn in_overlay(&self, vertex: usize) -> bool {
        self.visible(vertex) && !self.data[vertex].independent
    }

    /// Runs forward shortest-path searches in the current overlay graph to find the shortcuts of
    /// `vertex`, ignoring `vertex` itself, and passes each found shortcut to `consumer`. To
    /// prune the search, keeps track of the value $d(u, v) + max {c(v, w) : (v, w) \in E'}$. If
    /// the original graph is undirected each predecessor of `vertex` is considered only once and
    /// for each found shortcut in the forward direction another one in the backward direction is
    /// generated.
    fn iterate_shortcut_edges(&self, vertex: usize, consumer: &mut impl FnMut(usize, usize)) {
        let mut successors: IndexSet<usize> = IndexSet::new();
        let mut max_outgoing_edge_weight = f64::from_bits(1); // Double.MIN_VALUE
        for &out_edge in &self.outgoing[vertex] {
            let successor = self.edges[out_edge].target;
            if !self.in_overlay(successor) {
                continue;
            }
            successors.insert(successor);
            max_outgoing_edge_weight = max_outgoing_edge_weight.max(self.edges[out_edge].weight);
        }
        for &in_edge in &self.incoming[vertex] {
            let predecessor = self.edges[in_edge].source;
            if !self.in_overlay(predecessor) {
                continue;
            }
            let contained_predecessor = successors.shift_remove(&predecessor);
            let in_weight = self.edges[in_edge].weight;
            let distances = self.iterate_to_successors(
                predecessor,
                &successors,
                vertex,
                in_weight + max_outgoing_edge_weight,
            );
            for &successor in &successors {
                let out_edge = self.by_endpoints[&(vertex, successor)];
                let path_weight = in_weight + self.edges[out_edge].weight;
                if distances.get(&successor).is_none_or(|&d| d > path_weight) {
                    consumer(in_edge, out_edge);
                    if self.undirected {
                        consumer(
                            self.by_endpoints[&(successor, vertex)],
                            self.by_endpoints[&(vertex, predecessor)],
                        );
                    }
                }
            }
            if contained_predecessor && !self.undirected {
                successors.insert(predecessor); // restore predecessor if needed
            }
        }
    }

    /// Dijkstra search in the overlay graph from `source` ignoring `vertex_to_ignore`, limited by
    /// `radius`. It proceeds until all `successors` are reached or no vertex is left. Returns the
    /// computed distances of the reached vertices.
    fn iterate_to_successors(
        &self,
        source: usize,
        successors: &IndexSet<usize>,
        vertex_to_ignore: usize,
        radius: f64,
    ) -> HashMap<usize, f64> {
        let mut heap: JavaPairingHeap<OrderedF64, usize> = JavaPairingHeap::default();
        let mut seen: HashMap<
            usize,
            (
                f64,
                <JavaPairingHeap<OrderedF64, usize> as AddressableHeap<OrderedF64, usize>>::Handle,
            ),
        > = HashMap::new();
        let handle = heap.insert(OrderedF64(0.0), source);
        seen.insert(source, (0.0, handle));

        let number_of_successors = successors.len();
        let mut passed_successors = 0;
        while let Some((OrderedF64(distance), vertex)) = heap.pop() {
            if distance > radius {
                break;
            }
            if successors.contains(&vertex) {
                passed_successors += 1;
                if passed_successors == number_of_successors {
                    break;
                }
            }
            // relax the outgoing edges, skipping vertices that are not in the overlay graph
            for &e in &self.outgoing[vertex] {
                let successor = self.edges[e].target;
                if !self.visible(successor)
                    || successor == vertex_to_ignore
                    || self.data[successor].independent
                {
                    continue;
                }
                let updated = distance + self.edges[e].weight;
                match seen.get_mut(&successor) {
                    None => {
                        let handle = heap.insert(OrderedF64(updated), successor);
                        seen.insert(successor, (updated, handle));
                    }
                    Some((key, handle)) if updated < *key => {
                        *key = updated;
                        let _ = heap.decrease_key(*handle, OrderedF64(updated));
                    }
                    Some(_) => {}
                }
            }
        }
        seen.into_iter().map(|(v, (d, _))| (v, d)).collect()
    }

    /// Converts the contraction into a hierarchy whose vertex ids are the final positions in the
    /// vertex array (Java: `vertexId`).
    fn into_hierarchy<'g, G: Graph>(
        self,
        graph: &'g G,
        originals: Vec<G::Edge>,
    ) -> ContractionHierarchy<'g, G> {
        let input: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let vertices: Vec<ContractionVertex<G::Vertex>> = self
            .order
            .iter()
            .enumerate()
            .map(|(vertex_id, &v)| ContractionVertex {
                vertex_id,
                vertex: input[v].clone(),
                contraction_level: self.level[v],
            })
            .collect();
        let mapping = vertices
            .iter()
            .map(|v| (v.vertex.clone(), v.vertex_id))
            .collect();
        // mark upward edges
        let edges = self
            .edges
            .iter()
            .map(|e| {
                let (source, target) = (self.position[e.source], self.position[e.target]);
                ContractionEdge {
                    source,
                    target,
                    edge: e.original.map(|o| originals[o].clone()),
                    bypassed_edges: e.bypassed,
                    is_upward: vertices[source].contraction_level
                        < vertices[target].contraction_level,
                    original_edges: e.original_edges,
                    weight: e.weight,
                }
            })
            .collect();
        let outgoing = self
            .order
            .iter()
            .map(|&v| self.outgoing[v].clone())
            .collect();
        let incoming = self
            .order
            .iter()
            .map(|&v| self.incoming[v].clone())
            .collect();
        ContractionHierarchy {
            graph,
            data: Arc::new(HierarchyData {
                vertices,
                mapping,
                edges,
                outgoing,
                incoming,
            }),
        }
    }
}

/// A search frontier of the hierarchical query (Java: `DijkstraSearchFrontier`).
struct Frontier {
    heap: JavaPairingHeap<OrderedF64, usize>,
    seen: HashMap<usize, FrontierNode>,
    finished: bool,
}

struct FrontierNode {
    distance: f64,
    handle: <JavaPairingHeap<OrderedF64, usize> as AddressableHeap<OrderedF64, usize>>::Handle,
    tree_edge: Option<usize>,
}

impl Frontier {
    fn new() -> Self {
        Frontier {
            heap: JavaPairingHeap::default(),
            seen: HashMap::new(),
            finished: false,
        }
    }

    fn update_distance(&mut self, vertex: usize, edge: Option<usize>, distance: f64) {
        match self.seen.get_mut(&vertex) {
            None => {
                let handle = self.heap.insert(OrderedF64(distance), vertex);
                self.seen.insert(
                    vertex,
                    FrontierNode {
                        distance,
                        handle,
                        tree_edge: edge,
                    },
                );
            }
            Some(node) if distance < node.distance => {
                node.distance = distance;
                node.tree_edge = edge;
                let _ = self.heap.decrease_key(node.handle, OrderedF64(distance));
            }
            Some(_) => {}
        }
    }

    fn distance(&self, vertex: usize) -> f64 {
        self.seen.get(&vertex).map_or(f64::INFINITY, |n| n.distance)
    }

    fn tree_edge(&self, vertex: usize) -> Option<usize> {
        self.seen.get(&vertex).and_then(|n| n.tree_edge)
    }

    fn min_key(&self) -> Option<f64> {
        self.heap.peek().map(|(_, key, _)| key.0)
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
/// Comparing to usual shortest path algorithm, as `Dijkstra`,
/// `AStar`, etc., this algorithm spends time for computing contraction hierarchy
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
/// The two frontiers alternate and use the pairing heap of JHeaps, as in Java.
pub struct ContractionHierarchyBidirectionalDijkstra<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
    radius: f64,
}

impl<'h, 'g, G: Graph> ContractionHierarchyBidirectionalDijkstra<'h, 'g, G> {
    /// Constructs a new instance of the algorithm for a given contraction hierarchy.
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

    /// Computes a shortest path between `source` and `sink` using the hierarchy.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let h = self.hierarchy;
        if !h.graph.contains_vertex(source) || !h.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        // handle special case if source equals target
        if source == sink {
            return Ok(Some(GraphWalk::single(source.clone())));
        }
        let (source, sink) = (h.mapping[source], h.mapping[sink]);

        let mut forward = Frontier::new();
        let mut backward = Frontier::new();
        forward.update_distance(source, None, 0.0);
        backward.update_distance(sink, None, 0.0);

        let mut best_path = f64::INFINITY;
        let mut common_vertex = None;
        let mut forward_is_current = true;
        loop {
            let (frontier, other) = if forward_is_current {
                (&mut forward, &mut backward)
            } else {
                (&mut backward, &mut forward)
            };
            if frontier.heap.is_empty() {
                frontier.finished = true;
            }
            if other.heap.is_empty() {
                other.finished = true;
            }
            // stopping condition for search
            if frontier.finished && other.finished {
                break;
            }
            // stopping condition for current frontier
            match frontier.min_key() {
                Some(min) if min < best_path => {
                    // frontier scan
                    let (OrderedF64(distance), v) = frontier.heap.pop().expect("non-empty heap");
                    if forward_is_current {
                        for &e in &h.outgoing[v] {
                            let edge = &h.edges[e];
                            if !edge.is_upward {
                                continue;
                            }
                            frontier.update_distance(edge.target, Some(e), distance + edge.weight);
                            let path_distance =
                                distance + edge.weight + other.distance(edge.target);
                            if path_distance < best_path {
                                best_path = path_distance;
                                common_vertex = Some(edge.target);
                            }
                        }
                    } else {
                        for &e in &h.incoming[v] {
                            let edge = &h.edges[e];
                            if edge.is_upward {
                                continue;
                            }
                            frontier.update_distance(edge.source, Some(e), distance + edge.weight);
                            let path_distance =
                                distance + edge.weight + other.distance(edge.source);
                            if path_distance < best_path {
                                best_path = path_distance;
                                common_vertex = Some(edge.source);
                            }
                        }
                    }
                }
                _ => frontier.finished = true,
            }
            // swap frontiers only if the other frontier is not yet finished
            if !other.finished {
                forward_is_current = !forward_is_current;
            }
        }
        // create path if found
        let common_vertex = match common_vertex {
            Some(v) if best_path.is_finite() && best_path <= self.radius => v,
            _ => return Ok(None),
        };
        let mut vertices = VecDeque::from([h.vertices[common_vertex].vertex.clone()]);
        let mut edges = VecDeque::new();
        // traverse forward path
        let mut v = common_vertex;
        while let Some(e) = forward.tree_edge(v) {
            h.unpack_backward(e, &mut vertices, &mut edges);
            v = h.edges[e].source;
        }
        // traverse reverse path
        v = common_vertex;
        while let Some(e) = backward.tree_edge(v) {
            h.unpack_forward(e, &mut vertices, &mut edges);
            v = h.edges[e].target;
        }
        Ok(Some(GraphWalk {
            vertices: vertices.into(),
            edges: edges.into(),
            weight: best_path,
        }))
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
            let target = target.borrow();
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "V: serde::Serialize, E: serde::Serialize",
        deserialize = "V: serde::Deserialize<'de> + Eq + std::hash::Hash, E: serde::Deserialize<'de> + Eq + std::hash::Hash"
    ))
)]
#[derive(Debug, Clone)]
pub struct ManyToManyPaths<V, E> {
    sources: HashSet<V>,
    targets: HashSet<V>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_support::hash_pairs"))]
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
    /// Path weight; `None` when `target` is unreachable from `source`.
    pub fn weight(&self, source: &V, target: &V) -> Option<f64> {
        self.path(source, target).map(|p| p.weight)
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
/// The search spaces, buckets and middle vertices are the ones of the Java algorithm. Java unpacks
/// a path from the search spaces when it is asked for; here all requested paths are unpacked once
/// into a [`ManyToManyPaths`]. The iteration order of the vertex sets is the order of the
/// hierarchy ids (Java: `HashSet` order), which only decides between equally long paths.
pub struct ContractionHierarchyManyToManyShortestPaths<'h, 'g, G: Graph> {
    hierarchy: &'h ContractionHierarchy<'g, G>,
}

/// Distance and tree edge of the vertices reached by one search of the many-to-many algorithm.
type SearchSpace = HashMap<usize, (f64, Option<usize>)>;

impl<'h, 'g, G: Graph> ContractionHierarchyManyToManyShortestPaths<'h, 'g, G> {
    /// Constructs a new instance of the algorithm for a given contraction hierarchy.
    pub fn new(hierarchy: &'h ContractionHierarchy<'g, G>) -> Self {
        Self { hierarchy }
    }

    /// Computes shortest paths from all vertices in `sources` to all vertices in `targets`.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn many_to_many<I, J>(
        &self,
        sources: I,
        targets: J,
    ) -> Result<ManyToManyPaths<G::Vertex, G::Edge>>
    where
        I: IntoIterator<Item = G::Vertex>,
        J: IntoIterator<Item = G::Vertex>,
    {
        let h = self.hierarchy;
        let sources: HashSet<_> = sources.into_iter().collect();
        let targets: HashSet<_> = targets.into_iter().collect();
        if sources
            .iter()
            .chain(&targets)
            .any(|v| !h.graph.contains_vertex(v))
        {
            return Err(GraphError::VertexNotFound);
        }
        let ids = |set: &HashSet<G::Vertex>| {
            let mut ids: Vec<usize> = set.iter().map(|v| h.mapping[v]).collect();
            ids.sort_unstable();
            ids
        };
        let (source_ids, target_ids) = (ids(&sources), ids(&targets));

        // with more sources than targets the algorithm runs on the reversed graph
        let reversed = sources.len() > targets.len();
        let (search_sources, search_targets) = if reversed {
            (&target_ids, &source_ids)
        } else {
            (&source_ids, &target_ids)
        };
        let search_source_set: HashSet<usize> = search_sources.iter().copied().collect();
        let search_target_set: HashSet<usize> = search_targets.iter().copied().collect();

        let mut buckets: Vec<Vec<(usize, f64)>> = vec![Vec::new(); h.vertices.len()];
        let mut backward_spaces: HashMap<usize, SearchSpace> = HashMap::new();
        let mut forward_spaces: HashMap<usize, SearchSpace> = HashMap::new();
        let mut middle_vertices: HashMap<(usize, usize), (f64, usize)> = HashMap::new();

        for &target in search_targets {
            let space = h.search_space(target, &search_source_set, |v| {
                if reversed {
                    h.upward_outgoing(v)
                } else {
                    h.downward_incoming(v)
                }
            });
            let mut entries: Vec<_> = space.iter().map(|(&v, &(d, _))| (v, d)).collect();
            entries.sort_by_key(|&(v, _)| v);
            for (vertex, distance) in entries {
                buckets[vertex].push((target, distance));
            }
            backward_spaces.insert(target, space);
        }
        for &source in search_sources {
            let space = h.search_space(source, &search_target_set, |v| {
                if reversed {
                    h.downward_incoming(v)
                } else {
                    h.upward_outgoing(v)
                }
            });
            let mut entries: Vec<_> = space.iter().map(|(&v, &(d, _))| (v, d)).collect();
            entries.sort_by_key(|&(v, _)| v);
            for (middle, forward_distance) in entries {
                for &(target, bucket_distance) in &buckets[middle] {
                    let path_distance = forward_distance + bucket_distance;
                    let pair = if reversed {
                        (target, source)
                    } else {
                        (source, target)
                    };
                    match middle_vertices.get(&pair) {
                        Some(&(d, _)) if d <= path_distance => {}
                        _ => {
                            middle_vertices.insert(pair, (path_distance, middle));
                        }
                    }
                }
            }
            forward_spaces.insert(source, space);
        }
        // the spaces rooted at the original sources/targets
        let (source_spaces, target_spaces) = if reversed {
            (&backward_spaces, &forward_spaces)
        } else {
            (&forward_spaces, &backward_spaces)
        };

        let mut paths = HashMap::new();
        for &source in &source_ids {
            for &target in &target_ids {
                let Some(&(weight, common)) = middle_vertices.get(&(source, target)) else {
                    continue;
                };
                let mut vertices = VecDeque::from([h.vertices[common].vertex.clone()]);
                let mut edges = VecDeque::new();
                // traverse forward path
                let mut v = common;
                while let Some(e) = source_spaces[&source][&v].1 {
                    h.unpack_backward(e, &mut vertices, &mut edges);
                    v = h.edges[e].source;
                }
                // traverse reverse path
                v = common;
                while let Some(e) = target_spaces[&target][&v].1 {
                    h.unpack_forward(e, &mut vertices, &mut edges);
                    v = h.edges[e].target;
                }
                paths.insert(
                    (
                        h.vertices[source].vertex.clone(),
                        h.vertices[target].vertex.clone(),
                    ),
                    GraphWalk {
                        vertices: vertices.into(),
                        edges: edges.into(),
                        weight,
                    },
                );
            }
        }
        Ok(ManyToManyPaths::from_parts(sources, targets, paths))
    }
}

impl<'g, G: Graph> ContractionHierarchy<'g, G> {
    /// Upward edges leaving `vertex`, with their targets.
    pub(super) fn upward_outgoing(&self, vertex: usize) -> Vec<(usize, usize)> {
        self.outgoing[vertex]
            .iter()
            .filter(|&&e| self.edges[e].is_upward)
            .map(|&e| (e, self.edges[e].target))
            .collect()
    }

    /// Downward edges entering `vertex`, with their sources.
    pub(super) fn downward_incoming(&self, vertex: usize) -> Vec<(usize, usize)> {
        self.incoming[vertex]
            .iter()
            .filter(|&&e| !self.edges[e].is_upward)
            .map(|&e| (e, self.edges[e].source))
            .collect()
    }

    /// Dijkstra search from `source` that stops as soon as all `targets` are settled
    /// (`BaseManyToManyShortestPaths.getShortestPathsTree`). Returns the distance and tree edge of
    /// every vertex that was reached.
    fn search_space(
        &self,
        source: usize,
        targets: &HashSet<usize>,
        edges_of: impl Fn(usize) -> Vec<(usize, usize)>,
    ) -> SearchSpace {
        let mut frontier = Frontier::new();
        frontier.update_distance(source, None, 0.0);
        let mut reached_targets = 0;
        while !frontier.heap.is_empty() && reached_targets < targets.len() {
            let (OrderedF64(distance), vertex) = frontier.heap.pop().expect("non-empty heap");
            for (edge, other) in edges_of(vertex) {
                frontier.update_distance(other, Some(edge), distance + self.edges[edge].weight);
            }
            if targets.contains(&vertex) {
                reached_targets += 1;
            }
        }
        frontier
            .seen
            .into_iter()
            .map(|(v, node)| (v, (node.distance, node.tree_edge)))
            .collect()
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for ContractionHierarchyManyToManyShortestPaths<'_, '_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        ContractionHierarchyBidirectionalDijkstra::new(self.hierarchy).single_source(source)
    }

    /// Return the path from the `source` vertex to the `target` vertex. If no such
    /// path exists, `None` is returned.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        ContractionHierarchyBidirectionalDijkstra::new(self.hierarchy).path(source, sink)
    }
}
