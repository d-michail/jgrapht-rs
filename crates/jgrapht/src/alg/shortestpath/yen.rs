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

use super::{AllowAllPaths, Dijkstra, PathValidator, ShortestPathAlgorithm};
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::view::MaskSubgraph;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

struct Candidate<V, E> {
    path: GraphWalk<V, E>,
    valid: bool,
    ordinal: usize,
}

/// Lazy iterator over loopless paths in non-decreasing weight order.
pub struct YenShortestPathIterator<'g, G: Graph, P = AllowAllPaths> {
    graph: &'g G,
    source: G::Vertex,
    sink: G::Vertex,
    validator: P,
    initialized: bool,
    candidates: Vec<Candidate<G::Vertex, G::Edge>>,
    processed: Vec<GraphWalk<G::Vertex, G::Edge>>,
    seen: HashSet<Vec<G::Edge>>,
    next_ordinal: usize,
    failure: Option<GraphError>,
}

impl<'g, G: Graph> YenShortestPathIterator<'g, G, AllowAllPaths> {
    pub fn new(graph: &'g G, source: &G::Vertex, sink: &G::Vertex) -> Result<Self> {
        Self::with_validator(graph, source, sink, AllowAllPaths)
    }
}

impl<'g, G: Graph, P: PathValidator<G>> YenShortestPathIterator<'g, G, P> {
    pub fn with_validator(
        graph: &'g G,
        source: &G::Vertex,
        sink: &G::Vertex,
        validator: P,
    ) -> Result<Self> {
        if !graph.contains_vertex(source) || !graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(Self {
            graph,
            source: source.clone(),
            sink: sink.clone(),
            validator,
            initialized: false,
            candidates: Vec::new(),
            processed: Vec::new(),
            seen: HashSet::new(),
            next_ordinal: 0,
            failure: None,
        })
    }

    fn path_is_valid(&self, path: &GraphWalk<G::Vertex, G::Edge>) -> bool {
        let mut weight = 0.0;
        for i in 0..path.edges.len() {
            let partial = GraphWalk::new(
                path.vertices[..=i].to_vec(),
                path.edges[..i].to_vec(),
                weight,
            );
            if !self.validator.is_valid_path(&partial, &path.edges[i]) {
                return false;
            }
            weight += self.graph.edge_weight(&path.edges[i]).unwrap_or(1.0);
        }
        true
    }

    fn push_candidate(&mut self, path: GraphWalk<G::Vertex, G::Edge>) {
        if self.seen.insert(path.edges.clone()) {
            let valid = self.path_is_valid(&path);
            let ordinal = self.next_ordinal;
            self.next_ordinal += 1;
            self.candidates.push(Candidate {
                path,
                valid,
                ordinal,
            });
        }
    }

    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        if let Some(path) = Dijkstra::new(self.graph).path(&self.source, &self.sink)? {
            self.push_candidate(path);
        }
        Ok(())
    }

    fn pop_min_candidate(&mut self) -> Option<Candidate<G::Vertex, G::Edge>> {
        let index = (0..self.candidates.len()).min_by(|&a, &b| {
            self.candidates[a]
                .path
                .weight
                .total_cmp(&self.candidates[b].path.weight)
                .then_with(|| self.candidates[a].ordinal.cmp(&self.candidates[b].ordinal))
        })?;
        Some(self.candidates.remove(index))
    }

    fn add_deviations(&mut self, parent: &GraphWalk<G::Vertex, G::Edge>) -> Result<()> {
        for spur_index in 0..parent.edges.len() {
            let spur = &parent.vertices[spur_index];
            let root_edges = &parent.edges[..spur_index];
            let banned_edges: HashSet<G::Edge> = self
                .processed
                .iter()
                .filter(|path| {
                    path.edges.len() > spur_index && path.edges[..spur_index] == *root_edges
                })
                .map(|path| path.edges[spur_index].clone())
                .collect();
            let banned_vertices: HashSet<&G::Vertex> =
                parent.vertices[..spur_index].iter().collect();
            let masked = MaskSubgraph::new(
                self.graph,
                |vertex: &G::Vertex| banned_vertices.contains(vertex),
                |edge: &G::Edge| banned_edges.contains(edge),
            );
            let Some(spur_path) = Dijkstra::new(&masked).path(spur, &self.sink)? else {
                continue;
            };
            let mut vertices = parent.vertices[..spur_index].to_vec();
            vertices.extend(spur_path.vertices);
            let mut edges = root_edges.to_vec();
            edges.extend(spur_path.edges);
            let root_weight: f64 = root_edges
                .iter()
                .map(|edge| self.graph.edge_weight(edge).unwrap_or(1.0))
                .sum();
            self.push_candidate(GraphWalk::new(
                vertices,
                edges,
                root_weight + spur_path.weight,
            ));
        }
        Ok(())
    }

    /// Fallible analogue of `Iterator::next`.
    pub fn try_next(&mut self) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        self.initialize()?;
        while let Some(candidate) = self.pop_min_candidate() {
            let path = candidate.path;
            self.processed.push(path.clone());
            self.add_deviations(&path)?;
            if candidate.valid {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
}

impl<G: Graph, P: PathValidator<G>> Iterator for YenShortestPathIterator<'_, G, P> {
    type Item = GraphWalk<G::Vertex, G::Edge>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failure.is_some() {
            return None;
        }
        match self.try_next() {
            Ok(path) => path,
            Err(error) => {
                self.failure = Some(error);
                None
            }
        }
    }
}

