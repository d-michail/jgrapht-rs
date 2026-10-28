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

use crate::graph::{DirectedGraph, Graph, GraphExt};
use crate::graph::{IncomingGraph, OutgoingGraph};
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Path validator for shortest path algorithms. Shortest path algorithms typically maintain a set of
/// partial paths which are iteratively extended until some stopping criterion is met, e.g. the
/// target vertex is reached. A path validator can be used to enforce additional criteria as to the
/// validity of a given path. A partial path can only be extended by another edge if the path
/// validator deems this extension feasible. As per example, a path validator can be used to enforce
/// that a path connecting a source to a terminal vertex visits at least $n$ vertices.
///
/// # Rust notes
///
/// Decides whether an edge may be appended to a partial path.
///
/// This is the Rust counterpart of JGraphT's `PathValidator`. The validator is deliberately not
/// called for zero-length paths.
pub trait PathValidator<G: Graph> {
    /// Whether `edge` may extend `partial_path`.
    fn is_valid_path(&self, partial_path: &GraphWalk<G::Vertex, G::Edge>, edge: &G::Edge) -> bool;
}

impl<G, F> PathValidator<G> for F
where
    G: Graph,
    F: Fn(&GraphWalk<G::Vertex, G::Edge>, &G::Edge) -> bool,
{
    fn is_valid_path(&self, partial_path: &GraphWalk<G::Vertex, G::Edge>, edge: &G::Edge) -> bool {
        self(partial_path, edge)
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Accepts every path.
pub struct AllowAllPaths;

impl<G: Graph> PathValidator<G> for AllowAllPaths {
    fn is_valid_path(&self, _: &GraphWalk<G::Vertex, G::Edge>, _: &G::Edge) -> bool {
        true
    }
}

/// A Dijkstra-like algorithm to find all paths between two sets of nodes in a directed graph, with
/// options to search only simple paths and to limit the path length.
///
/// The algorithm runs in two phases. Preprocessing decorates each edge with the minimum number of
/// edges still needed to reach a target. Enumeration then walks the decorated edges from the source
/// set, using the decoration to abandon any partial path that cannot complete within the budget. By
/// default a forward BFS from the source set is also run as part of preprocessing, so that edges
/// whose source endpoint is not reachable from any requested source within the budget are dropped
/// from the decoration up front. This can substantially reduce preprocessing cost on graphs where a
/// non-trivial fraction of the target-reachable subgraph is not reachable from the sources. See
/// `setForwardPruning(boolean)` for how to disable it.
///
/// # Rust notes
///
/// Enumerates directed paths between one or more sources and targets.
///
/// As in Java's `AllDirectedPaths`, preprocessing decorates every usable edge with its minimum
/// remaining hop distance to a target. An optional forward BFS applies the exact sandwich prune
/// before enumeration. The preprocessing is `O(V + E)`; enumeration is necessarily proportional
/// to the number and total length of returned paths.
pub struct AllDirectedPaths<'g, G: DirectedGraph, P = AllowAllPaths> {
    graph: &'g G,
    path_validator: P,
    forward_pruning: bool,
}

impl<'g, G: DirectedGraph> AllDirectedPaths<'g, G, AllowAllPaths> {
    /// Create a new instance.
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            path_validator: AllowAllPaths,
            forward_pruning: true,
        }
    }

    /// Creates an instance with the given validator.
    pub fn with_validator<P>(graph: &'g G, path_validator: P) -> AllDirectedPaths<'g, G, P>
    where
        P: PathValidator<G>,
    {
        AllDirectedPaths {
            graph,
            path_validator,
            forward_pruning: true,
        }
    }
}

