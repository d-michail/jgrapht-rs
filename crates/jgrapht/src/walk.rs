// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Paths/walks through a graph and vertex/edge mappings between graphs.

use crate::graph::{Graph, GraphExt};
use crate::{GraphError, Result};
use std::collections::HashMap;
use std::hash::Hash;

/// A walk: consecutive vertices joined by edges (JGraphT's `GraphWalk` / `GraphPath`).
///
/// Both lists are always materialised: the empty walk has neither vertices nor edges, a
/// zero-length walk one vertex and no edges, and otherwise `vertices.len() == edges.len() + 1`.
/// The start and end vertex are the first and last entry of `vertices`.
///
/// The fields are private so that this cardinality invariant cannot be broken from outside; read
/// them with [`GraphWalk::vertex_list`], [`GraphWalk::edge_list`] and [`GraphWalk::weight`], and
/// build walks with [`GraphWalk::from_parts`], [`GraphWalk::from_vertex_list`],
/// [`GraphWalk::from_edge_list`] or [`GraphWalk::try_new`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "serde_walk::RawWalk<V, E>"))]
pub struct GraphWalk<V, E> {
    pub(crate) vertices: Vec<V>,
    pub(crate) edges: Vec<E>,
    pub(crate) weight: f64,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(GraphError::InvalidGraphWalk(msg.into()))
}

impl<V, E> GraphWalk<V, E> {
    /// A walk from lists the caller guarantees to be consistent (`vertices.len() == edges.len() + 1`,
    /// or both empty). Library code that builds walks from a search result uses this; the checked
    /// public form is [`GraphWalk::from_parts`].
    pub(crate) fn new_unchecked(vertices: Vec<V>, edges: Vec<E>, weight: f64) -> Self {
        debug_assert!(
            (vertices.is_empty() && edges.is_empty()) || vertices.len() == edges.len() + 1,
            "GraphWalk invariant: vertices.len() == edges.len() + 1"
        );
        GraphWalk {
            vertices,
            edges,
            weight,
        }
    }

    /// A walk from a vertex list and an edge list, without looking at any graph.
    ///
    /// # Errors
    ///
    /// [`GraphError::InvalidGraphWalk`] unless both lists are empty (the empty walk) or
    /// `vertices.len() == edges.len() + 1`. Use [`GraphWalk::verify`] to check the walk against a
    /// graph.
    pub fn from_parts(vertices: Vec<V>, edges: Vec<E>, weight: f64) -> Result<Self> {
        let empty = vertices.is_empty() && edges.is_empty();
        if !empty && vertices.len() != edges.len() + 1 {
            return invalid(
                "VertexList and edgeList do not correspond to the same path (cardinality of vertexList +1 must equal the cardinality of the edgeList)",
            );
        }
        Ok(GraphWalk {
            vertices,
            edges,
            weight,
        })
    }
    /// The empty walk (`GraphWalk.emptyWalk`).
    pub fn empty() -> Self {
        GraphWalk {
            vertices: Vec::new(),
            edges: Vec::new(),
            weight: 0.0,
        }
    }
    /// Zero-length walk at `v` with the given weight (`GraphWalk.singletonWalk`).
    pub fn singleton(v: V, weight: f64) -> Self {
        GraphWalk {
            vertices: vec![v],
            edges: Vec::new(),
            weight,
        }
    }
    /// The first vertex of the walk, or `None` for an empty walk.
    pub fn start_vertex(&self) -> Option<&V> {
        self.vertices.first()
    }
    /// The last vertex of the walk, or `None` for an empty walk.
    pub fn end_vertex(&self) -> Option<&V> {
        self.vertices.last()
    }
    /// All vertices of the walk in order; one more than the number of edges for a non-empty walk.
    pub fn vertex_list(&self) -> &[V] {
        &self.vertices
    }
    /// The edges of the walk in order.
    pub fn edge_list(&self) -> &[E] {
        &self.edges
    }
    /// Total weight of the walk, as recorded when it was built.
    pub fn weight(&self) -> f64 {
        self.weight
    }
    /// Replaces the recorded weight.
    pub fn set_weight(&mut self, weight: f64) {
        self.weight = weight;
    }
    /// Number of edges.
    pub fn length(&self) -> usize {
        self.edges.len()
    }
    /// Whether the walk has no vertices at all.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> GraphWalk<V, E> {
    /// Zero-length walk with weight `0`.
    pub fn single(v: V) -> Self {
        Self::singleton(v, 0.0)
    }