/// Java-compatible eager facade around [`YenShortestPathIterator`].
pub struct YenKShortestPath<'g, G: Graph, P = AllowAllPaths> {
    graph: &'g G,
    validator: P,
}

impl<'g, G: Graph> YenKShortestPath<'g, G, AllowAllPaths> {
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            validator: AllowAllPaths,
        }
    }
}

impl<'g, G: Graph, P: PathValidator<G> + Clone> YenKShortestPath<'g, G, P> {
    pub fn with_validator(graph: &'g G, validator: P) -> Self {
        Self { graph, validator }
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        let mut iterator = YenShortestPathIterator::with_validator(
            self.graph,
            source,
            sink,
            self.validator.clone(),
        )?;
        let mut result = Vec::new();
        while result.len() < k {
            let Some(path) = iterator.try_next()? else {
                break;
            };
            result.push(path);
        }
        Ok(result)
    }
}

/// Compatibility function retained for the pre-class API.
pub fn yen_k_shortest_paths<G: Graph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    YenKShortestPath::new(graph).get_paths(source, sink, k)
}

/// Statistics collected by [`BoundedPrunedYenKShortestPath`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundedPrunedYenStats {
    pub shortest_path_calls: usize,
    pub spur_tasks_created: usize,
    pub spur_tasks_materialized: usize,
    pub materialized_candidates: usize,
    pub deferred_tasks_left: usize,
    pub candidate_heap_pushes: usize,
    pub candidate_heap_pops: usize,
    pub unreachable_spurs: usize,
    /// Reserved for spur engines which expose their settled-vertex count.
    pub expanded_vertices: usize,
    pub skipped_impossible_spur_tasks: usize,
    candidate_ordinal: u64,
}

struct SpurTask<V> {
    parent_index: usize,
    spur_index: usize,
    banned_vertices: HashSet<V>,
    root_cost: f64,
    lower_bound: f64,
    ordinal: u64,
}

struct BoundedCandidate<V, E> {
    path: GraphWalk<V, E>,
    cost: f64,
    first_deviation_index: usize,
    ordinal: u64,
}

/// Bounded/pruned Yen algorithm with deferred lower-bound spur tasks.
///
/// Each spur task is ordered by `root_cost + distance_to_sink(spur)`. A task
/// is materialized only while its lower bound can tie or beat the cheapest
/// materialized candidate. Equal bounds and path weights use stable insertion
/// ordinals, matching the Java implementation's priority queues.
pub struct BoundedPrunedYenKShortestPath<'g, G: Graph> {
    graph: &'g G,
    stats: RefCell<BoundedPrunedYenStats>,
}

