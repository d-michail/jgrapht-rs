use super::{DirectedGraph, Graph, GraphMut, UndirectedGraph};
use crate::{GraphError, GraphType, Result};
use indexmap::{IndexMap, IndexSet};
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

/// Edge type used when the caller does not supply their own.
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
    const DIRECTED: bool;
}
#[derive(Debug, Clone, Copy)]
pub struct Directed;
#[derive(Debug, Clone, Copy)]
pub struct Undirected;
impl Direction for Directed {
    const DIRECTED: bool = true;
}
impl Direction for Undirected {
    const DIRECTED: bool = false;
}

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
pub struct BaseGraph<V, E = DefaultEdge, D = Undirected> {
    ty: GraphType,
    vertices: IndexMap<V, Adj<E>>,
    edges: IndexMap<E, EdgeRec<V>>,
    next_id: u64,
    factory: Option<fn(u64) -> E>,
    next_vertex: u64,
    vertex_factory: Option<fn(u64) -> V>,
    _dir: PhantomData<D>,
}

/// Undirected [`BaseGraph`] with [`DefaultEdge`]s.
pub type UGraph<V, E = DefaultEdge> = BaseGraph<V, E, Undirected>;
/// Directed [`BaseGraph`] with [`DefaultEdge`]s.
pub type DiGraph<V, E = DefaultEdge> = BaseGraph<V, E, Directed>;

impl<V, E, D: Direction> fmt::Debug for BaseGraph<V, E, D>
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

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> BaseGraph<V, E, D> {
    /// Creates a graph that mints edges with `factory(counter)` when `add_edge` is used.
    pub fn with_edge_factory(ty: GraphType, factory: fn(u64) -> E) -> Self {
        Self::build(ty, Some(factory))
    }

    /// Creates a graph without an edge factory: edges must be added with `add_edge_with`.
    pub fn without_edge_factory(ty: GraphType) -> Self {
        Self::build(ty, None)
    }

    fn build(ty: GraphType, factory: Option<fn(u64) -> E>) -> Self {
        BaseGraph {
            ty: GraphType {
                directed: D::DIRECTED,
                ..ty
            },
            vertices: IndexMap::new(),
            edges: IndexMap::new(),
            next_id: 0,
            factory,
            next_vertex: 0,
            vertex_factory: None,
            _dir: PhantomData,
        }
    }

    /// Installs a vertex factory used by [`GraphMut::add_new_vertex`]: `factory(0)`, `factory(1)`, ...
    pub fn with_vertex_factory(mut self, factory: fn(u64) -> V) -> Self {
        self.vertex_factory = Some(factory);
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

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> Graph for BaseGraph<V, E, D> {
    type Vertex = V;
    type Edge = E;

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
    fn edges_of(&self, v: &V) -> impl Iterator<Item = &E> {
        let a = self.vertices.get(v);
        let out = a.into_iter().flat_map(|a| a.out.iter());
        let inc = a.into_iter().flat_map(|a| a.inc.iter()).filter(move |e| {
            // directed self-loops are already in `out`
            D::DIRECTED && self.edges[*e].src != self.edges[*e].tgt
        });
        out.chain(inc)
    }
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
    fn out_edges_of(&self, v: &V) -> impl Iterator<Item = &E> {
        self.out_edges(v)
    }
    fn in_degree_of(&self, v: &V) -> usize {
        match self.vertices.get(v) {
            None => 0,
            Some(a) if D::DIRECTED => a.inc.len(),
            Some(_) => self.degree_of(v),
        }
    }
    fn out_degree_of(&self, v: &V) -> usize {
        match self.vertices.get(v) {
            None => 0,
            Some(a) if D::DIRECTED => a.out.len(),
            Some(_) => self.degree_of(v),
        }
    }
    fn edges_between<'a>(&'a self, u: &'a V, v: &'a V) -> impl Iterator<Item = &'a E> {
        self.out_edges(u).filter(move |e| {
            let r = &self.edges[*e];
            if D::DIRECTED {
                &r.tgt == v
            } else {
                (&r.src == u && &r.tgt == v) || (&r.src == v && &r.tgt == u)
            }
        })
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> DirectedGraph for BaseGraph<V, E, Directed> {}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> UndirectedGraph for BaseGraph<V, E, Undirected> {}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> GraphMut for BaseGraph<V, E, D> {
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
            e,
            EdgeRec {
                src: u.clone(),
                tgt: v.clone(),
                weight: 1.0,
            },
        );
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
        loop {
            let v = f(self.next_vertex);
            self.next_vertex += 1;
            if !self.vertices.contains_key(&v) {
                return Ok(v);
            }
        }
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

impl<V, E, D> fmt::Display for BaseGraph<V, E, D>
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
impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> PartialEq for BaseGraph<V, E, D> {
    fn eq(&self, other: &Self) -> bool {
        super::graph_equals(self, other)
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> Hash for BaseGraph<V, E, D> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(super::graph_hash(self));
    }
}