    /// Builds a walk from its edges; the vertex list is derived from `g`.
    /// Returns `None` if the edges are not consecutive starting at `start`.
    pub fn from_edges<G>(g: &G, start: V, edges: Vec<E>, weight: f64) -> Option<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        let mut vertices = vec![start.clone()];
        let mut cur = start;
        for e in &edges {
            cur = g.opposite_vertex(e, &cur)?.clone();
            vertices.push(cur.clone());
        }
        Some(GraphWalk {
            vertices,
            edges,
            weight,
        })
    }

    /// Builds a walk from a vertex list, picking the lightest edge between consecutive vertices.
    pub fn from_vertices<G>(g: &G, vertices: Vec<V>) -> Option<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        vertices.first()?;
        let mut edges = Vec::with_capacity(vertices.len() - 1);
        let mut weight = 0.0;
        for w in vertices.windows(2) {
            let (e, we) = g
                .edges_between(&w[0], &w[1])
                .map(|e| (e, g.weight_of(e)))
                .min_by(|a, b| a.1.total_cmp(&b.1))?;
            edges.push(e.clone());
            weight += we;
        }
        Some(GraphWalk {
            vertices,
            edges,
            weight,
        })
    }

    /// `new GraphWalk(graph, start, end, vertexList, edgeList, weight)` followed by `verify()`:
    /// at least one of the two lists must be given and a missing one is derived from the graph.
    /// `start`/`end` must both be given or both absent (an absent pair denotes the empty walk).
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn try_new<G>(
        g: &G,
        start: Option<V>,
        end: Option<V>,
        vertex_list: Option<Vec<V>>,
        edge_list: Option<Vec<E>>,
        weight: f64,
    ) -> Result<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        if vertex_list.is_none() && edge_list.is_none() {
            return Err(GraphError::InvalidGraphWalk(
                "Vertex list and edge list cannot both be null!".into(),
            ));
        }
        if let (Some(_), Some(vl), Some(el)) = (&start, &vertex_list, &edge_list) {
            if el.len() + 1 != vl.len() {
                return Err(GraphError::InvalidGraphWalk(
                    "VertexList and edgeList do not correspond to the same path (cardinality of vertexList +1 must equal the cardinality of the edgeList)".into(),
                ));
            }
        }
        if start.is_some() != end.is_some() {
            return Err(GraphError::InvalidGraphWalk(
                "Either the start and end vertices must both be null, or they must both be not null (one of them is null)".into(),
            ));
        }
        let Some(start) = start else {
            return Ok(GraphWalk {
                vertices: Vec::new(),
                edges: Vec::new(),
                weight,
            });
        };
        let end = end.expect("checked above");
        let walk = match (vertex_list, edge_list) {
            (Some(vertices), Some(edges)) => GraphWalk {
                vertices,
                edges,
                weight,
            },
            (Some(vertices), None) => {
                let mut edges = Vec::new();
                for w in vertices.windows(2) {
                    match g.edge_between(&w[0], &w[1]) {
                        Some(e) => edges.push(e.clone()),
                        None => {
                            return invalid(format!(
                                "The vertexList does not constitute to a feasible path. Edge ({}) does not exist in the graph.",
                                "u,v"
                            ));
                        }
                    }
                }
                GraphWalk {
                    vertices,
                    edges,
                    weight,
                }
            }
            (None, Some(edges)) => {
                let mut vertices = vec![start.clone()];
                let mut u = start.clone();
                if let Some(first) = edges.first() {
                    if !g.test_incidence(first, &start) {
                        return invalid(
                            "The first edge in the edge list must leave the start vertex",
                        );
                    }
                }
                for e in &edges {
                    match g.opposite_vertex(e, &u) {
                        Some(n) if g.test_incidence(e, &u) => {
                            u = n.clone();
                            vertices.push(u.clone());
                        }
                        _ => {
                            return invalid("The edgeList does not constitute to a feasible path.");
                        }
                    }
                }
                if u != end {
                    return invalid(
                        "The path defined by the edgeList does not end in the endVertex.",
                    );
                }
                GraphWalk {
                    vertices,
                    edges,
                    weight,
                }
            }
            (None, None) => unreachable!(),
        };
        if walk.vertices.first() != Some(&start) {
            return invalid("The start vertex must be the first vertex in the vertex list");
        }
        if walk.vertices.last() != Some(&end) {
            return invalid("The end vertex must be the last vertex in the vertex list");
        }
        walk.verify(g)?;
        Ok(walk)
    }

    /// `new GraphWalk(graph, vertexList, weight)`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_vertex_list<G>(g: &G, vertices: Vec<V>, weight: f64) -> Result<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        let (start, end) = (vertices.first().cloned(), vertices.last().cloned());
        Self::try_new(g, start, end, Some(vertices), None, weight)
    }

    /// `new GraphWalk(graph, start, end, edgeList, weight)`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_edge_list<G>(g: &G, start: V, end: V, edges: Vec<E>, weight: f64) -> Result<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        Self::try_new(g, Some(start), Some(end), None, Some(edges), weight)
    }

    /// Checks that the walk really exists in `g` (`GraphWalk.verify`).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EdgeNotFound`] if an edge argument is not in the graph. Errors of the
    /// graph operations it performs are propagated.
    pub fn verify<G>(&self, g: &G) -> Result<()>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        if self.is_empty() {
            return Ok(());
        }
        if self.edges.len() + 1 != self.vertices.len() {
            return invalid(
                "VertexList and edgeList do not correspond to the same path (cardinality of vertexList +1 must equal the cardinality of the edgeList)",
            );
        }
        if !self.vertices.iter().all(|v| g.contains_vertex(v)) {
            return invalid("Not all vertices in the path are contained in the graph");
        }
        if !self.edges.iter().all(|e| g.contains_edge(e)) {
            return invalid("Not all edges in the path are contained in the graph");
        }
        let directed = g.graph_type().directed;
        for (i, e) in self.edges.iter().enumerate() {
            let (u, v) = (&self.vertices[i], &self.vertices[i + 1]);
            let (s, t) = g.edge_endpoints(e).ok_or(GraphError::EdgeNotFound)?;
            let ok = if directed {
                s == u && t == v
            } else {
                (s == u && t == v) || (s == v && t == u)
            };
            if !ok {
                return invalid("VertexList and edgeList do not form a feasible path");
            }
        }
        Ok(())
    }

    /// Whether this walk exists in `g` and its recorded weight is the sum of its edge weights.
    pub fn is_valid<G>(&self, g: &G) -> bool
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        if self.verify(g).is_err() {
            return false;
        }
        let w: f64 = self.edges.iter().map(|e| g.weight_of(e)).sum();
        (w - self.weight).abs() <= 1e-9 * w.abs().max(1.0)
    }

    /// The reversed walk; `weight` computes the new weight (default: the reversed edges' weights,
    /// or the old weight in undirected graphs). Directed walks need the reverse arcs to exist.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EdgeNotFound`] if an edge argument is not in the graph. Errors of the
    /// graph operations it performs are propagated.
    pub fn reverse<G>(&self, g: &G, weight: Option<&dyn Fn(&Self) -> f64>) -> Result<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        let vertices: Vec<V> = self.vertices.iter().rev().cloned().collect();
        let (edges, rev_weight): (Vec<E>, f64) = if !g.graph_type().directed {
            (self.edges.iter().rev().cloned().collect(), self.weight)
        } else {
            let mut es = Vec::with_capacity(self.edges.len());
            let mut w = 0.0;
            for e in self.edges.iter().rev() {
                let (s, t) = g.edge_endpoints(e).ok_or(GraphError::EdgeNotFound)?;
                match g.edge_between(t, s) {
                    Some(r) => {
                        w += g.weight_of(r);
                        es.push(r.clone());
                    }
                    None => {
                        return invalid(
                            "this walk cannot be reversed. The graph does not contain a reverse arc",
                        );
                    }
                }
            }
            (es, w)
        };
        let mut rev = GraphWalk {
            vertices,
            edges,
            weight: 0.0,
        };
        rev.weight = match weight {
            Some(f) => f(&rev),
            None => rev_weight,
        };
        Ok(rev)
    }

    /// Reversed walk keeping the weight of an undirected walk.
    pub fn reversed(&self) -> Self {
        GraphWalk {
            vertices: self.vertices.iter().rev().cloned().collect(),
            edges: self.edges.iter().rev().cloned().collect(),
            weight: self.weight,
        }
    }

    /// Appends `extension`, which must start where this walk ends; `weight` computes the new weight.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn concat(&self, extension: &Self, weight: &dyn Fn(&Self) -> f64) -> Result<Self> {
        if self.is_empty() {
            return Err(GraphError::InvalidGraphWalk(
                "An empty path cannot be extended".into(),
            ));
        }
        if extension.is_empty() || self.end_vertex() != extension.start_vertex() {
            return Err(GraphError::InvalidGraphWalk(
                "This path can only be extended by another path if the end vertex of the orginal path and the start vertex of the extension are equal.".into(),
            ));
        }
        let mut vertices = self.vertices.clone();
        vertices.extend(extension.vertices.iter().skip(1).cloned());
        let mut edges = self.edges.clone();
        edges.extend(extension.edges.iter().cloned());
        let mut walk = GraphWalk {
            vertices,
            edges,
            weight: 0.0,
        };
        walk.weight = weight(&walk);
        Ok(walk)
    }

    /// Appends `other` adding the weights (`None` unless it starts where `self` ends).
    pub fn concat_sum(&self, other: &Self) -> Option<Self> {
        self.concat(other, &|_| self.weight + other.weight).ok()
    }
}

