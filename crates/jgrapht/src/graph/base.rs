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

use super::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use super::{DirectedGraph, Graph, GraphMut, UndirectedGraph};
use crate::{GraphError, GraphType, Result};
use indexmap::{IndexMap, IndexSet};
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

#[cfg(feature = "serde")]
mod serde_impl;

/// Edge type used when the caller does not supply their own.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefaultEdge(pub u64);

static NEXT_DEFAULT_EDGE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1_000_000);

impl DefaultEdge {
    /// A process-wide unique edge, mirroring the identity of `new DefaultEdge()` in Java.
    pub fn new() -> Self {
        DefaultEdge(NEXT_DEFAULT_EDGE.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
    /// Edge factory for [`BaseGraph::with_edge_factory`]; the per-graph counter is ignored because
    /// default edges are unique across all graphs.
    pub fn from_id(_: u64) -> Self {
        Self::new()
    }
}

impl Default for DefaultEdge {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DefaultEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

/// Compile-time direction tag for [`BaseGraph`].
pub trait Direction: Clone + Copy + fmt::Debug {
    /// The directed constant.
    const DIRECTED: bool;
}
#[derive(Debug, Clone, Copy)]
/// The `Directed` struct.
pub struct Directed;
#[derive(Debug, Clone, Copy)]
/// The `Undirected` struct.
pub struct Undirected;
impl Direction for Directed {
    const DIRECTED: bool = true;
}
impl Direction for Undirected {
    const DIRECTED: bool = false;
}

/// Storage marker used by the standard [`BaseGraph`] constructors.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultGraphSpecifics;

#[derive(Debug, Clone)]
struct EdgeRec<V> {
    src: V,
    tgt: V,
    weight: f64,
}

/// For undirected graphs only `out` is used and holds every incident edge.
#[derive(Debug, Clone)]
struct Adj<E> {
    inc: IndexSet<E>,
    out: IndexSet<E>,
}

impl<E> Default for Adj<E> {
    fn default() -> Self {
        Adj {
            inc: IndexSet::new(),
            out: IndexSet::new(),
        }
    }
}

/// The default adjacency-map graph. Its capabilities (loops, multi-edges, weights, acyclicity)
/// are described by a [`GraphType`]; direction is fixed by the `D` tag.
#[derive(Clone)]
pub struct BaseGraph<V, E = DefaultEdge, D = Undirected, S = DefaultGraphSpecifics> {
    ty: GraphType,
    vertices: IndexMap<V, Adj<E>>,
    edges: IndexMap<E, EdgeRec<V>>,
    endpoint_index: Option<IndexMap<(V, V), IndexSet<E>>>,
    next_id: u64,
    factory: Option<fn(u64) -> E>,
    next_vertex: u64,
    vertex_factory: Option<fn(u64) -> V>,
    _marker: PhantomData<(D, S)>,
}

/// Undirected [`BaseGraph`] with [`DefaultEdge`]s.
pub type UGraph<V, E = DefaultEdge> = BaseGraph<V, E, Undirected>;
/// Directed [`BaseGraph`] with [`DefaultEdge`]s.
pub type DiGraph<V, E = DefaultEdge> = BaseGraph<V, E, Directed>;

impl<V, E, D: Direction, S> fmt::Debug for BaseGraph<V, E, D, S>
where
    V: fmt::Debug,
    E: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseGraph")
            .field("type", &self.ty)
            .field("vertices", &self.vertices.keys().collect::<Vec<_>>())
            .field(
                "edges",
                &self
                    .edges
                    .iter()
                    .map(|(e, r)| (e, &r.src, &r.tgt))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction>
    BaseGraph<V, E, D, DefaultGraphSpecifics>
{
    /// Creates a graph that mints edges with `factory(counter)` when `add_edge` is used.
    pub fn with_edge_factory(ty: GraphType, factory: fn(u64) -> E) -> Self {
        Self::build(ty, Some(factory), false)
    }

    /// Creates a graph without an edge factory: edges must be added with `add_edge_with`.
    pub fn without_edge_factory(ty: GraphType) -> Self {
        Self::build(ty, None, false)
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> BaseGraph<V, E, D, S> {
    pub(crate) fn build(ty: GraphType, factory: Option<fn(u64) -> E>, fast_lookups: bool) -> Self {
        BaseGraph {
            ty: GraphType {
                directed: D::DIRECTED,
                ..ty
            },
            vertices: IndexMap::new(),
            edges: IndexMap::new(),
            endpoint_index: fast_lookups.then(IndexMap::new),
            next_id: 0,
            factory,
            next_vertex: 0,
            vertex_factory: None,
            _marker: PhantomData,
        }
    }

    /// Whether endpoint pairs are indexed for expected O(1) lookup.
    pub fn has_fast_lookups(&self) -> bool {
        self.endpoint_index.is_some()
    }

    /// The edge factory, if any.
    pub fn edge_factory(&self) -> Option<fn(u64) -> E> {
        self.factory
    }
    /// The vertex factory, if any.
    pub fn vertex_factory(&self) -> Option<fn(u64) -> V> {
        self.vertex_factory
    }

    /// Installs a vertex factory used by [`GraphMut::add_new_vertex`]: `factory(0)`, `factory(1)`, ...
    pub fn with_vertex_factory(mut self, factory: fn(u64) -> V) -> Self {
        self.vertex_factory = Some(factory);
        self
    }

    /// Installs the edge factory used by `add_edge` (builder style). Graphs read with `serde` come
    /// back without one, because function pointers cannot be serialized.
    pub fn with_edge_factory_fn(mut self, factory: fn(u64) -> E) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Marks the graph as weighted (builder style).
    pub fn weighted(mut self) -> Self {
        self.ty.weighted = true;
        self
    }

    /// Whether `target` is reachable from `from` following edge direction.
    fn reaches(&self, from: &V, target: &V) -> bool {
        let mut seen = IndexSet::new();
        let mut stack = vec![from];
        while let Some(x) = stack.pop() {
            if x == target {
                return true;
            }
            if !seen.insert(x) {
                continue;
            }
            for e in self.out_edges(x) {
                let r = &self.edges[e];
                stack.push(if D::DIRECTED || &r.src == x {
                    &r.tgt
                } else {
                    &r.src
                });
            }
        }
        false
    }

    fn out_edges(&self, v: &V) -> impl Iterator<Item = &E> {
        self.vertices.get(v).into_iter().flat_map(|a| a.out.iter())
    }
}

impl<V: Clone + Eq + Hash, D: Direction> BaseGraph<V, DefaultEdge, D> {
    /// Graph with an arbitrary [`GraphType`] (direction comes from `D`) and [`DefaultEdge`]s.
    pub fn with_graph_type(ty: GraphType) -> Self {
        Self::with_edge_factory(ty, DefaultEdge::from_id)
    }
    /// Installs the [`DefaultEdge`] factory, so that `add_edge` works on a graph read with `serde`
    /// (builder style).
    pub fn with_default_edge_factory(self) -> Self {
        self.with_edge_factory_fn(DefaultEdge::from_id)
    }
    /// No loops, no parallel edges.
    pub fn simple() -> Self {
        Self::with_graph_type(GraphType::simple())
    }
    /// Parallel edges allowed, loops not.
    pub fn multi() -> Self {
        Self::with_graph_type(GraphType::multigraph())
    }
    /// Self-loops allowed, parallel edges not (JGraphT's `Default*Graph`).
    pub fn with_loops() -> Self {
        Self::with_graph_type(GraphType {
            self_loops: true,
            ..GraphType::simple()
        })
    }
    /// Parallel edges and loops allowed.
    pub fn pseudo() -> Self {
        Self::with_graph_type(GraphType::pseudograph())
    }
}

impl<V: Clone + Eq + Hash> BaseGraph<V, DefaultEdge, Directed> {
    /// Directed acyclic graph: edge insertion that would close a cycle fails.
    pub fn dag() -> Self {
        Self::with_graph_type(GraphType {
            allows_cycles: false,
            ..GraphType::simple()
        })
    }
    /// Directed acyclic multigraph (parallel edges allowed).
    pub fn dag_multi() -> Self {
        Self::with_graph_type(GraphType {
            allows_cycles: false,
            ..GraphType::multigraph()
        })
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> Graph for BaseGraph<V, E, D, S> {
    type Vertex = V;
    type Edge = E;
    type VertexRef<'a>
        = &'a V
    where
        Self: 'a;
    type EdgeRef<'a>
        = &'a E
    where
        Self: 'a;

    fn graph_type(&self) -> GraphType {
        self.ty
    }
    fn vertices(&self) -> impl Iterator<Item = &V> {
        self.vertices.keys()
    }
    fn edges(&self) -> impl Iterator<Item = &E> {
        self.edges.keys()
    }
    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
    }
    fn contains_vertex(&self, v: &V) -> bool {
        self.vertices.contains_key(v)
    }
    fn contains_edge(&self, e: &E) -> bool {
        self.edges.contains_key(e)
    }
    fn edge_source(&self, e: &E) -> Option<&V> {
        self.edges.get(e).map(|r| &r.src)
    }
    fn edge_target(&self, e: &E) -> Option<&V> {
        self.edges.get(e).map(|r| &r.tgt)
    }
    fn edge_weight(&self, e: &E) -> Option<f64> {
        self.edges
            .get(e)
            .map(|r| if self.ty.weighted { r.weight } else { 1.0 })
    }
    fn edges_between<'a>(&'a self, u: &'a V, v: &'a V) -> impl Iterator<Item = &'a E> {
        if let Some(index) = &self.endpoint_index {
            Box::new(
                index
                    .get(&(u.clone(), v.clone()))
                    .into_iter()
                    .flat_map(|edges| edges.iter()),
            ) as Box<dyn Iterator<Item = &'a E> + 'a>
        } else {
            Box::new(self.out_edges(u).filter(move |e| {
                let r = &self.edges[*e];
                if D::DIRECTED {
                    &r.tgt == v
                } else {
                    (&r.src == u && &r.tgt == v) || (&r.src == v && &r.tgt == u)
                }
            })) as Box<dyn Iterator<Item = &'a E> + 'a>
        }
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> OutgoingGraph
    for BaseGraph<V, E, D, S>
{
    fn out_edges_of(&self, v: &V) -> impl Iterator<Item = &E> {
        self.out_edges(v)
    }
    fn out_degree_of(&self, v: &V) -> usize {
        match self.vertices.get(v) {
            None => 0,
            Some(a) if D::DIRECTED => a.out.len(),
            Some(_) => self.degree_of(v),
        }
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> IncomingGraph
    for BaseGraph<V, E, D, S>
{
    fn in_edges_of(&self, v: &V) -> impl Iterator<Item = &E> {
        let a = self.vertices.get(v);
        // undirected graphs keep every incident edge in `out`
        let (inc, out) = if D::DIRECTED {
            (a.map(|a| &a.inc), None)
        } else {
            (None, a.map(|a| &a.out))
        };
        inc.into_iter()
            .flat_map(|s| s.iter())
            .chain(out.into_iter().flat_map(|s| s.iter()))
    }
    fn in_degree_of(&self, v: &V) -> usize {
        match self.vertices.get(v) {
            None => 0,
            Some(a) if D::DIRECTED => a.inc.len(),
            Some(_) => self.degree_of(v),
        }
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> IncidenceGraph
    for BaseGraph<V, E, D, S>
{
    fn edges_of(&self, v: &V) -> impl Iterator<Item = &E> {
        // JGraphT order: incoming edges first, then outgoing edges (self-loops only once)
        let a = self.vertices.get(v);
        let inc = a.into_iter().flat_map(|a| a.inc.iter());
        let out = a
            .into_iter()
            .flat_map(|a| a.out.iter())
            .filter(move |e| !D::DIRECTED || self.edges[*e].src != self.edges[*e].tgt);
        inc.chain(out)
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, S> DirectedGraph for BaseGraph<V, E, Directed, S> {}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, S> UndirectedGraph
    for BaseGraph<V, E, Undirected, S>
{
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> GraphMut
    for BaseGraph<V, E, D, S>
{
    fn add_vertex(&mut self, v: V) -> bool {
        if self.vertices.contains_key(&v) {
            return false;
        }
        self.vertices.insert(v, Adj::default());
        true
    }

    fn remove_vertex(&mut self, v: &V) -> bool {
        if !self.vertices.contains_key(v) {
            return false;
        }
        let incident: Vec<E> = self.edges_of(v).cloned().collect();
        for e in &incident {
            self.remove_edge(e);
        }
        self.vertices.shift_remove(v);
        true
    }

    fn add_edge_with(&mut self, u: &V, v: &V, e: E) -> Result<bool> {
        if !self.vertices.contains_key(u) || !self.vertices.contains_key(v) {
            return Err(GraphError::VertexNotFound);
        }
        if let Some(r) = self.edges.get(&e) {
            // the same edge object may not be reused for different endpoints
            let same = if D::DIRECTED {
                &r.src == u && &r.tgt == v
            } else {
                (&r.src == u && &r.tgt == v) || (&r.src == v && &r.tgt == u)
            };
            return if same {
                Ok(false)
            } else {
                Err(GraphError::IntrusiveEdge)
            };
        }
        if u == v && !self.ty.self_loops {
            return Err(GraphError::LoopsNotAllowed);
        }
        if !self.ty.multiple_edges && self.edges_between(u, v).next().is_some() {
            return Ok(false);
        }
        if !self.ty.allows_cycles && (u == v || self.reaches(v, u)) {
            return Err(GraphError::CycleProhibited);
        }
        self.vertices[u].out.insert(e.clone());
        if D::DIRECTED {
            self.vertices[v].inc.insert(e.clone());
        } else if u != v {
            self.vertices[v].out.insert(e.clone());
        }
        self.edges.insert(
            e.clone(),
            EdgeRec {
                src: u.clone(),
                tgt: v.clone(),
                weight: 1.0,
            },
        );
        if let Some(index) = &mut self.endpoint_index {
            index
                .entry((u.clone(), v.clone()))
                .or_default()
                .insert(e.clone());
            if !D::DIRECTED && u != v {
                index.entry((v.clone(), u.clone())).or_default().insert(e);
            }
        }
        Ok(true)
    }

    fn remove_edge(&mut self, e: &E) -> bool {
        let Some(r) = self.edges.shift_remove(e) else {
            return false;
        };
        self.vertices[&r.src].out.shift_remove(e);
        if D::DIRECTED {
            self.vertices[&r.tgt].inc.shift_remove(e);
        } else if r.src != r.tgt {
            self.vertices[&r.tgt].out.shift_remove(e);
        }
        if let Some(index) = &mut self.endpoint_index {
            remove_indexed_edge(index, &(r.src.clone(), r.tgt.clone()), e);
            if !D::DIRECTED && r.src != r.tgt {
                remove_indexed_edge(index, &(r.tgt, r.src), e);
            }
        }
        true
    }

    fn set_edge_weight(&mut self, e: &E, w: f64) -> Result<()> {
        if !self.ty.weighted {
            return Err(GraphError::NotWeighted);
        }
        self.edges
            .get_mut(e)
            .ok_or(GraphError::EdgeNotFound)?
            .weight = w;
        Ok(())
    }

    fn new_vertex(&mut self) -> Result<V> {
        let f = self.vertex_factory.ok_or(GraphError::NoVertexFactory)?;
        // Like a Java supplier: no check against existing vertices (callers see `add_vertex` fail).
        let v = f(self.next_vertex);
        self.next_vertex += 1;
        Ok(v)
    }

    fn new_edge(&mut self) -> Result<E> {
        let f = self.factory.ok_or(GraphError::NoEdgeFactory)?;
        loop {
            let e = f(self.next_id);
            self.next_id += 1;
            if !self.edges.contains_key(&e) {
                return Ok(e);
            }
        }
    }
}

fn remove_indexed_edge<V: Clone + Eq + Hash, E: Eq + Hash>(
    index: &mut IndexMap<(V, V), IndexSet<E>>,
    endpoints: &(V, V),
    edge: &E,
) {
    let remove_entry = index.get_mut(endpoints).is_some_and(|edges| {
        edges.shift_remove(edge);
        edges.is_empty()
    });
    if remove_entry {
        index.shift_remove(endpoints);
    }
}

impl<V, E, D, S> fmt::Display for BaseGraph<V, E, D, S>
where
    V: fmt::Display + Clone + Eq + Hash,
    E: fmt::Display + Clone + Eq + Hash + 'static,
    D: Direction,
{
    /// JGraphT's `toString`; see [`super::graph_to_string`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&super::graph_to_string(self))
    }
}

/// JGraphT's `AbstractGraph.equals` (see [`super::graph_equals`]).
impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> PartialEq
    for BaseGraph<V, E, D, S>
{
    fn eq(&self, other: &Self) -> bool {
        super::graph_equals(self, other)
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction, S> Hash for BaseGraph<V, E, D, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(super::graph_hash(self));
    }
}
