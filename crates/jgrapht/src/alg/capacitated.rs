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

//! Capacitated minimum spanning trees: shared representation and the Esau–Williams heuristic
//! (`AbstractCapacitatedMinimumSpanningTree`, `EsauWilliamsCapacitatedMinimumSpanningTree`).
//!
//! The vertices other than the root are partitioned into subsets whose total demand does not
//! exceed the capacity; every subset hangs below the root as one or more subtrees. Hash sets and
//! maps of the Java code are insertion-ordered here, which makes the heuristics deterministic.

use super::connectivity::ConnectivityInspector;
use super::spanning::prim;
use super::util::Pair;
use crate::graph::Graph;
use crate::graph::GraphExt;
use crate::graph::IncidenceGraph;
use crate::traverse::DepthFirstIterator;
use crate::view::AsSubgraph;
use crate::{GraphError, Result};
use indexmap::{IndexMap, IndexSet};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

/// A capacitated spanning tree (`CapacitatedSpanningTreeAlgorithm.CapacitatedSpanningTree`).
#[derive(Debug, Clone)]
pub struct CapacitatedSpanningTree<V: Eq + Hash, E: Eq + Hash> {
    /// The partition label of every vertex except the root.
    pub labels: IndexMap<V, usize>,
    /// For each label the vertices of the subset and their total demand.
    pub partition: IndexMap<usize, Pair<IndexSet<V>, f64>>,
    /// Edges of the tree.
    pub edges: IndexSet<E>,
    /// Total weight of the tree.
    pub weight: f64,
}

impl<V: Eq + Hash + Clone, E: Eq + Hash + Clone> CapacitatedSpanningTree<V, E> {
    /// Checks the tree against the problem data (`isCapacitatedSpanningTree`).
    pub fn is_capacitated_spanning_tree<G: IncidenceGraph<Vertex = V, Edge = E>>(
        &self,
        graph: &G,
        root: &V,
        capacity: f64,
        demands: &HashMap<V, f64>,
    ) -> bool {
        if self.edges.len() != graph.vertex_count() - 1 {
            return false;
        }
        // disjointness
        for (i, a) in self.partition.values().enumerate() {
            for (j, b) in self.partition.values().enumerate() {
                if i != j && !a.first.is_disjoint(&b.first) {
                    return false;
                }
            }
        }
        // demands and number of vertices
        let mut explored = 0;
        for part in self.partition.values() {
            let mut current = 0i32;
            for v in &part.first {
                current = (current as f64 + demands[v]) as i32;
                explored += 1;
            }
            if current as f64 > capacity {
                return false;
            }
        }
        if graph.vertex_count() - 1 != explored {
            return false;
        }
        // partition and tree correspond to each other
        let tree =
            AsSubgraph::with_edges(graph, graph.vertices_cloned(), self.edges.iter().cloned());
        let Ok(mut dfs) = DepthFirstIterator::from_start(&tree, root) else {
            return false;
        };
        dfs.next();
        let mut root_edges_explored = 0;
        let mut current: IndexSet<V> = IndexSet::new();
        for next in dfs {
            if tree.contains_edge_between(root, &next) {
                if !current.is_empty() {
                    let first = current.iter().next().unwrap();
                    match self.labels.get(first).and_then(|l| self.partition.get(l)) {
                        Some(p) if p.first == current => {}
                        _ => return false,
                    }
                    current = IndexSet::new();
                }
                root_edges_explored += 1;
            }
            current.insert(next.clone());
        }
        root_edges_explored == tree.degree_of(root)
    }
}

/// An algorithm which computes a capacitated (minimum) spanning tree of a given connected graph with
/// a designated root vertex. The input is a connected undirected graph G = (V, E) with a designated
/// root r \in V, a capacity constraint K \in \mathbb{N}, a demand function d: V \rightarrow
/// \mathbb{N} and a capacity function c: E \rightarrow \mathbb{N}. A
/// [Capacitated Minimum Spanning Tree](https://en.wikipedia.org/wiki/Capacitated_minimum_spanning_tree) (CMST) is a rooted minimal cost spanning tree that satisfies the capacity
/// constraint on all trees that are connected to the designated root. That is, the sum of the
/// demands of all vertices is smaller or equal than K. These trees build up a partition on the
/// vertex set of the graph. The problem is NP-hard.
///
/// # Rust notes
///
/// An algorithm computing a capacitated spanning tree (`CapacitatedSpanningTreeAlgorithm`).
pub trait CapacitatedSpanningTreeAlgorithm<V: Eq + Hash, E: Eq + Hash> {
    /// Returns the capacitated spanning tree.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph is not undirected, an edge weight is invalid, or the capacity, demand or root arguments are inconsistent with the graph.
    fn capacitated_spanning_tree(&mut self) -> Result<CapacitatedSpanningTree<V, E>>;
}

/// Problem data with the validation of `AbstractCapacitatedMinimumSpanningTree`.
pub(crate) struct Problem<'g, G: IncidenceGraph> {
    pub graph: &'g G,
    pub root: G::Vertex,
    pub capacity: f64,
    pub demands: HashMap<G::Vertex, f64>,
    /// Position of every vertex in the graph; Java's hash sets of small integers iterate in
    /// this order, so sets are traversed sorted by it.
    index: HashMap<G::Vertex, usize>,
}

impl<G: IncidenceGraph> Clone for Problem<'_, G> {
    fn clone(&self) -> Self {
        Problem {
            graph: self.graph,
            root: self.root.clone(),
            capacity: self.capacity,
            demands: self.demands.clone(),
            index: self.index.clone(),
        }
    }
}

impl<'g, G: IncidenceGraph> Problem<'g, G> {
    pub fn new(
        graph: &'g G,
        root: G::Vertex,
        capacity: f64,
        demands: HashMap<G::Vertex, f64>,
    ) -> Result<Self> {
        if graph.graph_type().directed {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected));
        }
        if !ConnectivityInspector::new().is_connected(graph) {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Connected));
        }
        for v in graph.vertices() {
            let v = v.borrow();
            if *v != root {
                let Some(&d) = demands.get(v) else {
                    return Err(GraphError::MissingVertexData("demand"));
                };
                if d > capacity {
                    return Err(GraphError::Infeasible);
                }
            }
        }
        let index = graph
            .vertices()
            .enumerate()
            .map(|(i, v)| (v.borrow().clone(), i))
            .collect();
        Ok(Problem {
            graph,
            root,
            capacity,
            demands,
            index,
        })
    }

    /// The elements of `set` in graph order.
    pub fn sorted<'a>(&self, set: &'a IndexSet<G::Vertex>) -> Vec<&'a G::Vertex> {
        let mut v: Vec<_> = set.iter().collect();
        v.sort_by_key(|x| self.index[*x]);
        v
    }

    fn demand(&self, v: &G::Vertex) -> f64 {
        self.demands[v]
    }

    /// Minimum spanning tree of the subgraph induced by `vertices` plus the root.
    fn mst_with_root(&self, vertices: &IndexSet<G::Vertex>) -> (Vec<G::Edge>, f64) {
        let sub = AsSubgraph::induced(self.graph, vertices.iter().cloned());
        let t = prim(&sub);
        (t.edges, t.weight)
    }
}

