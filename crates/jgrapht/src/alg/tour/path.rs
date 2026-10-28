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

use super::base::require_not_empty;
use crate::alg::connectivity::{BiconnectivityInspector, ConnectivityInspector};
use crate::error::{GraphError, Result};
use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use indexmap::IndexSet;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};

/// `HamiltonianPathSearchResult.Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStatus {
    /// A Hamiltonian path was found.
    PathFound,
    /// The search finished without a path: none exists.
    ProvenAbsent,
    /// The search stopped at its limit before it could decide.
    Aborted,
}

/// Tri-state result of a `HamiltonianPathAlgorithm` search.
///
/// A Hamiltonian path search can terminate in three distinct ways:
///
/// - `Status#PATH_FOUND` — a Hamiltonian path was discovered. The path is available via
///   `getPath()`.
/// - `Status#PROVEN_ABSENT` — the search ran to completion (or was rejected by a sound
///   precheck) and proved that no Hamiltonian path exists.
/// - `Status#ABORTED` — the search hit a configured limit before completing. Whether a
///   Hamiltonian path exists in the input graph is unknown.
///
/// Callers must consult `getStatus()` before interpreting an empty `getPath()`:
/// an empty path may mean either `Status#PROVEN_ABSENT` or `Status#ABORTED`, and
/// the two have very different semantic implications.
///
/// Instances are obtained either from a `HamiltonianPathAlgorithm` implementation or via
/// the `found(GraphPath, long)`, `provenAbsent(long)`, and
/// `aborted(long)` static factory methods on this interface, which return a default
/// immutable implementation.
///
/// # Rust notes
///
/// `HamiltonianPathSearchResult`: the outcome of a Hamiltonian path search.
#[derive(Debug, Clone)]
pub struct HamiltonianPathSearchResult<V, E> {
    status: SearchStatus,
    path: Option<GraphWalk<V, E>>,
    states_expanded: u64,
}

impl<V, E> HamiltonianPathSearchResult<V, E> {
    /// Creates a result for a successful search.
    pub fn found(path: GraphWalk<V, E>, states_expanded: u64) -> Self {
        HamiltonianPathSearchResult {
            status: SearchStatus::PathFound,
            path: Some(path),
            states_expanded,
        }
    }

    /// Creates a result for an exhaustive search that proved no Hamiltonian path exists.
    pub fn proven_absent(states_expanded: u64) -> Self {
        HamiltonianPathSearchResult {
            status: SearchStatus::ProvenAbsent,
            path: None,
            states_expanded,
        }
    }

    /// Creates a result for a search that was stopped by a state limit before completing.
    pub fn aborted(states_expanded: u64) -> Self {
        HamiltonianPathSearchResult {
            status: SearchStatus::Aborted,
            path: None,
            states_expanded,
        }
    }

    /// Returns the termination status of this search.
    pub fn status(&self) -> SearchStatus {
        self.status
    }

    /// Returns the Hamiltonian path if the search found one.
    pub fn path(&self) -> Option<&GraphWalk<V, E>> {
        self.path.as_ref()
    }

    /// Returns the number of DFS / DP states the search explored. The exact counting semantics
    /// are implementation-defined; consult the JavaDoc of the producing algorithm for details.
    pub fn states_expanded(&self) -> u64 {
        self.states_expanded
    }
}

/// An algorithm solving the [Hamiltonian path problem](https://en.wikipedia.org/wiki/Hamiltonian_path).
///
/// A Hamiltonian path is a path in an undirected or directed graph that visits every vertex
/// exactly once. Unlike a Hamiltonian cycle, a Hamiltonian path is not required to return to the
/// start vertex. The path's endpoints may be any pair of vertices (or a single vertex if the graph
/// has only one vertex).
///
/// Deciding whether a Hamiltonian path exists in a general graph is NP-complete. Implementations
/// of this interface are therefore expected to use exact, exponential-time algorithms (possibly
/// with pruning) or specialised polynomial-time algorithms for restricted graph classes. Callers
/// should consult each implementation's JavaDoc for complexity and recommended graph sizes.
///
/// This interface is the path-specific counterpart of
/// `HamiltonianCycleAlgorithm`. It is intentionally separate because the path problem does
/// not require a return edge from the last visited vertex to the first, and because the existing
/// cycle / tour APIs often assume tour or complete-graph semantics that are not appropriate for
/// Hamiltonian paths.
///
/// # Rust notes
///
/// `HamiltonianPathAlgorithm`.
pub trait HamiltonianPathAlgorithm<G: Graph> {
    /// Returns the Hamiltonian path if the search found one.
    ///
    /// # Errors
    ///
    /// Returns an `InvalidGraph` error when the graph is empty or does not satisfy the requirements of the implementation, and an `InvalidParameter` error when it exceeds the vertex limit.
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>>;
}

