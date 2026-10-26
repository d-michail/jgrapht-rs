use crate::GraphType;
use crate::graph::{DirectedGraph, Graph, UndirectedGraph};
use std::collections::HashMap;

/// Overlays edge weights on any graph; edges without an entry get `default`.
#[derive(Debug, Clone)]
pub struct AsWeighted<G: Graph> {
    graph: G,
    weights: HashMap<G::Edge, f64>,
    default: f64,
}

impl<G: Graph> AsWeighted<G> {
    pub fn new(graph: G, weights: HashMap<G::Edge, f64>, default: f64) -> Self {
        AsWeighted {
            graph,
            weights,
            default,
        }
    }
    /// Weights computed by `f` for every current edge.
    pub fn from_fn(graph: G, f: impl Fn(&G::Edge) -> f64) -> Self {
        let weights = graph.edges().map(|e| (e.clone(), f(e))).collect();
        AsWeighted {
            graph,
            weights,
            default: 1.0,
        }
    }
    pub fn set_weight(&mut self, e: G::Edge, w: f64) {
        self.weights.insert(e, w);
    }
}

impl<G: Graph> Graph for AsWeighted<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.graph.graph_type().as_weighted()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.graph.vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.graph.edges()
    }
    fn vertex_count(&self) -> usize {
        self.graph.vertex_count()
    }
    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.graph.contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.graph.contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.graph.edge_source(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.graph.edge_target(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.graph
            .contains_edge(e)
            .then(|| self.weights.get(e).copied().unwrap_or(self.default))
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.graph.edges_of(v)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.graph.edges_between(u, v)
    }
}
impl<G: DirectedGraph> DirectedGraph for AsWeighted<G> {}
impl<G: UndirectedGraph> UndirectedGraph for AsWeighted<G> {}

/// Treats every edge weight as `1.0`.
#[derive(Debug, Clone)]
pub struct AsUnweighted<G>(pub G);

impl<G: Graph> Graph for AsUnweighted<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type().as_unweighted()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.0.vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.0.edges()
    }
    fn vertex_count(&self) -> usize {
        self.0.vertex_count()
    }
    fn edge_count(&self) -> usize {
        self.0.edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.0.contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.0.contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_source(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.0.edge_target(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.0.contains_edge(e).then_some(1.0)
    }
    fn edges_of(&self, v: &G::Vertex) -> impl Iterator<Item = &G::Edge> {
        self.0.edges_of(v)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.0.edges_between(u, v)
    }
}
impl<G: DirectedGraph> DirectedGraph for AsUnweighted<G> {}
impl<G: UndirectedGraph> UndirectedGraph for AsUnweighted<G> {}
