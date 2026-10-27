//! Eppstein's sidetrack-heap algorithm for the `k` shortest walks.
//!
//! Unlike Yen's algorithm, the paths returned here need not be simple. Cycles therefore produce
//! an unbounded iterator. The implementation follows JGraphT's improved paths-graph construction:
//! persistent heaps contain only sidetracks reachable from the source, and cross edges are built
//! only for paths-graph vertices reachable from its root.

use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

#[derive(Clone)]
struct PathsGraphVertex<E> {
    edge: Option<E>,
    delta: f64,
    size: usize,
    left: Option<usize>,
    right: Option<usize>,
    rest: Option<usize>,
    cross: Option<usize>,
}

impl<E> PathsGraphVertex<E> {
    fn new(edge: Option<E>, delta: f64) -> Self {
        Self {
            edge,
            delta,
            size: 1,
            left: None,
            right: None,
            rest: None,
            cross: None,
        }
    }
}

#[derive(Clone)]
struct ImplicitPath {
    paths_graph_vertices: Vec<usize>,
    weight: f64,
}

/// Lazy iterator over the shortest walks from a source to a sink.
///
/// Construction takes `O(m + n log n)` time for the reverse shortest-path tree and sidetrack
/// heaps. Once constructed, candidates are generated from the implicit paths graph rather than by
/// rerunning a shortest-path search.
pub struct EppsteinShortestPathIterator<'g, G: Graph> {
    graph: &'g G,
    source: G::Vertex,
    sink: G::Vertex,
    distance: HashMap<G::Vertex, f64>,
    tree_edge: HashMap<G::Vertex, G::Edge>,
    h_mapping: HashMap<G::Vertex, Option<usize>>,
    paths_graph: Vec<PathsGraphVertex<G::Edge>>,
    paths_graph_root: Option<usize>,
    candidates: Vec<ImplicitPath>,
    queue: BinaryHeap<Reverse<(OrderedF64, usize, usize)>>,
    sequence: usize,
}

impl<'g, G: Graph> EppsteinShortestPathIterator<'g, G> {
    /// Construct an Eppstein iterator.
    ///
    /// As in JGraphT, Eppstein is restricted to directed simple graphs with non-negative weights.
    pub fn new(graph: &'g G, source: &G::Vertex, sink: &G::Vertex) -> Result<Self> {
        let graph_type = graph.graph_type();
        if !graph_type.directed || !graph_type.is_simple() {
            return Err(GraphError::InvalidArgument(
                "graph must be simple and directed".into(),
            ));
        }
        if !graph.contains_vertex(source) || !graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }

        let (distance, tree_edge) = reverse_shortest_path_tree(graph, sink)?;
        let mut iterator = Self {
            graph,
            source: source.clone(),
            sink: sink.clone(),
            distance,
            tree_edge,
            h_mapping: HashMap::new(),
            paths_graph: Vec::new(),
            paths_graph_root: None,
            candidates: Vec::new(),
            queue: BinaryHeap::new(),
            sequence: 0,
        };