fn require_directed_or_undirected<G: Graph>(graph: &G) -> Result<()> {
    if graph.graph_type().mixed {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::NotMixed));
    }
    Ok(())
}

fn singleton_path<G: Graph>(graph: &G) -> GraphWalk<G::Vertex, G::Edge> {
    let only = graph.vertices_cloned().next().expect("one vertex");
    GraphWalk::new_unchecked(vec![only], Vec::new(), 0.0)
}

fn vertex_list_to_path<G: Graph>(
    vertices: Vec<G::Vertex>,
    graph: &G,
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    if vertices.len() == 1 {
        return Ok(GraphWalk::new_unchecked(vertices, Vec::new(), 0.0));
    }
    let mut edges = Vec::with_capacity(vertices.len() - 1);
    let mut weight = 0.0;
    for pair in vertices.windows(2) {
        let e = graph
            .get_edge(&pair[0], &pair[1])
            .ok_or(GraphError::EdgeNotFound)?;
        weight += graph.weight_of(e);
        edges.push(e.clone());
    }
    Ok(GraphWalk::new_unchecked(vertices, edges, weight))
}

/// Distinct neighbours (loops dropped) in first-seen order.
fn build_adjacency<G: IncidenceGraph>(
    graph: &G,
    index_list: &[G::Vertex],
    vertex_map: &HashMap<G::Vertex, usize>,
    directed: bool,
) -> Vec<Vec<usize>> {
    index_list
        .iter()
        .map(|u_vertex| {
            let mut neighbours: IndexSet<usize> = IndexSet::new();
            let others: Vec<&G::Vertex> = if directed {
                graph
                    .out_edges_of(u_vertex)
                    .map(|e| graph.edge_target(e).expect("edge of the graph"))
                    .collect()
            } else {
                graph
                    .edges_of(u_vertex)
                    .map(|e| {
                        graph
                            .opposite_vertex(e, u_vertex)
                            .expect("edge of the graph")
                    })
                    .collect()
            };
            for other in others {
                if other == u_vertex {
                    continue;
                }
                neighbours.insert(vertex_map[other]);
            }
            neighbours.into_iter().collect()
        })
        .collect()
}

/// Polynomial-time Hamiltonian path algorithm for directed acyclic graphs.
///
/// Hamiltonian path existence in a directed acyclic graph (DAG) reduces to deciding whether the
/// longest directed path covers all vertices. This implementation computes a topological order,
/// runs the standard longest-path-in-DAG dynamic program over that order, and reconstructs a
/// Hamiltonian path when the longest path's length equals the vertex count.
///
/// The longest-path-in-DAG DP is a textbook reduction; see for example CLRS, "Introduction to
/// Algorithms" (4th ed.), section on single-source shortest paths in DAGs, which dualises to
/// longest-path by negating edge weights, and the MIT 6.s078 lecture notes
/// ([lecture 17 on dynamic programming, MIT 6.s078, Spring 2022](https://people.csail.mit.edu/virgi/6.s078/lecture17.pdf)) for the Hamiltonian-path framing.
///
/// Total complexity is `O(|V| + |E|)` time and `O(|V|)` space.
///
/// This class only accepts directed acyclic graphs. Passing a directed graph that contains a
/// cycle, or an undirected graph, fails with an error. To solve the Hamiltonian path problem on
/// cyclic directed graphs, use `BacktrackingHamiltonianPath` or
/// `HeldKarpHamiltonianPath`.
///
/// In multigraphs, parallel edges between the same vertex pair do not change the result. The
/// returned path uses an arbitrary representative edge selected via `Graph#getEdge` and
/// is not weight-optimised across parallel edges.
///
/// # Rust notes
///
/// `DagHamiltonianPath`: a Hamiltonian path of a DAG exists iff its topological order is a path.
#[derive(Default)]
pub struct DagHamiltonianPath;

impl DagHamiltonianPath {
    /// Constructs a new instance.
    pub fn new() -> Self {
        DagHamiltonianPath
    }
}