impl<'g, G: Graph> BoundedPrunedYenKShortestPath<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            stats: RefCell::new(BoundedPrunedYenStats::default()),
        }
    }

    pub fn stats(&self) -> BoundedPrunedYenStats {
        self.stats.borrow().clone()
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(source) || !self.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        *self.stats.borrow_mut() = BoundedPrunedYenStats::default();
        if k == 0 {
            return Ok(Vec::new());
        }

        let reverse_distances = self.compute_reverse_distances(sink)?;
        let Some(first) = self.find_spur(source, sink, &HashSet::new(), &HashSet::new())? else {
            return Ok(Vec::new());
        };

        let mut accepted = vec![first];
        let mut first_deviation_index = vec![0usize];
        let mut seen_candidates = HashSet::from([accepted[0].vertices.clone()]);
        let mut tasks = Vec::new();
        let mut task_heap = BinaryHeap::new();
        let mut candidates = Vec::new();
        let mut candidate_heap = BinaryHeap::new();
        self.add_spur_tasks_for(
            0,
            &first_deviation_index,
            &accepted,
            &reverse_distances,
            &mut tasks,
            &mut task_heap,
        );

        while accepted.len() < k {
            while let Some(Reverse((OrderedF64(lower_bound), _, _))) = task_heap.peek().copied() {
                let candidate_cost = candidate_heap
                    .peek()
                    .map_or(f64::INFINITY, |Reverse((OrderedF64(cost), _, _))| *cost);
                if !candidate_heap.is_empty()
                    && lower_bound > candidate_cost
                    && !approx_eq(lower_bound, candidate_cost)
                {
                    break;
                }
                let Reverse((OrderedF64(heap_bound), heap_ordinal, task_index)) =
                    task_heap.pop().expect("peeked task");
                debug_assert_eq!(tasks[task_index].lower_bound, heap_bound);
                debug_assert_eq!(tasks[task_index].ordinal, heap_ordinal);
                self.stats.borrow_mut().spur_tasks_materialized += 1;
                let Some(candidate) = self.materialize(&tasks[task_index], &accepted)? else {
                    self.stats.borrow_mut().unreachable_spurs += 1;
                    continue;
                };
                if seen_candidates.insert(candidate.path.vertices.clone()) {
                    let index = candidates.len();
                    candidate_heap.push(Reverse((
                        OrderedF64(candidate.cost),
                        candidate.ordinal,
                        index,
                    )));
                    candidates.push(candidate);
                    let mut stats = self.stats.borrow_mut();
                    stats.candidate_heap_pushes += 1;
                    stats.materialized_candidates += 1;
                }
            }

            let Some(Reverse((_, _, candidate_index))) = candidate_heap.pop() else {
                break;
            };
            self.stats.borrow_mut().candidate_heap_pops += 1;
            let candidate = &candidates[candidate_index];
            if let Some(Reverse((OrderedF64(lower_bound), _, _))) = task_heap.peek().copied()
                && lower_bound < candidate.cost
                && !approx_eq(lower_bound, candidate.cost)
            {
                candidate_heap.push(Reverse((
                    OrderedF64(candidate.cost),
                    candidate.ordinal,
                    candidate_index,
                )));
                self.stats.borrow_mut().candidate_heap_pushes += 1;
                continue;
            }

            accepted.push(candidate.path.clone());
            first_deviation_index.push(candidate.first_deviation_index);
            self.add_spur_tasks_for(
                accepted.len() - 1,
                &first_deviation_index,
                &accepted,
                &reverse_distances,
                &mut tasks,
                &mut task_heap,
            );
        }

        self.stats.borrow_mut().deferred_tasks_left = task_heap.len();
        Ok(accepted)
    }

    fn find_spur(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        banned_vertices: &HashSet<G::Vertex>,
        banned_edges: &HashSet<G::Edge>,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        self.stats.borrow_mut().shortest_path_calls += 1;
        let masked = MaskSubgraph::new(
            self.graph,
            |vertex: &G::Vertex| banned_vertices.contains(vertex),
            |edge: &G::Edge| banned_edges.contains(edge),
        );
        Dijkstra::new(&masked).path(source, sink)
    }

    fn compute_reverse_distances(&self, sink: &G::Vertex) -> Result<HashMap<G::Vertex, f64>> {
        let sink = self
            .graph
            .vertices()
            .find(|vertex| *vertex == sink)
            .ok_or(GraphError::VertexNotFound)?;
        let mut distance = HashMap::from([(sink, 0.0)]);
        let mut nodes = vec![sink];
        let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), 0u64, 0usize))]);
        let mut ordinal = 0u64;
        while let Some(Reverse((OrderedF64(value), _, index))) = heap.pop() {
            let vertex = nodes[index];
            if value > distance[vertex] {
                continue;
            }
            for (edge, predecessor) in self.graph.try_incoming_of(vertex)? {
                let weight = self.graph.edge_weight(edge).unwrap_or(1.0);
                if weight.is_nan() || weight < 0.0 {
                    return Err(GraphError::InvalidArgument(
                        "bounded-pruned Yen requires non-negative edge weights".into(),
                    ));
                }
                let candidate = value + weight;
                if candidate < distance.get(predecessor).copied().unwrap_or(f64::INFINITY) {
                    distance.insert(predecessor, candidate);
                    nodes.push(predecessor);
                    ordinal += 1;
                    heap.push(Reverse((OrderedF64(candidate), ordinal, nodes.len() - 1)));
                }
            }
        }
        Ok(distance
            .into_iter()
            .map(|(vertex, value)| (vertex.clone(), value))
            .collect())
    }

    fn add_spur_tasks_for(
        &self,
        path_index: usize,
        first_deviation_index: &[usize],
        accepted: &[GraphWalk<G::Vertex, G::Edge>],
        reverse_distances: &HashMap<G::Vertex, f64>,
        tasks: &mut Vec<SpurTask<G::Vertex>>,
        task_heap: &mut BinaryHeap<Reverse<(OrderedF64, u64, usize)>>,
    ) {
        let path = &accepted[path_index];
        if path.vertices.len() < 2 {
            return;
        }
        let deviation = first_deviation_index[path_index];
        let mut banned_vertices = HashSet::new();
        let mut root_cost = 0.0;
        for spur_index in 0..path.edges.len() {
            if spur_index >= deviation {
                let spur = &path.vertices[spur_index];
                match reverse_distances.get(spur).copied() {
                    Some(reverse) if reverse.is_finite() => {
                        let banned_edges =
                            self.compute_yen_banned_edges(&path.vertices, spur_index, accepted);
                        if self.has_legal_exit(spur, &banned_vertices, &banned_edges) {
                            let mut stats = self.stats.borrow_mut();
                            let ordinal = stats.spur_tasks_created as u64;
                            stats.spur_tasks_created += 1;
                            drop(stats);
                            let lower_bound = root_cost + reverse;
                            let index = tasks.len();
                            tasks.push(SpurTask {
                                parent_index: path_index,
                                spur_index,
                                banned_vertices: banned_vertices.clone(),
                                root_cost,
                                lower_bound,
                                ordinal,
                            });
                            task_heap.push(Reverse((OrderedF64(lower_bound), ordinal, index)));
                        } else {
                            self.stats.borrow_mut().skipped_impossible_spur_tasks += 1;
                        }
                    }
                    _ => self.stats.borrow_mut().unreachable_spurs += 1,
                }
            }
            banned_vertices.insert(path.vertices[spur_index].clone());
            root_cost += self
                .graph
                .edge_weight(&path.edges[spur_index])
                .unwrap_or(1.0);
        }
    }

    fn materialize(
        &self,
        task: &SpurTask<G::Vertex>,
        accepted: &[GraphWalk<G::Vertex, G::Edge>],
    ) -> Result<Option<BoundedCandidate<G::Vertex, G::Edge>>> {
        let parent = &accepted[task.parent_index];
        let banned_edges =
            self.compute_yen_banned_edges(&parent.vertices, task.spur_index, accepted);
        let spur_node = &parent.vertices[task.spur_index];
        let sink = parent.vertices.last().expect("path has a sink");
        let Some(spur) = self.find_spur(spur_node, sink, &task.banned_vertices, &banned_edges)?
        else {
            return Ok(None);
        };

        let mut vertices = parent.vertices[..task.spur_index].to_vec();
        vertices.extend(spur.vertices);
        let mut edges = parent.edges[..task.spur_index].to_vec();
        edges.extend(spur.edges);
        let cost = task.root_cost + spur.weight;
        let mut stats = self.stats.borrow_mut();
        let ordinal = stats.candidate_ordinal;
        stats.candidate_ordinal += 1;
        Ok(Some(BoundedCandidate {
            path: GraphWalk::new(vertices, edges, cost),
            cost,
            first_deviation_index: task.spur_index,
            ordinal,
        }))
    }

    fn compute_yen_banned_edges(
        &self,
        parent_vertices: &[G::Vertex],
        spur_index: usize,
        accepted: &[GraphWalk<G::Vertex, G::Edge>],
    ) -> HashSet<G::Edge> {
        accepted
            .iter()
            .filter(|path| {
                path.vertices.len() > spur_index + 1
                    && path.vertices[..=spur_index] == parent_vertices[..=spur_index]
            })
            .map(|path| path.edges[spur_index].clone())
            .collect()
    }

    fn has_legal_exit(
        &self,
        spur: &G::Vertex,
        banned_vertices: &HashSet<G::Vertex>,
        banned_edges: &HashSet<G::Edge>,
    ) -> bool {
        self.graph
            .outgoing_of(spur)
            .any(|(edge, target)| !banned_edges.contains(edge) && !banned_vertices.contains(target))
    }
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0)
}