        if let Some(&weight) = iterator.distance.get(source) {
            iterator.build_paths_graph();
            iterator.push_candidate(ImplicitPath {
                paths_graph_vertices: Vec::new(),
                weight,
            });
        }
        Ok(iterator)
    }

    fn allocate(&mut self, vertex: PathsGraphVertex<G::Edge>) -> usize {
        let id = self.paths_graph.len();
        self.paths_graph.push(vertex);
        id
    }

    fn push_candidate(&mut self, path: ImplicitPath) {
        let id = self.candidates.len();
        let sequence = self.sequence;
        self.sequence += 1;
        self.queue
            .push(Reverse((OrderedF64(path.weight), sequence, id)));
        self.candidates.push(path);
    }

    fn build_paths_graph(&mut self) {
        self.build_d_graph();
        self.add_cross_edges();
        let mut root = PathsGraphVertex::new(None, 0.0);
        root.cross = self.h_mapping.get(&self.source).copied().flatten();
        self.paths_graph_root = Some(self.allocate(root));
    }

    fn reachable_from_source(&self) -> Vec<G::Vertex> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![self.source.clone()];
        while let Some(vertex) = stack.pop() {
            if !seen.insert(vertex.clone()) {
                continue;
            }
            result.push(vertex.clone());
            let mut successors: Vec<_> = self
                .graph
                .outgoing_of(&vertex)
                .map(|(_, target)| target.clone())
                .collect();
            successors.reverse();
            stack.extend(successors);
        }
        result
    }

    /// Build the persistent heap H(v) for every useful input vertex.
    fn build_d_graph(&mut self) {
        for vertex in self.reachable_from_source() {
            if !self.distance.contains_key(&vertex) || self.h_mapping.contains_key(&vertex) {
                continue;
            }
            let mut stack = Vec::new();
            let mut current = vertex;
            loop {
                if self.h_mapping.contains_key(&current) {
                    break;
                }
                stack.push(current.clone());
                if current == self.sink {
                    break;
                }
                let edge = self
                    .tree_edge
                    .get(&current)
                    .expect("reachable non-sink vertex has a shortest-tree edge");
                current = self
                    .graph
                    .edge_target(edge)
                    .expect("tree edge belongs to graph")
                    .clone();
            }
            let mut predecessor_heap = self.h_mapping.get(&current).copied().flatten();
            while let Some(v) = stack.pop() {
                if let Some(existing) = self.h_mapping.get(&v).copied() {
                    predecessor_heap = existing;
                    continue;
                }
                let mapping = self.insert_vertex(&v, predecessor_heap);
                self.h_mapping.insert(v, mapping);
                predecessor_heap = mapping;
            }
        }
    }

    fn insert_vertex(
        &mut self,
        vertex: &G::Vertex,
        predecessor_heap: Option<usize>,
    ) -> Option<usize> {
        let (outroot, rest_root) = self.outroot_and_rest_heap(vertex);
        let Some(outroot) = outroot else {
            return predecessor_heap;
        };
        let mapping = self.insert_persistently(predecessor_heap, outroot);
        self.paths_graph[mapping].rest = rest_root;
        Some(mapping)
    }

    /// Persistent insertion into the balanced heap of minimum sidetrack deltas.
    fn insert_persistently(&mut self, root: Option<usize>, vertex: usize) -> usize {
        let Some(root) = root else {
            self.paths_graph[vertex].left = None;
            self.paths_graph[vertex].right = None;
            self.paths_graph[vertex].size = 1;
            return vertex;
        };

        let root_copy = self.allocate(self.paths_graph[root].clone());
        let left_direction = self.paths_graph[root].left.is_none()
            || self.paths_graph[root].right.is_some_and(|right| {
                let left = self.paths_graph[root]
                    .left
                    .expect("left exists in this branch");
                self.paths_graph[left].size <= self.paths_graph[right].size
            });

        let (minimum, maximum) =
            if self.paths_graph[vertex].delta >= self.paths_graph[root_copy].delta {
                (root_copy, vertex)
            } else {
                self.paths_graph[vertex].left = self.paths_graph[root_copy].left;
                self.paths_graph[vertex].right = self.paths_graph[root_copy].right;
                self.paths_graph[vertex].size = self.paths_graph[root_copy].size;
                self.paths_graph[root_copy].left = None;
                self.paths_graph[root_copy].right = None;
                (vertex, root_copy)
            };

        let child = if left_direction {
            self.paths_graph[minimum].left
        } else {
            self.paths_graph[minimum].right
        };
        let inserted = self.insert_persistently(child, maximum);
        if left_direction {
            self.paths_graph[minimum].left = Some(inserted);
        } else {
            self.paths_graph[minimum].right = Some(inserted);
        }
        self.paths_graph[minimum].size += 1;
        minimum
    }

    fn outroot_and_rest_heap(&mut self, vertex: &G::Vertex) -> (Option<usize>, Option<usize>) {
        let predecessor = self.tree_edge.get(vertex).cloned();
        let mut outroot: Option<usize> = None;
        let mut rest: Vec<usize> = Vec::new();
        let outgoing: Vec<_> = self
            .graph
            .outgoing_of(vertex)
            .map(|(edge, target)| (edge.clone(), target.clone()))
            .collect();

        for (edge, target) in outgoing {
            if !self.distance.contains_key(&target) || predecessor.as_ref() == Some(&edge) {
                continue;
            }
            let source_distance = self.distance[vertex];
            let delta = self.graph.edge_weight(&edge).unwrap_or(1.0) + self.distance[&target]
                - source_distance;
            let node = self.allocate(PathsGraphVertex::new(Some(edge), delta));
            if outroot.is_none_or(|root| delta < self.paths_graph[root].delta) {
                if let Some(old) = outroot.replace(node) {
                    rest.push(old);
                }
            } else {
                rest.push(node);
            }
        }

        if rest.is_empty() {
            return (outroot, None);
        }
        self.heapify(&mut rest);
        let rest_root = self.link_rest_heap(&rest, 0);
        (outroot, rest_root)
    }

    fn heapify(&self, vertices: &mut [usize]) {
        for start in (0..vertices.len() / 2).rev() {
            let mut current = start;
            loop {
                let left = 2 * current + 1;
                let right = left + 1;
                let mut smaller = current;
                if left < vertices.len()
                    && self.paths_graph[vertices[left]].delta
                        < self.paths_graph[vertices[smaller]].delta
                {
                    smaller = left;
                }
                if right < vertices.len()
                    && self.paths_graph[vertices[right]].delta
                        < self.paths_graph[vertices[smaller]].delta
                {
                    smaller = right;
                }
                if smaller == current {
                    break;
                }
                vertices.swap(current, smaller);
                current = smaller;
            }
        }
    }

    fn link_rest_heap(&mut self, vertices: &[usize], position: usize) -> Option<usize> {
        if position >= vertices.len() {
            return None;
        }
        let node = vertices[position];
        self.paths_graph[node].left = self.link_rest_heap(vertices, 2 * position + 1);
        self.paths_graph[node].right = self.link_rest_heap(vertices, 2 * position + 2);
        Some(node)
    }

    fn add_cross_edges(&mut self) {
        let Some(source_heap) = self.h_mapping.get(&self.source).copied().flatten() else {
            return;
        };
        let mut queue = VecDeque::from([source_heap]);
        let mut seen = HashSet::new();
        while let Some(vertex) = queue.pop_front() {
            if !seen.insert(vertex) {
                continue;
            }
            let edge = self.paths_graph[vertex]
                .edge
                .as_ref()
                .expect("non-root paths-graph vertex has an edge");
            let target = self.graph.edge_target(edge).expect("edge belongs to graph");
            self.paths_graph[vertex].cross = self.h_mapping.get(target).copied().flatten();
            let node = &self.paths_graph[vertex];
            for next in [node.left, node.right, node.rest, node.cross]
                .into_iter()
                .flatten()
            {
                if !seen.contains(&next) {
                    queue.push_back(next);
                }
            }
        }
    }

    fn add_extension(&mut self, path: &ImplicitPath, extending_vertex: usize, added_weight: f64) {
        let mut vertices = path.paths_graph_vertices.clone();
        vertices.push(extending_vertex);
        self.push_candidate(ImplicitPath {
            paths_graph_vertices: vertices,
            weight: path.weight + added_weight,
        });
    }

    fn add_one_edge_extensions(&mut self, path: &ImplicitPath) {
        let last = path
            .paths_graph_vertices
            .last()
            .copied()
            .or(self.paths_graph_root)
            .expect("root exists when a path exists");
        let node = &self.paths_graph[last];
        let (left, right, rest, cross, delta) =
            (node.left, node.right, node.rest, node.cross, node.delta);
        for child in [left, right, rest].into_iter().flatten() {
            self.add_extension(path, child, self.paths_graph[child].delta - delta);
        }
        if let Some(cross) = cross {
            self.add_extension(path, cross, self.paths_graph[cross].delta);
        }
    }

    fn sidetracks(&self, vertices: &[usize]) -> Vec<usize> {
        let mut result = Vec::new();
        for (position, &vertex) in vertices.iter().enumerate() {
            let replaced_by_heap_child = vertices.get(position + 1).is_some_and(|&next| {
                let node = &self.paths_graph[vertex];
                node.left == Some(next) || node.right == Some(next) || node.rest == Some(next)
            });
            if !replaced_by_heap_child {
                result.push(vertex);
            }
        }
        result
    }

    fn materialize(&self, path: &ImplicitPath) -> GraphWalk<G::Vertex, G::Edge> {
        let mut edges = Vec::new();
        let mut current = self.source.clone();
        for sidetrack in self.sidetracks(&path.paths_graph_vertices) {
            let sidetrack_edge = self.paths_graph[sidetrack]
                .edge
                .as_ref()
                .expect("sidetrack node has edge");
            let sidetrack_source = self
                .graph
                .edge_source(sidetrack_edge)
                .expect("edge belongs to graph");
            while &current != sidetrack_source {
                let edge = self
                    .tree_edge
                    .get(&current)
                    .expect("sidetrack source lies on a shortest-tree continuation");
                edges.push(edge.clone());
                current = self
                    .graph
                    .edge_target(edge)
                    .expect("edge belongs to graph")
                    .clone();
            }
            edges.push(sidetrack_edge.clone());
            current = self
                .graph
                .edge_target(sidetrack_edge)
                .expect("edge belongs to graph")
                .clone();
        }
        while current != self.sink {
            let edge = self
                .tree_edge
                .get(&current)
                .expect("reachable vertex has shortest-tree continuation");
            edges.push(edge.clone());
            current = self
                .graph
                .edge_target(edge)
                .expect("edge belongs to graph")
                .clone();
        }

        let mut vertices = Vec::with_capacity(edges.len() + 1);
        vertices.push(self.source.clone());
        for edge in &edges {
            vertices.push(
                self.graph
                    .edge_target(edge)
                    .expect("edge belongs to graph")
                    .clone(),
            );
        }
        GraphWalk::new(vertices, edges, path.weight)
    }
}