impl<G: IncidenceGraph> HamiltonianPathAlgorithm<G> for DagHamiltonianPath {
    /// Computes a Hamiltonian path in the given graph and returns a tri-state
    /// `HamiltonianPathSearchResult`.
    ///
    /// The return value distinguishes:
    ///
    /// - `HamiltonianPathSearchResult.Status#PATH_FOUND` — the algorithm produced a
    ///   Hamiltonian path; it is accessible via
    ///   `get_path`.
    /// - `HamiltonianPathSearchResult.Status#PROVEN_ABSENT` — the algorithm ran to
    ///   completion (or was rejected by a sound precheck) and proved that no Hamiltonian path
    ///   exists.
    /// - `HamiltonianPathSearchResult.Status#ABORTED` — the algorithm performed a bounded
    ///   search that hit its limit before completing. Whether a Hamiltonian path exists in the
    ///   graph is unknown.
    ///
    /// Unbounded implementations only return `PATH_FOUND` or `PROVEN_ABSENT`.
    /// Implementations that accept an execution budget (e.g. a maximum state count) may also
    /// return `ABORTED`; such bounded variants must never report `PROVEN_ABSENT`
    /// for a search that was stopped early.
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        if !graph.graph_type().directed {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Directed));
        }
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(HamiltonianPathSearchResult::found(singleton_path(graph), 0));
        }
        // Kahn's algorithm
        let index_list: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let vertex_map: HashMap<G::Vertex, usize> = index_list
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let mut in_degree: Vec<usize> = index_list.iter().map(|v| graph.in_degree_of(v)).collect();
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut topo = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            topo.push(u);
            for e in graph.out_edges_of(&index_list[u]) {
                let v = vertex_map[graph
                    .edge_target(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?];
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        if topo.len() != n {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Acyclic));
        }
        let mut position = vec![0usize; n];
        for (i, &v) in topo.iter().enumerate() {
            position[v] = i;
        }
        let mut longest = vec![1usize; n];
        let mut predecessor = vec![usize::MAX; n];
        let mut best_end = 0;
        let mut best_length = 1;
        for i in 0..n {
            let v = &index_list[topo[i]];
            for e in graph.in_edges_of(v) {
                let u = position[vertex_map[graph
                    .edge_source(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?]];
                if longest[u] + 1 > longest[i] {
                    longest[i] = longest[u] + 1;
                    predecessor[i] = u;
                }
            }
            if longest[i] > best_length {
                best_length = longest[i];
                best_end = i;
            }
        }
        if best_length != n {
            return Ok(HamiltonianPathSearchResult::proven_absent(n as u64));
        }
        let mut reversed = Vec::with_capacity(n);
        let mut cur = best_end;
        while cur != usize::MAX {
            reversed.push(index_list[topo[cur]].clone());
            cur = predecessor[cur];
        }
        reversed.reverse();
        Ok(HamiltonianPathSearchResult::found(
            vertex_list_to_path(reversed, graph)?,
            n as u64,
        ))
    }
}

/// Dynamic-programming algorithm for the
/// [Hamiltonian path problem](https://en.wikipedia.org/wiki/Hamiltonian_path).
///
/// The implementation follows the classic Held-Karp subset DP, adapted to find a Hamiltonian
/// *path* rather than a Hamiltonian cycle. The state `dp[subset][v]` is
/// `true` when there exists a simple path that visits exactly the vertices in
/// `subset` and whose last vertex is `v`. The transition extends a path ending at
/// `v` by a new vertex `u` whenever the edge `(v, u)` exists and `u` is
/// not yet in the subset. A Hamiltonian path exists if and only if `dp[fullMask][v]` is
/// `true` for some `v`.
///
/// The subset-DP formulation is due to Held and Karp,
/// ["A Dynamic Programming Approach to Sequencing Problems", J. SIAM 10(1), 1962](https://www.jstor.org/stable/2098806), and independently to Bellman, "Dynamic Programming
/// Treatment of the Travelling Salesman Problem", J. ACM 9(1), 1962. The Hamiltonian-path
/// variant (where the DP does not need a closing edge back to the start) is a standard textbook
/// adaptation; see for example CLRS, "Introduction to Algorithms" (4th ed.), the
/// dynamic-programming chapter, and the MIT 6.s078 lecture notes on advanced algorithms
/// (lecture 17).
///
/// Complexity is `O(n^2 * 2^n)` time and `O(n * 2^n)` space, where `n` is the
/// number of vertices. Because the running time and memory grow exponentially in `n`,
/// this class refuses graphs with more than the configured maximum number of vertices with an
/// error. The default ceiling is `DEFAULT_MAX_VERTICES`; the
/// single-argument constructor lets callers opt in to a higher ceiling up to
/// `HARD_MAX_VERTICES` when they have the memory headroom. Callers handling larger
/// graphs should use `BacktrackingHamiltonianPath` instead and accept the lack of
/// polynomial-bound guarantees.
///
/// The algorithm is exact and deterministic. It supports directed and undirected graphs and
/// tolerates parallel edges and self-loops (self-loops are ignored because they cannot extend a
/// simple path). In multigraphs, parallel edges between the same vertex pair collapse to a
/// single DP transition; the returned path uses an arbitrary representative edge selected via
/// `Graph#getEdge` and is not weight-optimised across parallel edges.
///
/// # Rust notes
///
/// `HeldKarpHamiltonianPath`: bitmask dynamic programming, `O(2^n n²)`; a hard limit of 30
/// vertices.
pub struct HeldKarpHamiltonianPath {
    max_vertices: usize,
    states_expanded: u64,
}