/// `CapacitatedSpanningTreeSolutionRepresentation`: vertex labels plus the implied partition.
#[derive(Debug, Clone)]
pub(crate) struct Solution<V: Eq + Hash + Clone> {
    pub labels: IndexMap<V, usize>,
    pub partition: IndexMap<usize, Pair<IndexSet<V>, f64>>,
    next_free_label: usize,
}

impl<V: Eq + Hash + Clone> Solution<V> {
    pub fn empty() -> Self {
        Self::new(IndexMap::new(), IndexMap::new())
    }

    pub fn new(
        labels: IndexMap<V, usize>,
        partition: IndexMap<usize, Pair<IndexSet<V>, f64>>,
    ) -> Self {
        let mut s = Solution {
            labels,
            partition,
            next_free_label: 0,
        };
        s.next_free_label();
        s
    }

    pub fn resulting_spanning_tree<G: IncidenceGraph<Vertex = V>>(
        &self,
        problem: &Problem<'_, G>,
    ) -> CapacitatedSpanningTree<V, G::Edge> {
        let mut edges = IndexSet::new();
        let mut weight = 0.0;
        for part in self.partition.values() {
            let mut set = part.first.clone();
            set.insert(problem.root.clone());
            let (tree_edges, w) = problem.mst_with_root(&set);
            edges.extend(tree_edges);
            weight += w;
        }
        CapacitatedSpanningTree {
            labels: self.labels.clone(),
            partition: self.partition.clone(),
            edges,
            weight,
        }
    }

    pub fn move_vertex<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        problem: &Problem<'_, G>,
        vertex: &V,
        from: usize,
        to: usize,
    ) {
        self.labels.insert(vertex.clone(), to);
        let d = problem.demand(vertex);
        let old = self.partition.get_mut(&from).unwrap();
        old.first.shift_remove(vertex);
        old.second -= d;
        let new = self
            .partition
            .entry(to)
            .or_insert_with(|| Pair::of(IndexSet::new(), 0.0));
        new.first.insert(vertex.clone());
        new.second += d;
    }

    pub fn move_vertices<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        problem: &Problem<'_, G>,
        vertices: &IndexSet<V>,
        from: usize,
        to: usize,
    ) {
        let mut moved_weight = 0.0;
        for v in vertices {
            moved_weight += problem.demand(v);
            self.labels.insert(v.clone(), to);
        }
        let new = self
            .partition
            .entry(to)
            .or_insert_with(|| Pair::of(IndexSet::new(), 0.0));
        new.first.extend(vertices.iter().cloned());
        new.second += moved_weight;
        let old = self.partition.get_mut(&from).unwrap();
        old.first.retain(|v| !vertices.contains(v));
        old.second -= moved_weight;
    }

    /// Splits the vertices of `vertex_subset` (a subset of the partition set `label`) into one
    /// part per subtree below the root of their minimum spanning tree; returns the new labels.
    pub fn partition_subtrees_of_subset<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        problem: &Problem<'_, G>,
        vertex_subset: &IndexSet<V>,
        label: usize,
    ) -> IndexSet<usize> {
        if vertex_subset.is_empty() {
            return IndexSet::new();
        }
        let mut with_root = vertex_subset.clone();
        with_root.insert(problem.root.clone());
        let (tree_edges, _) = problem.mst_with_root(&with_root);
        let tree = AsSubgraph::with_edges(problem.graph, with_root.iter().cloned(), tree_edges);
        let degree_of_root = tree.degree_of(&problem.root);
        if degree_of_root == 1 {
            return IndexSet::new();
        }
        let mut subtrees: Vec<IndexSet<V>> = Vec::new();
        let mut dfs =
            DepthFirstIterator::from_start(&tree, &problem.root).expect("root is in the tree");
        dfs.next();
        let mut root_edges_explored = 0;
        let mut current: IndexSet<V> = IndexSet::new();
        for next in dfs {
            if tree.contains_edge_between(&problem.root, &next) {
                if !current.is_empty() {
                    subtrees.push(std::mem::take(&mut current));
                }
                root_edges_explored += 1;
                // the vertices of the last subtree stay where they are
                if root_edges_explored == degree_of_root {
                    break;
                }
            }
            current.insert(next.clone());
        }
        let mut affected = IndexSet::new();
        for subtree in subtrees {
            let next = self.next_free_label();
            self.move_vertices(problem, &subtree, label, next);
            affected.insert(next);
        }
        affected
    }

    /// The labels in ascending order (Java's `HashMap<Integer, ..>` iteration order).
    pub fn labels_sorted(&self) -> Vec<usize> {
        let mut l: Vec<usize> = self.partition.keys().copied().collect();
        l.sort_unstable();
        l
    }

    pub fn clean_up(&mut self) {
        self.partition.retain(|_, p| !p.first.is_empty());
    }

    pub fn next_free_label(&mut self) -> usize {
        let free = self.next_free_label;
        self.next_free_label += 1;
        while self.partition.contains_key(&self.next_free_label) {
            self.next_free_label += 1;
        }
        free
    }

    pub fn label(&self, v: &V) -> usize {
        self.labels[v]
    }

    pub fn partition_set(&self, label: usize) -> &IndexSet<V> {
        &self.partition[&label].first
    }

    pub fn partition_weight(&self, label: usize) -> f64 {
        self.partition[&label].second
    }
}

