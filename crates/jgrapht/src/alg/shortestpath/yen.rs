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
use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
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

/// Iterator over the shortest loopless paths between two vertices in a graph sorted by weight.
///
/// For this iterator to work correctly the graph must not be modified during iteration. Currently
/// there are no means to ensure that, nor to fail-fast. The results of such modifications are
/// undefined.
///
/// The main idea of this algorithm is to divide each path between the `source` and the
/// `sink` into the root part - the part that coincides within some of the paths computed so
/// far, and the spur part, the part that deviates from all other paths computed so far. Therefore,
/// for each path the algorithm maintains a vertex, at which the path deviates from its "parent" path
/// (the candidate path using which it was computed).
///
/// First the algorithm finds the shortest path between the `source` and the `sink`,
/// which is put into the candidates heap. The `source` is assigned to be its deviation vertex.
/// Then on each iteration the algorithm takes a candidate from the heap with minimum weight, puts it
/// into the result list and builds all possible deviations from it wrt. other paths, that are in the
/// result list. By generating spur paths starting only from the vertices that are after the
/// deviation vertex of current path (including the deviation vertex) it is possible to avoid
/// building duplicated candidates.
///
/// Additionally, the algorithm supports path validation by means of `PathValidator`.
///
/// # Rust notes
///
/// Lazy iterator over loopless paths in non-decreasing weight order.
///
/// Items are `Result`s: an error raised while computing a path (for example an invalid edge
/// weight met by the spur searches) is yielded once as `Err`, and the iterator then ends, so a
/// failure is never mistaken for exhaustion. Collect with `collect::<Result<Vec<_>>>()`.
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
    failed: bool,
}

impl<'g, G: OutgoingGraph> YenShortestPathIterator<'g, G, AllowAllPaths> {
    /// Constructs an instance of the algorithm for given `graph`, `source` and
    /// `sink`.
    pub fn new(graph: &'g G, source: &G::Vertex, sink: &G::Vertex) -> Result<Self> {
        Self::with_validator(graph, source, sink, AllowAllPaths)
    }
}

impl<'g, G: OutgoingGraph, P: PathValidator<G>> YenShortestPathIterator<'g, G, P> {
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
            failed: false,
        })
    }

    fn path_is_valid(&self, path: &GraphWalk<G::Vertex, G::Edge>) -> bool {
        let mut weight = 0.0;
        for i in 0..path.edges.len() {
            let partial = GraphWalk::new_unchecked(
                path.vertices[..=i].to_vec(),
                path.edges[..i].to_vec(),
                weight,
            );
            if !self.validator.is_valid_path(&partial, &path.edges[i]) {
                return false;
            }
            weight += self.graph.weight_of(&path.edges[i]);
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

    /// Builds unique loopless deviations from the given path in the `graph`. First receives
    /// the deviation vertex of the current path as well as sets of vertices and edges to be masked
    /// during the computations. Then creates an instance of the `MaskSubgraph` and builds a
    /// reversed shortest paths tree starting at `sink` in it. Finally builds new candidate
    /// paths by deviating from the vertices of the provided `path`. Puts only those candidates
    /// in the `candidatesList`, which deviate from `path` between $firstDeviation$ and
    /// $lastDeviation$. $firstDeviation$ and $lastDeviation$ are obtainer from
    /// `firstDeviations` and `lastDeviations` correspondingly.
    ///
    /// For more information on this step refer to the article with the original description of the
    /// algorithm.
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
                .map(|edge| self.graph.weight_of(edge))
                .sum();
            self.push_candidate(GraphWalk::new_unchecked(
                vertices,
                edges,
                root_weight + spur_path.weight,
            ));
        }
        Ok(())
    }

    /// The next path, or `None` once every path has been produced.
    fn advance(&mut self) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
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

impl<G: OutgoingGraph, P: PathValidator<G>> Iterator for YenShortestPathIterator<'_, G, P> {
    type Item = Result<GraphWalk<G::Vertex, G::Edge>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let step = self.advance().transpose();
        if matches!(step, Some(Err(_))) {
            self.failed = true;
        }
        step
    }
}