impl HeldKarpHamiltonianPath {
    /// Default limit on the number of vertices of the graph.
    pub const DEFAULT_MAX_VERTICES: usize = 20;
    /// Largest limit on the number of vertices that can be configured.
    pub const HARD_MAX_VERTICES: usize = 30;
    const UNVISITED: i8 = -1;
    const START_SENTINEL: i8 = -2;

    /// Constructs a new instance with the default vertex ceiling
    /// (`DEFAULT_MAX_VERTICES`).
    pub fn new() -> Self {
        HeldKarpHamiltonianPath {
            max_vertices: Self::DEFAULT_MAX_VERTICES,
            states_expanded: 0,
        }
    }

    /// Creates an instance with the given max vertices.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn with_max_vertices(max_vertices: usize) -> Result<Self> {
        if max_vertices < 1 {
            return Err(GraphError::InvalidParameter {
                name: "max_vertices",
                requirement: crate::ParameterRequirement::AtLeast(1),
            });
        }
        if max_vertices > Self::HARD_MAX_VERTICES {
            return Err(GraphError::InvalidParameter {
                name: "max_vertices",
                requirement: crate::ParameterRequirement::AtMost(Self::HARD_MAX_VERTICES as u64),
            });
        }
        Ok(HeldKarpHamiltonianPath {
            max_vertices,
            states_expanded: 0,
        })
    }

    /// Returns the maximum number of vertices this instance will accept.
    pub fn max_vertices(&self) -> usize {
        self.max_vertices
    }

    /// The number of states expanded by the last search.
    pub fn states_expanded(&self) -> u64 {
        self.states_expanded
    }
}

impl Default for HeldKarpHamiltonianPath {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: IncidenceGraph> HamiltonianPathAlgorithm<G> for HeldKarpHamiltonianPath {
    /// Computes a Hamiltonian path in the given graph and returns a tri-state
    /// `HamiltonianPathSearchResult`.
    ///
    /// The return value distinguishes:
    ///
    /// - `HamiltonianPathSearchResult.Status#PATH_FOUND` — the algorithm produced a
    ///   Hamiltonian path; it is accessible via
    ///   `get_path`.
    /// - `HamiltonianPathSearchResult.Status#PROVEN_ABSENT` — the algorithm ran to
    ///   completion (or was rejected by a sound precheck) and proved that no Hamiltonian path
    ///   exists.
    /// - `HamiltonianPathSearchResult.Status#ABORTED` — the algorithm performed a bounded
    ///   search that hit its limit before completing. Whether a Hamiltonian path exists in the
    ///   graph is unknown.
    ///
    /// Unbounded implementations only return `PATH_FOUND` or `PROVEN_ABSENT`.
    /// Implementations that accept an execution budget (e.g. a maximum state count) may also
    /// return `ABORTED`; such bounded variants must never report `PROVEN_ABSENT`
    /// for a search that was stopped early.
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        require_directed_or_undirected(graph)?;
        self.states_expanded = 0;
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(HamiltonianPathSearchResult::found(singleton_path(graph), 0));
        }
        if n > self.max_vertices {
            return Err(GraphError::TooManyVertices {
                max: self.max_vertices,
                got: n,
            });
        }
        let index_list: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let vertex_map: HashMap<G::Vertex, usize> = index_list
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let directed = graph.graph_type().directed;
        let adjacency = build_adjacency(graph, &index_list, &vertex_map, directed);
        let full_mask = (1usize << n) - 1;
        let mut pred = vec![vec![Self::UNVISITED; n]; 1 << n];
        for v in 0..n {
            pred[1 << v][v] = Self::START_SENTINEL;
            self.states_expanded += 1;
        }
        for mask in 1..(1usize << n) {
            for v in 0..n {
                if (mask >> v) & 1 == 0 || pred[mask][v] == Self::UNVISITED {
                    continue;
                }
                for &u in &adjacency[v] {
                    if (mask >> u) & 1 != 0 {
                        continue;
                    }
                    let new_mask = mask | (1 << u);
                    if pred[new_mask][u] == Self::UNVISITED {
                        pred[new_mask][u] = v as i8;
                        self.states_expanded += 1;
                    }
                }
            }
        }
        let Some(end_vertex) = (0..n).find(|&v| pred[full_mask][v] != Self::UNVISITED) else {
            return Ok(HamiltonianPathSearchResult::proven_absent(
                self.states_expanded,
            ));
        };
        let mut reversed = Vec::with_capacity(n);
        let mut mask = full_mask;
        let mut cur = end_vertex as isize;
        while cur != -1 {
            reversed.push(cur as usize);
            let p = pred[mask][cur as usize];
            mask ^= 1 << cur;
            cur = if p == Self::START_SENTINEL {
                -1
            } else {
                (p as u8) as isize
            };
        }
        reversed.reverse();
        let vertices = reversed
            .into_iter()
            .map(|i| index_list[i].clone())
            .collect();
        Ok(HamiltonianPathSearchResult::found(
            vertex_list_to_path(vertices, graph)?,
            self.states_expanded,
        ))
    }
}