/// Implementation of a randomized version of the Esau-Williams heuristic, a greedy randomized
/// adaptive search heuristic (GRASP) for the capacitated minimum spanning tree (CMST) problem. It
/// calculates a suboptimal CMST. The original version can be found in L. R. Esau and K. C. Williams.
/// 1966. On teleprocessing system design: part II a method for approximating the optimal network.
/// IBM Syst. J. 5, 3 (September 1966), 142-147. DOI=<http://dx.doi.org/10.1147/sj.53.0142> This
/// implementation runs in polynomial time O(|V|^3).
///
/// This implementation is a randomized version described in Ahuja, Ravindra K., Orlin, James B., and
/// Sharma, Dushyant, (1998). New neighborhood search structures for the capacitated minimum spanning
/// tree problem, No WP 4040-98. Working papers, Massachusetts Institute of Technology (MIT), Sloan
/// School of Management.
///
/// This version runs in polynomial time dependent on the number of considered operations per
/// iteration `numberOfOperationsParameter` (denoted by p), such that runs is in $O(|V|^3 +
/// p|V|) = O(|V|^3)$ since $p \leq |V|$.
///
/// A [Capacitated Minimum Spanning Tree](https://en.wikipedia.org/wiki/Capacitated_minimum_spanning_tree) (CMST) is a rooted minimal cost spanning tree that satisfies the capacity
/// constrained on all trees that are connected to the designated root. The problem is NP-hard.
///
/// # Rust notes
///
/// The Esau–Williams savings heuristic (`EsauWilliamsCapacitatedMinimumSpanningTree`).
///
/// `number_of_operations` best moves are kept per round and one of them is picked at random;
/// use [`with_seed`](Self::with_seed) for a reproducible run.
pub struct EsauWilliamsCapacitatedMinimumSpanningTree<'g, G: IncidenceGraph> {
    problem: Problem<'g, G>,
    number_of_operations: usize,
    best: Solution<G::Vertex>,
    executed: bool,
    rng: StdRng,
}

impl<'g, G: IncidenceGraph> EsauWilliamsCapacitatedMinimumSpanningTree<'g, G> {
    /// Constructs an Esau-Williams GRASP algorithm instance.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(
        graph: &'g G,
        root: G::Vertex,
        capacity: f64,
        demands: HashMap<G::Vertex, f64>,
        number_of_operations: usize,
    ) -> Result<Self> {
        Ok(EsauWilliamsCapacitatedMinimumSpanningTree {
            problem: Problem::new(graph, root, capacity, demands)?,
            number_of_operations,
            best: Solution::empty(),
            executed: false,
            rng: StdRng::from_os_rng(),
        })
    }

    /// Creates an instance with the given seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = StdRng::seed_from_u64(seed);
        self
    }

    /// Calculates a partition representation of the capacitated spanning tree. With that, it is
    /// possible to calculate the to the partition corresponding capacitated spanning tree in
    /// polynomial time by calculating the MSTs. The labels of the partition that are returned are
    /// non-negative.
    pub(crate) fn solution(&mut self) -> Solution<G::Vertex> {
        let p = &self.problem;
        let mut labels: IndexMap<G::Vertex, usize> = IndexMap::new();
        let mut partition: IndexMap<usize, Pair<IndexSet<G::Vertex>, f64>> = IndexMap::new();
        let mut counter = 0;
        for v in p.graph.vertices() {
            let v = v.borrow();
            if *v != p.root {
                labels.insert(v.clone(), counter);
                partition.insert(counter, Pair::of(IndexSet::from([v.clone()]), p.demand(v)));
                counter += 1;
            }
        }
        let mut best = Solution::new(labels, partition);
        let mut savings: IndexMap<G::Vertex, f64> = IndexMap::new();
        let mut closest_vertex: HashMap<G::Vertex, G::Vertex> = HashMap::new();
        let mut restriction: HashMap<G::Vertex, IndexSet<usize>> = HashMap::new();
        let mut shortest_gate: HashMap<usize, G::Vertex> = HashMap::new();
        let mut vertices: IndexSet<G::Vertex> = p.graph.vertices_cloned().collect();
        vertices.shift_remove(&p.root);

        loop {
            for v in vertices.clone() {
                let Some(closest) =
                    self.closest_vertex_of(&best, &v, &mut restriction, &shortest_gate)
                else {
                    // v cannot be connected to any vertex
                    vertices.shift_remove(&v);
                    savings.shift_remove(&v);
                    continue;
                };
                let gate = shortest_gate
                    .get(&best.label(&v))
                    .cloned()
                    .unwrap_or_else(|| v.clone());
                savings.insert(
                    v.clone(),
                    self.distance(&gate, &self.problem.root) - self.distance(&v, &closest),
                );
                closest_vertex.insert(v, closest);
            }
            let best_vertices = self.list_of_best_options(&savings);
            if best_vertices.is_empty() {
                break;
            }
            let pick = self.rng.random_range(0..best_vertices.len());
            let to_move = &best_vertices[pick];
            let label_move = best.label(to_move);
            let closest_move = &closest_vertex[to_move];
            let label_closest = best.label(closest_move);
            let gate1 = shortest_gate
                .get(&label_move)
                .cloned()
                .unwrap_or_else(|| to_move.clone());
            let gate2 = shortest_gate
                .get(&label_closest)
                .cloned()
                .unwrap_or_else(|| closest_move.clone());
            let root = &self.problem.root;
            let nearer = if self.distance(&gate1, root) < self.distance(&gate2, root) {
                gate1
            } else {
                gate2
            };
            if best.partition_weight(label_move) < best.partition_weight(label_closest) {
                let set = best.partition_set(label_move).clone();
                best.move_vertices(&self.problem, &set, label_move, label_closest);
                shortest_gate.insert(label_closest, nearer);
            } else {
                let set = best.partition_set(label_closest).clone();
                best.move_vertices(&self.problem, &set, label_closest, label_move);
                shortest_gate.insert(label_move, nearer);
            }
        }

        let mut result = Solution::new(best.labels, best.partition);
        result.clean_up();
        for label in result.labels_sorted() {
            let set = result.partition_set(label).clone();
            result.partition_subtrees_of_subset(&self.problem, &set, label);
        }
        result
    }

    /// Returns the list of the best options as stored in `savings`.
    fn list_of_best_options(&self, savings: &IndexMap<G::Vertex, f64>) -> Vec<G::Vertex> {
        let mut best: Vec<G::Vertex> = Vec::new();
        for (key, &value) in savings {
            let mut position = 0;
            for v in &best {
                if savings[v] < value {
                    break;
                }
                position += 1;
            }
            if best.len() == self.number_of_operations {
                if position < best.len() {
                    best.pop();
                    best.insert(position, key.clone());
                }
            } else {
                best.push(key.clone());
            }
        }
        best
    }

    fn closest_vertex_of(
        &self,
        best: &Solution<G::Vertex>,
        vertex: &G::Vertex,
        restriction: &mut HashMap<G::Vertex, IndexSet<usize>>,
        shortest_gate: &HashMap<usize, G::Vertex>,
    ) -> Option<G::Vertex> {
        let p = &self.problem;
        let mut closest = None;
        let mut distance_to_root = match shortest_gate.get(&best.label(vertex)) {
            Some(gate) => self.distance(gate, &p.root),
            None => self.distance(vertex, &p.root),
        };
        for label in best.labels_sorted() {
            if restriction.get(vertex).is_some_and(|r| r.contains(&label)) {
                continue;
            }
            let part = best.partition_set(label);
            if part.contains(vertex) {
                continue;
            }
            for v2 in p.sorted(part) {
                if p.graph.contains_edge_between(vertex, v2) {
                    let new_weight = best.partition_weight(best.label(v2))
                        + best.partition_weight(best.label(vertex));
                    if new_weight <= p.capacity {
                        let w = self.distance(vertex, v2);
                        if w < distance_to_root {
                            closest = Some(v2.clone());
                            distance_to_root = w;
                        }
                    } else {
                        restriction
                            .entry(vertex.clone())
                            .or_default()
                            .insert(best.label(v2));
                        break;
                    }
                }
            }
        }
        closest
    }

    fn distance(&self, a: &G::Vertex, b: &G::Vertex) -> f64 {
        match self.problem.graph.get_edge(a, b) {
            None => f64::MAX,
            Some(e) => self.problem.graph.weight_of(e),
        }
    }
}