impl<G: DirectedGraph + IncomingGraph + OutgoingGraph, P: PathValidator<G>>
    AllDirectedPaths<'_, G, P>
{
    /// Configure whether the preprocessing step applies the forward-pruning optimisation.
    ///
    /// When forward pruning is enabled (the default), `getAllPaths` first runs a forward
    /// BFS from the source set and uses the result to drop edges whose source endpoint is not
    /// reachable from any source within the bound, or whose forward-plus-backward length exceeds
    /// the bound. The prune is exact — it never drops an edge that could lie on a feasible
    /// source-to-target walk — and can be a large win when a substantial fraction of the
    /// graph is backward-reachable from the targets but not forward-reachable from the sources.
    /// On graphs where the prune never fires (e.g. small dense strongly-connected digraphs), the
    /// optimisation adds the cost of one extra `O(V + E)` BFS per `getAllPaths`
    /// call.
    ///
    /// Setting forward pruning to `false` recovers the historical preprocessing behaviour
    /// exactly — useful when the cost of the extra BFS is known to dominate.
    pub fn set_forward_pruning(&mut self, forward_pruning: bool) {
        self.forward_pruning = forward_pruning;
    }

    /// Whether the preprocessing step currently applies the forward-pruning optimisation.
    pub fn is_forward_pruning(&self) -> bool {
        self.forward_pruning
    }

    /// Calculate (and return) all paths from the source vertex to the target vertex.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn get_all_paths(
        &self,
        source: &G::Vertex,
        target: &G::Vertex,
        simple_paths_only: bool,
        max_path_length: Option<usize>,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        self.get_all_paths_between(
            std::iter::once(source),
            std::iter::once(target),
            simple_paths_only,
            max_path_length,
        )
    }

    /// Returns the all paths between.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::MissingParameter`] if a required parameter was not supplied;
    /// [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of the graph
    /// operations it performs are propagated.
    pub fn get_all_paths_between<'a>(
        &self,
        source_vertices: impl IntoIterator<Item = &'a G::Vertex>,
        target_vertices: impl IntoIterator<Item = &'a G::Vertex>,
        simple_paths_only: bool,
        max_path_length: Option<usize>,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>>
    where
        G::Vertex: 'a,
    {
        if !simple_paths_only && max_path_length.is_none() {
            return Err(GraphError::MissingParameter("max_path_length"));
        }
        let sources: Vec<_> = source_vertices.into_iter().collect();
        let targets: HashSet<_> = target_vertices.into_iter().collect();
        if sources.is_empty() || targets.is_empty() {
            return Ok(Vec::new());
        }
        if sources.iter().any(|v| !self.graph.contains_vertex(v))
            || targets.iter().any(|v| !self.graph.contains_vertex(v))
        {
            return Err(GraphError::VertexNotFound);
        }

        let forward = if self.forward_pruning {
            Some(self.forward_distances(&sources, max_path_length)?)
        } else {
            None
        };
        let edge_to_target =
            self.backward_edge_distances(&targets, forward.as_ref(), max_path_length)?;
        self.generate_paths(
            &sources,
            &targets,
            simple_paths_only,
            max_path_length,
            &edge_to_target,
        )
    }

    fn forward_distances<'a>(
        &'a self,
        sources: &[&'a G::Vertex],
        max_path_length: Option<usize>,
    ) -> Result<HashMap<&'a G::Vertex, usize>> {
        let mut distance = HashMap::new();
        let mut queue = VecDeque::new();
        for &source in sources {
            if distance.insert(source, 0).is_none() {
                queue.push_back(source);
            }
        }
        while let Some(vertex) = queue.pop_front() {
            let d = distance[vertex];
            if max_path_length.is_some_and(|bound| d >= bound) {
                continue;
            }
            for (_, child) in self.graph.outgoing_of(vertex) {
                if !distance.contains_key(child) {
                    distance.insert(child, d + 1);
                    queue.push_back(child);
                }
            }
        }
        Ok(distance)
    }

    fn backward_edge_distances<'a>(
        &'a self,
        targets: &HashSet<&'a G::Vertex>,
        forward: Option<&HashMap<&'a G::Vertex, usize>>,
        max_path_length: Option<usize>,
    ) -> Result<HashMap<&'a G::Edge, usize>> {
        let mut edge_distance = HashMap::new();
        if max_path_length == Some(0) {
            return Ok(edge_distance);
        }
        let mut vertex_distance = HashMap::new();
        let mut queue = VecDeque::new();
        for &target in targets {
            if forward.is_none_or(|f| f.contains_key(target))
                && vertex_distance.insert(target, 0).is_none()
            {
                queue.push_back(target);
            }
        }
        while let Some(vertex) = queue.pop_front() {
            let child_distance = vertex_distance[vertex] + 1;
            for edge in self.graph.in_edges_of(vertex) {
                let source = self
                    .graph
                    .edge_source(edge)
                    .ok_or(crate::GraphError::EdgeNotFound)?;
                if let Some(forward) = forward {
                    let Some(&source_distance) = forward.get(source) else {
                        continue;
                    };
                    if max_path_length.is_some_and(|bound| source_distance > bound - child_distance)
                    {
                        continue;
                    }
                }
                edge_distance
                    .entry(edge)
                    .and_modify(|d| *d = (*d).min(child_distance))
                    .or_insert(child_distance);
                let improve = vertex_distance
                    .get(source)
                    .is_none_or(|&old| child_distance < old);
                if improve {
                    vertex_distance.insert(source, child_distance);
                    if max_path_length.is_none_or(|bound| child_distance < bound) {
                        queue.push_back(source);
                    }
                }
            }
        }
        Ok(edge_distance)
    }

    /// Generate all paths from the sources to the targets, using pre-computed minimum distances.
    fn generate_paths(
        &self,
        sources: &[&G::Vertex],
        targets: &HashSet<&G::Vertex>,
        simple_paths_only: bool,
        max_path_length: Option<usize>,
        edge_to_target: &HashMap<&G::Edge, usize>,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        let mut complete = Vec::new();
        let mut incomplete: VecDeque<GraphWalk<G::Vertex, G::Edge>> = VecDeque::new();

        for &source in sources {
            if targets.contains(source) {
                complete.push(GraphWalk::single(source.clone()));
            }
            if max_path_length == Some(0) {
                continue;
            }
            for (edge, target) in self.graph.outgoing_of(source) {
                let partial = GraphWalk::empty();
                if !self.path_validator.is_valid_path(&partial, edge) {
                    continue;
                }
                let path = GraphWalk::new_unchecked(
                    vec![source.clone(), target.clone()],
                    vec![edge.clone()],
                    self.graph.weight_of(edge),
                );
                if targets.contains(target) {
                    complete.push(path.clone());
                }
                if edge_to_target.contains_key(edge)
                    && max_path_length.is_none_or(|bound| bound > 1)
                {
                    incomplete.push_back(path);
                }
            }
        }

        while let Some(path) = incomplete.pop_front() {
            let length = path.edges.len();
            let leaf = path.vertices.last().expect("non-empty path");
            let path_vertices: HashSet<_> = if simple_paths_only {
                path.vertices.iter().collect()
            } else {
                HashSet::new()
            };
            for (edge, child) in self.graph.outgoing_of(leaf) {
                let Some(&remaining) = edge_to_target.get(edge) else {
                    continue;
                };
                if max_path_length.is_some_and(|bound| remaining + length > bound) {
                    continue;
                }
                if simple_paths_only && path_vertices.contains(child) {
                    continue;
                }
                if !self.path_validator.is_valid_path(&path, edge) {
                    continue;
                }
                let mut next = path.clone();
                next.vertices.push(child.clone());
                next.edges.push(edge.clone());
                next.weight += self.graph.weight_of(edge);
                if targets.contains(child) {
                    complete.push(next.clone());
                }
                if max_path_length.is_none_or(|bound| next.edges.len() < bound) {
                    // Java uses addFirst here; this is a depth-first queue and bounds live memory.
                    incomplete.push_front(next);
                }
            }
        }
        Ok(complete)
    }
}