/// `GraphWalk.equals`: empty walks are equal; otherwise start, end and edge lists must match.
impl<V: PartialEq, E: PartialEq> PartialEq for GraphWalk<V, E> {
    fn eq(&self, other: &Self) -> bool {
        match (self.is_empty(), other.is_empty()) {
            (true, true) => true,
            (false, false) => {
                self.vertices.first() == other.vertices.first()
                    && self.vertices.last() == other.vertices.last()
                    && self.edges == other.edges
            }
            _ => false,
        }
    }
}

impl<V: Hash, E: Hash> Hash for GraphWalk<V, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_empty() {
            return;
        }
        self.vertices.first().hash(state);
        self.vertices.last().hash(state);
        self.edges.hash(state);
    }
}

impl<V: std::fmt::Display, E: std::fmt::Display> std::fmt::Display for GraphWalk<V, E> {
    /// The vertex list, e.g. `[a, b, c]`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.vertices.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "]")
    }
}

/// Bidirectional mapping of vertices and edges from one graph (`V`, `E`) to another
/// (`V2`, `E2`, defaulting to the same types).
#[derive(Debug, Clone)]
///
/// The forward and backward maps are kept consistent by [`GraphMapping::map_vertex`] and
/// [`GraphMapping::map_edge`], so they are private; look entries up with
/// [`GraphMapping::vertex`], [`GraphMapping::vertex_inverse`], [`GraphMapping::edge`] and
/// [`GraphMapping::edge_inverse`].
pub struct GraphMapping<V, E, V2 = V, E2 = E> {
    pub(crate) vertex_forward: HashMap<V, V2>,
    pub(crate) vertex_backward: HashMap<V2, V>,
    pub(crate) edge_forward: HashMap<E, E2>,
    pub(crate) edge_backward: HashMap<E2, E>,
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, V2: Clone + Eq + Hash, E2: Clone + Eq + Hash>
    Default for GraphMapping<V, E, V2, E2>
{
    fn default() -> Self {
        GraphMapping {
            vertex_forward: HashMap::new(),
            vertex_backward: HashMap::new(),
            edge_forward: HashMap::new(),
            edge_backward: HashMap::new(),
        }
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, V2: Clone + Eq + Hash, E2: Clone + Eq + Hash>
    GraphMapping<V, E, V2, E2>
{
    /// An empty mapping.
    pub fn new() -> Self {
        Self::default()
    }
    /// Records that the vertex `from` of the first graph corresponds to `to` in the second.
    pub fn map_vertex(&mut self, from: V, to: V2) {
        self.vertex_forward.insert(from.clone(), to.clone());
        self.vertex_backward.insert(to, from);
    }
    /// Records that the edge `from` of the first graph corresponds to `to` in the second.
    pub fn map_edge(&mut self, from: E, to: E2) {
        self.edge_forward.insert(from.clone(), to.clone());
        self.edge_backward.insert(to, from);
    }
    /// The vertex that `from` is mapped to, or `None` if it has no image.
    pub fn vertex(&self, from: &V) -> Option<&V2> {
        self.vertex_forward.get(from)
    }
    /// The vertex that maps to `to`, or `None` if nothing does.
    pub fn vertex_inverse(&self, to: &V2) -> Option<&V> {
        self.vertex_backward.get(to)
    }
    /// The edge that `from` is mapped to, or `None` if it has no image.
    pub fn edge(&self, from: &E) -> Option<&E2> {
        self.edge_forward.get(from)
    }
    /// The edge that maps to `to`, or `None` if nothing does.
    pub fn edge_inverse(&self, to: &E2) -> Option<&E> {
        self.edge_backward.get(to)
    }
}

/// Deserialization checks the cardinality invariant through [`GraphWalk::from_parts`], so corrupt
/// data cannot produce a walk that breaks it.
#[cfg(feature = "serde")]
mod serde_walk {
    use super::GraphWalk;
    use crate::GraphError;
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct RawWalk<V, E> {
        vertices: Vec<V>,
        edges: Vec<E>,
        weight: f64,
    }

    impl<V, E> TryFrom<RawWalk<V, E>> for GraphWalk<V, E> {
        type Error = GraphError;

        fn try_from(raw: RawWalk<V, E>) -> Result<Self, GraphError> {
            GraphWalk::from_parts(raw.vertices, raw.edges, raw.weight)
        }
    }
}