impl<'g, G: IncidenceGraph> CapacitatedSpanningTreeAlgorithm<G::Vertex, G::Edge>
    for EsauWilliamsCapacitatedMinimumSpanningTree<'g, G>
{
    /// Computes a capacitated spanning tree.
    fn capacitated_spanning_tree(&mut self) -> Result<CapacitatedSpanningTree<G::Vertex, G::Edge>> {
        if self.executed {
            return Ok(self.best.resulting_spanning_tree(&self.problem));
        }
        self.best = self.solution();
        let cmst = self.best.resulting_spanning_tree(&self.problem);
        self.executed = true;
        let p = &self.problem;
        if !cmst.is_capacitated_spanning_tree(p.graph, &p.root, p.capacity, &p.demands) {
            return Err(GraphError::Infeasible);
        }
        Ok(cmst)
    }
}

// ---------------------------------------------------------------------------------------------
// Ahuja–Orlin–Sharma
// ---------------------------------------------------------------------------------------------

use super::cyclic_exchange::AhujaOrlinSharmaCyclicExchangeLocalAugmentation;
use super::spanning::SpanningTree;
use crate::{DiGraph, GraphMut, GraphMutExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Single,
    Subtree,
    Pseudo,
    Origin,
}

/// Vertex of the improvement graph: an index and its kind.
type Iv = Pair<usize, Kind>;

const ORIGIN_LABEL: i64 = -1;

/// Options of [`AhujaOrlinSharmaCapacitatedMinimumSpanningTree`].
#[derive(Debug, Clone, Copy)]
pub struct AhujaOrlinSharmaOptions {
    /// Search for the best improving cycle instead of the first one.
    pub best_improvement: bool,
    /// Number of best moves of the initial Esau–Williams solution (ignored with a given start).
    pub number_of_operations: usize,
    /// Allow moving a single vertex from one subtree to another.
    pub use_vertex_operation: bool,
    /// Allow moving a whole subtree from one subtree to another.
    pub use_subtree_operation: bool,
    /// Use tabu search to escape local optima.
    pub use_tabu_search: bool,
    /// Number of iterations a changed vertex stays tabu.
    pub tabu_time: usize,
    /// Upper limit on the number of exchanges tried per tabu search.
    pub upper_limit_tabu_exchanges: usize,
}

impl Default for AhujaOrlinSharmaOptions {
    fn default() -> Self {
        AhujaOrlinSharmaOptions {
            best_improvement: false,
            number_of_operations: 1,
            use_vertex_operation: true,
            use_subtree_operation: true,
            use_tabu_search: true,
            tabu_time: 10,
            upper_limit_tabu_exchanges: 50,
        }
    }
}

/// Implementation of an algorithm for the capacitated minimum spanning tree problem using a cyclic
/// exchange neighborhood, based on Ravindra K. Ahuja, James B. Orlin, Dushyant Sharma, A composite
/// very large-scale neighborhood structure for the capacitated minimum spanning tree problem,
/// Operations Research Letters, Volume 31, Issue 3, 2003, Pages 185-194, ISSN 0167-6377,
/// <https://doi.org/10.1016/S0167-6377(02>)00236-5.
/// (<http://www.sciencedirect.com/science/article/pii/S0167637702002365>)
///
/// A [Capacitated Minimum Spanning Tree](https://en.wikipedia.org/wiki/Capacitated_minimum_spanning_tree) (CMST) is a rooted minimal cost spanning tree that satisfies the capacity
/// constrained on all trees that are connected to the designated root. The problem is NP-hard. The
/// hard part of the problem is the implicit partition defined by the subtrees. If one can find the
/// correct partition, the MSTs can be calculated in polynomial time.
///
/// This algorithm is a very large scale neighborhood search algorithm using a cyclic exchange
/// neighborhood until a local minimum is found. It makes frequently use of a MST algorithm and the
/// algorithm for subset disjoint cycles by Ahuja et al. That is, the algorithm may run in
/// exponential time. This algorithm is implemented in two different version: a local search and a
/// tabu search. In both cases we have to find the best neighbor of the current capacitated spanning
/// tree.
///
/// # Rust notes
///
/// Local search with cyclic exchanges of vertices and subtrees between the parts of the
/// partition, with optional tabu search (`AhujaOrlinSharmaCapacitatedMinimumSpanningTree`).
pub struct AhujaOrlinSharmaCapacitatedMinimumSpanningTree<'g, G: IncidenceGraph> {
    problem: Problem<'g, G>,
    length_bound: usize,
    options: AhujaOrlinSharmaOptions,
    initial_solution: Option<CapacitatedSpanningTree<G::Vertex, G::Edge>>,
    best: Solution<G::Vertex>,
    executed: bool,
}

type Subtrees<V> = HashMap<V, (IndexSet<V>, f64)>;
type PartitionTrees<E> = HashMap<usize, SpanningTree<E>>;