/// Exact backtracking algorithm for the
/// [Hamiltonian path problem](https://en.wikipedia.org/wiki/Hamiltonian_path) on
/// directed and undirected graphs.
///
/// The algorithm performs depth-first search over simple paths. From each candidate start vertex
/// it tries to extend the current path by an unused adjacent vertex; the first path that visits
/// every vertex exactly once is returned. If the exhaustive search proves that no such path
/// exists, a `HamiltonianPathSearchResult.Status#PROVEN_ABSENT` result is returned.
///
/// The unbounded `getPath(Graph)` entry point only returns `PATH_FOUND` or
/// `PROVEN_ABSENT`. The bounded `searchWithStateLimit(Graph, long)` variant may
/// additionally return `HamiltonianPathSearchResult.Status#ABORTED` when the search hits
/// its state budget before completing.
///
/// This implementation is a straightforward exact DFS / backtracking solver for Hamiltonian
/// path existence, using standard pruning and candidate-ordering ideas rather than reproducing
/// any single published algorithm verbatim. For background see Rubin, F., "A Search Procedure
/// for Hamilton Paths and Circuits", JACM 21(4), 1974 (backtracking search); the
/// minimum-remaining-values candidate ordering is in the spirit of Warnsdorff's rule (Warnsdorff,
/// "Des Rösselsprunges einfachste und allgemeinste Lösung", 1823) and of
/// constraint-satisfaction reachability propagation (Tsang, "Foundations of Constraint
/// Satisfaction", 1993); the structural prechecks (block / cut-vertex / bridge / SCC) are
/// well-known necessary conditions from graph theory (see Diestel, "Graph Theory", chapter 3,
/// for the underlying decompositions).
///
/// The general Hamiltonian path problem is NP-complete. This implementation is exact and runs in
/// exponential time in the worst case, so callers should expect it to be suitable for relatively
/// small graphs (the practical limit depends heavily on graph structure: sparse and highly
/// constrained graphs are typically tractable for much larger `n` than dense random
/// graphs).
///
/// The implementation applies the following correctness-preserving prechecks and search
/// heuristics, none of which can cause a false negative:
///
/// - If the graph contains exactly one vertex, the singleton path is returned.
/// - If an undirected graph is not connected, no Hamiltonian path can exist, so a
///   `HamiltonianPathSearchResult.Status#PROVEN_ABSENT` result is returned without DFS
///   search.
/// - If an undirected graph has more than two vertices of degree 1, no Hamiltonian path can
///   exist (a Hamiltonian path has at most two endpoints, and any degree-1 vertex must be one of
///   them).
/// - For undirected graphs, every cut vertex (articulation point) must belong to at most two
///   biconnected blocks. A Hamiltonian path visits each vertex once with at most two path-edges
///   incident to it, so it cannot enter more than two blocks meeting at a single cut vertex.
/// - For undirected graphs, the bridge tree (whose nodes are the 2-edge-connected components
///   and whose edges are the original graph's bridges) must itself be a path. Equivalently, no
///   2-edge-connected component may have more than two incident bridges, since a Hamiltonian
///   path enters and leaves each such component at most once.
/// - For directed graphs, the strongly connected component condensation must itself admit a
///   Hamiltonian path. Any Hamiltonian path in the original directed graph projects to one on the
///   condensation DAG; when that condensation has no Hamiltonian path, the original graph cannot
///   either.
/// - At every search step the current endpoint must be able to reach every still-unvisited
///   vertex through unvisited intermediaries. Otherwise the branch is pruned.
/// - Candidate next vertices are tried in ascending order of their remaining (unvisited)
///   onward degree. This is a minimum-remaining-values style heuristic: vertices with few onward
///   options tend to fail or commit early, which generally reduces search.
///
/// Empty graphs are rejected with an error, matching the convention used by the other
/// Hamiltonian / TSP solvers (for example `HeldKarpTSP`). Graphs with self-loops are accepted but self-loops
/// are ignored, since they cannot be part of a simple path. In multigraphs, parallel edges
/// between the same pair of vertices collapse into a single DFS branch and the returned path
/// picks an arbitrary representative edge via `Graph#getEdge`; the result is not
/// weight-optimised across parallel edges.
///
/// The returned `GraphPath` is a `GraphWalk` whose vertex list contains every vertex
/// of the graph exactly once, whose consecutive vertices are connected by an edge of the graph
/// (respecting direction in directed graphs), and whose weight is the sum of the chosen edges'
/// weights according to `Graph#getEdgeWeight(Object)`.
///
/// # Rust notes
///
/// `BacktrackingHamiltonianPath`: depth-first search with cheap necessary-condition prechecks,
/// reachability pruning and a fewest-onward-neighbours ordering.
#[derive(Default)]
pub struct BacktrackingHamiltonianPath {
    states_expanded: u64,
    max_states_limit: u64,
    aborted: bool,
}