/// Implementation of Yen's algorithm for finding $k$ shortest loopless paths.
///
/// The time complexity of the algorithm is $O(kn(m + n log n))$, where $n$ is the number of vertices
/// in the graph, $m$ is the number of edges in the graph and $k$ is the number of paths needed.
///
/// The algorithm is originally described in: Q. V. Martins, Ernesto and M. B. Pascoal, Marta.
/// (2003). A new implementation of Yen’s ranking loopless paths algorithm. Quarterly Journal of the
/// Belgian, French and Italian Operations Research Societies. 1. 121-133. 10.1007/s10288-002-0010-2.
///
/// The implementation iterates over the existing loopless path between the `source` and the
/// `sink` and forms the resulting list. It is possible to provide a `PathValidator` to
/// filter the resulting path list
pub struct YenKShortestPath<'g, G: Graph, P = AllowAllPaths> {
    graph: &'g G,
    validator: P,
}

impl<'g, G: Graph> YenKShortestPath<'g, G, AllowAllPaths> {
    /// Constructs an instance of the algorithm for the given `graph`.
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            validator: AllowAllPaths,
        }
    }
}

impl<'g, G: OutgoingGraph, P: PathValidator<G> + Clone> YenKShortestPath<'g, G, P> {
    pub fn with_validator(graph: &'g G, validator: P) -> Self {
        Self { graph, validator }
    }

    /// Computes `k` shortest loopless paths between `source` and `sink`. If the
    /// overall number of such paths is denoted by $n$, the method returns $m = min\{k, n\}$ such
    /// paths. The paths are produced in sorted order by weights.
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
            let Some(path) = iterator.next().transpose()? else {
                break;
            };
            result.push(path);
        }
        Ok(result)
    }
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
    /// Vertices expanded by spur engines which expose their settled-vertex count.
    pub expanded_vertices: usize,
    pub skipped_impossible_spur_tasks: usize,
    candidate_ordinal: u64,
}

mod spur_engine_private {
    pub trait Sealed {}
}

/// Shortest-path backend used to materialize bounded/pruned Yen spur tasks.
pub trait SpurShortestPathEngine: spur_engine_private::Sealed + Copy {
    #[doc(hidden)]
    fn uses_astar(self) -> bool;

    /// Symbolic engine name used in diagnostics and benchmarks.
    fn name(self) -> &'static str;
}

/// Dijkstra spur backend used by [`BoundedPrunedYenKShortestPath`] by default.
#[derive(Debug, Clone, Copy, Default)]
pub struct DijkstraSpurEngine;

impl DijkstraSpurEngine {
    pub fn new() -> Self {
        Self
    }
}

impl spur_engine_private::Sealed for DijkstraSpurEngine {}

impl SpurShortestPathEngine for DijkstraSpurEngine {
    fn uses_astar(self) -> bool {
        false
    }

    fn name(self) -> &'static str {
        "Dijkstra"
    }
}

/// A* spur backend using original-graph reverse distances as an admissible heuristic.
#[derive(Debug, Clone, Copy, Default)]
pub struct AStarSpurEngine;

impl AStarSpurEngine {
    pub fn new() -> Self {
        Self
    }
}

impl spur_engine_private::Sealed for AStarSpurEngine {}

impl SpurShortestPathEngine for AStarSpurEngine {
    fn uses_astar(self) -> bool {
        true
    }

    fn name(self) -> &'static str {
        "AStar"
    }
}