impl<'g, G: IncidenceGraph> AhujaOrlinSharmaCapacitatedMinimumSpanningTree<'g, G> {
    /// Starts from an Esau–Williams solution with default options.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(
        graph: &'g G,
        root: G::Vertex,
        capacity: f64,
        demands: HashMap<G::Vertex, f64>,
        length_bound: usize,
        number_of_operations: usize,
    ) -> Result<Self> {
        Self::with_options(
            graph,
            root,
            capacity,
            demands,
            length_bound,
            AhujaOrlinSharmaOptions {
                number_of_operations,
                ..Default::default()
            },
        )
    }

    /// Creates an instance with the given options.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_options(
        graph: &'g G,
        root: G::Vertex,
        capacity: f64,
        demands: HashMap<G::Vertex, f64>,
        length_bound: usize,
        options: AhujaOrlinSharmaOptions,
    ) -> Result<Self> {
        let problem = Problem::new(graph, root, capacity, demands)?;
        if !options.use_subtree_operation && !options.use_vertex_operation {
            return Err(GraphError::InvalidParameter {
                name: "options",
                requirement: crate::ParameterRequirement::Consistent,
            });
        }
        Ok(AhujaOrlinSharmaCapacitatedMinimumSpanningTree {
            problem,
            length_bound,
            options,
            initial_solution: None,
            best: Solution::empty(),
            executed: false,
        })
    }

    /// Starts from the given capacitated spanning tree.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_initial_solution(
        initial_solution: CapacitatedSpanningTree<G::Vertex, G::Edge>,
        graph: &'g G,
        root: G::Vertex,
        capacity: f64,
        demands: HashMap<G::Vertex, f64>,
        length_bound: usize,
        options: AhujaOrlinSharmaOptions,
    ) -> Result<Self> {
        let mut alg = Self::with_options(graph, root, capacity, demands, length_bound, options)?;
        let p = &alg.problem;
        if !initial_solution.is_capacitated_spanning_tree(p.graph, &p.root, p.capacity, &p.demands)
        {
            return Err(GraphError::InvalidParameter {
                name: "initial_solution",
                requirement: crate::ParameterRequirement::ValidSolution,
            });
        }
        alg.initial_solution = Some(initial_solution);
        Ok(alg)
    }

    fn initial(&self) -> Solution<G::Vertex> {
        match &self.initial_solution {
            Some(s) => Solution::new(s.labels.clone(), s.partition.clone()),
            None => EsauWilliamsCapacitatedMinimumSpanningTree {
                problem: self.problem.clone(),
                number_of_operations: self.options.number_of_operations,
                best: Solution::empty(),
                executed: false,
                rng: StdRng::from_os_rng(),
            }
            .solution(),
        }
    }

    fn run(&mut self) -> Result<Solution<G::Vertex>> {
        let o = self.options;
        let mut current = self.initial();
        // the best solution; `None` while it is the current one
        let mut best: Option<Solution<G::Vertex>> = None;
        let mut trees: PartitionTrees<G::Edge> = HashMap::new();
        let mut subtrees: Subtrees<G::Vertex> = HashMap::new();
        let mut affected: (IndexSet<usize>, IndexSet<G::Vertex>) = (
            current.labels_sorted().into_iter().collect(),
            IndexSet::new(),
        );
        let mut ig = ImprovementGraph::new(&self.problem, &o, &current)?;
        let mut tabu_list: IndexSet<G::Vertex> = IndexSet::new();
        let mut tabu_time_list: HashMap<i64, IndexSet<G::Vertex>> = HashMap::new();
        let mut tabu_timer: i64 = 0;
        let mut tabu_exchanges = 0;
        let mut cost_difference = 0.0;

        loop {
            self.calculate_spanning_trees(&current, &mut trees, &affected.0);
            if o.use_subtree_operation {
                self.calculate_subtrees(&current, &mut subtrees, &trees, &affected.0);
            }
            ig.update(
                &self.problem,
                &o,
                &current,
                &subtrees,
                &trees,
                &affected.0,
                &tabu_list,
            )?;
            let labels = ig.label_map(&o, &current);
            let cycle = AhujaOrlinSharmaCyclicExchangeLocalAugmentation::new(
                &ig.graph,
                self.length_bound,
                &labels,
                o.best_improvement,
            )?
            .local_augmentation_cycle();
            let current_cost = cycle.weight();
            cost_difference += current_cost;
            if o.use_tabu_search {
                if current_cost < 0.0 {
                    affected = self.execute(&mut current, &ig, &subtrees, cycle.vertex_list())?;
                    if cost_difference < 0.0 {
                        best = None;
                        cost_difference = 0.0;
                    }
                } else {
                    if o.upper_limit_tabu_exchanges <= tabu_exchanges {
                        break;
                    }
                    // keep the best solution safe from a non-improving exchange
                    if best.is_none() {
                        best = Some(current.clone());
                    }
                    affected = self.execute(&mut current, &ig, &subtrees, cycle.vertex_list())?;
                    tabu_list.extend(affected.1.iter().cloned());
                    tabu_time_list.insert(tabu_timer, affected.1.clone());
                    tabu_exchanges += 1;
                }
                if let Some(set) = tabu_time_list.remove(&(tabu_timer - o.tabu_time as i64 - 1)) {
                    tabu_list.retain(|v| !set.contains(v));
                }
                tabu_timer += 1;
            } else if current_cost < 0.0 {
                affected = self.execute(&mut current, &ig, &subtrees, cycle.vertex_list())?;
            } else {
                break;
            }
        }
        Ok(best.unwrap_or(current))
    }

    /// `executeNeighborhoodOperation`: applies the cycle to the solution; returns the labels and
    /// vertices that changed.
    fn execute(
        &self,
        current: &mut Solution<G::Vertex>,
        ig: &ImprovementGraph<G::Vertex>,
        subtrees: &Subtrees<G::Vertex>,
        cycle: &[Iv],
    ) -> Result<(IndexSet<usize>, IndexSet<G::Vertex>)> {
        let p = &self.problem;
        let mut affected_vertices = IndexSet::new();
        let mut affected_labels: IndexSet<usize> = IndexSet::new();
        let bug = || GraphError::Internal("invalid vertex types in an improvement cycle");
        if let Some(first) = cycle.first() {
            let vertex_label = |cur: &Iv, current: &Solution<G::Vertex>| {
                current.label(&ig.vertex_mapping[&cur.first])
            };
            let first_label = match first.second {
                Kind::Single | Kind::Subtree => Some(vertex_label(first, current)),
                _ => None,
            };
            for (i, cur) in cycle.iter().enumerate().take(cycle.len() - 1) {
                let next = &cycle[i + 1];
                let has_more = i + 2 < cycle.len();
                match cur.second {
                    Kind::Single | Kind::Subtree => {
                        let cur_vertex = &ig.vertex_mapping[&cur.first];
                        let cur_label = current.label(cur_vertex);
                        let next_label = if has_more {
                            match next.second {
                                Kind::Single | Kind::Subtree => vertex_label(next, current),
                                Kind::Pseudo => ig.path_exchange[next] as usize,
                                Kind::Origin => return Err(bug()),
                            }
                        } else {
                            first_label.ok_or_else(bug)?
                        };
                        affected_vertices.insert(cur_vertex.clone());
                        affected_labels.insert(cur_label);
                        if cur.second == Kind::Single {
                            current.move_vertex(p, cur_vertex, cur_label, next_label);
                        } else {
                            let subtree = &subtrees[cur_vertex].0;
                            current.move_vertices(p, subtree, cur_label, next_label);
                        }
                    }
                    Kind::Pseudo => {
                        affected_labels.insert(ig.path_exchange[cur] as usize);
                    }
                    Kind::Origin => {}
                }
            }
        }
        let mut more = IndexSet::new();
        let mut labels: Vec<usize> = affected_labels.iter().copied().collect();
        labels.sort_unstable();
        for label in labels {
            let subset = current
                .partition
                .get(&label)
                .map(|x| x.first.clone())
                .unwrap_or_default();
            if subset.is_empty() {
                affected_labels.shift_remove(&label);
            } else {
                more.extend(current.partition_subtrees_of_subset(p, &subset, label));
            }
        }
        affected_labels.extend(more);
        current.clean_up();
        Ok((affected_labels, affected_vertices))
    }

    /// Updates the map containing the MSTs for every subset of the partition.
    fn calculate_spanning_trees(
        &self,
        current: &Solution<G::Vertex>,
        trees: &mut PartitionTrees<G::Edge>,
        affected: &IndexSet<usize>,
    ) {
        let p = &self.problem;
        for &label in affected {
            let mut set = current.partition_set(label).clone();
            set.insert(p.root.clone());
            trees.insert(label, prim(&AsSubgraph::induced(p.graph, set)));
        }
    }

    fn calculate_subtrees(
        &self,
        current: &Solution<G::Vertex>,
        subtrees: &mut Subtrees<G::Vertex>,
        trees: &PartitionTrees<G::Edge>,
        affected: &IndexSet<usize>,
    ) {
        let p = &self.problem;
        for &label in affected {
            let mut modifiable = current.partition_set(label).clone();
            modifiable.insert(p.root.clone());
            for v in p.sorted(current.partition_set(label)) {
                let st = self.subtree(current, &modifiable, v, trees);
                subtrees.insert(v.clone(), st);
            }
        }
    }

    /// The vertices below `v` in the tree of its part, rooted at the root, and their demand.
    fn subtree(
        &self,
        current: &Solution<G::Vertex>,
        modifiable: &IndexSet<G::Vertex>,
        v: &G::Vertex,
        trees: &PartitionTrees<G::Edge>,
    ) -> (IndexSet<G::Vertex>, f64) {
        let p = &self.problem;
        let part_tree = &trees[&current.label(v)];
        let tree = AsSubgraph::with_edges(
            p.graph,
            modifiable.iter().cloned(),
            part_tree.edges.iter().cloned(),
        );
        let mut subtree = IndexSet::new();
        let mut subtree_weight = 0.0;
        let mut current_path: IndexSet<G::Vertex> = IndexSet::new();
        let mut current_weight = 0.0;
        let mut store_current_path = true;
        for next in DepthFirstIterator::from_start(&tree, v).expect("v is in the tree") {
            if tree.contains_edge_between(&next, v) {
                store_current_path = true;
                subtree.extend(std::mem::take(&mut current_path));
                subtree_weight += current_weight;
                current_weight = 0.0;
            }
            if next == p.root {
                store_current_path = false;
                current_path = IndexSet::new();
                current_weight = 0.0;
            }
            if store_current_path {
                current_path.insert(next.clone());
                current_weight += p.demand(&next);
            }
        }
        (subtree, subtree_weight)
    }
}