impl<G: Graph> Iterator for EppsteinShortestPathIterator<'_, G> {
    type Item = GraphWalk<G::Vertex, G::Edge>;

    fn next(&mut self) -> Option<Self::Item> {
        let Reverse((_, _, candidate)) = self.queue.pop()?;
        let path = self.candidates[candidate].clone();
        self.add_one_edge_extensions(&path);
        Some(self.materialize(&path))
    }
}

/// Return the first `k` shortest walks using [`EppsteinShortestPathIterator`].
pub struct EppsteinKShortestPath<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> EppsteinKShortestPath<'g, G> {
    /// Bind the algorithm to a graph.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    /// Compute at most `k` shortest walks in non-decreasing weight order.
    pub fn paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        if k == 0 {
            // Java still validates the vertices through iterator construction.
            EppsteinShortestPathIterator::new(self.graph, source, sink)?;
            return Ok(Vec::new());
        }
        Ok(EppsteinShortestPathIterator::new(self.graph, source, sink)?
            .take(k)
            .collect())
    }
}

fn reverse_shortest_path_tree<G: Graph>(
    graph: &G,
    sink: &G::Vertex,
) -> Result<(HashMap<G::Vertex, f64>, HashMap<G::Vertex, G::Edge>)> {
    let vertices: Vec<_> = graph.vertices().collect();
    let index: HashMap<_, _> = vertices
        .iter()
        .enumerate()
        .map(|(index, &vertex)| (vertex, index))
        .collect();
    let sink_index = index[sink];
    let mut distance = vec![f64::INFINITY; vertices.len()];
    let mut predecessor: Vec<Option<&G::Edge>> = vec![None; vertices.len()];
    let mut settled = vec![false; vertices.len()];
    let mut heap = BinaryHeap::new();
    distance[sink_index] = 0.0;
    heap.push(Reverse((OrderedF64(0.0), sink_index)));

    while let Some(Reverse((OrderedF64(current_distance), vertex_index))) = heap.pop() {
        if settled[vertex_index] || current_distance != distance[vertex_index] {
            continue;
        }
        settled[vertex_index] = true;
        let vertex = vertices[vertex_index];
        for (edge, source) in graph.try_incoming_of(vertex)? {
            let weight = graph.edge_weight(edge).unwrap_or(1.0);
            if weight.is_nan() || weight < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "negative or NaN edge weight".into(),
                ));
            }
            let source_index = index[source];
            let updated = current_distance + weight;
            if updated < distance[source_index] {
                distance[source_index] = updated;
                predecessor[source_index] = Some(edge);
                heap.push(Reverse((OrderedF64(updated), source_index)));
            }
        }
    }

    let mut distances = HashMap::new();
    let mut tree_edges = HashMap::new();
    for (vertex_index, vertex) in vertices.into_iter().enumerate() {
        if distance[vertex_index].is_finite() {
            distances.insert(vertex.clone(), distance[vertex_index]);
            if let Some(edge) = predecessor[vertex_index] {
                tree_edges.insert(vertex.clone(), edge.clone());
            }
        }
    }
    Ok((distances, tree_edges))
}