#[derive(Debug, Clone, Copy)]
enum SpurEngineKind {
    Dijkstra,
    AStar,
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

/// Experimental bounded-pruned variant of Yen's $k$ shortest loopless paths algorithm.
///
/// This class returns the same ordered sequence of path weights as `YenKShortestPath` (and is
/// regression-tested against it) but tries to perform fewer spur shortest-path computations and
/// fewer candidate materializations by deferring spur work behind *lower-bound certificates*.
///
/// **Key idea**
///
/// Standard Yen, when accepting a path $P$, eagerly enumerates every spur index of $P$, runs a
/// shortest-path query under the appropriate banning, materializes the full candidate path, and
/// pushes it into the candidate heap. For large $k$ on dense graphs this generates many candidates
/// that will never be output.
///
/// The bounded-pruned variant defers each spur enumeration as a `SpurTask` carrying:
///
///   - the parent accepted path,
///   - the spur index in that parent,
///   - the banned vertex set (the parent's prefix),
///   - the root cost so far,
///   - and a *lower bound* on any candidate it could ever produce:
///     `lowerBound = rootCost + reverseDistanceToSink[spurNode]`.
///
/// Reverse distances are computed once on the original graph. Because removing vertices/edges can
/// only increase shortest-path distance, those reverse distances remain valid lower bounds even
/// under arbitrary bans. So:
///
/// If the cheapest materialized candidate already satisfies
/// `candidate.cost ≤ task.lowerBound` for every still-deferred task, it is safe to output
/// the candidate as the next path — no deferred task could possibly produce something better.
///
/// **Algorithm sketch**
///
/// ```text
///   firstPath = shortestPath(source, sink)
///   accepted.add(firstPath)
///   addSpurTasks(firstPath)
///   while accepted.size < k:
///       while taskHeap not empty AND
///             (candidateHeap empty OR taskHeap.minLB < candidateHeap.minCost):
///           materialize(taskHeap.pop())  // run the actual spur query, push candidate
///       if candidateHeap empty: break
///       next = candidateHeap.pop()
///       accepted.add(next)
///       addSpurTasks(next)
/// ```
///
/// **Exactness**
///
/// The returned path-weight sequence equals that of `YenKShortestPath`. Tie-breaking among
/// paths of equal weight is deterministic: a stable ordinal is attached to every candidate / task
/// push so that two equal weights are broken by insertion order, which mirrors how the standard
/// implementation eagerly inserts candidates as it accepts each path. The benchmark harness
/// verifies the path-weight sequences match exactly across many graph families.
///
/// **What this class does NOT yet claim**
///
///   - No proven better worst-case complexity: Yen's $O(k\,n\,(m + n \log n))$ remains the
///     upper bound. The bounded-pruned layer is an output-sensitive optimisation only.
///
/// **References**
///
///   - Yen, J. Y. (1971). Finding the k shortest loopless paths in a network.
///     *Management Science*, 17(11), 712–716. The original loopless k-shortest-paths
///     algorithm whose spur step is reused here.
///   - Martins, E. Q. V., & Pascoal, M. M. B. (2003). A new implementation of Yen's
///     ranking loopless paths algorithm. *4OR — Quarterly Journal of the Belgian, French
///     and Italian Operations Research Societies*, 1(2), 121–133. Practical
///     implementation notes that informed JGraphT's existing `YenKShortestPath`.
///   - Aljazzar, H., & Leue, S. (2011). K*: A heuristic search algorithm for finding the
///     k shortest paths. *Artificial Intelligence*, 175(18), 2129–2154. The closest
///     prior published precedent for using admissible lower bounds (an A*-style heuristic on
///     the reverse graph) to defer expansion of provably non-improving k-paths candidates.
///     This class applies the same lower-bound-then-defer idea at the Yen spur-task level
///     rather than at K*'s edge-expansion level.
///
/// # Rust notes
///
/// Bounded/pruned Yen algorithm with deferred lower-bound spur tasks.
///
/// Each spur task is ordered by `root_cost + distance_to_sink(spur)`. A task
/// is materialized only while its lower bound can tie or beat the cheapest
/// materialized candidate. Equal bounds and path weights use stable insertion
/// ordinals, matching the Java implementation's priority queues.
pub struct BoundedPrunedYenKShortestPath<'g, G: Graph> {
    graph: &'g G,
    engine: SpurEngineKind,
    stats: RefCell<BoundedPrunedYenStats>,
}