impl<'g, G: IncidenceGraph> CapacitatedSpanningTreeAlgorithm<G::Vertex, G::Edge>
    for AhujaOrlinSharmaCapacitatedMinimumSpanningTree<'g, G>
{
    /// Computes a capacitated spanning tree.
    fn capacitated_spanning_tree(&mut self) -> Result<CapacitatedSpanningTree<G::Vertex, G::Edge>> {
        if !self.executed {
            self.best = self.run()?;
            self.executed = true;
        }
        Ok(self.best.resulting_spanning_tree(&self.problem))
    }
}

/// The improvement graph: a vertex per (non-root vertex, move kind), a pseudo vertex per part and
/// an origin. A negative cycle is an improving exchange.
struct ImprovementGraph<V: Eq + Hash + Clone> {
    graph: DiGraph<Iv>,
    origin: Iv,
    /// Index to vertex of the input graph.
    vertex_mapping: IndexMap<usize, V>,
    initial_vertex_mapping: HashMap<V, usize>,
    /// Label to pseudo vertex.
    pseudo_vertex_mapping: IndexMap<usize, Iv>,
    /// Label of every pseudo vertex and of the origin.
    path_exchange: IndexMap<Iv, i64>,
}

fn prim_of_set<G: IncidenceGraph>(
    p: &Problem<'_, G>,
    set: &IndexSet<G::Vertex>,
) -> SpanningTree<G::Edge> {
    prim(&AsSubgraph::induced(p.graph, set.iter().cloned()))
}

impl<V: Eq + Hash + Clone> ImprovementGraph<V> {
    fn new<G: IncidenceGraph<Vertex = V>>(
        p: &Problem<'_, G>,
        o: &AhujaOrlinSharmaOptions,
        solution: &Solution<V>,
    ) -> Result<Self> {
        let mut graph = DiGraph::<Iv>::simple().weighted();
        let mut vertex_mapping = IndexMap::new();
        let mut initial_vertex_mapping = HashMap::new();
        let mut counter = 0;
        for v in p.graph.vertices() {
            let v = v.borrow();
            if *v == p.root {
                continue;
            }
            if o.use_vertex_operation {
                graph.add_vertex(Pair::of(counter, Kind::Single));
            }
            if o.use_subtree_operation {
                graph.add_vertex(Pair::of(counter, Kind::Subtree));
            }
            vertex_mapping.insert(counter, v.clone());
            initial_vertex_mapping.insert(v.clone(), counter);
            counter += 1;
        }
        let origin = Pair::of(counter, Kind::Origin);
        graph.add_vertex(origin);
        let mut path_exchange = IndexMap::new();
        path_exchange.insert(origin, ORIGIN_LABEL);
        let mut pseudo_vertex_mapping = IndexMap::new();
        for label in solution.labels_sorted() {
            let pseudo = Pair::of(origin.first + label + 1, Kind::Pseudo);
            pseudo_vertex_mapping.insert(label, pseudo);
            path_exchange.insert(pseudo, label as i64);
            graph.add_vertex(pseudo);
        }
        for pseudo in pseudo_vertex_mapping.values() {
            Self::set_edge(&mut graph, *pseudo, origin, 0.0)?;
        }
        Ok(ImprovementGraph {
            graph,
            origin,
            vertex_mapping,
            initial_vertex_mapping,
            pseudo_vertex_mapping,
            path_exchange,
        })
    }

