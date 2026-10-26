use crate::GraphType;
use crate::graph::{DirectedGraph, Graph, UndirectedGraph};
use std::collections::HashSet;
use std::hash::Hash;

/// Decides which vertices or edges stay visible in a subgraph.
pub trait Mask<T> {
    fn keep(&self, item: &T) -> bool;
}
impl<T, F: Fn(&T) -> bool> Mask<T> for F {
    fn keep(&self, item: &T) -> bool {
        self(item)
    }
}
impl<T: Eq + Hash> Mask<T> for HashSet<T> {
    fn keep(&self, item: &T) -> bool {
        self.contains(item)
    }
}

/// Subgraph defined by vertex and edge masks. An edge is visible iff it passes the edge mask
/// and both its endpoints pass the vertex mask.
#[derive(Debug, Clone)]
pub struct MaskSubgraph<G, MV, ME> {
    graph: G,
    vmask: MV,
    emask: ME,
}

/// Subgraph given by explicit vertex and edge sets.
pub type AsSubgraph<G> =
    MaskSubgraph<G, HashSet<<G as Graph>::Vertex>, HashSet<<G as Graph>::Edge>>;

impl<G: Graph> MaskSubgraph<G, HashSet<G::Vertex>, HashSet<G::Edge>> {
    /// Vertex-induced subgraph: all edges of `g` between the given vertices.
    pub fn induced(g: G, vertices: impl IntoIterator<Item = G::Vertex>) -> Self {
        let vs: HashSet<_> = vertices
            .into_iter()
            .filter(|v| g.contains_vertex(v))
            .collect();
        let es = g
            .edges()
            .filter(|e| {
                vs.contains(g.edge_source(e).unwrap()) && vs.contains(g.edge_target(e).unwrap())
            })
            .cloned()
            .collect();
        MaskSubgraph {
            graph: g,
            vmask: vs,
            emask: es,
        }
    }

    /// Subgraph with exactly the given vertices and edges (edges lacking an endpoint are hidden).
    pub fn with_edges(
        g: G,
        vertices: impl IntoIterator<Item = G::Vertex>,
        edges: impl IntoIterator<Item = G::Edge>,
    ) -> Self {
        MaskSubgraph {
            vmask: vertices
                .into_iter()
                .filter(|v| g.contains_vertex(v))
                .collect(),
            emask: edges.into_iter().filter(|e| g.contains_edge(e)).collect(),
            graph: g,
        }
    }
}

impl<G: Graph, MV: Mask<G::Vertex>, ME: Mask<G::Edge>> MaskSubgraph<G, MV, ME> {
    /// Hides vertices and edges for which the masks return `false`.
    pub fn new(graph: G, vmask: MV, emask: ME) -> Self {
        MaskSubgraph {
            graph,
            vmask,
            emask,
        }
    }
    fn vertex_ok(&self, v: &G::Vertex) -> bool {
        self.vmask.keep(v) && self.graph.contains_vertex(v)
    }
    fn edge_ok(&self, e: &G::Edge) -> bool {
        match (self.graph.edge_source(e), self.graph.edge_target(e)) {
            (Some(s), Some(t)) => self.emask.keep(e) && self.vmask.keep(s) && self.vmask.keep(t),
            _ => false,
        }
    }
}

impl<G: Graph, MV: Mask<G::Vertex>, ME: Mask<G::Edge>> Graph for MaskSubgraph<G, MV, ME> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.graph.graph_type().as_unmodifiable()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.graph.vertices().filter(|v| self.vmask.keep(v))
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.graph.edges().filter(|e| self.edge_ok(e))
    }
    fn vertex_count(&self) -> usize {
        self.vertices().count()
    }
    fn edge_count(&self) -> usize {
        self.edges().count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.vertex_ok(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.edge_ok(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.edge_ok(e).then(|| self.graph.edge_source(e)).flatten()
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.edge_ok(e).then(|| self.graph.edge_target(e)).flatten()
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.edge_ok(e).then(|| self.graph.edge_weight(e)).flatten()
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertex_ok(v);
        self.graph
            .edges_of(v)
            .filter(move |e| ok && self.edge_ok(e))
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        let ok = self.vertex_ok(u) && self.vertex_ok(v);
        self.graph
            .edges_between(u, v)
            .filter(move |e| ok && self.edge_ok(e))
    }
}

impl<G: DirectedGraph, MV: Mask<G::Vertex>, ME: Mask<G::Edge>> DirectedGraph
    for MaskSubgraph<G, MV, ME>
{
    fn in_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertex_ok(v);
        self.graph
            .in_edges_of(v)
            .filter(move |e| ok && self.edge_ok(e))
    }
    fn out_edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        let ok = self.vertex_ok(v);
        self.graph
            .out_edges_of(v)
            .filter(move |e| ok && self.edge_ok(e))
    }
}
impl<G: UndirectedGraph, MV: Mask<G::Vertex>, ME: Mask<G::Edge>> UndirectedGraph
    for MaskSubgraph<G, MV, ME>
{
}