impl<'g, G: IncidenceGraph> BoundedPrunedYenKShortestPath<'g, G> {
    /// Default constructor using `DijkstraSpurEngine` as the spur shortest-path back-end.
    /// This default mirrors the spur step of the existing `YenKShortestPath` for
    /// principle-of-least-surprise. Pass an `AStarSpurEngine` via
    /// `BoundedPrunedYenKShortestPath(Graph, SpurShortestPathEngine)` to opt into the
    /// reverse-distance A* heuristic — recommended for dense graphs or large k. The A* engine
    /// is exact (the heuristic is admissible by construction).
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            engine: SpurEngineKind::Dijkstra,
            stats: RefCell::new(BoundedPrunedYenStats::default()),
        }
    }

    /// Bind the algorithm to a graph and choose its spur shortest-path backend.
    pub fn with_engine<E: SpurShortestPathEngine>(graph: &'g G, engine: E) -> Self {
        Self {
            graph,
            engine: if engine.uses_astar() {
                SpurEngineKind::AStar
            } else {
                SpurEngineKind::Dijkstra
            },
            stats: RefCell::new(BoundedPrunedYenStats::default()),
        }
    }

    /// Symbolic name of the configured spur engine.
    pub fn engine_name(&self) -> &'static str {
        match self.engine {
            SpurEngineKind::Dijkstra => "Dijkstra",
            SpurEngineKind::AStar => "AStar",
        }
    }

    pub fn stats(&self) -> BoundedPrunedYenStats {
        self.stats.borrow().clone()
    }

    /// Get a list of k-shortest paths from a source vertex to a sink vertex. If no such paths exist
    /// this method returns an empty list.
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
        let Some(first) = self.find_spur(
            source,
            sink,
            &HashSet::new(),
            &HashSet::new(),
            &reverse_distances,
        )?
        else {
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
        )?;

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
                let Some(candidate) =
                    self.materialize(&tasks[task_index], &accepted, &reverse_distances)?
                else {
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
            )?;
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
        reverse_distances: &HashMap<G::Vertex, f64>,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        self.stats.borrow_mut().shortest_path_calls += 1;
        let masked = MaskSubgraph::new(
            self.graph,
            |vertex: &G::Vertex| banned_vertices.contains(vertex),
            |edge: &G::Edge| banned_edges.contains(edge),
        );
        match self.engine {
            SpurEngineKind::Dijkstra => Dijkstra::new(&masked).path(source, sink),
            SpurEngineKind::AStar => {
                let (path, expanded) = Self::astar_spur(&masked, source, sink, reverse_distances)?;
                self.stats.borrow_mut().expanded_vertices += expanded;
                Ok(path)
            }
        }
    }

    fn astar_spur<M: Graph<Vertex = G::Vertex, Edge = G::Edge> + OutgoingGraph>(
        graph: &M,
        source: &G::Vertex,
        sink: &G::Vertex,
        reverse_distances: &HashMap<G::Vertex, f64>,
    ) -> Result<(Option<GraphWalk<G::Vertex, G::Edge>>, usize)> {
        if source == sink {
            return Ok((
                Some(GraphWalk::new_unchecked(
                    vec![source.clone()],
                    Vec::new(),
                    0.0,
                )),
                0,
            ));
        }

        let heuristic = |vertex: &G::Vertex| {
            reverse_distances
                .get(vertex)
                .copied()
                .filter(|value| value.is_finite())
                .unwrap_or(0.0)
        };
        let mut distance = HashMap::from([(source.clone(), 0.0)]);
        let mut predecessor: HashMap<G::Vertex, (G::Edge, G::Vertex)> = HashMap::new();
        let mut entries = vec![(source.clone(), 0.0)];
        let mut heap = BinaryHeap::from([Reverse((OrderedF64(heuristic(source)), 0u64, 0usize))]);
        let mut ordinal = 0u64;
        let mut expanded = 0usize;

        while let Some(Reverse((_, _, entry_index))) = heap.pop() {
            let (vertex, value) = entries[entry_index].clone();
            if value != distance[&vertex] {
                continue;
            }
            expanded += 1;
            if &vertex == sink {
                break;
            }
            let outgoing = graph
                .outgoing_of(&vertex)
                .map(|(edge, target)| (edge.clone(), target.clone()))
                .collect::<Vec<_>>();
            for (edge, target) in outgoing {
                let weight = graph.weight_of(&edge);
                if weight.is_nan() || weight < 0.0 {
                    return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
                }
                let candidate = value + weight;
                if candidate < distance.get(&target).copied().unwrap_or(f64::INFINITY) {
                    distance.insert(target.clone(), candidate);
                    predecessor.insert(target.clone(), (edge, vertex.clone()));
                    entries.push((target.clone(), candidate));
                    ordinal += 1;
                    heap.push(Reverse((
                        OrderedF64(candidate + heuristic(&target)),
                        ordinal,
                        entries.len() - 1,
                    )));
                }
            }
        }

        let Some(&weight) = distance.get(sink).filter(|value| value.is_finite()) else {
            return Ok((None, expanded));
        };
        let mut vertices = vec![sink.clone()];
        let mut edges = Vec::new();
        let mut current = sink.clone();
        while &current != source {
            let Some((edge, parent)) = predecessor.get(&current) else {
                return Ok((None, expanded));
            };
            edges.push(edge.clone());
            vertices.push(parent.clone());
            current = parent.clone();
        }
        vertices.reverse();
        edges.reverse();
        Ok((
            Some(GraphWalk::new_unchecked(vertices, edges, weight)),
            expanded,
        ))
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
            for (edge, predecessor) in self.graph.incoming_of(vertex) {
                let weight = self.graph.weight_of(edge);
                if weight.is_nan() || weight < 0.0 {
                    return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
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
    ) -> Result<()> {
        let path = &accepted[path_index];
        if path.vertices.len() < 2 {
            return Ok(());
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
                        if self.has_legal_exit(spur, &banned_vertices, &banned_edges)? {
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
            root_cost += self.graph.weight_of(&path.edges[spur_index]);
        }
        Ok(())
    }

    fn materialize(
        &self,
        task: &SpurTask<G::Vertex>,
        accepted: &[GraphWalk<G::Vertex, G::Edge>],
        reverse_distances: &HashMap<G::Vertex, f64>,
    ) -> Result<Option<BoundedCandidate<G::Vertex, G::Edge>>> {
        let parent = &accepted[task.parent_index];
        let banned_edges =
            self.compute_yen_banned_edges(&parent.vertices, task.spur_index, accepted);
        let spur_node = &parent.vertices[task.spur_index];
        let sink = parent.vertices.last().expect("path has a sink");
        let Some(spur) = self.find_spur(
            spur_node,
            sink,
            &task.banned_vertices,
            &banned_edges,
            reverse_distances,
        )?
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
            path: GraphWalk::new_unchecked(vertices, edges, cost),
            cost,
            first_deviation_index: task.spur_index,
            ordinal,
        }))
    }

    /// Compute the set of edges that standard Yen would ban at `spurIndex` given the
    /// current `accepted` list. An edge is banned iff there exists a previously accepted
    /// path whose first `spurIndex+1` vertices match `parentVertices` and whose
    /// `spurIndex`-th edge would otherwise be the next edge of the spur.
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

    /// True iff the spur node has at least one outgoing edge that survives the current bans
    /// (banned-vertex set + banned-edge set). When this is false the spur is provably impossible
    /// and the task can be skipped exactly — adding more bans later can only remove legal edges,
    /// never add them, so a "false" verdict here is stable through the rest of the run.
    fn has_legal_exit(
        &self,
        spur: &G::Vertex,
        banned_vertices: &HashSet<G::Vertex>,
        banned_edges: &HashSet<G::Edge>,
    ) -> Result<bool> {
        Ok(self.graph.outgoing_of(spur).any(|(edge, target)| {
            !banned_edges.contains(edge) && !banned_vertices.contains(target)
        }))
    }
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0)
}