impl BacktrackingHamiltonianPath {
    /// Constructs a new instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of states expanded by the last search.
    pub fn states_expanded(&self) -> u64 {
        self.states_expanded
    }

    /// Like [`get_path`](HamiltonianPathAlgorithm::get_path) but gives up after `max_states`
    /// states with an `Aborted` result.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn search_with_state_limit<G: IncidenceGraph>(
        &mut self,
        graph: &G,
        max_states: u64,
    ) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        if max_states == 0 {
            return Err(GraphError::must_be_positive("max_states"));
        }
        self.search(graph, max_states)
    }

    fn search<G: IncidenceGraph>(
        &mut self,
        graph: &G,
        max_states: u64,
    ) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        require_directed_or_undirected(graph)?;
        self.states_expanded = 0;
        self.aborted = false;
        self.max_states_limit = max_states;
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(HamiltonianPathSearchResult::found(singleton_path(graph), 0));
        }
        let directed = graph.graph_type().directed;
        if !directed && !Self::cheap_undirected_prechecks(graph) {
            return Ok(HamiltonianPathSearchResult::proven_absent(
                self.states_expanded,
            ));
        }
        let index_to_vertex: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let vertex_to_index: HashMap<G::Vertex, usize> = index_to_vertex
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let adjacency = build_adjacency(graph, &index_to_vertex, &vertex_to_index, directed);
        if directed && !Self::cheap_directed_prechecks(&adjacency) {
            return Ok(HamiltonianPathSearchResult::proven_absent(
                self.states_expanded,
            ));
        }
        let mut path_idx = vec![0usize; n];
        let mut visited = vec![false; n];
        for start in 0..n {
            path_idx[0] = start;
            visited[start] = true;
            if self.extend(&adjacency, &mut path_idx, &mut visited, 1, n) {
                let vertices = path_idx
                    .iter()
                    .map(|&i| index_to_vertex[i].clone())
                    .collect();
                return Ok(HamiltonianPathSearchResult::found(
                    vertex_list_to_path(vertices, graph)?,
                    self.states_expanded,
                ));
            }
            visited[start] = false;
            if self.aborted {
                return Ok(HamiltonianPathSearchResult::aborted(self.states_expanded));
            }
        }
        Ok(HamiltonianPathSearchResult::proven_absent(
            self.states_expanded,
        ))
    }

    /// Cheap necessary conditions for the existence of a Hamiltonian path in an undirected graph.
    /// Returns `false` if the graph trivially has no Hamiltonian path, `true`
    /// otherwise. A `true` result does not guarantee a path exists.
    fn cheap_undirected_prechecks<G: IncidenceGraph>(graph: &G) -> bool {
        if !ConnectivityInspector::new().is_connected(graph) {
            return false;
        }
        let mut leaves = 0;
        for v in graph.vertices() {
            let v = v.borrow();
            if Self::effective_undirected_degree(graph, v) == 1 {
                leaves += 1;
                if leaves > 2 {
                    return false;
                }
            }
        }
        let bcc = BiconnectivityInspector::new(graph);
        Self::cut_vertex_block_degree_within_path_budget(&bcc)
            && Self::bridge_tree_degree_within_path_budget(graph, &bcc)
    }

    /// Necessary condition: in the block-cut tree of an undirected graph, every cut vertex must
    /// belong to at most two biconnected blocks. A cut vertex incident to more than two blocks
    /// disconnects the graph into more than two components on removal, but a Hamiltonian path
    /// uses at most two edges at any internal vertex, so it cannot weave through more than two
    /// such components.
    fn cut_vertex_block_degree_within_path_budget<G: IncidenceGraph>(
        bcc: &BiconnectivityInspector<'_, G>,
    ) -> bool {
        bcc.cutpoints()
            .iter()
            .all(|cut| bcc.blocks_of(cut).map(|b| b.len() <= 2).unwrap_or(true))
    }

    /// Necessary condition: in the bridge tree (where 2-edge-connected components are nodes and
    /// bridges are edges) every component must have at most two incident bridges. A Hamiltonian
    /// path passing through a 2-edge-connected component enters and leaves it at most once, so
    /// it can use at most two of the component's incident bridges. This prune catches some
    /// graphs the cut-vertex check misses, in particular those where each bridge attaches to a
    /// different vertex inside a single component.
    fn bridge_tree_degree_within_path_budget<G: IncidenceGraph>(
        graph: &G,
        bcc: &BiconnectivityInspector<'_, G>,
    ) -> bool {
        let bridges = bcc.bridges();
        if bridges.len() <= 2 {
            return true; // the bridge tree then has degree <= 2 everywhere
        }
        let bridge_set: HashSet<&G::Edge> = bridges.iter().collect();
        let bridge_free =
            crate::view::MaskSubgraph::new(graph, |_| false, |e: &G::Edge| bridge_set.contains(e));
        let component_sets = ConnectivityInspector::new().connected_sets(&bridge_free);
        let mut vertex_to_component: HashMap<&G::Vertex, usize> = HashMap::new();
        for (i, set) in component_sets.iter().enumerate() {
            for v in set {
                vertex_to_component.insert(v, i);
            }
        }
        let mut incident = vec![0usize; component_sets.len()];
        for bridge in bridges {
            let (s, t) = graph.edge_endpoints(bridge).expect("edge of the graph");
            let cu = vertex_to_component[s];
            let cv = vertex_to_component[t];
            incident[cu] += 1;
            if cu != cv {
                incident[cv] += 1;
            }
            if incident[cu] > 2 || incident[cv] > 2 {
                return false;
            }
        }
        true
    }

    /// The condensation of the digraph must itself have a Hamiltonian path.
    fn cheap_directed_prechecks(adjacency: &[Vec<usize>]) -> bool {
        let n = adjacency.len();
        let (comp_of, count) = strongly_connected_components(adjacency);
        if count <= 1 {
            return true;
        }
        let mut cond: Vec<IndexSet<usize>> = vec![IndexSet::new(); count];
        for (u, targets) in adjacency.iter().enumerate().take(n) {
            for &v in targets {
                if comp_of[u] != comp_of[v] {
                    cond[comp_of[u]].insert(comp_of[v]);
                }
            }
        }
        // longest path in the condensation, which is a DAG
        let mut in_degree = vec![0usize; count];
        for targets in &cond {
            for &v in targets {
                in_degree[v] += 1;
            }
        }
        let mut queue: VecDeque<usize> = (0..count).filter(|&i| in_degree[i] == 0).collect();
        let mut longest = vec![1usize; count];
        let mut best = 1;
        while let Some(u) = queue.pop_front() {
            for &v in &cond[u] {
                longest[v] = longest[v].max(longest[u] + 1);
                best = best.max(longest[v]);
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        best == count
    }

    /// Degree of `v` in an undirected graph, ignoring self-loops since they cannot
    /// participate in a simple path.
    fn effective_undirected_degree<G: IncidenceGraph>(graph: &G, v: &G::Vertex) -> usize {
        graph
            .edges_of(v)
            .filter(|e| graph.opposite_vertex(e, v).is_some_and(|o| o != v))
            .count()
    }

    /// Recursive DFS extension. Returns `true` and leaves `pathIdx` filled with a
    /// Hamiltonian vertex sequence as soon as one is discovered. Applies reachability pruning
    /// and minimum-remaining-values candidate ordering.
    fn extend(
        &mut self,
        adjacency: &[Vec<usize>],
        path_idx: &mut [usize],
        visited: &mut [bool],
        depth: usize,
        n: usize,
    ) -> bool {
        if self.aborted {
            return false;
        }
        if self.max_states_limit > 0 && self.states_expanded >= self.max_states_limit {
            self.aborted = true;
            return false;
        }
        self.states_expanded += 1;
        if depth == n {
            return true;
        }
        let current = path_idx[depth - 1];
        let remaining = n - depth;
        if !Self::all_remaining_reachable(adjacency, visited, current, remaining) {
            return false;
        }
        let neighbours = &adjacency[current];
        let mut candidates = vec![0usize; neighbours.len()];
        let mut onward_degrees = vec![0usize; neighbours.len()];
        let mut k = 0;
        for &next in neighbours {
            if !visited[next] {
                candidates[k] = next;
                onward_degrees[k] = Self::onward_degree(adjacency, visited, next);
                k += 1;
            }
        }
        // insertion sort by onward degree (stable)
        for i in 1..k {
            let cv = candidates[i];
            let dv = onward_degrees[i];
            let mut j = i as isize - 1;
            while j >= 0 && onward_degrees[j as usize] > dv {
                onward_degrees[j as usize + 1] = onward_degrees[j as usize];
                candidates[j as usize + 1] = candidates[j as usize];
                j -= 1;
            }
            onward_degrees[(j + 1) as usize] = dv;
            candidates[(j + 1) as usize] = cv;
        }
        for &next in candidates.iter().take(k) {
            visited[next] = true;
            path_idx[depth] = next;
            if self.extend(adjacency, path_idx, visited, depth + 1, n) {
                return true;
            }
            visited[next] = false;
        }
        false
    }

    /// Returns `true` when every still-unvisited vertex is reachable from `start`
    /// through unvisited vertices using the supplied adjacency (which respects edge direction in
    /// directed graphs). Returning `false` proves the current branch cannot be completed
    /// into a Hamiltonian path. The check itself never causes a false negative because Hamiltonian
    /// path existence from the current endpoint requires every remaining vertex to be reachable
    /// from it.
    fn all_remaining_reachable(
        adjacency: &[Vec<usize>],
        visited: &[bool],
        start: usize,
        remaining: usize,
    ) -> bool {
        if remaining == 0 {
            return true;
        }
        let n = visited.len();
        let mut reached = vec![false; n];
        let mut queue = vec![0usize; n];
        let (mut head, mut tail) = (0, 0);
        queue[tail] = start;
        tail += 1;
        reached[start] = true;
        let mut found = 0;
        while head < tail {
            let u = queue[head];
            head += 1;
            for &v in &adjacency[u] {
                if !reached[v] && !visited[v] {
                    reached[v] = true;
                    queue[tail] = v;
                    tail += 1;
                    found += 1;
                    if found == remaining {
                        return true;
                    }
                }
            }
        }
        found == remaining
    }

    /// Counts the unvisited neighbours of `v` reachable in one step along the supplied
    /// adjacency.
    fn onward_degree(adjacency: &[Vec<usize>], visited: &[bool], v: usize) -> usize {
        adjacency[v].iter().filter(|&&w| !visited[w]).count()
    }
}

impl<G: IncidenceGraph> HamiltonianPathAlgorithm<G> for BacktrackingHamiltonianPath {
    /// Computes a Hamiltonian path in the given graph and returns a tri-state
    /// `HamiltonianPathSearchResult`.
    ///
    /// The return value distinguishes:
    ///
    /// - `HamiltonianPathSearchResult.Status#PATH_FOUND` — the algorithm produced a
    ///   Hamiltonian path; it is accessible via
    ///   `get_path`.
    /// - `HamiltonianPathSearchResult.Status#PROVEN_ABSENT` — the algorithm ran to
    ///   completion (or was rejected by a sound precheck) and proved that no Hamiltonian path
    ///   exists.
    /// - `HamiltonianPathSearchResult.Status#ABORTED` — the algorithm performed a bounded
    ///   search that hit its limit before completing. Whether a Hamiltonian path exists in the
    ///   graph is unknown.
    ///
    /// Unbounded implementations only return `PATH_FOUND` or `PROVEN_ABSENT`.
    /// Implementations that accept an execution budget (e.g. a maximum state count) may also
    /// return `ABORTED`; such bounded variants must never report `PROVEN_ABSENT`
    /// for a search that was stopped early.
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        self.search(graph, 0)
    }
}

/// Tarjan's algorithm (iterative): the component of every vertex and the component count.
fn strongly_connected_components(adjacency: &[Vec<usize>]) -> (Vec<usize>, usize) {
    const NONE: usize = usize::MAX;
    let n = adjacency.len();
    let mut index = vec![NONE; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut comp = vec![NONE; n];
    let mut stack = Vec::new();
    let mut next_index = 0;
    let mut count = 0;
    for root in 0..n {
        if index[root] != NONE {
            continue;
        }
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (v, ref mut child)) = call.last_mut() {
            if *child < adjacency[v].len() {
                let w = adjacency[v][*child];
                *child += 1;
                if index[w] == NONE {
                    index[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(parent, _)) = call.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    loop {
                        let w = stack.pop().expect("component member");
                        on_stack[w] = false;
                        comp[w] = count;
                        if w == v {
                            break;
                        }
                    }
                    count += 1;
                }
            }
        }
    }
    (comp, count)
}