    fn set_edge(graph: &mut DiGraph<Iv>, a: Iv, b: Iv, w: f64) -> Result<()> {
        let e = match graph.get_edge(&a, &b) {
            Some(e) => *e,
            None => graph.add_edge(&a, &b)?.ok_or(GraphError::Internal(
                "improvement graph edge could not be added",
            ))?,
        };
        graph.set_edge_weight(&e, w)
    }

    /// The labels of all improvement graph vertices in the current solution.
    fn label_map(&self, o: &AhujaOrlinSharmaOptions, solution: &Solution<V>) -> HashMap<Iv, i64> {
        let mut m = HashMap::new();
        for (&i, v) in &self.vertex_mapping {
            let label = solution.label(v) as i64;
            if o.use_vertex_operation {
                m.insert(Pair::of(i, Kind::Single), label);
            }
            if o.use_subtree_operation {
                m.insert(Pair::of(i, Kind::Subtree), label);
            }
        }
        for (k, &l) in &self.path_exchange {
            m.insert(*k, l);
        }
        m
    }

    #[allow(clippy::too_many_arguments)]
    fn update<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        p: &Problem<'_, G>,
        o: &AhujaOrlinSharmaOptions,
        solution: &Solution<V>,
        subtrees: &Subtrees<V>,
        trees: &PartitionTrees<G::Edge>,
        labels_to_update: &IndexSet<usize>,
        tabu_list: &IndexSet<V>,
    ) -> Result<()> {
        self.update_pseudo_nodes_of_new_labels(solution)?;
        let vertices: Vec<V> = p.graph.vertices_cloned().collect();
        for v1 in &vertices {
            if *v1 == p.root {
                continue;
            }
            let id = self.initial_vertex_mapping[v1];
            let single = Pair::of(id, Kind::Single);
            let subtree = Pair::of(id, Kind::Subtree);
            if tabu_list.contains(v1) {
                if o.use_vertex_operation {
                    self.graph.remove_vertex(&single);
                    self.graph.add_vertex(single);
                }
                if o.use_subtree_operation {
                    self.graph.remove_vertex(&subtree);
                    self.graph.add_vertex(subtree);
                }
                continue;
            }
            self.update_origin_node_connections(
                p,
                o,
                solution,
                subtrees,
                trees,
                labels_to_update,
                v1,
                single,
                subtree,
            )?;
            for label in solution.labels_sorted() {
                let v1_label = solution.label(v1);
                if label == v1_label
                    || (!labels_to_update.contains(&v1_label) && !labels_to_update.contains(&label))
                {
                    continue;
                }
                let pseudo = self.pseudo_vertex_mapping[&label];
                let mut modifiable = solution.partition_set(label).clone();
                modifiable.insert(p.root.clone());
                let old_weight = trees[&label].weight;
                self.update_single_node(
                    p,
                    o,
                    solution,
                    subtrees,
                    tabu_list,
                    label,
                    old_weight,
                    &mut modifiable,
                    pseudo,
                    v1,
                    single,
                )?;
                self.update_subtree_node(
                    p,
                    o,
                    solution,
                    subtrees,
                    tabu_list,
                    label,
                    old_weight,
                    &mut modifiable,
                    pseudo,
                    v1,
                    subtree,
                )?;
            }
        }
        Ok(())
    }

    fn update_pseudo_nodes_of_new_labels(&mut self, solution: &Solution<V>) -> Result<()> {
        let labels = solution.labels_sorted();
        let same = labels.len() == self.pseudo_vertex_mapping.len()
            && labels
                .iter()
                .all(|l| self.pseudo_vertex_mapping.contains_key(l));
        if same {
            return Ok(());
        }
        for &label in &labels {
            if !self.pseudo_vertex_mapping.contains_key(&label) {
                let pseudo = Pair::of(self.origin.first + label + 1, Kind::Pseudo);
                self.pseudo_vertex_mapping.insert(label, pseudo);
                self.path_exchange.insert(pseudo, label as i64);
                self.graph.add_vertex(pseudo);
                Self::set_edge(&mut self.graph, pseudo, self.origin, 0.0)?;
            }
        }
        if labels.len() != self.pseudo_vertex_mapping.len() {
            let stale: Vec<usize> = self
                .pseudo_vertex_mapping
                .keys()
                .copied()
                .filter(|l| !labels.contains(l))
                .collect();
            for label in stale {
                let pseudo = Pair::of(self.origin.first + label + 1, Kind::Pseudo);
                self.pseudo_vertex_mapping.shift_remove(&label);
                self.path_exchange.shift_remove(&pseudo);
                self.graph.remove_vertex(&pseudo);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn update_origin_node_connections<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        p: &Problem<'_, G>,
        o: &AhujaOrlinSharmaOptions,
        solution: &Solution<V>,
        subtrees: &Subtrees<V>,
        trees: &PartitionTrees<G::Edge>,
        labels_to_update: &IndexSet<usize>,
        v1: &V,
        single: Iv,
        subtree: Iv,
    ) -> Result<()> {
        let label = solution.label(v1);
        if !labels_to_update.contains(&label) {
            return Ok(());
        }
        let old_weight = trees[&label].weight;
        let mut set = solution.partition_set(label).clone();
        set.insert(p.root.clone());
        let weight_of = |set: &IndexSet<V>| {
            let t = prim_of_set(p, set);
            if t.edges.len() == set.len() - 1 {
                t.weight
            } else {
                f64::NAN
            }
        };
        if o.use_vertex_operation {
            set.shift_remove(v1);
            let new_weight = weight_of(&set);
            self.update_edge(p, self.origin, single, 0.0, new_weight - old_weight)?;
            set.insert(v1.clone());
        }
        if o.use_subtree_operation {
            let st = &subtrees[v1].0;
            if st.len() > 1 || !o.use_vertex_operation {
                set.retain(|v| !st.contains(v));
                let new_weight = weight_of(&set);
                self.update_edge(p, self.origin, subtree, 0.0, new_weight - old_weight)?;
                set.extend(st.iter().cloned());
            } else {
                self.graph.remove_vertex(&subtree);
                self.graph.add_vertex(subtree);
            }
        }
        Ok(())
    }

    /// A tree of the modified part, with its largest subtree demand if it spans the set.
    fn evaluate<G: IncidenceGraph<Vertex = V>>(
        &self,
        p: &Problem<'_, G>,
        set: &IndexSet<V>,
        total_demand: f64,
    ) -> (f64, f64) {
        let t = prim_of_set(p, set);
        if t.edges.len() == set.len() - 1 {
            (
                self.maximum_demand_of_subtrees(p, set, &t, total_demand),
                t.weight,
            )
        } else {
            (f64::NAN, f64::NAN)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_single_node<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        p: &Problem<'_, G>,
        o: &AhujaOrlinSharmaOptions,
        solution: &Solution<V>,
        subtrees: &Subtrees<V>,
        tabu_list: &IndexSet<V>,
        label: usize,
        old_weight: f64,
        modifiable: &mut IndexSet<V>,
        pseudo: Iv,
        v1: &V,
        single: Iv,
    ) -> Result<()> {
        modifiable.insert(v1.clone());
        if !o.use_vertex_operation {
            return Ok(());
        }
        let part_weight = solution.partition_weight(label);
        for v2 in p.sorted(solution.partition_set(label)) {
            if *v2 == p.root {
                return Err(GraphError::Internal("the root is in a partition"));
            }
            if tabu_list.contains(v2) {
                continue;
            }
            let id2 = self.initial_vertex_mapping[v2];
            modifiable.shift_remove(v2);
            let (cap, w) = self.evaluate(p, modifiable, part_weight + p.demand(v1) - p.demand(v2));
            self.update_edge(p, single, Pair::of(id2, Kind::Single), cap, w - old_weight)?;
            modifiable.insert(v2.clone());
            if o.use_subtree_operation && subtrees[v2].0.len() > 1 {
                let st = &subtrees[v2];
                modifiable.retain(|v| !st.0.contains(v));
                let (cap, w) = self.evaluate(p, modifiable, part_weight + p.demand(v1) - st.1);
                self.update_edge(p, single, Pair::of(id2, Kind::Subtree), cap, w - old_weight)?;
                modifiable.extend(st.0.iter().cloned());
            }
        }
        let (cap, w) = self.evaluate(p, modifiable, part_weight + p.demand(v1));
        self.update_edge(p, single, pseudo, cap, w - old_weight)?;
        modifiable.shift_remove(v1);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn update_subtree_node<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        p: &Problem<'_, G>,
        o: &AhujaOrlinSharmaOptions,
        solution: &Solution<V>,
        subtrees: &Subtrees<V>,
        tabu_list: &IndexSet<V>,
        label: usize,
        old_weight: f64,
        modifiable: &mut IndexSet<V>,
        pseudo: Iv,
        v1: &V,
        subtree: Iv,
    ) -> Result<()> {
        if !(o.use_subtree_operation && (subtrees[v1].0.len() > 1 || !o.use_vertex_operation)) {
            return Ok(());
        }
        let st1 = &subtrees[v1];
        modifiable.extend(st1.0.iter().cloned());
        let part_weight = solution.partition_weight(label);
        for v2 in p.sorted(solution.partition_set(label)) {
            if *v2 == p.root {
                return Err(GraphError::Internal("the root is in a partition"));
            }
            if tabu_list.contains(v2) {
                continue;
            }
            let id2 = self.initial_vertex_mapping[v2];
            if o.use_vertex_operation {
                modifiable.shift_remove(v2);
                let (cap, w) = self.evaluate(p, modifiable, part_weight + st1.1 - p.demand(v2));
                self.update_edge(p, subtree, Pair::of(id2, Kind::Single), cap, w - old_weight)?;
                modifiable.insert(v2.clone());
            }
            let st2 = &subtrees[v2];
            modifiable.retain(|v| !st2.0.contains(v));
            let (cap, w) = self.evaluate(
                p,
                modifiable,
                solution.partition_weight(solution.label(v2)) + st1.1 - st2.1,
            );
            self.update_edge(
                p,
                subtree,
                Pair::of(id2, Kind::Subtree),
                cap,
                w - old_weight,
            )?;
            modifiable.extend(st2.0.iter().cloned());
        }
        let (cap, w) = self.evaluate(p, modifiable, part_weight + st1.1);
        self.update_edge(p, subtree, pseudo, cap, w - old_weight)
    }

    fn update_edge<G: IncidenceGraph<Vertex = V>>(
        &mut self,
        p: &Problem<'_, G>,
        a: Iv,
        b: Iv,
        new_capacity: f64,
        new_cost: f64,
    ) -> Result<()> {
        if !new_capacity.is_nan() && new_capacity <= p.capacity && !new_cost.is_nan() {
            Self::set_edge(&mut self.graph, a, b, new_cost)
        } else {
            self.graph.remove_edge_between(&a, &b);
            Ok(())
        }
    }

    /// `calculateMaximumDemandOfSubtrees`: the largest demand below one root edge of the tree,
    /// cut short once it is clear that the capacity bound decides.
    fn maximum_demand_of_subtrees<G: IncidenceGraph<Vertex = V>>(
        &self,
        p: &Problem<'_, G>,
        subset: &IndexSet<V>,
        tree: &SpanningTree<G::Edge>,
        total_demand: f64,
    ) -> f64 {
        let st =
            AsSubgraph::with_edges(p.graph, subset.iter().cloned(), tree.edges.iter().cloned());
        let degree_of_root = st.degree_of(&p.root);
        if degree_of_root == 1 {
            return total_demand;
        }
        let mut maximum = 0.0;
        let mut dfs = DepthFirstIterator::from_start(&st, &p.root).expect("root is in the tree");
        dfs.next();
        let mut root_edges_explored = 0;
        let mut explored = 0.0;
        let mut current = 0.0;
        for next in dfs {
            if st.contains_edge_between(&p.root, &next) {
                explored += current;
                if maximum < current {
                    maximum = current;
                }
                if maximum >= 0.5 * total_demand || explored + maximum >= total_demand {
                    return maximum;
                }
                if root_edges_explored + 1 == degree_of_root {
                    return f64::max(maximum, total_demand - explored);
                }
                root_edges_explored += 1;
                current = 0.0;
            }
            current += p.demand(&next);
        }
        maximum
    }
}
